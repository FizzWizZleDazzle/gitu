use crate::ui::{App, Panel};
use anyhow::Result;
use crossterm::event::KeyCode;

pub fn handle_key_event(app: &mut App, key_code: KeyCode) -> Result<()> {
    // Handle help popup first
    if app.help_visible {
        match key_code {
            KeyCode::Char('?') | KeyCode::Esc => app.help_visible = false,
            _ => {}
        }
        return Ok(());
    }

    // Handle input modes
    if app.search_mode {
        handle_search_mode(app, key_code)?;
    } else if app.branch_input_mode {
        handle_branch_input_mode(app, key_code);
    } else if app.commit_message_mode {
        handle_commit_message_mode(app, key_code);
    } else if app.stash_input_mode {
        handle_stash_input_mode(app, key_code);
    } else if app.new_branch_input_mode {
        handle_new_branch_mode(app, key_code);
    } else if app.tree_view_mode {
        handle_tree_view_mode(app, key_code)?;
    } else {
        handle_normal_mode(app, key_code)?;
    }

    Ok(())
}

fn handle_search_mode(app: &mut App, key_code: KeyCode) -> Result<()> {
    match key_code {
        KeyCode::Esc => app.exit_search_mode(),
        KeyCode::Enter => app.execute_search()?,
        KeyCode::Backspace => app.delete_search_char(),
        KeyCode::Char(c) => app.add_search_char(c),
        _ => {}
    }
    Ok(())
}

fn handle_branch_input_mode(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Esc => app.exit_branch_input_mode(),
        KeyCode::Enter => app.create_branch_from_commit(),
        KeyCode::Backspace => app.delete_branch_char(),
        KeyCode::Char(c) => app.add_branch_char(c),
        _ => {}
    }
}

fn handle_commit_message_mode(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Esc => app.exit_commit_message_mode(),
        KeyCode::Enter => app.execute_commit(),
        KeyCode::Backspace => app.delete_commit_char(),
        KeyCode::Char(c) => app.add_commit_char(c),
        _ => {}
    }
}

fn handle_stash_input_mode(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Esc => app.exit_stash_input_mode(),
        KeyCode::Enter => app.execute_create_stash(),
        KeyCode::Backspace => app.delete_stash_char(),
        KeyCode::Char(c) => app.add_stash_char(c),
        _ => {}
    }
}

fn handle_new_branch_mode(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Esc => app.exit_new_branch_mode(),
        KeyCode::Enter => app.execute_create_new_branch(),
        KeyCode::Backspace => app.delete_new_branch_char(),
        KeyCode::Char(c) => app.add_new_branch_char(c),
        _ => {}
    }
}

fn handle_tree_view_mode(app: &mut App, key_code: KeyCode) -> Result<()> {
    match key_code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Char('?') => app.help_visible = true,
        KeyCode::Char('t') => app.toggle_tree_view()?,
        KeyCode::Esc => app.exit_tree_view(),
        KeyCode::PageUp => {
            if app.tree_file_selected {
                app.scroll_diff_page_up();
            }
        }
        KeyCode::PageDown => {
            if app.tree_file_selected {
                app.scroll_diff_page_down();
            }
        }
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
    Ok(())
}

fn handle_normal_mode(app: &mut App, key_code: KeyCode) -> Result<()> {
    // Global keybindings (work in all panels)
    match key_code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Char('?') => app.help_visible = true,
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
                Panel::Status => handle_status_panel(app, key_code),
                Panel::Log => handle_log_panel(app, key_code)?,
                Panel::Stash => handle_stash_panel(app, key_code),
                Panel::Branches => handle_branches_panel(app, key_code),
            }
        }
    }
    Ok(())
}

fn handle_status_panel(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Char(' ') => app.toggle_stage(),
        KeyCode::Char('a') => app.stage_all_files(),
        KeyCode::Char('u') => app.unstage_all_files(),
        KeyCode::Char('c') => app.enter_commit_message_mode(),
        KeyCode::Char('A') => app.enter_amend_mode(),
        KeyCode::Char('x') => app.discard_selected_file(),
        KeyCode::Char('s') => app.enter_stash_input_mode(),
        KeyCode::Enter => app.toggle_status_diff(),
        KeyCode::PageUp => {
            if app.status_show_diff {
                app.scroll_status_diff_page_up();
            }
        }
        KeyCode::PageDown => {
            if app.status_show_diff {
                app.scroll_status_diff_page_down();
            }
        }
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

fn handle_log_panel(app: &mut App, key_code: KeyCode) -> Result<()> {
    match key_code {
        KeyCode::Char('t') => app.toggle_tree_view()?,
        KeyCode::Char('/') => app.enter_search_mode(),
        KeyCode::Char('y') => app.copy_commit_hash(),
        KeyCode::Char('c') => app.checkout_selected_commit(),
        KeyCode::Char('b') => app.enter_branch_input_mode(),
        KeyCode::Char('p') => app.cherry_pick_commit(),
        KeyCode::Char('r') => app.revert_selected_commit(),
        KeyCode::Char('f') => app.fetch_from_remote(),
        KeyCode::Char('P') => app.push_to_remote(),
        KeyCode::Char('U') => app.pull_from_remote(),
        KeyCode::PageUp => {
            if app.show_diff {
                app.scroll_diff_page_up();
            }
        }
        KeyCode::PageDown => {
            if app.show_diff {
                app.scroll_diff_page_down();
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
    Ok(())
}

fn handle_stash_panel(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Char('a') => app.apply_selected_stash(),
        KeyCode::Char('p') => app.pop_selected_stash(),
        KeyCode::Char('d') => app.drop_selected_stash(),
        KeyCode::Down | KeyCode::Char('j') => app.next_stash(),
        KeyCode::Up | KeyCode::Char('k') => app.previous_stash(),
        _ => {}
    }
}

fn handle_branches_panel(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Enter => app.switch_to_selected_branch(),
        KeyCode::Char('d') => app.delete_selected_branch(),
        KeyCode::Char('n') => app.enter_new_branch_mode(),
        KeyCode::Char('m') => app.merge_selected_branch(),
        KeyCode::Down | KeyCode::Char('j') => app.next_branch(),
        KeyCode::Up | KeyCode::Char('k') => app.previous_branch(),
        _ => {}
    }
}
