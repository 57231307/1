import { test, expect } from '@playwright/test';
import { loginViaUI, trackPageHealth, assertPageHealthy } from '../flow/helpers';
import { TRAVERSAL_MODULES, type TraversalModule } from './modules.config';

/**
 * P5.12 全量遍历——sales/crm/supplier/product/fabric/inventory 域
 */
async function visitModule(page: import('@playwright/test').Page, mod: TraversalModule): Promise<void> {
  await loginViaUI(page);
  const collector = trackPageHealth(page);

  await page.goto(mod.route).catch(() => {});
  await page.waitForLoadState('networkidle', { timeout: 15000 }).catch(() => {});

  await assertPageHealthy(page, collector, { allowConsoleWarn: true });

  if (mod.tier === 'A' && mod.listApi) {
    const resp = await page.request.get(`${process.env.API_BASE || 'http://localhost:8082'}/api/v1/erp${mod.listApi.startsWith('/') ? '' : '/'}${mod.listApi}?page=1&page_size=1`);
    expect(resp.status()).toBeLessThan(500);
  }

  if (mod.tier === 'A' && !mod.noCreate) {
    const newBtn = page.locator('button:has-text("新建"), button:has-text("新增"), button:has-text("添加")').first();
    if (await newBtn.isVisible().catch(() => false)) {
      await newBtn.click().catch(() => {});
      await page.waitForTimeout(800);
      const dialogVisible = await page.locator('.el-dialog:visible, .el-drawer:visible').first().isVisible().catch(() => false);
      const navigated = page.url() !== `${process.env.BASE_URL || 'http://localhost:3000'}${mod.route}`;
      expect(dialogVisible || navigated).toBeTruthy();
    }
  }
}

test.describe('P5.12 遍历：sales/crm/supplier', () => {
  const mods = TRAVERSAL_MODULES.filter(
    (m) => ['sales', 'crm', 'supplier'].includes(m.domain),
  );
  for (const mod of mods) {
    test(`${mod.id} (${mod.tier})`, async ({ page }) => {
      await visitModule(page, mod);
    });
  }
});

test.describe('P5.12 遍历：product/fabric/inventory', () => {
  const mods = TRAVERSAL_MODULES.filter(
    (m) => ['product', 'fabric', 'inventory'].includes(m.domain),
  );
  for (const mod of mods) {
    test(`${mod.id} (${mod.tier})`, async ({ page }) => {
      await visitModule(page, mod);
    });
  }
});
