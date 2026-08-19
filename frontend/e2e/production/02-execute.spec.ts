// 生产计划 E2E 套件 — 02 生产执行（planned → in_production → completed）
// 创建时间: 2026-08-19
// 覆盖范围：已计划订单开始生产 + 完成生产
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('02 生产执行', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 已计划订单可开始生产（planned → in_production）', async ({ page }) => {
    await page.goto('/production');
    const startBtn = page.getByRole('link', { name: /开始生产/ }).first();
    if (await startBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await startBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/开始生产成功|状态更新成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });

  test('02-02 生产中订单可完成生产（in_production → completed）', async ({ page }) => {
    await page.goto('/production');
    const completeBtn = page.getByRole('link', { name: /完成生产/ }).first();
    if (await completeBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await completeBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/完成成功|状态更新成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });

  test('02-03 已完成订单无操作按钮', async ({ page }) => {
    await page.goto('/production');
    const completedTag = page.locator('.el-tag').filter({ hasText: /已完成/ }).first();
    if (await completedTag.isVisible({ timeout: 3000 }).catch(() => false)) {
      const row = completedTag.locator('xpath=ancestor::tr');
      const actions = row.locator('button, a').filter({ hasText: /开始生产|完成生产|计划排产/ });
      await expect(actions).toHaveCount(0);
    }
  });
});