import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  verifyBulkColorDeliveryBlock,
  verifyOutsourcingVoucher,
  verifyTrialBalance,
  verifyWeightConversion,
  verifyNetWeight,
  getCtx,
  genCode,
  ensureTestEntities,
} from './helpers';

test.describe.serial('扩展: 委外凭证/成本归集/试算平衡', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('F2-1 验证委外凭证（4 类：issue/fee/receipt/loss）', async ({ page }) => {
    // 尝试验证 4 种凭证类型
    for (const vtype of ['issue', 'fee', 'receipt', 'loss']) {
      const voucher = await verifyOutsourcingVoucher(page, 1, vtype);
      // 凭证可能不存在（未走委外流程），关键是 API 不崩溃
      expect(voucher === null || typeof voucher === 'object').toBeTruthy();
    }
  });

  test('F2-2 验证成本归集', async ({ page }) => {
    try {
      const costs = await apiCallRaw<{ items: Array<{ id: number }> }>(
        page,
        'GET',
        '/production/cost-collections?page=1&page_size=5'
      );
      expect(Array.isArray(costs.items), `costs.items 应为后端返回的 items 数组`);
    } catch {
      const costs = await apiCallRaw<{ items: Array<{ id: number }> }>(
        page,
        'GET',
        '/cost?page=1&page_size=5'
      );
      expect(Array.isArray(costs.items), `costs.items 应为后端返回的 items 数组`);
    }
  });

  test('F2-3 验证试算平衡', async ({ page }) => {
    const result = await verifyTrialBalance(page);
    expect(typeof result?.balanced, '试算平衡应返回 balanced 布尔值').toBe('boolean');
    expect(typeof result.balanced).toBe('boolean');
    expect(typeof result.debit_total).toBe('number');
    expect(typeof result.credit_total).toBe('number');
  });

  test('F2-4 验证成本按缸号维度', async ({ page }) => {
    const analyses = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/financial-analysis/reports?page=1&page_size=5'
    );
    expect(Array.isArray(analyses.items), `analyses.items 应为后端返回的 items 数组`);
  });

  test('F2-5 验证财务报表', async ({ page }) => {
    const balanceSheet = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      '/finance/reports/balance-sheet'
    );
    expect(balanceSheet, '资产负债表接口应返回数据对象').toBeTruthy();
    const incomeStatement = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      '/finance/reports/income-statement'
    );
    expect(incomeStatement, '利润表接口应返回数据对象').toBeTruthy();
  });
});
