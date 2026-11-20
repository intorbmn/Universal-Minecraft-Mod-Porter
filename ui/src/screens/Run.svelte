<script lang="ts">
  import { api } from '../lib/api';
  import { onCoreEvent } from '../lib/events';
  import { t } from '../lib/i18n.svelte';
  import JobLog from '../lib/components/JobLog.svelte';
  import Stepper from '../lib/components/Stepper.svelte';
  import type { CoreEvent, JobView, UiError } from '../lib/types';

  let { jobId, onFinished }: { jobId: string; onFinished: (job: JobView) => void } = $props();
  let job = $state<JobView | null>(null);
  let error = $state<UiError | null>(null);
  let stopping = $state(false);
  const steps = ['validate', 'planning', 'porting', 'applying', 'verify', 'done'];

  async function refresh() {
    try {
      job = await api.job(jobId);
      if (job.state !== 'Running' && job.state !== 'Pending') onFinished(job);
    } catch (e) {
      error = e as UiError;
    }
  }

  async function stop() {
    stopping = true;
    try {
      await api.cancel(jobId);
    } catch (e) {
      error = e as UiError;
    }
  }

  $effect(() => {
    let alive = true;
    let unlisten: (() => void) | null = null;
    api.job(jobId).then((value) => {
      if (alive) {
        job = value;
        if (value.state !== 'Running' && value.state !== 'Pending') onFinished(value);
      }
    }).catch((e: UiError) => (error = e));
    onCoreEvent((event: CoreEvent) => {
      const payload = event.payload;
      if (payload.job !== jobId) return;
      if (event.kind === 'job.log' && typeof payload.line === 'string' && job) {
        job = { ...job, log: [...job.log, payload.line].slice(-200) };
      }
      if (event.kind === 'job.progress' && typeof payload.step === 'string') {
        const progress = {
          done: Number(payload.done ?? 0),
          total: Number(payload.total ?? 0),
        };
        if (job) job = { ...job, step: payload.step, progress };
      }
      if (event.kind === 'job.finished') void refresh();
    }).then((stopListening) => {
      if (alive) unlisten = stopListening;
      else stopListening();
    }).catch((e: UiError) => (error = e));
    return () => {
      alive = false;
      unlisten?.();
    };
  });
</script>

<h1>{t('run.title')}</h1>
{#if error}<div class="error" role="alert">{error.message}</div>{/if}
{#if job}
  <p><strong>{t(`state.${job.state}`)}</strong> · {job.step}</p>
  <Stepper {steps} current={job.step} />
  {#if job.progress}
    <progress max={job.progress.total || 1} value={job.progress.done}></progress>
    <span>{job.progress.done}/{job.progress.total}</span>
  {/if}
  <JobLog lines={job.log} />
  {#if job.state === 'Running' || job.state === 'Pending'}
    <button class="danger" onclick={stop} disabled={stopping}>{stopping ? t('run.stopping') : t('run.cancel')}</button>
  {:else}
    <p role="status">{job.state === 'Succeeded' ? t('run.finished') : t('run.ended')}</p>
  {/if}
{:else}
  <p role="status">{t('run.loading')}</p>
{/if}

<style>
  progress { width: min(520px, 75%); accent-color: var(--accent); margin-right: 10px; }
  .danger { margin-top: 12px; border-color: var(--high); color: var(--high); }
</style>