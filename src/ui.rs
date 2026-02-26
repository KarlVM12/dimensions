use crate::app::{App, InputMode, MatchType};
use crate::tmux::Tmux;
use ansi_to_tui::IntoText;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
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

fn format_path_with_tilde(path: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if path.starts_with(&home) {
            return path.replacen(&home, "~", 1);
        }
    }
    path.to_string()
}

pub fn render(f: &mut Frame, app: &mut App) {
    let show_completion = app.input_mode == InputMode::CreatingDimensionDirectory
        && app.completion_candidates.len() > 1;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(0),     // Main content
            Constraint::Length(3),  // Status bar
            Constraint::Length(if show_completion { 5 } else { 0 }),  // Completion overlay
            Constraint::Length(5),  // Help
        ])
        .split(f.area());

    render_title(f, chunks[0]);
    render_main_content(f, app, chunks[1]);
    render_status_bar(f, app, chunks[2]);

    // Render completion overlay if applicable
    if show_completion {
        render_completion_overlay(f, app, chunks[3]);
    }

    // Help is always at index 4 (last chunk)
    render_help(f, app, chunks[4]);
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

            let style = if is_current {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Create styled line with name, tab count, marker, and path (faded)
            let mut spans = vec![
                Span::styled(dim.name.clone(), style),
                Span::styled(format!(" [{} tabs]", tab_count), style),
                Span::styled(current_marker, style),
            ];

            if let Some(path) = dim.base_dir.as_ref().and_then(|p| p.to_str()) {
                spans.push(Span::styled(
                    format!(" ({})", format_path_with_tilde(path)),
                    Style::default().fg(Color::Gray)
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = match app.input_mode {
        InputMode::CreatingDimension => "Dimensions (Enter name)".to_string(),
        InputMode::CreatingDimensionDirectory => {
            if let Some(name) = &app.pending_dimension_name {
                format!("Creating '{}' - Enter base directory", name)
            } else {
                "Dimensions (Enter base directory)".to_string()
            }
        }
        InputMode::DeletingDimension => "Dimensions (Confirm delete? y/n)".to_string(),
        InputMode::RenamingDimension => "Dimensions (Rename)".to_string(),
        _ => "Dimensions".to_string(),
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
    // Check if we should show preview
    let show_preview = app.preview_content.is_some() && app.selected_tab.is_some();

    // Split area vertically: tabs list (top) and preview (bottom)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if show_preview {
            [Constraint::Percentage(40), Constraint::Percentage(60)]
        } else {
            [Constraint::Percentage(100), Constraint::Length(0)]
        })
        .split(area);

    if let Some(dimension) = app.get_current_dimension() {
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

                    let style = if is_current {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };

                    // Find configured tab for this window
                    let configured_tab = dimension
                        .configured_tabs
                        .iter()
                        .find(|t| &t.name == window_name);

                    let current_marker = if is_current { " *" } else { "" };

                    // Build spans with name, command, and marker
                    let mut spans = vec![
                        Span::styled(format!("{}. {}", window_idx, window_name), style)
                    ];

                    // Add command if available
                    if let Some(tab) = configured_tab {
                        if let Some(cmd) = &tab.command {
                            spans.push(Span::styled(
                                format!(" ({})", cmd),
                                style
                            ));
                        }
                    }

                    spans.push(Span::styled(current_marker, style));

                    ListItem::new(Line::from(spans))
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
                    // Build spans with name and command
                    let mut spans = vec![
                        Span::raw(format!("{}. {}", i, tab.name))
                    ];

                    // Add command if available
                    if let Some(cmd) = &tab.command {
                        spans.push(Span::raw(format!(" ({})", cmd)));
                    }

                    ListItem::new(Line::from(spans))
                })
                .collect();
            (items, app.selected_tab)
        };

        let title = match app.input_mode {
            InputMode::AddingTab => "Tabs (Format: name or name:command)".to_string(),
            InputMode::DeletingTab => "Tabs (Confirm delete? y/n)".to_string(),
            InputMode::RenamingTab => "Tabs (Rename)".to_string(),
            _ => {
                // Show dimension's base_dir in the title if available
                if let Some(path) = dimension.base_dir.as_ref().and_then(|p| p.to_str()) {
                    format!("Tabs ({})", format_path_with_tilde(path))
                } else {
                    "Tabs".to_string()
                }
            }
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
        f.render_stateful_widget(list, chunks[0], &mut state);
    } else {
        let text = Paragraph::new("No dimension selected")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().title("Tabs").borders(Borders::ALL));
        f.render_widget(text, chunks[0]);
    }

    // Render preview pane if showing
    if show_preview {
        render_preview_pane(f, app, chunks[1]);
    }
}

