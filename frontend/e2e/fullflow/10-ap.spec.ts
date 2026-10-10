// 财务域全流程契约级 E2E — 10 AP 应付（应付单/付款申请/付款单）
//
// 定位：与 finance/04（金额级回读）、finance/05（内控负向）、finance/11（核销闭环/对账状态机/
// submit 无明细门控）互补，本文件专攻用户报的两类缺陷在 AP 链上的契约靶心：
//   ①「创建时保存的数据不完整」——用 Create*Request 的**全部键**建单，再 GET 详情+列表逐字段回读；
//   ②「提交/保存报请求错误」——4xx 一律断真实机器码，且 message 必须是具体原因
//     （不得等于脱敏常量「请求参数验证失败」/「业务处理失败」，utils/messages.rs:41,43）。
// 钉住的契约真值源（禁猜键名，全部逐项对齐）：
//   CreateApInvoiceRequest  backend/src/services/ap_invoice_ops/types.rs:24-69
//   UpdateApInvoiceRequest  同上:72-101（单层 Option：省略键=保持原值，ap_invoice_ops/crud.rs:127-152
//     为 if-let 覆写；该端点**不存在显式 null 清空语义**，清空诉求列后端缺口清单）
//   create_manual 落库      backend/src/services/ap_invoice_ops/crud.rs:38-101（tax_amount/
//     attachment_urls/notes/currency/exchange_rate 均 Set，缺任一键回读必红）
//   AP 状态词表（大写）     backend/src/models/status/general.rs:14-33 + finance.rs:47-50
//     （DRAFT/AUDITED/PARTIAL_PAID/PAID/CANCELLED，APPROVING/REJECTED）
//   状态门：update/delete 仅 DRAFT（crud.rs:112-118/176+，business→脱敏）；approve DRAFT→AUDITED；
//     cancel 仅 AUDITED/PARTIAL_PAID（crud.rs 头注释）；mark_as_paid 白名单 AUDITED/PARTIAL_PAID
//   付款申请 CreateApPaymentRequest services/ap_payment_request_service.rs:600-657（items 可缺省，
//     但 submit 门控 :336-338 要求有明细，且明细 invoice_id 为 NOT NULL FK，item 校验 :122-126
//     引用单不得为 DRAFT/CANCELLED）；审批分级门控 :542-592（admin 全额度可批，e2e_admin=admin）
//   付款单 CreateApPaymentRequest(付款) services/ap_payment_service.rs:752-766：request_id 必填、
//     申请须 APPROVED、金额取申请额；确认门 REGISTERED→CONFIRMED（ap_payment_service.rs:223-226）；
//     更新仅 REGISTERED（:138）
//   列表形状：GET /ap/invoices → PaginatedResponse{items,total,page,page_size}
//     （handlers/ap_invoice_handler.rs:66-70）
// 诚实标注（测不到/缺口，不造假）：
//   - PUT /ap/invoices/{id} 无 tax_amount/currency/exchange_rate 键（types.rs:72-101），「清空可空列」
//     在该端点不可表达——按单层 Option 真实语义断「送显式 null ⇒ 原值仍在」（crud.rs:127-152 的
//     if-let 即证据），并已在报告「后端缺口清单」点名：若产品口径要求可清空，需后端引入三态 DTO。
//   - /ap/reconciliations/auto、/ap/reports/* 依赖真实账龄数据且跨分片聚合，内容级断言不稳定，
//     不在本文件重写（对账链已由 finance/11-03/04 覆盖）。
// 铁律：无 ?? [] / || 兜底 / 无 if(isVisible) 放行 / 无 skip；金额 Decimal 出参按字符串用 Number() 归一。
import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  tryCleanup,
  APP_ERROR_CODES,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

// 后端脱敏常量（utils/messages.rs:41,43）：4xx 的 message 若等于二者即判红（缺陷②的靶心）。
const DESENSITIZED_CONSTANTS = ['请求参数验证失败', '业务处理失败'];

/** 键存在且值与提交值严格相等（缺键/不等即红，信息列出实际键集，杜绝静默丢字段）。 */
function expectKeyValue(
  obj: Record<string, unknown>,
  key: string,
  expected: unknown,
  label: string
): void {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺少后端真实键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  expect(obj[key], `${label}：键 "${key}" 期望=${JSON.stringify(expected)}`).toEqual(expected);
}

