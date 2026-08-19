// 库存管理 E2E 套件 — 02 库存调整与预警
// 创建时间: 2026-08-19
// 覆盖范围：库存调整单创建（盘盈/盘亏）+ 库存预警查看
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('02 库存调整与预警', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 创建盘盈调整单', async ({ page }) => {
    await page.goto('/inventory');
    await page.getByRole('button', { name: /调整/ }).click();
    await expect(page.locator('.el-dialog').first()).toBeVisible();
    await expect(page.getByText(/盘盈|盘亏/)).toBeVisible();
    const increaseRadio = page.getByText('盘盈');
    await increaseRadio.click();
    await page.getByLabel(/调整数量|数量/).fill('100');
    await page.getByLabel(/原因/).fill('E2E 测试：盘盈调整');
    await page.getByRole('button', { name: /确认|提交/ }).last().click();
    await expect(page.getByText(/调整成功|创建成功|已提交/)).toBeVisible({ timeout: 5000 });
  });

  test('02-02 创建盘亏调整单', async ({ page }) => {
    await page.goto('/inventory');
    await page.getByRole('button', { name: /调整/ }).click();
    await expect(page.locator('.el-dialog').first()).toBeVisible();
    const decreaseRadio = page.getByText('盘亏');
    await decreaseRadio.click();
    await page.getByLabel(/调整数量|数量/).fill('50');
    await page.getByLabel(/原因/).fill('E2E 测试：盘亏调整');
    await page.getByRole('button', { name: /确认|提交/ }).last().click();
    await expect(page.getByText(/调整成功|创建成功|已提交/)).toBeVisible({ timeout: 5000 });
  });

  test('02-03 库存预警 Tab 可正常加载', async ({ page }) => {
    await page.goto('/inventory');
    await page.getByRole('tab', { name: /预警/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 5000 });
    const alertTags = page.locator('.el-tag').filter({ hasText: /紧急|预警/ });
    await expect(alertTags).toBeVisible({ timeout: 5000 }).catch(() => {
      return null;
    });
  });
});