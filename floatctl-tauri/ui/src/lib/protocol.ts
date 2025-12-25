/**
 * Protocol definitions for Tauri frontend/backend communication
 *
 * These types mirror the Rust structs in src/protocol.rs
 * All types are serializable for JSON transport via Tauri commands/events.
 */

// ============================================================================
// Core Item Types - Hierarchical data model (Boards → Posts → Actions)
// ============================================================================

/** The kind of navigable item in the hierarchy */
export type ItemKind =
  | 'board'
  | 'post'
  | 'job'
  | 'file'
  | 'search_result'
  | 'persona';

/** A navigable item in the hierarchical browser */
export interface Item {
  /** Unique identifier */
  id: string;

  /** Item type for rendering/routing */
  kind: ItemKind;

  /** Display title */
  title: string;

  /** Optional subtitle (e.g., timestamp, author) */
  subtitle?: string;

  /** Available action IDs for this item */
  actions: string[];

  /** Parent ID for hierarchy traversal */
  parent_id?: string;

  /** Whether item has children (expandable) */
  has_children: boolean;

  /** Badge/indicator text (e.g., unread count) */
  badge?: string;

  /** Additional metadata for item-specific rendering */
  meta: Record<string, unknown>;
}

// ============================================================================
// Source - Data Provider Abstraction
// ============================================================================

/** Scope constraints for data queries */
export interface Scope {
  /** Filter by parent ID (e.g., show posts in board) */
  parent_id?: string;

  /** Filter by item kinds */
  kinds?: ItemKind[];

  /** Active folder/project context */
  active_context?: string;

  /** Search query (for RAG sources) */
  query?: string;

  /** Limit results */
  limit?: number;

  /** Offset for pagination */
  offset?: number;
}

/** Response from a source fetch */
export interface SourceResponse {
  /** Fetched items */
  items: Item[];

  /** Total count (for pagination) */
  total?: number;

  /** Source identifier */
  source: string;

  /** Whether more items available */
  has_more: boolean;
}

/** Identifies a data source type */
export type SourceKind = 'bbs' | 'filesystem' | 'search' | 'jobs' | 'static';

// ============================================================================
// Action - Command Execution
// ============================================================================

/** Action that can be executed on an item */
export interface Action {
  /** Unique action identifier */
  id: string;

  /** Display label */
  label: string;

  /** Keyboard shortcut (e.g., "Enter", "Space", "e") */
  shortcut?: string;

  /** Icon identifier (for UI rendering) */
  icon?: string;

  /** Whether action is destructive (shows warning) */
  destructive: boolean;

  /** Whether action runs in background */
  background: boolean;
}

/** Request to execute an action */
export interface ActionRequest {
  /** Action ID to execute */
  action_id: string;

  /** Target item ID */
  item_id: string;

  /** Additional parameters */
  params: Record<string, unknown>;
}

/** Result of action execution */
export type ActionResult =
  | { status: 'success'; message?: string; navigate_to?: string }
  | { status: 'job_started'; job_id: string }
  | { status: 'error'; message: string };

// ============================================================================
// Navigation State - Mode-based UI
// ============================================================================

/** Current UI mode (vim-like modal editing) */
export type Mode = 'normal' | 'edit' | 'visual' | 'command';

/** Navigation cursor position */
export interface Cursor {
  /** Current item ID under cursor */
  item_id?: string;

  /** Index in current list */
  index: number;

  /** Hierarchy depth (0 = root boards) */
  depth: number;

  /** Parent path for breadcrumb */
  path: string[];
}

/** Full navigation state synced between backend and frontend */
export interface NavigationState {
  /** Current UI mode */
  mode: Mode;

  /** Cursor position */
  cursor: Cursor;

  /** Active source kind */
  source?: SourceKind;

  /** Active view/route */
  view?: string;
}

// ============================================================================
// Job Status - Background task tracking
// ============================================================================

/** Status of a background job */
export type JobStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

/** Job progress update (emitted via Tauri events) */
export interface JobProgress {
  /** Job ID */
  job_id: string;

  /** Current status */
  status: JobStatus;

  /** Progress percentage (0-100) */
  progress?: number;

  /** Status message */
  message?: string;

  /** Result data (when completed) */
  result?: unknown;
}

// ============================================================================
// Scratch Commands - Parsed from scratch pane input
// ============================================================================

/** Command parsed from scratch pane */
export type ScratchCommand =
  | { type: 'search'; query: string }
  | { type: 'navigate'; target: string }
  | { type: 'ask'; question: string }
  | { type: 'shell'; command: string }
  | { type: 'text'; content: string };
