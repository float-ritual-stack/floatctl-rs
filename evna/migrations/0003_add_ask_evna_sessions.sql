-- Migration: Add ask_evna session management
-- Simple table to store conversation history for multi-turn sessions

CREATE TABLE IF NOT EXISTS ask_evna_sessions (
  session_id TEXT PRIMARY KEY,
  messages JSONB NOT NULL,
  created_at TIMESTAMP DEFAULT NOW(),
  last_used TIMESTAMP DEFAULT NOW()
);
