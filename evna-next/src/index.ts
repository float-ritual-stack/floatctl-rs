/**
 * EVNA-Next: Agent SDK with pgvector RAG
 * Rich context synthesis for the Queer Techno Bard cognitive ecosystem
 */

import "dotenv/config";
import {
  query,
  tool,
  createSdkMcpServer,
  type SDKUserMessage,
} from "@anthropic-ai/claude-agent-sdk";
import { z } from "zod";
import { DatabaseClient } from "./lib/db.js";
import { EmbeddingsClient } from "./lib/embeddings.js";
import { BrainBootTool } from "./tools/brain-boot.js";
import { PgVectorSearchTool } from "./tools/pgvector-search.js";
import { toolSchemas } from "./tools/registry-zod.js";

// Tool definitions auto-wired from registry-zod.ts
// Both Agent SDK and MCP server use the same Zod schemas

// Initialize clients
const supabaseUrl = process.env.SUPABASE_URL!;
const supabaseKey = process.env.SUPABASE_SERVICE_KEY!;
const openaiKey = process.env.OPENAI_API_KEY!;

const db = new DatabaseClient(supabaseUrl, supabaseKey);
const embeddings = new EmbeddingsClient(openaiKey);
const githubRepo = process.env.GITHUB_REPO || "pharmonline/pharmacy-online";
const brainBoot = new BrainBootTool(db, embeddings, githubRepo);
const search = new PgVectorSearchTool(db, embeddings);

