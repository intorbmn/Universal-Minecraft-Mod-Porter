// Shapes returned by the Rust commands. The golden JSON in tests/fixtures/golden/app-views
// is checked against the Rust structs (core/tests/app.rs) and against these types (Vitest).

export type RiskLevel = 'low' | 'medium' | 'high';
export type JobState =
  | 'Pending'
  | 'Running'
  | 'Succeeded'
  | 'Failed'
  | 'Skipped'
  | 'TimedOut'
  | 'Cancelled';

export interface ModView {
  id: string;
  name: string;
  version: string;
  loader: string;
  minecraft: string;
  java: number | null;
}

export interface AnalysisView {
  kind: 'folder' | 'zip';
  mods: ModView[];
  build_system: string | null;
  languages: string[];
  technologies: unknown[];
}

export interface FindingView {
  rule: string;
  severity: RiskLevel;
  file: string;
  evidence: string;
}

export interface RiskView {
  level: RiskLevel | null;
  findings: FindingView[];
}

export interface SessionView {
  id: string;
  state: JobState;
  step: string;
  output: string | null;
  error: string | null;
}

export interface TargetsView {
  loaders: string[];
  versions: { minecraft: string; loaders: string[] }[];
}

export interface UiError {
  kind: 'input' | 'refused' | 'not_found' | 'cancelled' | 'internal';
  message: string;
}

/** Every job event arrives in this envelope on `ummp://event`. */
export interface CoreEvent {
  id: string;
  kind: string;
  payload: Record<string, unknown>;
  timestamp: string;
}
