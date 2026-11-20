<script lang="ts">
  let { diff }: { diff: string } = $props();
  let lines = $derived(diff.split('\n').filter((line, i, all) => !(line === '' && i === all.length - 1)));
</script>

<div class="diff" role="region" aria-label="File diff" tabindex="0">
  {#each lines as line, i (i)}
    <div class:added={line.startsWith('+') && !line.startsWith('+++')}
      class:removed={line.startsWith('-') && !line.startsWith('---')}
      class:hunk={line.startsWith('@@')}
      class:meta={line.startsWith('---') || line.startsWith('+++')}>
      <span class="number" aria-hidden="true">{i + 1}</span>
      <span class="marker" aria-hidden="true">{line.startsWith('+') && !line.startsWith('+++') ? '+' : line.startsWith('-') && !line.startsWith('---') ? '−' : ' '}</span>
      <code>{line}</code>
    </div>
  {/each}
  {#if lines.length === 0}<p class="muted">No textual changes.</p>{/if}
</div>

<style>
  .diff { overflow: auto; max-height: 56vh; border: 1px solid var(--line); border-radius: 5px; background: var(--panel); font: 12px/1.55 ui-monospace, SFMono-Regular, Consolas, monospace; }
  .diff > div { display: grid; grid-template-columns: 3.2em 1.5em minmax(max-content, 1fr); white-space: pre; padding-right: 10px; }
  .number { text-align: right; padding-right: 8px; color: var(--muted); user-select: none; border-right: 1px solid var(--line); }
  .marker { text-align: center; font-weight: 700; }
  code { padding-left: 8px; font: inherit; }
  .added { background: color-mix(in srgb, var(--low-bg) 75%, transparent); }
  .removed { background: color-mix(in srgb, var(--high-bg) 75%, transparent); }
  .added .marker { color: var(--low); } .removed .marker { color: var(--high); }
  .hunk { color: var(--accent); background: color-mix(in srgb, var(--accent) 9%, var(--panel)); }
  .meta { color: var(--muted); }
</style>