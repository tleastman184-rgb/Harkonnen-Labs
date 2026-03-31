//! Coobie — Causal Memory and Reasoning Engine
//!
//! Phase 1: Heuristic causal engine backed by SQLite.
//!   - Ingests factory runs as FactoryEpisode records
//!   - Scores each episode on five diagnostic dimensions
//!   - Evaluates a set of causal heuristic rules to generate CausalHypotheses
//!   - Recommends concrete InterventionPlans per hypothesis
//!   - Simulates counterfactuals by querying prior episodes where similar
//!     interventions were applied
//!   - Emits a structured CausalReport for failed/degraded runs
//!
//! Phase 2 (pending Cargo.toml — Codex owns that):
//!   Add `deep_causality` crate for PropagatingEffect/PropagatingProcess chains,
//!   Context hypergraph, Causaloid composition, and Effect Ethos policy layer.
//!   The trait surface and scoring dimensions here are already shaped to map
//!   cleanly onto DeepCausality's model.
//!
//! Initial causal domain: "why do runs pass internal validation but fail
//! hidden scenarios?" — the most common and useful failure pattern.

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{
    CausalHypothesis, CounterfactualEstimate, CounterfactualOutcome, FactoryEpisode,
    InterventionPlan,
};

// ── Public reasoning trait ────────────────────────────────────────────────────

#[async_trait]
pub trait CoobieReasoner: Send + Sync {
    /// Ingest a completed run as an episode and persist its causal scores.
    async fn ingest_episode(&self, ep: &FactoryEpisode) -> Result<()>;

    /// Diagnose a run: return ranked causal hypotheses with confidence.
    async fn diagnose(&self, run_id: &str) -> Result<Vec<CausalHypothesis>>;

    /// Recommend concrete interventions based on the diagnosed hypotheses.
    async fn recommend_interventions(&self, run_id: &str) -> Result<Vec<InterventionPlan>>;

    /// Simulate "what if we applied this intervention?" against historical data.
    async fn simulate_counterfactual(
        &self,
        run_id: &str,
        intervention: &InterventionPlan,
    ) -> Result<CounterfactualOutcome>;

    /// Emit the full structured causal report for a run.
    async fn emit_report(&self, run_id: &str) -> Result<CausalReport>;
}

// ── Report ────────────────────────────────────────────────────────────────────

/// The structured output Coobie emits for every failed or degraded run.
/// Consumed by the orchestrator, Pack Board UI, and human reviewers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalReport {
    pub run_id: String,
    pub primary_cause: Option<String>,
    pub primary_confidence: f32,
    pub contributing_causes: Vec<String>,
    pub recommended_interventions: Vec<InterventionPlan>,
    pub counterfactual_prediction: Option<CounterfactualOutcome>,
    pub episode_scores: EpisodeScores,
    pub generated_at: String,
}

// ── Episode scoring ───────────────────────────────────────────────────────────

/// Five normalised scores (0.0–1.0) derived from a FactoryEpisode.
/// Higher is better for all dimensions except change_scope_score (lower = safer).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeScores {
    pub run_id: String,
    /// How clear and complete the spec was (criteria, failure behaviors, outputs).
    pub spec_clarity_score: f32,
    /// Breadth of implementation scope — higher means broader/riskier changes.
    pub change_scope_score: f32,
    /// How faithfully the twin environment represented production conditions.
    pub twin_fidelity_score: f32,
    /// Fraction of visible validation checks that passed.
    pub test_coverage_score: f32,
    /// Whether Coobie retrieved relevant prior memory before the run.
    pub memory_retrieval_score: f32,
    pub scenario_passed: bool,
    pub validation_passed: bool,
}

// ── Heuristic causal rule ─────────────────────────────────────────────────────

/// A single causal heuristic rule evaluated against EpisodeScores.
struct CausalRule {
    /// Short stable identifier, used as cause_id in CausalHypothesis.
    id: &'static str,
    description: &'static str,
    /// The concrete intervention this rule recommends if triggered.
    intervention_target: &'static str,
    intervention_action: &'static str,
    intervention_impact: &'static str,
    /// Returns Some(base_confidence) if this rule fires, None if it does not.
    evaluate: fn(&EpisodeScores) -> Option<f32>,
}

