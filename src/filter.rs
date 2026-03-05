use crate::diff::{DiffLine, FileDiff};
use std::fmt::Write;

fn write_patch_header(out: &mut String, file_diff: &FileDiff) {
    let a_path = &file_diff.old_path;
    let b_path = &file_diff.new_path;
    writeln!(out, "diff --git {a_path} {b_path}").unwrap();
    writeln!(out, "--- {a_path}").unwrap();
    writeln!(out, "+++ {b_path}").unwrap();
}

fn write_hunk_with_start(
    out: &mut String,
    old_start: u32,
    new_start: u32,
    lines: &[&DiffLine],
) {
    let old_lines = lines
        .iter()
        .filter(|l| matches!(l.origin, '-' | ' '))
        .count() as u32;
    let new_lines = lines
        .iter()
        .filter(|l| matches!(l.origin, '+' | ' '))
        .count() as u32;

    if old_lines == 0 && new_lines == 0 {
        return;
    }

    writeln!(out, "@@ -{old_start},{old_lines} +{new_start},{new_lines} @@").unwrap();

    for line in lines {
        let content = &line.content;
        let has_newline = content.ends_with('\n');
        match line.origin {
            '+' => write!(out, "+{content}").unwrap(),
            '-' => write!(out, "-{content}").unwrap(),
            _ => write!(out, " {content}").unwrap(),
        }
        if !has_newline {
            writeln!(out).unwrap();
        }
    }
}

pub fn filter_all(file_diff: &FileDiff) -> String {
    let mut out = String::new();
    write_patch_header(&mut out, file_diff);
    for hunk in &file_diff.hunks {
        let lines: Vec<&DiffLine> = hunk.lines.iter().collect();
        write_hunk_with_start(&mut out, hunk.old_start, hunk.new_start, &lines);
    }
    out
}

pub fn filter_by_hunks(file_diff: &FileDiff, selected: &[usize]) -> String {
    let mut out = String::new();
    write_patch_header(&mut out, file_diff);

    // Track cumulative offset from skipped hunks
    let mut skipped_offset: i64 = 0;

    for (idx, hunk) in file_diff.hunks.iter().enumerate() {
        let hunk_delta = hunk.new_lines as i64 - hunk.old_lines as i64;
        if selected.contains(&idx) {
            let adjusted_new_start = (hunk.new_start as i64 - skipped_offset) as u32;
            let lines: Vec<&DiffLine> = hunk.lines.iter().collect();
            write_hunk_with_start(&mut out, hunk.old_start, adjusted_new_start, &lines);
        } else {
            skipped_offset += hunk_delta;
        }
    }
    out
}

