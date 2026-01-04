mod app;
mod dimension;
mod path_completion;
mod tmux;
mod ui;
mod update;

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
    // Lightweight CLI flags (before terminal init).
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("dimensions v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|a| a == "--update" || a == "-u") {
        let current = env!("CARGO_PKG_VERSION");
        let Some(tag) = update::latest_tag() else {
            eprintln!("Could not check for updates right now.");
            eprintln!("Current: dimensions v{current}");
            eprintln!("Releases: https://github.com/KarlVM12/Dimensions/releases");
            return Ok(());
        };

        match update::is_newer_than_current(&tag, current) {
            Some(false) => {
                println!("Already on the latest version (v{current}).");
                return Ok(());
            }
            None => {
                eprintln!("Could not compare versions (current v{current}, latest {tag}).");
                println!("{}", update::update_instructions(&tag));
                return Ok(());
            }
            Some(true) => {}
        }

        eprintln!("Update available: {tag} (current v{current})");
        eprint!("Update now? [y/N] ");
        use std::io::Write;
        std::io::stderr().flush().ok();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        let answer = input.trim().to_lowercase();
        if answer != "y" && answer != "yes" {
            eprintln!("Cancelled.");
            println!("{}", update::update_instructions(&tag));
            return Ok(());
        }

        if std::process::Command::new("curl").arg("--version").output().is_err() {
            eprintln!("`curl` is required for `dimensions --update`.");
            println!("{}", update::update_instructions(&tag));
            return Ok(());
        }

        // Install into the directory of the currently-running binary so PATH precedence doesn't
        // cause the update to appear to "not work" (e.g. ~/.cargo/bin vs ~/.local/bin).
        let install_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .and_then(|d| d.to_str().map(|s| s.to_string()))
            .unwrap_or_else(|| String::from(format!("{}/.local/bin", std::env::var("HOME").unwrap_or_default())));

        // Run the installer pinned to the latest tag.
        let cmd = format!(
            "curl -fsSL https://raw.githubusercontent.com/KarlVM12/Dimensions/{tag}/install.sh | sh -s -- --version {tag} --dir \"{dir}\"",
            tag = tag,
            dir = install_dir
        );
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .status();

        match status {
            Ok(s) if s.success() => {}
            Ok(s) => {
                eprintln!("Update command failed (exit {}).", s);
                println!("{}", update::update_instructions(&tag));
            }
            Err(e) => {
                eprintln!("Failed to run update command: {e}");
                println!("{}", update::update_instructions(&tag));
            }
        }
        return Ok(());
    }

    // Check if tmux is installed
    if !Tmux::is_installed() {
        eprintln!("Error: tmux is not installed. Please install tmux first.");
        eprintln!("  brew install tmux");
        std::process::exit(1);
    }

    // Setup terminal
    if let Err(e) = enable_raw_mode() {
        eprintln!("Error: Cannot start Dimensions from within another TUI application.");
        eprintln!("       Exit the current TUI first, or use a tmux popup keybinding.");
        eprintln!("       Tip: bind any key (commonly Ctrl+G) to a popup in ~/.tmux.conf, e.g.:");
        eprintln!("         bind -n C-g display-popup -E -w 80% -h 80% \"dimensions\"");
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
    let should_select_window = app.should_select_window;
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
        // Build target with window index if specified
        let target = if let Some(window_index) = should_select_window {
            format!("{}:{}", session, window_index)
        } else {
            session.clone()
        };

        // Switch/attach to the target session
        if Tmux::is_inside_session() {
            // We're in tmux, switch client
            Tmux::switch_session(&target)?;
        } else {
            // Not in tmux, attach to session
            Tmux::attach_session(&target)?;
        }
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        app.poll_update();
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

                let result = match app.input_mode {
                    InputMode::Normal => handle_normal_mode(app, key.code),
                    InputMode::CreatingDimension | InputMode::CreatingDimensionDirectory | InputMode::AddingTab | InputMode::Searching | InputMode::JumpingToTab => {
                        handle_input_mode(app, key.code)
                    }
                    InputMode::DeletingDimension | InputMode::DeletingTab => handle_delete_mode(app, key.code),
                };

                // Display errors in status bar instead of crashing
                if let Err(e) = result {
                    app.cancel_input(); // Exit input mode so error message is visible
                    app.set_message(format!("Error: {}", e));
                }

                // Update preview if selection changed
                if app.should_refresh_preview() {
                    app.update_preview();
                }
            }
        }
    }

    Ok(())
}

fn handle_normal_mode(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Esc => app.close_popup(),
        KeyCode::Char('j') | KeyCode::Down => app.next_dimension(),
        KeyCode::Char('k') | KeyCode::Up => app.previous_dimension(),
        KeyCode::Char('l') | KeyCode::Right => app.next_tab(),
        KeyCode::Char('h') | KeyCode::Left => app.previous_tab(),
        KeyCode::Char('n') => app.start_create_dimension(),
        KeyCode::Char('t') => app.start_add_tab(),
        KeyCode::Char('d') => {
            // Context-sensitive delete: tab if selected, otherwise dimension
            if app.selected_tab.is_some() {
                app.start_delete_tab();
            } else {
                app.start_delete_dimension();
            }
        }
        KeyCode::Char('/') => app.start_search(),
        KeyCode::Char(':') => {
            // Only allow jump mode when dimension is selected
            if !app.config.dimensions.is_empty() {
                app.start_jump_to_tab();
            }
        }
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
        KeyCode::Enter => {
            if app.input_mode == InputMode::Searching && !app.search_results.is_empty() {
                // In search mode with results, Enter selects and switches
                app.select_search_result()?;
            } else {
                // Normal submit for other input modes
                app.submit_input()?;
            }
        }
        KeyCode::Tab => {
            // Handle tab completion for directory input
            app.handle_tab_completion();
        }
        KeyCode::BackTab => {
            // Handle backward tab completion for directory input
            app.handle_backtab_completion();
        }
        KeyCode::Char(c) => app.handle_input_char(c),
        KeyCode::Backspace => app.handle_input_backspace(),
        KeyCode::Esc => app.cancel_input(),
        KeyCode::Up | KeyCode::Down => {
            // In search mode, navigate results
            if app.input_mode == InputMode::Searching {
                if key == KeyCode::Up {
                    app.previous_search_result();
                } else {
                    app.next_search_result();
                }
            }
        }
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
