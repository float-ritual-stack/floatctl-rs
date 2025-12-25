/**
 * Items store
 *
 * Manages the current list of items displayed in the main view.
 * Handles fetching from sources and caching.
 */

import type { Item, SourceKind, Scope, SourceResponse } from '../lib/protocol';
import { fetchItems as fetchItemsCmd } from '../lib/commands';

type ItemsListener = (items: Item[]) => void;
type LoadingListener = (loading: boolean) => void;

/** Items store singleton */
class ItemsStore {
  private items: Item[] = [];
  private loading = false;
  private error: string | null = null;
  private hasMore = false;
  private total = 0;

  private itemListeners: Set<ItemsListener> = new Set();
  private loadingListeners: Set<LoadingListener> = new Set();

  /** Get current items */
  get current(): Item[] {
    return this.items;
  }

  /** Get loading state */
  get isLoading(): boolean {
    return this.loading;
  }

  /** Get error message */
  get errorMessage(): string | null {
    return this.error;
  }

  /** Get total count */
  get totalCount(): number {
    return this.total;
  }

  /** Subscribe to items changes */
  subscribeItems(listener: ItemsListener): () => void {
    this.itemListeners.add(listener);
    listener(this.items);
    return () => this.itemListeners.delete(listener);
  }

  /** Subscribe to loading state changes */
  subscribeLoading(listener: LoadingListener): () => void {
    this.loadingListeners.add(listener);
    listener(this.loading);
    return () => this.loadingListeners.delete(listener);
  }

  private notifyItems(): void {
    for (const listener of this.itemListeners) {
      listener(this.items);
    }
  }

  private notifyLoading(): void {
    for (const listener of this.loadingListeners) {
      listener(this.loading);
    }
  }

  /** Fetch items from source */
  async fetch(source: SourceKind, scope?: Scope): Promise<void> {
    this.loading = true;
    this.error = null;
    this.notifyLoading();

    try {
      const response = await fetchItemsCmd(source, scope);
      this.items = response.items;
      this.hasMore = response.has_more;
      this.total = response.total ?? response.items.length;
      this.notifyItems();
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
      this.items = [];
    } finally {
      this.loading = false;
      this.notifyLoading();
    }
  }

  /** Clear items */
  clear(): void {
    this.items = [];
    this.error = null;
    this.hasMore = false;
    this.total = 0;
    this.notifyItems();
  }

  /** Get item by index */
  getByIndex(index: number): Item | undefined {
    return this.items[index];
  }

  /** Get item by ID */
  getById(id: string): Item | undefined {
    return this.items.find((item) => item.id === id);
  }
}

export const itemsStore = new ItemsStore();
