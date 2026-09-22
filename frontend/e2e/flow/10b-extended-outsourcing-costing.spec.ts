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

  test('F2-1 委外凭证四类端点可用且类型过滤生效', async ({ page }) => {
    // 原断言 `voucher === null || typeof voucher === 'object'` 是恒真式：
    // 404、500、权限失败、后端忽略 voucher_type 过滤，全都判通过。
    // 现在逐类断言：请求成功（失败由 helper 直接抛出）、响应为 {items} 形状、
    // 且返回的每一行类型都等于所请求类型（后端 outsourcing_handler.rs:350-364 确实按类型过滤）。
    for (const vtype of ['issue', 'fee', 'receipt', 'loss']) {
      const items = await verifyOutsourcingVoucher(page, 1, vtype);
      expect(Array.isArray(items), `${vtype}：凭证列表形状不符`).toBe(true);
      for (const row of items) {
        expect(
          row.voucher_type,
          `${vtype}：返回了非本类型凭证（过滤未生效），实际行=${JSON.stringify(row)}`
        ).toBe(vtype);
      }
    }
  });

  test('F2-2 验证成本归集', async ({ page }) => {
    // GET /production/cost-collections 由 cost_collection_handler::list_collections 处理，
    // 出参是裸数组 Vec<cost_collection::Model>（没有分页信封，page/page_size 不参与）。
    // 原实现读 costs.items 恒为 undefined、expect 又不带匹配器，失败时还改查另一个端点顶包，
    // 等于无论后端返回什么都通过。
    const costs = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      '/production/cost-collections'
    );
    expect(
      Array.isArray(costs),
      `成本归集列表应为数组，实际：${JSON.stringify(costs).slice(0, 200)}`
    ).toBe(true);
    for (const row of costs) {
      expect(Number(row.id), `成本归集行缺少 id：${JSON.stringify(row)}`).toBeGreaterThan(0);
      expect(
        String(row.collection_no ?? ''),
        `成本归集行缺少归集单号：${JSON.stringify(row)}`
      ).not.toBe('');
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
    expect(Array.isArray(analyses.items), `analyses.items 应为后端返回的 items 数组`).toBe(true);
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
