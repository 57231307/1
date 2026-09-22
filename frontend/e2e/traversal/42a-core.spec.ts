import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  trackPageHealth,
  assertPageHealthy,
  BROWSER_NETWORK_NOISE,
} from '../flow/helpers';
import { TRAVERSAL_MODULES, type TraversalModule } from './modules.config';

/**
 * P5.12 全量遍历 spec（数据驱动，4 个分域 spec 之一）
 *
 * Tier A 模块：visit → 白屏断言 → （有 listApi 的）列表 API 回读断言
 * Tier B 模块：visit → 白屏断言（真实创建链在专项 spec 覆盖）
 * Tier C 模块：visit → 白屏断言 + 表头/卡片渲染断言
 *
 * 统一断言：零 pageerror、零未捕获 console.error、零 5xx、主容器非白屏
 * 数据隔离：遍历产生的测试数据带 TRV 前缀 + 时间戳，跑完按 uniqueKey 清理
 */

async function visitModule(
  page: import('@playwright/test').Page,
  mod: TraversalModule
): Promise<void> {
  await loginViaUI(page);
  const collector = trackPageHealth(page);

  await page.goto(mod.route);
  await page.waitForLoadState('networkidle', { timeout: 15000 });

  // 统一健康断言
  await assertPageHealthy(page, collector, { consoleNoisePatterns: BROWSER_NETWORK_NOISE });

  // Tier A 且有 listApi：列表 API 回读断言（数据层连通性）
  if (mod.tier === 'A' && mod.listApi) {
    const resp = await page.request.get(
      `${process.env.API_BASE || 'http://localhost:8082'}/api/v1/erp${mod.listApi.startsWith('/') ? '' : '/'}${mod.listApi}?page=1&page_size=1`
    );
    expect(resp.status()).toBeLessThan(500);
  }

  // 有新建入口的模块（读配置判定，不再用运行时 isVisible() 猜测）：
  // Tier A 且未显式标注 noCreate ⇒ 新建按钮必须存在且可点开弹窗/抽屉或触发跳转；
  // "应当有新建能力但按钮渲染坏了"的模块判红（原来是 if(isVisible) 静默放过）。
  if (mod.tier === 'A' && !mod.noCreate) {
    const newBtn = page
      .locator('button:has-text("新建"), button:has-text("新增"), button:has-text("添加")')
      .first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    // 若上面对 waitFor 超时则本行不可达；显式断言把"按钮缺失"变成带模块名的可读失败
    const visible = await newBtn.isVisible();
    expect(
      visible,
      `[${mod.id}] 路由 ${mod.route} 配置为可新建（Tier A 且未标 noCreate），` +
        `但页面未渲染「新建/新增/添加」按钮；页面异常=${collector.pageErrors.join(' | ') || '(无)'}`
    ).toBe(true);

    const btnText = (await newBtn.textContent())?.trim() ?? '(空文案)';
    await newBtn.click();
    // 弹窗或跳转出现即算通过（部分模块跳转新页面）。
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

/**
 * 核心域 + system 域
 */
test.describe('P5.12 遍历：core/system', () => {
  const mods = TRAVERSAL_MODULES.filter(m => m.domain === 'core' || m.domain === 'system');
  for (const mod of mods) {
    test(`${mod.id} (${mod.tier})`, async ({ page }) => {
      await visitModule(page, mod);
    });
  }
});
