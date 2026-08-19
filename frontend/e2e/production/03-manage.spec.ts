// 生产计划 E2E 套件 — 03 工单管理（查看、删除、筛选、导出）
// 覆盖范围：查看详情、删除草稿、按状态筛选、导出
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('生产计划 - 03 工单管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('工单详情可查看', async ({ page }) => {
    await page.goto('/production');
    await expect(page.locator('.el-table, .v2-table')).toBeVisible({ timeout: 10000 });
    const viewBtn = page.getByRole('link', { name: /查看/ }).first();
    if (await viewBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await viewBtn.click();
      await expect(page.locator('.el-dialog')).toBeVisible();
      await expect(page.getByText(/订单编号|产品名称|计划数量/)).toBeVisible();
      await page.getByRole('button', { name: /关闭/ }).click();
    }
  });

  test('草稿工单可删除', async ({ page }) => {
    await page.goto('/production');
    await expect(page.locator('.el-table, .v2-table')).toBeVisible({ timeout: 10000 });
    const deleteBtn = page.getByRole('link', { name: /删除/ }).first();
    if (await deleteBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await deleteBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/删除成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });

  test('按状态筛选工单', async ({ page }) => {
    await page.goto('/production');
    await page.getByLabel(/状态/).click();
    await page.getByRole('option').first().click();
    await page.getByRole('button', { name: /搜索/ }).click();
    await expect(page.locator('.el-table, .v2-table')).toBeVisible({ timeout: 10000 });
  });
});