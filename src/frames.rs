use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};

use crate::{
    benchmark::BenchmarkStatus,
    chat::{self, ChatMessage},
    config::Paths,
    llm::{self, LlmRequest, Message},
    memory::{MemoryProvenance, MemoryRetrievalHit, MemoryStore},
};

const DEFAULT_AGENT: &str = "coobie";
const DEFAULT_DIRECT_CONTEXT_CHAR_LIMIT: usize = 16_000;
const MIN_DIRECT_CONTEXT_CHAR_LIMIT: usize = 1_600;
const DEFAULT_HARKONNEN_RETRIEVAL_DEPTH: usize = 2;
const DEFAULT_DIRECT_RETRIEVAL_DEPTH: usize = 1;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FramesMode {
    Harkonnen,
    Direct,
}

impl FramesMode {
    fn parse(value: Option<String>) -> Result<Self> {
        match value
            .unwrap_or_else(|| "harkonnen".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "harkonnen" | "packchat" => Ok(Self::Harkonnen),
            "direct" | "raw" | "baseline" => Ok(Self::Direct),
            other => bail!("unsupported FRAMES_MODE: {}", other),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Harkonnen => "harkonnen",
            Self::Direct => "direct",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FramesRunConfig {
    pub dataset_path: PathBuf,
    pub output_dir: PathBuf,
    pub mode: FramesMode,
    pub agent: String,
    pub direct_provider: String,
    pub limit: Option<usize>,
    pub min_accuracy: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FramesRunOutput {
    pub output_dir: PathBuf,
    pub predictions_path: PathBuf,
    pub summary_path: PathBuf,
    pub markdown_path: PathBuf,
    pub mode: FramesMode,
    pub provider_label: String,
    pub metrics: FramesMetrics,
    pub threshold_failure: Option<String>,
}

#[derive(Debug, Clone)]
pub enum FramesSuiteOutcome {
    Completed(FramesRunOutput),
    Skipped(String),
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct FramesMetrics {
    pub total: usize,
    pub accuracy: f64,
    pub exact_match: f64,
    pub token_f1: f64,
    pub by_reasoning_type: BTreeMap<String, FramesReasoningMetrics>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct FramesReasoningMetrics {
    pub total: usize,
    pub accuracy: f64,
    pub exact_match: f64,
    pub token_f1: f64,
}

#[derive(Debug, Clone, Serialize)]
struct FramesSummary {
    dataset_path: String,
    mode: String,
    agent: String,
    provider_label: String,
    limit: Option<usize>,
    generated_at: String,
    metrics: FramesMetrics,
    predictions_path: String,
    questions: Vec<FramesQuestionResult>,
}

#[derive(Debug, Clone, Serialize)]
struct FramesQuestionResult {
    question_id: String,
    prompt: String,
    expected_answer: String,
    raw_hypothesis: String,
    hypothesis: String,
    accuracy: bool,
    exact_match: bool,
    token_f1: f64,
    reasoning_types: Vec<String>,
    contexts_used: Vec<String>,
    retrieval_trace: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct FramesPrediction<'a> {
    question_id: &'a str,
    hypothesis: &'a str,
}

#[derive(Debug, Clone, Deserialize)]
struct FramesFixtureQuestion {
    #[serde(default)]
    question_id: String,
    #[serde(default, alias = "Prompt")]
    prompt: String,
    #[serde(default, alias = "Answer")]
    answer: String,
    #[serde(default)]
    reasoning_types: Value,
    #[serde(default)]
    wiki_links: Vec<String>,
    #[serde(default)]
    contexts: Vec<FramesContextDocument>,
}

#[derive(Debug, Clone)]
struct FramesQuestion {
    question_id: String,
    prompt: String,
    answer: String,
    reasoning_types: Vec<String>,
    wiki_links: Vec<String>,
    contexts: Vec<FramesContextDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FramesContextDocument {
    title: String,
    text: String,
    #[serde(default)]
    source_url: Option<String>,
}

#[derive(Debug, Default)]
struct FramesAccumulator {
    total: usize,
    accuracy: f64,
    exact_match: f64,
    token_f1: f64,
}

impl FramesAccumulator {
    fn add(&mut self, result: &FramesQuestionResult) {
        self.total += 1;
        if result.accuracy {
            self.accuracy += 1.0;
        }
        if result.exact_match {
            self.exact_match += 1.0;
        }
        self.token_f1 += result.token_f1;
    }

    fn finalize(&self) -> FramesReasoningMetrics {
        FramesReasoningMetrics {
            total: self.total,
            accuracy: ratio(self.accuracy, self.total),
            exact_match: ratio(self.exact_match, self.total),
            token_f1: ratio(self.token_f1, self.total),
        }
    }
}

#[derive(Debug, Clone)]
struct RetrievedFrameContext {
    document: FramesContextDocument,
    hit: MemoryRetrievalHit,
    hop: u8,
    query: String,
}

pub async fn run_with_overrides(
    paths: &Paths,
    overrides: &BTreeMap<String, String>,
) -> Result<FramesSuiteOutcome> {
    let dataset_path = match read_env("FRAMES_DATASET", overrides) {
        Some(value) => PathBuf::from(value),
        None => {
            return Ok(FramesSuiteOutcome::Skipped(
                "FRAMES_DATASET is not set".to_string(),
            ))
        }
    };

    let mode = FramesMode::parse(read_env("FRAMES_MODE", overrides))?;
    let output_dir = read_env("FRAMES_OUTPUT_DIR", overrides)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            paths.artifacts.join("benchmarks").join(format!(
                "frames-{}-{}",
                mode.as_str(),
                Utc::now().timestamp_millis()
            ))
        });

    let config = FramesRunConfig {
        dataset_path,
        output_dir,
        mode,
        agent: read_env("FRAMES_AGENT", overrides).unwrap_or_else(|| DEFAULT_AGENT.to_string()),
        direct_provider: read_env("FRAMES_DIRECT_PROVIDER", overrides)
            .unwrap_or_else(|| "default".to_string()),
        limit: parse_env_usize_with_overrides("FRAMES_LIMIT", overrides)?,
        min_accuracy: parse_env_f64_with_overrides("FRAMES_MIN_ACCURACY", overrides)?,
    };

    Ok(FramesSuiteOutcome::Completed(run(paths, &config).await?))
}

pub async fn run(paths: &Paths, config: &FramesRunConfig) -> Result<FramesRunOutput> {
    let mut questions = load_questions(&config.dataset_path).await?;
    if let Some(limit) = config.limit {
        questions.truncate(limit);
    }
    if questions.is_empty() {
        bail!("FRAMES dataset contains no questions")
    }

    tokio::fs::create_dir_all(&config.output_dir)
        .await
        .with_context(|| format!("creating FRAMES output dir {}", config.output_dir.display()))?;

    let predictions_path = config.output_dir.join("predictions.jsonl");
    let summary_path = config.output_dir.join("summary.json");
    let markdown_path = config.output_dir.join("summary.md");
    let cache_dir = config.output_dir.join("wiki-cache");
    tokio::fs::create_dir_all(&cache_dir)
        .await
        .with_context(|| format!("creating FRAMES cache dir {}", cache_dir.display()))?;

    let mut prediction_lines = Vec::with_capacity(questions.len());
    let mut results = Vec::with_capacity(questions.len());
    let mut accumulator = FramesAccumulator::default();
    let mut by_reasoning_type: HashMap<String, FramesAccumulator> = HashMap::new();
    let verbose_progress = benchmark_verbose_progress();
    let show_raw_output = benchmark_show_raw_output();

    for (index, question) in questions.iter().enumerate() {
        if verbose_progress {
            eprintln!(
                "[FRAMES][{}/{}][{}] starting {}",
                index + 1,
                questions.len(),
                config.mode.as_str(),
                question.question_id
            );
        }

        let contexts = resolve_question_contexts(question, &cache_dir).await?;
        if contexts.is_empty() {
            bail!(
                "FRAMES question {} has no usable contexts",
                question.question_id
            );
        }

        let (raw_hypothesis, contexts_used, retrieval_trace) = match config.mode {
            FramesMode::Harkonnen => {
                answer_with_harkonnen(paths, config, question, &contexts).await?
            }
            FramesMode::Direct => answer_with_direct(paths, config, question, &contexts).await?,
        };

        let result = evaluate_question(question, raw_hypothesis, contexts_used, retrieval_trace);
        if verbose_progress {
            eprintln!(
                "[FRAMES][{}/{}][{}] done {} accuracy={} exact_match={} token_f1={:.4}",
                index + 1,
                questions.len(),
                config.mode.as_str(),
                result.question_id,
                result.accuracy,
                result.exact_match,
                result.token_f1
            );
            eprintln!(
                "[FRAMES][{}] answer: {}",
                result.question_id, result.hypothesis
            );
            if show_raw_output {
                eprintln!(
                    "[FRAMES][{}] raw output:\n{}\n---",
                    result.question_id, result.raw_hypothesis
                );
            }
        }

        accumulator.add(&result);
        for reasoning_type in effective_reasoning_types(&result.reasoning_types) {
            by_reasoning_type
                .entry(reasoning_type)
                .or_default()
                .add(&result);
        }
        prediction_lines.push(serde_json::to_string(&FramesPrediction {
            question_id: &result.question_id,
            hypothesis: &result.hypothesis,
        })?);
        results.push(result);
    }

    let metrics = FramesMetrics {
        total: accumulator.total,
        accuracy: ratio(accumulator.accuracy, accumulator.total),
        exact_match: ratio(accumulator.exact_match, accumulator.total),
        token_f1: ratio(accumulator.token_f1, accumulator.total),
        by_reasoning_type: finalize_by_reasoning_type(by_reasoning_type),
    };

    tokio::fs::write(
        &predictions_path,
        format!("{}\n", prediction_lines.join("\n")),
    )
    .await
    .with_context(|| format!("writing FRAMES predictions {}", predictions_path.display()))?;

    let summary = FramesSummary {
        dataset_path: config.dataset_path.display().to_string(),
        mode: config.mode.as_str().to_string(),
        agent: config.agent.clone(),
        provider_label: provider_label(config),
        limit: config.limit,
        generated_at: Utc::now().to_rfc3339(),
        metrics: metrics.clone(),
        predictions_path: predictions_path.display().to_string(),
        questions: results,
    };

    tokio::fs::write(&summary_path, serde_json::to_string_pretty(&summary)?)
        .await
        .with_context(|| format!("writing FRAMES summary {}", summary_path.display()))?;
    tokio::fs::write(&markdown_path, render_markdown_summary(&summary))
        .await
        .with_context(|| format!("writing FRAMES markdown {}", markdown_path.display()))?;

    let threshold_failure = threshold_failure(&metrics, config);

    Ok(FramesRunOutput {
        output_dir: config.output_dir.clone(),
        predictions_path,
        summary_path,
        markdown_path,
        mode: config.mode,
        provider_label: provider_label(config),
        metrics,
        threshold_failure,
    })
}

pub fn render_step_stdout(output: &FramesRunOutput) -> String {
    let mut lines = vec![
        format!("FRAMES mode: {}", output.mode.as_str()),
        format!("Provider path: {}", output.provider_label),
        format!("FRAMES output dir: {}", output.output_dir.display()),
        format!("Predictions: {}", output.predictions_path.display()),
        format!("Summary JSON: {}", output.summary_path.display()),
        format!("Summary Markdown: {}", output.markdown_path.display()),
        format!("Questions: {}", output.metrics.total),
        format!("Proxy accuracy: {:.4}", output.metrics.accuracy),
        format!("Proxy exact match: {:.4}", output.metrics.exact_match),
        format!("Proxy token F1: {:.4}", output.metrics.token_f1),
    ];
    if let Some(reason) = &output.threshold_failure {
        lines.push(format!("Threshold failure: {}", reason));
    }
    lines.join("\n")
}

pub fn status_for_output(output: &FramesRunOutput) -> BenchmarkStatus {
    if output.threshold_failure.is_some() {
        BenchmarkStatus::Failed
    } else {
        BenchmarkStatus::Passed
    }
}

pub fn reason_for_output(output: &FramesRunOutput) -> Option<String> {
    output.threshold_failure.clone()
}

async fn answer_with_harkonnen(
    paths: &Paths,
    config: &FramesRunConfig,
    question: &FramesQuestion,
    contexts: &[FramesContextDocument],
) -> Result<(String, Vec<String>, Vec<String>)> {
    let memory_root = config
        .output_dir
        .join("question-memory")
        .join(sanitize_id(&question.question_id));
    if memory_root.exists() {
        let _ = tokio::fs::remove_dir_all(&memory_root).await;
    }
    let (store, context_by_id) =
        build_question_memory_store(&memory_root, question, contexts).await?;
    let retrieved = retrieve_context_chain(
        &store,
        &context_by_id,
        &question.prompt,
        DEFAULT_HARKONNEN_RETRIEVAL_DEPTH,
    )
    .await?;
    let selected_contexts = contexts_from_hits(&retrieved, contexts, 4);
    let retrieval_trace = retrieval_trace(&retrieved, "two-hop");
    let history = build_context_history(&question.question_id, &config.agent, &selected_contexts);
    let prompt = build_harkonnen_prompt(question, &retrieval_trace);
    let history = history_with_prompt(&history, &question.question_id, &config.agent, &prompt);
    let reply = chat::complete_agent_reply(&config.agent, &prompt, &history, None, paths)
        .await
        .with_context(|| {
            format!(
                "answering FRAMES question {} via PackChat",
                question.question_id
            )
        })?;
    Ok((
        reply,
        selected_contexts
            .iter()
            .map(|doc| doc.title.clone())
            .collect(),
        retrieval_trace,
    ))
}

async fn answer_with_direct(
    paths: &Paths,
    config: &FramesRunConfig,
    question: &FramesQuestion,
    contexts: &[FramesContextDocument],
) -> Result<(String, Vec<String>, Vec<String>)> {
    let memory_root = config
        .output_dir
        .join("question-memory")
        .join(sanitize_id(&question.question_id));
    if memory_root.exists() {
        let _ = tokio::fs::remove_dir_all(&memory_root).await;
    }
    let (store, context_by_id) =
        build_question_memory_store(&memory_root, question, contexts).await?;
    let retrieved = retrieve_context_chain(
        &store,
        &context_by_id,
        &question.prompt,
        DEFAULT_DIRECT_RETRIEVAL_DEPTH,
    )
    .await?;
    let selected_contexts = contexts_from_hits(&retrieved, contexts, 4);
    let retrieval_trace = retrieval_trace(&retrieved, "single-hop");

    let provider = llm::build_provider("benchmark", &config.direct_provider, &paths.setup)
        .with_context(|| {
            format!(
                "no configured provider available for FRAMES direct mode via {}",
                config.direct_provider
            )
        })?;

    let mut context_budget = direct_context_char_limit();
    loop {
        let req = LlmRequest {
            messages: vec![Message::user(build_direct_prompt(
                question,
                &selected_contexts,
                context_budget,
            ))],
            max_tokens: 256,
            temperature: 0.0,
        };

        match provider.complete(req).await {
            Ok(resp) => {
                return Ok((
                    resp.content,
                    selected_contexts
                        .iter()
                        .map(|doc| doc.title.clone())
                        .collect(),
                    retrieval_trace,
                ))
            }
            Err(err)
                if is_context_window_error(&err)
                    && context_budget > MIN_DIRECT_CONTEXT_CHAR_LIMIT =>
            {
                let next_budget = (context_budget / 2).max(MIN_DIRECT_CONTEXT_CHAR_LIMIT);
                if benchmark_verbose_progress() {
                    eprintln!(
                        "[FRAMES][direct] context retry {} budget {} -> {} chars",
                        question.question_id, context_budget, next_budget
                    );
                }
                if next_budget == context_budget {
                    return Err(err).with_context(|| {
                        format!(
                            "FRAMES direct provider failed via {}",
                            config.direct_provider
                        )
                    });
                }
                context_budget = next_budget;
            }
            Err(err) => {
                return Err(err).with_context(|| {
                    format!(
                        "FRAMES direct provider failed via {}",
                        config.direct_provider
                    )
                });
            }
        }
    }
}

async fn build_question_memory_store(
    root: &Path,
    question: &FramesQuestion,
    contexts: &[FramesContextDocument],
) -> Result<(MemoryStore, HashMap<String, FramesContextDocument>)> {
    tokio::fs::create_dir_all(root.join("imports"))
        .await
        .with_context(|| format!("creating FRAMES memory dir {}", root.display()))?;
    let store = MemoryStore::new(root.to_path_buf());
    let mut context_by_id = HashMap::new();
    for (index, context) in contexts.iter().enumerate() {
        let id = format!("{}-ctx-{}", sanitize_id(&question.question_id), index + 1);
        let mut tags = vec!["frames".to_string(), "wikipedia".to_string()];
        for token in retrieval_tokens(&context.title, 4, 6) {
            if !tags.iter().any(|existing| existing == &token) {
                tags.push(token);
            }
        }
        let provenance = MemoryProvenance {
            source_label: Some(context.title.clone()),
            source_kind: Some("frames_context".to_string()),
            source_path: context.source_url.clone(),
            ..MemoryProvenance::default()
        };
        store
            .store_with_metadata(&id, tags, &context.title, &context.text, provenance)
            .await?;
        context_by_id.insert(id, context.clone());
    }
    Ok((store, context_by_id))
}

async fn retrieve_context_chain(
    store: &MemoryStore,
    context_by_id: &HashMap<String, FramesContextDocument>,
    original_query: &str,
    depth: usize,
) -> Result<Vec<RetrievedFrameContext>> {
    let mut active_query = original_query.to_string();
    let mut all_hits = Vec::new();
    let mut seen_ids = HashSet::new();

    for hop in 1..=depth.max(1) {
        let hits = store
            .retrieve_ranked_entries(&active_query, None, 4)
            .await?;
        for hit in hits {
            if !seen_ids.insert(hit.id.clone()) {
                continue;
            }
            if let Some(document) = context_by_id.get(&hit.id).cloned() {
                all_hits.push(RetrievedFrameContext {
                    document,
                    hit,
                    hop: hop as u8,
                    query: active_query.clone(),
                });
            }
        }

        if hop >= depth {
            break;
        }
        let Some(next_query) = follow_up_query_from_hits(original_query, &all_hits) else {
            break;
        };
        if next_query == active_query {
            break;
        }
        active_query = next_query;
    }

    Ok(all_hits)
}

fn contexts_from_hits(
    hits: &[RetrievedFrameContext],
    fallback: &[FramesContextDocument],
    limit: usize,
) -> Vec<FramesContextDocument> {
    if hits.is_empty() {
        return fallback.iter().take(limit).cloned().collect();
    }
    hits.iter()
        .take(limit)
        .map(|hit| hit.document.clone())
        .collect()
}

fn retrieval_trace(hits: &[RetrievedFrameContext], label: &str) -> Vec<String> {
    if hits.is_empty() {
        return vec![format!("{}:no_hits", label)];
    }
    hits.iter()
        .map(|hit| {
            let mut parts = vec![
                format!("{}:hop_{}", label, hit.hop),
                format!("query={}", hit.query),
                format!("title={}", hit.document.title),
                format!("score={:.2}", hit.hit.score),
            ];
            if let Some(status) = hit.hit.status.as_deref() {
                parts.push(format!("status={}", status));
            }
            if let Some(superseded_by) = hit.hit.superseded_by.as_deref() {
                parts.push(format!("superseded_by={}", superseded_by));
            }
            if !hit.hit.surfaced_via.is_empty() {
                parts.push(format!("via={}", hit.hit.surfaced_via.join("; ")));
            }
            parts.join(" | ")
        })
        .collect()
}

fn build_context_history(
    thread_id: &str,
    agent: &str,
    contexts: &[FramesContextDocument],
) -> Vec<ChatMessage> {
    contexts
        .iter()
        .enumerate()
        .map(|(index, context)| ChatMessage {
            message_id: format!("{}-ctx-{}", thread_id, index + 1),
            thread_id: thread_id.to_string(),
            role: "operator".to_string(),
            agent: Some(agent.to_string()),
            content: format_context_note(context, 3_000),
            checkpoint_id: None,
            created_at: Utc::now(),
        })
        .collect()
}

fn format_context_note(context: &FramesContextDocument, max_chars: usize) -> String {
    let excerpt = truncate_chars(context.text.trim(), max_chars);
    match context.source_url.as_deref() {
        Some(url) if !url.trim().is_empty() => {
            format!(
                "Context note [{}]\nSource: {}\n{}",
                context.title, url, excerpt
            )
        }
        _ => format!("Context note [{}]\n{}", context.title, excerpt),
    }
}

fn build_harkonnen_prompt(question: &FramesQuestion, retrieval_trace: &[String]) -> String {
    let reasoning = render_reasoning_types(&question.reasoning_types);
    let trace = if retrieval_trace.is_empty() {
        "none".to_string()
    } else {
        retrieval_trace.join(" ; ")
    };
    format!(
        "FRAMES reasoning types: {}\nRetrieval trace: {}\nQuestion: {}\nAnswer using only the earlier context notes. Chain multiple notes when needed. Return only the bare answer text with no reasoning, no XML tags, no <think> blocks, and no preamble. If the notes do not contain enough information, reply exactly: I do not know.",
        reasoning,
        trace,
        question.prompt.trim()
    )
}

fn build_direct_prompt(
    question: &FramesQuestion,
    contexts: &[FramesContextDocument],
    context_budget: usize,
) -> String {
    let context_blocks = contexts
        .iter()
        .map(|context| format_context_note(context, 6_000))
        .collect::<Vec<_>>();
    let (joined_context, truncated) = truncate_context_blocks(&context_blocks, context_budget);
    let context_label = if truncated {
        "Context notes (truncated to fit local model context):"
    } else {
        "Context notes:"
    };
    format!(
        "You are answering a FRAMES multi-hop benchmark question using only the context notes below.\n\n{}\n{}\n\nReasoning types: {}\nQuestion: {}\n\nReturn only the bare answer text with no reasoning, no XML tags, no <think> blocks, and no preamble. If the context notes do not contain enough information, reply exactly: I do not know.",
        context_label,
        joined_context,
        render_reasoning_types(&question.reasoning_types),
        question.prompt.trim()
    )
}

fn history_with_prompt(
    history: &[ChatMessage],
    thread_id: &str,
    agent: &str,
    prompt: &str,
) -> Vec<ChatMessage> {
    let mut augmented = history.to_vec();
    augmented.push(ChatMessage {
        message_id: format!("{}-prompt", thread_id),
        thread_id: thread_id.to_string(),
        role: "operator".to_string(),
        agent: Some(agent.to_string()),
        content: prompt.to_string(),
        checkpoint_id: None,
        created_at: Utc::now(),
    });
    augmented
}

async fn resolve_question_contexts(
    question: &FramesQuestion,
    cache_dir: &Path,
) -> Result<Vec<FramesContextDocument>> {
    if !question.contexts.is_empty() {
        return Ok(question.contexts.clone());
    }

    let mut contexts = Vec::new();
    let mut seen = HashSet::new();
    for link in &question.wiki_links {
        let normalized = link.trim();
        if normalized.is_empty() || !seen.insert(normalized.to_string()) {
            continue;
        }
        contexts.push(fetch_wikipedia_context(normalized, cache_dir).await?);
    }
    Ok(contexts)
}

async fn fetch_wikipedia_context(
    source_url: &str,
    cache_dir: &Path,
) -> Result<FramesContextDocument> {
    tokio::fs::create_dir_all(cache_dir)
        .await
        .with_context(|| format!("creating FRAMES cache dir {}", cache_dir.display()))?;
    let title = wikipedia_title_from_url(source_url)
        .with_context(|| format!("parsing wikipedia title from {}", source_url))?;
    let cache_path = cache_dir.join(format!("{}.json", sanitize_id(&title)));
    if cache_path.exists() {
        let raw = tokio::fs::read_to_string(&cache_path).await?;
        let cached: FramesContextDocument = serde_json::from_str(&raw)
            .with_context(|| format!("parsing cached FRAMES context {}", cache_path.display()))?;
        return Ok(cached);
    }

    let client = reqwest::Client::builder().build()?;
    let response = client
        .get("https://en.wikipedia.org/w/api.php")
        .query(&[
            ("action", "query"),
            ("format", "json"),
            ("formatversion", "2"),
            ("prop", "extracts"),
            ("explaintext", "1"),
            ("redirects", "1"),
            ("titles", title.as_str()),
        ])
        .header("User-Agent", "Harkonnen-Labs/0.1 FRAMES benchmark")
        .send()
        .await?
        .error_for_status()?;
    let value: Value = response.json().await?;
    let page = value
        .get("query")
        .and_then(|query| query.get("pages"))
        .and_then(|pages| pages.as_array())
        .and_then(|pages| pages.first())
        .cloned()
        .context("FRAMES wikipedia response missing pages[0]")?;
    let extract = page
        .get("extract")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("FRAMES wikipedia response missing article extract")?;
    let context = FramesContextDocument {
        title: page
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or(&title)
            .to_string(),
        text: extract.to_string(),
        source_url: Some(source_url.to_string()),
    };
    tokio::fs::write(&cache_path, serde_json::to_string_pretty(&context)?)
        .await
        .with_context(|| format!("writing FRAMES cache {}", cache_path.display()))?;
    Ok(context)
}

async fn load_questions(path: &Path) -> Result<Vec<FramesQuestion>> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("reading FRAMES dataset {}", path.display()))?;
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "json" => load_questions_from_json(&raw, path),
        "tsv" | "csv" => load_questions_from_tsv(&raw, path),
        other => bail!("unsupported FRAMES dataset extension `{}`", other),
    }
}

fn load_questions_from_json(raw: &str, path: &Path) -> Result<Vec<FramesQuestion>> {
    let fixtures: Vec<FramesFixtureQuestion> = serde_json::from_str(raw)
        .with_context(|| format!("parsing FRAMES json dataset {}", path.display()))?;
    Ok(fixtures
        .into_iter()
        .enumerate()
        .map(|(index, fixture)| FramesQuestion {
            question_id: if fixture.question_id.trim().is_empty() {
                format!("frames-json-{}", index)
            } else {
                fixture.question_id
            },
            prompt: fixture.prompt,
            answer: fixture.answer,
            reasoning_types: parse_reasoning_types_value(&fixture.reasoning_types),
            wiki_links: fixture.wiki_links,
            contexts: fixture.contexts,
        })
        .collect())
}

fn load_questions_from_tsv(raw: &str, path: &Path) -> Result<Vec<FramesQuestion>> {
    let mut lines = raw.lines();
    let header_line = lines
        .next()
        .with_context(|| format!("FRAMES dataset {} is empty", path.display()))?;
    let headers = header_line.split('\t').collect::<Vec<_>>();
    let mut questions = Vec::new();

    for (row_idx, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let values = line.split('\t').collect::<Vec<_>>();
        let mut row = HashMap::new();
        for (index, header) in headers.iter().enumerate() {
            row.insert(
                (*header).to_string(),
                values.get(index).copied().unwrap_or("").to_string(),
            );
        }

        let prompt = row.get("Prompt").cloned().unwrap_or_default();
        let answer = row.get("Answer").cloned().unwrap_or_default();
        if prompt.trim().is_empty() || answer.trim().is_empty() {
            continue;
        }
        let mut wiki_links = Vec::new();
        if let Some(value) = row.get("wiki_links") {
            extend_unique_strings(&mut wiki_links, extract_urls(value));
        }
        let mut ordered_link_keys = row
            .keys()
            .filter(|key| key.starts_with("wikipedia_link_"))
            .cloned()
            .collect::<Vec<_>>();
        ordered_link_keys.sort();
        for key in ordered_link_keys {
            if let Some(value) = row.get(&key) {
                extend_unique_strings(&mut wiki_links, extract_urls(value));
            }
        }

        questions.push(FramesQuestion {
            question_id: row
                .get("Unnamed: 0")
                .filter(|value| !value.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| format!("frames-tsv-{}", row_idx)),
            prompt,
            answer,
            reasoning_types: row
                .get("reasoning_types")
                .map(|value| parse_reasoning_types_string(value))
                .unwrap_or_else(|| vec!["unknown".to_string()]),
            wiki_links,
            contexts: Vec::new(),
        });
    }

    Ok(questions)
}

fn parse_reasoning_types_value(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => {
            let parsed = items
                .iter()
                .filter_map(Value::as_str)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            if parsed.is_empty() {
                vec!["unknown".to_string()]
            } else {
                parsed
            }
        }
        Value::String(value) => parse_reasoning_types_string(value),
        _ => vec!["unknown".to_string()],
    }
}

fn parse_reasoning_types_string(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return vec!["unknown".to_string()];
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let inner = trimmed.trim_start_matches('[').trim_end_matches(']');
        let parsed = inner
            .split(',')
            .map(|item| item.trim().trim_matches('\'').trim_matches('"').to_string())
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>();
        if parsed.is_empty() {
            vec!["unknown".to_string()]
        } else {
            parsed
        }
    } else {
        let parsed = trimmed
            .split('|')
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>();
        if parsed.is_empty() {
            vec!["unknown".to_string()]
        } else {
            parsed
        }
    }
}

