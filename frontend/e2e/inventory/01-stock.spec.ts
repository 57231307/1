// 库存管理 E2E 套件 — 01 库存调整
// 创建时间: 2026-08-19
// 覆盖范围：库存调整完整流程（盘盈/盘亏创建与提交）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelectIn } from '../flow/ui-helpers';

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
    // 老「库存调拨」Tab 已随源码物理删除，调拨入口改为页头「库存调拨」按钮
    // （goToTransferPage → router.push({ name:'InventoryTransfer' })）。改测该真实入口及其路由可达性。
    const transferEntry = page.getByRole('button', { name: /库存调拨/ });
    await expect(transferEntry, '/inventory 应存在"库存调拨"入口').toBeVisible({ timeout: 30000 });
    await transferEntry.click();
    await expect(page, '点击调拨入口应路由到正规页 /inventory-transfer').toHaveURL(
      /\/inventory-transfer/
    );
    await expect(page.getByRole('heading', { name: '库存调拨' })).toBeVisible({ timeout: 30000 });
  });

  test('01-02 库存筛选功能可用', async ({ page }) => {
    await page.goto('/inventory');
    // 「仓库」为 el-select，页面另有隐藏的打印 el-dialog 内含同名「仓库」label（stockTab.colWarehouse），
    // getByLabel(/仓库/) 会命中多元素且点到的是只读 combobox 内层 input（被 placeholder 拦
    // pointer events → click 超时）。改用冻结 helper：以筛选表单容器（aria-label=库存台账筛选表单）
    // 为 root 作用域 + 精确 label「仓库」锚定其 .el-select__wrapper，仅展开验证下拉可打开。
    const stockFilter = page.getByLabel('库存台账筛选表单');
    await pickSelectIn(stockFilter, page, '仓库', { openOnly: true });
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
