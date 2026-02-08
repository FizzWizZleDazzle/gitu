use super::{App, MessageType, Panel};
use crate::git::{Branch, Decoration, SearchFilter, StatusFile};
use crate::syntax;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

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

    // Render help popup overlay (on top of everything)
    if app.help_visible {
        render_help_popup(f);
    }
}

fn render_tab_bar(f: &mut Frame, app: &App, area: Rect) {
    let tabs = [
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

    // Build a mapping from list index to file index (accounting for headers)
    let mut index_to_file: Vec<usize> = Vec::new();

    let items: Vec<ListItem> = {
        let mut items = Vec::new();
        let mut file_idx = 0;

        if !staged.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "Staged Changes:",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ))));
            index_to_file.push(usize::MAX); // Header, no file mapping

            for file in &staged {
                index_to_file.push(file_idx);
                file_idx += 1;
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
            index_to_file.push(usize::MAX); // Header, no file mapping

            for file in &unstaged {
                index_to_file.push(file_idx);
                file_idx += 1;
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
            index_to_file.push(usize::MAX); // No file
        }

        items
    };

    let title = format!(" Status ({} files) ", app.status_files.len());
    let help = if app.status_show_diff {
        " j/k: Scroll | PgUp/PgDn: Page | Enter: Hide diff | Space: Stage/Unstage "
    } else {
        " Space: Stage/Unstage | a/u: Stage/Unstage all | c: Commit | A: Amend | x: Discard | ?: Help "
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
            // Map list index to file index using the mapping we built
            let selected_file = app.status_list_state.selected()
                .and_then(|list_idx| index_to_file.get(list_idx))
                .and_then(|&file_idx| {
                    if file_idx == usize::MAX {
                        None // This was a header, not a file
                    } else {
                        app.status_files.get(file_idx)
                    }
                });

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
    let help = " Enter: Switch | d: Delete | n: New | m: Merge | ?: Help ";

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
    let (title, help) = if app.amend_mode {
        (" Amend Commit Message ", " Edit message | Enter: Amend | Esc: Cancel ")
    } else {
        (" Commit Message ", " Type commit message | Enter: Commit | Esc: Cancel ")
    };

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

    let border_color = if app.amend_mode { Color::Yellow } else { Color::Green };

    let paragraph = Paragraph::new(input_text)
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_bottom(help)
                .border_style(Style::default().fg(border_color)),
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

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_help_popup(f: &mut Frame) {
    let area = centered_rect(60, 70, f.area());
    f.render_widget(Clear, area);

    let help_text = vec![
        Line::from(Span::styled("Keybindings", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("Global", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("  1-4        Switch panels (Status/Log/Stash/Branches)"),
        Line::from("  ?          Toggle this help"),
        Line::from("  q          Quit / Close diff"),
        Line::from("  Esc        Cancel / Clear"),
        Line::from("  PgUp/PgDn  Scroll diff by 10 lines"),
        Line::from(""),
        Line::from(Span::styled("Status Panel", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("  Space      Stage / Unstage file"),
        Line::from("  a          Stage all files"),
        Line::from("  u          Unstage all files"),
        Line::from("  c          Commit"),
        Line::from("  A          Amend last commit"),
        Line::from("  x          Discard changes in file"),
        Line::from("  s          Stash changes"),
        Line::from("  Enter      Show / Hide diff"),
        Line::from(""),
        Line::from(Span::styled("Log Panel", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("  Enter      Show / Hide diff"),
        Line::from("  t          Tree view"),
        Line::from("  /          Search commits"),
        Line::from("  y          Copy commit hash"),
        Line::from("  c          Checkout commit"),
        Line::from("  b          Create branch from commit"),
        Line::from("  p          Cherry-pick commit"),
        Line::from("  r          Revert commit"),
        Line::from("  f          Fetch from remote"),
        Line::from("  P          Push to remote"),
        Line::from("  U          Pull from remote"),
        Line::from(""),
        Line::from(Span::styled("Stash Panel", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("  a          Apply stash"),
        Line::from("  p          Pop stash"),
        Line::from("  d          Drop stash"),
        Line::from(""),
        Line::from(Span::styled("Branches Panel", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("  Enter      Switch to branch"),
        Line::from("  d          Delete branch"),
        Line::from("  n          Create new branch"),
        Line::from("  m          Merge branch into current"),
        Line::from(""),
        Line::from(Span::styled("  Press ? or Esc to close", Style::default().fg(Color::DarkGray))),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help ")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}
