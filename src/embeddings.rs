use anyhow::{Context, Result};
use chrono::Utc;
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};

use crate::memory::MemoryEntry;

// ── EmbeddingStore ────────────────────────────────────────────────────────────

/// Semantic memory layer: wraps a local fastembed model (BGESmallENV15, ~33MB,
/// downloaded once to ~/.cache/huggingface/hub/) and stores 384-dim embeddings
/// in the shared SQLite DB under the `memory_embeddings` table.
///
/// Works alongside `MemoryStore` — same markdown files, same index, but
/// retrieval uses cosine similarity instead of keyword matching.
pub struct EmbeddingStore {
    /// BGESmallENV15 model wrapped in a std Mutex so it can be sent into
    /// spawn_blocking (the fastembed session is Send but not Sync).
    model: Arc<Mutex<TextEmbedding>>,
    pool: SqlitePool,
}

impl std::fmt::Debug for EmbeddingStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingStore").finish_non_exhaustive()
    }
}

impl Clone for EmbeddingStore {
    fn clone(&self) -> Self {
        Self {
            model: Arc::clone(&self.model),
            pool: self.pool.clone(),
        }
    }
}

impl EmbeddingStore {
    /// Load the BGESmallENV15 model and return a store backed by `pool`.
    ///
    /// First call downloads ~33MB to the HuggingFace cache; subsequent calls
    /// load from disk. Runs model init in a blocking thread so the tokio runtime
    /// stays free during the ONNX session creation.
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let model = tokio::task::spawn_blocking(|| {
            TextEmbedding::try_new(
                TextInitOptions::new(EmbeddingModel::BGESmallENV15)
                    .with_show_download_progress(false),
            )
        })
        .await
        .context("embedding model init thread panicked")?
        .context("loading fastembed BGESmallENV15 model")?;

        Ok(Self {
            model: Arc::new(Mutex::new(model)),
            pool,
        })
    }

    /// Compute and persist embeddings for any `entries` not yet stored under
    /// `memory_root`. Idempotent — already-embedded entries are skipped.
    pub async fn ensure_embedded(&self, entries: &[MemoryEntry], memory_root: &str) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        // Which entry IDs are already in the DB for this memory root?
        let existing: Vec<String> = sqlx::query_scalar(
            "SELECT entry_id FROM memory_embeddings WHERE memory_root = ?",
        )
        .bind(memory_root)
        .fetch_all(&self.pool)
        .await?;

        let existing_set: std::collections::HashSet<String> = existing.into_iter().collect();

        let new_entries: Vec<&MemoryEntry> = entries
            .iter()
            .filter(|e| !existing_set.contains(&e.id))
            .collect();

        if new_entries.is_empty() {
            return Ok(());
        }

        // Build one text per entry: summary + tags + first 512 chars of content.
        let texts: Vec<String> = new_entries
            .iter()
            .map(|e| {
                let tags = e.tags.join(", ");
                let body: String = e.content.chars().take(512).collect();
                format!("{}\n{}\n{}", e.summary, tags, body)
            })
            .collect();

        let model = Arc::clone(&self.model);
        let embeddings = tokio::task::spawn_blocking(move || {
            model.lock().unwrap().embed(texts, None)
        })
        .await
        .context("embedding thread panicked")?
        .context("computing embeddings for memory entries")?;

        let now = Utc::now().to_rfc3339();
        for (entry, embedding) in new_entries.iter().zip(embeddings.iter()) {
            let blob = vec_to_bytes(embedding);
            sqlx::query(
                "INSERT OR REPLACE INTO memory_embeddings \
                 (entry_id, memory_root, embedding, embedded_at) VALUES (?, ?, ?, ?)",
            )
            .bind(&entry.id)
            .bind(memory_root)
            .bind(&blob)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Cosine-similarity search. Returns up to `top_k` `(score, entry_id)` pairs
    /// sorted descending. Scores are in [0, 1] (cosine similarity against the
    /// embeddings in the DB for this `memory_root`).
    pub async fn query_semantic(
        &self,
        query: &str,
        memory_root: &str,
        top_k: usize,
    ) -> Result<Vec<(f32, String)>> {
        let query_text = query.to_string();
        let model = Arc::clone(&self.model);
        let query_embeddings = tokio::task::spawn_blocking(move || {
            model.lock().unwrap().embed(vec![query_text], None)
        })
        .await
        .context("query embedding thread panicked")?
        .context("computing query embedding")?;

        let q_vec = &query_embeddings[0];

        let rows: Vec<(String, Vec<u8>)> = sqlx::query_as(
            "SELECT entry_id, embedding FROM memory_embeddings WHERE memory_root = ?",
        )
        .bind(memory_root)
        .fetch_all(&self.pool)
        .await?;

        let mut scored: Vec<(f32, String)> = rows
            .into_iter()
            .map(|(id, blob)| {
                let entry_vec = bytes_to_vec(&blob);
                let score = cosine_similarity(q_vec, &entry_vec);
                (score, id)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        Ok(scored)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn vec_to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn bytes_to_vec(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
