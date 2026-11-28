import { describe, expect, it } from 'vitest';
import { claudeCodeCommand, desktopConfig } from './mcp';

describe('MCP connection snippets', () => {
  it('builds the Claude Code command', () => {
    expect(claudeCodeCommand('masterhubd')).toBe('claude mcp add --scope user openmasterhub -- masterhubd mcp');
    expect(claudeCodeCommand('C:\\Program Files\\OpenMasterHUB\\masterhubd.exe')).toBe(
      'claude mcp add --scope user openmasterhub -- "C:\\Program Files\\OpenMasterHUB\\masterhubd.exe" mcp',
    );
  });

  it('builds the mcpServers JSON with the path unquoted', () => {
    const cfg = JSON.parse(desktopConfig('C:\\Program Files\\OpenMasterHUB\\masterhubd.exe'));
    expect(cfg).toEqual({ mcpServers: { openmasterhub: { command: 'C:\\Program Files\\OpenMasterHUB\\masterhubd.exe', args: ['mcp'] } } });
  });
});