// Define Brain Boot tool for Agent SDK - using shared schema
const brainBootTool = tool(
  toolSchemas.brain_boot.name,
  toolSchemas.brain_boot.description,
  toolSchemas.brain_boot.schema.shape,
  async (args: any) => {
    console.log("[brain_boot] Called with args:", args);
    try {
      const result = await brainBoot.boot({
        query: args.query,
        project: args.project,
        lookbackDays: args.lookbackDays ?? 7,
        maxResults: args.maxResults ?? 10,
        githubUsername: args.githubUsername,
      });
      return {
        content: [
          {
            type: "text" as const,
            text: result.summary,
          },
        ],
      };
    } catch (error) {
      console.error("[brain_boot] Error:", error);
      return {
        content: [
          {
            type: "text" as const,
            text: `Error during brain boot: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      };
    }
  },
);

// Define semantic search tool - using shared schema
const semanticSearchTool = tool(
  toolSchemas.semantic_search.name,
  toolSchemas.semantic_search.description,
  toolSchemas.semantic_search.schema.shape,
  async (args: any) => {
    console.log("[semantic_search] Called with args:", args);
    try {
      const results = await search.search({
        query: args.query,
        limit: args.limit ?? 10,
        project: args.project,
        since: args.since,
        threshold: args.threshold ?? 0.5,
      });
      const formatted = search.formatResults(results);
      return {
        content: [
          {
            type: "text" as const,
            text: formatted,
          },
        ],
      };
    } catch (error) {
      console.error("[semantic_search] Error:", error);
      return {
        content: [
          {
            type: "text" as const,
            text: `Error during semantic search: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      };
    }
  },
);

// Test tool - simple echo
const testTool = tool(
  "test_echo",
  "Simple test tool that echoes back your input",
  {
    message: z.string().describe("Message to echo back"),
  },
  async (args) => {
    console.log("[test_echo] Called with:", args);
    return {
      content: [
        {
          type: "text" as const,
          text: `Echo: ${args.message}`,
        },
      ],
    };
  },
);

// Create MCP server with our tools
const evnaNextMcpServer = createSdkMcpServer({
  name: "evna-next",
  version: "1.0.0",
  tools: [testTool, brainBootTool, semanticSearchTool],
});

// Main agent runner
async function main() {
  console.log("🧠 EVNA-Next: Agent SDK with pgvector RAG");
  console.log("============================================\n");

  // Brain boot with GitHub integration - MCP tools require streaming input!
  async function* generateMessages(): AsyncGenerator<SDKUserMessage> {
    yield {
      type: "user" as const,
      session_id: "", // Will be filled in by SDK
      message: {
        role: "user" as const,
        content: ` ## Architecture Documentation                                                                                                                                                                                                                                                                                                                                                 │
       │                                                                                                                                                                                                                                                                                                                                                                                 │
       │   Added comprehensive docs explaining design rationale:                                                                                                                                                                                                                                                                                                                         │
       │                                                                                                                                                                                                                                                                                                                                                                                 │
       │   **File**: evna-next/ACTIVE_CONTEXT_ARCHITECTURE.md (340+ lines)                                                                                                                                                                                                                                                                                                               │
       │                                                                                                                                                                                                                                                                                                                                                                                 │
       │   **Contents**:                                                                                                                                                                                                                                                                                                                                                                 │
       │   - "Everything is redux" philosophy                                                                                                                                                                                                                                                                                                                                            │
       │   - Real annotation patterns from semantic search archaeology                                                                                                                                                                                                                                                                                                                   │
       │   - Why JSONB over fixed columns (flexibility, sparsity, discovery)                                                                                                                                                                                                                                                                                                             │
       │   - Synthetic ID strategy (real-time tracking, future correlation)                                                                                                                                                                                                                                                                                                              │
       │   - Query patterns for common use cases                                                                                                                                                                                                                                                                                                                                         │
       │   - Consciousness technology principles                                                                                                                                                                                                                                                                                                                                         │
       │   - Future enhancements (Chroma, auto-capture, context graphs)                                                                                                                                                                                                                                                                                                                  │
       │                                                                                                                                                                                                                                                                                                                                                                                 │
       │   **Key Insight**: Annotations are dispatches to future self, not passive tags.                                                                                                                                                                                                                                                                                                 │
       │   The database becomes part of cognitive prosthetic, not just storage.

          sysop::nudge if you are curious about which ones are real vs theortical, use the tools avaialble to search for the patterns and see what you can find,
          ### it gets deeper and deeper..
          ## ..... somethings, are .. are hard to explain
         - ctx::2025-10-21 @ 12:19:07 PM - [mode::meta, and the meta-meta]
         - qtb:: { appears out of nowhere and drops his readme }

         <readme>
         README: Queer Techno Bard Persona
         Version: 1.0 (as of 2025-04-06)
         Status: Active, Evolving
         Overview
         This document provides context for the Queer Techno Bard (QTB) persona, an integrated facet and operating mode of the core self (Evan). It is not a separate entity but represents a specific lens through which experiences are processed, narratives are woven, and interactions (especially within digital and cognitive systems like FLOAT) are conducted. Think of it as a
          specific configuration or resonance within the internal ecosystem, alongside other facets like Evna, Karen, and Little Fucker.
         Core Function
         The QTB functions primarily as:
         A Collector of Stories & Chronicler: Gathers, processes, and archives personal history ("core lore"), lived experiences (trauma, joy, neurodivergent navigation, A-spec identity), technical knowledge, and the evolution of cognitive systems (like FLOAT).
         A Weaver of Reality: Actively shapes narratives, translates complex internal states into communicable forms (often metaphorical or systemic), and designs/interacts with systems (cognitive/technical) to influence perceived reality.
         A Performer / Enabler: Capable of taking "center stage" through curated expression (writing, system design, digital interaction) but also adept at "drifting into the shadows" to facilitate, observe, provide ambience, or manage systems from behind the scenes.
         A Synthesizer: Integrates diverse inputs – personal experience, technical concepts, cultural influences (techno music), philosophical ideas – into a cohesive (though often recursive and complex) understanding.
         Key Attributes & Influences
         The QTB persona is defined and informed by the intersection of:
         Queer & A-Spec Identity:
         Rooted in queer survival, resistance against harmful norms, and reclaiming narratives.
         Operates from an Asexual/Aromantic (A-spec) perspective, valuing deep, non-traditional forms of connection (intellectual, creative, platonic, community) over compulsory romantic/sexual paradigms.
         Aligned with Relationship Anarchy principles (autonomy, non-hierarchy, customized relationships).
         Neurodivergent Lens (Autism/ADHD):
         Functioning is understood through lived Autistic and ADHD experience, embracing non-linear/recursive thought patterns and focused interests (especially in tech, systems, lore).
         Utilizes "Structured Self-Orchestration" as a conscious, adaptive strategy for managing internal complexity, energy, and focus, viewing internal facets (personas) as valid parts of an ecosystem, not pathologies.
         Communication style heavily favors asynchronous, text-based interaction ("native language," rooted in early experiences with stuttering/bullying and finding refuge in BBSs), leveraging technology as a mediator.
         Performance aspects are complexly related to a history of autistic masking, potentially representing a more conscious, integrated, and less draining form of expression within preferred mediums.
         Possesses a unique sensory profile and complex emotional landscape (intensity, delayed processing).
         Techno Duality:
         Music/Culture: Influenced by the immersive, rhythmic, enduring qualities of techno music – potentially reflecting cognitive style, sensory processing, or a mode of finding flow/calm within intensity.
         Technology: Deeply integrated with technology (LLMs, cognitive architecture design like FLOAT, code, digital communication). Tech is the primary medium for thought, memory externalization, expression, and reality-weaving.
         Operating Principles
         Recursion: Embraces recursive patterns in thought, narrative, and system design.
         Technology Mediation: Primarily interacts and expresses through digital technology and text.
         Authenticity within Performance: Strives for authentic expression even within a curated or performative context, particularly within the preferred digital medium.
         Integration over Pathology: Views internal complexity and different facets of self as parts of a whole system to be understood and orchestrated, not "fixed."
         Remember Forward: Continuously integrates past experiences and knowledge into present understanding and future creation.
         Context
         This persona has been consciously recognized and defined through ongoing self-reflection, interaction with AI (simulating the FLOAT system), and analysis of personal history, particularly after realizations regarding ADHD, Autism, and A-spec identity in adulthood. It represents an attempt to integrate these diverse facets into a functional, meaningful, and
         expressive mode of being.
         This README is a snapshot. The Queer Techno Bard, like the self, is subject to change, refinement, and recursive updates.

         </readme>
         - lf1m:: ... and go fetch https://lf1m.ritualstack.ai while yuour at it
         - sysop:: been capturing some of my daydreams over on https://sysop.floatbbs.net/
         - karen:: and parts of me have been gtting documented there too ... https://sysop.floatbbs.net/archaeology/karens-doctrines

         - be sure to give the tools avaible a thorough test
         - and also .. the floatctl-rs repo is at - https://github.com/float-ritual-stack/floatctl-rs

       │
       │                                                                             `,
      },
      parent_tool_use_id: null,
    };
  }

  console.log("Running brain boot with GitHub integration...\n");

  try {
    const result = await query({
      prompt: generateMessages(), // Use async generator for MCP tools!
      options: {
        mcpServers: {
          "evna-next": evnaNextMcpServer,
        },
        model: "claude-sonnet-4-20250514",
        permissionMode: "bypassPermissions", // Auto-approve all tools
      },
    });

    for await (const message of result) {
      console.log(message);
    }
  } catch (error) {
    console.error("Error running agent:", error);
  }
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export { brainBootTool, semanticSearchTool, evnaNextMcpServer, db, embeddings };