fn render_preview_pane(f: &mut Frame, app: &App, area: Rect) {
    let content = normalize_preview_content(app.preview_content.as_deref().unwrap_or(""));

    // Build title
    let title = if let (Some(session), Some(window)) = (&app.preview_session, &app.preview_window) {
        format!("Preview: {}:{}", session, window)
    } else {
        "Preview".to_string()
    };

    // Parse ANSI escape codes into styled text and convert to ratatui types.
    let parsed = content.as_bytes().into_text().unwrap_or_default();
    let text = Text {
        alignment: convert_alignment(parsed.alignment),
        style: Style::default(), // Don't inherit global style
        lines: parsed
            .lines
            .into_iter()
            .map(|line| {
                let mut spans: Vec<Span> = line
                    .spans
                    .into_iter()
                    .map(|span| Span::styled(span.content.into_owned(), convert_style(span.style)))
                    .collect();

                // Ensure line ends with a style reset to prevent color bleeding to next line
                if !spans.is_empty() {
                    spans.push(Span::raw(""));
                }

                Line {
                    style: Style::default(), // Don't inherit line-level style
                    alignment: convert_alignment(line.alignment),
                    spans,
                }
            })
            .collect(),
    };

    let inner_height = area.height.saturating_sub(2) as usize;
    let total_lines = text.lines.len();

    // If content fits, show it all. Otherwise, show first 8, "...", last 8
    let display_text = if total_lines <= inner_height {
        text
    } else {
        let preview_lines = 8;
        let mut lines = Vec::new();

        // First 8 lines
        lines.extend(text.lines.iter().take(preview_lines).cloned());

        // Separator
        lines.push(Line::from(Span::styled(
            "...",
            Style::default().fg(Color::DarkGray),
        )));

        // Last 8 lines
        if total_lines > preview_lines {
            lines.extend(
                text.lines
                    .iter()
                    .skip(total_lines.saturating_sub(preview_lines))
                    .cloned(),
            );
        }

        Text::from(lines)
    };

    let paragraph = Paragraph::new(display_text)
        .block(Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)));

    f.render_widget(paragraph, area);
}

fn normalize_preview_content(content: &str) -> String {
    // Normalize CRLF and stray carriage returns that can skew TUI layout.
    let normalized = content.replace("\r\n", "\n").replace('\r', "");

    // Strip OSC 8 hyperlink sequences which break ansi-to-tui parsing
    // Format: ESC ] 8 ; ; URL (BEL or ESC \) for opening
    //         ESC ] 8 ; ; (BEL or ESC \) for closing
    let bytes = normalized.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        // Check for ESC ] 8 ; sequence
        if i + 4 <= bytes.len()
            && bytes[i] == 0x1b      // ESC
            && bytes[i + 1] == b']'   // ]
            && bytes[i + 2] == b'8'   // 8
            && bytes[i + 3] == b';'   // ;
        {
            // Skip the OSC 8 sequence until we find the terminator
            i += 4;
            while i < bytes.len() {
                if bytes[i] == 0x07 {
                    // BEL terminator
                    i += 1;
                    break;
                } else if i + 1 < bytes.len() && bytes[i] == 0x1b && bytes[i + 1] == b'\\' {
                    // ESC \ terminator
                    i += 2;
                    break;
                }
                i += 1;
            }
        } else {
            // Keep this byte
            result.push(bytes[i]);
            i += 1;
        }
    }

    String::from_utf8_lossy(&result).into_owned()
}

// Helper functions to convert ratatui_core types into ratatui types.
fn convert_color(color: ratatui_core::style::Color) -> Color {
    use ratatui_core::style::Color as CoreColor;
    match color {
        CoreColor::Reset => Color::Reset,
        CoreColor::Black => Color::Black,
        CoreColor::Red => Color::Red,
        CoreColor::Green => Color::Green,
        CoreColor::Yellow => Color::Yellow,
        CoreColor::Blue => Color::Blue,
        CoreColor::Magenta => Color::Magenta,
        CoreColor::Cyan => Color::Cyan,
        CoreColor::Gray => Color::Gray,
        CoreColor::DarkGray => Color::DarkGray,
        CoreColor::LightRed => Color::LightRed,
        CoreColor::LightGreen => Color::LightGreen,
        CoreColor::LightYellow => Color::LightYellow,
        CoreColor::LightBlue => Color::LightBlue,
        CoreColor::LightMagenta => Color::LightMagenta,
        CoreColor::LightCyan => Color::LightCyan,
        CoreColor::White => Color::White,
        CoreColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
        CoreColor::Indexed(i) => Color::Indexed(i),
    }
}

