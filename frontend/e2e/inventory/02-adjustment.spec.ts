// 库存管理 E2E 套件 — 02 库存调整（盘盈/盘亏）
// 覆盖范围：库存调整对话框（increase/decrease）、表单填写、提交
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('库存管理 - 02 库存调整', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('库存调整 - 盘盈', async ({ page }) => {
    await page.goto('/inventory');
    await page.getByRole('button', { name: '库存调整' }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    // 调整类型是 radio 组件（AdjustmentDialog.vue:40），标签为「增加」/「减少」
    // （i18n typeIncrease/typeDecrease），非「盘盈」「盘亏」字面量。
    await expect(page.getByRole('radio', { name: '增加' })).toBeVisible();
    await page.getByRole('radio', { name: '增加' }).click();
    await page.getByLabel('调整数量').fill('50');
    await page.getByLabel('调整原因').fill('E2E 测试盘盈调整');
    await page.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText(/成功/)).toBeVisible({ timeout: 30000 });
  });

  test('库存调整 - 盘亏', async ({ page }) => {
    await page.goto('/inventory');
    await page.getByRole('button', { name: '库存调整' }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('radio', { name: '减少' })).toBeVisible();
    await page.getByRole('radio', { name: '减少' }).click();
    await page.getByLabel('调整数量').fill('30');
    await page.getByLabel('调整原因').fill('E2E 测试盘亏调整');
    await page.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText(/成功/)).toBeVisible({ timeout: 30000 });
  });
});
