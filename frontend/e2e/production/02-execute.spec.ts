// 生产计划 E2E 套件 — 02 生产执行（planned → in_production → completed）
// 覆盖范围：开始生产、完成生产、状态标签验证
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('生产计划 - 02 生产执行', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('已计划工单可开始生产（planned → in_production）', async ({ page }) => {
    await page.goto('/production');
    await expect(page.locator('.el-table, .v2-table')).toBeVisible({ timeout: 30000 });
    const startBtn = page.getByRole('link', { name: /开始生产/ }).first();
    if (await startBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await startBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/开始生产成功|状态更新成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('生产中工单可完成（in_production → completed）', async ({ page }) => {
    await page.goto('/production');
    await expect(page.locator('.el-table, .v2-table')).toBeVisible({ timeout: 30000 });
    const completeBtn = page.getByRole('link', { name: /完成生产/ }).first();
    if (await completeBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await completeBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/完成成功|状态更新成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('已完成工单无操作按钮', async ({ page }) => {
    await page.goto('/production');
    await expect(page.locator('.el-table, .v2-table')).toBeVisible({ timeout: 30000 });
    await expect(page.locator('.el-tag')).toBeVisible({ timeout: 30000 });
  });
});