fn effective_reasoning_types(values: &[String]) -> Vec<String> {
    if values.is_empty() {
        vec!["unknown".to_string()]
    } else {
        values.to_vec()
    }
}

fn extract_urls(value: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for token in value.split_whitespace() {
        if token.contains("http://") || token.contains("https://") {
            let cleaned = token
                .trim()
                .trim_matches(',')
                .trim_matches('[')
                .trim_matches(']')
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            if !cleaned.is_empty() {
                urls.push(cleaned);
            }
        }
    }
    urls
}

fn wikipedia_title_from_url(url: &str) -> Result<String> {
    let parsed =
        reqwest::Url::parse(url).with_context(|| format!("parsing wikipedia url {}", url))?;
    let segment = parsed
        .path_segments()
        .and_then(|segments| segments.last())
        .filter(|segment| !segment.trim().is_empty())
        .context("missing wikipedia page title segment")?;
    Ok(segment.to_string())
}

fn evaluate_question(
    question: &FramesQuestion,
    raw_hypothesis: String,
    contexts_used: Vec<String>,
    retrieval_trace: Vec<String>,
) -> FramesQuestionResult {
    let hypothesis = extract_final_answer(&raw_hypothesis);
    let normalized_hypothesis = normalize_text(&hypothesis);
    let normalized_answer = normalize_text(&question.answer);
    let exact_match =
        !normalized_hypothesis.is_empty() && normalized_hypothesis == normalized_answer;
    let accuracy = exact_match
        || (!normalized_hypothesis.is_empty()
            && !normalized_answer.is_empty()
            && (normalized_hypothesis.contains(&normalized_answer)
                || normalized_answer.contains(&normalized_hypothesis)));

    FramesQuestionResult {
        question_id: question.question_id.clone(),
        prompt: question.prompt.clone(),
        expected_answer: question.answer.clone(),
        raw_hypothesis,
        hypothesis,
        accuracy,
        exact_match,
        token_f1: token_f1(&normalized_hypothesis, &normalized_answer),
        reasoning_types: question.reasoning_types.clone(),
        contexts_used,
        retrieval_trace,
    }
}

