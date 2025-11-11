use crate::git::{get_commit_diff, get_commits, Branch, Commit, CommitDiff, Decoration, SearchFilter, StatusFile, StashEntry};
use crate::syntax;
use anyhow::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Panel {
    Status,
    Log,
    Stash,
    Branches,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    Success,
    Error,
    Info,
}

pub struct App {
    // Panel system
    pub current_panel: Panel,

    // Log panel (existing functionality)
    pub commits: Vec<Commit>,
    pub list_state: ListState,
    pub show_diff: bool,
    pub current_diff: Option<CommitDiff>,
    pub diff_scroll: u16,
    pub file_list_state: ListState,
    pub search_mode: bool,
    pub search_query: String,
    pub active_filter: Option<SearchFilter>,
    pub tree_view_mode: bool,
    pub tree_file_selected: bool,

    // Status panel
    pub status_files: Vec<StatusFile>,
    pub status_list_state: ListState,
    pub commit_message_mode: bool,
    pub commit_message_input: String,
    pub status_show_diff: bool,
    pub status_diff_content: Option<String>,
    pub status_diff_scroll: u16,

    // Stash panel
    pub stashes: Vec<StashEntry>,
    pub stash_list_state: ListState,
    pub stash_input_mode: bool,
    pub stash_message_input: String,

    // Branches panel
    pub branches: Vec<Branch>,
    pub branch_list_state: ListState,
    pub new_branch_input_mode: bool,
    pub new_branch_name_input: String,

    // Common
    pub should_quit: bool,
    pub branch_input_mode: bool,
    pub branch_name_input: String,
    pub status_message: Option<String>,
    pub status_message_type: MessageType,
}

impl App {
    pub fn new(commits: Vec<Commit>) -> Self {
        let mut list_state = ListState::default();
        if !commits.is_empty() {
            list_state.select(Some(0));
        }

        // Try to load status, stash, and branch data
        let status_files = crate::git::get_status().unwrap_or_default();
        let stashes = crate::git::get_stashes().unwrap_or_default();
        let branches = crate::git::get_branches().unwrap_or_default();

        let mut status_list_state = ListState::default();
        if !status_files.is_empty() {
            status_list_state.select(Some(0));
        }

        let mut stash_list_state = ListState::default();
        if !stashes.is_empty() {
            stash_list_state.select(Some(0));
        }

        let mut branch_list_state = ListState::default();
        if !branches.is_empty() {
            branch_list_state.select(Some(0));
        }

        Self {
            current_panel: Panel::Status,

            // Log panel
            commits,
            list_state,
            show_diff: false,
            current_diff: None,
            diff_scroll: 0,
            file_list_state: ListState::default(),
            search_mode: false,
            search_query: String::new(),
            active_filter: None,
            tree_view_mode: false,
            tree_file_selected: false,

            // Status panel
            status_files,
            status_list_state,
            commit_message_mode: false,
            commit_message_input: String::new(),
            status_show_diff: false,
            status_diff_content: None,
            status_diff_scroll: 0,

            // Stash panel
            stashes,
            stash_list_state,
            stash_input_mode: false,
            stash_message_input: String::new(),

            // Branches panel
            branches,
            branch_list_state,
            new_branch_input_mode: false,
            new_branch_name_input: String::new(),

            // Common
            should_quit: false,
            branch_input_mode: false,
            branch_name_input: String::new(),
            status_message: None,
            status_message_type: MessageType::Info,
        }
    }

    pub fn next(&mut self) {
        if self.commits.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.commits.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.diff_scroll = 0;
    }

    pub fn previous(&mut self) {
        if self.commits.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.commits.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.diff_scroll = 0;
    }

    pub fn scroll_diff_up(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_sub(1);
    }

    pub fn scroll_diff_down(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_add(1);
    }

