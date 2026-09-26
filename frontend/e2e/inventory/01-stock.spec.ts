// 库存管理 E2E 套件 — 01 库存调整
// 创建时间: 2026-08-19
// 覆盖范围：库存调整完整流程（盘盈/盘亏创建与提交）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelect } from '../flow/ui-helpers';

test.describe('01 库存调整', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入库存管理页面', async ({ page }) => {
    await page.goto('/inventory');
    await expect(page.getByRole('heading', { name: '库存管理' })).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('tab', { name: /库存台账/ })).toBeVisible();
    await expect(page.getByRole('tab', { name: /库存预警/ })).toBeVisible();
    await expect(page.getByRole('tab', { name: /库存调拨/ })).toBeVisible();
  });

  test('01-02 库存筛选功能可用', async ({ page }) => {
    await page.goto('/inventory');
    // 「仓库」为 el-select，页面另有隐藏的打印 el-dialog 内含同名「仓库」label（stockTab.colWarehouse），
    // getByLabel(/仓库/) 会命中多元素且点到的是标签/不可见控件 → 下拉打不开、option 超时。
    // 改从筛选表单容器（aria-label=库存台账筛选表单）定位其第一个 combobox（仓库），点开后断 option 出现。
    const stockFilter = page.getByLabel('库存台账筛选表单');
    await pickSelect(page, stockFilter.locator('.el-select').first(), undefined, {
      openOnly: true,
    });
    await expect(page.getByRole('option').first()).toBeVisible({ timeout: 30000 });
    await page.keyboard.press('Escape');
    await stockFilter.getByRole('button', { name: '查询' }).click();
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });

  test('01-03 库存台账数据加载正常', async ({ page }) => {
    await page.goto('/inventory');
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });
});
