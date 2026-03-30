use anyhow::{bail, Result};
use std::path::Path;

pub fn ensure_path_within(base: &Path, candidate: &Path) -> Result<()> {
    let base = base.canonicalize()?;
    let candidate = candidate.canonicalize()?;
    if !candidate.starts_with(&base) {
        bail!("path escapes allowed workspace");
    }
    Ok(())
}
