mod git;
mod syntax;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use ui::App;

fn main() -> Result<()> {
    // Load commits from git (no filter initially)
    let commits = git::get_commits(None)?;

    if commits.is_empty() {
        eprintln!("No commits found in the current repository.");
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new(commits);
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::ui(f, app))?;

        if app.should_quit {
            return Ok(());
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Only handle key press events, not release
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Handle input modes separately
                if app.search_mode {
                    match key.code {
                        KeyCode::Esc => app.exit_search_mode(),
                        KeyCode::Enter => app.execute_search()?,
                        KeyCode::Backspace => app.delete_search_char(),
                        KeyCode::Char(c) => app.add_search_char(c),
                        _ => {}
                    }
                } else if app.branch_input_mode {
                    match key.code {
                        KeyCode::Esc => app.exit_branch_input_mode(),
                        KeyCode::Enter => app.create_branch_from_commit(),
                        KeyCode::Backspace => app.delete_branch_char(),
                        KeyCode::Char(c) => app.add_branch_char(c),
                        _ => {}
                    }
                } else if app.tree_view_mode {
                    // Tree view mode
                    match key.code {
                        KeyCode::Char('q') => app.quit(),
                        KeyCode::Char('t') => app.toggle_tree_view()?,
                        KeyCode::Esc => app.exit_tree_view(),
                        KeyCode::Down | KeyCode::Char('j') => {
                            if app.tree_file_selected {
                                app.scroll_diff_down();
                            } else {
                                app.next_tree_file();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.tree_file_selected {
                                app.scroll_diff_up();
                            } else {
                                app.previous_tree_file();
                            }
                        }
                        KeyCode::Enter => app.select_tree_file(),
                        _ => {}
                    }
                } else {
                    // Normal mode
                    match key.code {
                        KeyCode::Char('q') => app.quit(),
                        KeyCode::Char('t') => app.toggle_tree_view()?,
                        KeyCode::Char('/') => app.enter_search_mode(),
                        KeyCode::Char('y') => {
                            app.copy_commit_hash();
                        }
                        KeyCode::Char('c') => {
                            app.checkout_selected_commit();
                        }
                        KeyCode::Char('b') => {
                            app.enter_branch_input_mode();
                        }
                        KeyCode::Char('p') => {
                            app.cherry_pick_commit();
                        }
                        KeyCode::Char('r') => {
                            app.revert_selected_commit();
                        }
                        KeyCode::Esc => {
                            if app.status_message.is_some() {
                                app.clear_status();
                            } else if app.active_filter.is_some() {
                                app.clear_search()?;
                            } else {
                                app.quit();
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if app.show_diff {
                                app.scroll_diff_down();
                            } else {
                                app.next();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.show_diff {
                                app.scroll_diff_up();
                            } else {
                                app.previous();
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            if app.show_diff {
                                app.previous_file();
                            }
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            if app.show_diff {
                                app.next_file();
                            }
                        }
                        KeyCode::Enter => app.toggle_diff()?,
                        _ => {}
                    }
                }
            }
        }
    }
}
