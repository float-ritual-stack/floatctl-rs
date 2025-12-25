//! UI rendering using ratatui

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

use super::app::{App, FocusedPane, MainTab, Mode};

/// Primary accent color
const ACCENT: Color = Color::Cyan;
/// Secondary color for less important elements
const SECONDARY: Color = Color::DarkGray;
/// Highlight color for selected items
const HIGHLIGHT: Color = Color::Yellow;
/// Success color
const SUCCESS: Color = Color::Green;
/// Dim text color
const DIM: Color = Color::Rgb(100, 100, 100);

/// Render the entire UI
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Main layout: scratch pane (left) + main area (right)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // Scratch pane
            Constraint::Percentage(75), // Main area
        ])
        .split(area);

    // Render scratch pane
    render_scratch_pane(frame, app, main_chunks[0]);

    // Main area layout: header + content + status
    let main_area_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Tab header
            Constraint::Min(10),    // Content area
            Constraint::Length(1),  // Status bar
        ])
        .split(main_chunks[1]);

    // Render tab header
    render_tabs(frame, app, main_area_chunks[0]);

    // Content area: list + preview (if enabled)
    if app.show_preview {
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(main_area_chunks[1]);

        render_list(frame, app, content_chunks[0]);
        render_preview(frame, app, content_chunks[1]);
    } else {
        render_list(frame, app, main_area_chunks[1]);
    }

    // Render status bar
    render_status_bar(frame, app, main_area_chunks[2]);

    // Render overlays (action palette, etc.)
    if app.mode == Mode::ActionPalette {
        render_action_palette(frame, app);
    }

    // Render search/filter input if in search mode
    if app.mode == Mode::Search {
        render_filter_input(frame, app);
    }

    // Render help overlay if showing
    if app.help_text.is_some() {
        render_help_overlay(frame, app);
    }
}

/// Render the scratch pane
fn render_scratch_pane(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focused_pane == FocusedPane::Scratch;
    let is_editing = app.mode == Mode::Edit && is_focused;

    let border_style = if is_focused {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(SECONDARY)
    };

    let title = if is_editing {
        " Scratch [EDIT] "
    } else {
        " Scratch "
    };

    let block = Block::default()
        .title(title)
        .title_style(if is_focused {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(SECONDARY)
        })
        .borders(Borders::ALL)
        .border_style(border_style);

    let content = if app.scratch_content.is_empty() && !is_editing {
        Text::from(vec![
            Line::from(Span::styled("Press 'i' to edit", Style::default().fg(DIM))),
            Line::from(""),
            Line::from(Span::styled("Commands:", Style::default().fg(DIM))),
            Line::from(Span::styled("  /search <query>", Style::default().fg(DIM))),
            Line::from(Span::styled("  /board <name>", Style::default().fg(DIM))),
            Line::from(Span::styled("  /rag <query>", Style::default().fg(DIM))),
            Line::from(Span::styled("  /help or ?", Style::default().fg(DIM))),
        ])
    } else {
        // Show content with cursor if editing
        let mut lines = Vec::new();
        if is_editing {
            let before = &app.scratch_content[..app.scratch_cursor];
            let after = &app.scratch_content[app.scratch_cursor..];
            lines.push(Line::from(format!("{}|{}", before, after)));

            // Show completions if available
            if !app.completions.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled("Completions (Tab):", Style::default().fg(DIM))));
                for comp in app.completions.iter().take(5) {
                    lines.push(Line::from(Span::styled(format!("  {}", comp), Style::default().fg(ACCENT))));
                }
            }
            Text::from(lines)
        } else {
            Text::from(app.scratch_content.as_str())
        }
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Render the tab header
fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles = vec!["1:Home", "2:Boards", "3:Search", "4:Dashboard"];
    let selected = match app.main_tab {
        MainTab::Home => 0,
        MainTab::Boards => 1,
        MainTab::Search => 2,
        MainTab::Dashboard => 3,
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .title(" Float Control ")
                .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(SECONDARY)),
        )
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD));

    frame.render_widget(tabs, area);
}

/// Render the list navigator
fn render_list(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focused_pane == FocusedPane::Main && app.mode != Mode::ActionPalette;

    // Get displayed items (filtered if search active)
    let displayed = app.displayed_items();

    // Build title with navigation breadcrumb and filter info
    let title = if app.search_state.active && !app.search_state.query.is_empty() {
        format!(" Filter: '{}' ({} results) ", app.search_state.query, displayed.len())
    } else if let Some(nav) = &app.current_nav {
        format!(" {} :: {} ", nav.kind, nav.title)
    } else {
        format!(" {} ", tab_title(app.main_tab))
    };

    let border_style = if is_focused {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(SECONDARY)
    };

    let block = Block::default()
        .title(title)
        .title_style(if is_focused {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(SECONDARY)
        })
        .borders(Borders::ALL)
        .border_style(border_style);

    // Calculate visible area
    let inner = block.inner(area);
    let visible_height = inner.height as usize;

    // Build list items from displayed (filtered) items
    let items: Vec<ListItem> = displayed
        .iter()
        .enumerate()
        .skip(app.scroll_offset)
        .take(visible_height)
        .map(|(idx, item)| {
            let is_selected = idx == app.selected_index;
            let icon = item.kind.icon();
            let arrow = if item.has_children { " >" } else { "" };

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let content = if let Some(ref sub) = item.subtitle {
                format!("{} {} - {}{}", icon, item.title, sub, arrow)
            } else {
                format!("{} {}{}", icon, item.title, arrow)
            };

            ListItem::new(Line::from(Span::styled(content, style)))
        })
        .collect();

    // Show placeholder if empty
    let list = if items.is_empty() {
        let placeholder_text = if app.search_state.active {
            "  No matches"
        } else {
            "  No items"
        };
        let placeholder = ListItem::new(Line::from(Span::styled(
            placeholder_text,
            Style::default().fg(DIM),
        )));
        List::new(vec![placeholder]).block(block)
    } else {
        List::new(items).block(block)
    };

    frame.render_widget(list, area);

    // Show scroll indicator
    if displayed.len() > visible_height {
        let indicator = format!(
            " {}/{} ",
            app.selected_index + 1,
            displayed.len()
        );
        let indicator_area = Rect {
            x: area.x + area.width.saturating_sub(indicator.len() as u16 + 2),
            y: area.y,
            width: indicator.len() as u16 + 2,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(indicator).style(Style::default().fg(DIM)),
            indicator_area,
        );
    }
}

