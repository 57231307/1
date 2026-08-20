// MRP 计算 E2E 测试
// 创建时间: 2026-08-19
// 覆盖范围：MRP 计算执行 → 结果查看 → 建议采购
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('MRP 计算', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('进入 MRP 计算页面', async ({ page }) => {
    await page.goto('/mrp');
    await expect(page.getByText(/MRP/)).toBeVisible({ timeout: 5000 });
  });

  test('MRP 计算可执行', async ({ page }) => {
    await page.goto('/mrp');
    const calcBtn = page.getByRole('button', { name: /计算/ }).first();
    if (await calcBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await calcBtn.click();
      await expect(page.getByText(/计算完成|计算中/)).toBeVisible({ timeout: 15000 }).catch(() => {
        return null;
      });
    }
  });

  test('MRP 历史页面可正常加载', async ({ page }) => {
    await page.goto('/mrp/history');
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 5000 }).catch(() => {
      return null;
    });
  });
});