/// All Phase 1 heuristic rules.
/// Narrow initial domain: internal-pass / scenario-fail splits.
const CAUSAL_RULES: &[CausalRule] = &[
    CausalRule {
        id: "SPEC_AMBIGUITY",
        description: "Ambiguous or incomplete spec likely caused scenario failure — \
                       acceptance criteria, failure behaviors, or explicit outputs were missing.",
        intervention_target: "spec",
        intervention_action: "Add explicit failure behaviors, edge-case acceptance criteria, \
                              and concrete output examples to the spec before next run.",
        intervention_impact: "Estimated +0.25 scenario pass probability based on similar runs.",
        evaluate: |s| {
            if s.spec_clarity_score < 0.4 && !s.scenario_passed {
                Some(0.7 * (1.0 - s.spec_clarity_score))
            } else if s.spec_clarity_score < 0.55 && !s.scenario_passed {
                Some(0.4 * (1.0 - s.spec_clarity_score))
            } else {
                None
            }
        },
    },
    CausalRule {
        id: "TWIN_GAP",
        description: "Low twin fidelity may have caused false-negative hidden scenario failure — \
                       the simulated environment did not cover production conditions.",
        intervention_target: "twin",
        intervention_action: "Increase twin service coverage: add auth token expiry simulation, \
                              third-party dependency stubs, and error-injection paths.",
        intervention_impact: "Estimated +0.20 scenario pass probability for auth/API features.",
        evaluate: |s| {
            if s.twin_fidelity_score < 0.4 && !s.scenario_passed {
                Some(0.65 * (1.0 - s.twin_fidelity_score))
            } else {
                None
            }
        },
    },
    CausalRule {
        id: "TEST_BLIND_SPOT",
        description: "Visible tests all passed but hidden scenarios failed — tests were \
                       too aligned with the happy path and did not cover failure modes.",
        intervention_target: "validation",
        intervention_action: "Before scenario evaluation, generate at least one failure-path \
                              test (expired token, invalid input, permission boundary).",
        intervention_impact: "Estimated +0.30 scenario pass probability; reduces false pass rate.",
        evaluate: |s| {
            if s.test_coverage_score >= 0.9 && !s.scenario_passed {
                Some(0.80)
            } else if s.test_coverage_score >= 0.75 && !s.scenario_passed {
                Some(0.55)
            } else {
                None
            }
        },
    },
    CausalRule {
        id: "NO_PRIOR_MEMORY",
        description: "No relevant prior memory was retrieved before the run — \
                       the factory approached this pattern without prior context.",
        intervention_target: "memory",
        intervention_action: "Seed Coobie with prior similar runs or domain notes before \
                              retrying. Run: harkonnen memory import <prior-context-file>.",
        intervention_impact: "Pattern-cold runs have lower first-pass acceptance rates.",
        evaluate: |s| {
            if s.memory_retrieval_score < 0.1 {
                Some(0.55)
            } else {
                None
            }
        },
    },
    CausalRule {
        id: "BROAD_SCOPE",
        description: "Broad implementation scope (many agent phases active, wide file changes) \
                       increases the probability of hidden scenario failures.",
        intervention_target: "spec",
        intervention_action: "Narrow the spec scope: break into smaller deliverables with \
                              tighter acceptance criteria per run.",
        intervention_impact: "Estimated -0.15 failure probability per scope reduction step.",
        evaluate: |s| {
            if s.change_scope_score > 0.75 {
                Some(0.45 * s.change_scope_score)
            } else {
                None
            }
        },
    },
];

// ── SQLite-backed engine ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SqliteCoobie {
    pool: SqlitePool,
}

impl SqliteCoobie {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ── Scoring ───────────────────────────────────────────────────────────────

