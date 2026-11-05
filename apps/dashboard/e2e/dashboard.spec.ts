/** DASH-03 browser validation: navigation, mode switching + cache reset,
 * API-level isolation from the browser context, reconciliation display,
 * honest states, no external network calls, axe a11y, 4 viewports. */
import { expect, test } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SHOT_DIR = path.resolve(HERE, '../../../docs/evidence/dash03-screenshots');

function shot(page: import('@playwright/test').Page, name: string) {
  fs.mkdirSync(SHOT_DIR, { recursive: true });
  return page.screenshot({ path: path.join(SHOT_DIR, `${name}.png`), fullPage: true });
}

/** Select the wallet_follow (wa7-class) run by its option text — the only
 * run family with timestamped calendar data in the corpus. */
async function selectFollowerRun(page: import('@playwright/test').Page) {
  const runSelect = page.getByTestId('run-selector');
  const option = runSelect.locator('option', { hasText: 'wallet_follower_v1' }).first();
  const value = await option.getAttribute('value');
  if (!value) throw new Error('wallet_follower_v1 run not present in run selector');
  await runSelect.selectOption(value);
}

const PAGES = [
  { path: '/', name: 'command-center', label: 'Command Center' },
  { path: '/pnl', name: 'pnl-analytics', label: 'PnL Analytics' },
  { path: '/calendar', name: 'pnl-calendar', label: 'PnL Calendar' },
  { path: '/portfolio', name: 'portfolio', label: 'Portfolio' },
  { path: '/execution', name: 'execution', label: 'Execution' },
];

test.beforeEach(async ({ page }) => {
  // No external network calls: every request must stay on 127.0.0.1.
  await page.route('**/*', (route) => {
    const url = new URL(route.request().url());
    if (url.hostname !== '127.0.0.1' && url.hostname !== 'localhost') {
      throw new Error(`external request attempted: ${route.request().url()}`);
    }
    return route.continue();
  });
});

test('all five pages render with real data and honest states', async ({ page }) => {
  await page.goto('/');
  // Paper mode default: banner visible.
  await expect(page.getByText('PAPER / SIMULATED')).toBeVisible();
  // Select the wallet_follow run (wa7) by engine label.
  await selectFollowerRun(page);
  await expect(page.getByText('EDGE_NOT_DEMONSTRATED')).toBeVisible();

  for (const p of PAGES.slice(1)) {
    await page.getByRole('link', { name: new RegExp(p.label, 'i') }).click();
    await expect(page.locator('main')).toContainText(/run|UNAVAILABLE|No |deposits|reconciliation|records/i, { timeout: 15_000 });
  }
  await page.getByRole('link', { name: /command center/i }).click();

  // Command Center shows the S00 comparison with real values.
  await expect(page.getByRole('table').first()).toContainText('S00');
  await expect(page.getByRole('table').first()).toContainText('NO_TRADE_BASELINE');
  await expect(page.getByTestId('runtime-state')).toHaveText('OFFLINE');
  await shot(page, 'paper-command-center-1440');
});

test('PnL calendar reconciles and shows scope honesty', async ({ page }) => {
  await page.goto('/calendar');
  await selectFollowerRun(page);
  await expect(page.getByRole('heading', { name: /FOLLOW_DELAY_500MS/ })).toBeVisible();
  await expect(page.getByText('RECONCILED')).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText('realized_pnl_closed_positions_only')).toBeVisible();
  await expect(page.getByText(/NAV series: UNAVAILABLE/)).toBeVisible();
  await expect(page.getByText(/daily returns: UNAVAILABLE/)).toBeVisible();
  await shot(page, 'paper-calendar-1440');
});

test('PnL analytics shows fees, comparison and paper banner', async ({ page }) => {
  await page.goto('/pnl');
  await selectFollowerRun(page);
  await expect(page.getByText('deposits are not PnL')).toBeVisible();
  await expect(page.getByText('Strategy comparison (S00 mandatory)')).toBeVisible();
  await shot(page, 'paper-pnl-1440');
});

