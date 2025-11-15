import { streamText } from 'ai';
import { getModel, BLOCK_CHAT_SYSTEM_PROMPT } from '@/lib/ai/config';

export const runtime = 'edge';

export async function POST(req: Request) {
  const { messages } = await req.json();

  const result = streamText({
    model: getModel(),
    system: BLOCK_CHAT_SYSTEM_PROMPT,
    messages,
    temperature: 0.7,
    maxOutputTokens: 4096,
  });

  // Return a simple text stream response
  const encoder = new TextEncoder();
  const stream = new ReadableStream({
    async start(controller) {
      for await (const chunk of result.textStream) {
        controller.enqueue(encoder.encode(chunk));
      }
      controller.close();
    },
  });

  return new Response(stream, {
    headers: {
      'Content-Type': 'text/plain; charset=utf-8',
    },
  });
}
