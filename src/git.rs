use anyhow::{Context, Result};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Commit {
    pub graph: String,
    pub hash: String,
    pub message: String,
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

/// Gets the full diff for a specific commit
pub fn get_commit_diff(hash: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["show", "--color=never", hash])
        .output()
        .context("Failed to execute git show command")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git show failed: {}", error);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
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
