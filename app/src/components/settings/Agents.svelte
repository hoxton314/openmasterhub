<!-- Settings › AI agents: the opt-in MCP server (`masterhubd mcp`, docs/MCP.md) and how to connect an agent. Saved to [app] mcp. -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { Check, Copy } from 'lucide-svelte';
  import { hub } from '../../lib/hub.svelte';
  import Banner from '../ui/Banner.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import Toggle from '../ui/Toggle.svelte';
  import { run, saveApp } from './actions';
  import { claudeCodeCommand, desktopConfig } from './mcp';
  import Page from './Page.svelte';
  import Row from './Row.svelte';

  const on = $derived(hub.config.app?.mcp === true);
  let program = $state('masterhubd');
  let copied = $state('');
  onMount(async () => {
    program = (await run<string>('mcp_program')) ?? program;
  });

  const snippets = $derived([
    { id: 'code', title: 'Claude Code', help: 'Run once in a terminal; the server is then there in every project.', text: claudeCodeCommand(program) },
    {
      id: 'desktop',
      title: 'Claude Desktop and other MCP clients',
      help: 'Add to claude_desktop_config.json (Settings › Developer › Edit Config), or to the mcpServers list of your client, then restart it.',
      text: desktopConfig(program),
    },
  ]);

  async function copy(id: string, text: string) {
    await navigator.clipboard?.writeText(text);
    copied = id;
    setTimeout(() => (copied = copied === id ? '' : copied), 1500);
  }
</script>

<Page title="AI agents" about="Let AI agents such as Claude read your hub and edit profiles, keys, dial actions and setups through the Model Context Protocol (MCP).">
  <section>
    <Row label="Run MCP server" help="Off by default. Agents start masterhubd mcp themselves; it only works while this is on and the service runs">
      <Toggle title="Run MCP server" checked={on} onchange={(v) => saveApp({ mcp: v }, v ? 'MCP server on' : 'MCP server off')} />
    </Row>
  </section>
  <section>
    <SectionLabel as="h3">Connect an agent</SectionLabel>
    {#if !on}<p class="t-small muted">Turn the server on first; until then agents are told it is off.</p>{/if}
    {#each snippets as s (s.id)}
      <div class="snippet">
        <div class="head">
          <div class="text">
            <span class="t-body">{s.title}</span>
            <span class="t-small muted">{s.help}</span>
          </div>
          <IconButton icon={copied === s.id ? Check : Copy} label={copied === s.id ? 'Copied' : `Copy the ${s.title} snippet`} onclick={() => copy(s.id, s.text)} />
        </div>
        <pre class="t-mono">{s.text}</pre>
      </div>
    {/each}
  </section>
  <Banner kind="info">
    Agents can change your whole config, including keys that run shell commands when you press them. Every change is checked the same way as in the app before it is saved, the app's own settings stay out of reach, and agents cannot switch the modules to hardware mode.
  </Banner>
</Page>

<style>
  section {
    display: grid;
    gap: var(--space-3);
  }
  p {
    margin: 0;
  }
  .snippet {
    display: grid;
    gap: var(--space-2);
  }
  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-4);
  }
  .text {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  pre {
    margin: 0;
    padding: var(--space-3);
    overflow-x: auto;
    color: var(--color-text-2);
    background: var(--color-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-well);
    white-space: pre;
    user-select: text;
  }
</style>
