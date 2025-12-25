/**
 * Navigation state store
 *
 * Manages the global navigation state including mode, cursor, and view.
 * Syncs with the Rust backend for persistence and cross-window state.
 */

import type { NavigationState, Mode, Cursor, SourceKind, Item } from '../lib/protocol';
import { getNavigationState, setMode as setModeCmd, navigateTo as navigateToCmd } from '../lib/commands';

type Listener = (state: NavigationState) => void;

/** Navigation store singleton */
class NavigationStore {
  private state: NavigationState = {
    mode: 'normal',
    cursor: { index: 0, depth: 0, path: [] },
    source: undefined,
    view: undefined,
  };

  private listeners: Set<Listener> = new Set();

  /** Get current state */
  get current(): NavigationState {
    return this.state;
  }

  /** Subscribe to state changes */
  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    listener(this.state);
    return () => this.listeners.delete(listener);
  }

  /** Notify all listeners */
  private notify(): void {
    for (const listener of this.listeners) {
      listener(this.state);
    }
  }

  /** Sync state from backend */
  async sync(): Promise<void> {
    this.state = await getNavigationState();
    this.notify();
  }

  /** Set UI mode */
  async setMode(mode: Mode): Promise<void> {
    await setModeCmd(mode);
    this.state.mode = mode;
    this.notify();
  }

  /** Navigate to item */
  async navigateTo(target: string): Promise<void> {
    this.state = await navigateToCmd(target);
    this.notify();
  }

  /** Move cursor up */
  cursorUp(): void {
    this.state.cursor.index = Math.max(0, this.state.cursor.index - 1);
    this.notify();
  }

  /** Move cursor down */
  cursorDown(maxIndex: number): void {
    this.state.cursor.index = Math.min(maxIndex - 1, this.state.cursor.index + 1);
    this.notify();
  }

  /** Set cursor to specific index */
  setCursor(index: number, itemId?: string): void {
    this.state.cursor.index = index;
    this.state.cursor.item_id = itemId;
    this.notify();
  }

  /** Enter child level */
  enter(parentId: string): void {
    this.state.cursor.path.push(parentId);
    this.state.cursor.depth += 1;
    this.state.cursor.index = 0;
    this.state.cursor.item_id = undefined;
    this.notify();
  }

  /** Go back one level */
  back(): void {
    if (this.state.cursor.depth > 0) {
      this.state.cursor.path.pop();
      this.state.cursor.depth -= 1;
      this.state.cursor.index = 0;
      this.state.cursor.item_id = undefined;
      this.notify();
    }
  }

  /** Set active source */
  setSource(source: SourceKind): void {
    this.state.source = source;
    this.notify();
  }

  /** Set active view */
  setView(view: string): void {
    this.state.view = view;
    this.notify();
  }

  /** Jump to first item */
  jumpToFirst(): void {
    this.state.cursor.index = 0;
    this.notify();
  }

  /** Jump to last item */
  jumpToLast(maxIndex: number): void {
    this.state.cursor.index = Math.max(0, maxIndex - 1);
    this.notify();
  }
}

export const navigationStore = new NavigationStore();
