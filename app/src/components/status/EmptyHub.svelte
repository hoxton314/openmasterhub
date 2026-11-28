<!-- The hub with nothing connected (S06): the Base with a 6 x 4 grid of empty sockets, drawn from the --hub-* tokens. -->
<script lang="ts"></script>

<div class="hub" aria-hidden="true">
  {#each Array.from({ length: 24 }) as _}<span></span>{/each}
</div>

<style>
  .hub {
    display: grid;
    grid-template-columns: repeat(6, 36px);
    grid-auto-rows: 36px;
    gap: 10px;
    padding: 12px;
    background: var(--hub-chassis);
    border-radius: var(--hub-base-radius);
    box-shadow: var(--hub-base-shadow);
  }
  span {
    background: var(--hub-empty);
    border-radius: var(--hub-key-radius);
    box-shadow: var(--hub-key-frame);
  }
</style>
