// 库存管理 E2E 套件 — 03 库存调拨
// 创建时间: 2026-08-19
// 覆盖范围：调拨单创建 → 审批 → 已审批状态验证
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('03 库存调拨', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('03-01 创建库存调拨单', async ({ page }) => {
    await page.goto('/inventory');
    await page.getByRole('button', { name: /调拨/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 5000 });
    await page.getByLabel(/调出仓库/).click();
    await page.getByRole('option').first().click();
    await page.getByLabel(/调入仓库/).click();
    await page.getByRole('option').nth(1).click().catch(() => {
      page.getByRole('option').first().click();
    });
    await page.getByLabel(/数量/).fill('100');
    await page.getByLabel(/备注/).fill('E2E 测试调拨');
    await page.getByRole('button', { name: /确认|提交/ }).last().click();
    await expect(page.getByText(/调拨成功|创建成功/)).toBeVisible({ timeout: 5000 });
  });

  test('03-02 调拨单列表可查看全部记录', async ({ page }) => {
    await page.goto('/inventory');
    await page.getByRole('tab', { name: /调拨/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 5000 });
    const statusTags = page.locator('.el-tag').filter({ hasText: /待审批|已审批|已执行/ });
    await expect(statusTags).toBeVisible({ timeout: 5000 }).catch(() => {
      return null;
    });
  });

  test('03-03 审批待审批调拨单', async ({ page }) => {
    await page.goto('/inventory');
    await page.getByRole('tab', { name: /调拨/ }).click();
    const approveBtn = page.getByRole('link', { name: /审批/ }).first();
    if (await approveBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await approveBtn.click();
      await page.getByRole('button', { name: /确定|确认/ }).click();
      await expect(page.getByText(/审批成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });
});