    pub fn next_file(&mut self) {
        if let Some(ref diff) = self.current_diff {
            if diff.files.is_empty() {
                return;
            }

            let i = match self.file_list_state.selected() {
                Some(i) => {
                    if i >= diff.files.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.file_list_state.select(Some(i));
            self.diff_scroll = 0;
        }
    }

    pub fn previous_file(&mut self) {
        if let Some(ref diff) = self.current_diff {
            if diff.files.is_empty() {
                return;
            }

            let i = match self.file_list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        diff.files.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.file_list_state.select(Some(i));
            self.diff_scroll = 0;
        }
    }

    pub fn toggle_diff(&mut self) -> Result<()> {
        if self.show_diff {
            self.show_diff = false;
            self.current_diff = None;
            self.diff_scroll = 0;
            self.file_list_state.select(None);
        } else if let Some(index) = self.list_state.selected() {
            let commit = &self.commits[index];
            let diff = get_commit_diff(&commit.hash)?;

            // Select the first file by default
            let mut file_state = ListState::default();
            if !diff.files.is_empty() {
                file_state.select(Some(0));
            }

            self.current_diff = Some(diff);
            self.show_diff = true;
            self.diff_scroll = 0;
            self.file_list_state = file_state;
        }
        Ok(())
    }

    pub fn quit(&mut self) {
        if self.show_diff {
            self.show_diff = false;
            self.current_diff = None;
            self.diff_scroll = 0;
            self.file_list_state.select(None);
        } else {
            self.should_quit = true;
        }
    }

    pub fn enter_search_mode(&mut self) {
        self.search_mode = true;
        self.search_query.clear();
    }

    pub fn exit_search_mode(&mut self) {
        self.search_mode = false;
    }

    pub fn add_search_char(&mut self, c: char) {
        self.search_query.push(c);
    }

    pub fn delete_search_char(&mut self) {
        self.search_query.pop();
    }

    pub fn execute_search(&mut self) -> Result<()> {
        if self.search_query.is_empty() {
            // Empty query = clear filter
            self.active_filter = None;
        } else if self.search_query.starts_with('@') {
            // Author search
            let author = self.search_query[1..].to_string();
            self.active_filter = Some(SearchFilter::Author(author));
        } else {
            // Message search
            self.active_filter = Some(SearchFilter::Message(self.search_query.clone()));
        }

        // Reload commits with the filter
        self.commits = get_commits(self.active_filter.as_ref())?;

        // Reset selection
        let mut list_state = ListState::default();
        if !self.commits.is_empty() {
            list_state.select(Some(0));
        }
        self.list_state = list_state;

        self.search_mode = false;
        Ok(())
    }

    pub fn clear_search(&mut self) -> Result<()> {
        self.active_filter = None;
        self.search_query.clear();
        self.commits = get_commits(None)?;

        // Reset selection
        let mut list_state = ListState::default();
        if !self.commits.is_empty() {
            list_state.select(Some(0));
        }
        self.list_state = list_state;

        Ok(())
    }

    pub fn toggle_tree_view(&mut self) -> Result<()> {
        if self.tree_view_mode {
            // Already in tree view, exit it
            self.tree_view_mode = false;
            self.tree_file_selected = false;
            self.current_diff = None;
            self.file_list_state.select(None);
            self.diff_scroll = 0;
        } else {
            // Enter tree view mode
            if let Some(index) = self.list_state.selected() {
                let commit = &self.commits[index];
                let diff = get_commit_diff(&commit.hash)?;

                // Select the first file by default
                let mut file_state = ListState::default();
                if !diff.files.is_empty() {
                    file_state.select(Some(0));
                }

                self.current_diff = Some(diff);
                self.file_list_state = file_state;
                self.tree_view_mode = true;
                self.tree_file_selected = false;
                self.diff_scroll = 0;
            }
        }
        Ok(())
    }

    pub fn next_tree_file(&mut self) {
        if let Some(ref diff) = self.current_diff {
            if diff.files.is_empty() {
                return;
            }

            let i = match self.file_list_state.selected() {
                Some(i) => {
                    if i >= diff.files.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.file_list_state.select(Some(i));
        }
    }

    pub fn previous_tree_file(&mut self) {
        if let Some(ref diff) = self.current_diff {
            if diff.files.is_empty() {
                return;
            }

            let i = match self.file_list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        diff.files.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.file_list_state.select(Some(i));
        }
    }

    pub fn select_tree_file(&mut self) {
        // Toggle between showing the file list and showing the selected file's diff
        self.tree_file_selected = !self.tree_file_selected;
        self.diff_scroll = 0;
    }

    pub fn exit_tree_view(&mut self) {
        if self.tree_file_selected {
            // If viewing a file, go back to file list
            self.tree_file_selected = false;
            self.diff_scroll = 0;
        } else {
            // If viewing file list, exit tree view entirely
            self.tree_view_mode = false;
            self.current_diff = None;
            self.file_list_state.select(None);
        }
    }

    pub fn set_status(&mut self, message: String, message_type: MessageType) {
        self.status_message = Some(message);
        self.status_message_type = message_type;
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    pub fn copy_commit_hash(&mut self) {
        if let Some(index) = self.list_state.selected() {
            let commit = &self.commits[index];
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => {
                    if let Err(e) = clipboard.set_text(&commit.hash) {
                        self.set_status(
                            format!("Failed to copy to clipboard: {}", e),
                            MessageType::Error,
                        );
                    } else {
                        self.set_status(
                            format!("Copied hash: {}", commit.hash),
                            MessageType::Success,
                        );
                    }
                }
                Err(e) => {
                    self.set_status(
                        format!("Failed to access clipboard: {}", e),
                        MessageType::Error,
                    );
                }
            }
        }
    }

    pub fn checkout_selected_commit(&mut self) {
        if let Some(index) = self.list_state.selected() {
            let commit = &self.commits[index];
            match crate::git::checkout_commit(&commit.hash) {
                Ok(msg) => self.set_status(msg, MessageType::Success),
                Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
            }
        }
    }

    pub fn enter_branch_input_mode(&mut self) {
        self.branch_input_mode = true;
        self.branch_name_input.clear();
    }

    pub fn exit_branch_input_mode(&mut self) {
        self.branch_input_mode = false;
    }

    pub fn add_branch_char(&mut self, c: char) {
        self.branch_name_input.push(c);
    }

    pub fn delete_branch_char(&mut self) {
        self.branch_name_input.pop();
    }

    pub fn create_branch_from_commit(&mut self) {
        if self.branch_name_input.is_empty() {
            self.set_status("Branch name cannot be empty".to_string(), MessageType::Error);
            self.branch_input_mode = false;
            return;
        }

        if let Some(index) = self.list_state.selected() {
            let commit = &self.commits[index];
            match crate::git::create_branch(&self.branch_name_input, &commit.hash) {
                Ok(msg) => {
                    self.set_status(msg, MessageType::Success);
                    self.branch_input_mode = false;
                }
                Err(e) => {
                    self.set_status(format!("Error: {}", e), MessageType::Error);
                    self.branch_input_mode = false;
                }
            }
        }
    }

    pub fn cherry_pick_commit(&mut self) {
        if let Some(index) = self.list_state.selected() {
            let commit = &self.commits[index];
            match crate::git::cherry_pick(&commit.hash) {
                Ok(msg) => self.set_status(msg, MessageType::Info),
                Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
            }
        }
    }

    pub fn revert_selected_commit(&mut self) {
        if let Some(index) = self.list_state.selected() {
            let commit = &self.commits[index];
            match crate::git::revert_commit(&commit.hash) {
                Ok(msg) => self.set_status(msg, MessageType::Info),
                Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
            }
        }
    }

    // Panel navigation
    pub fn switch_to_panel(&mut self, panel: Panel) {
        self.current_panel = panel;
    }

    pub fn refresh_status(&mut self) {
        match crate::git::get_status() {
            Ok(files) => {
                self.status_files = files;
                let mut state = ListState::default();
                if !self.status_files.is_empty() {
                    state.select(Some(0));
                }
                self.status_list_state = state;
            }
            Err(e) => self.set_status(format!("Failed to refresh status: {}", e), MessageType::Error),
        }
    }

    pub fn refresh_stashes(&mut self) {
        match crate::git::get_stashes() {
            Ok(stashes) => {
                self.stashes = stashes;
                let mut state = ListState::default();
                if !self.stashes.is_empty() {
                    state.select(Some(0));
                }
                self.stash_list_state = state;
            }
            Err(e) => self.set_status(format!("Failed to refresh stashes: {}", e), MessageType::Error),
        }
    }

    // Status panel operations
    pub fn next_status_file(&mut self) {
        if self.status_files.is_empty() {
            return;
        }
        let i = match self.status_list_state.selected() {
            Some(i) if i >= self.status_files.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.status_list_state.select(Some(i));
    }

    pub fn previous_status_file(&mut self) {
        if self.status_files.is_empty() {
            return;
        }
        let i = match self.status_list_state.selected() {
            Some(i) if i == 0 => self.status_files.len() - 1,
            Some(i) => i - 1,
            None => 0,
        };
        self.status_list_state.select(Some(i));
    }

    pub fn toggle_stage(&mut self) {
        if let Some(index) = self.status_list_state.selected() {
            if let Some(file) = self.status_files.get(index) {
                let result = if file.staged {
                    crate::git::unstage_file(&file.path)
                } else {
                    crate::git::stage_file(&file.path)
                };

                match result {
                    Ok(msg) => {
                        self.set_status(msg, MessageType::Success);
                        self.refresh_status();
                    }
                    Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
                }
            }
        }
    }

    pub fn stage_all_files(&mut self) {
        match crate::git::stage_all() {
            Ok(msg) => {
                self.set_status(msg, MessageType::Success);
                self.refresh_status();
            }
            Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
        }
    }

    pub fn unstage_all_files(&mut self) {
        match crate::git::unstage_all() {
            Ok(msg) => {
                self.set_status(msg, MessageType::Success);
                self.refresh_status();
            }
            Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
        }
    }

    pub fn enter_commit_message_mode(&mut self) {
        self.commit_message_mode = true;
        self.commit_message_input.clear();
    }

    pub fn exit_commit_message_mode(&mut self) {
        self.commit_message_mode = false;
    }

    pub fn add_commit_char(&mut self, c: char) {
        self.commit_message_input.push(c);
    }

    pub fn delete_commit_char(&mut self) {
        self.commit_message_input.pop();
    }

    pub fn execute_commit(&mut self) {
        if self.commit_message_input.is_empty() {
            self.set_status("Commit message cannot be empty".to_string(), MessageType::Error);
            self.commit_message_mode = false;
            return;
        }

        match crate::git::commit(&self.commit_message_input) {
            Ok(msg) => {
                self.set_status(msg, MessageType::Success);
                self.commit_message_mode = false;
                self.refresh_status();
            }
            Err(e) => {
                self.set_status(format!("Error: {}", e), MessageType::Error);
                self.commit_message_mode = false;
            }
        }
    }

    pub fn toggle_status_diff(&mut self) {
        self.status_show_diff = !self.status_show_diff;

        if self.status_show_diff {
            // Load diff for selected file
            if let Some(index) = self.status_list_state.selected() {
                if let Some(file) = self.status_files.get(index) {
                    match crate::git::get_file_diff(&file.path, file.staged) {
                        Ok(diff) => self.status_diff_content = Some(diff),
                        Err(e) => {
                            self.set_status(format!("Failed to load diff: {}", e), MessageType::Error);
                            self.status_show_diff = false;
                        }
                    }
                }
            }
        } else {
            self.status_diff_content = None;
            self.status_diff_scroll = 0;
        }
    }

    pub fn scroll_status_diff_up(&mut self) {
        if self.status_diff_scroll > 0 {
            self.status_diff_scroll -= 1;
        }
    }

    pub fn scroll_status_diff_down(&mut self) {
        self.status_diff_scroll += 1;
    }

    // Stash panel operations
    pub fn next_stash(&mut self) {
        if self.stashes.is_empty() {
            return;
        }
        let i = match self.stash_list_state.selected() {
            Some(i) if i >= self.stashes.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.stash_list_state.select(Some(i));
    }

    pub fn previous_stash(&mut self) {
        if self.stashes.is_empty() {
            return;
        }
        let i = match self.stash_list_state.selected() {
            Some(i) if i == 0 => self.stashes.len() - 1,
            Some(i) => i - 1,
            None => 0,
        };
        self.stash_list_state.select(Some(i));
    }

    pub fn apply_selected_stash(&mut self) {
        if let Some(index) = self.stash_list_state.selected() {
            if let Some(stash) = self.stashes.get(index) {
                match crate::git::apply_stash(stash.index) {
                    Ok(msg) => {
                        self.set_status(msg, MessageType::Success);
                        self.refresh_status();
                    }
                    Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
                }
            }
        }
    }

    pub fn pop_selected_stash(&mut self) {
        if let Some(index) = self.stash_list_state.selected() {
            if let Some(stash) = self.stashes.get(index) {
                match crate::git::pop_stash(stash.index) {
                    Ok(msg) => {
                        self.set_status(msg, MessageType::Success);
                        self.refresh_status();
                        self.refresh_stashes();
                    }
                    Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
                }
            }
        }
    }

    pub fn drop_selected_stash(&mut self) {
        if let Some(index) = self.stash_list_state.selected() {
            if let Some(stash) = self.stashes.get(index) {
                match crate::git::drop_stash(stash.index) {
                    Ok(msg) => {
                        self.set_status(msg, MessageType::Success);
                        self.refresh_stashes();
                    }
                    Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
                }
            }
        }
    }

    // Stash creation methods
    pub fn enter_stash_input_mode(&mut self) {
        self.stash_input_mode = true;
        self.stash_message_input.clear();
    }

    pub fn exit_stash_input_mode(&mut self) {
        self.stash_input_mode = false;
    }

    pub fn add_stash_char(&mut self, c: char) {
        self.stash_message_input.push(c);
    }

    pub fn delete_stash_char(&mut self) {
        self.stash_message_input.pop();
    }

    pub fn execute_create_stash(&mut self) {
        let message = if self.stash_message_input.is_empty() {
            None
        } else {
            Some(self.stash_message_input.as_str())
        };

        match crate::git::create_stash(message, false) {
            Ok(msg) => {
                self.set_status(msg, MessageType::Success);
                self.stash_input_mode = false;
                self.refresh_status();
                self.refresh_stashes();
            }
            Err(e) => {
                self.set_status(format!("Error: {}", e), MessageType::Error);
                self.stash_input_mode = false;
            }
        }
    }

    // Branches panel operations
    pub fn refresh_branches(&mut self) {
        match crate::git::get_branches() {
            Ok(branches) => {
                self.branches = branches;
                let mut state = ListState::default();
                if !self.branches.is_empty() {
                    state.select(Some(0));
                }
                self.branch_list_state = state;
            }
            Err(e) => self.set_status(format!("Failed to refresh branches: {}", e), MessageType::Error),
        }
    }

    pub fn next_branch(&mut self) {
        if self.branches.is_empty() {
            return;
        }
        let i = match self.branch_list_state.selected() {
            Some(i) if i >= self.branches.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.branch_list_state.select(Some(i));
    }

    pub fn previous_branch(&mut self) {
        if self.branches.is_empty() {
            return;
        }
        let i = match self.branch_list_state.selected() {
            Some(i) if i == 0 => self.branches.len() - 1,
            Some(i) => i - 1,
            None => 0,
        };
        self.branch_list_state.select(Some(i));
    }

    pub fn switch_to_selected_branch(&mut self) {
        if let Some(index) = self.branch_list_state.selected() {
            if let Some(branch) = self.branches.get(index) {
                if branch.is_current {
                    self.set_status("Already on this branch".to_string(), MessageType::Info);
                    return;
                }

                match crate::git::switch_branch(&branch.name) {
                    Ok(msg) => {
                        self.set_status(msg, MessageType::Success);
                        self.refresh_branches();
                    }
                    Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
                }
            }
        }
    }

    pub fn delete_selected_branch(&mut self) {
        if let Some(index) = self.branch_list_state.selected() {
            if let Some(branch) = self.branches.get(index) {
                if branch.is_current {
                    self.set_status("Cannot delete current branch".to_string(), MessageType::Error);
                    return;
                }

                if branch.is_remote {
                    self.set_status("Cannot delete remote branches from this view".to_string(), MessageType::Error);
                    return;
                }

                match crate::git::delete_branch(&branch.name, false) {
                    Ok(msg) => {
                        self.set_status(msg, MessageType::Success);
                        self.refresh_branches();
                    }
                    Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
                }
            }
        }
    }

    pub fn enter_new_branch_mode(&mut self) {
        self.new_branch_input_mode = true;
        self.new_branch_name_input.clear();
    }

    pub fn exit_new_branch_mode(&mut self) {
        self.new_branch_input_mode = false;
    }

    pub fn add_new_branch_char(&mut self, c: char) {
        self.new_branch_name_input.push(c);
    }

    pub fn delete_new_branch_char(&mut self) {
        self.new_branch_name_input.pop();
    }

    pub fn execute_create_new_branch(&mut self) {
        if self.new_branch_name_input.is_empty() {
            self.set_status("Branch name cannot be empty".to_string(), MessageType::Error);
            self.new_branch_input_mode = false;
            return;
        }

        match crate::git::create_new_branch(&self.new_branch_name_input) {
            Ok(msg) => {
                self.set_status(msg, MessageType::Success);
                self.new_branch_input_mode = false;
                self.refresh_branches();
            }
            Err(e) => {
                self.set_status(format!("Error: {}", e), MessageType::Error);
                self.new_branch_input_mode = false;
            }
        }
    }

    // Remote operations
    pub fn fetch_from_remote(&mut self) {
        match crate::git::fetch() {
            Ok(msg) => {
                self.set_status(msg, MessageType::Success);
                self.refresh_branches();
            }
            Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
        }
    }

    pub fn push_to_remote(&mut self) {
        match crate::git::push(false) {
            Ok(msg) => self.set_status(msg, MessageType::Success),
            Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
        }
    }

    pub fn pull_from_remote(&mut self) {
        match crate::git::pull(false) {
            Ok(msg) => {
                self.set_status(msg, MessageType::Success);
                self.refresh_status();
                self.refresh_branches();
            }
            Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
        }
    }
}

pub fn ui(f: &mut Frame, app: &mut App) {
    // Calculate constraints based on what needs to be shown
    let has_status_msg = app.status_message.is_some();
    let has_input = app.search_mode || app.branch_input_mode || app.commit_message_mode || app.stash_input_mode || app.new_branch_input_mode;

    let mut constraints = vec![];
    if has_status_msg {
        constraints.push(Constraint::Length(1)); // Status message
    }
    constraints.push(Constraint::Length(1)); // Tab bar
    constraints.push(Constraint::Min(3));    // Main content
    if has_input {
        constraints.push(Constraint::Length(3)); // Input prompt
    }

    let root_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    let mut chunk_idx = 0;
    let status_area = if has_status_msg {
        let area = root_chunks[chunk_idx];
        chunk_idx += 1;
        Some(area)
    } else {
        None
    };

    let tab_area = root_chunks[chunk_idx];
    chunk_idx += 1;

    let main_area = root_chunks[chunk_idx];
    chunk_idx += 1;

    let input_area = if has_input && chunk_idx < root_chunks.len() {
        Some(root_chunks[chunk_idx])
    } else {
        None
    };

    // Render components
    if let Some(status_rect) = status_area {
        render_status_message(f, app, status_rect);
    }

    render_tab_bar(f, app, tab_area);

    // Render appropriate panel
    match app.current_panel {
        Panel::Status => render_status_panel(f, app, main_area),
        Panel::Log => render_log_panel(f, app, main_area),
        Panel::Stash => render_stash_panel(f, app, main_area),
        Panel::Branches => render_branches_panel(f, app, main_area),
    }

    // Render input prompts
    if let Some(input_rect) = input_area {
        if app.search_mode {
            render_search_input(f, app, input_rect);
        } else if app.branch_input_mode {
            render_branch_input(f, app, input_rect);
        } else if app.commit_message_mode {
            render_commit_message_input(f, app, input_rect);
        } else if app.stash_input_mode {
            render_stash_input(f, app, input_rect);
        } else if app.new_branch_input_mode {
            render_new_branch_input(f, app, input_rect);
        }
    }
}

fn render_tab_bar(f: &mut Frame, app: &App, area: Rect) {
    let tabs = vec![
        ("[1] Status", Panel::Status),
        ("[2] Log", Panel::Log),
        ("[3] Stash", Panel::Stash),
        ("[4] Branches", Panel::Branches),
    ];

    let mut spans = Vec::new();
    for (i, (label, panel)) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" | "));
        }

        let style = if *panel == app.current_panel {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        spans.push(Span::styled(label.to_string(), style));
    }

    let line = Line::from(spans);
    f.render_widget(Paragraph::new(line), area);
}

fn render_status_panel(f: &mut Frame, app: &mut App, area: Rect) {
    // Split area if showing diff
    let chunks = if app.status_show_diff {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(area)
    };

    let (staged, unstaged): (Vec<&StatusFile>, Vec<&StatusFile>) =
        app.status_files.iter().partition(|f| f.staged);

    let items: Vec<ListItem> = {
        let mut items = Vec::new();

        if !staged.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "Staged Changes:",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ))));

            for file in &staged {
                let status_char = match file.status {
                    crate::git::FileStatus::Modified => "M",
                    crate::git::FileStatus::Added => "A",
                    crate::git::FileStatus::Deleted => "D",
                    crate::git::FileStatus::Renamed => "R",
                    crate::git::FileStatus::Untracked => "?",
                };

                items.push(ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("[{}] ", status_char),
                        Style::default().fg(Color::Green),
                    ),
                    Span::raw(&file.path),
                ])));
            }
        }

        if !unstaged.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "Unstaged Changes:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ))));

            for file in &unstaged {
                let status_char = match file.status {
                    crate::git::FileStatus::Modified => "M",
                    crate::git::FileStatus::Added => "A",
                    crate::git::FileStatus::Deleted => "D",
                    crate::git::FileStatus::Renamed => "R",
                    crate::git::FileStatus::Untracked => "?",
                };

                items.push(ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("[{}] ", status_char),
                        Style::default().fg(Color::Red),
                    ),
                    Span::raw(&file.path),
                ])));
            }
        }

        if items.is_empty() {
            items.push(ListItem::new("No changes"));
        }

        items
    };

    let title = format!(" Status ({} files) ", app.status_files.len());
    let help = if app.status_show_diff {
        " j/k: Scroll diff | Enter: Hide diff | Space: Stage/Unstage "
    } else {
        " Space: Stage/Unstage | a: Stage all | u: Unstage all | c: Commit | s: Stash | Enter: Show diff "
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_bottom(help),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[0], &mut app.status_list_state);

    // Render diff if showing
    if app.status_show_diff && chunks.len() > 1 {
        if let Some(ref diff_content) = app.status_diff_content {
            let selected_file = app.status_list_state.selected()
                .and_then(|i| app.status_files.get(i));

            let filename = selected_file
                .map(|f| f.path.as_str())
                .unwrap_or("unknown");

            let lines = crate::syntax::highlight_diff(diff_content, filename);

            let visible_lines: Vec<Line> = lines
                .into_iter()
                .skip(app.status_diff_scroll as usize)
                .collect();

            let paragraph = Paragraph::new(visible_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" Diff: {} ", filename)),
                )
                .wrap(ratatui::widgets::Wrap { trim: false });

            f.render_widget(paragraph, chunks[1]);
        }
    }
}

