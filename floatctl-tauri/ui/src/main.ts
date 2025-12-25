/**
 * Float Control - Main Entry Point
 *
 * Initializes the application, sets up stores, and renders the initial view.
 */

import './styles/base.css';
import './styles/layout.css';
import './styles/components.css';

import { navigationStore } from './stores/navigation';
import { itemsStore } from './stores/items';
import { VirtualList, defaultItemRenderer } from './components/VirtualList';
import { keyHandler } from './components/KeyHandler';
import type { Item, Mode } from './lib/protocol';

// Application state
let virtualList: VirtualList | null = null;

/** Initialize the application */
async function init(): Promise<void> {
  // Create main layout
  const app = document.getElementById('app');
  if (!app) {
    console.error('App container not found');
    return;
  }

  app.innerHTML = `
    <header class="header">
      <span class="header-title">Float Control</span>
      <div class="header-actions">
        <button id="btn-refresh" class="action-button">
          Refresh
          <span class="action-button-shortcut">r</span>
        </button>
      </div>
    </header>

    <aside class="scratch">
      <div class="scratch-editor" id="scratch-editor" contenteditable="true"></div>
      <div class="scratch-input">
        <input type="text" id="scratch-input" placeholder="Type a command..." />
      </div>
    </aside>

    <main class="stage">
      <div class="stage-header">
        <nav class="breadcrumb" id="breadcrumb">
          <span class="breadcrumb-item active">Boards</span>
        </nav>
      </div>
      <div class="stage-content" id="list-container"></div>
    </main>

    <footer class="status-bar">
      <div class="status-mode" id="status-mode">
        <span>NORMAL</span>
      </div>
      <div class="status-info">
        <span class="status-item" id="status-count">0 items</span>
        <span class="status-item" id="status-cursor">0/0</span>
      </div>
    </footer>
  `;

  // Setup virtual list
  const listContainer = document.getElementById('list-container');
  if (listContainer) {
    virtualList = new VirtualList({
      container: listContainer,
      itemHeight: 48,
      renderItem: defaultItemRenderer,
      onItemClick: (item: Item, index: number) => {
        console.log('Clicked:', item.id, index);
      },
      onItemSelect: (item: Item) => {
        if (item.has_children) {
          navigationStore.enter(item.id);
          loadItems();
        }
      },
    });
  }

  // Subscribe to store updates
  navigationStore.subscribe(updateStatusBar);
  itemsStore.subscribeItems(updateList);
  itemsStore.subscribeLoading(updateLoading);

  // Setup event handlers
  setupEventHandlers();

  // Load initial data
  await loadItems();

  // Sync navigation state from backend
  await navigationStore.sync();

  console.log('Float Control initialized');
}

/** Load items based on current navigation state */
async function loadItems(): Promise<void> {
  const navState = navigationStore.current;
  const parentId = navState.cursor.path[navState.cursor.path.length - 1];

  await itemsStore.fetch('bbs', {
    parent_id: parentId,
  });
}

/** Update the virtual list with new items */
function updateList(items: Item[]): void {
  if (virtualList) {
    virtualList.setItems(items);
  }
  updateBreadcrumb();
}

/** Update loading state */
function updateLoading(loading: boolean): void {
  const container = document.getElementById('list-container');
  if (container) {
    if (loading) {
      container.classList.add('loading');
    } else {
      container.classList.remove('loading');
    }
  }
}

/** Update status bar */
function updateStatusBar(): void {
  const navState = navigationStore.current;
  const items = itemsStore.current;

  // Update mode indicator
  const modeEl = document.getElementById('status-mode');
  if (modeEl) {
    modeEl.className = `status-mode ${navState.mode}`;
    modeEl.textContent = navState.mode.toUpperCase();
  }

  // Update item count
  const countEl = document.getElementById('status-count');
  if (countEl) {
    countEl.textContent = `${items.length} items`;
  }

  // Update cursor position
  const cursorEl = document.getElementById('status-cursor');
  if (cursorEl) {
    cursorEl.textContent = `${navState.cursor.index + 1}/${items.length}`;
  }
}

/** Update breadcrumb navigation */
function updateBreadcrumb(): void {
  const breadcrumb = document.getElementById('breadcrumb');
  const navState = navigationStore.current;

  if (!breadcrumb) return;

  const parts = ['Boards', ...navState.cursor.path];
  breadcrumb.innerHTML = parts
    .map((part, i) => {
      const isLast = i === parts.length - 1;
      const itemHtml = `<span class="breadcrumb-item ${isLast ? 'active' : ''}">${part}</span>`;

      if (isLast) {
        return itemHtml;
      }
      return `${itemHtml}<span class="breadcrumb-separator">/</span>`;
    })
    .join('');
}

/** Setup event handlers */
function setupEventHandlers(): void {
  // Refresh button
  const refreshBtn = document.getElementById('btn-refresh');
  if (refreshBtn) {
    refreshBtn.addEventListener('click', loadItems);
  }

  // Scratch input
  const scratchInput = document.getElementById('scratch-input') as HTMLInputElement;
  if (scratchInput) {
    scratchInput.addEventListener('keydown', async (e) => {
      if (e.key === 'Enter') {
        const value = scratchInput.value.trim();
        if (value) {
          // Parse and execute command
          console.log('Scratch command:', value);
          scratchInput.value = '';

          // Add to scratch editor
          const editor = document.getElementById('scratch-editor');
          if (editor) {
            editor.textContent += `> ${value}\n`;
            editor.scrollTop = editor.scrollHeight;
          }
        }
      } else if (e.key === 'Escape') {
        scratchInput.blur();
        navigationStore.setMode('normal');
      }
    });

    // Focus scratch input when entering edit mode in scratch area
    scratchInput.addEventListener('focus', () => {
      navigationStore.setMode('edit');
    });
  }

  // Handle 'r' key for refresh in normal mode
  keyHandler.bind('r', 'normal', loadItems);

  // Back navigation with backspace
  keyHandler.bind('Backspace', 'normal', () => {
    navigationStore.back();
    loadItems();
  });
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}
