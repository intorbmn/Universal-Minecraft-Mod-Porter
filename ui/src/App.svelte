<script lang="ts">
  import { t, setLang, getLang } from './lib/i18n.svelte';
  import { api } from './lib/api';
  import Home from './screens/Home.svelte';
  import Analysis from './screens/Analysis.svelte';
  import Target from './screens/Target.svelte';
  import Plan from './screens/Plan.svelte';
  import Run from './screens/Run.svelte';
  import Report from './screens/Report.svelte';
  import type { JobView, PlanRequest, UiError } from './lib/types';

  type Screen = 'home' | 'analysis' | 'target' | 'plan' | 'run' | 'report' | 'build';
  let screen = $state<Screen>('home');
  let input = $state('');
  let request = $state<PlanRequest | null>(null);
  let mode = $state<'port' | 'in_place'>('port');
  let output = $state<string | null>(null);
  let jobId = $state('');
  let finishedJob = $state<JobView | null>(null);
  let error = $state<UiError | null>(null);

  function analyze(path: string) {
    input = path;
    screen = 'analysis';
  }

  function chooseTarget() {
    screen = 'target';
  }

  function preview(next: PlanRequest, nextMode: 'port' | 'in_place', nextOutput: string | null) {
    request = next;
    mode = nextMode;
    output = nextOutput;
    screen = 'plan';
  }

  async function start(next: PlanRequest, nextMode: 'port' | 'in_place', nextOutput: string | null, reviewed: boolean) {
    error = null;
    try {
      jobId = nextMode === 'port'
        ? await api.startPort({
            input: next.input,
            output: nextOutput,
            to: next.to,
            from: next.from,
            loader: next.loader,
            zip: false,
            min_confidence: next.min_confidence,
          })
        : await api.startMigrateInPlace(next, reviewed);
      finishedJob = null;
      screen = 'run';
    } catch (e) {
      error = e as UiError;
    }
  }

  function complete(job: JobView) {
    finishedJob = job;
    screen = 'report';
  }
</script>

<div class="shell">
  <nav aria-label="Main">
    <strong class="brand">{t('app.name')}</strong>
    <button class:active={screen === 'home'} onclick={() => (screen = 'home')}>{t('nav.home')}</button>
    <button class:active={screen === 'analysis'} disabled={!input} onclick={() => (screen = 'analysis')}>{t('nav.analysis')}</button>
    <button class:active={screen === 'target' || screen === 'plan'} disabled={!input} onclick={chooseTarget}>{t('nav.target')}</button>
    <span class="spacer"></span>
    <select aria-label="Language" value={getLang()} onchange={(e) => setLang(e.currentTarget.value as 'vi' | 'en')}>
      <option value="en">English</option>
      <option value="vi">Tiếng Việt</option>
    </select>
  </nav>
  <main>
    {#if error}<div class="error" role="alert">{t(`error.${error.kind}`)}: {error.message}</div>{/if}
    {#if screen === 'home'}
      <Home onAnalyze={analyze} />
    {:else if screen === 'analysis'}
      <Analysis {input} onContinue={chooseTarget} />
    {:else if screen === 'target'}
      <Target {input} onContinue={preview} />
    {:else if screen === 'plan' && request}
      <Plan {request} {mode} {output} onRun={start} />
    {:else if screen === 'run'}
      <Run {jobId} onFinished={complete} />
    {:else if screen === 'report' && finishedJob}
      <Report job={finishedJob} {input} onBuild={() => (screen = 'build')} />
    {:else if screen === 'build'}
      <h1>{t('build.title')}</h1>
      <p>{t('build.comingSoon')}</p>
    {/if}
  </main>
</div>