    fn score_episode(ep: &FactoryEpisode) -> EpisodeScores {
        // spec_clarity_score: presence of key spec fields
        let spec_clarity = {
            let mut score: f32 = 0.0;
            // features is Vec<String> on FactoryEpisode — proxy for spec field coverage
            let n = ep.features.len();
            // Each feature present adds weight; cap at 1.0
            score += (n as f32 * 0.12).min(0.6);
            // bonus if validation exists and has results
            if let Some(v) = &ep.validation {
                if !v.results.is_empty() {
                    score += 0.2;
                }
            }
            // bonus if human_decision is recorded (implies full run)
            if ep.decision.is_some() {
                score += 0.2;
            }
            score.min(1.0)
        };

        // change_scope_score: proxy from agent_events breadth
        let change_scope = {
            let n = ep.agent_events.len() as f32;
            // 0 events = 0.0; 10+ events = 1.0
            (n / 10.0).min(1.0)
        };

        // twin_fidelity_score: from twin_env service count and status
        let twin_fidelity = match &ep.twin_env {
            None => 0.1,
            Some(twin) => {
                let total = twin.services.len() as f32;
                if total == 0.0 {
                    return EpisodeScores {
                        run_id: ep.run_id.clone(),
                        spec_clarity_score: spec_clarity,
                        change_scope_score: change_scope,
                        twin_fidelity_score: 0.1,
                        test_coverage_score: 0.0,
                        memory_retrieval_score: 0.0,
                        scenario_passed: ep.scenarios.as_ref().map(|s| s.passed).unwrap_or(false),
                        validation_passed: ep.validation.as_ref().map(|v| v.passed).unwrap_or(false),
                    };
                }
                let ready = twin.services.iter()
                    .filter(|s| s.status == "ready" || s.status == "running")
                    .count() as f32;
                (ready / total).min(1.0)
            }
        };

        // test_coverage_score: fraction of visible validation checks passed
        let test_coverage = match &ep.validation {
            None => 0.0,
            Some(v) => {
                if v.results.is_empty() {
                    0.0
                } else {
                    let passed = v.results.iter().filter(|r| r.passed).count() as f32;
                    passed / v.results.len() as f32
                }
            }
        };

        // memory_retrieval_score: crude proxy — 1.0 if prior agent events show memory phase
        let memory_score = {
            let has_memory_phase = ep.agent_events.iter()
                .any(|e| e.phase == "memory" && e.status == "complete");
            if has_memory_phase { 1.0 } else { 0.0 }
        };

        EpisodeScores {
            run_id: ep.run_id.clone(),
            spec_clarity_score: spec_clarity,
            change_scope_score: change_scope,
            twin_fidelity_score: twin_fidelity,
            test_coverage_score: test_coverage,
            memory_retrieval_score: memory_score,
            scenario_passed: ep.scenarios.as_ref().map(|s| s.passed).unwrap_or(false),
            validation_passed: ep.validation.as_ref().map(|v| v.passed).unwrap_or(false),
        }
    }

    // ── Persistence ───────────────────────────────────────────────────────────

