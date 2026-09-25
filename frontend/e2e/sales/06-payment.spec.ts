// P9-3 销售 E2E 套件 — 06 收付款
// 创建时间: 2026-06-17
// 覆盖范围：销售收款全流程（4 用例）

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

/**
 * 测试套件：销售收付款
 *
 * 业务流程：
 * 1. 应收单收款（一次性）
 * 2. 应收单部分收款
 * 3. 收款方式（现金/银行转账/承兑汇票/支付宝/微信）
 * 4. 收款单打印
 */
/**
 * 状态标签事实来源（判责 #4647）：AR 发票列表状态列由
 * src/views/ar/tabs/InvoiceTab.vue:getInvoiceStatusLabel 渲染，后端写入值集
 * （backend/src/services/ar_invoice_service.rs，models/status/general.rs::common/payment）
 * 与 i18n 文案（src/locales/zh-CN.ts arModule.invoice.*）为：
 *   DRAFT→草稿 / APPROVED→已审核 / PARTIAL_PAID→部分收款 / PAID→已收讫 / CANCELLED→已取消。
 * 深链 seed 建的是 APPROVED（未收款）发票，其状态标签真实为“已审核”，实体无 payment_status 列，
 * 词表中不存在“未收款”文案 → 原 hasText:'未收款' 恒不命中。断言改用真实存在的标签。
 */
test.describe('06 销售收付款', () => {
  test.beforeEach(async ({ page, context }) => {
    // V15 Batch 487 P0-T05：注入 auth mock，业务 API 走真实后端（applyAuthMocks 不再 mock 业务 API）
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('06-01 应收单可一次性收清', async ({ page }) => {
    // 应收管理为扁平 Tab 页（router index.ts:179 path:'ar'），默认停「应收发票」tab；
    // 无 /ar/invoice/list 子路由 → 原落 404
    await page.goto('/ar');
    const invoice = page.locator('tr, .el-table__row').filter({ hasText: '已审核' }).first();
    await invoice.getByRole('button', { name: /详情/ }).click();
    await page.getByRole('button', { name: /收款/ }).click();
    // 收款金额 = 应收金额（一键全额）
    await page.getByRole('button', { name: /全额收款/ }).click();
    await page.getByRole('button', { name: /确认/ }).click();
    await expect(page.getByText(/已收款|收款成功/)).toBeVisible();
  });

  test('06-02 应收单可多次部分收款', async ({ page }) => {
    // 应收管理为扁平 Tab 页（router index.ts:179 path:'ar'），默认停「应收发票」tab；
    // 无 /ar/invoice/list 子路由 → 原落 404
    await page.goto('/ar');
    const invoice = page
      .locator('tr, .el-table__row')
      .filter({ hasText: /部分收款|已审核/ })
      .first();
    await invoice.getByRole('button', { name: /详情/ }).click();
    // 第 1 次收款
    await page.getByRole('button', { name: /收款/ }).click();
    await page.getByLabel(/收款金额/).fill('5000');
    await page.getByRole('button', { name: /确认/ }).click();
    // 第 2 次收款
    await page.getByRole('button', { name: /收款/ }).click();
    await page.getByLabel(/收款金额/).fill('3000');
    await page.getByRole('button', { name: /确认/ }).click();
    await expect(page.getByText(/已收.*8000/)).toBeVisible();
  });

  test('06-03 支持 5 种收款方式', async ({ page }) => {
    // 应收管理为扁平 Tab 页（router index.ts:179 path:'ar'），默认停「应收发票」tab；
    // 无 /ar/invoice/list 子路由 → 原落 404
    await page.goto('/ar');
    const invoice = page.locator('tr, .el-table__row').filter({ hasText: '已审核' }).first();
    await invoice.getByRole('button', { name: /详情/ }).click();
    await page.getByRole('button', { name: /收款/ }).click();
    // 验证 5 种方式
    await page.getByLabel(/收款方式/).click();
    const options = page.getByRole('option');
    await expect(options.filter({ hasText: /银行转账/ })).toBeVisible();
    await expect(options.filter({ hasText: /现金/ })).toBeVisible();
    await expect(options.filter({ hasText: /承兑汇票/ })).toBeVisible();
    await expect(options.filter({ hasText: /支付宝/ })).toBeVisible();
    await expect(options.filter({ hasText: /微信/ })).toBeVisible();
  });

  test('06-04 收款单可打印', async ({ page }) => {
    // 应收管理为扁平 Tab 页（router index.ts:179 path:'ar'），收款单列表位于「收款管理」tab；
    // 无 /ar/payment/list 子路由 → 原落 404。先进 /ar 再切「收款管理」tab 驱动到收款列表。
    await page.goto('/ar');
    await page.getByRole('tab', { name: /收款管理/ }).click();
    const payment = page.locator('tr, .el-table__row').first();
    await payment.getByRole('button', { name: /详情/ }).click();
    await page.getByRole('button', { name: /打印/ }).click();
    // 验证打印预览或 PDF 生成
    await expect(page.getByText(/收款单|收款凭证/)).toBeVisible();
  });
});