fn render_log_panel(f: &mut Frame, app: &mut App, area: Rect) {
    // Split based on view mode
    let chunks = if app.tree_view_mode {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area)
    } else if app.show_diff {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(25),
                Constraint::Percentage(45),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(area)
    };

    render_commit_list(f, app, chunks[0]);

    if app.tree_view_mode && chunks.len() >= 2 {
        if app.tree_file_selected {
            render_tree_file_diff(f, app, chunks[1]);
        } else {
            render_tree_file_list(f, app, chunks[1]);
        }
    } else if app.show_diff && chunks.len() >= 3 {
        render_file_list(f, app, chunks[1]);
        render_diff(f, app, chunks[2]);
    }
}

fn render_stash_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .stashes
        .iter()
        .map(|stash| {
            let line = Line::from(vec![
                Span::styled(
                    format!("stash@{{{}}}", stash.index),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" on "),
                Span::styled(&stash.branch, Style::default().fg(Color::Cyan)),
                Span::raw(": "),
                Span::raw(&stash.message),
            ]);
            ListItem::new(line)
        })
        .collect();

    let items = if items.is_empty() {
        vec![ListItem::new("No stashes")]
    } else {
        items
    };

    let title = format!(" Stashes ({}) ", app.stashes.len());
    let help = " a: Apply | p: Pop | d: Drop | q: Quit ";

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_bottom(help),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.stash_list_state);
}

