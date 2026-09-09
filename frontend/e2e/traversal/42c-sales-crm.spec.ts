import { test, expect } from '@playwright/test';
import { loginViaUI, trackPageHealth, assertPageHealthy } from '../flow/helpers';
import { TRAVERSAL_MODULES } from './modules.config';

/**
 * P5.12c 销售/CRM/质量域遍历
 */
test.describe('P5.12c 销售/CRM/质量域遍历', () => {
  const salesModules = TRAVERSAL_MODULES.filter(
    (m) => m.domain === 'sales' || m.domain === 'crm' || m.domain === 'quality',
  );

  for (const mod of salesModules) {
    test(`${mod.id} (${mod.tier}) 页面可达无白屏`, async ({ page }) => {
      await loginViaUI(page);
      const collector = trackPageHealth(page);

      await page.goto(mod.route).catch(() => {});
      await page.waitForLoadState('networkidle', { timeout: 15000 }).catch(() => {});

      await assertPageHealthy(page, collector, { allowConsoleWarn: true });
    });
  }
});