fn follow_up_query_from_hits(
    original_query: &str,
    hits: &[RetrievedFrameContext],
) -> Option<String> {
    let top_hit = hits.first()?;
    let original_tokens = retrieval_tokens(original_query, 4, usize::MAX)
        .into_iter()
        .collect::<HashSet<_>>();

    let mut parts = vec![top_hit.document.title.clone()];
    let excerpt_terms = retrieval_tokens(&top_hit.document.text, 5, 6)
        .into_iter()
        .filter(|token| !original_tokens.contains(token))
        .collect::<Vec<_>>();
    if excerpt_terms.is_empty() {
        return None;
    }
    parts.extend(excerpt_terms);
    Some(parts.join(" "))
}

fn retrieval_tokens(value: &str, min_len: usize, limit: usize) -> Vec<String> {
    let mut tokens = Vec::new();
    for token in value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() >= min_len)
        .map(|token| token.to_ascii_lowercase())
    {
        if tokens.iter().any(|existing| existing == &token) {
            continue;
        }
        tokens.push(token);
        if tokens.len() >= limit {
            break;
        }
    }
    tokens
}

fn truncate_context_blocks(blocks: &[String], max_chars: usize) -> (String, bool) {
    let joined = blocks.join("\n\n");
    if joined.chars().count() <= max_chars {
        return (joined, false);
    }

    let mut kept = Vec::new();
    let mut used = 0usize;
    for block in blocks.iter() {
        let block_chars = block.chars().count();
        let separator = if kept.is_empty() { 0 } else { 2 };
        if used + block_chars + separator > max_chars {
            let remaining = max_chars.saturating_sub(used + separator);
            if remaining > 0 && kept.is_empty() {
                kept.push(truncate_chars(block, remaining));
            }
            break;
        }
        kept.push(block.clone());
        used += block_chars + separator;
    }
    (kept.join("\n\n"), true)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let total = value.chars().count();
    if total <= max_chars {
        return value.to_string();
    }
    let mut out = value.chars().take(max_chars).collect::<String>();
    out.push_str("...");
    out
}

