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
    // 原断言 `voucher === null || typeof voucher === 'object'` 是恒真式（404/500/权限失败/忽略过滤全判绿）。
    // helpers.verifyOutsourcingVoucher 打的是 `/outsourcing-vouchers`，但委外凭证路由在
    // routes/production.rs:425 注册、经 routes/mod.rs:510 `nest("/api/v1/erp/production", ...)` 挂载，
    // 真实路径必须带 production 前缀——run 35887709282 分片 flow(5/20) 里该 helper 返回 404
    // （"GET /outsourcing-vouchers... returned non-JSON (status 404)"）。helpers 不在本次可改范围，
    // 故在本用例内直连正确端点，并逐类断言：data 为 PaginatedResponse 的 items 数组，
    // 且返回每一行类型都等于所请求类型（outsourcing_ops/voucher.rs:158 确实按 voucher_type 过滤）。
    for (const vtype of ['issue', 'fee', 'receipt', 'loss']) {
      const resp = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
        page,
        'GET',
        `/production/outsourcing-vouchers?outsourcing_order_id=1&voucher_type=${vtype}&page=1&page_size=5`
      );
      expect(
        Array.isArray(resp?.items),
        `${vtype}：凭证列表 data 应为分页信封的 items 数组，实际：${JSON.stringify(resp).slice(0, 200)}`
      ).toBe(true);
      for (const row of resp.items) {
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
    // helpers.verifyTrialBalance 读的是 `/finance/reports/trial-balance` 响应里根本不存在的
    // debit_total/credit_total 键（真实 TrialBalance DTO 只有 total_initial/period/ending_debit/credit
    // 与 entries/period，见 models/dto/finance_report_dto.rs:78-87），再用 `|| 0` 兜底成 0，
    // 于是 balanced 恒 true、debit_total/credit_total 恒为 number 0 —— F2-3 旧断言
    // `typeof result.debit_total === 'number'` 因此恒真，是真·假绿。
    // 这里绕开该 helper，直连端点并按后端真实键断言：entries 为数组、期末借贷合计两键存在且可解析为数字
    // （rust_decimal serde 输出为字符串），并校验试算平衡的核心不变量：期末借方合计 == 期末贷方合计。
    const tb = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      '/finance/reports/trial-balance'
    );
    expect(
      Array.isArray(tb?.entries),
      `试算平衡表应返回 entries 数组，实际：${JSON.stringify(tb).slice(0, 200)}`
    ).toBe(true);
    expect(
      Object.prototype.hasOwnProperty.call(tb ?? {}, 'total_ending_debit'),
      `响应缺少后端真实键 total_ending_debit（旧用例误读 debit_total 而恒绿）：${JSON.stringify(
        tb
      ).slice(0, 200)}`
    ).toBe(true);
    expect(
      Object.prototype.hasOwnProperty.call(tb ?? {}, 'total_ending_credit'),
      `响应缺少后端真实键 total_ending_credit：${JSON.stringify(tb).slice(0, 200)}`
    ).toBe(true);
    const endingDebit = Number(tb.total_ending_debit);
    const endingCredit = Number(tb.total_ending_credit);
    expect(Number.isFinite(endingDebit), `期末借方合计非数字：${tb.total_ending_debit}`).toBe(true);
    expect(Number.isFinite(endingCredit), `期末贷方合计非数字：${tb.total_ending_credit}`).toBe(
      true
    );
    expect(
      Math.abs(endingDebit - endingCredit) < 0.01,
      `试算不平衡：期末借方合计(${endingDebit}) ≠ 期末贷方合计(${endingCredit})`
    ).toBe(true);
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
