use crate::diff::FileDiff;
use colored::Colorize;

pub fn print_diff(file_diff: &FileDiff) {
    println!("{}", format!("--- {}", file_diff.old_path).red());
    println!("{}", format!("+++ {}", file_diff.new_path).green());

    let mut line_num = 1usize;

    for (idx, hunk) in file_diff.hunks.iter().enumerate() {
        let label = (b'a' + idx as u8) as char;
        println!();
        println!(
            "{}",
            format!("[{label}] {}", hunk.header).cyan().bold()
        );

        for diff_line in &hunk.lines {
            let num_str = format!("{:>3}", line_num);
            let line_content = diff_line.content.trim_end_matches('\n');
            match diff_line.origin {
                '+' => {
                    println!("{} | {}", num_str.green(), format!("+{line_content}").green());
                }
                '-' => {
                    println!("{} | {}", num_str.red(), format!("-{line_content}").red());
                }
                _ => {
                    println!("{} |  {}", num_str, line_content);
                }
            }
            line_num += 1;
        }
    }
}
