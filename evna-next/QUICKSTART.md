# EVNA-Next Quick Start Guide

Get up and running with EVNA-Next in 5 minutes!

## Prerequisites

✅ Node.js 18+ installed
✅ PostgreSQL database with pgvector (or Supabase account)
✅ API keys:
- Anthropic API key (Claude)
- OpenAI API key (embeddings)
- Cohere API key (optional, for reranking)

## Step 1: Install Dependencies

```bash
cd evna-next
npm install
```

## Step 2: Configure Environment

Create `.env` file:

```bash
cp .env.example .env
```

Edit `.env` with your credentials:

```env
# Required
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
DATABASE_URL=postgresql://user:pass@host:port/db
SUPABASE_URL=https://xxx.supabase.co
SUPABASE_SERVICE_KEY=eyJ...

# Optional
COHERE_API_KEY=...
AI_GATEWAY_URL=https://gateway.ai.cloudflare.com/v1/account/gateway
AI_GATEWAY_API_KEY=...
```

## Step 3: Verify Database

Make sure your PostgreSQL database has the required tables:
- `conversations`
- `messages`
- `embeddings`
- `active_context_stream`

See `../evna/migrations/` for schema details.

## Step 4: Run Development Server

```bash
npm run dev
```

Open http://localhost:3000 in your browser.

## Step 5: Try It Out!

Example queries to try:

### Brain Boot
```
Good morning! What was I working on yesterday?
```

### Semantic Search
```
Find conversations about authentication bugs
```

### Active Context
```
Show me recent activity on the pharmacy project
```

## Troubleshooting

### Build Errors

**Font Loading Issues**
- Fonts removed from layout to avoid network issues during build
- Add fonts back via next/font/local if needed

**Module Not Found: 'ai/react'**
- Using AI SDK v5 (v6 beta doesn't export React hooks yet)
- Custom useChat hook implemented in chat-interface.tsx

**Edge Runtime Errors**
- API route uses Node.js runtime (required for pg module)
- Don't change to edge runtime

### Database Connection Issues

**"Pool is not defined"**
- Make sure DATABASE_URL is set in .env
- Check PostgreSQL connection string format

**"supabaseUrl is required"**
- Ensure SUPABASE_URL and SUPABASE_SERVICE_KEY are set
- Don't include quotes around values in .env

### API Errors

**"Missing credentials" for OpenAI**
- Set OPENAI_API_KEY in .env
- Required for embeddings generation

**"Model not found" for Anthropic**
- Check ANTHROPIC_API_KEY is valid
- Default model: claude-3-5-sonnet-20241022

## Production Deployment

### Vercel (Recommended)

1. Push to GitHub
2. Import project in Vercel dashboard
3. Add environment variables
4. Deploy!

Environment variables to set in Vercel:
- All variables from `.env`
- NEXT_PUBLIC_APP_URL (your Vercel URL)

### Other Platforms

Works on:
- Netlify
- Railway
- Render
- Fly.io
- Any Node.js 18+ host

## Next Steps

- Read [README.md](./README.md) for full documentation
- Check [Architecture](./README.md#architecture) section
- Review [Comparison with Original EVNA](./README.md#comparison-with-original-evna)
- Set up [AI Gateway](./README.md#ai-gateway-integration) for caching/analytics

## Support

Issues? Check:
1. Environment variables are set correctly
2. Database is accessible
3. API keys are valid
4. Node.js version is 18+

Still stuck? Open an issue in the repository.

---

**Built with**: Next.js 16, Vercel AI SDK, shadcn/ui, PostgreSQL/pgvector