/// Render the preview pane
fn render_preview(frame: &mut Frame, app: &App, area: Rect) {
    let title = if let Some(item) = app.selected_item() {
        format!(" Preview: {} ", item.title)
    } else {
        " Preview ".to_string()
    };

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(SECONDARY))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(SECONDARY));

    let content = app
        .preview_content
        .as_deref()
        .unwrap_or("Select an item to preview");

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}

/// Render the status bar
fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mode_indicator = match app.mode {
        Mode::Normal => Span::styled(" NORMAL ", Style::default().bg(ACCENT).fg(Color::Black)),
        Mode::Edit => Span::styled(" EDIT ", Style::default().bg(SUCCESS).fg(Color::Black)),
        Mode::ActionPalette => {
            Span::styled(" ACTION ", Style::default().bg(HIGHLIGHT).fg(Color::Black))
        }
        Mode::Search => Span::styled(" SEARCH ", Style::default().bg(Color::Magenta).fg(Color::Black)),
    };

    let help_text = match app.mode {
        Mode::Normal => "j/k:nav  Enter:select  a:actions  /:filter  ?:help  Tab:pane  q:quit",
        Mode::Edit => "Esc:exit  Enter:exec cmd  Tab:complete",
        Mode::ActionPalette => "j/k:nav  Enter:execute  1-9:quick  Esc:cancel",
        Mode::Search => "Type to filter  Ctrl+n/p:nav  Enter:apply  Esc:cancel",
    };

    let status = if let Some(ref msg) = app.status_message {
        msg.as_str()
    } else {
        ""
    };

    let line = Line::from(vec![
        mode_indicator,
        Span::raw(" "),
        Span::styled(help_text, Style::default().fg(DIM)),
        Span::raw(" "),
        Span::styled(status, Style::default().fg(HIGHLIGHT)),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

/// Render the action palette overlay
fn render_action_palette(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Center the palette
    let width = 50.min(area.width.saturating_sub(4));
    let height = (app.action_items.len() + 4).min(20) as u16;

    let popup_area = Rect {
        x: (area.width.saturating_sub(width)) / 2,
        y: (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    // Clear the area
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Actions ")
        .title_style(Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(HIGHLIGHT));

    let items: Vec<ListItem> = app
        .action_items
        .iter()
        .enumerate()
        .map(|(idx, action)| {
            let is_selected = idx == app.action_selected;
            let shortcut = format!("[{}] ", idx + 1);

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(HIGHLIGHT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let content = format!("{}{} - {}", shortcut, action.name, action.description);
            ListItem::new(Line::from(Span::styled(content, style)))
        })
        .collect();

    let list = if items.is_empty() {
        let placeholder = ListItem::new(Line::from(Span::styled(
            "  No actions available",
            Style::default().fg(DIM),
        )));
        List::new(vec![placeholder]).block(block)
    } else {
        List::new(items).block(block)
    };

    frame.render_widget(list, popup_area);
}

/// Render filter input overlay
fn render_filter_input(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let width = 60.min(area.width.saturating_sub(4));
    let popup_area = Rect {
        x: (area.width.saturating_sub(width)) / 2,
        y: 2, // Near top for quick filtering
        width,
        height: 3,
    };

    frame.render_widget(Clear, popup_area);

    let result_count = app.search_state.results.len();
    let title = format!(" Filter ({} matches) ", result_count);

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    // Show input with cursor
    let before = &app.search_input[..app.search_cursor];
    let after = &app.search_input[app.search_cursor..];
    let content = format!("{}|{}", before, after);

    let paragraph = Paragraph::new(content).block(block);

    frame.render_widget(paragraph, popup_area);
}

/// Render help overlay
fn render_help_overlay(frame: &mut Frame, app: &App) {
    let area = frame.area();

    if let Some(ref help_text) = app.help_text {
        let lines: Vec<&str> = help_text.lines().collect();
        let height = (lines.len() + 4).min(area.height as usize - 4) as u16;
        let width = 70.min(area.width.saturating_sub(4));

        let popup_area = Rect {
            x: (area.width.saturating_sub(width)) / 2,
            y: (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(" Help (press any key to close) ")
            .title_style(Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(SUCCESS));

        let paragraph = Paragraph::new(help_text.as_str())
            .block(block)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::White));

        frame.render_widget(paragraph, popup_area);
    }
}

/// Get display title for a tab
fn tab_title(tab: MainTab) -> &'static str {
    match tab {
        MainTab::Home => "Home",
        MainTab::Boards => "Boards",
        MainTab::Search => "Search",
        MainTab::Dashboard => "Dashboard",
    }
}
