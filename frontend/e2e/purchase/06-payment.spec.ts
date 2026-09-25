// P9-4 采购 E2E 套件 — 06 采购付款
// 创建时间: 2026-06-17
// 覆盖范围：采购付款全流程（4 用例）

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

/**
 * 测试套件：采购付款
 *
 * 业务流程：
 * 1. 应付单付款（一次性）
 * 2. 应付单部分付款
 * 3. 付款方式（5 种）
 * 4. 付款单打印
 */
/**
 * 状态标签事实来源（判责 #4647）：AP 发票列表状态列由
 * src/views/ap/tabs/InvoiceTab.vue:getInvoiceStatusLabel 渲染，后端写入值集
 * （models/ap_invoice.rs + ap_invoice_ops/crud.rs，finance.rs::INVOICE_AUDITED）与 i18n 文案
 * （src/locales/zh-CN.ts apModule.invoice.*）为：
 *   DRAFT→草稿 / AUDITED→已审核 / PARTIAL_PAID→部分付款 / PAID→已付清 / CANCELLED→已取消。
 * 深链 seed 建的是 AUDITED（已审核、未付款）发票，实体无 payment_status 列（payment_status 属
 * AP 付款单且词表为 REGISTERED/CONFIRMED），词表中不存在“未付款”文案 → 原 hasText:'未付款' 恒不命中。
 */
test.describe('06 采购付款', () => {
  test.beforeEach(async ({ page, context }) => {
    // V15 Batch 487 P0-T05：注入 auth mock，业务 API 走真实后端（applyAuthMocks 不再 mock 业务 API）
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('06-01 应付单可一次性付清', async ({ page }) => {
    // 应付管理为扁平 Tab 页（router index.ts:173 path:'ap'），默认停「应付发票」tab；
    // 无 /ap/invoice/list 子路由 → 原落 404
    await page.goto('/ap');
    const invoice = page.locator('tr, .el-table__row').filter({ hasText: '已审核' }).first();
    await invoice.getByRole('button', { name: /详情/ }).click();
    await page.getByRole('button', { name: /付款/ }).click();
    // 全额付款
    await page.getByRole('button', { name: /全额付款/ }).click();
    await page.getByRole('button', { name: /确认/ }).click();
    await expect(page.getByText(/已付款|付款成功/)).toBeVisible();
  });

  test('06-02 应付单可多次部分付款', async ({ page }) => {
    // 应付管理为扁平 Tab 页（router index.ts:173 path:'ap'），默认停「应付发票」tab；
    // 无 /ap/invoice/list 子路由 → 原落 404
    await page.goto('/ap');
    const invoice = page.locator('tr, .el-table__row').filter({ hasText: '已审核' }).first();
    await invoice.getByRole('button', { name: /详情/ }).click();
    // 第 1 次付款
    await page.getByRole('button', { name: /付款/ }).click();
    await page.getByLabel(/付款金额/).fill('5000');
    await page.getByRole('button', { name: /确认/ }).click();
    // 第 2 次付款
    await page.getByRole('button', { name: /付款/ }).click();
    await page.getByLabel(/付款金额/).fill('3000');
    await page.getByRole('button', { name: /确认/ }).click();
    await expect(page.getByText(/已付.*8000/)).toBeVisible();
  });

  test('06-03 支持 5 种付款方式', async ({ page }) => {
    // 应付管理为扁平 Tab 页（router index.ts:173 path:'ap'），默认停「应付发票」tab；
    // 无 /ap/invoice/list 子路由 → 原落 404
    await page.goto('/ap');
    const invoice = page.locator('tr, .el-table__row').filter({ hasText: '已审核' }).first();
    await invoice.getByRole('button', { name: /详情/ }).click();
    await page.getByRole('button', { name: /付款/ }).click();
    await page.getByLabel(/付款方式/).click();
    const options = page.getByRole('option');
    await expect(options.filter({ hasText: /银行转账/ })).toBeVisible();
    await expect(options.filter({ hasText: /现金/ })).toBeVisible();
    await expect(options.filter({ hasText: /承兑汇票/ })).toBeVisible();
    await expect(options.filter({ hasText: /支付宝/ })).toBeVisible();
    await expect(options.filter({ hasText: /微信/ })).toBeVisible();
  });

  test('06-04 付款单可打印', async ({ page }) => {
    // 应付管理为扁平 Tab 页（router index.ts:173 path:'ap'），付款单列表位于「付款管理」tab；
    // 无 /ap/payment/list 子路由 → 原落 404。先进 /ap 再切「付款管理」tab 驱动到付款列表。
    await page.goto('/ap');
    await page.getByRole('tab', { name: /付款管理/ }).click();
    const payment = page.locator('tr, .el-table__row').first();
    await payment.getByRole('button', { name: /详情/ }).click();
    await page.getByRole('button', { name: /打印/ }).click();
    await expect(page.getByText(/付款单|付款凭证/)).toBeVisible();
  });
});
