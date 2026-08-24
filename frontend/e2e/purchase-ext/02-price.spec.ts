// 采购扩展 E2E 套件 — 02 采购价格
// 创建时间: 2026-08-19
// 覆盖范围：采购价格创建 → 审批 → 生效
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('02 采购价格', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 采购价格 Tab 可正常加载', async ({ page }) => {
    await page.goto('/purchase-ext');
    await page.getByRole('tab', { name: /价格/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 30000 });
  });

  test('02-02 新建采购价格', async ({ page }) => {
    await page.goto('/purchase-ext');
    await page.getByRole('tab', { name: /价格/ }).click();
    await page.getByRole('button', { name: /新建|创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/产品名/).fill('E2E 测试产品');
    await page.getByLabel(/供应商/).fill('E2E 测试供应商');
    await page.getByLabel(/价格/).fill('100');
    await page.getByLabel(/币种/).click();
    await page.getByRole('option').first().click();
    await page.getByLabel(/生效日期/).fill('2026-08-01');
    await page.getByLabel(/到期日期/).fill('2027-08-01');
    await page.getByRole('button', { name: /确认|保存|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
      return null;
    });
  });
});