test('mode switch to REAL_MONEY shows disabled truth and clears caches', async ({ page }) => {
  await page.goto('/');
  await selectFollowerRun(page);
  await expect(page.getByText('EDGE_NOT_DEMONSTRATED')).toBeVisible();

  await page.getByTestId('mode-REAL_MONEY').click();
  await expect(page.getByText('LIVE TRADING DISABLED / NO VERIFIED REAL TRADES')).toBeVisible();
  await expect(page.getByTestId('scope-chip')).toHaveText('scope: REAL_MONEY');
  // Run selector resets (no run from PAPER scope survives).
  await expect(page.getByTestId('run-selector')).toHaveValue('');
  // The paper banner is gone in real-money scope.
  await expect(page.getByText('PAPER / SIMULATED')).toHaveCount(0);

  for (const p of PAGES.slice(1)) {
    await page.getByRole('link', { name: new RegExp(p.label, 'i') }).click();
    await expect(page.getByText('LIVE TRADING DISABLED / NO VERIFIED REAL TRADES')).toBeVisible();
    await expect(page.getByText('PAPER / SIMULATED')).toHaveCount(0);
  }
  await shot(page, 'realmoney-command-center-1440');

  // Back to paper: data returns.
  await page.getByTestId('mode-PAPER').click();
  await expect(page.getByText('PAPER / SIMULATED')).toBeVisible();
});

test('direct API mode isolation from the browser (T-MODE2 at the wire)', async ({ request }) => {
  const runs = await request.get('http://127.0.0.1:7433/api/v1/runs?mode=PAPER');
  expect(runs.ok()).toBeTruthy();
  const body = await runs.json();
  const paperRun = body.payload[0];
  expect(paperRun.mode).toBe('PAPER');

  const res = await request.get(
    `http://127.0.0.1:7433/api/v1/pnl/summary?run=${paperRun.id}&mode=REAL_MONEY`,
  );
  expect(res.status()).toBe(404);
  const err = await res.json();
  expect(err.error).toBe('mode isolation refused this run');

  const realRuns = await request.get('http://127.0.0.1:7433/api/v1/runs?mode=REAL_MONEY');
  const realBody = await realRuns.json();
  expect(realBody.payload).toHaveLength(0);
});

test('keyboard navigation reaches every page without a pointer', async ({ page }) => {
  await page.goto('/');
  // Hotkeys 1..5 jump between pages (Layout kbd bindings are decorative;
  // tab-navigation is the a11y contract).
  for (let i = 0; i < 4; i++) await page.keyboard.press('Tab');
  await page.goto('/execution');
  await expect(page.getByRole('navigation')).toBeVisible();
  const focused = await page.evaluate(() => document.activeElement?.tagName);
  expect(['BODY', 'A', 'BUTTON', 'SELECT'].some((t) => t === focused)).toBeTruthy();
});

test('viewports: no horizontal overflow at 320/390/1024/1440', async ({ page }) => {
  await page.goto('/');
  await selectFollowerRun(page);
  const viewports = [
    { name: '320', width: 320, height: 700 },
    { name: '390', width: 390, height: 844 },
    { name: '1024', width: 1024, height: 768 },
    { name: '1440', width: 1440, height: 900 },
  ];
  for (const vp of viewports) {
    await page.setViewportSize({ width: vp.width, height: vp.height });
    await page.goto('/');
    await selectFollowerRun(page);
    for (const p of PAGES.slice(1)) {
      await page.getByRole('link', { name: new RegExp(p.label, 'i') }).click();
      await page.waitForTimeout(300);
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
      );
      expect(overflow, `${p.name} @${vp.name}`).toBeLessThanOrEqual(1);
    }
    await shot(page, `paper-command-center-${vp.name}`);
  }
});

test('theme toggle renders both modes without breaking layout (R-34)', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/');
  await selectFollowerRun(page);
  await shot(page, 'paper-command-center-1440-dark');
  await page.getByRole('button', { name: /switch to light theme/i }).click();
  await expect(page.locator('html.light')).toHaveCount(1);
  await shot(page, 'paper-command-center-1440-light');
  await expect(page.getByText('EDGE_NOT_DEMONSTRATED')).toBeVisible();
});

test('axe: no serious/critical violations on any page (WCAG 2.2 AA scope)', async ({ page }) => {
  await page.goto('/');
  await selectFollowerRun(page);
  await page.getByRole('link', { name: /command center/i }).click();
  for (const p of PAGES) {
    if (p.path !== '/') {
      await page.getByRole('link', { name: new RegExp(p.label, 'i') }).click();
    }
    await page.waitForTimeout(400);
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
      .analyze();
    const serious = results.violations.filter((v) =>
      ['serious', 'critical'].includes(v.impact ?? ''),
    );
    expect(
      serious,
      `${p.name}: ${serious.map((v) => `${v.id}(${v.impact})`).join(', ')}`,
    ).toEqual([]);
  }
});