fn render_branches_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let (local, remote): (Vec<&Branch>, Vec<&Branch>) =
        app.branches.iter().partition(|b| !b.is_remote);

    let items: Vec<ListItem> = {
        let mut items = Vec::new();

        if !local.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "Local Branches:",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ))));

            for branch in &local {
                let mut spans = vec![];

                if branch.is_current {
                    spans.push(Span::styled("* ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
                } else {
                    spans.push(Span::raw("  "));
                }

                spans.push(Span::styled(
                    &branch.name,
                    if branch.is_current {
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ));

                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    &branch.commit_hash[..7.min(branch.commit_hash.len())],
                    Style::default().fg(Color::Yellow),
                ));

                if !branch.commit_message.is_empty() {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(&branch.commit_message, Style::default().fg(Color::Gray)));
                }

                items.push(ListItem::new(Line::from(spans)));
            }
        }

        if !remote.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "Remote Branches:",
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
            ))));

            for branch in &remote {
                let mut spans = vec![Span::raw("  ")];

                spans.push(Span::styled(&branch.name, Style::default().fg(Color::Blue)));
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    &branch.commit_hash[..7.min(branch.commit_hash.len())],
                    Style::default().fg(Color::Yellow),
                ));

                if !branch.commit_message.is_empty() {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(&branch.commit_message, Style::default().fg(Color::Gray)));
                }

                items.push(ListItem::new(Line::from(spans)));
            }
        }

        if items.is_empty() {
            items.push(ListItem::new("No branches"));
        }

        items
    };

    let title = format!(" Branches ({}) ", app.branches.len());
    let help = " Enter: Switch | d: Delete | n: New branch | q: Quit ";

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_bottom(help),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.branch_list_state);
}

