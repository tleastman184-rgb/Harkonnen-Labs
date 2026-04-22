use anyhow::{Context, Result};
use futures::StreamExt;
use typedb_driver::{Credentials, DriverOptions, TransactionType, TypeDBDriver};
use uuid::Uuid;

const SCHEMA_TQL: &str = include_str!("../../factory/calvin_archive/typedb/schema.tql");
const SEED_TQL: &str = include_str!("../../factory/calvin_archive/typedb/coobie_kernel_seed.tql");

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub(crate) struct ArchiveExperience {
    pub run_id: String,
    pub episode_id: Option<String>,
    pub provider: String,
    pub model: String,
    pub narrative_summary: String,
    pub scope: String,
    pub chamber: Chamber,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Chamber {
    Mythos,
    Episteme,
    Ethos,
    Pathos,
    Logos,
    Praxis,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub(crate) struct BeliefRevision {
    pub belief_id: String,
    pub revised_summary: String,
    pub new_confidence: f64,
    pub revision_reason: String,
    pub preservation_note: Option<String>,
}

pub(crate) struct ArchiveStore {
    driver: TypeDBDriver,
    db_name: String,
}

impl ArchiveStore {
    pub(crate) async fn connect(url: &str, db_name: &str) -> Result<Self> {
        let credentials = Credentials::new("admin", "password");
        let options = DriverOptions::new(false, None).context("building DriverOptions")?;
        let driver = TypeDBDriver::new(url, credentials, options)
            .await
            .with_context(|| format!("connecting to TypeDB at {url}"))?;

        let dbs = driver.databases();
        if !dbs.contains(db_name).await? {
            dbs.create(db_name).await?;
            tracing::info!("Created TypeDB database '{db_name}'");
        }

        Ok(Self {
            driver,
            db_name: db_name.to_string(),
        })
    }

    pub(crate) async fn deploy_schema(&self) -> Result<()> {
        let tx = self
            .driver
            .transaction(&self.db_name, TransactionType::Schema)
            .await
            .context("opening schema transaction")?;
        tx.query(SCHEMA_TQL)
            .await
            .context("deploying TypeDB schema")?;
        tx.commit().await.context("committing schema")?;
        Ok(())
    }

    pub(crate) async fn seed_coobie_kernel(&self) -> Result<()> {
        let tx = self
            .driver
            .transaction(&self.db_name, TransactionType::Write)
            .await
            .context("opening write transaction for seed")?;
        tx.query(SEED_TQL).await.context("seeding Coobie kernel")?;
        tx.commit().await.context("committing seed")?;
        Ok(())
    }

    pub(crate) async fn open_run(
        &self,
        run_id: &str,
        spec_id: &str,
        provider: &str,
        model: &str,
    ) -> Result<()> {
        let tx = self
            .driver
            .transaction(&self.db_name, TransactionType::Write)
            .await?;
        let tql = format!(
            r#"insert $r isa run_record,
                has uuid "{run_id}",
                has name "run-{run_id}",
                has narrative_summary "Run {run_id} — spec: {spec_id}",
                has provider-name "{provider}",
                has model-name "{model}",
                has status "open";"#
        );
        tx.query(&tql).await.context("open_run query")?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn record_experience(&self, exp: &ArchiveExperience) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let chamber_tag = format!("{:?}", exp.chamber).to_lowercase();
        let tx = self
            .driver
            .transaction(&self.db_name, TransactionType::Write)
            .await?;
        let tql = format!(
            r#"insert $e isa experience,
                has uuid "{id}",
                has narrative_summary "{summary}",
                has scope "{scope}",
                has provider-name "{provider}",
                has model-name "{model}",
                has chamber-label "{chamber}";"#,
            summary = escape_tql(&exp.narrative_summary),
            scope = exp.scope,
            provider = exp.provider,
            model = exp.model,
            chamber = chamber_tag,
        );
        tx.query(&tql).await.context("record_experience query")?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn revise_belief(&self, rev: &BeliefRevision) -> Result<()> {
        let new_id = Uuid::new_v4().to_string();
        let preservation = rev.preservation_note.as_deref().unwrap_or("");
        let tx = self
            .driver
            .transaction(&self.db_name, TransactionType::Write)
            .await?;
        let tql = format!(
            r#"match $old isa belief, has uuid "{old_id}";
               insert $new isa belief,
                has uuid "{new_id}",
                has name "revised-{new_id}",
                has narrative_summary "{summary}",
                has confidence {conf};
               (prior: $old, next: $new) isa revised_into,
                has revision_reason "{reason}",
                has preservation_note "{preservation}";"#,
            old_id = rev.belief_id,
            summary = escape_tql(&rev.revised_summary),
            conf = rev.new_confidence,
            reason = escape_tql(&rev.revision_reason),
        );
        tx.query(&tql).await.context("revise_belief query")?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn close_run(&self, run_id: &str, outcome: &str) -> Result<()> {
        let tx = self
            .driver
            .transaction(&self.db_name, TransactionType::Write)
            .await?;
        let tql = format!(
            r#"match $r isa run_record, has uuid "{run_id}", has status $s;
               delete has $s of $r;
               insert has status "closed" of $r,
                      has outcome-label "{outcome}" of $r;"#
        );
        tx.query(&tql).await.context("close_run query")?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn get_kernel_traits(&self, agent_name: &str) -> Result<Vec<String>> {
        let tx = self
            .driver
            .transaction(&self.db_name, TransactionType::Read)
            .await?;
        let tql = format!(
            r#"match
                $a isa agent_self, has name "{agent_name}";
                $t isa trait, has name $tn;
                (target: $a, source: $t) isa stabilizes;
               select $tn;"#
        );
        let answer = tx.query(&tql).await.context("get_kernel_traits query")?;
        let mut results = Vec::new();
        let mut stream = answer.into_rows();
        while let Some(row_result) = stream.next().await {
            let row = row_result?;
            if let Ok(Some(concept)) = row.get("tn") {
                if let Some(s) = concept.try_get_string() {
                    results.push(s.to_string());
                }
            }
        }
        Ok(results)
    }

    pub(crate) async fn get_active_beliefs(&self, agent_name: &str) -> Result<Vec<String>> {
        let tx = self
            .driver
            .transaction(&self.db_name, TransactionType::Read)
            .await?;
        // Active beliefs: not yet superseded
        let tql = format!(
            r#"match
                $a isa agent_self, has name "{agent_name}";
                $b isa belief, has narrative_summary $ns, has confidence $c;
                not {{ (prior: $b) isa revised_into; }};
               select $ns, $c;
               sort $c desc;
               limit 20;"#
        );
        let answer = tx.query(&tql).await.context("get_active_beliefs query")?;
        let mut results = Vec::new();
        let mut stream = answer.into_rows();
        while let Some(row_result) = stream.next().await {
            let row = row_result?;
            if let Ok(Some(concept)) = row.get("ns") {
                if let Some(s) = concept.try_get_string() {
                    results.push(s.to_string());
                }
            }
        }
        Ok(results)
    }

    pub(crate) async fn get_causal_patterns(&self, domain: &str) -> Result<Vec<String>> {
        let tx = self
            .driver
            .transaction(&self.db_name, TransactionType::Read)
            .await?;
        let tql = format!(
            r#"match
                $p isa causal_pattern, has narrative_summary $ns, has confidence $c;
               select $ns, $c;
               sort $c desc;
               limit 10;"#
        );
        let _ = domain; // future: filter by scope containing domain
        let answer = tx.query(&tql).await.context("get_causal_patterns query")?;
        let mut results = Vec::new();
        let mut stream = answer.into_rows();
        while let Some(row_result) = stream.next().await {
            let row = row_result?;
            if let Ok(Some(concept)) = row.get("ns") {
                if let Some(s) = concept.try_get_string() {
                    results.push(s.to_string());
                }
            }
        }
        Ok(results)
    }

    pub(crate) async fn check_adaptation_safe(
        &self,
        adaptation_summary: &str,
        agent_name: &str,
    ) -> Result<bool> {
        let tx = self
            .driver
            .transaction(&self.db_name, TransactionType::Read)
            .await?;
        let tql = format!(
            r#"match
                $a isa agent_self, has name "{agent_name}";
                $t isa trait, has name $tn;
                (target: $a, source: $t) isa stabilizes, has confidence $c;
                $c > 0.8;
               select $tn;"#
        );
        let answer = tx
            .query(&tql)
            .await
            .context("check_adaptation_safe query")?;
        let mut high_confidence_traits: Vec<String> = Vec::new();
        let mut stream = answer.into_rows();
        while let Some(row_result) = stream.next().await {
            let row = row_result?;
            if let Ok(Some(concept)) = row.get("tn") {
                if let Some(s) = concept.try_get_string() {
                    high_confidence_traits.push(s.to_string());
                }
            }
        }
        let lower = adaptation_summary.to_lowercase();
        for trait_name in &high_confidence_traits {
            let tl = trait_name.to_lowercase();
            if lower.contains(&format!("not {tl}"))
                || lower.contains(&format!("remove {tl}"))
                || lower.contains(&format!("eliminate {tl}"))
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub(crate) async fn entity_counts(&self) -> Result<serde_json::Value> {
        let mut counts = serde_json::Map::new();
        for entity in &[
            "experience",
            "belief",
            "trait",
            "agent_self",
            "run_record",
            "causal_pattern",
        ] {
            let tx = self
                .driver
                .transaction(&self.db_name, TransactionType::Read)
                .await?;
            let tql = format!("match $x isa {entity}; select $x; count;");
            let count: i64 = match tx.query(&tql).await {
                Ok(answer) => {
                    let mut stream = answer.into_rows();
                    if let Some(Ok(row)) = stream.next().await {
                        row.get("_count")
                            .ok()
                            .flatten()
                            .and_then(|c| c.try_get_integer())
                            .unwrap_or(0)
                    } else {
                        0
                    }
                }
                Err(_) => 0,
            };
            counts.insert(entity.to_string(), serde_json::Value::Number(count.into()));
        }
        Ok(serde_json::Value::Object(counts))
    }
}

fn escape_tql(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
