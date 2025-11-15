use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;
use crate::block::Block as ContentBlock;
use crate::mode::Pane;

/// Board panel - stateless component for rendering board content
pub struct BoardPanel;

impl BoardPanel {
    /// Create a new board panel
    pub fn new() -> Self {
        Self
    }

    /// Render the board panel with provided state
    pub fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        app: &App,
        blocks: &[ContentBlock],
        selected: usize,
    ) {
        // Border color based on focus
        let border_color = if app.focused_pane == Pane::Board {
            app.mode.color()
        } else {
            Color::DarkGray
        };

        let title = format!(" {} ", app.current_board);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color));

        if blocks.is_empty() {
            // Show empty state with helpful guidance
            let empty_msg = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No posts yet",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Press 'i' to enter Insert mode, then type:",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "  ctx::2025-11-15 @ 10:30 - your context entry",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Agents will post here based on your ctx:: entries",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .block(block)
            .alignment(Alignment::Center);

            f.render_widget(empty_msg, area);
        } else {
            // Render block list
            let items: Vec<ListItem> = blocks
                .iter()
                .enumerate()
                .map(|(idx, block)| {
                    let is_selected = idx == selected;
                    Self::render_block_item(block, is_selected)
                })
                .collect();

            let list = List::new(items).block(block);

            f.render_widget(list, area);
        }
    }

    /// Render a single block as a list item
    fn render_block_item(block: &ContentBlock, is_selected: bool) -> ListItem<'_> {
        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let content = match block {
            ContentBlock::Text { content, .. } => {
                format!("  {}", content)
            }

            ContentBlock::ContextEntry {
                marker, content, ..
            } => {
                let preview = content.first().map(|s| s.as_str()).unwrap_or("");
                format!("📝 {} - {}", marker, preview)
            }

            ContentBlock::AgentPost { agent, title, .. } => {
                let title_str = title.as_ref().map(|s| s.as_str()).unwrap_or("(no title)");
                format!("🤖 [{}] {}", agent, title_str)
            }

            ContentBlock::Code { language, .. } => {
                let lang = language.as_ref().map(|s| s.as_str()).unwrap_or("text");
                format!("```{}", lang)
            }

            ContentBlock::Link { display, .. } => {
                format!("🔗 {}", display)
            }

            ContentBlock::Component { component_type, .. } => {
                format!("⚙️  {}", component_type)
            }
        };

        ListItem::new(Line::from(Span::styled(content, style)))
    }
}

impl Default for BoardPanel {
    fn default() -> Self {
        Self::new()
    }
}