fn render_reasoning_types(values: &[String]) -> String {
    if values.is_empty() {
        "unknown".to_string()
    } else {
        values.join(" | ")
    }
}

fn render_markdown_summary(summary: &FramesSummary) -> String {
    let mut lines = vec![
        "# FRAMES Summary".to_string(),
        String::new(),
        format!("- Dataset: {}", summary.dataset_path),
        format!("- Mode: {}", summary.mode),
        format!("- Agent: {}", summary.agent),
        format!("- Provider path: {}", summary.provider_label),
        format!(
            "- Limit: {}",
            summary
                .limit
                .map(|value| value.to_string())
                .unwrap_or_else(|| "full".to_string())
        ),
        format!("- Generated: {}", summary.generated_at),
        format!("- Predictions: {}", summary.predictions_path),
        String::new(),
        "## Aggregate Metrics".to_string(),
        String::new(),
        format!("- Questions: {}", summary.metrics.total),
        format!("- Proxy accuracy: {:.4}", summary.metrics.accuracy),
        format!("- Proxy exact match: {:.4}", summary.metrics.exact_match),
        format!("- Proxy token F1: {:.4}", summary.metrics.token_f1),
        String::new(),
        "## By Reasoning Type".to_string(),
        String::new(),
        "| Reasoning Type | Questions | Accuracy | Exact Match | Token F1 |".to_string(),
        "| --- | ---: | ---: | ---: | ---: |".to_string(),
    ];

    for (reasoning_type, metrics) in &summary.metrics.by_reasoning_type {
        lines.push(format!(
            "| {} | {} | {:.4} | {:.4} | {:.4} |",
            reasoning_type, metrics.total, metrics.accuracy, metrics.exact_match, metrics.token_f1,
        ));
    }

    lines.push(String::new());
    lines.join("\n")
}

