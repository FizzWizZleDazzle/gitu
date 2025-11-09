use anyhow::{Context, Result};
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
}

#[derive(Debug, Clone)]
pub struct StatusFile {
    pub path: String,
    pub status: FileStatus,
    pub staged: bool,
}

#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub branch: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decoration {
    Head,
    Branch(String),
    RemoteBranch(String),
    Tag(String),
}

#[derive(Debug, Clone)]
pub struct Commit {
    pub graph: String,
    pub hash: String,
    pub message: String,
    pub decorations: Vec<Decoration>,
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

/// Search filter type for git log
#[derive(Debug, Clone, PartialEq)]
pub enum SearchFilter {
    Message(String),
    Author(String),
}

/// Parses git log output and returns a vector of commits
pub fn get_commits(filter: Option<&SearchFilter>) -> Result<Vec<Commit>> {
    let mut args = vec!["log", "--graph", "--oneline", "--all", "--decorate"];

    // Add search filter arguments
    let filter_arg;
    match filter {
        Some(SearchFilter::Message(query)) => {
            filter_arg = format!("--grep={}", query);
            args.push(&filter_arg);
        }
        Some(SearchFilter::Author(query)) => {
            filter_arg = format!("--author={}", query);
            args.push(&filter_arg);
        }
        None => {}
    }

    let output = Command::new("git")
        .args(&args)
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

        // Parse decorations and message
        // Format can be: "hash (decorations) message" or "hash message"
        let rest_after_hash = if parts.len() > 1 { parts[1] } else { "" };
        let (decorations, message) = parse_decorations_and_message(rest_after_hash);

        commits.push(Commit {
            graph,
            hash,
            message,
            decorations,
        });
    }

    commits
}

/// Parses decorations and message from the text after the hash
/// Input format: "(HEAD -> main, tag: v1.0) Commit message" or just "Commit message"
fn parse_decorations_and_message(text: &str) -> (Vec<Decoration>, String) {
    let text = text.trim();

    // Check if there are decorations (starts with '(')
    if !text.starts_with('(') {
        return (Vec::new(), text.to_string());
    }

    // Find the closing parenthesis
    if let Some(close_paren) = text.find(')') {
        let decoration_str = &text[1..close_paren]; // Skip opening '('
        let message = text[close_paren + 1..].trim().to_string();

        let decorations = parse_decoration_string(decoration_str);
        (decorations, message)
    } else {
        // Malformed, treat as message
        (Vec::new(), text.to_string())
    }
}

/// Parses a decoration string like "HEAD -> main, origin/main, tag: v1.0"
fn parse_decoration_string(decoration_str: &str) -> Vec<Decoration> {
    let mut decorations = Vec::new();

    for part in decoration_str.split(',') {
        let part = part.trim();

        if part.is_empty() {
            continue;
        }

        // Handle "HEAD -> branch" format
        if part.starts_with("HEAD -> ") {
            decorations.push(Decoration::Head);
            let branch = part[8..].trim(); // Skip "HEAD -> "
            if !branch.is_empty() {
                if branch.contains('/') {
                    decorations.push(Decoration::RemoteBranch(branch.to_string()));
                } else {
                    decorations.push(Decoration::Branch(branch.to_string()));
                }
            }
        }
        // Handle "HEAD" alone
        else if part == "HEAD" {
            decorations.push(Decoration::Head);
        }
        // Handle "tag: name" format
        else if part.starts_with("tag: ") {
            let tag_name = part[5..].trim(); // Skip "tag: "
            if !tag_name.is_empty() {
                decorations.push(Decoration::Tag(tag_name.to_string()));
            }
        }
        // Handle remote branches (contain '/')
        else if part.contains('/') {
            decorations.push(Decoration::RemoteBranch(part.to_string()));
        }
        // Handle local branches
        else {
            decorations.push(Decoration::Branch(part.to_string()));
        }
    }

    decorations
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
    let mut found_first_diff = false;

    for line in lines {
        // Skip everything before the first "diff --git" line
        if !found_first_diff && !line.starts_with("diff --git") {
            continue;
        }

        // Detect start of a new file diff
        if line.starts_with("diff --git") {
            found_first_diff = true;

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

        // Add line to current file (skip the "diff --git" line itself and metadata)
        if let Some(ref mut file_diff) = current_file {
            // Skip diff metadata lines, only keep the actual diff content
            if !line.starts_with("diff --git")
                && !line.starts_with("index ")
                && !line.starts_with("--- ")
                && !line.starts_with("+++ ")
            {
                file_diff.diff_content.push_str(line);
                file_diff.diff_content.push('\n');
            }
        }
    }

    // Don't forget the last file
    if let Some(file_diff) = current_file {
        files.push(file_diff);
    }

    // If no files were found, show a message
    if files.is_empty() {
        files.push(FileDiff {
            filename: "(no changes)".to_string(),
            diff_content: "No file changes in this commit.\n".to_string(),
        });
    }

    CommitDiff { files }
}

/// Checkout a specific commit (detached HEAD state)
pub fn checkout_commit(hash: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["checkout", hash])
        .output()
        .context("Failed to execute git checkout")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Checkout failed: {}", error);
    }

    Ok(format!("Checked out commit {} (detached HEAD)", &hash[..7]))
}

