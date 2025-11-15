import type { NextConfig } from 'next';

const nextConfig: NextConfig = {
  turbopack: {
    // Set workspace root to evna-blocks directory to avoid lockfile confusion
    // (evna/ parent directory has its own package-lock.json for the MCP server)
    root: __dirname,
  },
};

export default nextConfig;