fn convert_modifier(modifier: ratatui_core::style::Modifier) -> Modifier {
    let mut result = Modifier::empty();

    if modifier.contains(ratatui_core::style::Modifier::BOLD) {
        result |= Modifier::BOLD;
    }
    if modifier.contains(ratatui_core::style::Modifier::DIM) {
        result |= Modifier::DIM;
    }
    if modifier.contains(ratatui_core::style::Modifier::ITALIC) {
        result |= Modifier::ITALIC;
    }
    if modifier.contains(ratatui_core::style::Modifier::UNDERLINED) {
        result |= Modifier::UNDERLINED;
    }
    if modifier.contains(ratatui_core::style::Modifier::SLOW_BLINK) {
        result |= Modifier::SLOW_BLINK;
    }
    if modifier.contains(ratatui_core::style::Modifier::RAPID_BLINK) {
        result |= Modifier::RAPID_BLINK;
    }
    if modifier.contains(ratatui_core::style::Modifier::REVERSED) {
        result |= Modifier::REVERSED;
    }
    if modifier.contains(ratatui_core::style::Modifier::HIDDEN) {
        result |= Modifier::HIDDEN;
    }
    if modifier.contains(ratatui_core::style::Modifier::CROSSED_OUT) {
        result |= Modifier::CROSSED_OUT;
    }

    result
}

fn convert_style(style: ratatui_core::style::Style) -> Style {
    let mut converted = Style::default();
    if let Some(fg) = style.fg {
        converted = converted.fg(convert_color(fg));
    }
    if let Some(bg) = style.bg {
        converted = converted.bg(convert_color(bg));
    }
    converted = converted.add_modifier(convert_modifier(style.add_modifier));
    converted = converted.remove_modifier(convert_modifier(style.sub_modifier));
    converted
}