fn render_commit_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .commits
        .iter()
        .map(|commit| {
            let mut spans = vec![
                Span::styled(&commit.graph, Style::default().fg(Color::Cyan)),
                Span::styled(&commit.hash, Style::default().fg(Color::Yellow)),
                Span::raw(" "),
            ];

            // Add decoration pills
            for decoration in &commit.decorations {
                spans.extend(render_decoration(decoration));
                spans.push(Span::raw(" "));
            }

            // Add commit message
            spans.push(Span::raw(&commit.message));

            let line = Line::from(spans);
            ListItem::new(line)
        })
        .collect();

    let title = if let Some(ref filter) = app.active_filter {
        let filter_str = match filter {
            SearchFilter::Message(q) => format!("grep: {}", q),
            SearchFilter::Author(q) => format!("author: {}", q),
        };
        format!(" Git Log ({} commits) [{}] ", app.commits.len(), filter_str)
    } else {
        format!(" Git Log ({} commits) ", app.commits.len())
    };

    let help = if app.show_diff {
        " Enter: Close | q: Quit "
    } else if app.tree_view_mode {
        " t: Exit tree view | q: Quit "
    } else if app.active_filter.is_some() {
        " ↑/↓: Nav | Enter: View | t: Tree | /: Search | Esc: Clear | q: Quit "
    } else {
        " ↑/↓: Nav | Enter: View | t: Tree view | /: Search | q: Quit "
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_bottom(help),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

/// Renders a decoration as styled spans (pills)
fn render_decoration(decoration: &Decoration) -> Vec<Span<'static>> {
    match decoration {
        Decoration::Head => vec![Span::styled(
            "HEAD".to_string(),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )],
        Decoration::Branch(name) => vec![
            Span::styled(
                "[".to_string(),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                name.clone(),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "]".to_string(),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ],
        Decoration::RemoteBranch(name) => vec![
            Span::styled(
                "[".to_string(),
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                name.clone(),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "]".to_string(),
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
            ),
        ],
        Decoration::Tag(name) => vec![
            Span::styled(
                "(".to_string(),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                name.clone(),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                ")".to_string(),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ],
    }
}

fn render_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    if let Some(ref diff) = app.current_diff {
        let items: Vec<ListItem> = diff
            .files
            .iter()
            .map(|file| {
                let line = Line::from(Span::raw(&file.filename));
                ListItem::new(line)
            })
            .collect();

        let title = format!(" Files ({}) ", diff.files.len());
        let help = " ←/→: Switch File ";

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .title_bottom(help),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut app.file_list_state);
    }
}

