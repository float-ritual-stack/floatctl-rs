import { Board, BoardPost, CreateBoardRequest, UpdateBoardRequest } from '@/lib/types';
import { generateId } from '@/lib/utils';

/**
 * In-memory board store
 * In production, this would be backed by a database or API
 */
class BoardStore {
  private boards: Map<string, Board> = new Map();

  /**
   * Create a new board
   */
  createBoard(request: CreateBoardRequest): Board {
    const now = new Date().toISOString();
    const boardId = generateId('board');

    const board: Board = {
      id: boardId,
      title: request.title,
      description: request.description,
      tags: request.tags || [],
      posts: request.initialPost
        ? [
            {
              id: generateId('post'),
              content: request.initialPost,
              timestamp: now,
            },
          ]
        : [],
      createdAt: now,
      lastUpdatedAt: now,
    };

    this.boards.set(boardId, board);
    return board;
  }

  /**
   * Get a board by ID
   */
  getBoard(boardId: string): Board | undefined {
    return this.boards.get(boardId);
  }

  /**
   * Get all boards
   */
  getAllBoards(): Board[] {
    return Array.from(this.boards.values()).sort(
      (a, b) => new Date(b.lastUpdatedAt).getTime() - new Date(a.lastUpdatedAt).getTime()
    );
  }

  /**
   * Update a board
   */
  updateBoard(boardId: string, request: UpdateBoardRequest): Board | undefined {
    const board = this.boards.get(boardId);
    if (!board) return undefined;

    const updated: Board = {
      ...board,
      title: request.title ?? board.title,
      description: request.description ?? board.description,
      tags: request.tags ?? board.tags,
      posts: request.addPost ? [...board.posts, request.addPost] : board.posts,
      lastUpdatedAt: new Date().toISOString(),
    };

    this.boards.set(boardId, updated);
    return updated;
  }

  /**
   * Add a post to a board
   */
  addPost(boardId: string, content: string, author?: string): BoardPost | undefined {
    const board = this.boards.get(boardId);
    if (!board) return undefined;

    const post: BoardPost = {
      id: generateId('post'),
      content,
      author,
      timestamp: new Date().toISOString(),
    };

    board.posts.push(post);
    board.lastUpdatedAt = post.timestamp;
    this.boards.set(boardId, board);

    return post;
  }

  /**
   * Search boards by title or tags
   */
  searchBoards(query: string): Board[] {
    const lowerQuery = query.toLowerCase();
    return Array.from(this.boards.values()).filter(
      (board) =>
        board.title.toLowerCase().includes(lowerQuery) ||
        board.tags.some((tag) => tag.toLowerCase().includes(lowerQuery)) ||
        board.description?.toLowerCase().includes(lowerQuery)
    );
  }

  /**
   * Clear all boards (for testing)
   */
  clear() {
    this.boards.clear();
  }
}

/**
 * Global board store instance
 */
export const boardStore = new BoardStore();

/**
 * Initialize with some sample boards for demonstration
 */
export function initializeSampleBoards() {
  if (boardStore.getAllBoards().length === 0) {
    // Sample board: Project ideas
    const projectBoard = boardStore.createBoard({
      title: 'Project Ideas',
      description: 'Brainstorming for future projects',
      tags: ['projects', 'ideas', 'brainstorming'],
      initialPost: 'Let\'s collect interesting project ideas here.',
    });

    boardStore.addPost(
      projectBoard.id,
      'Idea: Build a distributed semantic search system using pgvector',
      'evna'
    );

    boardStore.addPost(
      projectBoard.id,
      'Idea: Create a block-based note-taking system with AI assistance',
      'evna'
    );

    // Sample board: Meeting notes
    const meetingBoard = boardStore.createBoard({
      title: 'Daily Sync Notes',
      description: 'Notes from daily sync meetings',
      tags: ['meetings', 'sync', 'daily'],
      initialPost: 'Meeting notes will be collected here.',
    });

    boardStore.addPost(
      meetingBoard.id,
      'Nov 15: Discussed architecture for block chat UI. Agreed on three-panel layout.',
      'team'
    );
  }
}
