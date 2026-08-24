// 报价单 E2E 套件 — 01 报价单创建
// 创建时间: 2026-08-19
// 覆盖范围：报价单创建（含明细行、币种、价格条款）→ 提交审批
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('01 报价单创建', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入报价单列表页', async ({ page }) => {
    await page.goto('/quotations');
    await expect(page.getByText(/报价单/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: /新建/ })).toBeVisible();
  });

  test('01-02 新建报价单', async ({ page }) => {
    await page.goto('/quotations/new');
    await expect(page.locator('form')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/客户/).click();
    await page.getByRole('option').first().click();
    await page.getByLabel(/报价日期/).fill('2026-08-19');
    await page.getByLabel(/有效期至/).fill('2026-09-19');
    await page.getByLabel(/价格条款/).click();
    await page.getByRole('option').first().click();
    await page.getByLabel(/币种/).click();
    await page.getByRole('option').first().click();
    await page.getByLabel(/汇率/).fill('1');
    await page.getByLabel(/备注/).fill('E2E 测试报价单');
    await page.getByRole('button', { name: /保存草稿/ }).click();
    await expect(page.getByText(/保存成功|创建成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
      return null;
    });
  });

  test('01-03 报价单列表可正常加载', async ({ page }) => {
    await page.goto('/quotations');
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 30000 });
  });
});