fn render_diff(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref commit_diff) = app.current_diff {
        let selected_file_index = app.file_list_state.selected().unwrap_or(0);

        let file_diff = commit_diff.files.get(selected_file_index);
        let diff_content = file_diff.map(|f| f.diff_content.as_str()).unwrap_or("");
        let filename = file_diff.map(|f| f.filename.as_str()).unwrap_or("");

        // Apply syntax highlighting to the diff
        let all_highlighted_lines = syntax::highlight_diff(diff_content, filename);

        // Apply scroll offset
        let diff_lines: Vec<Line> = all_highlighted_lines
            .into_iter()
            .skip(app.diff_scroll as usize)
            .collect();

        let title = format!(" {} ", filename);
        let help = " ↑/↓: Scroll | ESC: Close ";

        let paragraph = Paragraph::new(diff_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .title_bottom(help),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }
}

fn render_search_input(f: &mut Frame, app: &App, area: Rect) {
    let search_type = if app.search_query.starts_with('@') {
        "Author Search"
    } else {
        "Message Search"
    };

    let help = " Type to search | @ prefix for author | Enter: Apply | Esc: Cancel ";

    let input_text = if app.search_query.is_empty() {
        "Type to search commits...".to_string()
    } else {
        app.search_query.clone()
    };

    let input_style = if app.search_query.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let paragraph = Paragraph::new(input_text)
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", search_type))
                .title_bottom(help)
                .border_style(Style::default().fg(Color::Yellow)),
        );

    f.render_widget(paragraph, area);
}

