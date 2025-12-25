/**
 * Tauri command bindings
 *
 * Type-safe wrappers around Tauri invoke() calls.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
  Item,
  ItemKind,
  Action,
  ActionRequest,
  ActionResult,
  NavigationState,
  Mode,
  Scope,
  SourceKind,
  SourceResponse,
  ScratchCommand,
} from './protocol';

/**
 * Fetch items from a source with optional scope constraints
 */
export async function fetchItems(
  source: SourceKind,
  scope?: Scope
): Promise<SourceResponse> {
  return invoke('fetch_items', { source, scope });
}

/**
 * Execute an action on an item
 */
export async function executeAction(
  request: ActionRequest
): Promise<ActionResult> {
  return invoke('execute_action', { request });
}

/**
 * Get current navigation state
 */
export async function getNavigationState(): Promise<NavigationState> {
  return invoke('get_navigation_state');
}

/**
 * Set the current UI mode
 */
export async function setMode(mode: Mode): Promise<void> {
  return invoke('set_mode', { mode });
}

/**
 * Navigate to a specific item or path
 */
export async function navigateTo(target: string): Promise<NavigationState> {
  return invoke('navigate_to', { target });
}

/**
 * Parse a scratch pane command
 */
export async function parseScratchCommand(
  input: string
): Promise<ScratchCommand> {
  return invoke('parse_scratch_command', { input });
}

/**
 * Execute a search query
 */
export async function search(
  query: string,
  scope?: Scope
): Promise<SourceResponse> {
  return invoke('search', { query, scope });
}

/**
 * Get available actions for an item kind
 */
export async function getActionsForItem(kind: ItemKind): Promise<Action[]> {
  return invoke('get_actions_for_item', { kind });
}
