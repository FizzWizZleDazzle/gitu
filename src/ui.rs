use crate::git::{get_commit_diff, Commit, CommitDiff};
use anyhow::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub struct App {
    pub commits: Vec<Commit>,
    pub list_state: ListState,
    pub should_quit: bool,
    pub show_diff: bool,
    pub current_diff: Option<CommitDiff>,
    pub diff_scroll: u16,
    pub file_list_state: ListState,
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
}

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = if app.show_diff {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(25),
                Constraint::Percentage(45),
            ])
            .split(f.area())
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(f.area())
    };

    render_commit_list(f, app, chunks[0]);

    if app.show_diff && chunks.len() >= 3 {
        render_file_list(f, app, chunks[1]);
        render_diff(f, app, chunks[2]);
    }
}

fn render_commit_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .commits
        .iter()
        .map(|commit| {
            let line = Line::from(vec![
                Span::styled(&commit.graph, Style::default().fg(Color::Cyan)),
                Span::styled(&commit.hash, Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::raw(&commit.message),
            ]);
            ListItem::new(line)
        })
        .collect();

    let title = format!(" Git Log ({} commits) ", app.commits.len());
    let help = if app.show_diff {
        " Enter: Close | q: Quit "
    } else {
        " ↑/↓: Nav | Enter: View | q: Quit "
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

        let diff_content = if let Some(file_diff) = commit_diff.files.get(selected_file_index) {
            &file_diff.diff_content
        } else {
            ""
        };

        let diff_lines: Vec<Line> = diff_content
            .lines()
            .skip(app.diff_scroll as usize)
            .map(|line| {
                let style = if line.starts_with('+') && !line.starts_with("+++") {
                    Style::default().fg(Color::Green)
                } else if line.starts_with('-') && !line.starts_with("---") {
                    Style::default().fg(Color::Red)
                } else if line.starts_with("@@") {
                    Style::default().fg(Color::Cyan)
                } else if line.starts_with("diff --git") || line.starts_with("index ") {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };

                Line::from(Span::styled(line, style))
            })
            .collect();

        let filename = commit_diff
            .files
            .get(selected_file_index)
            .map(|f| f.filename.as_str())
            .unwrap_or("");

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
