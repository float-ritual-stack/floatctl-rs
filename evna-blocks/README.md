# EVNA Blocks

> Block-based AI workspace with continuous note-taking and BBS integration

EVNA Blocks reimagines AI interaction as a **living document** where agent responses are embedded as rich, interactive blocks within your continuous workspace. Inspired by v0, Notion, and Linear's command interfaces, but designed for fluid AI collaboration.

## 🎯 Core Concept

Instead of traditional linear chat, EVNA Blocks is **compositional** - agent responses become part of your continuous note.

## 🚀 Tech Stack

- **Next.js 16** - Latest App Router with React Server Components
- **Vercel AI SDK 6 (beta)** - Agent abstraction and structured outputs
- **TipTap** - Rich text editor with custom React node views
- **shadcn/ui** - Beautiful, accessible component library
- **TypeScript** - Full type safety
- **Tailwind CSS** - Utility-first styling

## 🛠️ Development

### Installation

```bash
cd evna-blocks
npm install
```

### Running Dev Server

```bash
npm run dev
```

Open [http://localhost:3000](http://localhost:3000)

### Building for Production

```bash
npm run build
npm start
```

## 🎨 Key Features

### 1. Block-Based Chat

Agent responses are **first-class blocks** in your document, not ephemeral messages.

### 2. Slash Commands

Type `/` anywhere to trigger AI agents:

- `/brain_boot` - Morning synthesis with multi-source context
- `/search` - Semantic search across conversation history
- `/context` - Query recent active context
- `/ask` - Ask evna orchestrator
- `/board` - Insert BBS board embed

### 3. Three-Pane Layout

- **Left Sidebar**: Navigation, recent sessions (collapsible)
- **Center**: TipTap editor (continuous note)
- **Right**: BBS board preview (collapsible)

## 📖 Resources

- **Architecture Doc**: `../evna-blocks-architecture.md` (comprehensive design)
- **TipTap Docs**: https://tiptap.dev/docs
- **AI SDK 6 Docs**: https://v6.ai-sdk.dev/docs
- **Next.js 16 Docs**: https://nextjs.org/docs

---

Built with 🧠 by the EVNA team