fn render_commit_message_input(f: &mut Frame, app: &App, area: Rect) {
    let help = " Type commit message | Enter: Commit | Esc: Cancel ";

    let input_text = if app.commit_message_input.is_empty() {
        "Enter commit message...".to_string()
    } else {
        app.commit_message_input.clone()
    };

    let input_style = if app.commit_message_input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let paragraph = Paragraph::new(input_text)
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Commit Message ")
                .title_bottom(help)
                .border_style(Style::default().fg(Color::Green)),
        );

    f.render_widget(paragraph, area);
}

fn render_stash_input(f: &mut Frame, app: &App, area: Rect) {
    let help = " Type stash message (optional) | Enter: Create stash | Esc: Cancel ";

    let input_text = if app.stash_message_input.is_empty() {
        "Enter stash message (optional)...".to_string()
    } else {
        app.stash_message_input.clone()
    };

    let input_style = if app.stash_message_input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let paragraph = Paragraph::new(input_text)
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Stash Message ")
                .title_bottom(help)
                .border_style(Style::default().fg(Color::Magenta)),
        );

    f.render_widget(paragraph, area);
}

fn render_new_branch_input(f: &mut Frame, app: &App, area: Rect) {
    let help = " Type branch name | Enter: Create | Esc: Cancel ";

    let input_text = if app.new_branch_name_input.is_empty() {
        "Enter new branch name...".to_string()
    } else {
        app.new_branch_name_input.clone()
    };

    let input_style = if app.new_branch_name_input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let paragraph = Paragraph::new(input_text)
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" New Branch ")
                .title_bottom(help)
                .border_style(Style::default().fg(Color::Green)),
        );

    f.render_widget(paragraph, area);
}

