use anyhow::{bail, Context, Result};
use git2::{Delta, DiffOptions, Patch, Repository};

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub origin: char,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub header: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub old_path: String,
    pub new_path: String,
    pub hunks: Vec<Hunk>,
}

pub fn get_file_diff(repo: &Repository, path: &str) -> Result<FileDiff> {
    let mut opts = DiffOptions::new();
    opts.pathspec(path);
    opts.context_lines(3);

    let diff = repo
        .diff_index_to_workdir(None, Some(&mut opts))
        .context("Failed to get diff")?;

    let num_deltas = diff.deltas().len();
    if num_deltas == 0 {
        bail!("No changes found for '{}'", path);
    }

    let patch = Patch::from_diff(&diff, 0)
        .context("Failed to get patch")?
        .ok_or_else(|| anyhow::anyhow!("No patch for '{}'", path))?;

    let delta = diff.deltas().next().unwrap();

    let old_path = delta
        .old_file()
        .path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let new_path = delta
        .new_file()
        .path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let old_path = if delta.status() == Delta::Added {
        "/dev/null".to_string()
    } else {
        format!("a/{old_path}")
    };
    let new_path = if delta.status() == Delta::Deleted {
        "/dev/null".to_string()
    } else {
        format!("b/{new_path}")
    };

    let num_hunks = patch.num_hunks();
    let mut hunks = Vec::with_capacity(num_hunks);

    for hunk_idx in 0..num_hunks {
        let (hunk_header, num_lines) = patch.hunk(hunk_idx)?;
        let header = String::from_utf8_lossy(hunk_header.header()).trim().to_string();

        let mut lines = Vec::new();
        for line_idx in 0..num_lines {
            let line = patch.line_in_hunk(hunk_idx, line_idx)?;
            let origin = line.origin();
            if matches!(origin, '+' | '-' | ' ') {
                let content = String::from_utf8_lossy(line.content()).to_string();
                lines.push(DiffLine { origin, content });
            }
        }

        hunks.push(Hunk {
            header,
            old_start: hunk_header.old_start(),
            old_lines: hunk_header.old_lines(),
            new_start: hunk_header.new_start(),
            new_lines: hunk_header.new_lines(),
            lines,
        });
    }

    if hunks.is_empty() {
        bail!("No changes found for '{}'", path);
    }

    Ok(FileDiff {
        old_path,
        new_path,
        hunks,
    })
}
