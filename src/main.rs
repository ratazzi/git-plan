mod diff;
mod display;
mod filter;
mod stage;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::io::{self, BufRead, Write};

#[derive(Parser)]
#[command(name = "git-plan", about = "Fine-grained git staging tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show diff with hunk labels and line numbers
    Diff {
        /// File path to diff
        file: String,
    },
    /// Stage changes by hunk or line range
    Stage {
        /// File path to stage
        file: String,
        /// Hunks to stage (e.g. a,c)
        #[arg(long)]
        hunks: Option<String>,
        /// Lines to stage (e.g. 3-8,15-20)
        #[arg(long)]
        lines: Option<String>,
        /// Stage entire file
        #[arg(long)]
        all: bool,
    },
}

fn parse_hunk_selection(input: &str) -> Vec<usize> {
    input
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.len() == 1 {
                let c = s.chars().next()?;
                if c.is_ascii_lowercase() {
                    Some((c as usize) - ('a' as usize))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

fn parse_line_selection(input: &str) -> Vec<(usize, usize)> {
    input
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if let Some((start, end)) = s.split_once('-') {
                let start: usize = start.trim().parse().ok()?;
                let end: usize = end.trim().parse().ok()?;
                Some((start, end))
            } else {
                let n: usize = s.parse().ok()?;
                Some((n, n))
            }
        })
        .collect()
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Diff { file } => {
            let repo = git2::Repository::open_from_env()
                .context("Failed to open git repository")?;
            let file_diff = diff::get_file_diff(&repo, &file)?;
            display::print_diff(&file_diff);
        }
        Commands::Stage {
            file,
            hunks,
            lines,
            all,
        } => {
            let repo = git2::Repository::open_from_env()
                .context("Failed to open git repository")?;
            let file_diff = diff::get_file_diff(&repo, &file)?;

            if all {
                let patch = filter::filter_all(&file_diff);
                stage::apply_to_index(&repo, &patch)?;
                println!("Staged all changes in {}", file);
            } else if let Some(hunks_str) = hunks {
                let selected = parse_hunk_selection(&hunks_str);
                if selected.is_empty() {
                    bail!("No valid hunks in '{}'", hunks_str);
                }
                let patch = filter::filter_by_hunks(&file_diff, &selected);
                stage::apply_to_index(&repo, &patch)?;
                println!("Staged selected hunks in {}", file);
            } else if let Some(lines_str) = lines {
                let selected = parse_line_selection(&lines_str);
                if selected.is_empty() {
                    bail!("No valid line ranges in '{}'", lines_str);
                }
                let patch = filter::filter_by_lines(&file_diff, &selected);
                stage::apply_to_index(&repo, &patch)?;
                println!("Staged selected lines in {}", file);
            } else {
                display::print_diff(&file_diff);
                print!("\nStage hunks (e.g. a,b) or lines (e.g. 3-8,15): ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().lock().read_line(&mut input)?;
                let input = input.trim();

                if input.is_empty() {
                    println!("Nothing selected, aborting.");
                    return Ok(());
                }

                let is_hunk_input =
                    input.chars().all(|c| c.is_ascii_lowercase() || c == ',' || c == ' ');

                let patch = if is_hunk_input {
                    let selected = parse_hunk_selection(input);
                    filter::filter_by_hunks(&file_diff, &selected)
                } else {
                    let selected = parse_line_selection(input);
                    filter::filter_by_lines(&file_diff, &selected)
                };

                stage::apply_to_index(&repo, &patch)?;
                println!("Staged selected changes in {}", file);
            }
        }
    }

    Ok(())
}
