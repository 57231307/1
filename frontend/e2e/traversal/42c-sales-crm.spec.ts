import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  trackPageHealth,
  assertPageHealthy,
  BROWSER_NETWORK_NOISE,
} from '../flow/helpers';
import { TRAVERSAL_MODULES, type TraversalModule } from './modules.config';

/**
 * P5.12 全量遍历——sales/crm/supplier/product/fabric/inventory 域
 */
async function visitModule(
  page: import('@playwright/test').Page,
  mod: TraversalModule
): Promise<void> {
  await loginViaUI(page);
  const collector = trackPageHealth(page);

  await page.goto(mod.route);
  await page.waitForLoadState('networkidle', { timeout: 15000 });

  await assertPageHealthy(page, collector, { consoleNoisePatterns: BROWSER_NETWORK_NOISE });

  if (mod.tier === 'A' && mod.listApi) {
    const resp = await page.request.get(
      `${process.env.API_BASE || 'http://localhost:8082'}/api/v1/erp${mod.listApi.startsWith('/') ? '' : '/'}${mod.listApi}?page=1&page_size=1`
    );
    expect(resp.status()).toBeLessThan(500);
  }

  // 有新建入口的模块（读配置判定，不再用运行时 isVisible() 猜测）：
  // Tier A 且未标 noCreate ⇒ 新建按钮必须可见且能点开/跳转（原来 if(isVisible) 静默放过）。
  if (mod.tier === 'A' && !mod.noCreate) {
    const newBtn = page
      .locator('button:has-text("新建"), button:has-text("新增"), button:has-text("添加")')
      .first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const visible = await newBtn.isVisible();
    expect(
      visible,
      `[${mod.id}] 路由 ${mod.route} 配置为可新建（Tier A 且未标 noCreate），` +
        `但页面未渲染「新建/新增/添加」按钮；页面异常=${collector.pageErrors.join(' | ') || '(无)'}`
    ).toBe(true);
    const btnText = (await newBtn.textContent())?.trim() ?? '(空文案)';
    await newBtn.click();
    const appeared = await page
      .locator('.el-dialog:visible, .el-drawer:visible')
      .first()
      .waitFor({ state: 'visible', timeout: 8000 })
      .then(() => true)
      .catch(() => false);
    const navigated =
      page.url() !== `${process.env.BASE_URL || 'http://localhost:3000'}${mod.route}`;
    expect(
      appeared || navigated,
      `[${mod.id}] 点击「${btnText}」后 8s 内既未出现弹窗/抽屉，URL 仍停在 ${page.url()}；` +
        `页面异常=${collector.pageErrors.join(' | ') || '(无)'}，` +
        `console 错误=${collector.consoleErrors.slice(0, 2).join(' | ') || '(无)'}`
    ).toBeTruthy();
  }
}

test.describe('P5.12 遍历：sales/crm/supplier', () => {
  const mods = TRAVERSAL_MODULES.filter(m => ['sales', 'crm', 'supplier'].includes(m.domain));
  for (const mod of mods) {
    test(`${mod.id} (${mod.tier})`, async ({ page }) => {
      await visitModule(page, mod);
    });
  }
});

test.describe('P5.12 遍历：product/fabric/inventory', () => {
  const mods = TRAVERSAL_MODULES.filter(m => ['product', 'fabric', 'inventory'].includes(m.domain));
  for (const mod of mods) {
    test(`${mod.id} (${mod.tier})`, async ({ page }) => {
      await visitModule(page, mod);
    });
  }
});
