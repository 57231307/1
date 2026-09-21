import { test, expect } from '../diagnose-fixture';
import { loginViaUI, trackPageHealth, assertPageHealthy } from '../flow/helpers';
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
  await assertPageHealthy(page, collector, { allowConsoleWarn: true });

  // Tier A 且有 listApi：列表 API 回读断言（数据层连通性）
  if (mod.tier === 'A' && mod.listApi) {
    const resp = await page.request.get(
      `${process.env.API_BASE || 'http://localhost:8082'}/api/v1/erp${mod.listApi.startsWith('/') ? '' : '/'}${mod.listApi}?page=1&page_size=1`
    );
    expect(resp.status()).toBeLessThan(500);
  }

  // 有新建入口的 Tier A：点新建按钮弹窗出现
  if (mod.tier === 'A' && !mod.noCreate) {
    const newBtn = page
      .locator('button:has-text("新建"), button:has-text("新增"), button:has-text("添加")')
      .first();
    if (await newBtn.isVisible()) {
      const btnText = (await newBtn.textContent())?.trim() ?? '(空文案)';
      await newBtn.click();
      // 弹窗或跳转出现即算通过（部分模块跳转新页面）。
      // 原实现点完固定等 800ms 再 isVisible()，慢渲染的模块被误判为"无新建能力"，
      // 且失败信息不含任何可判责线索。
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
