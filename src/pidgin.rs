use crate::{
    coobie::CausalReport,
    models::{CoobieBriefing, FactoryEvent},
};

pub fn prepend_pidgin(pidgin: &str, detail: &str) -> String {
    let pidgin = pidgin.trim();
    let detail = detail.trim();
    match (pidgin.is_empty(), detail.is_empty()) {
        (true, true) => String::new(),
        (false, true) => pidgin.to_string(),
        (true, false) => detail.to_string(),
        (false, false) => format!("{}\n{}", pidgin, detail),
    }
}

pub fn pidgin_summary(event: &FactoryEvent) -> String {
    let mut phrases = Vec::new();
    let lower = event.message.to_lowercase();

    match event.status.as_str() {
        "complete" => phrases.push(if event.agent == "coobie" {
            "thassrealgrate jerry"
        } else {
            "thassgrate jerry"
        }),
        "running" => phrases.push(if event.phase == "twin" {
            "field is weird"
        } else {
            "pack is workin"
        }),
        "warning" => phrases.push(if lower.contains("forced") || lower.contains("failed") {
            "thasrealnotgrate"
        } else {
            "thasnotgrate"
        }),
        "failed" | "error" => phrases.push("thasrealnotgrate"),
        _ => {}
    }

    match event.agent.as_str() {
        "coobie" => {
            if event.status == "running" {
                phrases.push("coobie smells somethin");
            } else if event.status == "complete" {
                if lower.contains("briefing") || lower.contains("causal") || lower.contains("memory") {
                    phrases.push("coobie found the trail");
                }
                if lower.contains("required check") || lower.contains("guardrail") {
                    phrases.push("coobie would try this jerry");
                }
            }
            if lower.contains("confus") || lower.contains("unclear") {
                phrases.push("coobie is confuzd");
            }
        }
        "scout" => {
            if lower.contains("ambigu") || lower.contains("missing") || lower.contains("unclear") {
                phrases.push("thasconfusin jerry");
            } else if event.status == "complete" {
                phrases.push("scout is a real geed dawg");
            }
        }
        "mason" => {
            if event.status == "warning" || lower.contains("fail") {
                phrases.push("mason is not a geed dawg");
            } else if event.status == "complete" {
                phrases.push("mason is a real geed dawg");
            }
        }
        "bramble" => {
            if event.status == "warning" || lower.contains("fail") {
                phrases.push("still not grrrate");
            } else if event.status == "running" {
                phrases.push("gonna try agin");
            }
        }
        "sable" => {
            if event.status == "warning" || lower.contains("hidden") {
                phrases.push("thasrealnotgrate");
            }
        }
        "ash" => {
            if event.status == "running" || lower.contains("twin") {
                phrases.push("field is weird");
            }
            if event.status == "complete" {
                phrases.push("field is clean");
            }
        }
        "flint" => {
            if event.status == "complete" {
                phrases.push("brought it back jerry");
            }
        }
        "keeper" => {
            if event.status == "warning" || event.status == "failed" || lower.contains("policy") {
                phrases.push("keeper says no");
            }
        }
        "piper" => {
            if event.status == "complete" {
                phrases.push("this stick better");
            } else if event.status == "running" {
                phrases.push("need different stick");
            }
        }
        _ => {}
    }

    if lower.contains("complex") || lower.contains("many") {
        phrases.push("too many smells");
    }
    if lower.contains("coordination") || lower.contains("conflict") {
        phrases.push("pack is messy");
    }

    dedupe_join(&phrases)
}

pub fn pidgin_for_agent_result(agent_name: &str, summary: &str, output: &str) -> String {
    let mut phrases = Vec::new();
    let lower = format!("{}\n{}", summary, output).to_lowercase();

    if contains_any(&lower, &["failed", "failure", "missing", "warning", "blocked"]) {
        phrases.push("thasnotgrate");
    } else if contains_any(&lower, &["prepared", "captured", "ready", "provisioned", "stored", "packaged", "passed"]) {
        phrases.push("thassgrate");
    }

    match agent_name {
        "coobie" => {
            if contains_any(&lower, &["causal", "briefing", "trail", "memory"]) {
                phrases.push("coobie found the trail");
            } else {
                phrases.push("coobie smells somethin");
            }
            if contains_any(&lower, &["guardrail", "required check", "intervention", "recommend"]) {
                phrases.push("coobie would try this jerry");
            }
        }
        "scout" => {
            if contains_any(&lower, &["ambigu", "missing", "unclear"]) {
                phrases.push("thasconfusin jerry");
            } else {
                phrases.push("scout is a real geed dawg");
            }
        }
        "mason" => {
            phrases.push(if contains_any(&lower, &["failed", "blocked", "warning"]) {
                "mason is not a geed dawg"
            } else {
                "mason is a real geed dawg"
            });
        }
        "bramble" => {
            phrases.push(if contains_any(&lower, &["failed", "warning"]) {
                "still not grrrate"
            } else {
                "gonna try agin"
            });
        }
        "sable" => phrases.push("thasrealnotgrate"),
        "ash" => phrases.push(if contains_any(&lower, &["gap", "missing", "stub"]) {
            "field is weird"
        } else {
            "field is clean"
        }),
        "flint" => phrases.push("brought it back jerry"),
        "piper" => phrases.push(if contains_any(&lower, &["gap", "missing", "unsupported"]) {
            "need different stick"
        } else {
            "this stick better"
        }),
        "keeper" => phrases.push(if contains_any(&lower, &["policy", "risk", "deny", "violation"]) {
            "keeper says no"
        } else {
            "keeper is a real geed dawg"
        }),
        _ => {}
    }

    dedupe_join(&phrases)
}

pub fn coobie_briefing_pidgin(briefing: &CoobieBriefing) -> String {
    let mut phrases = vec!["coobie smells somethin"];
    if !briefing.prior_causes.is_empty() || !briefing.relevant_lessons.is_empty() {
        phrases.push("coobie found the trail");
    }
    if !briefing.recommended_guardrails.is_empty() || !briefing.required_checks.is_empty() {
        phrases.push("coobie would try this jerry");
    }
    if !briefing.open_questions.is_empty() {
        phrases.push("thasconfusin jerry");
    } else {
        phrases.push("thassrealgrate jerry");
    }
    dedupe_join(&phrases)
}

pub fn coobie_report_pidgin(report: &CausalReport) -> String {
    let mut phrases = vec!["coobie found the trail"];
    if report.primary_cause.is_some() {
        phrases.push("coobie thinks this is why");
    } else {
        phrases.push("coobie lost the trail");
    }
    if !report.recommended_interventions.is_empty() {
        phrases.push("coobie would try this jerry");
    }
    if report.primary_confidence >= 0.75 {
        phrases.push("thassrealgrate jerry");
    } else {
        phrases.push("thasskinda-grate");
    }
    dedupe_join(&phrases)
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn dedupe_join(phrases: &[&str]) -> String {
    let mut out = Vec::new();
    for phrase in phrases {
        if !out.iter().any(|existing: &String| existing == phrase) {
            out.push((*phrase).to_string());
        }
    }
    out.join("\n")
}
