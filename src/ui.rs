use crate::git::{get_commit_diff, get_commits, Commit, CommitDiff, Decoration, SearchFilter, StatusFile, StashEntry};
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    Success,
    Error,
    Info,
}

pub struct App {
    pub commits: Vec<Commit>,
    pub list_state: ListState,
    pub should_quit: bool,
    pub show_diff: bool,
    pub current_diff: Option<CommitDiff>,
    pub diff_scroll: u16,
    pub file_list_state: ListState,
    pub search_mode: bool,
    pub search_query: String,
    pub active_filter: Option<SearchFilter>,
    pub tree_view_mode: bool,
    pub tree_file_selected: bool,
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

        Self {
            commits,
            list_state,
            should_quit: false,
            show_diff: false,
            current_diff: None,
            diff_scroll: 0,
            file_list_state: ListState::default(),
            search_mode: false,
            search_query: String::new(),
            active_filter: None,
            tree_view_mode: false,
            tree_file_selected: false,
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
}

pub fn ui(f: &mut Frame, app: &mut App) {
    // Split vertically for status message, main area, and input prompts
    let root_chunks = if app.status_message.is_some() {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Status message
                Constraint::Min(3),     // Main area
                Constraint::Length(if app.search_mode || app.branch_input_mode { 3 } else { 0 }),
            ])
            .split(f.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),     // Main area
                Constraint::Length(if app.search_mode || app.branch_input_mode { 3 } else { 0 }),
            ])
            .split(f.area())
    };

    let (status_area, main_area, input_area) = if app.status_message.is_some() {
        (Some(root_chunks[0]), root_chunks[1], if root_chunks.len() > 2 { Some(root_chunks[2]) } else { None })
    } else {
        (None, root_chunks[0], if root_chunks.len() > 1 { Some(root_chunks[1]) } else { None })
    };

    // Split main area horizontally based on view mode
    let chunks = if app.tree_view_mode {
        // Tree view mode: 2-pane layout [commits | files or diff]
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(main_area)
    } else if app.show_diff {
        // Normal diff view: 3-pane layout [commits | files | diff]
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(25),
                Constraint::Percentage(45),
            ])
            .split(main_area)
    } else {
        // Commit list only
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(main_area)
    };

    render_commit_list(f, app, chunks[0]);

    if app.tree_view_mode && chunks.len() >= 2 {
        // Tree view mode
        if app.tree_file_selected {
            // Showing selected file's diff full-width
            render_tree_file_diff(f, app, chunks[1]);
        } else {
            // Showing file list full-width
            render_tree_file_list(f, app, chunks[1]);
        }
    } else if app.show_diff && chunks.len() >= 3 {
        // Normal diff view (3-pane)
        render_file_list(f, app, chunks[1]);
        render_diff(f, app, chunks[2]);
    }

    // Render status message if present
    if let Some(status_rect) = status_area {
        render_status_message(f, app, status_rect);
    }

    // Render input prompts
    if let Some(input_rect) = input_area {
        if app.search_mode {
            render_search_input(f, app, input_rect);
        } else if app.branch_input_mode {
            render_branch_input(f, app, input_rect);
        }
    }
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