fn threshold_failure(metrics: &FramesMetrics, config: &FramesRunConfig) -> Option<String> {
    if let Some(min_accuracy) = config.min_accuracy {
        if metrics.accuracy < min_accuracy {
            return Some(format!(
                "proxy accuracy {:.4} below FRAMES_MIN_ACCURACY {:.4}",
                metrics.accuracy, min_accuracy
            ));
        }
    }
    None
}

fn finalize_by_reasoning_type(
    by_reasoning_type: HashMap<String, FramesAccumulator>,
) -> BTreeMap<String, FramesReasoningMetrics> {
    let mut ordered = BTreeMap::new();
    for (reasoning_type, acc) in by_reasoning_type {
        ordered.insert(reasoning_type, acc.finalize());
    }
    ordered
}

fn extract_final_answer(raw_hypothesis: &str) -> String {
    let without_think = strip_think_blocks(raw_hypothesis);
    let trimmed = without_think.trim();
    if trimmed.is_empty() {
        return raw_hypothesis.trim().to_string();
    }

    let mut chosen = trimmed.to_string();
    let non_empty_lines = trimmed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if let Some(last_line) = non_empty_lines.last() {
        chosen = (*last_line).to_string();
    }

    for prefix in ["answer:", "final answer:", "response:"] {
        if chosen.to_lowercase().starts_with(prefix) {
            chosen = chosen[prefix.len()..].trim().to_string();
            break;
        }
    }

    chosen
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches(' ')
        .to_string()
}