/// Create a new branch from a commit and check it out
pub fn create_branch(branch_name: &str, hash: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["checkout", "-b", branch_name, hash])
        .output()
        .context("Failed to execute git checkout -b")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Create branch failed: {}", error);
    }

    Ok(format!("Created and checked out branch '{}'", branch_name))
}

/// Cherry-pick a commit
pub fn cherry_pick(hash: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["cherry-pick", hash])
        .output()
        .context("Failed to execute git cherry-pick")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);

        // Check if it's a conflict
        if error.contains("conflict") || error.contains("CONFLICT") {
            return Ok(format!(
                "Cherry-pick has conflicts. Resolve them and run 'git cherry-pick --continue'"
            ));
        }

        anyhow::bail!("Cherry-pick failed: {}", error);
    }

    Ok(format!("Cherry-picked commit {}", &hash[..7]))
}

/// Revert a commit
pub fn revert_commit(hash: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["revert", "--no-edit", hash])
        .output()
        .context("Failed to execute git revert")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);

        // Check if it's a conflict
        if error.contains("conflict") || error.contains("CONFLICT") {
            return Ok(format!(
                "Revert has conflicts. Resolve them and run 'git revert --continue'"
            ));
        }

        anyhow::bail!("Revert failed: {}", error);
    }

    Ok(format!("Reverted commit {}", &hash[..7]))
}

/// Get git status (staged and unstaged files)
pub fn get_status() -> Result<Vec<StatusFile>> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to execute git status")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git status failed: {}", error);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_status_output(&stdout))
}

/// Parse git status --porcelain output
fn parse_status_output(output: &str) -> Vec<StatusFile> {
    let mut files = Vec::new();

    for line in output.lines() {
        if line.len() < 3 {
            continue;
        }

        let staged_char = line.chars().next().unwrap();
        let unstaged_char = line.chars().nth(1).unwrap();
        let path = line[3..].to_string();

        // Handle staged files
        if staged_char != ' ' && staged_char != '?' {
            let status = match staged_char {
                'M' => FileStatus::Modified,
                'A' => FileStatus::Added,
                'D' => FileStatus::Deleted,
                'R' => FileStatus::Renamed,
                _ => FileStatus::Modified,
            };

            files.push(StatusFile {
                path: path.clone(),
                status,
                staged: true,
            });
        }

        // Handle unstaged files
        if unstaged_char != ' ' {
            let status = match unstaged_char {
                'M' => FileStatus::Modified,
                'D' => FileStatus::Deleted,
                _ => FileStatus::Modified,
            };

            files.push(StatusFile {
                path: path.clone(),
                status,
                staged: false,
            });
        }

        // Handle untracked files
        if staged_char == '?' && unstaged_char == '?' {
            files.push(StatusFile {
                path,
                status: FileStatus::Untracked,
                staged: false,
            });
        }
    }

    files
}

