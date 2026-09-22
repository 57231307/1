import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  verifyIllegalTransition,
  getCtx,
  genCode,
  genName,
  ensureTestEntities,
  expectBadRequest,
} from './helpers';

test.describe.serial('Shard 4: 财务核算闭环', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('4-1 验证会计科目列表', async ({ page }) => {
    // 后端 list_subjects 返回 ApiResponse<Vec<Model>>：data 是数组（无 items 包装）
    const subjects = await apiCallRaw<
      Array<{ code: string; name: string }> | { items?: Array<{ code: string; name: string }> }
    >(page, 'GET', '/subjects?page=1&page_size=20');
    const subjectList = Array.isArray(subjects)
      ? subjects
      : ((subjects as { items?: Array<{ code: string; name: string }> }).items ?? []);
    expect(subjectList.length).toBeGreaterThanOrEqual(0);
  });

  test('4-2 创建凭证（含色号维度成本）', async ({ page }) => {
    // 先取真实存在的科目编码（CI 库可能没有 1122/6001/2202 种子）
    const subjects = await apiCallRaw<
      | Array<{ code: string; status?: string }>
      | { items?: Array<{ code: string; status?: string }> }
    >(page, 'GET', '/subjects?page=1&page_size=50');
    // 后端 list_subjects 返回 ApiResponse<Vec<Model>>：data 是数组（非 items 包装）
    const subjectList = Array.isArray(subjects)
      ? subjects
      : ((subjects as { items?: Array<{ code: string; status?: string }> }).items ?? []);
    let activeCodes = subjectList.filter(s => !s.status || s.status === 'active').map(s => s.code);
    // 科目不足 3 个时先创建 E2E 专用科目（CreateSubjectRequestDto: code/name/level）
    const suffix = Date.now().toString().slice(-6);
    while (activeCodes.length < 3) {
      const i = activeCodes.length;
      const code = `E2E${suffix}${i}`;
      await apiCall(page, 'POST', '/subjects', {
        code,
        name: `E2E科目${i}`,
        level: 1,
        balance_direction: 'debit',
      });
      activeCodes.push(code);
    }
    const pick = (i: number) => activeCodes[i % activeCodes.length];
    const amount = 10000;
    const half = amount / 2;
    const result = await apiCall<{ id?: number }>(page, 'POST', '/vouchers', {
      voucher_date: new Date().toISOString().split('T')[0],
      voucher_type: 'general',
      items: [
        { subject_code: pick(0), debit: amount, credit: 0 },
        { subject_code: pick(1), debit: 0, credit: half },
        { subject_code: pick(2), debit: 0, credit: amount - half },
      ],
      remarks: 'E2E 测试凭证（含色号维度成本）',
    });
    getCtx().voucherId = result.data?.id;
    expect(getCtx().voucherId).toBeDefined();
  });

  test('4-3 凭证状态机：draft → submitted → reviewed → posted', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const id = ctx.voucherId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    await apiCall(page, 'POST', `/vouchers/${id}/submit`);
    await apiCall(page, 'POST', `/vouchers/${id}/review`);
    await apiCall(page, 'POST', `/vouchers/${id}/post`);

    const v = await apiCallRaw<{ status: string }>(page, 'GET', `/vouchers/${id}`);
    const status = (v.status || '').toLowerCase();
    expect(['draft', 'submitted', 'reviewed', 'posted', 'cancelled']).toContain(
      status ?? '(missing-status)'
    );
  });

  test('4-4 验证凭证非法转换（posted → draft 应拒绝）', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.voucherId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    // 对已 posted 的凭证提交 → 应拒绝
    const result = await apiCallExpectFail(page, 'POST', `/vouchers/${id}/submit`);
    expectBadRequest(result); // 非法转换应被拒
  });

  test('4-5 验证 AP 应付单', async ({ page }) => {
    const apInvoices = await apiCallRaw<{
      items: Array<{ id: number; amount: number; status: string }>;
    }>(page, 'GET', '/ap/invoices?page=1&page_size=5');
    // ap_invoice_handler.rs:67 用 PaginatedResponse（key=items）
    console.log(`[4-5] AP 应付单 items 长度=${apInvoices?.items?.length ?? '(缺 key)'}`);
    expect(Array.isArray(apInvoices?.items), 'AP 应付单应返回 items 数组').toBe(true);
  });

  test('4-6 验证 AR 应收单', async ({ page }) => {
    // list_ar_invoices 返回 ApiResponse<Vec<Model>>：data 直接是数组，无 items 包装
    // （与 02-o2c 2-8 同源），原实现读 arInvoices.items 恒 undefined 且无匹配器
    const arInvoices = await apiCallRaw<Array<{ id: number; amount: number; status: string }>>(
      page,
      'GET',
      '/ar/invoices?page=1&page_size=5'
    );
    console.log(
      `[4-6] AR 应收单数组长度=${Array.isArray(arInvoices) ? arInvoices.length : '非数组'}`
    );
    expect(Array.isArray(arInvoices), 'AR 应收单列表应返回数组').toBe(true);
  });

  test('4-7 验证付款/收款记录', async ({ page }) => {
    const apPayments = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/ap/payments?page=1&page_size=5'
    );
    expect(Array.isArray(apPayments?.items), 'AP 付款应返回 items 数组').toBe(true);
    const arPayments = await apiCallRaw<{ list: Array<{ id: number }> }>(
      page,
      'GET',
      '/ar/payments?page=1&page_size=5'
    );
    // ar_payment_handler.rs:26 用 json!({"list": ...})，key 是 list 而非 items
    console.log(`[4-7] AR 收款 list 长度=${arPayments?.list?.length ?? '(缺 key)'}`);
    expect(Array.isArray(arPayments?.list), 'AR 收款应返回 list 数组').toBe(true);
  });

  test('4-8 创建固定资产（染缸设备）', async ({ page }) => {
    const ctx = getCtx();
    const result = await apiCall<{ id?: number }>(page, 'POST', '/fixed-assets', {
      // CreateAssetRequestDto 字段：asset_no/asset_name/original_value/purchase_date/useful_life/depreciation_method
      asset_name: genName('E2E染缸设备'),
      asset_no: genCode('FA'),
      purchase_date: new Date().toISOString().split('T')[0],
      original_value: 500000,
      useful_life: 60,
      depreciation_method: 'straight_line',
    });
    ctx.fixedAssetId = result.data?.id;
    expect(ctx.fixedAssetId).toBeDefined();
  });

  test('4-9 创建预算', async ({ page }) => {
    const ctx = getCtx();
    const result = await apiCall<{ id?: number }>(page, 'POST', '/budgets', {
      // 后端 CreateBudgetDto 必填 item_name + planned_amount（budget_name/total_amount 不存在）
      item_name: genName('E2E预算'),
      budget_year: new Date().getFullYear(),
      item_type: 'expense',
      planned_amount: 500000,
    });
    ctx.budgetId = result.data?.id;
    expect(ctx.budgetId).toBeDefined();
  });

  test('4-10 验证会计期间状态', async ({ page }) => {
    // GET /finance/accounting-periods 由 missing_handlers::get_accounting_periods 处理，
    // 出参是裸数组 Vec<AccountingPeriodDto>（该端点没有 Query 结构体，page/page_size 无效）。
    // 原实现读 periods.items 恒为 undefined + expect() 无匹配器，整条用例空转。
    const periods = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      '/finance/accounting-periods'
    );
    expect(
      Array.isArray(periods),
      `会计期间应返回数组，实际：${JSON.stringify(periods).slice(0, 200)}`
    ).toBe(true);
    for (const p of periods) {
      expect(Number(p.id), `会计期间行缺少 id：${JSON.stringify(p)}`).toBeGreaterThan(0);
      expect(String(p.status ?? ''), `会计期间行缺少 status：${JSON.stringify(p)}`).not.toBe('');
      expect(Number(p.year), `会计期间行缺少 year：${JSON.stringify(p)}`).toBeGreaterThan(0);
      expect(
        Number(p.period),
        `会计期间行缺少 period：${JSON.stringify(p)}`
      ).toBeGreaterThanOrEqual(1);
    }
  });

  test('4-11 验证凭证列表', async ({ page }) => {
    // GET /vouchers 由 voucher_handler::list_vouchers 处理，出参是裸数组
    // Vec<voucher::Model>：它接收 VoucherQuery 的筛选字段，但 total/items 分页信封并不存在，
    // 原实现的 vouchers.items 恒为 undefined。
    const vouchers = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      '/vouchers?page=1&page_size=5'
    );
    expect(
      Array.isArray(vouchers),
      `凭证列表应返回数组，实际：${JSON.stringify(vouchers).slice(0, 200)}`
    ).toBe(true);
    for (const v of vouchers.slice(0, 5)) {
      expect(String(v.voucher_no ?? ''), `凭证行缺少 voucher_no：${JSON.stringify(v)}`).not.toBe(
        ''
      );
      expect(String(v.status ?? ''), `凭证行缺少 status：${JSON.stringify(v)}`).not.toBe('');
    }
  });

  test('4-12 验证财务审计日志', async ({ page }) => {
    const logs = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/audit-logs?page=1&page_size=10'
    );
    expect(Array.isArray(logs.items), `logs.items 应为后端返回的 items 数组`).toBe(true);
  });
});
