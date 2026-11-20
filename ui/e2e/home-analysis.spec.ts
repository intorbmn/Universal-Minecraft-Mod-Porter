import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';

const golden = (n: string) =>
  JSON.parse(readFileSync(`../tests/fixtures/golden/app-views/${n}.json`, 'utf8'));

test('Home to Analysis shows the mod and risk from golden JSON', async ({ page }) => {
  const views = { analyze: golden('analysis'), scan: golden('risk'), sessions: golden('sessions') };
  await page.addInitScript((v) => {
    // Minimal stand-in for the Tauri IPC bridge that @tauri-apps/api calls.
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
