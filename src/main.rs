mod app;
mod dimension;
mod tmux;
mod ui;

use anyhow::Result;
use app::{App, InputMode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tmux::Tmux;

fn main() -> Result<()> {
    // Check if tmux is installed
    if !Tmux::is_installed() {
        eprintln!("Error: tmux is not installed. Please install tmux first.");
        eprintln!("  brew install tmux");
        std::process::exit(1);
    }

    // Setup terminal
    if let Err(e) = enable_raw_mode() {
        eprintln!("Error: Cannot start Dimensions from within another TUI application.");
        eprintln!("       If you're in nvim/vim, exit first or use a tmux keybinding.");
        eprintln!("       Try: Press Ctrl+B then run 'dimensions' in a new window.");
        eprintln!("\nTechnical error: {:?}", e);
        std::process::exit(1);
    }

    let mut stdout = io::stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
        eprintln!("Error: Cannot initialize terminal interface.");
        eprintln!("       Make sure you're running this in a proper terminal.");
        eprintln!("\nTechnical error: {:?}", e);
        std::process::exit(1);
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new()?;

    // Run the app
    let res = run_app(&mut terminal, &mut app);

    // Get the session to attach to and detach flag before restoring terminal
    let should_attach = app.should_attach.clone();
    let should_detach = app.should_detach;

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
        return Ok(());
    }

    // Handle post-TUI actions
    if should_detach && Tmux::is_inside_session() {
        // User pressed 'q' and we're in tmux - detach
        Tmux::detach()?;
    } else if let Some(session) = should_attach {
        // User pressed Enter to switch - attach/switch to session
        if Tmux::is_inside_session() {
            // We're already in tmux, switch client
            Tmux::switch_session(&session)?;
        } else {
            // Not in tmux, attach to session
            Tmux::attach_session(&session)?;
        }
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if app.should_quit {
            break;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Only process key press events, not release
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match app.input_mode {
                    InputMode::Normal => handle_normal_mode(app, key.code)?,
                    InputMode::CreatingDimension | InputMode::AddingTab => {
                        handle_input_mode(app, key.code)?
                    }
                    InputMode::DeletingDimension => handle_delete_mode(app, key.code)?,
                }
            }
        }
    }

    Ok(())
}

fn handle_normal_mode(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Char('j') | KeyCode::Down => app.next_dimension(),
        KeyCode::Char('k') | KeyCode::Up => app.previous_dimension(),
        KeyCode::Char('l') | KeyCode::Right => app.next_tab(),
        KeyCode::Char('h') | KeyCode::Left => app.previous_tab(),
        KeyCode::Char(' ') => app.toggle_collapse_dimension(),
        KeyCode::Char('n') => app.start_create_dimension(),
        KeyCode::Char('t') => app.start_add_tab(),
        KeyCode::Char('d') => app.start_delete_dimension(),
        KeyCode::Char('x') => app.remove_tab_from_current_dimension()?,
        KeyCode::Enter => {
            if let Err(e) = app.switch_to_dimension() {
                app.set_message(format!("Error: {}", e));
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_input_mode(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Enter => app.submit_input()?,
        KeyCode::Char(c) => app.handle_input_char(c),
        KeyCode::Backspace => app.handle_input_backspace(),
        KeyCode::Esc => app.cancel_input(),
        _ => {}
    }
    Ok(())
}

fn handle_delete_mode(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.submit_input()?,
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_input(),
        _ => {}
    }
    Ok(())
}
