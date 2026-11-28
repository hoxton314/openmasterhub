<!-- Switch card as shown on the dial: PROFILE / SETUP caption, name, sub-line, ring in the setup colour. -->
<script lang="ts">
  let { caption = 'PROFILE', name, sub, ring, size = 150 }: { caption?: string; name: string; sub?: string; ring?: string; size?: number } = $props();
</script>

<div class="card" role="img" aria-label="Dial screen: {name}" style="--s:{size}px;--ring:{ring ?? 'var(--hub-caption)'}">
  <span class="caption">{caption}</span>
  <span class="name">{name}</span>
  {#if sub}<span class="sub">{sub}</span>{/if}
</div>

<style>
  .card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    flex: none;
    width: var(--s);
    height: var(--s);
    margin: 16px;
    border-radius: 50%;
    font-family: var(--type-body-family);
    text-align: center;
    background: var(--hub-dial-face);
    box-shadow: var(--hub-dial-bezel), inset 0 0 0 4px var(--ring);
  }
  .caption {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.18em;
    color: var(--hub-dial-caption);
  }
  .name {
    max-width: 85%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: calc(var(--s) * 0.2);
    font-weight: 800;
    line-height: 1.1;
    color: var(--hub-key-text);
  }
  .sub {
    font-size: 13px;
    color: var(--hub-dial-caption);
  }
</style>