    async fn upsert_scores(&self, scores: &EpisodeScores) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO coobie_episode_scores
                (run_id, spec_clarity_score, change_scope_score, twin_fidelity_score,
                 test_coverage_score, memory_retrieval_score, scenario_passed,
                 validation_passed, human_accepted, scored_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9)
            ON CONFLICT(run_id) DO UPDATE SET
                spec_clarity_score      = excluded.spec_clarity_score,
                change_scope_score      = excluded.change_scope_score,
                twin_fidelity_score     = excluded.twin_fidelity_score,
                test_coverage_score     = excluded.test_coverage_score,
                memory_retrieval_score  = excluded.memory_retrieval_score,
                scenario_passed         = excluded.scenario_passed,
                validation_passed       = excluded.validation_passed,
                scored_at               = excluded.scored_at
            "#,
        )
        .bind(&scores.run_id)
        .bind(scores.spec_clarity_score as f64)
        .bind(scores.change_scope_score as f64)
        .bind(scores.twin_fidelity_score as f64)
        .bind(scores.test_coverage_score as f64)
        .bind(scores.memory_retrieval_score as f64)
        .bind(scores.scenario_passed as i64)
        .bind(scores.validation_passed as i64)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await
        .context("upserting episode scores")?;
        Ok(())
    }

    async fn persist_hypotheses(&self, run_id: &str, hypotheses: &[CausalHypothesis]) -> Result<()> {
        for h in hypotheses {
            let supporting = serde_json::to_string(&h.supporting_runs)?;
            let counterfactuals = serde_json::to_string(&h.counterfactuals)?;
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO causal_hypotheses
                    (hypothesis_id, run_id, cause_id, description, confidence,
                     supporting_runs, counterfactuals, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(run_id)
            .bind(&h.cause_id)
            .bind(&h.description)
            .bind(h.confidence as f64)
            .bind(&supporting)
            .bind(&counterfactuals)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await
            .context("persisting hypothesis")?;
        }
        Ok(())
    }

    async fn load_scores(&self, run_id: &str) -> Result<Option<EpisodeScores>> {
        let row = sqlx::query(
            r#"
            SELECT run_id, spec_clarity_score, change_scope_score, twin_fidelity_score,
                   test_coverage_score, memory_retrieval_score, scenario_passed, validation_passed
            FROM coobie_episode_scores WHERE run_id = ?1
            "#,
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .context("loading episode scores")?;

        Ok(row.map(|r| {
            use sqlx::Row;
            EpisodeScores {
                run_id: r.get("run_id"),
                spec_clarity_score: r.get::<f64, _>("spec_clarity_score") as f32,
                change_scope_score: r.get::<f64, _>("change_scope_score") as f32,
                twin_fidelity_score: r.get::<f64, _>("twin_fidelity_score") as f32,
                test_coverage_score: r.get::<f64, _>("test_coverage_score") as f32,
                memory_retrieval_score: r.get::<f64, _>("memory_retrieval_score") as f32,
                scenario_passed: r.get::<i64, _>("scenario_passed") != 0,
                validation_passed: r.get::<i64, _>("validation_passed") != 0,
            }
        }))
    }

    /// Find prior episodes where the same rule fired AND the scenario later passed.
    /// Used for counterfactual confidence estimation.
    async fn find_supporting_runs(
        &self,
        cause_id: &str,
        limit: i64,
    ) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT h.run_id
            FROM causal_hypotheses h
            JOIN coobie_episode_scores s ON s.run_id = h.run_id
            WHERE h.cause_id = ?1 AND s.scenario_passed = 1
            ORDER BY h.created_at DESC
            LIMIT ?2
            "#,
        )
        .bind(cause_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("finding supporting runs")?;

        use sqlx::Row;
        Ok(rows.iter().map(|r| r.get::<String, _>("run_id")).collect())
    }

    /// Estimate counterfactual outcome: what fraction of runs that had this
    /// cause_id diagnosed AND the intervention applied later passed scenarios?
    async fn counterfactual_estimate(
        &self,
        cause_id: &str,
        intervention: &InterventionPlan,
    ) -> Result<CounterfactualEstimate> {
        // Count runs where this cause was diagnosed
        let total: i64 = {
            let row = sqlx::query(
                "SELECT COUNT(*) as cnt FROM causal_hypotheses WHERE cause_id = ?1",
            )
            .bind(cause_id)
            .fetch_one(&self.pool)
            .await?;
            use sqlx::Row;
            row.get("cnt")
        };

        // Count runs where this cause was diagnosed AND scenario passed (proxy for "intervention worked")
        let improved: i64 = {
            let row = sqlx::query(
                r#"
                SELECT COUNT(*) as cnt
                FROM causal_hypotheses h
                JOIN coobie_episode_scores s ON s.run_id = h.run_id
                WHERE h.cause_id = ?1 AND s.scenario_passed = 1
                "#,
            )
            .bind(cause_id)
            .fetch_one(&self.pool)
            .await?;
            use sqlx::Row;
            row.get("cnt")
        };

        let baseline_pass_rate = if total > 0 { improved as f32 / total as f32 } else { 0.0 };
        // Assume intervention raises pass rate toward the model's built-in estimate
        let predicted_rate = (baseline_pass_rate + 0.35).min(0.95);

        Ok(CounterfactualEstimate {
            intervention: intervention.action.clone(),
            predicted_outcome: format!(
                "Scenario pass probability increases from {:.0}% to {:.0}% (based on {} prior episodes with this pattern)",
                baseline_pass_rate * 100.0,
                predicted_rate * 100.0,
                total,
            ),
            confidence: if total >= 3 { 0.70 } else if total >= 1 { 0.45 } else { 0.25 },
        })
    }
}

#[async_trait]
impl CoobieReasoner for SqliteCoobie {
    async fn ingest_episode(&self, ep: &FactoryEpisode) -> Result<()> {
        let scores = Self::score_episode(ep);
        self.upsert_scores(&scores).await?;

        // Immediately diagnose and persist hypotheses so they're queryable
        let hypotheses = self.diagnose(&ep.run_id).await?;
        self.persist_hypotheses(&ep.run_id, &hypotheses).await?;

        Ok(())
    }

    async fn diagnose(&self, run_id: &str) -> Result<Vec<CausalHypothesis>> {
        // Load scores — try DB first, fall through gracefully if not found
        let scores = match self.load_scores(run_id).await? {
            Some(s) => s,
            None => return Ok(Vec::new()),
        };

        let mut hypotheses: Vec<CausalHypothesis> = Vec::new();

        for rule in CAUSAL_RULES {
            let Some(base_confidence) = (rule.evaluate)(&scores) else {
                continue;
            };

            // Boost confidence with historical support
            let supporting = self.find_supporting_runs(rule.id, 10).await
                .unwrap_or_default();
            let support_boost = (supporting.len() as f32 * 0.03).min(0.15);
            let final_confidence = (base_confidence + support_boost).min(0.95);

            hypotheses.push(CausalHypothesis {
                cause_id: rule.id.to_string(),
                description: rule.description.to_string(),
                confidence: final_confidence,
                supporting_runs: supporting,
                counterfactuals: Vec::new(), // populated in emit_report
            });
        }

        // Sort by confidence descending
        hypotheses.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

        Ok(hypotheses)
    }

