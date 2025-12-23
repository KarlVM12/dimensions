use crate::app::{App, InputMode, MatchType};
use crate::tmux::Tmux;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(0),     // Main content
            Constraint::Length(3),  // Status bar
            Constraint::Length(5),  // Help
        ])
        .split(f.area());

    render_title(f, chunks[0]);
    render_main_content(f, app, chunks[1]);
    render_status_bar(f, app, chunks[2]);
    render_help(f, app, chunks[3]);
}

fn render_title(f: &mut Frame, area: Rect) {
    let title = Paragraph::new("🌌 Dimensions - Visual Tmux Session Manager")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn render_main_content(f: &mut Frame, app: &mut App, area: Rect) {
    // Check if we're in active search mode with a query
    if app.input_mode == InputMode::Searching && !app.search_query.is_empty() {
        // Compute search results if needed
        app.compute_search_results();

        // Render single-column search results
        render_search_results(f, app, area);
    } else {
        // Render normal two-column layout
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40),  // Dimensions list
                Constraint::Percentage(60),  // Tabs list
            ])
            .split(area);

        render_dimensions_list(f, app, chunks[0]);
        render_tabs_list(f, app, chunks[1]);
    }
}

fn render_dimensions_list(f: &mut Frame, app: &App, area: Rect) {
    let dimensions: Vec<ListItem> = app
        .config
        .dimensions
        .iter()
        .map(|dim| {
            let is_current = app.current_session.as_ref() == Some(&dim.name);
            let collapse_icon = if dim.collapsed { "▶" } else { "▼" };

            // Get actual window count from tmux if session exists
            let tab_count = if Tmux::session_exists(&dim.name) {
                Tmux::get_window_count(&dim.name).unwrap_or(dim.configured_tabs.len())
            } else {
                dim.configured_tabs.len()
            };

            let current_marker = if is_current { " *" } else { "" };

            let content = format!(
                "{} {} ({} tabs){}",
                collapse_icon,
                dim.name,
                tab_count,
                current_marker
            );

            let style = if is_current {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let title = match app.input_mode {
        InputMode::CreatingDimension => "Dimensions (Enter name)",
        InputMode::DeletingDimension => "Dimensions (Confirm delete? y/n)",
        _ => "Dimensions",
    };

    let list = List::new(dimensions)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    if !app.config.dimensions.is_empty() {
        state.select(Some(app.selected_dimension));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn render_tabs_list(f: &mut Frame, app: &App, area: Rect) {
    if let Some(dimension) = app.get_current_dimension() {
        if dimension.collapsed {
            let text = Paragraph::new("Dimension is collapsed")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().title("Tabs").borders(Borders::ALL));
            f.render_widget(text, area);
            return;
        }

        // Get actual windows from tmux if session exists
        let tabs: Vec<ListItem> = if Tmux::session_exists(&dimension.name) {
            let windows = Tmux::list_windows(&dimension.name).unwrap_or_default();
            windows
                .iter()
                .filter(|(_, window_name)| {
                    // Filter based on search query
                    if app.search_query.is_empty() {
                        true
                    } else {
                        window_name.to_lowercase().contains(&app.search_query.to_lowercase())
                    }
                })
                .map(|(window_idx, window_name)| {
                    let is_current = app.current_session.as_ref() == Some(&dimension.name)
                        && app.current_window == Some(*window_idx);

                    // Check if this window has a configured command
                    let command_text = dimension
                        .configured_tabs
                        .iter()
                        .find(|t| &t.name == window_name)
                        .and_then(|t| t.command.as_ref())
                        .map(|c| format!(" ({})", c))
                        .unwrap_or_default();

                    let current_marker = if is_current { " *" } else { "" };

                    let content = format!("{}. {}{}{}", window_idx, window_name, command_text, current_marker);

                    let style = if is_current {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };

                    ListItem::new(content).style(style)
                })
                .collect()
        } else {
            // Session doesn't exist, show configured tabs
            dimension
                .configured_tabs
                .iter()
                .enumerate()
                .filter(|(_, tab)| {
                    // Filter based on search query
                    if app.search_query.is_empty() {
                        true
                    } else {
                        tab.name.to_lowercase().contains(&app.search_query.to_lowercase())
                    }
                })
                .map(|(i, tab)| {
                    let command_text = tab
                        .command
                        .as_ref()
                        .map(|c| format!(" ({})", c))
                        .unwrap_or_default();

                    let content = format!("{}. {}{}", i, tab.name, command_text);

                    ListItem::new(content)
                })
                .collect()
        };
        let tabs_len = tabs.len();

        let title = match app.input_mode {
            InputMode::AddingTab => "Tabs (Format: name or name:command)",
            _ => "Tabs",
        };

    let list = List::new(tabs)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

        let mut state = ListState::default();
        if let Some(selected_tab) = app.selected_tab {
            if selected_tab < tabs_len {
                state.select(Some(selected_tab));
            }
        }
        f.render_stateful_widget(list, area, &mut state);
    } else {
        let text = Paragraph::new("No dimension selected")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().title("Tabs").borders(Borders::ALL));
        f.render_widget(text, area);
    }
}

fn render_search_results(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .map(|result| {
            let is_current_session = app.current_session.as_ref() == Some(&result.dimension_name);
            let is_current_tab = is_current_session
                && app.current_window == Some(result.tmux_window_index)
                && result.tab_name != "(no tabs)";

            let base_style = match result.match_type {
                MatchType::Both => Style::default().fg(Color::White),
                MatchType::DimensionOnly => Style::default().fg(Color::Gray),
                MatchType::TabOnly => Style::default().fg(Color::White),
            };

            let dim_style = if is_current_session {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                base_style
            };

            let tab_style = if is_current_tab {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                base_style
            };

            let mut spans = Vec::new();
            let separator_style = base_style;

            // Dimension name (green only for the current session).
            spans.push(Span::styled(result.dimension_name.clone(), dim_style));

            if result.tab_name == "(no tabs)" {
                spans.push(Span::styled(" (no tabs)", tab_style));
            } else {
                spans.push(Span::styled(": ", separator_style));
                spans.push(Span::styled(result.tab_name.clone(), tab_style));

                if is_current_tab {
                    spans.push(Span::styled(" *", tab_style));
                }
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = if app.search_results.is_empty() {
        format!("Search Results: '{}' (no matches)", app.search_query)
    } else {
        format!("Search Results: '{}' ({} matches)", app.search_query, app.search_results.len())
    };

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    if !app.search_results.is_empty() && app.search_selected_index < app.search_results.len() {
        state.select(Some(app.search_selected_index));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![];

    match app.input_mode {
        InputMode::Normal => {
            if let Some(msg) = &app.message {
                spans.push(Span::styled(
                    msg.clone(),
                    Style::default().fg(Color::Green),
                ));
            }
        }
        InputMode::CreatingDimension | InputMode::AddingTab => {
            spans.push(Span::raw("Input: "));
            spans.push(Span::styled(
                app.input_buffer.clone(),
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::styled(" █", Style::default().fg(Color::White)));
        }
        InputMode::Searching => {
            spans.push(Span::raw("Search: /"));
            spans.push(Span::styled(
                app.input_buffer.clone(),
                Style::default().fg(Color::Cyan),
            ));
            spans.push(Span::styled(" █", Style::default().fg(Color::White)));
        }
        InputMode::DeletingDimension => {
            if let Some(dim) = app.get_current_dimension() {
                spans.push(Span::styled(
                    format!("Delete dimension '{}'? (y/n)", dim.name),
                    Style::default().fg(Color::Red),
                ));
            }
        }
    }

    let status = Paragraph::new(Line::from(spans))
        .block(Block::default().title("Status").borders(Borders::ALL));

    f.render_widget(status, area);
}

fn render_help(f: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.input_mode {
        InputMode::Normal => vec![
            Line::from(vec![
                Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
                Span::raw(" Navigate dimensions  "),
                Span::styled("←/→", Style::default().fg(Color::Yellow)),
                Span::raw(" Navigate tabs  "),
                Span::styled("Space", Style::default().fg(Color::Yellow)),
                Span::raw(" Collapse/expand"),
            ]),
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::raw(" Switch  "),
                Span::styled("n", Style::default().fg(Color::Yellow)),
                Span::raw(" New dim  "),
                Span::styled("t", Style::default().fg(Color::Yellow)),
                Span::raw(" New tab  "),
                Span::styled("d", Style::default().fg(Color::Yellow)),
                Span::raw(" Delete  "),
                Span::styled("/", Style::default().fg(Color::Yellow)),
                Span::raw(" Search  "),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::raw(" Close  "),
                Span::styled("q", Style::default().fg(Color::Yellow)),
                Span::raw(" Quit"),
            ]),
        ],
        InputMode::CreatingDimension | InputMode::AddingTab => vec![
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::raw(" Submit  "),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::raw(" Cancel"),
            ]),
        ],
        InputMode::Searching => {
            if app.search_query.is_empty() {
                // Before query is entered
                vec![
                    Line::from(vec![
                        Span::raw("Type to search dimensions and tabs (live)  "),
                        Span::styled("Esc", Style::default().fg(Color::Yellow)),
                        Span::raw(" Cancel"),
                    ]),
                ]
            } else {
                // After query is entered, showing results
                vec![
                    Line::from(vec![
                        Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
                        Span::raw(" Navigate results  "),
                        Span::styled("Enter", Style::default().fg(Color::Yellow)),
                        Span::raw(" Select  "),
                        Span::styled("Esc", Style::default().fg(Color::Yellow)),
                        Span::raw(" Cancel"),
                    ]),
                ]
            }
        }
        InputMode::DeletingDimension => vec![
            Line::from(vec![
                Span::styled("y", Style::default().fg(Color::Yellow)),
                Span::raw(" Confirm  "),
                Span::styled("n/Esc", Style::default().fg(Color::Yellow)),
                Span::raw(" Cancel"),
            ]),
        ],
    };

    let help = Paragraph::new(help_text)
        .block(Block::default().title("Help").borders(Borders::ALL));

    f.render_widget(help, area);
}
