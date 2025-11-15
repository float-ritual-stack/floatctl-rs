# EVNA-Next Deployment Guide

Complete guide for deploying EVNA-Next to production.

## Prerequisites

Before deploying, ensure you have:
- [ ] PostgreSQL database with pgvector extension
- [ ] Anthropic API key
- [ ] OpenAI API key  
- [ ] (Optional) Cohere API key for reranking
- [ ] (Optional) Cloudflare AI Gateway configured

## Deployment Platforms

### Vercel (Recommended)

**Why Vercel?**
- Native Next.js support
- Automatic deployments from Git
- Built-in analytics
- Easy environment variable management
- Free tier available

**Steps:**

1. **Push to GitHub**
   ```bash
   git push origin main
   ```

2. **Import to Vercel**
   - Go to [vercel.com/new](https://vercel.com/new)
   - Select your repository
   - Click "Import"

3. **Configure Environment Variables**
   
   In Vercel dashboard → Settings → Environment Variables, add:
   
   ```
   ANTHROPIC_API_KEY=sk-ant-...
   OPENAI_API_KEY=sk-...
   DATABASE_URL=postgresql://...
   SUPABASE_URL=https://...
   SUPABASE_SERVICE_KEY=eyJ...
   COHERE_API_KEY=...              # Optional
   AI_GATEWAY_URL=https://...      # Optional
   AI_GATEWAY_API_KEY=...          # Optional
   NEXT_PUBLIC_APP_URL=https://your-app.vercel.app
   ```

4. **Deploy**
   - Click "Deploy"
   - Wait for build to complete (~2-3 minutes)
   - Visit your new URL!

**Post-Deployment:**
- Set up custom domain (optional)
- Enable Vercel Analytics
- Configure monitoring

### Netlify

**Steps:**

1. **Create netlify.toml**
   ```toml
   [build]
     command = "npm run build"
     publish = ".next"

   [[redirects]]
     from = "/api/*"
     to = "/.netlify/functions/:splat"
     status = 200
   ```

2. **Deploy via Netlify CLI**
   ```bash
   npm install -g netlify-cli
   netlify login
   netlify init
   netlify deploy --prod
   ```

3. **Set Environment Variables**
   ```bash
   netlify env:set ANTHROPIC_API_KEY "sk-ant-..."
   netlify env:set OPENAI_API_KEY "sk-..."
   # etc.
   ```

### Railway

**Steps:**

1. **Create railway.json**
   ```json
   {
     "$schema": "https://railway.app/railway.schema.json",
     "build": {
       "builder": "NIXPACKS"
     },
     "deploy": {
       "startCommand": "npm start",
       "restartPolicyType": "ON_FAILURE",
       "restartPolicyMaxRetries": 10
     }
   }
   ```

2. **Deploy**
   ```bash
   npm install -g @railway/cli
   railway login
   railway init
   railway up
   ```

3. **Set Variables**
   - Go to Railway dashboard
   - Select your service
   - Add environment variables

### Docker Deployment

**Create Dockerfile:**

```dockerfile
FROM node:18-alpine AS base

# Install dependencies only when needed
FROM base AS deps
WORKDIR /app
COPY package*.json ./
RUN npm ci

# Rebuild the source code only when needed
FROM base AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .
RUN npm run build

# Production image, copy all the files and run next
FROM base AS runner
WORKDIR /app

ENV NODE_ENV=production

RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 nextjs

COPY --from=builder /app/public ./public
COPY --from=builder --chown=nextjs:nodejs /app/.next ./.next
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/package.json ./package.json

USER nextjs

EXPOSE 3000

ENV PORT=3000
ENV HOSTNAME="0.0.0.0"

CMD ["npm", "start"]
```

**Build and Run:**
```bash
docker build -t evna-next .
docker run -p 3000:3000 \
  -e ANTHROPIC_API_KEY=... \
  -e OPENAI_API_KEY=... \
  -e DATABASE_URL=... \
  evna-next
```

**Docker Compose:**

```yaml
version: '3.8'

services:
  evna-next:
    build: .
    ports:
      - "3000:3000"
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - DATABASE_URL=${DATABASE_URL}
      - SUPABASE_URL=${SUPABASE_URL}
      - SUPABASE_SERVICE_KEY=${SUPABASE_SERVICE_KEY}
    restart: unless-stopped
```

### Self-Hosted (VPS)

**Requirements:**
- Ubuntu 22.04+ or similar
- Node.js 18+
- Nginx (for reverse proxy)
- PM2 (for process management)

**Steps:**

1. **Setup Server**
   ```bash
   # Install Node.js
   curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
   sudo apt-get install -y nodejs

   # Install PM2
   sudo npm install -g pm2

   # Install Nginx
   sudo apt-get install nginx
   ```

2. **Deploy Application**
   ```bash
   # Clone repository
   git clone https://github.com/your-repo/floatctl-rs
   cd floatctl-rs/evna-next

   # Install dependencies
   npm ci

   # Build
   npm run build

   # Create .env file
   cp .env.example .env
   nano .env  # Add your credentials
   ```

3. **Start with PM2**
   ```bash
   pm2 start npm --name "evna-next" -- start
   pm2 save
   pm2 startup
   ```

4. **Configure Nginx**
   ```nginx
   server {
       listen 80;
       server_name your-domain.com;

       location / {
           proxy_pass http://localhost:3000;
           proxy_http_version 1.1;
           proxy_set_header Upgrade $http_upgrade;
           proxy_set_header Connection 'upgrade';
           proxy_set_header Host $host;
           proxy_cache_bypass $http_upgrade;
       }
   }
   ```

5. **Setup SSL (Let's Encrypt)**
   ```bash
   sudo apt-get install certbot python3-certbot-nginx
   sudo certbot --nginx -d your-domain.com
   ```

## Database Setup

### Supabase (Managed)

1. **Create Project**
   - Go to [supabase.com](https://supabase.com)
   - Create new project
   - Wait for provisioning (~2 minutes)

2. **Enable pgvector**
   ```sql
   CREATE EXTENSION IF NOT EXISTS vector;
   ```

3. **Run Migrations**
   - Copy SQL from `../evna/migrations/`
   - Run in Supabase SQL editor

4. **Get Credentials**
   - Project Settings → API
   - Copy:
     - Project URL → `SUPABASE_URL`
     - Service Role Key → `SUPABASE_SERVICE_KEY`
     - Connection String → `DATABASE_URL`

### Self-Hosted PostgreSQL

1. **Install PostgreSQL**
   ```bash
   sudo apt-get install postgresql postgresql-contrib
   ```

2. **Install pgvector**
   ```bash
   git clone https://github.com/pgvector/pgvector
   cd pgvector
   make
   sudo make install
   ```

3. **Create Database**
   ```sql
   CREATE DATABASE evna;
   CREATE EXTENSION vector;
   ```

4. **Run Migrations**
   ```bash
   psql -U postgres -d evna -f ../evna/migrations/*.sql
   ```

## Environment Variables

### Required

```env
ANTHROPIC_API_KEY=sk-ant-...      # Claude API key
OPENAI_API_KEY=sk-...             # OpenAI embeddings
DATABASE_URL=postgresql://...     # PostgreSQL connection
SUPABASE_URL=https://...          # Supabase project URL
SUPABASE_SERVICE_KEY=eyJ...       # Supabase service role
```

### Optional

```env
COHERE_API_KEY=...                # Reranking (graceful fallback if missing)
AI_GATEWAY_URL=...                # Cloudflare AI Gateway
AI_GATEWAY_API_KEY=...            # Gateway authentication
NEXT_PUBLIC_APP_URL=...           # Your app's public URL
```

## Monitoring & Observability

### Vercel Analytics

Enable in Vercel dashboard:
- Web Analytics (page views, demographics)
- Speed Insights (Core Web Vitals)
- Real User Monitoring

### Custom Logging

Add to `app/api/chat/route.ts`:

```typescript
import { headers } from 'next/headers';

export async function POST(req: Request) {
  const startTime = Date.now();
  
  try {
    // ... existing code ...
    
    console.log({
      timestamp: new Date().toISOString(),
      duration: Date.now() - startTime,
      userAgent: headers().get('user-agent'),
      success: true,
    });
  } catch (error) {
    console.error({
      timestamp: new Date().toISOString(),
      duration: Date.now() - startTime,
      error: error.message,
      success: false,
    });
  }
}
```

### Error Tracking

**Sentry Integration:**

```bash
npm install @sentry/nextjs
```

```typescript
// sentry.client.config.ts
import * as Sentry from "@sentry/nextjs";

Sentry.init({
  dsn: process.env.NEXT_PUBLIC_SENTRY_DSN,
  tracesSampleRate: 1.0,
});
```

## Performance Optimization

### Edge Caching

**Enable in AI Gateway:**
- Cache embeddings (24h TTL)
- Cache common queries (1h TTL)
- Rate limit per user

### Database Optimization

```sql
-- Index for semantic search
CREATE INDEX embeddings_vector_idx ON embeddings 
USING ivfflat (embedding vector_cosine_ops);

-- Index for active context
CREATE INDEX active_context_timestamp_idx 
ON active_context_stream (timestamp DESC);
```

### Next.js Optimizations

```typescript
// next.config.ts
export default {
  experimental: {
    optimizePackageImports: ['@ai-sdk/anthropic', '@ai-sdk/openai'],
  },
  compress: true,
  poweredByHeader: false,
};
```

## Security Checklist

- [ ] Environment variables stored securely
- [ ] Database uses SSL connection
- [ ] API keys rotated regularly
- [ ] Rate limiting enabled
- [ ] CORS configured properly
- [ ] Helmet.js for security headers
- [ ] Input validation on all endpoints
- [ ] SQL injection prevention (parameterized queries)
- [ ] XSS protection (React default)
- [ ] CSRF tokens for mutations

## Troubleshooting

### Build Failures

**"Module not found"**
- Run `npm ci` to clean install
- Delete `node_modules` and `.next`
- Rebuild

**"Out of memory"**
- Increase Node.js heap: `NODE_OPTIONS=--max-old-space-size=4096`

### Runtime Errors

**"Connection refused" (Database)**
- Check DATABASE_URL format
- Verify PostgreSQL is running
- Check firewall rules

**"API key invalid"**
- Verify keys in environment
- Check key permissions
- Try regenerating keys

### Performance Issues

**Slow responses**
- Enable AI Gateway caching
- Add database indexes
- Use connection pooling
- Monitor Vercel logs

## Cost Estimation

### Vercel (Hobby Tier)
- Hosting: Free
- Bandwidth: 100GB/month free
- Build minutes: 6000/month free

### API Costs (Approximate)
- Claude 3.5 Sonnet: $3 per 1M input tokens
- OpenAI Embeddings: $0.02 per 1M tokens
- Cohere Rerank: $1 per 1M tokens

**Example Monthly Cost:**
- 10k queries
- Avg 500 tokens per query
- ~$15-25/month

### Database (Supabase)
- Free tier: 500MB database, 2GB bandwidth
- Pro tier: $25/month (8GB database, 50GB bandwidth)

## Backup Strategy

1. **Database Backups**
   ```bash
   pg_dump -h host -U user -d evna > backup-$(date +%Y%m%d).sql
   ```

2. **Scheduled Backups**
   ```bash
   # Crontab
   0 2 * * * /path/to/backup.sh
   ```

3. **Off-site Storage**
   - Upload to S3/R2
   - Retain 30 days
   - Test restore monthly

## Scaling Considerations

### Horizontal Scaling
- Deploy multiple instances behind load balancer
- Use Redis for session storage
- Database read replicas

### Vertical Scaling
- Upgrade Vercel tier (more CPU/memory)
- Larger database instance
- More aggressive caching

---

**Deployment Checklist:**
- [ ] Environment variables set
- [ ] Database migrated
- [ ] API keys tested
- [ ] Build successful
- [ ] Monitoring enabled
- [ ] Backups configured
- [ ] SSL certificate active
- [ ] Custom domain configured (optional)

**Support:**
- Check logs: `vercel logs` or PM2 logs
- Database status: Supabase dashboard
- API status: Status pages

---

**Last Updated**: 2025-11-15  
**Maintained by**: Evan (QTB)
