// The only file that calls `invoke` (ADR 0004: the UI talks to application services only).
import { invoke } from '@tauri-apps/api/core';
import type {
  AnalysisView,
  JobView,
  PlanRequest,
  PlanView,
  PortRequest,
  RiskView,
  SessionView,
  TargetsView,
  UiError,
} from './types';

/** Turns whatever `invoke` rejected with into a `UiError` the screens can show. */
export function toUiError(e: unknown): UiError {
  if (e && typeof e === 'object' && 'kind' in e && 'message' in e) {
    return e as UiError;
  }
  return { kind: 'internal', message: String(e) };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toUiError(e);
  }
}

export const api = {
  analyze: (input: string) => call<AnalysisView>('analyze', { input }),
  scan: (input: string) => call<RiskView>('scan', { input }),
  sessions: (limit = 10) => call<SessionView[]>('sessions', { limit }),
  targets: () => call<TargetsView>('targets'),
  plan: (request: PlanRequest) => call<PlanView>('plan', { request }),
  startPort: (request: PortRequest) => call<string>('start_port', { request }),
  startMigrateInPlace: (request: PlanRequest, reviewed: boolean) =>
    call<string>('start_migrate_in_place', { request, reviewed }),
  job: (id: string) => call<JobView>('job', { id }),
  cancel: (id: string) => call<void>('cancel', { id }),
  rollback: (project: string, checkpoint: string | null) =>
    call<void>('rollback', { project, checkpoint }),
};
