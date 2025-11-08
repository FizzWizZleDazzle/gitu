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
    // Load commits from git
    let commits = git::get_commits()?;

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

                match key.code {
                    KeyCode::Char('q') => app.quit(),
                    KeyCode::Esc => app.quit(),
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
