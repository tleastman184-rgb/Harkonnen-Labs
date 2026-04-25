use anyhow::{bail, Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use crate::claude_pack::write_text_file;

const CLONE_TIMEOUT_SECS: u64 = 30;

pub(crate) async fn list_skills_in_repo(clone_url: &str) -> Result<Vec<String>> {
    let temp_dir = temp_clone_dir("list");
    let result = async {
        clone_repo(clone_url, &temp_dir).await?;
        let discovered = discover_skill_sources(&temp_dir)?;
        let mut names = discovered.into_keys().collect::<Vec<_>>();
        names.sort();
        Ok(names)
    }
    .await;
    cleanup_temp_dir(&temp_dir);
    result
}

pub(crate) async fn fetch_skills_from_repo(
    clone_url: &str,
    target_repo_path: &Path,
    skill_names: &[String],
) -> Result<Vec<String>> {
    let temp_dir = temp_clone_dir("fetch");
    let result = async {
        clone_repo(clone_url, &temp_dir).await?;
        let discovered = discover_skill_sources(&temp_dir)?;
        let requested = if skill_names.is_empty() {
            discovered.keys().cloned().collect::<Vec<_>>()
        } else {
            skill_names.to_vec()
        };

        let mut copied = Vec::new();
        for name in requested {
            let Some(source_path) = discovered.get(&name) else {
                continue;
            };
            let target_path = target_repo_path
                .join(".claude")
                .join("skills")
                .join(&name)
                .join("SKILL.md");
            let content = fs::read_to_string(source_path)
                .with_context(|| format!("reading {}", source_path.display()))?;
            write_text_file(&target_path, &content)?;
            copied.push(name);
        }
        copied.sort();
        Ok(copied)
    }
    .await;
    cleanup_temp_dir(&temp_dir);
    result
}

async fn clone_repo(clone_url: &str, temp_dir: &Path) -> Result<()> {
    let parent = temp_dir
        .parent()
        .ok_or_else(|| anyhow::anyhow!("temporary clone dir has no parent"))?;
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;

    let mut command = Command::new("git");
    command
        .arg("clone")
        .arg("--depth=1")
        .arg(clone_url)
        .arg(temp_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    let output = timeout(Duration::from_secs(CLONE_TIMEOUT_SECS), command.output())
        .await
        .with_context(|| format!("timed out cloning {clone_url} after {CLONE_TIMEOUT_SECS}s"))?
        .with_context(|| format!("spawning git clone for {clone_url}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            bail!("git clone failed for {clone_url}");
        }
        bail!("git clone failed for {clone_url}: {stderr}");
    }
    Ok(())
}

fn discover_skill_sources(root: &Path) -> Result<HashMap<String, PathBuf>> {
    let mut files = Vec::new();
    collect_skill_files(root, &mut files)?;

    let mut selected = HashMap::<String, (usize, PathBuf)>::new();
    for file in files {
        let name = infer_skill_name(&file)?;
        let score = discovery_score(&file);
        match selected.get(&name) {
            Some((existing_score, _)) if *existing_score >= score => {}
            _ => {
                selected.insert(name, (score, file));
            }
        }
    }

    Ok(selected
        .into_iter()
        .map(|(name, (_, path))| (name, path))
        .collect())
}

fn collect_skill_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|value| value.to_str()) == Some(".git") {
                continue;
            }
            collect_skill_files(&path, files)?;
        } else if path.file_name().and_then(|value| value.to_str()) == Some("SKILL.md") {
            files.push(path);
        }
    }
    Ok(())
}

fn infer_skill_name(path: &Path) -> Result<String> {
    let components = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str().map(|value| value.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    if let Some(index) = components.iter().position(|value| value == ".claude") {
        if components.get(index + 1).map(String::as_str) == Some("skills") {
            if let Some(name) = components.get(index + 2) {
                return Ok(name.clone());
            }
        }
    }

    if let Some(index) = components
        .iter()
        .position(|value| value == "skill-registry")
    {
        if let Some(name) = components.get(index + 2) {
            return Ok(name.clone());
        }
    }

    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|value| value.to_str())
        .map(|value| value.to_string())
        .ok_or_else(|| anyhow::anyhow!("could not infer skill name from {}", path.display()))
}

fn discovery_score(path: &Path) -> usize {
    let rendered = path.to_string_lossy();
    if rendered.contains(".claude/skills/") {
        3
    } else if rendered.contains("skill-registry/") {
        2
    } else {
        1
    }
}

fn temp_clone_dir(label: &str) -> PathBuf {
    let suffix = Utc::now().timestamp_nanos_opt().unwrap_or_default();
    std::env::temp_dir().join(format!("harkonnen-skill-fetch-{label}-{suffix}"))
}

fn cleanup_temp_dir(temp_dir: &Path) {
    if let Err(error) = fs::remove_dir_all(temp_dir) {
        if temp_dir.exists() {
            eprintln!(
                "warning: failed to clean temporary skill fetch dir {}: {}",
                temp_dir.display(),
                error
            );
        }
    }
}
