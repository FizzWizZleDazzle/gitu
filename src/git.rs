use anyhow::{Context, Result};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Commit {
    pub graph: String,
    pub hash: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub filename: String,
    pub diff_content: String,
}

#[derive(Debug, Clone)]
pub struct CommitDiff {
    pub files: Vec<FileDiff>,
}

/// Parses git log output and returns a vector of commits
pub fn get_commits() -> Result<Vec<Commit>> {
    let output = Command::new("git")
        .args(["log", "--graph", "--oneline", "--all", "--decorate"])
        .output()
        .context("Failed to execute git log command")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git log failed: {}", error);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let commits = parse_log_output(&stdout);

    Ok(commits)
}

/// Parses the git log output into structured Commit objects
fn parse_log_output(output: &str) -> Vec<Commit> {
    let mut commits = Vec::new();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }

        // Find where the commit hash starts (after graph characters)
        // Graph characters include: |, *, /, \, space
        let mut hash_start = 0;
        for (i, ch) in line.chars().enumerate() {
            if ch.is_ascii_hexdigit() {
                hash_start = i;
                break;
            }
        }

        if hash_start == 0 && !line.chars().next().unwrap_or(' ').is_ascii_hexdigit() {
            // No hash found, skip this line
            continue;
        }

        let graph = if hash_start > 0 {
            line[..hash_start].to_string()
        } else {
            String::new()
        };

        let rest = &line[hash_start..];

        // Parse hash and message
        // Format is: hash message
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();

        if parts.is_empty() {
            continue;
        }

        let hash = parts[0].to_string();
        let message = if parts.len() > 1 {
            parts[1].to_string()
        } else {
            String::new()
        };

        commits.push(Commit {
            graph,
            hash,
            message,
        });
    }

    commits
}

/// Gets the full diff for a specific commit, split by files
pub fn get_commit_diff(hash: &str) -> Result<CommitDiff> {
    let output = Command::new("git")
        .args(["show", "--color=never", hash])
        .output()
        .context("Failed to execute git show command")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git show failed: {}", error);
    }

    let full_output = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(parse_commit_diff(&full_output))
}

/// Parses the git show output into structured file diffs
fn parse_commit_diff(output: &str) -> CommitDiff {
    let lines: Vec<&str> = output.lines().collect();
    let mut files = Vec::new();
    let mut current_file: Option<FileDiff> = None;

    for line in lines {
        // Detect start of a new file diff
        if line.starts_with("diff --git") {
            // Save the previous file diff if exists
            if let Some(file_diff) = current_file.take() {
                files.push(file_diff);
            }

            // Extract filename from "diff --git a/file b/file"
            let filename = line
                .split_whitespace()
                .nth(2)
                .unwrap_or("unknown")
                .trim_start_matches("a/")
                .to_string();

            current_file = Some(FileDiff {
                filename,
                diff_content: String::new(),
            });
        }

        // Add line to current file
        if let Some(ref mut file_diff) = current_file {
            file_diff.diff_content.push_str(line);
            file_diff.diff_content.push('\n');
        }
    }

    // Don't forget the last file
    if let Some(file_diff) = current_file {
        files.push(file_diff);
    }

    // If no files were found, put everything in a single "diff"
    if files.is_empty() {
        files.push(FileDiff {
            filename: "(complete diff)".to_string(),
            diff_content: output.to_string(),
        });
    }

    CommitDiff { files }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_log() {
        let input = "* abc1234 Initial commit\n* def5678 Second commit";
        let commits = parse_log_output(input);

        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].hash, "abc1234");
        assert_eq!(commits[0].message, "Initial commit");
    }

    #[test]
    fn test_parse_with_graph() {
        let input = "* | abc1234 Merge commit\n|\\ \n| * def5678 Feature branch";
        let commits = parse_log_output(input);

        assert!(commits.len() >= 2);
        assert_eq!(commits[0].hash, "abc1234");
    }
}
