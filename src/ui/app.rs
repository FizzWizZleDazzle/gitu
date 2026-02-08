use crate::git::{get_commit_diff, get_commits, Branch, Commit, CommitDiff, SearchFilter, StatusFile, StashEntry};
use anyhow::Result;
use ratatui::widgets::ListState;

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

    // Amend mode
    pub amend_mode: bool,

    // Help popup
    pub help_visible: bool,

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

            // Amend mode
            amend_mode: false,

            // Help popup
            help_visible: false,

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

    pub fn scroll_diff_page_up(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_sub(10);
    }

    pub fn scroll_diff_page_down(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_add(10);
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

    /// Maps a list index (which includes headers) to the actual file index
    /// Returns None if the index points to a header or is out of bounds
    fn list_index_to_file_index(&self, list_idx: usize) -> Option<usize> {
        let (staged, unstaged): (Vec<&StatusFile>, Vec<&StatusFile>) =
            self.status_files.iter().partition(|f| f.staged);

        let mut file_idx = 0;
        let mut current_list_idx = 0;

        // Account for "Staged Changes:" header
        if !staged.is_empty() {
            if list_idx == current_list_idx {
                return None; // This is the header
            }
            current_list_idx += 1;

            // Check if we're in the staged files section
            for _ in &staged {
                if list_idx == current_list_idx {
                    return Some(file_idx);
                }
                current_list_idx += 1;
                file_idx += 1;
            }
        }

        // Account for "Unstaged Changes:" header
        if !unstaged.is_empty() {
            if list_idx == current_list_idx {
                return None; // This is the header
            }
            current_list_idx += 1;

            // Check if we're in the unstaged files section
            for _ in &unstaged {
                if list_idx == current_list_idx {
                    return Some(file_idx);
                }
                current_list_idx += 1;
                file_idx += 1;
            }
        }

        None // Out of bounds
    }

    /// Get the total number of list items (files + headers)
    fn get_status_list_len(&self) -> usize {
        if self.status_files.is_empty() {
            return 1; // "No changes" message
        }

        let (staged, unstaged): (Vec<&StatusFile>, Vec<&StatusFile>) =
            self.status_files.iter().partition(|f| f.staged);

        let mut count = 0;
        if !staged.is_empty() {
            count += 1 + staged.len(); // Header + files
        }
        if !unstaged.is_empty() {
            count += 1 + unstaged.len(); // Header + files
        }
        count
    }

    pub fn next_status_file(&mut self) {
        let list_len = self.get_status_list_len();
        if list_len == 0 {
            return;
        }
        let i = match self.status_list_state.selected() {
            Some(i) if i >= list_len - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.status_list_state.select(Some(i));
    }

    pub fn previous_status_file(&mut self) {
        let list_len = self.get_status_list_len();
        if list_len == 0 {
            return;
        }
        let i = match self.status_list_state.selected() {
            Some(0) => list_len - 1,
            Some(i) => i - 1,
            None => 0,
        };
        self.status_list_state.select(Some(i));
    }

    pub fn toggle_stage(&mut self) {
        if let Some(list_idx) = self.status_list_state.selected() {
            if let Some(file_idx) = self.list_index_to_file_index(list_idx) {
                if let Some(file) = self.status_files.get(file_idx) {
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
        self.amend_mode = false;
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
            self.amend_mode = false;
            return;
        }

        let result = if self.amend_mode {
            crate::git::commit_amend(&self.commit_message_input)
        } else {
            crate::git::commit(&self.commit_message_input)
        };

        match result {
            Ok(msg) => {
                self.set_status(msg, MessageType::Success);
                self.commit_message_mode = false;
                self.amend_mode = false;
                self.refresh_status();
            }
            Err(e) => {
                self.set_status(format!("Error: {}", e), MessageType::Error);
                self.commit_message_mode = false;
                self.amend_mode = false;
            }
        }
    }

    pub fn enter_amend_mode(&mut self) {
        match crate::git::get_last_commit_message() {
            Ok(msg) => {
                self.amend_mode = true;
                self.commit_message_mode = true;
                self.commit_message_input = msg;
            }
            Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
        }
    }

    pub fn discard_selected_file(&mut self) {
        if let Some(list_idx) = self.status_list_state.selected() {
            if let Some(file_idx) = self.list_index_to_file_index(list_idx) {
                if let Some(file) = self.status_files.get(file_idx) {
                if file.staged {
                    self.set_status("Cannot discard staged file. Unstage it first.".to_string(), MessageType::Error);
                    return;
                }

                match crate::git::discard_file(&file.path) {
                    Ok(msg) => {
                        self.set_status(msg, MessageType::Success);
                        self.refresh_status();
                    }
                    Err(e) => self.set_status(format!("Error: {}", e), MessageType::Error),
                }
                }
            }
        }
    }

    pub fn toggle_status_diff(&mut self) {
        self.status_show_diff = !self.status_show_diff;

        if self.status_show_diff {
            // Load diff for selected file
            if let Some(list_idx) = self.status_list_state.selected() {
                if let Some(file_idx) = self.list_index_to_file_index(list_idx) {
                    if let Some(file) = self.status_files.get(file_idx) {
                        match crate::git::get_file_diff(&file.path, file.staged) {
                            Ok(diff) => self.status_diff_content = Some(diff),
                            Err(e) => {
                                self.set_status(format!("Failed to load diff: {}", e), MessageType::Error);
                                self.status_show_diff = false;
                            }
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

    pub fn scroll_status_diff_page_up(&mut self) {
        self.status_diff_scroll = self.status_diff_scroll.saturating_sub(10);
    }

    pub fn scroll_status_diff_page_down(&mut self) {
        self.status_diff_scroll = self.status_diff_scroll.saturating_add(10);
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
            Some(0) => self.stashes.len() - 1,
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
            Some(0) => self.branches.len() - 1,
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

    pub fn merge_selected_branch(&mut self) {
        if let Some(index) = self.branch_list_state.selected() {
            if let Some(branch) = self.branches.get(index) {
                if branch.is_current {
                    self.set_status("Cannot merge a branch into itself".to_string(), MessageType::Error);
                    return;
                }

                match crate::git::merge_branch(&branch.name) {
                    Ok(msg) => {
                        self.set_status(msg, MessageType::Success);
                        self.refresh_branches();
                        self.refresh_status();
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

