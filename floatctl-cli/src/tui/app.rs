//! Core application state and mode management

use std::collections::VecDeque;

/// Input mode for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Navigation mode - navigate lists, invoke actions, switch panes
    #[default]
    Normal,
    /// Edit mode - focused text input (scratch, search, forms)
    Edit,
    /// Action palette is open
    ActionPalette,
    /// Search/filter input active
    Search,
}

/// Which pane is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPane {
    /// Left scratch pane
    Scratch,
    /// Main content area (right side)
    #[default]
    Main,
}

/// Active tab in the main pane
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MainTab {
    /// Home dashboard - recent sessions, notifications, daily note
    #[default]
    Home,
    /// Boards browser - hierarchical board navigation
    Boards,
    /// Search results
    Search,
    /// System dashboards - tasks, activity, jobs
    Dashboard,
}

/// Navigation stack entry for hierarchical browsing
#[derive(Debug, Clone)]
pub struct NavEntry {
    /// Type of view (board, post, folder, etc.)
    pub kind: String,
    /// Display title
    pub title: String,
    /// ID or path for the entry
    pub id: String,
    /// Selected index when we left this view
    pub selected_index: usize,
}

/// Main application state
#[derive(Debug)]
pub struct App {
    /// Current input mode
    pub mode: Mode,
    /// Which pane is focused
    pub focused_pane: FocusedPane,
    /// Active tab in main pane
    pub main_tab: MainTab,
    /// Navigation stack for back/forward
    pub nav_stack: VecDeque<NavEntry>,
    /// Current navigation context (what we're viewing)
    pub current_nav: Option<NavEntry>,
    /// Scratch pane text content
    pub scratch_content: String,
    /// Scratch cursor position
    pub scratch_cursor: usize,
    /// Search/filter input
    pub search_input: String,
    /// Search cursor position
    pub search_cursor: usize,
    /// Currently selected item index in list
    pub selected_index: usize,
    /// Scroll offset for list view
    pub scroll_offset: usize,
    /// Total items in current list
    pub total_items: usize,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Status message (shown in status bar)
    pub status_message: Option<String>,
    /// Action palette items (when open)
    pub action_items: Vec<ActionItem>,
    /// Selected action in palette
    pub action_selected: usize,
    /// Preview content for current selection
    pub preview_content: Option<String>,
    /// Show preview pane
    pub show_preview: bool,
    /// List items for current view
    pub list_items: Vec<ListItem>,
}

/// An item in the list navigator
#[derive(Debug, Clone)]
pub struct ListItem {
    /// Unique identifier
    pub id: String,
    /// Display title
    pub title: String,
    /// Optional subtitle/description
    pub subtitle: Option<String>,
    /// Item kind (for icons/styling)
    pub kind: ItemKind,
    /// Whether this item can be navigated into
    pub has_children: bool,
    /// Optional metadata
    pub meta: Option<String>,
}

/// Kind of list item (for styling/icons)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Board,
    Post,
    Folder,
    File,
    Note,
    Session,
    Task,
    Notification,
    Action,
}

impl ItemKind {
    /// Get icon for this item kind
    pub fn icon(&self) -> &'static str {
        match self {
            ItemKind::Board => "📋",
            ItemKind::Post => "📄",
            ItemKind::Folder => "📁",
            ItemKind::File => "📝",
            ItemKind::Note => "🗒️",
            ItemKind::Session => "💻",
            ItemKind::Task => "☐",
            ItemKind::Notification => "🔔",
            ItemKind::Action => "⚡",
        }
    }
}

