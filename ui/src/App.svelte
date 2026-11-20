<script lang="ts">
  import { t, setLang, getLang } from './lib/i18n.svelte';
  import Home from './screens/Home.svelte';
  import Analysis from './screens/Analysis.svelte';

  let screen = $state<'home' | 'analysis'>('home');
  let input = $state('');

  function analyze(path: string) {
    input = path;
    screen = 'analysis';
  }
</script>

<div class="shell">
  <nav aria-label="Main">
    <strong class="brand">{t('app.name')}</strong>
    <button class:active={screen === 'home'} onclick={() => (screen = 'home')}>{t('nav.home')}</button>
    <button class:active={screen === 'analysis'} disabled={!input} onclick={() => (screen = 'analysis')}>
      {t('nav.analysis')}
    </button>
    <span class="spacer"></span>
    <select aria-label="Language" value={getLang()} onchange={(e) => setLang(e.currentTarget.value as 'vi' | 'en')}>
      <option value="en">English</option>
      <option value="vi">Tiếng Việt</option>
    </select>
  </nav>
  <main>
    {#if screen === 'home'}
      <Home onAnalyze={analyze} />
    {:else}
      <Analysis {input} />
    {/if}
  </main>
</div>
