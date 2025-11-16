/**
 * Read from stdin if data is available (piped or redirected)
 *
 * Supports both patterns:
 * - Pipe: echo "query" | evna ask
 * - Heredoc: evna ask <<EOF ... EOF
 * - File: evna ask < query.txt
 *
 * Returns null if stdin is a TTY (interactive terminal)
 */
export async function readStdinIfAvailable(): Promise<string | null> {
  // If stdin is a TTY (interactive terminal), don't read
  if (process.stdin.isTTY) {
    return null;
  }

  // Stdin has data - read it
  const chunks: Buffer[] = [];

  return new Promise((resolve, reject) => {
    process.stdin.on('data', (chunk) => {
      chunks.push(chunk);
    });

    process.stdin.on('end', () => {
      const content = Buffer.concat(chunks).toString('utf-8').trim();
      resolve(content || null);
    });

    process.stdin.on('error', (error) => {
      reject(error);
    });
  });
}

/**
 * Get query from either args or stdin (with priority to args)
 *
 * Priority:
 * 1. Command-line argument (if provided)
 * 2. Stdin (if piped/redirected)
 * 3. null (error - user must provide input)
 */
export async function getQueryFromArgsOrStdin(args: string[]): Promise<string | null> {
  // Priority 1: Use command-line arg if provided
  if (args[0]) {
    return args[0];
  }

  // Priority 2: Try reading from stdin
  return await readStdinIfAvailable();
}
