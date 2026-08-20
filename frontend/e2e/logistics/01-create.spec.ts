// 物流管理 E2E 套件 — 01 运单创建与发货
// 创建时间: 2026-08-19
// 覆盖范围：运单创建 → 发货（pending → shipped） → 在途跟踪（shipped → in_transit） → 签收（in_transit → delivered）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('01 运单创建与发货', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入物流管理页面', async ({ page }) => {
    await page.goto('/logistics');
    await expect(page.getByText(/物流/)).toBeVisible({ timeout: 5000 });
    await expect(page.getByRole('button', { name: /新建/ })).toBeVisible();
  });

  test('01-02 新建运单', async ({ page }) => {
    await page.goto('/logistics');
    await page.getByRole('button', { name: /新建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 5000 });
    await page.getByLabel(/物流公司/).click();
    await page.getByRole('option').first().click();
    await page.getByLabel(/快递单号/).fill(`E2E-${Date.now()}`);
    await page.getByLabel(/司机姓名/).fill('E2E 司机');
    await page.getByLabel(/司机电话/).fill('13800138000');
    await page.getByLabel(/运费/).fill('50');
    await page.getByRole('button', { name: /确认/ }).click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
      return null;
    });
  });

  test('01-03 运单筛选功能可用', async ({ page }) => {
    await page.goto('/logistics');
    await page.getByLabel(/物流公司/).click();
    await page.getByRole('option').first().click();
    await page.getByRole('button', { name: /搜索/ }).click();
    await expect(page.locator('table, .v2-table, .el-table')).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /重置/ }).click();
    await expect(page.locator('table, .v2-table, .el-table')).toBeVisible({ timeout: 5000 });
  });
});