fn strip_think_blocks(value: &str) -> String {
    let mut remaining = value;
    let mut cleaned = String::new();
    loop {
        if let Some(start) = remaining.find("<think>") {
            cleaned.push_str(&remaining[..start]);
            let after_start = &remaining[start + "<think>".len()..];
            if let Some(end) = after_start.find("</think>") {
                remaining = &after_start[end + "</think>".len()..];
                continue;
            }
            break;
        }
        cleaned.push_str(remaining);
        break;
    }
    cleaned
}

fn normalize_text(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn token_f1(prediction: &str, answer: &str) -> f64 {
    if prediction.is_empty() || answer.is_empty() {
        return 0.0;
    }
    let pred_tokens = prediction.split_whitespace().collect::<Vec<_>>();
    let answer_tokens = answer.split_whitespace().collect::<Vec<_>>();
    if pred_tokens.is_empty() || answer_tokens.is_empty() {
        return 0.0;
    }

    let mut answer_counts = HashMap::new();
    for token in answer_tokens {
        *answer_counts.entry(token).or_insert(0usize) += 1;
    }

    let mut overlap = 0usize;
    for token in pred_tokens.iter().copied() {
        if let Some(count) = answer_counts.get_mut(token) {
            if *count > 0 {
                overlap += 1;
                *count -= 1;
            }
        }
    }
    if overlap == 0 {
        return 0.0;
    }

    let precision = overlap as f64 / pred_tokens.len() as f64;
    let recall = overlap as f64
        / answer_counts
            .values()
            .sum::<usize>()
            .saturating_add(overlap) as f64;
    if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    }
}

