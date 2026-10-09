// 财务域全流程契约级 E2E — 11 AR 应收（应收单/收款/对账单）
//
// 定位：与 finance/04（金额级回读）、finance/11-02（AR 核销 auto→manual→cancel）互补，
// 本文件补其未覆盖的「创建全字段回读 + PUT 三态 + 对账单状态机」契约链（用户缺陷①②靶心）：
//   ①建单全键 → GET 逐字段回读；②4xx 必断机器码且 message 非脱敏常量（utils/messages.rs:41,43）。
// 契约真值源：
//   CreateArInvoiceRequestDto handlers/ar_invoice_handler.rs:46-58（日期为字符串键，服务层 parse）
//   AR 发票状态门 services/ar_invoice_service.rs:330（update 仅 DRAFT）、:405-428（approve
//     DRAFT→APPROVED 且 approval_status=APPROVED，business_displayable 可外显）、:459-468
//     （mark_as_paid 白名单 APPROVED/PARTIAL_PAID）、:387（delete 仅 DRAFT）
//   UpdateArInvoiceRequest services/ar_invoice_service.rs:26-30（单层 Option，省略=保持，
//     update 金额联动 unpaid=amount-received，:346-351）
//   AR 收款 CreateArPaymentRequest handlers/ar_payment_handler.rs:31-46；出参双键命名
//     services/ar_ops/json_helpers.rs:11-30（payment_no/collection_no、amount/collection_amount…
//     为源码明确契约，两侧同值显式断言，非"双形状探测"）；状态词表小写
//     pending/confirmed/cancelled models/status/finance.rs:11-19；门 services/ar_ops/collection.rs
//     :428-430（update 仅 pending）、:494-497（confirm 仅 pending）、:682-685（cancel 仅 pending）
//   AR 对账 services/ar/recon_ops/crud.rs:30-63（create，closing=opening+invoices-collections
//     算术真值 :35）、lifecycle.rs:37（delete 仅 draft）、:67（send 仅 draft）、:131-136
//     （close 仅 confirmed/disputed）、vfy_ops/confirm.rs（confirm/dispute 门）；
//     状态词表小写 draft/sent/confirmed/disputed/closed/cancelled models/status/finance.rs:21-37；
//     响应键 ReconciliationResponse handlers/ar_reconciliation_handler.rs:26-72（金额/日期字符串）
//   列表形状：GET /ar/invoices 与 /ar-reconciliations 均 PaginatedResponse{items,total,page,page_size}
//     （ar_invoice_handler.rs:73-90、ar_reconciliation_handler.rs:117-141）；
//     GET /ar/payments 为 {list,total,page,page_size}（ar_payment_handler.rs:81-87，按源码断真实形状）
// 诚实标注：CreateArInvoiceRequestDto 不含 tax_amount/quantity_meters/quantity_kg/unit_price
//   （model 有列、导出表头有"税额"列），契约上无法录入 ⇒ 该域列恒 null ⇒ 列入后端缺口清单，
//   本文件断 tax_amount 回读为 null 以钉死该事实，不伪造录入。
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

const DESENSITIZED_CONSTANTS = ['请求参数验证失败', '业务处理失败'];

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

function expectDecimal(obj: Record<string, unknown>, key: string, expected: number, label: string) {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：缺金额键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n)) {
    throw new Error(`${label}：${key} 非数字，raw=${JSON.stringify(obj[key])}`);
  }
  expect(Math.abs(n - expected), `${label}：${key} 期望=${expected} 实际=${n}`).toBeLessThan(0.005);
}

function requireId(obj: Record<string, unknown>, label: string): number {
  const id = Number(obj.id);
  if (!Number.isFinite(id) || id <= 0) {
    throw new Error(`${label}：无有效 id，实际键=${Object.keys(obj).join(',')}`);
  }
  return id;
}

async function seedCustomer(page: import('@playwright/test').Page, tag: string): Promise<number> {
  const name = `E2E全流程AR${tag}${genCode('C')}`;
  const res = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
    customer_name: name,
  });
  if (!res.data?.id) throw new Error(`建客户失败：${JSON.stringify(res)}`);
  CLEANUP.push({ path: `/crm/customers/${res.data.id}`, label: 'customer' });
  return res.data.id;
}

