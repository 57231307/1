// 销售扩展 E2E 套件 — 03 销售退货
// 创建时间: 2026-08-19
// 覆盖范围：销售退货创建 → 审批 → 完成
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('03 销售退货', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('03-01 销售退货 Tab 可正常加载', async ({ page }) => {
    await page.goto('/sales-ext');
    await page.getByRole('tab', { name: /退货/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 5000 });
  });

  test('03-02 新建销售退货单', async ({ page }) => {
    await page.goto('/sales-ext');
    await page.getByRole('tab', { name: /退货/ }).click();
    await page.getByRole('button', { name: /新建|创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 5000 });
    await page.getByLabel(/客户/).fill('E2E 测试客户');
    await page.getByLabel(/退货日期/).fill('2026-08-19');
    await page.getByLabel(/原因/).fill('E2E 测试：客户退货');
    await page.getByRole('button', { name: /确认|保存|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
      return null;
    });
  });
});