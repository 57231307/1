// 财务管理 E2E 套件 — 04 AP/AR 核销「金额级」回读
//
// 契约：AP/AR 结算闭环若只断状态迁移（PARTIAL→PAID）而无金额校验即属假绿
// （状态词表写死也能过）。本套件按「付款/收款后 GET 发票回读真实金额」断言：
//   - AR 应收：收款后回读 received_amount 递增、unpaid_amount 递减（invoice - received = unpaid 恒等）。
//   - AP 应付：走完整方案B内控链（应付单审核 → 付款申请 → 提交 → 审批 → 建付款 → 填交易流水号 → 确认付款），
//     确认付款后回读 paid_amount 递增、unpaid_amount 递减（amount - paid = unpaid 恒等）。
// 全部基于真实后端 + 数值断言，禁用 verifyEndpointHealthy / >=400 / 仅 toast / toBeTruthy。
//   - 04-03 AR 列表分页真值：GET /ar/invoices 为 PaginatedResponse{items,total,page,page_size}，
//     seed 6 条 > page_size=3，断跨页满页、total=全量、两页 id 不相交且日期并集不重不漏。
import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall, apiCallRaw, genCode, tryCleanup } from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/**
 * 读取发票的一个金额字段：键缺失/非数字直接抛错，不 `?? 0` 兜底掩盖缺键（假绿来源）。
 * rust_decimal 默认序列化为字符串（"400.00"），故 Number() 解析。
 */
function readAmount(obj: Record<string, unknown>, key: string, label: string): number {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺少后端真实键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n)) {
    throw new Error(`${label}：键 "${key}" 不可解析为有限数字，raw=${JSON.stringify(obj[key])}`);
  }
  return n;
}

/** 断言两个金额相等（容差 0.01，货币精度），失败信息打印实际值。 */
function expectMoney(actual: number, expected: number, label: string): void {
  expect(Math.abs(actual - expected), `${label}：期望=${expected} 实际=${actual}`).toBeLessThan(
    0.01
  );
}

async function seedSupplier(page: import('@playwright/test').Page): Promise<number> {
  const res = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
    supplier_name: `E2E核销供${Date.now().toString().slice(-8)}`,
    supplier_short_name: 'E2E核',
    contact_phone: '13800001111',
  });
  if (!res.data?.id) throw new Error(`建供应商失败：${JSON.stringify(res)}`);
  CLEANUP.push({ path: `/purchase/suppliers/${res.data.id}`, label: 'supplier' });
  return res.data.id;
}

async function seedCustomer(page: import('@playwright/test').Page): Promise<number> {
  const res = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
    customer_name: `E2E核销客${Date.now().toString().slice(-8)}`,
  });
  if (!res.data?.id) throw new Error(`建客户失败：${JSON.stringify(res)}`);
  CLEANUP.push({ path: `/crm/customers/${res.data.id}`, label: 'customer' });
  return res.data.id;
}

