use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::claude_pack::write_text_file;

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct McpTrigger {
    pub(crate) kind: String,
    pub(crate) value: String,
}

fn default_settings_target() -> String {
    "settings.json".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct McpToml {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    #[serde(default)]
    pub(crate) requires_config: Vec<String>,
    #[serde(default)]
    pub(crate) requires_env: Vec<String>,
    #[serde(default)]
    pub(crate) extra_permissions: Vec<String>,
    #[serde(default = "default_settings_target")]
    pub(crate) settings_target: String,
    #[serde(default)]
    pub(crate) triggers: Vec<McpTrigger>,
}

#[derive(Debug, Clone)]
pub(crate) struct McpEntry {
    pub(crate) toml: McpToml,
}

#[derive(Debug, Clone)]
pub(crate) struct ConfiguredMcpServer {
    pub(crate) name: String,
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) extra_permissions: Vec<String>,
    pub(crate) requires_env: Vec<String>,
    pub(crate) settings_target: String,
}

pub(crate) fn load_mcp_registry(registry_root: &Path) -> Result<Vec<McpEntry>> {
    let mut entries = Vec::new();
    if !registry_root.exists() {
        return Ok(entries);
    }

    let read_dir = fs::read_dir(registry_root)
        .with_context(|| format!("reading MCP registry dir {}", registry_root.display()))?;
    for entry in read_dir {
        let entry = entry?;
        let entry_dir = entry.path();
        if !entry_dir.is_dir() {
            continue;
        }
        let toml_path = entry_dir.join("mcp.toml");
        if !toml_path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&toml_path)
            .with_context(|| format!("reading {}", toml_path.display()))?;
        let toml: McpToml =
            toml::from_str(&raw).with_context(|| format!("parsing {}", toml_path.display()))?;
        entries.push(McpEntry { toml });
    }

    entries.sort_by(|left, right| left.toml.name.cmp(&right.toml.name));
    Ok(entries)
}

pub(crate) fn auto_select_mcps<'a>(
    entries: &'a [McpEntry],
    confirmed_skills: &[String],
    stack_signals: &[String],
) -> Vec<&'a McpEntry> {
    entries
        .iter()
        .filter(|entry| {
            entry
                .toml
                .triggers
                .iter()
                .any(|trigger| mcp_trigger_fires(trigger, confirmed_skills, stack_signals))
        })
        .collect()
}

fn mcp_trigger_fires(
    trigger: &McpTrigger,
    confirmed_skills: &[String],
    stack_signals: &[String],
) -> bool {
    match trigger.kind.as_str() {
        "always" => true,
        "skill" => confirmed_skills
            .iter()
            .any(|skill| skill.eq_ignore_ascii_case(&trigger.value)),
        "file" => stack_signals.iter().any(|signal| {
            signal
                .to_ascii_lowercase()
                .contains(&trigger.value.to_ascii_lowercase())
        }),
        "env_var" => std::env::var(&trigger.value).is_ok(),
        _ => false,
    }
}

pub(crate) fn render_mcp_args(args: &[String], config: &HashMap<String, String>) -> Vec<String> {
    let mut rendered = Vec::new();
    for arg in args {
        let mut value = arg.clone();
        let exact_placeholder = placeholder_name(arg);
        for (key, replacement) in config {
            value = value.replace(&format!("{{{{{key}}}}}"), replacement);
        }

        if let Some(key) = exact_placeholder {
            if let Some(replacement) = config.get(key) {
                let split_values = replacement
                    .split(',')
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>();
                if split_values.len() > 1 {
                    rendered.extend(split_values);
                    continue;
                }
            }
        }

        rendered.push(value);
    }
    rendered
}

fn placeholder_name(arg: &str) -> Option<&str> {
    if arg.starts_with("{{") && arg.ends_with("}}") && arg.len() > 4 {
        Some(&arg[2..arg.len() - 2])
    } else {
        None
    }
}

pub(crate) fn write_mcps_to_settings(
    settings_path: &Path,
    local_settings_path: &Path,
    entries: &[ConfiguredMcpServer],
) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    let mut normal_entries = Vec::new();
    let mut local_entries = Vec::new();
    for entry in entries {
        if entry.settings_target == "settings.local.json" {
            local_entries.push(entry.clone());
        } else {
            normal_entries.push(entry.clone());
        }
    }

    merge_mcp_entries_into_settings(settings_path, &normal_entries)?;
    merge_mcp_entries_into_settings(local_settings_path, &local_entries)?;
    Ok(())
}

fn merge_mcp_entries_into_settings(path: &Path, entries: &[ConfiguredMcpServer]) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    let existing = if path.exists() {
        let raw =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| Value::Object(Map::new()))
    } else {
        Value::Object(Map::new())
    };

    let mut root = match existing {
        Value::Object(map) => map,
        _ => Map::new(),
    };

    {
        let permissions_value = root
            .entry("permissions".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        let permissions_obj = if let Value::Object(obj) = permissions_value {
            obj
        } else {
            *permissions_value = Value::Object(Map::new());
            permissions_value
                .as_object_mut()
                .expect("permissions object just inserted")
        };

        let allow_value = permissions_obj
            .entry("allow".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        let allow_arr = if let Value::Array(arr) = allow_value {
            arr
        } else {
            *allow_value = Value::Array(Vec::new());
            allow_value
                .as_array_mut()
                .expect("allow array just inserted")
        };

        for entry in entries {
            for permission in &entry.extra_permissions {
                let permission_value = Value::String(permission.clone());
                if !allow_arr.contains(&permission_value) {
                    allow_arr.push(permission_value);
                }
            }
        }
    }

    let mcp_servers_value = root
        .entry("mcpServers".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let mcp_servers_obj = if let Value::Object(obj) = mcp_servers_value {
        obj
    } else {
        *mcp_servers_value = Value::Object(Map::new());
        mcp_servers_value
            .as_object_mut()
            .expect("mcpServers object just inserted")
    };

    for entry in entries {
        mcp_servers_obj.insert(
            entry.name.clone(),
            serde_json::json!({
                "command": entry.command,
                "args": entry.args,
            }),
        );
    }

    let rendered = serde_json::to_string_pretty(&Value::Object(root))?;
    write_text_file(path, &rendered)
}
