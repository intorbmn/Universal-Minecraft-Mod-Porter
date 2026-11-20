import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';

const golden = (name: string) =>
  JSON.parse(readFileSync(`../tests/fixtures/golden/app-views/${name}.json`, 'utf8'));

test('Home to Analysis shows the mod and risk from golden JSON', async ({ page }) => {
  const views = { analyze: golden('analysis'), scan: golden('risk'), sessions: golden('sessions') };
  await page.addInitScript((v) => {
    (window as any).__TAURI_INTERNALS__ = {
      invoke: async (cmd: string) => (v as Record<string, unknown>)[cmd],
      transformCallback: () => 0,
    };
  }, views);
  await page.goto('/');
  await expect(page.getByText('boom')).toBeVisible();
  await page.getByLabel('Path to a mod').fill('/mods/demo.jar');
  await page.getByRole('button', { name: 'Analyze' }).click();
  await expect(page.getByTestId('mod-id')).toHaveText('demomod');
  await expect(page.getByTestId('risk-badge').first()).toContainText('High risk');
});

test('Target → Plan → reviewed in-place Run → Report → Rollback', async ({ page }) => {
  const completedJob = {
    id: 'job-11c',
    session: 'session-11c',
    step: 'done',
    state: 'Succeeded',
    progress: { done: 5, total: 5 },
    log: ['migration completed'],
    result: {
      checkpoint: 'checkpoint-11c',
      report: {
        files_changed: 2, replacements: 3, moves: 1, deletes: 0,
        skipped_mappings: 0, breaking_changes: 0, warnings: [],
        technologies: [], checkpoint_id: 'checkpoint-11c',
      },
    },
    error: null,
  };
  await page.addInitScript(({ job }) => {
    const plan = {
      source_minecraft: '1.20.1',
      target_minecraft: '1.21.1',
      source_loader: 'fabric',
      target_loader: 'neoforge',
      files: [{
        path: 'src/main/resources/fabric.mod.json',
        kind: 'modified',
        diff: '--- a/fabric.mod.json\n+++ b/fabric.mod.json\n@@ -1 +1 @@\n-old\n+new',
        replacements: 1,
        needs_attention: false,
      }],
      moves: [], deletes: [], technologies: [], skipped: [], warnings: [], files_scanned: 3,
    };
    (window as any).__TAURI_INTERNALS__ = {
      invoke: async (cmd: string, args: any) => {
        if (cmd === 'analyze') return {
          kind: 'folder',
          mods: [{ id: 'demomod', name: 'Demo Mod', version: '1.0', loader: 'fabric', minecraft: '1.20.1', java: 21 }],
          build_system: 'gradle', languages: ['Java'], technologies: [],
        };
        if (cmd === 'scan') return { level: null, findings: [] };
        if (cmd === 'sessions') return [];
        if (cmd === 'targets') return {
          loaders: ['forge', 'fabric', 'neoforge', 'quilt'],
          versions: [{ minecraft: '1.21.1', loaders: ['fabric', 'neoforge'] }],
        };
        if (cmd === 'plan') return plan;
        if (cmd === 'start_migrate_in_place') return 'job-11c';
        if (cmd === 'job') return job;
        if (cmd === 'rollback') return null;
        if (cmd === 'plugin:event|listen') return 1;
        if (cmd === 'plugin:event|unlisten') return null;
        return null;
      },
      transformCallback: () => 0,
      unregisterCallback: () => undefined,
    };
  }, { job: completedJob });
  await page.goto('/');
  await page.getByLabel('Path to a mod').fill('/mods/demo');
  await page.getByRole('button', { name: 'Analyze' }).click();
  await expect(page.getByTestId('mod-id')).toHaveText('demomod');
  await page.getByRole('button', { name: 'Choose target' }).click();
  await page.getByLabel('Minecraft version').selectOption('1.21.1');
  await page.getByLabel('Mod loader').selectOption('neoforge');
  await page.getByLabel('Migrate in place (checkpoint + rollback)').check();
  await page.getByRole('button', { name: 'Preview changes' }).click();
  await expect(page.getByText('src/main/resources/fabric.mod.json')).toBeVisible();
  const apply = page.getByRole('button', { name: 'Apply reviewed migration' });
  await expect(apply).toBeDisabled();
  await page.getByLabel('I reviewed the changes and understand what will be modified.').check();
  await expect(apply).toBeEnabled();
  await apply.click();
  await expect(page.getByRole('heading', { name: 'Migration report' })).toBeVisible();
  await expect(page.getByText('Files changed')).toBeVisible();
  await page.getByRole('button', { name: 'Roll back in-place changes' }).click();
  await expect(page.getByRole('button', { name: 'Rolled back' })).toBeVisible();
});
