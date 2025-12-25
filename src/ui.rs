use crate::app::{App, InputMode, MatchType};
use crate::tmux::Tmux;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

fn inner_list_width(area: Rect) -> usize {
    // Account for left/right borders.
    area.width.saturating_sub(2) as usize
}

fn truncate_ellipsis(input: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if input.width() <= max_width {
        return input.to_string();
    }
    if max_width == 1 {
        return "…".to_string();
    }

    let mut out = String::new();
    let mut used = 0usize;
    let budget = max_width - 1; // leave room for ellipsis
    for ch in input.chars() {
        let w = ch.to_string().width();
        if used + w > budget {
            break;
        }
        out.push(ch);
        used += w;
    }
    out.push('…');
    out
}

fn truncate_with_suffix(main: &str, suffix: &str, max_width: usize) -> String {
    let suffix_width = suffix.width();
    if max_width == 0 {
        return String::new();
    }
    if main.width() + suffix_width <= max_width {
        return format!("{}{}", main, suffix);
    }
    if suffix_width >= max_width {
        // Suffix alone doesn't fit; just truncate the combined string.
        return truncate_ellipsis(&format!("{}{}", main, suffix), max_width);
    }
    let main_max = max_width - suffix_width;
    format!("{}{}", truncate_ellipsis(main, main_max), suffix)
}

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
    let title = Paragraph::new("🌌 Dimensions - Terminal Tab Manager")
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
    let max_width = inner_list_width(area);
    let dimensions: Vec<ListItem> = app
        .config
        .dimensions
        .iter()
        .map(|dim| {
            let is_current = app.current_session.as_ref() == Some(&dim.name);

            // Get actual window count from tmux if session exists
            let tab_count = if Tmux::session_exists(&dim.name) {
                Tmux::get_window_count(&dim.name).unwrap_or(dim.configured_tabs.len())
            } else {
                dim.configured_tabs.len()
            };

            let current_marker = if is_current { " *" } else { "" };

            let suffix = format!(" ({} tabs){}", tab_count, current_marker);
            let content = truncate_with_suffix(&dim.name, &suffix, max_width);

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
        let max_width = inner_list_width(area);
        // Get actual windows from tmux if session exists
        let (tabs, selected_pos): (Vec<ListItem>, Option<usize>) = if Tmux::session_exists(&dimension.name) {
            let windows = Tmux::list_windows(&dimension.name).unwrap_or_default();
            let mut selected_pos: Option<usize> = None;
            let items: Vec<ListItem> = windows
                .iter()
                .filter(|(_, window_name)| {
                    // Filter based on search query
                    if app.search_query.is_empty() {
                        true
                    } else {
                        window_name.to_lowercase().contains(&app.search_query.to_lowercase())
                    }
                })
                .enumerate()
                .map(|(pos, (window_idx, window_name))| {
                    if app.selected_tab == Some(*window_idx) {
                        selected_pos = Some(pos);
                    }
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

                    let main = format!("{}. {}{}", window_idx, window_name, command_text);
                    let content = truncate_with_suffix(&main, current_marker, max_width);

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
            ;
            (items, selected_pos)
        } else {
            // Session doesn't exist, show configured tabs
            let items: Vec<ListItem> = dimension
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

                    let content = truncate_ellipsis(&format!("{}. {}{}", i, tab.name, command_text), max_width);

                    ListItem::new(content)
                })
                .collect();
            (items, app.selected_tab)
        };

        let title = match app.input_mode {
            InputMode::AddingTab => "Tabs (Format: name or name:command)",
            InputMode::DeletingTab => "Tabs (Confirm delete? y/n)",
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
        state.select(selected_pos);
        f.render_stateful_widget(list, area, &mut state);
    } else {
        let text = Paragraph::new("No dimension selected")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().title("Tabs").borders(Borders::ALL));
        f.render_widget(text, area);
    }
}

fn render_search_results(f: &mut Frame, app: &App, area: Rect) {
    let max_width = inner_list_width(area);
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

            let marker = if is_current_tab { " *" } else { "" };
            let marker_width = marker.width();
            let available = max_width.saturating_sub(marker_width);

            let dim = result.dimension_name.as_str();
            let (sep, tab) = if result.tab_name == "(no tabs)" {
                (" ", "(no tabs)")
            } else {
                (": ", result.tab_name.as_str())
            };

            let sep_width = sep.width();
            let mut dim_out = dim.to_string();
            let mut tab_out = tab.to_string();

            if dim.width() + sep_width + tab.width() > available {
                // Truncate tab first, then dimension if needed.
                let tab_max = available.saturating_sub(dim.width() + sep_width);
                if tab_max > 0 {
                    tab_out = truncate_ellipsis(tab, tab_max);
                } else {
                    dim_out = truncate_ellipsis(dim, available.saturating_sub(sep_width));
                    let tab_max2 = available
                        .saturating_sub(dim_out.width() + sep_width);
                    if tab_max2 > 0 {
                        tab_out = truncate_ellipsis(tab, tab_max2);
                    } else {
                        tab_out.clear();
                    }
                }
            }

            spans.push(Span::styled(dim_out, dim_style));
            spans.push(Span::styled(sep, separator_style));
            spans.push(Span::styled(tab_out, tab_style));
            if !marker.is_empty() {
                spans.push(Span::styled(marker, tab_style));
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
            } else if let Some(msg) = &app.update_message {
                spans.push(Span::styled(
                    msg.clone(),
                    Style::default().fg(Color::Yellow),
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
        InputMode::DeletingTab => {
            if let Some(dimension) = app.get_current_dimension() {
                if let Some(tab_index) = app.selected_tab {
                    // Get tab name from tmux or config
                    let tab_name = if Tmux::session_exists(&dimension.name) {
                        Tmux::list_windows(&dimension.name)
                            .ok()
                            .and_then(|windows| {
                                windows.iter()
                                    .find(|(idx, _)| *idx == tab_index)
                                    .map(|(_, name)| name.clone())
                            })
                            .unwrap_or_else(|| "unknown".to_string())
                    } else {
                        dimension.configured_tabs
                            .get(tab_index)
                            .map(|t| t.name.clone())
                            .unwrap_or_else(|| "unknown".to_string())
                    };

                    spans.push(Span::styled(
                        format!("Delete tab '{}'? (y/n)", tab_name),
                        Style::default().fg(Color::Red),
                    ));
                }
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
                Span::raw(" Navigate tabs"),
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
        InputMode::DeletingDimension | InputMode::DeletingTab => vec![
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
