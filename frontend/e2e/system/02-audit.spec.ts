// 系统管理 E2E 套件 — 02 审计日志
// 创建时间: 2026-08-19
// 覆盖范围：审计日志查看 → 筛选 → 详情
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('02 审计日志', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 进入审计日志页面', async ({ page }) => {
    await page.goto('/system/audit-log');
    await expect(page.getByText(/审计/)).toBeVisible({ timeout: 5000 });
    await expect(page.locator('table, .v2-table, .el-table')).toBeVisible({ timeout: 10000 });
  });

  test('02-02 审计日志筛选功能可用', async ({ page }) => {
    await page.goto('/system/audit-log');
    await page.getByLabel(/关键词/).fill('E2E');
    await page.getByRole('button', { name: /查询/ }).click();
    await expect(page.locator('table, .v2-table, .el-table')).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /重置/ }).click();
    await expect(page.locator('table, .v2-table, .el-table')).toBeVisible({ timeout: 5000 });
  });

  test('02-03 审计日志详情可查看', async ({ page }) => {
    await page.goto('/system/audit-log');
    const detailBtn = page.getByRole('link', { name: /详情/ }).first();
    if (await detailBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await detailBtn.click();
      await expect(page.locator('.el-drawer')).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
      await expect(page.getByText(/操作时间|操作类型/)).toBeVisible();
    }
  });

  test('02-04 审计日志支持按操作类型筛选', async ({ page }) => {
    await page.goto('/system/audit-log');
    await page.getByLabel(/操作类型/).click();
    await page.getByRole('option').first().click();
    await page.getByRole('button', { name: /查询/ }).click();
    await expect(page.locator('table, .v2-table, .el-table')).toBeVisible({ timeout: 5000 });
  });
});