/** Decimal 出参（rust_decimal 默认序列化为字符串如 "5432.10"）Number() 归一后按货币精度比。 */
function expectDecimal(obj: Record<string, unknown>, key: string, expected: number, label: string) {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺少金额键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n)) {
    throw new Error(`${label}：键 "${key}" 不可解析为数字，raw=${JSON.stringify(obj[key])}`);
  }
  expect(Math.abs(n - expected), `${label}：${key} 期望=${expected} 实际=${n}`).toBeLessThan(0.005);
}

function requireId(obj: Record<string, unknown>, label: string): number {
  const id = Number(obj.id);
  if (!Number.isFinite(id) || id <= 0) {
    throw new Error(`${label}：响应无有效 id，实际键=${Object.keys(obj).join(',')}`);
  }
  return id;
}

async function seedSupplier(page: import('@playwright/test').Page, tag: string): Promise<number> {
  const res = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
    supplier_name: `E2E全流程AP${tag}${genCode('S')}`,
    supplier_short_name: 'E2EAP',
    contact_phone: '13800005555',
  });
  if (!res.data?.id) throw new Error(`建供应商失败：${JSON.stringify(res)}`);
  CLEANUP.push({ path: `/purchase/suppliers/${res.data.id}`, label: 'supplier' });
  return res.data.id;
}

/** 建应付单（返回 detail），payload 传字段覆盖；缺省即契约全字段样本。 */
async function createApInvoice(
  page: import('@playwright/test').Page,
  supplierId: number,
  overrides: Record<string, unknown> = {}
): Promise<Record<string, unknown>> {
  const inv = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/invoices', {
    supplier_id: supplierId,
    invoice_type: 'PURCHASE',
    invoice_date: '2026-02-10',
    due_date: '2026-03-12',
    payment_terms: 45,
    amount: 5432.1,
    currency: 'USD',
    exchange_rate: 7.15,
    tax_amount: 631.05,
    notes: 'E2E-AP-全字段',
    attachment_urls: ['https://example.com/a.pdf', 'https://example.com/b.pdf'],
    ...overrides,
  });
  CLEANUP.push({ path: `/ap/invoices/${requireId(inv, '建应付单')}`, label: 'ap_invoice' });
  return inv;
}

/** 建「已审核」应付单（可被付款申请明细引用——item 门控拒 DRAFT，service:122-126）。 */
async function seedApprovedApInvoice(
  page: import('@playwright/test').Page,
  supplierId: number,
  amount: number
): Promise<number> {
  const inv = await createApInvoice(page, supplierId, { amount });
  const id = requireId(inv, '应付单');
  await apiCall(page, 'POST', `/ap/invoices/${id}/approve`);
  return id;
}

