/**
 * VirtualList - Headless virtualized list component
 *
 * Renders only visible items for performance with large datasets.
 * Keyboard navigation integrated via NavigationStore.
 */

import type { Item } from '../lib/protocol';
import { navigationStore } from '../stores/navigation';

interface VirtualListOptions {
  /** Container element */
  container: HTMLElement;

  /** Item height in pixels */
  itemHeight: number;

  /** Overscan count (extra items to render above/below viewport) */
  overscan?: number;

  /** Render function for each item */
  renderItem: (item: Item, index: number, isSelected: boolean) => HTMLElement;

  /** Click handler */
  onItemClick?: (item: Item, index: number) => void;

  /** Enter/select handler */
  onItemSelect?: (item: Item, index: number) => void;
}

interface VirtualListState {
  items: Item[];
  scrollTop: number;
  containerHeight: number;
}

export class VirtualList {
  private container: HTMLElement;
  private content: HTMLElement;
  private options: Required<VirtualListOptions>;
  private state: VirtualListState = {
    items: [],
    scrollTop: 0,
    containerHeight: 0,
  };
  private unsubscribeNav: (() => void) | null = null;

  constructor(options: VirtualListOptions) {
    this.options = {
      overscan: 3,
      onItemClick: () => {},
      onItemSelect: () => {},
      ...options,
    };

    this.container = options.container;

    // Create content wrapper for virtual scrolling
    this.content = document.createElement('div');
    this.content.className = 'virtual-list-content';
    this.content.style.position = 'relative';
    this.container.appendChild(this.content);

    // Setup styles
    this.container.style.overflow = 'auto';
    this.container.style.position = 'relative';

    // Bind scroll handler
    this.container.addEventListener('scroll', this.handleScroll.bind(this));

    // Track container resize
    if (typeof ResizeObserver !== 'undefined') {
      const observer = new ResizeObserver((entries) => {
        for (const entry of entries) {
          this.state.containerHeight = entry.contentRect.height;
          this.render();
        }
      });
      observer.observe(this.container);
    }

    // Subscribe to navigation state
    this.unsubscribeNav = navigationStore.subscribe((navState) => {
      this.scrollToIndex(navState.cursor.index);
      this.render();
    });
  }

  /** Set items to display */
  setItems(items: Item[]): void {
    this.state.items = items;

    // Update content height for scrollbar
    const totalHeight = items.length * this.options.itemHeight;
    this.content.style.height = `${totalHeight}px`;

    this.render();
  }

  /** Scroll to specific index */
  scrollToIndex(index: number): void {
    const targetTop = index * this.options.itemHeight;
    const viewportBottom = this.state.scrollTop + this.state.containerHeight;

    // Check if item is in view
    if (targetTop < this.state.scrollTop) {
      // Item is above viewport
      this.container.scrollTop = targetTop;
    } else if (targetTop + this.options.itemHeight > viewportBottom) {
      // Item is below viewport
      this.container.scrollTop =
        targetTop - this.state.containerHeight + this.options.itemHeight;
    }
  }

  /** Handle scroll events */
  private handleScroll(): void {
    this.state.scrollTop = this.container.scrollTop;
    this.render();
  }

  /** Render visible items */
  private render(): void {
    const { items, scrollTop, containerHeight } = this.state;
    const { itemHeight, overscan, renderItem, onItemClick } = this.options;

    if (items.length === 0 || containerHeight === 0) {
      this.content.innerHTML = '';
      return;
    }

    // Calculate visible range
    const startIndex = Math.max(0, Math.floor(scrollTop / itemHeight) - overscan);
    const endIndex = Math.min(
      items.length,
      Math.ceil((scrollTop + containerHeight) / itemHeight) + overscan
    );

    // Get current selection from navigation store
    const navState = navigationStore.current;
    const selectedIndex = navState.cursor.index;

    // Create document fragment for batch DOM update
    const fragment = document.createDocumentFragment();

    for (let i = startIndex; i < endIndex; i++) {
      const item = items[i];
      const isSelected = i === selectedIndex;

      const element = renderItem(item, i, isSelected);
      element.style.position = 'absolute';
      element.style.top = `${i * itemHeight}px`;
      element.style.left = '0';
      element.style.right = '0';
      element.style.height = `${itemHeight}px`;

      // Add click handler
      element.addEventListener('click', () => {
        navigationStore.setCursor(i, item.id);
        onItemClick(item, i);
      });

      // Add double-click for select
      element.addEventListener('dblclick', () => {
        this.options.onItemSelect(item, i);
      });

      fragment.appendChild(element);
    }

    // Replace content
    this.content.innerHTML = '';
    this.content.appendChild(fragment);
  }

  /** Cleanup */
  destroy(): void {
    if (this.unsubscribeNav) {
      this.unsubscribeNav();
    }
    this.container.innerHTML = '';
  }
}

/** Default item renderer */
export function defaultItemRenderer(
  item: Item,
  index: number,
  isSelected: boolean
): HTMLElement {
  const el = document.createElement('div');
  el.className = `list-item ${isSelected ? 'selected' : ''}`;
  el.dataset.id = item.id;
  el.dataset.kind = item.kind;

  el.innerHTML = `
    <div class="list-item-content">
      <span class="list-item-title">${escapeHtml(item.title)}</span>
      ${item.subtitle ? `<span class="list-item-subtitle">${escapeHtml(item.subtitle)}</span>` : ''}
    </div>
    ${item.badge ? `<span class="list-item-badge">${escapeHtml(item.badge)}</span>` : ''}
  `;

  return el;
}

function escapeHtml(str: string): string {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}
