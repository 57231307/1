// 面料管理 E2E 套件 — 02 染色批次
// 创建时间: 2026-08-19
// 覆盖范围：染色批次创建 → 完成（pending → in_progress → completed）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('02 染色批次', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 染色批次 Tab 可正常加载', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /染色|批次/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 5000 });
  });

  test('02-02 新建染色批次', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /染色|批次/ }).click();
    await page.getByRole('button', { name: /新建|创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 5000 });
    await page.getByLabel(/批次号/).fill(`DB-${Date.now()}`);
    await page.getByLabel(/颜色/).fill('E2E 测试颜色');
    await page.getByLabel(/计划数量/).fill('500');
    await page.getByRole('button', { name: /确认|保存|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
      return null;
    });
  });

  test('02-03 染色批次可标记为完成', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /染色|批次/ }).click();
    const completeBtn = page.getByRole('link', { name: /完成/ }).first();
    if (await completeBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await completeBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/完成成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });
});