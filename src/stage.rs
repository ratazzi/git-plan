use anyhow::{Context, Result};
use git2::{ApplyLocation, Diff, Repository};

pub fn apply_to_index(repo: &Repository, patch: &str) -> Result<()> {
    let diff =
        Diff::from_buffer(patch.as_bytes()).context("Failed to parse patch")?;
    repo.apply(&diff, ApplyLocation::Index, None)
        .context("Failed to apply patch to index")?;
    Ok(())
}
