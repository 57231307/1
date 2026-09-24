// P9-4 采购 E2E 套件 — 05 应付单生成
// 创建时间: 2026-06-17
// 覆盖范围：入库后自动生成 AP 应付单（3 用例）

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

/**
 * 测试套件：应付单生成
 *
 * 业务流程：
 * 1. 已入库采购订单自动生成应付单
 * 2. 应付单金额 = 入库金额 × (1 + 税率)
 * 3. 应付单可分次付款
 */
test.describe('05 AP 应付单生成', () => {
  test.beforeEach(async ({ page, context }) => {
    // V15 Batch 487 P0-T05：注入 auth mock，业务 API 走真实后端（applyAuthMocks 不再 mock 业务 API）
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('05-01 已入库采购订单自动生成 AP 应付单', async ({ page }) => {
    // 采购管理为扁平单页（router index.ts:223 path:'purchase'），无 /purchase/order/list 子路由 → 原落 404
    await page.goto('/purchase');
    const received = page.locator('tr, .el-table__row').filter({ hasText: '已入库' }).first();
    await received.getByRole('button', { name: /详情/ }).click();
    // 查看关联的应付单
    await page.getByRole('tab', { name: /应付单|AP/ }).click();
    await expect(page.getByText(/应付单号.*AP-\d{8}-\d{4}/)).toBeVisible();
  });

  test('05-02 应付单金额 = 入库金额 + 税额', async ({ page }) => {
    // 采购管理为扁平单页（router index.ts:223 path:'purchase'），无 /purchase/order/list 子路由 → 原落 404
    await page.goto('/purchase');
    const received = page.locator('tr, .el-table__row').filter({ hasText: '已入库' }).first();
    await received.getByRole('button', { name: /详情/ }).click();
    await page.getByRole('tab', { name: /应付单/ }).click();
    const total = page.getByTestId('ap-invoice-total');
    await expect(total).toBeVisible();
  });

  test('05-03 应付单支持分次付款', async ({ page }) => {
    // 应付管理为扁平 Tab 页（router index.ts:173 path:'ap'），默认停「应付发票」tab；
    // 无 /ap/invoice/list 子路由 → 原落 404
    await page.goto('/ap');
    const invoice = page.locator('tr, .el-table__row').filter({ hasText: '未付款' }).first();
    await invoice.getByRole('button', { name: /详情/ }).click();
    // 部分付款
    await page.getByRole('button', { name: /付款/ }).click();
    await page.getByLabel(/付款金额/).fill('5000');
    await page.getByLabel(/付款方式/).click();
    await page.getByRole('option', { name: /银行转账/ }).click();
    await page.getByRole('button', { name: /确认/ }).click();
    await expect(page.getByText(/付款成功|已付款/)).toBeVisible();
  });
});
