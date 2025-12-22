use crate::app::{App, InputMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
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
    let title = Paragraph::new("🌌 Dimensions - Tab Groups for Ghostty")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn render_main_content(f: &mut Frame, app: &App, area: Rect) {
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

fn render_dimensions_list(f: &mut Frame, app: &App, area: Rect) {
    let dimensions: Vec<ListItem> = app
        .config
        .dimensions
        .iter()
        .enumerate()
        .map(|(i, dim)| {
            let is_selected = i == app.selected_dimension;
            let is_active = app.config.active_dimension.as_ref() == Some(&dim.name);
            let collapse_icon = if dim.collapsed { "▶" } else { "▼" };
            let active_marker = if is_active { " [active]" } else { "" };

            let content = format!(
                "{} {} ({} tabs){}",
                collapse_icon,
                dim.name,
                dim.tabs.len(),
                active_marker
            );

            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::default().fg(Color::Green)
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

    f.render_widget(list, area);
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

        let tabs: Vec<ListItem> = dimension
            .tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let is_selected = app.selected_tab == Some(i);
                let command_text = tab
                    .command
                    .as_ref()
                    .map(|c| format!(" ({})", c))
                    .unwrap_or_default();

                let content = format!("{}. {}{}", i + 1, tab.name, command_text);

                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(content).style(style)
            })
            .collect();

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

        f.render_widget(list, area);
    } else {
        let text = Paragraph::new("No dimension selected")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().title("Tabs").borders(Borders::ALL));
        f.render_widget(text, area);
    }
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
                Span::raw(" Switch to dimension  "),
                Span::styled("n", Style::default().fg(Color::Yellow)),
                Span::raw(" New dimension  "),
                Span::styled("t", Style::default().fg(Color::Yellow)),
                Span::raw(" New tab  "),
                Span::styled("d", Style::default().fg(Color::Yellow)),
                Span::raw(" Delete  "),
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