/** 建应收单（键集 = CreateArInvoiceRequestDto 全集，ar_invoice_handler.rs:46-58） */
async function createArInvoice(
  page: import('@playwright/test').Page,
  customerId: number,
  customerName: string,
  overrides: Record<string, unknown> = {}
): Promise<Record<string, unknown>> {
  const inv = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/invoices', {
    invoice_date: '2026-03-01',
    due_date: '2026-03-31',
    customer_id: customerId,
    customer_name: customerName,
    source_type: 'MANUAL',
    source_bill_no: genCode('SRC'),
    invoice_amount: 6000.5,
    batch_no: 'B-E2E-01',
    color_no: 'C-E2E-RED',
    sales_order_no: 'SO-E2E-0001',
    ...overrides,
  });
  CLEANUP.push({ path: `/ar/invoices/${requireId(inv, '建应收单')}`, label: 'ar_invoice' });
  return inv;
}

test.describe('11 AR 应收全流程契约链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('11-01 应收单全键建单 → GET 详情+列表逐字段回读（缺陷①靶心，含双 0 初始额与 null 税额钉桩）', async ({
    page,
  }) => {
    const custId = await seedCustomer(page, '全字段');
    const custName = 'E2E全流程AR全字段客户';
    const created = await createArInvoice(page, custId, custName);
    const id = requireId(created, '建应收单');

    const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/invoices/${id}`);
    expectKeyValue(detail, 'customer_id', custId, 'AR详情');
    expectKeyValue(detail, 'customer_name', custName, 'AR详情');
    expectKeyValue(detail, 'invoice_date', '2026-03-01', 'AR详情');
    expectKeyValue(detail, 'due_date', '2026-03-31', 'AR详情');
    expectKeyValue(detail, 'source_type', 'MANUAL', 'AR详情');
    expectKeyValue(detail, 'source_bill_no', created.source_bill_no, 'AR详情(回显自建源单号)');
    expectDecimal(detail, 'invoice_amount', 6000.5, 'AR详情');
    expectDecimal(detail, 'received_amount', 0, 'AR详情初始');
    expectDecimal(detail, 'unpaid_amount', 6000.5, 'AR详情初始');
    expectKeyValue(detail, 'status', 'DRAFT', 'AR详情初始');
    expectKeyValue(detail, 'approval_status', 'PENDING', 'AR详情初始');
    expectKeyValue(detail, 'batch_no', 'B-E2E-01', 'AR详情');
    expectKeyValue(detail, 'color_no', 'C-E2E-RED', 'AR详情');
    expectKeyValue(detail, 'sales_order_no', 'SO-E2E-0001', 'AR详情');
    // 契约缺口钉桩：Create DTO 无 tax_amount 键（ar_invoice_handler.rs:46-58）⇒ 恒 null
    expectKeyValue(detail, 'tax_amount', null, 'AR详情(契约无录入键)');

    const list = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/invoices?customer_id=${custId}&page=1&page_size=50`
    );
    for (const k of ['items', 'total', 'page', 'page_size']) {
      expect(Object.prototype.hasOwnProperty.call(list, k), `AR列表缺 PaginatedResponse 键 ${k}`);
    }
    const items = list.items as Array<Record<string, unknown>>;
    const row = items.find(r => Number(r.id) === id);
    if (!row) throw new Error(`AR列表未含自建 id=${id}`);
    expectKeyValue(row, 'invoice_no', detail.invoice_no, 'AR列表行');
    expectDecimal(row, 'invoice_amount', 6000.5, 'AR列表行');
    expectKeyValue(row, 'status', 'DRAFT', 'AR列表行');
    expectKeyValue(row, 'sales_order_no', 'SO-E2E-0001', 'AR列表行');
  });

  test('11-02 应收单 PUT 三态（单层 Option）→ 金额联动 unpaid、省略键保持、approve 后改被拒无痕', async ({
    page,
  }) => {
    const custId = await seedCustomer(page, '改');
    const created = await createArInvoice(page, custId, 'E2E改客户');
    const id = requireId(created, '建应收单');

    await apiCall(page, 'PUT', `/ar/invoices/${id}`, {
      invoice_amount: 8000.25, // ① 覆盖
      invoice_date: '2026-03-05', // ① 覆盖
      // ② due_date 省略 → 单层 Option 语义=保持（ar_invoice_service.rs:339-345）
    });
    const d1 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/invoices/${id}`);
    expectDecimal(d1, 'invoice_amount', 8000.25, 'AR改后');
    expectDecimal(d1, 'unpaid_amount', 8000.25, 'AR改后联动 unpaid=amt-received');
    expectKeyValue(d1, 'invoice_date', '2026-03-05', 'AR改后');
    expectKeyValue(d1, 'due_date', '2026-03-31', 'AR省略键应保持原值');

    // approve：DRAFT→APPROVED 且 approval_status=APPROVED（service:423-428）
    const approved = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/ar/invoices/${id}/approve`
    );
    expectKeyValue(approved, 'status', 'APPROVED', 'approve 响应');
    expectKeyValue(approved, 'approval_status', 'APPROVED', 'approve 响应');
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/invoices/${id}`);
    expectKeyValue(d2, 'status', 'APPROVED', 'approve 回读');
    expect(d2.reviewed_by !== null, 'approve 后 reviewed_by 应落库').toBe(true);

    // 非草稿改/删均被拒（门 service:330/387 business_displayable），且回读无痕
    const failUpd = await apiCallExpectFail(page, 'PUT', `/ar/invoices/${id}`, {
      invoice_amount: 1,
    });
    expect(failUpd.status, 'APPROVED 改应 400').toBe(400);
    expect(failureCode(failUpd), 'APPROVED 改机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const failDel = await apiCallExpectFail(page, 'DELETE', `/ar/invoices/${id}`);
    expect(failDel.status, 'APPROVED 删应 400').toBe(400);
    expect(failureCode(failDel), 'APPROVED 删机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const d3 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/invoices/${id}`);
    expectDecimal(d3, 'invoice_amount', 8000.25, '被拒后金额未半改');
    expectKeyValue(d3, 'status', 'APPROVED', '被拒后状态未漂移');

    // 重复 approve 被拒且文案可外显（business_displayable，service:418-419）
    const failRe = await apiCallExpectFail(page, 'POST', `/ar/invoices/${id}/approve`);
    expect(failRe.status, '重复 approve 应 400').toBe(400);
    expect(failureCode(failRe), '重复 approve 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      typeof failRe.message === 'string' && !DESENSITIZED_CONSTANTS.includes(failRe.message),
      `business_displayable 门控文案应可外显，实际=${JSON.stringify(failRe.message)}`
    ).toBe(true);
  });

  test('11-03 AR 建单非法值负例：客户缺失/金额为负/日期格式错均 400+VALIDATION_ERROR+具体原因', async ({
    page,
  }) => {
    const custId = await seedCustomer(page, '负例');

    const f1 = await apiCallExpectFail(page, 'POST', '/ar/invoices', {
      invoice_amount: 100,
      customer_id: null,
    });
    expect(f1.status, '缺客户应 400').toBe(400);
    expect(failureCode(f1), '缺客户机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      typeof f1.message === 'string' && !DESENSITIZED_CONSTANTS.includes(f1.message),
      `validate_create_request 用 validation_displayable（ar_invoice_service.rs:106-110），message 应具体，实际=${JSON.stringify(f1.message)}`
    ).toBe(true);

    const f2 = await apiCallExpectFail(page, 'POST', '/ar/invoices', {
      customer_id: custId,
      invoice_amount: -5,
    });
    expect(f2.status, '负金额应 400').toBe(400);
    expect(failureCode(f2), '负金额机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 日期字符串解析失败（handler 走 validation_displayable("应收单日期格式错误"），可外显）
    const f3 = await apiCallExpectFail(page, 'POST', '/ar/invoices', {
      customer_id: custId,
      invoice_amount: 10,
      invoice_date: '2026/03/01',
    });
    expect(f3.status, '坏日期应 400').toBe(400);
    expect(failureCode(f3), '坏日期机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      typeof f3.message === 'string' && !DESENSITIZED_CONSTANTS.includes(f3.message),
      `坏日期应给出具体原因（缺陷②靶心），实际=${JSON.stringify(f3.message)}`
    ).toBe(true);

    const list = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/invoices?customer_id=${custId}&page=1&page_size=10`
    );
    expect(Number(list.total), '非法建单不应落库').toBe(0);
  });

  test('11-04 收款全键建单→双命名键逐字段回读→PUT(仅 pending)→confirm→回读词值与 confirmed_by', async ({
    page,
  }) => {
    const custId = await seedCustomer(page, '收款');
    const pay = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/payments', {
      customer_id: custId,
      amount: 2500.75,
      payment_method: '银行汇款',
      payment_date: '2026-05-06',
      bank_account: '6217000000000009',
      remark: 'E2E-收款-建单',
    });
    const id = requireId(pay, '建收款');
    // json_helpers.rs:11-30 双命名为源码契约：两侧键同时显式断同值
    expectKeyValue(pay, 'payment_no', pay.collection_no, '收款双键 payment_no=collection_no');
    expectKeyValue(pay, 'amount', pay.collection_amount, '收款双键 amount=collection_amount');
    expectDecimal(pay, 'amount', 2500.75, '收款创建响应');
    expectKeyValue(pay, 'status', 'pending', '收款初始（models/status/finance.rs:13）');
    expectKeyValue(pay, 'payment_date', '2026-05-06', '收款创建响应');
    expectKeyValue(pay, 'bank_account', '6217000000000009', '收款创建响应');
    expectKeyValue(pay, 'customer_id', custId, '收款创建响应');
    // 收款备注落专用真实列（services/ar_ops/collection.rs 的 build_collection_active_model
    // 只写 remark 列），出参 collection_to_json 同时输出 remark 与 check_no 两个独立键，
    // 因此提交的备注按原键回读、支票号不再被备注顶用。
    expectKeyValue(pay, 'remark', 'E2E-收款-建单', '收款建单 remark 按专用列原文回读');
    // 并钉住反向不变量：未提交支票号时该键必须是 null，防止备注再次污染 check_no。
    expectKeyValue(pay, 'check_no', null, '收款建单不得把备注写入 check_no');

    // 列表真实形状 {list,total,page,page_size}（ar_payment_handler.rs:81-87）
    const list = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/payments?customer_id=${custId}&page=1&page_size=20`
    );
    for (const k of ['list', 'total', 'page', 'page_size']) {
      expect(Object.prototype.hasOwnProperty.call(list, k), `AR收款列表缺键 ${k}`);
    }
    const rows = list.list as Array<Record<string, unknown>>;
    const row = rows.find(r => Number(r.id) === id);
    if (!row) throw new Error(`AR收款列表未含自建 id=${id}`);
    expectDecimal(row, 'amount', 2500.75, 'AR收款列表行');
    expectKeyValue(row, 'status', 'pending', 'AR收款列表行');

    // PUT 仅 pending（collection.rs:406-430）：remark 与 check_no 是**独立两列**——
    // collection.rs:579-598 里 remark 入参只写 active.remark、check_no 入参只写 active.check_no，
    // 各列互不覆盖，键缺席=保持原值（收款备注与支票号分列两列）。本次只提交 remark+bank_account，
    // 未提交 check_no，故 remark 落 remark 列、check_no 保持建单时的 null（不被备注顶用）。
    // 省略 amount/payment_date 保持
    await apiCall(page, 'PUT', `/ar/payments/${id}`, {
      remark: 'E2E-收款-改后',
      bank_account: '6217000000000010',
    });
    const afterUpd = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/payments/${id}`);
    // 与建单侧 :283-285 同口径：提交什么键就按什么键原文回读，不被别的列顶用。
    expectKeyValue(afterUpd, 'remark', 'E2E-收款-改后', '收款 PUT 后 remark 按专用列原文回读');
    // 反向不变量：未提交 check_no，PUT 后仍须为 null，防备注再次污染支票号列（同建单侧 :285）。
    expectKeyValue(afterUpd, 'check_no', null, '收款 PUT 不得把备注写入 check_no');
    expectKeyValue(afterUpd, 'bank_account', '6217000000000010', '收款 PUT 回读');
    expectDecimal(afterUpd, 'amount', 2500.75, '收款 PUT 省略键应保持');
    expectKeyValue(afterUpd, 'payment_date', '2026-05-06', '收款 PUT 省略键应保持');

    const confirmed = await apiCall(page, 'POST', `/ar/payments/${id}/confirm`);
    expect(confirmed.code, 'confirm 信封 code 应为 200').toBe(200);
    const d = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/payments/${id}`);
    expectKeyValue(d, 'status', 'confirmed', 'confirm 回读（finance.rs:16）');
    expect(d.confirmed_by !== null, 'confirm 后 confirmed_by 应落库').toBe(true);
    expect(d.confirmed_at !== null, 'confirm 后 confirmed_at 应落库').toBe(true);

    // 非法前置：confirmed 再 confirm 被拒且无痕（门 collection.rs:494-497 AppError::bad_request）
    const fail = await apiCallExpectFail(page, 'POST', `/ar/payments/${id}/confirm`);
    expect(fail.status, '重复 confirm 应 400').toBe(400);
    expect(failureCode(fail), '重复 confirm 机器码').toBe(APP_ERROR_CODES.BAD_REQUEST);
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/payments/${id}`);
    expectKeyValue(d2, 'status', 'confirmed', '重复 confirm 被拒后未漂移');
  });

  test('11-05 收款取消门：pending 可 cancel→cancelled 并清空 confirmed_*；confirmed 取消被拒无痕', async ({
    page,
  }) => {
    const custId = await seedCustomer(page, '取消');
    const a = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/payments', {
      customer_id: custId,
      amount: 300,
      payment_method: '现金',
      payment_date: '2026-05-07',
    });
    const idA = requireId(a, '建收款A');
    await apiCall(page, 'POST', `/ar/payments/${idA}/cancel`);
    const da = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/payments/${idA}`);
    expectKeyValue(da, 'status', 'cancelled', 'cancel 回读（finance.rs:19）');
    expectKeyValue(da, 'confirmed_by', null, '取消后 confirmed_by 应清空（collection.rs:712-718）');

    const b = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/payments', {
      customer_id: custId,
      amount: 400,
      payment_method: '现金',
      payment_date: '2026-05-07',
    });
    const idB = requireId(b, '建收款B');
    await apiCall(page, 'POST', `/ar/payments/${idB}/confirm`);
    const fail = await apiCallExpectFail(page, 'POST', `/ar/payments/${idB}/cancel`);
    expect(fail.status, 'confirmed 取消应 400').toBe(400);
    expect(failureCode(fail), 'confirmed 取消机器码').toBe(APP_ERROR_CODES.BAD_REQUEST);
    const db = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/payments/${idB}`);
    expectKeyValue(db, 'status', 'confirmed', '取消被拒后仍 confirmed');
  });

  test('11-06 对账单建单→closing 算术真值回读→PUT(draft 门)金额改+省略键保持+notes 新值', async ({
    page,
  }) => {
    const custId = await seedCustomer(page, '对账');
    const no = genCode('RECON');
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar-reconciliations', {
      reconciliation_no: no,
      customer_id: custId,
      customer_name: 'E2E对账客户',
      period_start: '2026-05-01',
      period_end: '2026-05-31',
      opening_balance: 1000.5,
      total_invoices: 2000.25,
      total_collections: 600.75,
    });
    const id = requireId(created, '建对账单');
    expectKeyValue(created, 'reconciliation_no', no, '对账创建响应');
    expectKeyValue(created, 'period_start', '2026-05-01', '对账创建响应');
    expectDecimal(created, 'opening_balance', 1000.5, '对账创建响应');
    // closing = opening + invoices - collections（recon_ops/crud.rs:35）
    expectDecimal(created, 'closing_balance', 1000.5 + 2000.25 - 600.75, 'closing 算术真值');
    expectKeyValue(created, 'reconciliation_status', 'draft', '对账初始（finance.rs:22）');

    const detail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar-reconciliations/${id}`
    );
    expectKeyValue(detail, 'customer_id', custId, '对账详情');
    expectKeyValue(detail, 'customer_name', 'E2E对账客户', '对账详情(缺陷①靶心：客户名回显)');

    // PUT 仅 draft；单层 Option 省略保持（crud.rs:112-143）；closing 自动重算
    await apiCall(page, 'PUT', `/ar-reconciliations/${id}`, {
      total_collections: 1000, // 覆盖
      notes: 'E2E-对账-备注', // 覆盖（notes 列已接入持久化，crud.rs:141-143）
      // opening_balance / total_invoices 省略 → 保持
    });
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar-reconciliations/${id}`);
    expectDecimal(d2, 'total_collections', 1000, '对账 PUT 回读');
    expectDecimal(d2, 'opening_balance', 1000.5, '对账 PUT 省略键保持');
    expectDecimal(d2, 'total_invoices', 2000.25, '对账 PUT 省略键保持');
    expectDecimal(d2, 'closing_balance', 1000.5 + 2000.25 - 1000, 'PUT 后 closing 重算');
    expectKeyValue(d2, 'notes', 'E2E-对账-备注', 'notes 应持久化（缺失即缺陷①判红）');
  });

  test('11-07 对账单状态机：send→dispute 分支 + draft 上 close/send 门负例无痕', async ({
    page,
  }) => {
    const custId = await seedCustomer(page, '状态机');
    const no = genCode('RECON');
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar-reconciliations', {
      reconciliation_no: no,
      customer_id: custId,
      period_start: '2026-06-01',
      period_end: '2026-06-30',
      opening_balance: 0,
      total_invoices: 100,
      total_collections: 0,
    });
    const id = requireId(created, '建对账单');

    // draft 上 close 被拒（门仅 confirmed/disputed，lifecycle.rs:131-136 AppError::business）
    const failClose = await apiCallExpectFail(page, 'POST', `/ar-reconciliations/${id}/close`);
    expect(failClose.status, 'draft close 应 400').toBe(400);
    expect(failureCode(failClose), 'draft close 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const d0 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar-reconciliations/${id}`);
    expectKeyValue(d0, 'reconciliation_status', 'draft', 'close 被拒后未漂移');

    // send：draft→sent（lifecycle.rs:67-74）
    const sent = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/ar-reconciliations/${id}/send`
    );
    expectKeyValue(sent, 'reconciliation_status', 'sent', 'send 响应');
    const d1 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar-reconciliations/${id}`);
    expectKeyValue(d1, 'reconciliation_status', 'sent', 'send 回读');

    // sent 上再 send 被拒（门仅 draft，lifecycle.rs:67-74 AppError::business）
    const failSend = await apiCallExpectFail(page, 'POST', `/ar-reconciliations/${id}/send`);
    expect(failSend.status, '重复 send 应 400').toBe(400);
    expect(failureCode(failSend), '重复 send 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // sent 上 update 被拒（仅 draft 可改，handler 注释 + service 门）
    const failUpd = await apiCallExpectFail(page, 'PUT', `/ar-reconciliations/${id}`, {
      total_invoices: 999,
    });
    expect(failUpd.status, 'sent 上 PUT 应 400').toBe(400);
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar-reconciliations/${id}`);
    expectDecimal(d2, 'total_invoices', 100, 'PUT 被拒后金额未半改');

    // dispute：sent→disputed 且原因回显（vfy_ops/confirm.rs customer_dispute）
    const disputed = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/ar-reconciliations/${id}/dispute`,
      { reason: 'E2E 金额不符' }
    );
    expectKeyValue(disputed, 'reconciliation_status', 'disputed', 'dispute 响应');
    const d3 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar-reconciliations/${id}`);
    expectKeyValue(d3, 'reconciliation_status', 'disputed', 'dispute 回读');
    // 争议原因持久化回读（列缺失/未写入=缺陷①，判红并报 recon.rs dispute_reason 写入点）
    if (!Object.prototype.hasOwnProperty.call(d3, 'dispute_reason')) {
      throw new Error(
        `对账详情响应缺 dispute_reason 键（ReconciliationResponse 未透出该列），实际键=${Object.keys(d3).join(',')}`
      );
    }

    // 非法状态白名单值经 PUT /status 应被拒（lifecycle.rs:246-250）
    const failStatus = await apiCallExpectFail(page, 'PUT', `/ar-reconciliations/${id}/status`, {
      status: 'BOGUS_STATE',
    });
    expect(failStatus.status, '非法状态词应 400').toBe(400);
    expect(failureCode(failStatus), '非法状态机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const d4 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar-reconciliations/${id}`);
    expectKeyValue(d4, 'reconciliation_status', 'disputed', '非法状态被拒后未漂移');
  });

  test('11-08 对账单确认链与删除门：sent→confirm→confirmed；非 draft 删被拒；draft 删后 GET 404', async ({
    page,
  }) => {
    const custId = await seedCustomer(page, '确认删');
    const no = genCode('RECON');
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar-reconciliations', {
      reconciliation_no: no,
      customer_id: custId,
      period_start: '2026-07-01',
      period_end: '2026-07-31',
      opening_balance: 50,
      total_invoices: 150,
      total_collections: 50,
    });
    const id = requireId(created, '建对账单');
    await apiCall(page, 'POST', `/ar-reconciliations/${id}/send`);

    const confirmed = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/ar-reconciliations/${id}/confirm`
    );
    expectKeyValue(confirmed, 'reconciliation_status', 'confirmed', 'confirm 响应');
    const d1 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar-reconciliations/${id}`);
    expectKeyValue(d1, 'reconciliation_status', 'confirmed', 'confirm 回读（finance.rs:28）');
    expectKeyValue(d1, 'customer_name', created.customer_name, 'confirm 后客户名仍完整');

    // 重复 confirm 被拒（confirm.rs:39 business）且无痕
    const failRe = await apiCallExpectFail(page, 'POST', `/ar-reconciliations/${id}/confirm`);
    expect(failRe.status, '重复 confirm 应 400').toBe(400);
    expect(failureCode(failRe), '重复 confirm 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar-reconciliations/${id}`);
    expectKeyValue(d2, 'reconciliation_status', 'confirmed', '重复 confirm 后未漂移');

    // 非 draft 删除被拒（lifecycle.rs:37-39 business）
    const failDel = await apiCallExpectFail(page, 'DELETE', `/ar-reconciliations/${id}`);
    expect(failDel.status, 'confirmed 删除应 400').toBe(400);
    expect(failureCode(failDel), 'confirmed 删除机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const d3 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar-reconciliations/${id}`);
    expectDecimal(d3, 'closing_balance', 50 + 150 - 50, '删除被拒后余额未动');

    // draft 可删 → GET 404/NOT_FOUND
    const no2 = genCode('RECON');
    const draft = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar-reconciliations', {
      reconciliation_no: no2,
      customer_id: custId,
      period_start: '2026-08-01',
      period_end: '2026-08-31',
      opening_balance: 0,
      total_invoices: 0,
      total_collections: 0,
    });
    const draftId = requireId(draft, '建草稿对账');
    await apiCall(page, 'DELETE', `/ar-reconciliations/${draftId}`);
    const gone = await apiCallExpectFail(page, 'GET', `/ar-reconciliations/${draftId}`);
    expect(gone.status, '删除后 GET 404').toBe(404);
    expect(failureCode(gone), '删除后机器码').toBe('NOT_FOUND');
  });
});
