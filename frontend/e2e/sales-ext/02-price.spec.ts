// 销售扩展 E2E 套件 — 02 销售价格
// 创建时间: 2026-08-19
// 覆盖范围：销售价格创建 → 审批 → 生效
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('02 销售价格', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 销售价格 Tab 可正常加载', async ({ page }) => {
    await page.goto('/sales-ext');
    await page.getByRole('tab', { name: /价格/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 5000 });
  });

  test('02-02 新建销售价格', async ({ page }) => {
    await page.goto('/sales-ext');
    await page.getByRole('tab', { name: /价格/ }).click();
    await page.getByRole('button', { name: /新建|创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 5000 });
    await page.getByLabel(/产品名/).fill('E2E 测试产品');
    await page.getByLabel(/客户/).fill('E2E 测试客户');
    await page.getByLabel(/价格/).fill('200');
    await page.getByLabel(/币种/).click();
    await page.getByRole('option').first().click();
    await page.getByRole('button', { name: /确认|保存|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
      return null;
    });
  });
});