// P9-4 采购 E2E 套件 — 01 创建采购订单
// 创建时间: 2026-06-17
// 覆盖范围：采购订单创建全流程（5 用例）

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

/**
 * 测试套件：采购订单创建
 *
 * 业务流程：
 * 1. 进入采购 → 采购订单 → 新建
 * 2. 选择供应商（必填）
 * 3. 选择产品行（必填 ≥1 行）
 * 4. 设置采购数量、单价、税率
 * 5. 提交并验证采购订单号生成
 */
test.describe('01 创建采购订单', () => {
  test.beforeEach(async ({ page, context }) => {
    // V15 Batch 487 P0-T05：注入 auth mock，业务 API 走真实后端（applyAuthMocks 不再 mock 业务 API）
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入采购订单列表页可见菜单', async ({ page }) => {
    // 采购管理为扁平单页（router index.ts:223 path:'purchase' → views/purchase/index.vue 采购订单列表），
    // 无 /purchase/order/list 子路由 → 原落 404
    await page.goto('/purchase');
    // PurchaseTop 渲染 <h1>采购管理</h1>，breadcrumb 也渲染同名 meta.title，需 .first()
    await expect(page.getByText('采购管理').first()).toBeVisible();
    // 新建按钮真实文案 purchase.top.create = '新建采购单'
    await expect(page.getByRole('button', { name: /新建采购单/ })).toBeVisible();
  });

  test('01-02 创建空采购订单应校验失败', async ({ page }) => {
    // 采购建单为 /purchase 页内 PurchaseCreateDialog 对话框（router 无 /purchase/order/create）；
    // 先进真实扁平路由，再由「新建采购单」按钮打开建单对话框驱动
    await page.goto('/purchase');
    await page.getByRole('button', { name: /新建采购单/ }).click();
    // 不填任何字段直接提交（确认按钮文本 purchase.createDlg.confirm = '确定'）
    await page.getByRole('button', { name: '确定', exact: true }).click();
    // 应显示供应商必填错误
    await expect(page.getByText('请选择供应商')).toBeVisible();
  });

  test('01-03 创建有效采购订单成功并生成采购单号', async ({ page }) => {
    // 采购建单为 /purchase 页内 PurchaseCreateDialog 对话框
    await page.goto('/purchase');
    await page.getByRole('button', { name: /新建采购单/ }).click();
    const dlg = page.locator('.el-dialog:visible');
    await expect(dlg).toBeVisible({ timeout: 30000 });
    // 选择供应商（form-item label = '供应商'）
    // 供应商是 el-select 的 readonly combobox，getByLabel 命中内层 input，EP 拦截其直接
    // click → 30s 超时。改锚含该 label 的 form-item 内 .el-select 触发，选项取 body-level
    // popper 的 .el-select-dropdown__item（对齐 8c1adf03 ai/crm 既有写法）。
    const supplierItem = dlg
      .locator('.el-form-item')
      .filter({ has: dlg.locator('.el-form-item__label', { hasText: '供应商' }) })
      .first();
    await supplierItem.locator('.el-select').first().click();
    await page.locator('.el-select-dropdown:visible .el-select-dropdown__item').first().click();
    // 产品：div 明细行内 el-select（placeholder '选择产品'），非 form-item label
    await dlg.locator('.items-row').first().locator('.el-select').first().click();
    await page.locator('.el-select-dropdown:visible .el-select-dropdown__item').first().click();
    // 数量 / 单价（el-input-number → spinbutton，在 .items-row 内无标签）
    const spinbuttons = dlg.locator('.items-row').first().locator('input[type="number"]');
    await spinbuttons.first().fill('200');
    await spinbuttons.nth(1).fill('30');
    // 提交（确认按钮 purchase.createDlg.confirm = '确定'）
    await dlg.getByRole('button', { name: '确定', exact: true }).click();
    // 真实成功提示 purchase.message.purchaseOrderCreated = '采购单创建成功'
    await expect(page.getByText('采购单创建成功')).toBeVisible({ timeout: 30000 });
  });

  test('01-04 采购订单可指定要求交货日期', async ({ page }) => {
    // 采购建单为 /purchase 页内 PurchaseCreateDialog 对话框
    await page.goto('/purchase');
    await page.getByRole('button', { name: /新建采购单/ }).click();
    const dlg = page.locator('.el-dialog:visible');
    await expect(dlg).toBeVisible({ timeout: 30000 });
    // 供应商
    // 供应商是 el-select 的 readonly combobox，getByLabel 命中内层 input，EP 拦截其直接
    // click → 30s 超时。改锚含该 label 的 form-item 内 .el-select 触发，选项取 body-level
    // popper 的 .el-select-dropdown__item（对齐 8c1adf03 ai/crm 既有写法）。
    const supplierItem = dlg
      .locator('.el-form-item')
      .filter({ has: dlg.locator('.el-form-item__label', { hasText: '供应商' }) })
      .first();
    await supplierItem.locator('.el-select').first().click();
    await page.locator('.el-select-dropdown:visible .el-select-dropdown__item').first().click();
    // 要求交货日期（form-item label = '要求交货日期'，el-date-picker 内 input）
    const dateInput = dlg.getByLabel('要求交货日期');
    await dateInput.click();
    await dateInput.fill('2026-07-15');
    await page.keyboard.press('Enter');
    // 产品
    await dlg.locator('.items-row').first().locator('.el-select').first().click();
    await page.locator('.el-select-dropdown:visible .el-select-dropdown__item').first().click();
    // 数量/单价（spinbutton in items-row）
    const spinbuttons = dlg.locator('.items-row').first().locator('input[type="number"]');
    await spinbuttons.first().fill('100');
    await spinbuttons.nth(1).fill('25.5');
    await dlg.getByRole('button', { name: '确定', exact: true }).click();
    await expect(page.getByText('采购单创建成功')).toBeVisible({ timeout: 30000 });
  });

  test('01-05 采购订单支持多产品行批量下单', async ({ page }) => {
    // 采购建单为 /purchase 页内 PurchaseCreateDialog 对话框
    await page.goto('/purchase');
    await page.getByRole('button', { name: /新建采购单/ }).click();
    const dlg = page.locator('.el-dialog:visible');
    await expect(dlg).toBeVisible({ timeout: 30000 });
    // 供应商是 el-select 的 readonly combobox，getByLabel 命中内层 input，EP 拦截其直接
    // click → 30s 超时。改锚含该 label 的 form-item 内 .el-select 触发，选项取 body-level
    // popper 的 .el-select-dropdown__item（对齐 8c1adf03 ai/crm 既有写法）。
    const supplierItem = dlg
      .locator('.el-form-item')
      .filter({ has: dlg.locator('.el-form-item__label', { hasText: '供应商' }) })
      .first();
    await supplierItem.locator('.el-select').first().click();
    await page.locator('.el-select-dropdown:visible .el-select-dropdown__item').first().click();
    // 添加行项：按钮文本 purchase.createDlg.addItem = '+ 添加明细'
    // 默认已有 1 行，添加 2 次 → 共 3 行
    await dlg.getByRole('button', { name: '+ 添加明细' }).click();
    await dlg.getByRole('button', { name: '+ 添加明细' }).click();
    // 验证明细行数为 3（div.items-row）
    const rows = dlg.locator('.items-row');
    await expect(rows).toHaveCount(3);
  });
});
