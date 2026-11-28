// How an AI agent connects to the MCP server (`masterhubd mcp`, docs/MCP.md): the snippets shown in Settings › AI agents.

/** The server's name in the agent's MCP list. */
export const SERVER_NAME = 'openmasterhub';

/** `program` quoted for a shell when it has spaces (e.g. `C:\Program Files\…`). */
function quoted(program: string): string {
  return /[\s"]/.test(program) ? `"${program.replaceAll('"', '\\"')}"` : program;
}

/** Claude Code: add the server for every project of this user. */
export function claudeCodeCommand(program: string): string {
  return `claude mcp add --scope user ${SERVER_NAME} -- ${quoted(program)} mcp`;
}

/** Claude Desktop (and other clients with an `mcpServers` JSON file). */
export function desktopConfig(program: string): string {
  return JSON.stringify({ mcpServers: { [SERVER_NAME]: { command: program, args: ['mcp'] } } }, null, 2);
}
