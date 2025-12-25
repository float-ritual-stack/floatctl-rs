/**
 * KeyHandler - Global keyboard input manager
 *
 * Implements vim-like modal editing with Normal/Edit/Command modes.
 * Dispatches to NavigationStore for state updates.
 */

import type { Mode } from '../lib/protocol';
import { navigationStore } from '../stores/navigation';
import { itemsStore } from '../stores/items';
import { executeAction } from '../lib/commands';

type KeyBinding = {
  key: string;
  mode?: Mode | Mode[];
  shift?: boolean;
  ctrl?: boolean;
  meta?: boolean;
  action: () => void | Promise<void>;
};

/** Keyboard handler singleton */
class KeyHandler {
  private bindings: KeyBinding[] = [];
  private enabled = true;
  private commandBuffer = '';

  constructor() {
    this.setupDefaultBindings();
    this.attachListeners();
  }

  /** Setup default vim-like bindings */
  private setupDefaultBindings(): void {
    // Navigation (Normal mode)
    this.bind('j', 'normal', () => {
      navigationStore.cursorDown(itemsStore.current.length);
    });

    this.bind('k', 'normal', () => {
      navigationStore.cursorUp();
    });

    this.bind('ArrowDown', 'normal', () => {
      navigationStore.cursorDown(itemsStore.current.length);
    });

    this.bind('ArrowUp', 'normal', () => {
      navigationStore.cursorUp();
    });

    this.bind('l', 'normal', () => {
      const item = itemsStore.getByIndex(navigationStore.current.cursor.index);
      if (item?.has_children) {
        navigationStore.enter(item.id);
      }
    });

    this.bind('h', 'normal', () => {
      navigationStore.back();
    });

    this.bind('ArrowRight', 'normal', () => {
      const item = itemsStore.getByIndex(navigationStore.current.cursor.index);
      if (item?.has_children) {
        navigationStore.enter(item.id);
      }
    });

    this.bind('ArrowLeft', 'normal', () => {
      navigationStore.back();
    });

    // Jump navigation
    this.bind('g', 'normal', () => {
      // Wait for second 'g' for gg
      this.commandBuffer = 'g';
      setTimeout(() => {
        this.commandBuffer = '';
      }, 500);
    });

    this.bind('G', 'normal', () => {
      navigationStore.jumpToLast(itemsStore.current.length);
    });

    // Enter/Select
    this.bind('Enter', 'normal', async () => {
      const item = itemsStore.getByIndex(navigationStore.current.cursor.index);
      if (item) {
        if (item.has_children) {
          navigationStore.enter(item.id);
        } else {
          await executeAction({
            action_id: 'view',
            item_id: item.id,
            params: {},
          });
        }
      }
    });

    // Mode switching
    this.bind('i', 'normal', () => {
      navigationStore.setMode('edit');
    });

    this.bind(':', 'normal', () => {
      navigationStore.setMode('command');
    });

    this.bind('/', 'normal', () => {
      navigationStore.setMode('command');
    });

    this.bind('v', 'normal', () => {
      navigationStore.setMode('visual');
    });

    this.bind('Escape', ['edit', 'command', 'visual'], () => {
      navigationStore.setMode('normal');
    });

    // Action shortcuts
    this.bind(' ', 'normal', async () => {
      // Space = preview
      const item = itemsStore.getByIndex(navigationStore.current.cursor.index);
      if (item) {
        await executeAction({
          action_id: 'view',
          item_id: item.id,
          params: { preview: true },
        });
      }
    });

    this.bind('e', 'normal', async () => {
      const item = itemsStore.getByIndex(navigationStore.current.cursor.index);
      if (item) {
        await executeAction({
          action_id: 'edit_metadata',
          item_id: item.id,
          params: {},
        });
      }
    });

    this.bind('d', 'normal', async () => {
      const item = itemsStore.getByIndex(navigationStore.current.cursor.index);
      if (item) {
        await executeAction({
          action_id: 'dispatch',
          item_id: item.id,
          params: {},
        });
      }
    });
  }

  /** Register a key binding */
  bind(
    key: string,
    mode: Mode | Mode[],
    action: () => void | Promise<void>,
    modifiers?: { shift?: boolean; ctrl?: boolean; meta?: boolean }
  ): void {
    this.bindings.push({
      key,
      mode,
      action,
      ...modifiers,
    });
  }

  /** Attach global keyboard listener */
  private attachListeners(): void {
    document.addEventListener('keydown', this.handleKeyDown.bind(this));
  }

  /** Handle keydown event */
  private handleKeyDown(event: KeyboardEvent): void {
    if (!this.enabled) return;

    // Skip if typing in input/textarea (unless in normal mode)
    const target = event.target as HTMLElement;
    const isInput =
      target.tagName === 'INPUT' ||
      target.tagName === 'TEXTAREA' ||
      target.isContentEditable;

    const currentMode = navigationStore.current.mode;

    // In edit mode, let inputs handle keys (except Escape)
    if (currentMode === 'edit' && isInput && event.key !== 'Escape') {
      return;
    }

    // Check command buffer for 'gg'
    if (this.commandBuffer === 'g' && event.key === 'g') {
      navigationStore.jumpToFirst();
      this.commandBuffer = '';
      event.preventDefault();
      return;
    }

    // Find matching binding
    const binding = this.bindings.find((b) => {
      if (b.key !== event.key) return false;

      // Check mode
      const modes = Array.isArray(b.mode) ? b.mode : [b.mode];
      if (b.mode && !modes.includes(currentMode)) return false;

      // Check modifiers
      if (b.shift && !event.shiftKey) return false;
      if (b.ctrl && !event.ctrlKey) return false;
      if (b.meta && !event.metaKey) return false;

      return true;
    });

    if (binding) {
      event.preventDefault();
      binding.action();
    }
  }

  /** Enable/disable keyboard handling */
  setEnabled(enabled: boolean): void {
    this.enabled = enabled;
  }
}

export const keyHandler = new KeyHandler();