fn render_tree_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    if let Some(ref diff) = app.current_diff {
        let items: Vec<ListItem> = diff
            .files
            .iter()
            .map(|file| {
                // Add a change indicator
                let indicator = "M"; // Could parse from git for A/M/D
                let line = Line::from(vec![
                    Span::styled(
                        format!("[{}] ", indicator),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(&file.filename),
                ]);
                ListItem::new(line)
            })
            .collect();

        let title = format!(" Files Changed ({}) ", diff.files.len());
        let help = " ↑/↓: Navigate | Enter: View File | Esc: Close | t: Toggle view ";

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .title_bottom(help),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut app.file_list_state);
    }
}

fn render_tree_file_diff(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref commit_diff) = app.current_diff {
        let selected_file_index = app.file_list_state.selected().unwrap_or(0);

        let file_diff = commit_diff.files.get(selected_file_index);
        let diff_content = file_diff.map(|f| f.diff_content.as_str()).unwrap_or("");
        let filename = file_diff.map(|f| f.filename.as_str()).unwrap_or("");

        // Apply syntax highlighting to the diff
        let all_highlighted_lines = syntax::highlight_diff(diff_content, filename);

        // Apply scroll offset
        let diff_lines: Vec<Line> = all_highlighted_lines
            .into_iter()
            .skip(app.diff_scroll as usize)
            .collect();

        let title = format!(" {} ", filename);
        let help = " ↑/↓: Scroll | Esc: Back to file list ";

        let paragraph = Paragraph::new(diff_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .title_bottom(help),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }
}

fn render_status_message(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref message) = app.status_message {
        let style = match app.status_message_type {
            MessageType::Success => Style::default().fg(Color::Black).bg(Color::Green),
            MessageType::Error => Style::default().fg(Color::White).bg(Color::Red),
            MessageType::Info => Style::default().fg(Color::Black).bg(Color::Yellow),
        };

        let span = Span::styled(format!(" {} ", message), style);
        f.render_widget(Paragraph::new(span), area);
    }
}

fn render_branch_input(f: &mut Frame, app: &App, area: Rect) {
    let help = " Type branch name | Enter: Create | Esc: Cancel ";

    let input_text = if app.branch_name_input.is_empty() {
        "Enter new branch name...".to_string()
    } else {
        app.branch_name_input.clone()
    };

    let input_style = if app.branch_name_input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let paragraph = Paragraph::new(input_text)
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Create Branch ")
                .title_bottom(help)
                .border_style(Style::default().fg(Color::Green)),
        );

    f.render_widget(paragraph, area);
}