test.describe('04 AP/AR 核销金额级回读', () => {
  test.beforeEach(async ({ page }) => {
    // 真实 admin 会话（apiCall 携带 CSRF 一次性消费）。
    await loginViaUI(page);
    // 保证当期会计期间开放：AR 收款与 AP 付款确认生成的付款凭证过账都要 check_date_locked。
    await apiCall(page, 'POST', '/finance/accounting-periods/init', {}).catch(e =>
      console.warn('[04] 会计期间初始化失败:', (e as Error).message)
    );
  });

  test('04-01 AR 应收：收款后回读 received_amount/unpaid_amount 数值变化（部分→结清）', async ({
    page,
  }) => {
    const customerId = await seedCustomer(page);
    const today = new Date().toISOString().slice(0, 10);

    // 建应收单：invoice_amount=1000，后端默认 received_amount=0、unpaid_amount=invoice_amount。
    const inv = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/invoices', {
      customer_id: customerId,
      invoice_amount: 1000,
      invoice_date: today,
      due_date: today,
    });
    const invoiceId = Number(inv.id);
    if (!invoiceId) throw new Error(`建应收单失败：${JSON.stringify(inv)}`);
    CLEANUP.push({ path: `/ar/invoices/${invoiceId}`, label: 'ar_invoice' });

    expectMoney(readAmount(inv, 'received_amount', '建单后'), 0, '建单 received_amount');
    expectMoney(readAmount(inv, 'unpaid_amount', '建单后'), 1000, '建单 unpaid_amount');

    // 第一次收款 300（关联该应收单）→ 后端 link_invoices 即时累加 received、扣减 unpaid。
    await apiCall(page, 'POST', '/ar/payments', {
      customer_id: customerId,
      amount: 300,
      payment_method: '银行转账',
      payment_date: today,
      invoice_ids: [invoiceId],
    });
    const after300 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/invoices/${invoiceId}`
    );
    expectMoney(readAmount(after300, 'received_amount', '收款300后'), 300, 'received_amount 增量');
    expectMoney(readAmount(after300, 'unpaid_amount', '收款300后'), 700, 'unpaid_amount 余额');
    // 会计恒等：invoice_amount - received_amount = unpaid_amount
    expectMoney(
      readAmount(after300, 'invoice_amount', '应收原额') -
        readAmount(after300, 'received_amount', '已收'),
      readAmount(after300, 'unpaid_amount', '未收'),
      'AR 恒等：invoice-received=unpaid'
    );

    // 第二次收款 700 → 结清，received=1000、unpaid=0、状态 PAID。
    await apiCall(page, 'POST', '/ar/payments', {
      customer_id: customerId,
      amount: 700,
      payment_method: '银行转账',
      payment_date: today,
      invoice_ids: [invoiceId],
    });
    const after1000 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/invoices/${invoiceId}`
    );
    expectMoney(
      readAmount(after1000, 'received_amount', '收款1000后'),
      1000,
      'received_amount 全额'
    );
    expectMoney(readAmount(after1000, 'unpaid_amount', '收款1000后'), 0, 'unpaid_amount 归零');
    expect(String(after1000.status), '结清后状态应为 PAID').toBe('PAID');
  });

  test('04-02 AP 应付：付款确认后回读 paid_amount/unpaid_amount 数值变化（部分付款）', async ({
    page,
  }) => {
    const supplierId = await seedSupplier(page);
    const today = new Date().toISOString().slice(0, 10);

    // 建应付单：amount=1000 → 默认 paid=0、unpaid=1000、status=DRAFT。
    const inv = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/invoices', {
      supplier_id: supplierId,
      invoice_type: 'PURCHASE',
      amount: 1000,
      invoice_date: today,
      due_date: today,
    });
    const invoiceId = Number(inv.id);
    if (!invoiceId) throw new Error(`建应付单失败：${JSON.stringify(inv)}`);
    CLEANUP.push({ path: `/ap/invoices/${invoiceId}`, label: 'ap_invoice' });
    expectMoney(readAmount(inv, 'paid_amount', '建单后'), 0, '建单 paid_amount');
    expectMoney(readAmount(inv, 'unpaid_amount', '建单后'), 1000, '建单 unpaid_amount');

    // 应付单须先审核（DRAFT→AUDITED）方能被付款申请引用（validate_invoice_items_txn 拒绝 DRAFT）。
    await apiCall(page, 'POST', `/ap/invoices/${invoiceId}/approve`);

    // 付款申请：引用该应付单，申请额 400（< unpaid 1000，合法）。
    const req = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payment-requests', {
      supplier_id: supplierId,
      request_date: today,
      payment_type: 'PROGRESS',
      payment_method: 'TT',
      request_amount: 400,
      currency: 'CNY',
      items: [{ invoice_id: invoiceId, apply_amount: 400, notes: 'E2E 部分付款' }],
    });
    const requestId = Number(req.id);
    if (!requestId) throw new Error(`建付款申请失败：${JSON.stringify(req)}`);

    // 方案B内控链：提交 → 审批（admin，400<10万可审）。
    await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/submit`);
    const approved = await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/approve`);
    expect(
      String((approved.data as Record<string, unknown>).approval_status),
      '审批后应为 APPROVED'
    ).toBe('APPROVED');

    // 从已审批申请创建付款单（金额由申请派生，非手工录入）。
    const pay = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payments', {
      request_id: requestId,
      payment_date: today,
    });
    const paymentId = Number(pay.id);
    if (!paymentId) throw new Error(`建付款单失败：${JSON.stringify(pay)}`);

    // 确认付款要求交易流水号非空（资金内控），先更新再确认。
    await apiCall(page, 'PUT', `/ap/payments/${paymentId}`, {
      transaction_no: genCode('E2E-TXN'),
    });
    await apiCall(page, 'POST', `/ap/payments/${paymentId}/confirm`);

    // 回读应付单：paid=400、unpaid=600、状态 PARTIAL_PAID；恒等 amount-paid=unpaid。
    const after = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/invoices/${invoiceId}`
    );
    expectMoney(readAmount(after, 'paid_amount', '付款确认后'), 400, 'paid_amount');
    expectMoney(readAmount(after, 'unpaid_amount', '付款确认后'), 600, 'unpaid_amount');
    expectMoney(
      readAmount(after, 'amount', '应付原额') - readAmount(after, 'paid_amount', '已付'),
      readAmount(after, 'unpaid_amount', '未付'),
      'AP 恒等：amount-paid=unpaid'
    );
    expect(String(after.invoice_status), '部分付款后状态应为 PARTIAL_PAID').toBe('PARTIAL_PAID');
  });

  test('04-03 AR 列表分页真值：PaginatedResponse{items,total,page,page_size}，跨页不重不漏', async ({
    page,
  }) => {
    // 契约：GET /ar/invoices 返回标准 PaginatedResponse（ar_invoice_handler.rs:61-88
    // → ApiResponse::success_paginated，utils/response.rs:95-113：data={items,total,page,page_size}）。
    // 专属新客户的行数完全可控（customer_id 过滤 = 真值 total），
    // 且每单 invoice_date 互不相同 —— 服务层唯一排序是 invoice_date desc
    //（ar_invoice_service.rs:263-268），同值排序在 DB 层不保证稳定，异值才谈得上"不重不漏"。
    const customerId = await seedCustomer(page);
    const dates: string[] = [];
    for (let i = 0; i < 6; i++) {
      const day = new Date(Date.now() - i * 86_400_000).toISOString().slice(0, 10);
      dates.push(day);
      const inv = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/invoices', {
        customer_id: customerId,
        invoice_amount: 100 + i,
        invoice_date: day,
        due_date: day,
      });
      const id = Number(inv.id);
      if (!id) throw new Error(`建应收单失败：${JSON.stringify(inv)}`);
      CLEANUP.push({ path: `/ar/invoices/${id}`, label: 'ar_invoice' });
    }

    interface ArInvoicePage {
      items: Array<Record<string, unknown>>;
      total: number;
      page: number;
      page_size: number;
    }
    const qs = `customer_id=${customerId}&page_size=3`;
    const p1 = await apiCallRaw<ArInvoicePage>(page, 'GET', `/ar/invoices?page=1&${qs}`);
    const p2 = await apiCallRaw<ArInvoicePage>(page, 'GET', `/ar/invoices?page=2&${qs}`);

    // PaginatedResponse 四键齐全（缺任一键即假绿温床，直接抛错而非 undefined 通过）。
    for (const [label, body] of [
      ['第1页', p1],
      ['第2页', p2],
    ] as Array<[string, ArInvoicePage]>) {
      for (const k of ['items', 'total', 'page', 'page_size']) {
        if (!Object.prototype.hasOwnProperty.call(body, k)) {
          throw new Error(`${label}响应缺少分页键 "${k}"，实际键=${Object.keys(body).join(',')}`);
        }
      }
    }

    expect(p1.items.length, '第1页应满页 3 条').toBe(3);
    expect(p2.items.length, `第2页应满页 3 条（共 6 条 = 2 整页）实际=${p2.items.length}`).toBe(3);
    expect(p1.total, 'total 应等于全量 6（客户过滤真值）').toBe(6);
    expect(p2.total, 'total 跨页恒定等于全量 6').toBe(6);
    expect(p1.page, 'page 回显请求页码 1').toBe(1);
    expect(p2.page, 'page 回显请求页码 2').toBe(2);
    expect(p1.page_size, 'page_size 回显 3').toBe(3);
    expect(p2.page_size, 'page_size 回显 3').toBe(3);

    const ids1 = p1.items.map(o => Number(o.id));
    const ids2 = p2.items.map(o => Number(o.id));
    expect(ids1.every(Number.isFinite) && ids2.every(Number.isFinite), '条目 id 均为数字').toBe(
      true
    );
    const overlap = ids1.filter(id => ids2.includes(id));
    expect(overlap, `第2页与第1页 id 必不相交，实际交集=${JSON.stringify(overlap)}`).toEqual([]);
    // 并集恰好覆盖 6 个不同 invoice_date → 不重之外还要求不漏。
    const allDates = [...ids1, ...ids2].length;
    expect(allDates, '两页合计行数 = 全量 6').toBe(6);
    const d1 = p1.items.map(o => String(o.invoice_date));
    const d2 = p2.items.map(o => String(o.invoice_date));
    expect(
      new Set([...d1, ...d2]).size,
      '两页并集 invoice_date 互不相同（覆盖全部 6 个种子日）'
    ).toBe(6);
    // 排序真值：invoice_date desc → 第 1 页每行日期不早于第 2 页任一行。
    const minP1 = d1.slice().sort().at(0)!;
    const maxP2 = d2.slice().sort().at(-1)!;
    expect(
      minP1 >= maxP2,
      `应按 invoice_date 降序：P1 最旧=${minP1} 不应早于 P2 最新=${maxP2}`
    ).toBe(true);
  });
});
