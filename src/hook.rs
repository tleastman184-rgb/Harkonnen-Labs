/// Deterministic enforcement hooks for Claude Code's PreToolUse / PostToolUse events.
///
/// Each function corresponds to one hook event. The binary is called by the thin
/// wrapper scripts in .claude/hooks/; jq-based bash logic in those scripts is the
/// fallback when the binary has not been built yet.
///
/// Exit behaviour:
///   - All functions return Ok(()) on success (caller exits 0).
///   - Blocking guards call std::process::exit(2) directly so the stderr message
///     reaches Claude Code as the reason the tool call was blocked.
///   - Non-blocking hooks (memory gate, audit) always return Ok(()).
use anyhow::Result;
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use std::io::{Read, Write as IoWrite};
use std::path::{Path, PathBuf};

// ── Input ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct HookInput {
    pub tool_name: Option<String>,
    pub tool_input: Option<Value>,
    pub tool_response: Option<Value>,
}

impl HookInput {
    pub fn file_path(&self) -> Option<&str> {
        self.tool_input.as_ref()?.get("file_path")?.as_str()
    }

    pub fn command(&self) -> Option<&str> {
        self.tool_input.as_ref()?.get("command")?.as_str()
    }
}

pub fn read_stdin_input() -> HookInput {
    let mut buf = String::new();
    let _ = std::io::stdin().read_to_string(&mut buf);
    serde_json::from_str(&buf).unwrap_or_default()
}

// ── Project dir resolution ────────────────────────────────────────────────────

pub fn resolve_project_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CLAUDE_PROJECT_DIR") {
        return PathBuf::from(dir);
    }
    // walk up looking for harkonnen.toml
    if let Ok(mut dir) = std::env::current_dir() {
        loop {
            if dir.join("harkonnen.toml").exists() {
                return dir;
            }
            if !dir.pop() {
                break;
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

// ── Pre-write: sable guard + memory gate ─────────────────────────────────────

/// Blocks writes to `factory/scenarios/hidden/` and logs writes to `factory/memory/`.
/// This is the most critical enforcement point in the system: Sable's isolation
/// invariant I-SB-01/I-SB-02 must hold even if the system prompt is ignored.
pub fn run_pre_write(input: &HookInput, project_dir: &Path) -> Result<()> {
    sable_guard(input);
    memory_gate(input, project_dir)?;
    Ok(())
}

fn sable_guard(input: &HookInput) {
    let Some(path) = input.file_path() else {
        return;
    };
    if !path.contains("factory/scenarios/hidden") {
        return;
    }
    eprintln!("BLOCKED [sable-guard]: Direct writes to factory/scenarios/hidden/ are forbidden.");
    eprintln!();
    eprintln!("Hidden scenarios may only be created via the Sable scenario-generation flow.");
    eprintln!("Writing directly contaminates the evaluation corpus and violates Sable's");
    eprintln!("isolation invariant (I-SB-01 / I-SB-02 in factory/agents/contracts/sable.yaml).");
    std::process::exit(2);
}

fn memory_gate(input: &HookInput, project_dir: &Path) -> Result<()> {
    let Some(path) = input.file_path() else {
        return Ok(());
    };
    if !path.contains("factory/memory/") {
        return Ok(());
    }
    append_jsonl(
        &project_dir.join("factory/artifacts/memory-write-audit.jsonl"),
        serde_json::json!({
            "event": "memory_write",
            "timestamp": Utc::now().to_rfc3339(),
            "tool": input.tool_name.as_deref().unwrap_or("unknown"),
            "file": path,
        }),
    )
}

// ── Pre-bash: destructive command guard ───────────────────────────────────────

const BLOCKED_PATTERNS: &[&str] = &[
    "rm -rf",
    "rm -fr",
    "git push --force",
    "git push -f",
    "git reset --hard",
    "git clean -f",
    "DROP TABLE",
    "DROP DATABASE",
    "DROP SCHEMA",
    "> factory/state.db",
];

/// Blocks known-destructive shell patterns. Exit 2 surfaces the reason to Claude.
pub fn run_pre_bash(input: &HookInput) -> Result<()> {
    let Some(command) = input.command() else {
        return Ok(());
    };
    for pattern in BLOCKED_PATTERNS {
        if command.contains(pattern) {
            eprintln!("BLOCKED [bash-guard]: Destructive command pattern detected.");
            eprintln!("  Pattern: '{pattern}'");
            eprintln!("  Command: {}", &command[..command.len().min(200)]);
            eprintln!();
            eprintln!("If this is intentional, confirm explicitly with the operator before");
            eprintln!("re-attempting. Hard stops are listed in src/hook.rs.");
            std::process::exit(2);
        }
    }
    Ok(())
}

// ── Post-format: rustfmt on .rs files ────────────────────────────────────────

/// Runs `rustfmt --edition 2021` on the edited file if it is a Rust source file.
pub fn run_post_format(input: &HookInput) -> Result<()> {
    let Some(path) = input.file_path() else {
        return Ok(());
    };
    if !path.ends_with(".rs") {
        return Ok(());
    }
    let _ = std::process::Command::new("rustfmt")
        .args(["--edition", "2021", path])
        .status();
    Ok(())
}

// ── Post-audit: structured tool audit log ────────────────────────────────────

/// Appends one JSONL entry per tool call to `factory/artifacts/tool-audit.jsonl`.
/// This is the compliance trail for Keeper policy enforcement and the sub-agent
/// dispatch observability layer.
pub fn run_post_audit(input: &HookInput, project_dir: &Path) -> Result<()> {
    let action = input
        .file_path()
        .map(str::to_owned)
        .or_else(|| {
            input
                .command()
                .map(|c| c.chars().take(200).collect::<String>())
        })
        .unwrap_or_else(|| "(no action captured)".into());

    append_jsonl(
        &project_dir.join("factory/artifacts/tool-audit.jsonl"),
        serde_json::json!({
            "timestamp": Utc::now().to_rfc3339(),
            "tool": input.tool_name.as_deref().unwrap_or("unknown"),
            "action": action,
        }),
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn append_jsonl(path: &Path, entry: Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", entry)?;
    Ok(())
}
