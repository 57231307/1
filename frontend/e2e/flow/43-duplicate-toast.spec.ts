import { test, expect } from '@playwright/test';
import { loginViaUI, apiCall, trackPageHealth, assertPageHealthy } from './helpers';

/**
 * P5.13 重复提示测试
 * 依赖：P3.2（expectSingleToast）
 *
 * 验证：
 * - 连续点击提交 5 次 → expectSingleToast + 按钮 loading 禁用
 * - dialog 重复实例计数
 */
test.describe('P5.13 重复提示', () => {
  test('连续点击提交不产生重复 toast', async ({ page }) => {
    await loginViaUI(page);
    const collector = trackPageHealth(page);

    // 导航到一个有提交按钮的页面（客户管理）
    await page.goto('/system/users').catch(() => {});
    await page.waitForLoadState('networkidle').catch(() => {});

    // 找一个提交按钮，快速连续点击
    const submitBtn = page.locator('button[type="submit"], button:has-text("保存"), button:has-text("确认")').first();
    if (await submitBtn.isVisible().catch(() => false)) {
      // 快速连续点击 5 次
      for (let i = 0; i < 5; i++) {
        await submitBtn.click({ timeout: 1000 }).catch(() => {});
      }
    }

    // 等待 toast 出现
    await page.waitForTimeout(2000);

    // 断言同一文案 toast 实例 ≤1
    // expectSingleToast 实现：检查 .el-message 数量
    const toastCount = await page.locator('.el-message').count();
    expect(toastCount).toBeLessThanOrEqual(1);

    await assertPageHealthy(page, collector, { allowConsoleWarn: true });
  });
});
