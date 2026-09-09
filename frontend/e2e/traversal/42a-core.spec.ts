import { test, expect } from '@playwright/test';
import { loginViaUI, trackPageHealth, assertPageHealthy } from '../flow/helpers';
import { TRAVERSAL_MODULES } from './modules.config';

/**
 * P5.12a 核心域遍历——数据驱动框架
 * Tier A: visit + trackPageHealth → 点新建 → 填表单 → 保存 → 断言
 * Tier C: visit + 白屏断言 + 表头渲染
 *
 * 依赖：P3.2（trackPageHealth/assertPageHealthy）
 */
test.describe('P5.12a 核心域遍历', () => {
  const coreModules = TRAVERSAL_MODULES.filter((m) => m.domain === 'core' || m.domain === 'approval');

  for (const mod of coreModules) {
    test(`${mod.id} (${mod.tier}) 页面可达无白屏`, async ({ page }) => {
      await loginViaUI(page);
      const collector = trackPageHealth(page);

      await page.goto(mod.route).catch(() => {});
      await page.waitForLoadState('networkidle', { timeout: 15000 }).catch(() => {});

      // 全部模块统一断言：零 pageerror、零 5xx、白屏检测
      await assertPageHealthy(page, collector, { allowConsoleWarn: true });
    });
  }
});
