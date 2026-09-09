import { test, expect } from '@playwright/test';
import { loginViaUI, trackPageHealth, assertPageHealthy } from '../flow/helpers';
import { TRAVERSAL_MODULES } from './modules.config';

/**
 * P5.12d 财务/系统/BI域遍历
 */
test.describe('P5.12d 财务/系统/BI域遍历', () => {
  const financeModules = TRAVERSAL_MODULES.filter(
    (m) => m.domain === 'finance' || m.domain === 'bi',
  );

  for (const mod of financeModules) {
    test(`${mod.id} (${mod.tier}) 页面可达无白屏`, async ({ page }) => {
      await loginViaUI(page);
      const collector = trackPageHealth(page);

      await page.goto(mod.route).catch(() => {});
      await page.waitForLoadState('networkidle', { timeout: 15000 }).catch(() => {});

      await assertPageHealthy(page, collector, { allowConsoleWarn: true });
    });
  }
});