/// Get list of stashes
pub fn get_stashes() -> Result<Vec<StashEntry>> {
    let output = Command::new("git")
        .args(["stash", "list"])
        .output()
        .context("Failed to execute git stash list")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git stash list failed: {}", error);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_stash_output(&stdout))
}

/// Parse git stash list output
fn parse_stash_output(output: &str) -> Vec<StashEntry> {
    let mut stashes = Vec::new();

    for (index, line) in output.lines().enumerate() {
        // Format: stash@{0}: WIP on branch: message
        // or: stash@{0}: On branch: message

        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() < 2 {
            continue;
        }

        let rest = parts[1].trim();
        let (branch, message) = if rest.starts_with("WIP on ") {
            let rest = &rest[7..];
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() == 2 {
                (parts[0].trim().to_string(), parts[1].trim().to_string())
            } else {
                ("unknown".to_string(), rest.to_string())
            }
        } else if rest.starts_with("On ") {
            let rest = &rest[3..];
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() == 2 {
                (parts[0].trim().to_string(), parts[1].trim().to_string())
            } else {
                ("unknown".to_string(), rest.to_string())
            }
        } else {
            ("unknown".to_string(), rest.to_string())
        };

        stashes.push(StashEntry {
            index,
            branch,
            message,
        });
    }

    stashes
}

/// Stage a file
pub fn stage_file(path: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["add", path])
        .output()
        .context("Failed to execute git add")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Staging failed: {}", error);
    }

    Ok(format!("Staged: {}", path))
}

/// Unstage a file
pub fn unstage_file(path: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["reset", "HEAD", path])
        .output()
        .context("Failed to execute git reset")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Unstaging failed: {}", error);
    }

    Ok(format!("Unstaged: {}", path))
}

/// Stage all files
pub fn stage_all() -> Result<String> {
    let output = Command::new("git")
        .args(["add", "."])
        .output()
        .context("Failed to execute git add .")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Staging all failed: {}", error);
    }

    Ok("Staged all files".to_string())
}

/// Unstage all files
pub fn unstage_all() -> Result<String> {
    let output = Command::new("git")
        .args(["reset", "HEAD"])
        .output()
        .context("Failed to execute git reset")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Unstaging all failed: {}", error);
    }

    Ok("Unstaged all files".to_string())
}

/// Commit with a message
pub fn commit(message: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["commit", "-m", message])
        .output()
        .context("Failed to execute git commit")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Commit failed: {}", error);
    }

    Ok("Committed successfully".to_string())
}

/// Apply a stash
pub fn apply_stash(index: usize) -> Result<String> {
    let stash_ref = format!("stash@{{{}}}", index);
    let output = Command::new("git")
        .args(["stash", "apply", &stash_ref])
        .output()
        .context("Failed to execute git stash apply")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Stash apply failed: {}", error);
    }

    Ok(format!("Applied stash@{{{}}}", index))
}

/// Pop a stash (apply and remove)
pub fn pop_stash(index: usize) -> Result<String> {
    let stash_ref = format!("stash@{{{}}}", index);
    let output = Command::new("git")
        .args(["stash", "pop", &stash_ref])
        .output()
        .context("Failed to execute git stash pop")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Stash pop failed: {}", error);
    }

    Ok(format!("Popped stash@{{{}}}", index))
}

/// Drop a stash
pub fn drop_stash(index: usize) -> Result<String> {
    let stash_ref = format!("stash@{{{}}}", index);
    let output = Command::new("git")
        .args(["stash", "drop", &stash_ref])
        .output()
        .context("Failed to execute git stash drop")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Stash drop failed: {}", error);
    }

    Ok(format!("Dropped stash@{{{}}}", index))
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