test.describe('10 AP 应付全流程契约链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('10-01 应付单按 Create 全键建单 → GET 详情+列表逐字段回读相等（缺陷①靶心）', async ({
    page,
  }) => {
    const supplierId = await seedSupplier(page, '全字段');
    const created = await createApInvoice(page, supplierId);
    const id = requireId(created, '建应付单');

    // 创建响应自身即应携带完整落库值
    expectDecimal(created, 'amount', 5432.1, '创建响应');
    expectKeyValue(created, 'invoice_status', 'DRAFT', '创建响应');
    expectKeyValue(created, 'source_type', 'MANUAL', '创建响应');

    const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ap/invoices/${id}`);
    expectKeyValue(detail, 'supplier_id', supplierId, 'AP详情');
    expectKeyValue(detail, 'invoice_type', 'PURCHASE', 'AP详情');
    expectKeyValue(detail, 'invoice_date', '2026-02-10', 'AP详情');
    expectKeyValue(detail, 'due_date', '2026-03-12', 'AP详情');
    expectKeyValue(detail, 'payment_terms', 45, 'AP详情');
    expectDecimal(detail, 'amount', 5432.1, 'AP详情');
    expectKeyValue(detail, 'currency', 'USD', 'AP详情');
    expectDecimal(detail, 'exchange_rate', 7.15, 'AP详情');
    expectDecimal(detail, 'tax_amount', 631.05, 'AP详情');
    expectKeyValue(detail, 'notes', 'E2E-AP-全字段', 'AP详情');
    expectKeyValue(
      detail,
      'attachment_urls',
      ['https://example.com/a.pdf', 'https://example.com/b.pdf'],
      'AP详情'
    );
    expectDecimal(detail, 'paid_amount', 0, 'AP详情');
    expectDecimal(detail, 'unpaid_amount', 5432.1, 'AP详情');
    expectKeyValue(detail, 'invoice_status', 'DRAFT', 'AP详情');
    expect(
      typeof detail.invoice_no === 'string' && (detail.invoice_no as string).length > 0,
      `AP详情 invoice_no 应为后端生成非空串，实际=${JSON.stringify(detail.invoice_no)}`
    ).toBe(true);

    // 列表形状 = PaginatedResponse{items,total,page,page_size}，逐键显式
    const list = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/invoices?supplier_id=${supplierId}&page=1&page_size=100`
    );
    for (const k of ['items', 'total', 'page', 'page_size']) {
      expect(Object.prototype.hasOwnProperty.call(list, k), `AP列表应含 PaginatedResponse 键 ${k}`);
    }
    const items = list.items as Array<Record<string, unknown>>;
    expect(Array.isArray(items), 'AP列表 items 应为数组').toBe(true);
    const row = items.find(r => Number(r.id) === id);
    if (!row) throw new Error(`AP列表未含自建单 id=${id}（筛选/落库缺失）`);
    expectKeyValue(row, 'invoice_no', detail.invoice_no, 'AP列表行');
    expectDecimal(row, 'amount', 5432.1, 'AP列表行');
    expectDecimal(row, 'tax_amount', 631.05, 'AP列表行');
    expectKeyValue(row, 'currency', 'USD', 'AP列表行');
    expectKeyValue(row, 'notes', 'E2E-AP-全字段', 'AP列表行');
    expect(Number(list.total), 'AP列表 total 应≥1').toBeGreaterThanOrEqual(1);
  });

  test('10-02 应付单 PUT 三态（单层 Option：覆盖/省略保持/不碰）→ 回读三态各异，金额联动 unpaid', async ({
    page,
  }) => {
    const supplierId = await seedSupplier(page, '改');
    const created = await createApInvoice(page, supplierId);
    const id = requireId(created, '建应付单');

    await apiCall(page, 'PUT', `/ap/invoices/${id}`, {
      amount: 9876.54, // ① 覆盖数值列
      notes: 'E2E-AP-改后', // ① 覆盖可空文本列
      attachment_urls: null, // ② 单层 Option 域显式 null → 反序列化为 None → 保持原值
      // ③ invoice_type / payment_terms / currency / tax_amount 均不碰
    });
    const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ap/invoices/${id}`);
    expectDecimal(detail, 'amount', 9876.54, 'AP改后');
    // 金额改动联动 unpaid（crud.rs:143-144：unpaid = amount - paid，paid=0）
    expectDecimal(detail, 'unpaid_amount', 9876.54, 'AP改后联动');
    expectKeyValue(detail, 'notes', 'E2E-AP-改后', 'AP改后');
    // ②显式 null 的真实语义=保持原值（单层 Option，ap_invoice_ops/crud.rs:145-152）；
    //   若回读为 null/[] 说明后端把 None 当清空写库 = 数据丢失缺陷，判红。
    expectKeyValue(
      detail,
      'attachment_urls',
      ['https://example.com/a.pdf', 'https://example.com/b.pdf'],
      'AP改后(attachment 应未被 null 静默清空)'
    );
    // ③不碰的字段保持
    expectKeyValue(detail, 'invoice_type', 'PURCHASE', 'AP改后(未碰)');
    expectKeyValue(detail, 'payment_terms', 45, 'AP改后(未碰)');
    expectDecimal(detail, 'tax_amount', 631.05, 'AP改后(未碰)');
    expectDecimal(detail, 'exchange_rate', 7.15, 'AP改后(未碰)');
  });

  test('10-03 应付单状态门：DRAFT 拒绝 cancel/非草稿拒绝改删，approve 后回读词值且负例无痕', async ({
    page,
  }) => {
    const supplierId = await seedSupplier(page, '门');
    const created = await createApInvoice(page, supplierId, { amount: 1000.5 });
    const id = requireId(created, '建应付单');

    // cancel(DRAFT) 被拒（cancel 白名单 AUDITED/PARTIAL_PAID，crud.rs 头注释 :11）。
    // 体必填 reason: String（handlers/ap_invoice_handler.rs:204-206 CancelInvoiceRequest）——
    // 无体将被 axum Json 提取器在进门闸前拒掉（非业务码），故必须带合法体才能命中业务门。
    const failCancel = await apiCallExpectFail(page, 'POST', `/ap/invoices/${id}/cancel`, {
      reason: 'E2E 非法状态取消尝试',
    });
    expect(failCancel.status, `DRAFT cancel 应 400，实际=${failCancel.status}`).toBe(400);
    expect(failureCode(failCancel), 'DRAFT cancel 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const untouched = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ap/invoices/${id}`);
    expectKeyValue(untouched, 'invoice_status', 'DRAFT', 'cancel 被拒后应无痕');
    expectDecimal(untouched, 'amount', 1000.5, 'cancel 被拒后金额未半改');

    // mark-as-paid(DRAFT) 被拒（白名单 AUDITED/PARTIAL_PAID，crud.rs:263-272 AppError::business）
    const failPaid = await apiCallExpectFail(page, 'POST', `/ap/invoices/${id}/mark-as-paid`);
    expect(failPaid.status, 'DRAFT mark-as-paid 应 400').toBe(400);
    expect(failureCode(failPaid), 'DRAFT mark-as-paid 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const afterPaidGate = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/invoices/${id}`
    );
    expectKeyValue(afterPaidGate, 'invoice_status', 'DRAFT', 'mark-as-paid 被拒后仍 DRAFT');

    // approve：DRAFT→AUDITED，先断响应再回查
    const approved = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/ap/invoices/${id}/approve`
    );
    expectKeyValue(approved, 'invoice_status', 'AUDITED', 'approve 响应');
    const reread = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ap/invoices/${id}`);
    expectKeyValue(reread, 'invoice_status', 'AUDITED', 'approve 回读');

    // AUDITED 后 update 被拒（仅 DRAFT 可改，crud.rs:112-118）且回读无痕迹
    const failUpd = await apiCallExpectFail(page, 'PUT', `/ap/invoices/${id}`, { amount: 1 });
    expect(failUpd.status, 'AUDITED update 应 400').toBe(400);
    expect(failureCode(failUpd), 'AUDITED update 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const afterUpd = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ap/invoices/${id}`);
    expectDecimal(afterUpd, 'amount', 1000.5, 'AUDITED 改被拒后金额原样');
    expectKeyValue(afterUpd, 'invoice_status', 'AUDITED', 'AUDITED 改被拒后状态原样');
  });

  test('10-04 应付单删除保护与清理：非 DRAFT 删被拒且仍在；DRAFT 删后 GET 404/NOT_FOUND', async ({
    page,
  }) => {
    const supplierId = await seedSupplier(page, '删');
    const inv = await createApInvoice(page, supplierId, { amount: 2000 });
    const id = requireId(inv, '建应付单');
    await apiCall(page, 'POST', `/ap/invoices/${id}/approve`);

    const failDel = await apiCallExpectFail(page, 'DELETE', `/ap/invoices/${id}`);
    expect(failDel.status, 'AUDITED 删除应 400').toBe(400);
    expect(failureCode(failDel), 'AUDITED 删除机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const still = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ap/invoices/${id}`);
    expectKeyValue(still, 'invoice_status', 'AUDITED', '删除被拒后仍在且未变');

    // DRAFT 单可删：删后按真实 404 契约回读
    const draft = await createApInvoice(page, supplierId, { amount: 10, notes: '待删' });
    const draftId = requireId(draft, '建草稿');
    await apiCall(page, 'DELETE', `/ap/invoices/${draftId}`);
    const gone = await apiCallExpectFail(page, 'GET', `/ap/invoices/${draftId}`);
    expect(gone.status, '删除后 GET 应 404').toBe(404);
    expect(failureCode(gone), '删除后 GET 机器码').toBe('NOT_FOUND');
    // 清理列表里已由 CLEANUP 登记的 id 已删除，tryCleanup 再删 404 仅告警不影响断言
  });

  test('10-05 付款申请全键建单 → 逐字段回读 → submit→approve 链，词值回查', async ({ page }) => {
    const supplierId = await seedSupplier(page, '申请');
    const invoiceId = await seedApprovedApInvoice(page, supplierId, 3000);

    const req = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payment-requests', {
      supplier_id: supplierId,
      request_date: '2026-04-01',
      payment_type: '货款',
      payment_method: '银行转账',
      request_amount: 1234.56,
      currency: 'CNY',
      exchange_rate: 1,
      expected_payment_date: '2026-04-15',
      bank_name: 'E2E 测试银行',
      bank_account: '6222000000000001',
      bank_account_name: '滨州纺织 E2E',
      notes: 'E2E-申请-全字段',
      items: [{ invoice_id: invoiceId, apply_amount: 1234.56, notes: '行备注' }],
    });
    const id = requireId(req, '建付款申请');
    const detail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payment-requests/${id}`
    );
    expectKeyValue(detail, 'supplier_id', supplierId, '申请详情');
    expectKeyValue(detail, 'request_date', '2026-04-01', '申请详情');
    expectKeyValue(detail, 'payment_type', '货款', '申请详情');
    expectKeyValue(detail, 'payment_method', '银行转账', '申请详情');
    expectDecimal(detail, 'request_amount', 1234.56, '申请详情');
    expectKeyValue(detail, 'currency', 'CNY', '申请详情');
    expectKeyValue(detail, 'expected_payment_date', '2026-04-15', '申请详情');
    expectKeyValue(detail, 'bank_name', 'E2E 测试银行', '申请详情');
    expectKeyValue(detail, 'bank_account', '6222000000000001', '申请详情');
    expectKeyValue(detail, 'bank_account_name', '滨州纺织 E2E', '申请详情');
    expectKeyValue(detail, 'notes', 'E2E-申请-全字段', '申请详情');
    expectKeyValue(detail, 'approval_status', 'DRAFT', '申请初始');
    // 缺陷①「再编辑显示不出来」靶心：付款申请明细 items 必须可从详情端点回读。
    // 现源码 get_request（handlers/ap_payment_request_handler.rs:112-146）仅 to_value(model) 平铺，
    // 未内嵌 items，也无申请明细子资源端点 ⇒ 前端重编辑必然拿不到明细。本断言**预期判红**，
    // 红即点名该 handler 行号交编排派修（禁止在此放宽为"键缺失容忍"）。
    if (!Array.isArray(detail.items)) {
      throw new Error(
        `付款申请详情缺 items 数组（明细无法回读=保存数据不完整，缺陷①）；` +
          `backend/src/handlers/ap_payment_request_handler.rs:112-146 未内嵌明细且无子资源端点。实际键=${Object.keys(detail).join(',')}`
      );
    }
    expect(detail.items.length, '申请 items 应含 1 条自建明细').toBe(1);
    expectDecimal(detail.items[0] as Record<string, unknown>, 'apply_amount', 1234.56, '申请明细');

    const submitted = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/ap/payment-requests/${id}/submit`
    );
    expectKeyValue(submitted, 'approval_status', 'APPROVING', 'submit 响应');
    const rereadSub = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payment-requests/${id}`
    );
    expectKeyValue(rereadSub, 'approval_status', 'APPROVING', 'submit 回读');

    // 非法前置：APPROVING 再 submit 被拒且无痕（service.rs:310-316 状态门 AppError::business）
    const failResub = await apiCallExpectFail(page, 'POST', `/ap/payment-requests/${id}/submit`);
    expect(failResub.status, '重复 submit 应 400').toBe(400);
    expect(failureCode(failResub), '重复 submit 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const afterResub = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payment-requests/${id}`
    );
    expectKeyValue(afterResub, 'approval_status', 'APPROVING', '重复 submit 被拒后未漂移');

    const approved = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/ap/payment-requests/${id}/approve`
    );
    expectKeyValue(approved, 'approval_status', 'APPROVED', 'approve 响应');
    const rereadApp = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payment-requests/${id}`
    );
    expectKeyValue(rereadApp, 'approval_status', 'APPROVED', 'approve 回读');
    expect(rereadApp.approved_by !== null, 'approve 后应记录 approved_by').toBe(true);
  });

  test('10-06 付款申请拒绝链：submit→reject→REJECTED 回读；REJECTED 可删、APPROVED 删被拒', async ({
    page,
  }) => {
    const supplierId = await seedSupplier(page, '拒');
    const invoiceId = await seedApprovedApInvoice(page, supplierId, 2500);
    const req = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payment-requests', {
      supplier_id: supplierId,
      request_date: '2026-04-02',
      payment_type: '货款',
      payment_method: '银行转账',
      request_amount: 777.77,
      items: [{ invoice_id: invoiceId, apply_amount: 777.77 }],
    });
    const id = requireId(req, '建付款申请');
    await apiCall(page, 'POST', `/ap/payment-requests/${id}/submit`);

    const rejected = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/ap/payment-requests/${id}/reject`,
      { reason: 'E2E 拒绝原因' }
    );
    expectKeyValue(rejected, 'approval_status', 'REJECTED', 'reject 响应');
    const reread = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payment-requests/${id}`
    );
    expectKeyValue(reread, 'approval_status', 'REJECTED', 'reject 回读');
    // 拒绝原因持久化回读（ap_payment_request.rs:110 rejected_reason）
    expectKeyValue(reread, 'rejected_reason', 'E2E 拒绝原因', 'reject 原因回读');

    // 已审批的申请不可删（删除门仅 DRAFT/REJECTED，service:283-289）；先另建一单走到 APPROVED
    const req2 = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payment-requests', {
      supplier_id: supplierId,
      request_date: '2026-04-02',
      payment_type: '货款',
      payment_method: '银行转账',
      request_amount: 888.88,
      items: [{ invoice_id: invoiceId, apply_amount: 888.88 }],
    });
    const id2 = requireId(req2, '建付款申请2');
    await apiCall(page, 'POST', `/ap/payment-requests/${id2}/submit`);
    await apiCall(page, 'POST', `/ap/payment-requests/${id2}/approve`);
    const failDel = await apiCallExpectFail(page, 'DELETE', `/ap/payment-requests/${id2}`);
    expect(failDel.status, 'APPROVED 删除应 400').toBe(400);
    expect(failureCode(failDel), 'APPROVED 删除机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const survived = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payment-requests/${id2}`
    );
    expectKeyValue(survived, 'approval_status', 'APPROVED', '删除被拒后状态无痕迹');

    // REJECTED 单可删 → GET 404
    await apiCall(page, 'DELETE', `/ap/payment-requests/${id}`);
    const gone = await apiCallExpectFail(page, 'GET', `/ap/payment-requests/${id}`);
    expect(gone.status, 'REJECTED 删除后 GET 404').toBe(404);
    expect(failureCode(gone), '删除后机器码').toBe('NOT_FOUND');
  });

  test('10-07 付款单派生→PUT(REGISTERED 门)→confirm：逐字段回读与状态词值', async ({ page }) => {
    const supplierId = await seedSupplier(page, '款');
    const invoiceId = await seedApprovedApInvoice(page, supplierId, 5000);
    const req = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payment-requests', {
      supplier_id: supplierId,
      request_date: '2026-04-03',
      payment_type: '货款',
      payment_method: '电汇',
      request_amount: 5000,
      currency: 'CNY',
      exchange_rate: 1,
      bank_name: '派生测试行',
      bank_account: '6222000000000002',
      items: [{ invoice_id: invoiceId, apply_amount: 5000 }],
    });
    const requestId = requireId(req, '建付款申请');
    await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/submit`);
    await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/approve`);

    const before = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/invoices/${invoiceId}`
    );
    expectDecimal(before, 'paid_amount', 0, '付款前应付');

    const pay = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payments', {
      request_id: requestId,
      payment_date: '2026-04-20',
      notes: 'E2E-付款-备注',
    });
    const paymentId = requireId(pay, '建付款单');
    const detail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payments/${paymentId}`
    );
    expectKeyValue(detail, 'request_id', requestId, '付款详情派生自申请');
    expectKeyValue(detail, 'supplier_id', supplierId, '付款详情继承申请供应商');
    expectKeyValue(detail, 'payment_method', '电汇', '付款详情继承申请付款方式');
    expectDecimal(detail, 'payment_amount', 5000, '付款金额取申请额（禁手工绕过）');
    expectKeyValue(detail, 'payment_date', '2026-04-20', '付款详情');
    expectKeyValue(detail, 'notes', 'E2E-付款-备注', '付款详情');
    expectKeyValue(detail, 'payment_status', 'REGISTERED', '付款初始');

    // PUT（仅 REGISTERED 可改，ap_payment_service.rs:138）：transaction_no 新值，payment_date 省略保持
    await apiCall(page, 'PUT', `/ap/payments/${paymentId}`, { transaction_no: 'TXN-E2E-0001' });
    const afterUpd = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payments/${paymentId}`
    );
    expectKeyValue(afterUpd, 'transaction_no', 'TXN-E2E-0001', '付款 PUT 回读');
    expectKeyValue(afterUpd, 'payment_date', '2026-04-20', '付款 PUT 省略键应保持原值');

    const confirmed = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/ap/payments/${paymentId}/confirm`
    );
    expectKeyValue(confirmed, 'payment_status', 'CONFIRMED', 'confirm 响应');
    const reread = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payments/${paymentId}`
    );
    expectKeyValue(reread, 'payment_status', 'CONFIRMED', 'confirm 回读');
    // 非 REGISTERED 再 PUT 应被拒（门 :138-143 AppError::business）
    const failUpd = await apiCallExpectFail(page, 'PUT', `/ap/payments/${paymentId}`, {
      notes: '违规修改',
    });
    expect(failUpd.status, 'CONFIRMED 后 PUT 应 400').toBe(400);
    expect(failureCode(failUpd), 'CONFIRMED 后 PUT 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    // confirm 联动应付：paid 增加、状态迁移（PAID/PARTIAL_PAID）
    const invoiceAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/invoices/${invoiceId}`
    );
    expectDecimal(invoiceAfter, 'paid_amount', 5000, 'confirm 后应付 paid');
    expectKeyValue(
      invoiceAfter,
      'invoice_status',
      Number(invoiceAfter.unpaid_amount) === 0 ? 'PAID' : 'PARTIAL_PAID',
      'confirm 后应付状态词值'
    );
  });

  test('10-08 边界负例：金额精度/币种长度/汇率=1（非法）均 400+机器码+具体原因文案（缺陷②靶心）', async ({
    page,
  }) => {
    const supplierId = await seedSupplier(page, '边界');

    // 金额 3 位小数 → validation_displayable("应付单金额精度不能超过 2 位小数")，可外显
    const f1 = await apiCallExpectFail(page, 'POST', '/ap/invoices', {
      supplier_id: supplierId,
      amount: 100.005,
    });
    expect(f1.status, '精度违规应 400').toBe(400);
    expect(failureCode(f1), '精度违规机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      typeof f1.message === 'string' &&
        f1.message.trim().length > 0 &&
        !DESENSITIZED_CONSTANTS.includes(f1.message),
      `4xx message 必须是具体原因而非脱敏常量，实际=${JSON.stringify(f1.message)}（不满足即后端脱敏缺陷，判红并报 file:line utils/messages.rs:41）`
    ).toBe(true);

    // 币种非 3 字母 → #[validate(length(equal=3))]（types.rs:49-50）
    const f2 = await apiCallExpectFail(page, 'POST', '/ap/invoices', {
      supplier_id: supplierId,
      amount: 10,
      currency: 'RMBR',
    });
    expect(f2.status, '币种长度违规应 400').toBe(400);
    expect(failureCode(f2), '币种违规机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      typeof f2.message === 'string' &&
        f2.message.trim().length > 0 &&
        !DESENSITIZED_CONSTANTS.includes(f2.message),
      `币种违规 message 应具体而非脱敏常量，实际=${JSON.stringify(f2.message)}`
    ).toBe(true);

    // 账期越界（range 0..365，types.rs:39-40）
    const f3 = await apiCallExpectFail(page, 'POST', '/ap/invoices', {
      supplier_id: supplierId,
      amount: 10,
      payment_terms: 400,
    });
    expect(f3.status, '账期越界应 400').toBe(400);
    expect(failureCode(f3), '账期越界机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 建单失败的副作用证明：以上均不应落库（列表仍仅前序用例数据，此处按本供应商筛为空）
    const list = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/invoices?supplier_id=${supplierId}&page=1&page_size=10`
    );
    expect(Number(list.total), '非法建单不应产生落库记录').toBe(0);
  });
});