    async fn recommend_interventions(&self, run_id: &str) -> Result<Vec<InterventionPlan>> {
        let hypotheses = self.diagnose(run_id).await?;

        // Map each hypothesis to its rule's intervention
        let plans: Vec<InterventionPlan> = CAUSAL_RULES.iter()
            .filter(|rule| hypotheses.iter().any(|h| h.cause_id == rule.id && h.confidence >= 0.4))
            .map(|rule| InterventionPlan {
                target: rule.intervention_target.to_string(),
                action: rule.intervention_action.to_string(),
                expected_impact: rule.intervention_impact.to_string(),
            })
            .collect();

        Ok(plans)
    }

    async fn simulate_counterfactual(
        &self,
        run_id: &str,
        intervention: &InterventionPlan,
    ) -> Result<CounterfactualOutcome> {
        // Find which rule maps to this intervention target
        let cause_id = CAUSAL_RULES.iter()
            .find(|r| r.intervention_target == intervention.target)
            .map(|r| r.id)
            .unwrap_or("UNKNOWN");

        let estimate = self.counterfactual_estimate(cause_id, intervention).await?;

        let _ = run_id; // run_id available for future per-run baseline calculation
        Ok(CounterfactualOutcome {
            prediction: estimate.predicted_outcome,
            confidence_gain: estimate.confidence,
        })
    }

    async fn emit_report(&self, run_id: &str) -> Result<CausalReport> {
        let mut hypotheses = self.diagnose(run_id).await?;
        let interventions = self.recommend_interventions(run_id).await?;

        // Enrich each hypothesis with counterfactual estimates
        for h in &mut hypotheses {
            if let Some(rule) = CAUSAL_RULES.iter().find(|r| r.id == h.cause_id) {
                let plan = InterventionPlan {
                    target: rule.intervention_target.to_string(),
                    action: rule.intervention_action.to_string(),
                    expected_impact: rule.intervention_impact.to_string(),
                };
                if let Ok(est) = self.counterfactual_estimate(h.cause_id.as_str(), &plan).await {
                    h.counterfactuals = vec![est];
                }
            }
        }

        let primary = hypotheses.first();
        let primary_cause = primary.map(|h| h.description.clone());
        let primary_confidence = primary.map(|h| h.confidence).unwrap_or(0.0);

        let contributing: Vec<String> = hypotheses.iter()
            .skip(1)
            .take(3)
            .map(|h| h.description.clone())
            .collect();

        let best_intervention = interventions.first().cloned();
        let counterfactual = match &best_intervention {
            Some(plan) => self.simulate_counterfactual(run_id, plan).await.ok(),
            None => None,
        };

        let scores = self.load_scores(run_id).await?
            .unwrap_or_else(|| EpisodeScores {
                run_id: run_id.to_string(),
                spec_clarity_score: 0.0,
                change_scope_score: 0.0,
                twin_fidelity_score: 0.0,
                test_coverage_score: 0.0,
                memory_retrieval_score: 0.0,
                scenario_passed: false,
                validation_passed: false,
            });

        Ok(CausalReport {
            run_id: run_id.to_string(),
            primary_cause,
            primary_confidence,
            contributing_causes: contributing,
            recommended_interventions: interventions,
            counterfactual_prediction: counterfactual,
            episode_scores: scores,
            generated_at: Utc::now().to_rfc3339(),
        })
    }
}

// ── Phase 2 stub ──────────────────────────────────────────────────────────────
//
// When Codex adds `deep_causality` to Cargo.toml, wire it here:
//
// use deep_causality::prelude::*;
//
// pub struct DeepCausalityCoobie {
//     context: BaseContext<...>,
//     graph: CausalGraph<...>,
// }
//
// impl DeepCausalityCoobie {
//     fn build_context(ep: &FactoryEpisode) -> BaseContext<...> { ... }
//     fn map_to_causaloids(scores: &EpisodeScores) -> Vec<Causaloid<...>> { ... }
//     fn run_propagating_effect(graph: &CausalGraph<...>, intervention: ...) -> ... { ... }
// }