pub fn filter_by_lines(file_diff: &FileDiff, selected_ranges: &[(usize, usize)]) -> String {
    let mut out = String::new();
    write_patch_header(&mut out, file_diff);

    let is_selected = |line_num: usize| -> bool {
        selected_ranges
            .iter()
            .any(|&(start, end)| line_num >= start && line_num <= end)
    };

    let mut global_line = 1usize;
    let mut offset_adjustment: i64 = 0;

    for hunk in &file_diff.hunks {
        let mut filtered_lines: Vec<DiffLine> = Vec::new();
        let mut hunk_added = 0i64;
        let mut hunk_removed = 0i64;
        let mut hunk_selected_added = 0i64;
        let mut hunk_selected_removed = 0i64;

        for line in &hunk.lines {
            let selected = is_selected(global_line);
            match line.origin {
                '+' => {
                    hunk_added += 1;
                    if selected {
                        hunk_selected_added += 1;
                        filtered_lines.push(line.clone());
                    }
                }
                '-' => {
                    hunk_removed += 1;
                    if selected {
                        hunk_selected_removed += 1;
                        filtered_lines.push(line.clone());
                    } else {
                        filtered_lines.push(DiffLine {
                            origin: ' ',
                            content: line.content.clone(),
                        });
                    }
                }
                _ => {
                    filtered_lines.push(line.clone());
                }
            }
            global_line += 1;
        }

        let adjusted_new_start = (hunk.new_start as i64 - offset_adjustment) as u32;
        let refs: Vec<&DiffLine> = filtered_lines.iter().collect();
        write_hunk_with_start(&mut out, hunk.old_start, adjusted_new_start, &refs);

        // Original hunk delta vs what we actually applied
        let original_delta = hunk_added - hunk_removed;
        let applied_delta = hunk_selected_added - hunk_selected_removed;
        offset_adjustment += original_delta - applied_delta;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::Hunk;

    fn make_line(origin: char, content: &str) -> DiffLine {
        DiffLine {
            origin,
            content: format!("{content}\n"),
        }
    }

    fn make_test_diff() -> FileDiff {
        FileDiff {
            old_path: "a/foo.rs".to_string(),
            new_path: "b/foo.rs".to_string(),
            hunks: vec![
                Hunk {
                    header: "@@ -1,3 +1,4 @@".to_string(),
                    old_start: 1,
                    old_lines: 3,
                    new_start: 1,
                    new_lines: 4,
                    lines: vec![
                        make_line(' ', "fn hello() {"),
                        make_line('+', "    println!(\"world\");"),
                        make_line(' ', "}"),
                    ],
                },
                Hunk {
                    header: "@@ -5,3 +6,4 @@".to_string(),
                    old_start: 5,
                    old_lines: 3,
                    new_start: 6,
                    new_lines: 4,
                    lines: vec![
                        make_line(' ', "fn bye() {"),
                        make_line('+', "    println!(\"bye\");"),
                        make_line(' ', "}"),
                    ],
                },
            ],
        }
    }

    #[test]
    fn filter_by_hunks_selects_first_only() {
        let diff = make_test_diff();
        let patch = filter_by_hunks(&diff, &[0]);
        assert!(patch.contains("println!(\"world\")"));
        assert!(!patch.contains("println!(\"bye\")"));
    }

    #[test]
    fn filter_by_hunks_selects_second_only() {
        let diff = make_test_diff();
        let patch = filter_by_hunks(&diff, &[1]);
        assert!(!patch.contains("println!(\"world\")"));
        assert!(patch.contains("println!(\"bye\")"));
        // new_start adjusted: original 6, hunk a added 1 line (skipped), so 6-1=5
        // old_lines=2, new_lines=3 (recalculated from actual line content)
        assert!(patch.contains("@@ -5,2 +5,3 @@"));
    }

    #[test]
    fn filter_by_lines_selects_added_line() {
        let diff = make_test_diff();
        // line 2 is the '+' in hunk a
        let patch = filter_by_lines(&diff, &[(2, 2)]);
        assert!(patch.contains("println!(\"world\")"));
        // hunk b's '+' at line 5 should not be included
        assert!(!patch.contains("+    println!(\"bye\")"));
    }

    #[test]
    fn filter_by_lines_unselected_minus_becomes_context() {
        let diff = FileDiff {
            old_path: "a/foo.rs".to_string(),
            new_path: "b/foo.rs".to_string(),
            hunks: vec![Hunk {
                header: "@@ -1,3 +1,2 @@".to_string(),
                old_start: 1,
                old_lines: 3,
                new_start: 1,
                new_lines: 2,
                lines: vec![
                    make_line(' ', "keep"),
                    make_line('-', "removed"),
                    make_line(' ', "end"),
                ],
            }],
        };
        // Don't select line 2 (the '-' line) -> should become context
        let patch = filter_by_lines(&diff, &[(1, 1)]);
        assert!(patch.contains(" removed\n"));
        assert!(!patch.contains("-removed"));
    }

    #[test]
    fn filter_all_includes_everything() {
        let diff = make_test_diff();
        let patch = filter_all(&diff);
        assert!(patch.contains("println!(\"world\")"));
        assert!(patch.contains("println!(\"bye\")"));
    }
}