/// An action in the action palette
#[derive(Debug, Clone)]
pub struct ActionItem {
    /// Action identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Keyboard shortcut hint
    pub shortcut: Option<String>,
    /// Description
    pub description: String,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Create a new App instance
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            focused_pane: FocusedPane::Main,
            main_tab: MainTab::Home,
            nav_stack: VecDeque::new(),
            current_nav: None,
            scratch_content: String::new(),
            scratch_cursor: 0,
            search_input: String::new(),
            search_cursor: 0,
            selected_index: 0,
            scroll_offset: 0,
            total_items: 0,
            should_quit: false,
            status_message: None,
            action_items: Vec::new(),
            action_selected: 0,
            preview_content: None,
            show_preview: true,
            list_items: Vec::new(),
        }
    }

    /// Set status message
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Switch to a tab
    pub fn switch_tab(&mut self, tab: MainTab) {
        self.main_tab = tab;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// Toggle focused pane
    pub fn toggle_pane(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::Scratch => FocusedPane::Main,
            FocusedPane::Main => FocusedPane::Scratch,
        };
    }

    /// Enter edit mode for scratch pane
    pub fn enter_scratch_edit(&mut self) {
        self.focused_pane = FocusedPane::Scratch;
        self.mode = Mode::Edit;
    }

    /// Enter search mode
    pub fn enter_search(&mut self) {
        self.mode = Mode::Search;
        self.search_input.clear();
        self.search_cursor = 0;
    }

    /// Exit current mode back to normal
    pub fn exit_mode(&mut self) {
        self.mode = Mode::Normal;
    }

    /// Open action palette for current selection
    pub fn open_action_palette(&mut self) {
        self.mode = Mode::ActionPalette;
        self.action_selected = 0;
        // Actions will be populated by the source
    }

    /// Close action palette
    pub fn close_action_palette(&mut self) {
        self.mode = Mode::Normal;
        self.action_items.clear();
    }

    /// Navigate into a child item
    pub fn navigate_into(&mut self, entry: NavEntry) {
        // Save current position
        if let Some(current) = self.current_nav.take() {
            let mut saved = current;
            saved.selected_index = self.selected_index;
            self.nav_stack.push_back(saved);
        }

        // Navigate to new entry
        self.current_nav = Some(entry);
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// Navigate back to parent
    pub fn navigate_back(&mut self) -> bool {
        if let Some(prev) = self.nav_stack.pop_back() {
            self.current_nav = Some(prev.clone());
            self.selected_index = prev.selected_index;
            self.scroll_offset = self.selected_index.saturating_sub(5);
            true
        } else {
            false
        }
    }

    /// Select next item in list
    pub fn select_next(&mut self) {
        if self.mode == Mode::ActionPalette {
            if !self.action_items.is_empty() {
                self.action_selected = (self.action_selected + 1) % self.action_items.len();
            }
        } else if !self.list_items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.list_items.len();
            self.ensure_visible();
        }
    }

    /// Select previous item in list
    pub fn select_prev(&mut self) {
        if self.mode == Mode::ActionPalette {
            if !self.action_items.is_empty() {
                self.action_selected = self.action_selected
                    .checked_sub(1)
                    .unwrap_or(self.action_items.len().saturating_sub(1));
            }
        } else if !self.list_items.is_empty() {
            self.selected_index = self.selected_index
                .checked_sub(1)
                .unwrap_or(self.list_items.len().saturating_sub(1));
            self.ensure_visible();
        }
    }

    /// Ensure selected item is visible
    fn ensure_visible(&mut self) {
        // Keep 2 items of context when scrolling
        const CONTEXT: usize = 2;

        if self.selected_index < self.scroll_offset + CONTEXT {
            self.scroll_offset = self.selected_index.saturating_sub(CONTEXT);
        }
        // Note: max visible will be calculated based on actual render height
    }

    /// Get currently selected item
    pub fn selected_item(&self) -> Option<&ListItem> {
        self.list_items.get(self.selected_index)
    }

    /// Add text to scratch pane
    pub fn scratch_insert(&mut self, c: char) {
        self.scratch_content.insert(self.scratch_cursor, c);
        self.scratch_cursor += 1;
    }

    /// Delete char before cursor in scratch
    pub fn scratch_backspace(&mut self) {
        if self.scratch_cursor > 0 {
            self.scratch_cursor -= 1;
            self.scratch_content.remove(self.scratch_cursor);
        }
    }

    /// Add text to search input
    pub fn search_insert(&mut self, c: char) {
        self.search_input.insert(self.search_cursor, c);
        self.search_cursor += 1;
    }

    /// Delete char before cursor in search
    pub fn search_backspace(&mut self) {
        if self.search_cursor > 0 {
            self.search_cursor -= 1;
            self.search_input.remove(self.search_cursor);
        }
    }
}
