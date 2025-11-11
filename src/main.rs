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
                } else if app.commit_message_mode {
                    match key.code {
                        KeyCode::Esc => app.exit_commit_message_mode(),
                        KeyCode::Enter => app.execute_commit(),
                        KeyCode::Backspace => app.delete_commit_char(),
                        KeyCode::Char(c) => app.add_commit_char(c),
                        _ => {}
                    }
                } else if app.stash_input_mode {
                    match key.code {
                        KeyCode::Esc => app.exit_stash_input_mode(),
                        KeyCode::Enter => app.execute_create_stash(),
                        KeyCode::Backspace => app.delete_stash_char(),
                        KeyCode::Char(c) => app.add_stash_char(c),
                        _ => {}
                    }
                } else if app.new_branch_input_mode {
                    match key.code {
                        KeyCode::Esc => app.exit_new_branch_mode(),
                        KeyCode::Enter => app.execute_create_new_branch(),
                        KeyCode::Backspace => app.delete_new_branch_char(),
                        KeyCode::Char(c) => app.add_new_branch_char(c),
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
                    // Normal mode - check which panel is active
                    use ui::Panel;

                    // Global keybindings (work in all panels)
                    match key.code {
                        KeyCode::Char('q') => app.quit(),
                        KeyCode::Char('1') => app.switch_to_panel(Panel::Status),
                        KeyCode::Char('2') => app.switch_to_panel(Panel::Log),
                        KeyCode::Char('3') => app.switch_to_panel(Panel::Stash),
                        KeyCode::Char('4') => app.switch_to_panel(Panel::Branches),
                        KeyCode::Esc => {
                            if app.status_message.is_some() {
                                app.clear_status();
                            } else if app.active_filter.is_some() {
                                app.clear_search()?;
                            } else {
                                app.quit();
                            }
                        }
                        _ => {
                            // Panel-specific keybindings
                            match app.current_panel {
                                Panel::Status => {
                                    match key.code {
                                        KeyCode::Char(' ') => app.toggle_stage(),
                                        KeyCode::Char('a') => app.stage_all_files(),
                                        KeyCode::Char('u') => app.unstage_all_files(),
                                        KeyCode::Char('c') => app.enter_commit_message_mode(),
                                        KeyCode::Char('s') => app.enter_stash_input_mode(),
                                        KeyCode::Enter => app.toggle_status_diff(),
                                        KeyCode::Down | KeyCode::Char('j') => {
                                            if app.status_show_diff {
                                                app.scroll_status_diff_down();
                                            } else {
                                                app.next_status_file();
                                            }
                                        }
                                        KeyCode::Up | KeyCode::Char('k') => {
                                            if app.status_show_diff {
                                                app.scroll_status_diff_up();
                                            } else {
                                                app.previous_status_file();
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                Panel::Log => {
                                    match key.code {
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
                                        KeyCode::Char('f') => {
                                            app.fetch_from_remote();
                                        }
                                        KeyCode::Char('P') => {
                                            app.push_to_remote();
                                        }
                                        KeyCode::Char('U') => {
                                            app.pull_from_remote();
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
                                Panel::Stash => {
                                    match key.code {
                                        KeyCode::Char('a') => app.apply_selected_stash(),
                                        KeyCode::Char('p') => app.pop_selected_stash(),
                                        KeyCode::Char('d') => app.drop_selected_stash(),
                                        KeyCode::Down | KeyCode::Char('j') => app.next_stash(),
                                        KeyCode::Up | KeyCode::Char('k') => app.previous_stash(),
                                        _ => {}
                                    }
                                }
                                Panel::Branches => {
                                    match key.code {
                                        KeyCode::Enter => app.switch_to_selected_branch(),
                                        KeyCode::Char('d') => app.delete_selected_branch(),
                                        KeyCode::Char('n') => app.enter_new_branch_mode(),
                                        KeyCode::Down | KeyCode::Char('j') => app.next_branch(),
                                        KeyCode::Up | KeyCode::Char('k') => app.previous_branch(),
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