fn ratio(numerator: f64, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator / denominator as f64
    }
}

fn provider_label(config: &FramesRunConfig) -> String {
    match config.mode {
        FramesMode::Harkonnen => {
            format!(
                "PackChat agent {} with two-hop retrieved context",
                config.agent
            )
        }
        FramesMode::Direct => format!(
            "Direct provider {} with flat single-hop retrieved context",
            config.direct_provider
        ),
    }
}

fn read_env(name: &str, overrides: &BTreeMap<String, String>) -> Option<String> {
    overrides
        .get(name)
        .cloned()
        .or_else(|| env::var(name).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_env_usize_with_overrides(
    name: &str,
    overrides: &BTreeMap<String, String>,
) -> Result<Option<usize>> {
    match read_env(name, overrides) {
        Some(value) => value
            .parse::<usize>()
            .map(Some)
            .with_context(|| format!("parsing {} as usize", name)),
        None => Ok(None),
    }
}

fn parse_env_f64_with_overrides(
    name: &str,
    overrides: &BTreeMap<String, String>,
) -> Result<Option<f64>> {
    match read_env(name, overrides) {
        Some(value) => value
            .parse::<f64>()
            .map(Some)
            .with_context(|| format!("parsing {} as f64", name)),
        None => Ok(None),
    }
}

fn direct_context_char_limit() -> usize {
    env::var("FRAMES_DIRECT_MAX_CHARS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value >= MIN_DIRECT_CONTEXT_CHAR_LIMIT)
        .unwrap_or(DEFAULT_DIRECT_CONTEXT_CHAR_LIMIT)
}

fn is_context_window_error(err: &anyhow::Error) -> bool {
    let text = err.to_string().to_ascii_lowercase();
    text.contains("context length")
        || text.contains("context window")
        || text.contains("context size")
        || text.contains("has been exceeded")
        || text.contains("n_keep")
        || text.contains("n_ctx")
        || text.contains("too many tokens")
}

fn benchmark_verbose_progress() -> bool {
    benchmark_flag_enabled("HARKONNEN_BENCH_VERBOSE")
        || benchmark_flag_enabled("HARKONNEN_BENCH_PROGRESS")
}

fn benchmark_show_raw_output() -> bool {
    benchmark_flag_enabled("HARKONNEN_BENCH_SHOW_RAW")
        || benchmark_flag_enabled("HARKONNEN_BENCH_SHOW_THINK")
}

fn benchmark_flag_enabled(name: &str) -> bool {
    env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn sanitize_id(value: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in value.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            prev_dash = false;
            ch.to_ascii_lowercase()
        } else {
            if prev_dash {
                continue;
            }
            prev_dash = true;
            '-'
        };
        out.push(mapped);
    }
    out.trim_matches('-').to_string()
}

fn extend_unique_strings(target: &mut Vec<String>, values: Vec<String>) {
    for value in values {
        if !target.iter().any(|existing| existing == &value) {
            target.push(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reasoning_types_supports_pipe_strings() {
        assert_eq!(
            parse_reasoning_types_string("Multiple constraints | Temporal reasoning"),
            vec![
                "Multiple constraints".to_string(),
                "Temporal reasoning".to_string()
            ]
        );
    }

    #[test]
    fn extract_urls_handles_python_style_lists() {
        assert_eq!(
            extract_urls("['https://en.wikipedia.org/wiki/Ada_Lovelace', 'https://en.wikipedia.org/wiki/London']"),
            vec![
                "https://en.wikipedia.org/wiki/Ada_Lovelace".to_string(),
                "https://en.wikipedia.org/wiki/London".to_string()
            ]
        );
    }

    #[test]
    fn frames_mode_parses_synonyms() {
        assert!(matches!(
            FramesMode::parse(Some("packchat".to_string())).unwrap(),
            FramesMode::Harkonnen
        ));
        assert!(matches!(
            FramesMode::parse(Some("raw".to_string())).unwrap(),
            FramesMode::Direct
        ));
    }

    #[test]
    fn extract_final_answer_prefers_last_line() {
        let raw = "Reasoning\nAnswer: Jane Ballou";
        assert_eq!(extract_final_answer(raw), "Jane Ballou");
    }
}
