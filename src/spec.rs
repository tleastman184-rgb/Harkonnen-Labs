use anyhow::{Context, Result};
use std::fs;

use crate::models::Spec;

pub fn load_spec(path: &str) -> Result<Spec> {
    let raw = fs::read_to_string(path).with_context(|| format!("failed to read spec: {path}"))?;
    let spec: Spec =
        serde_yaml::from_str(&raw).with_context(|| format!("failed to parse yaml spec: {path}"))?;
    Ok(spec)
}