fn convert_alignment(
    alignment: Option<ratatui_core::layout::Alignment>,
) -> Option<ratatui::layout::Alignment> {
    match alignment {
        Some(ratatui_core::layout::Alignment::Left) => Some(ratatui::layout::Alignment::Left),
        Some(ratatui_core::layout::Alignment::Center) => Some(ratatui::layout::Alignment::Center),
        Some(ratatui_core::layout::Alignment::Right) => Some(ratatui::layout::Alignment::Right),
        None => None,
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
        InputMode::RenamingDimension => {
            if let Some(msg) = &app.message {
                spans.push(Span::styled(msg.clone(), Style::default().fg(Color::Red)));
                spans.push(Span::raw("  "));
            }
            spans.push(Span::raw("Rename dimension: "));
            spans.push(Span::styled(
                app.input_buffer.clone(),
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::styled(" █", Style::default().fg(Color::White)));
        }
        InputMode::RenamingTab => {
            if let Some(msg) = &app.message {
                spans.push(Span::styled(msg.clone(), Style::default().fg(Color::Red)));
                spans.push(Span::raw("  "));
            }
            spans.push(Span::raw("Rename tab: "));
            spans.push(Span::styled(
                app.input_buffer.clone(),
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::styled(" █", Style::default().fg(Color::White)));
        }
        InputMode::CreatingDimensionDirectory => {
            spans.push(Span::raw("Directory: "));
            spans.push(Span::styled(
                app.input_buffer.clone(),
                Style::default().fg(Color::Cyan),
            ));
            spans.push(Span::styled(" █", Style::default().fg(Color::White)));

            // Show completion candidates if available, or hint to press Tab
            if !app.completion_candidates.is_empty() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!("[{}/{}]", app.completion_index + 1, app.completion_candidates.len()),
                    Style::default().fg(Color::DarkGray),
                ));
            } else {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    "Press Tab ⇥ for completion",
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                ));
            }
        }
        InputMode::Searching => {
            spans.push(Span::raw("Search: /"));
            spans.push(Span::styled(
                app.input_buffer.clone(),
                Style::default().fg(Color::Cyan),
            ));
            spans.push(Span::styled(" █", Style::default().fg(Color::White)));
        }
        InputMode::JumpingToTab => {
            spans.push(Span::raw("Jump to tab #"));
            spans.push(Span::styled(
                app.input_buffer.clone(),
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::styled(" █", Style::default().fg(Color::White)));
        }
        InputMode::DeletingDimension => {
            if let Some(dim) = app.get_current_dimension() {
                let is_current = app.current_session.as_deref() == Some(dim.name.as_str());
                let msg = if is_current && Tmux::session_exists(&dim.name) {
                    format!("Delete dimension '{}'? Will switch to first available tab (y/n)", dim.name)
                } else {
                    format!("Delete dimension '{}'? (y/n)", dim.name)
                };
                spans.push(Span::styled(msg, Style::default().fg(Color::Red)));
            }
        }
        InputMode::DeletingTab => {
            if let Some(dimension) = app.get_current_dimension() {
                if let Some(tab_index) = app.selected_tab {
                    let is_current_session =
                        app.current_session.as_deref() == Some(dimension.name.as_str());

                    let (tab_name, is_last) = if Tmux::session_exists(&dimension.name) {
                        let windows = Tmux::list_windows(&dimension.name).unwrap_or_default();
                        let name = windows
                            .iter()
                            .find(|(idx, _)| *idx == tab_index)
                            .map(|(_, name)| name.clone())
                            .unwrap_or_else(|| "unknown".to_string());
                        let is_last = windows.len() == 1;
                        (name, is_last)
                    } else {
                        let name = dimension
                            .configured_tabs
                            .get(tab_index)
                            .map(|t| t.name.clone())
                            .unwrap_or_else(|| "unknown".to_string());
                        let is_last = dimension.configured_tabs.len() == 1;
                        (name, is_last)
                    };

                    let msg = if is_last && is_current_session {
                        format!("Delete last tab '{}'? Will switch to first available tab (y/n)", tab_name)
                    } else {
                        format!("Delete tab '{}'? (y/n)", tab_name)
                    };

                    spans.push(Span::styled(msg, Style::default().fg(Color::Red)));
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
                Span::styled("r", Style::default().fg(Color::Yellow)),
                Span::raw(" Rename  "),
                Span::styled("/", Style::default().fg(Color::Yellow)),
                Span::raw(" Search  "),
                Span::styled(":", Style::default().fg(Color::Yellow)),
                Span::raw(" Jump  "),
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
        InputMode::CreatingDimensionDirectory => vec![
            Line::from(vec![
                Span::styled("Tab", Style::default().fg(Color::Yellow)),
                Span::raw(" Next  "),
                Span::styled("Shift+Tab", Style::default().fg(Color::Yellow)),
                Span::raw(" Prev  "),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::raw(" Submit (empty for none)  "),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::raw(" Cancel"),
            ]),
            Line::from(vec![
                Span::raw("Supports: "),
                Span::styled("~/path", Style::default().fg(Color::Cyan)),
                Span::raw(", "),
                Span::styled("../relative", Style::default().fg(Color::Cyan)),
                Span::raw(", "),
                Span::styled("$VAR/path", Style::default().fg(Color::Cyan)),
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
        InputMode::JumpingToTab => vec![
            Line::from(vec![
                Span::raw("Type window number to jump  "),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::raw(" Switch  "),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::raw(" Cancel"),
            ]),
        ],
        InputMode::RenamingDimension | InputMode::RenamingTab => vec![
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::raw(" Confirm  "),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::raw(" Cancel"),
            ]),
        ],
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

fn render_completion_overlay(f: &mut Frame, app: &App, area: Rect) {
    let max_candidates = 3;
    let candidates_text: Vec<Line> = app.completion_candidates
        .iter()
        .enumerate()
        .take(max_candidates)
        .map(|(i, candidate)| {
            let style = if i == app.completion_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let marker = if i == app.completion_index { "→ " } else { "  " };
            Line::from(vec![
                Span::styled(marker, style),
                Span::styled(candidate.clone(), style),
            ])
        })
        .collect();

    let remaining = app.completion_candidates.len().saturating_sub(max_candidates);
    let mut lines = candidates_text;
    if remaining > 0 {
        lines.push(Line::from(Span::styled(
            format!("  ... {} more", remaining),
            Style::default().fg(Color::DarkGray),
        )));
    }

    let completion_widget = Paragraph::new(lines)
        .block(Block::default().title("Matches (Tab to cycle)").borders(Borders::ALL));

    f.render_widget(completion_widget, area);
}
