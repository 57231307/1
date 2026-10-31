/**
 * 161 付款申请到付款计划的链路口径、对账单与供应商应付汇总的集合差、发票关联的跨表回读。
 *
 * 本文件守的口径面。付款申请列表的分页回显与上限钳位、明细跨口回读、超未付额申请被拒后
 * 零落库；表头更新的 PATCH 语义（键缺省即不改列）与提交后的状态门拒绝；付款计划的两道门
 * （只取 APPROVED 且 expected_payment_date 落在闭区间）；对账单列表与生成口同一行的逐列一致、
 * 状态与日期两族过滤；供应商应付汇总把草稿计入而余额出口把草稿挡住的集合差、
 * 汇总出参的供应商编码与名称来自供应商表富化而非应付表；发票关联口对纯手工票返回空集、
 * 对同供应商付款单按登记与确认两态如实带出。
 *
 * 覆盖的端点（全部取自路由快照在册的真实「方法 + 路径」组合，不做别名与拼装）。
 * GET /api/v1/erp/ap/payment-requests 付款申请列表
 * PUT /api/v1/erp/ap/payment-requests/{id} 付款申请更新
 * GET /api/v1/erp/ap/payments/schedule 付款计划
 * GET /api/v1/erp/ap/reconciliations 对账单列表
 * GET /api/v1/erp/ap/reconciliations/summary 供应商应付汇总
 * GET /api/v1/erp/ap/invoices/{id}/relations 发票关联信息
 * GET /api/v1/erp/ap/verifications 应付核销列表
 * （路径里的 {id} 是后端路由注册时的原样段名，路由快照与覆盖判定按通配归一。）
 *
 * 契约真相（逐段读后端源，非推测）。
 * 路由装配在 backend/src/routes/finance.rs 的 ap_payment_request_routes、ap_payment_routes、
 * ap_reconciliation_routes 与 ap_verification_routes。列表出口统一走
 * backend/src/utils/response.rs 的 ApiResponse 套 PaginatedResponse，分页四键落在 data 内、
 * 顶层不带 total；page 钳位在 1 到 1000、page_size 钳位在 1 到 100，钳位后的值原样回显。
 * 付款申请建单（backend/src/services/ap_payment_request_service.rs 的 create）：
 * 必填 supplier_id、request_date、payment_type、payment_method、request_amount，
 * request_no 为 PRQ 前缀加八位日期加流水，初值 approval_status 为 DRAFT，
 * 币种与汇率由 resolve_currency_and_rate 服务端权威解析（本位币短路为 1，外币缺汇率被
 * VALIDATION_ERROR 拒绝），明细 items 可缺省，行落 ap_payment_request_item。
 * 明细校验（validate_invoice_items_txn）：引用的应付单不存在即 404 NOT_FOUND；状态为 DRAFT
 * 或 CANCELLED 即 400 BUSINESS_ERROR；单行 apply_amount 超过该票未付额即 400 BUSINESS_ERROR，
 * 且拒绝发生在建单事务内、主表与明细零写入。
 * 表头更新（update）：仅 DRAFT 可改，否则 400 BUSINESS_ERROR；UpdateApPaymentRequest 不含
 * items 键，表头更新不改明细行；PATCH 语义下缺省的键保持原列值。
 * 删除（delete）：仅 DRAFT 与 REJECTED 可删，先按 request_id 清明细再删主表。
 * 提交（submit）：仅 DRAFT 且明细非空才推进 APPROVING，无明细走可外显业务拒绝；
 * 审批（approve）：仅 APPROVING 可推进 APPROVED，并按金额分级校验审批人角色
 * （本 worker 会话为 admin，十万以下放行）。
 * 付款计划（ap_payment_service.rs 的 get_payment_schedule）：只取 approval_status 为 APPROVED
 * 且 expected_payment_date 在起止闭区间内的申请，按日期升序分桶，输出 payment_date、
 * total_amount、payment_count 三键；start_date 与 end_date 是非 Option 的 NaiveDate 入参，
 * 缺参死在 axum Query 抽取层后由 middleware/trace_context.rs 的 normalize_extractor_rejection
 * 收口为统一信封：400 配 code=VALIDATION_ERROR 与固定公开规则文案（serde 原文只进日志）。
 * 对账单生成（ap_reconciliation_ops/crud.rs 的 generate_reconciliation）：本期应付取
 * invoice_date 在区间内且状态不在 CANCELLED 与 DRAFT 内的应付单合计，本期付款取
 * payment_date 在区间内且状态 CONFIRMED 的付款单合计，期初取区间起之前同排除口径的未付合计，
 * 期末等于期初加本期应付减本期付款，初值 reconciliation_status 为 PENDING，
 * 单号 REC 前缀加八位日期加流水；该族只注册了 GET 与 POST，没有删除通道。
 * 供应商应付汇总（ap_reconciliation_ops/report.rs 的 get_supplier_summary）：从 ap_invoice
 * 全量分组，不排除草稿与已取消，供应商编码与名称由 suppliers 表富化，
 * 出参是按供应商聚合后的数组（该供应商无任何应付单时数组为空）。
 * 发票关联（get_invoice_relations）：应付单不存在即 404 NOT_FOUND；source_type 为
 * PURCHASE_RECEIPT 时带出一行入库单关联，另按该票所属供应商列出其全部付款单，
 * 行的 source_type 写 PAYMENT、amount 取 payment_amount、status 取 payment_status，
 * 因此纯手工票且该供应商尚无付款单时返回空数组；采购退货来源不在本出口的关联集合内。
 *
 * 假绿防线。每个用例为本 worker 另建专属供应商，使按 supplier_id 过滤的列表与聚合出口
 * 只有我自己的行，绝对值与空集都能直接断言；每次写后独立读回逐列对表，被拒申请用
 * 「该供应商名下付款申请计数」反查零落库；金额一律按 Decimal 字符串折分位比较，
 * 标度超出两位且尾数非零即抛错，不静默舍入；未提交的可选列断言为 NULL 而不是 0；
 * 状态词与机器码与后端常量表逐字符同源（DRAFT/APPROVING/APPROVED/REJECTED/PENDING/
 * REGISTERED/CONFIRMED/PAID）；对账单与被审批的付款申请在 HTTP 面上没有删除通道，
 * 本文件如实写明并不打空清理；全程不用可选链与兜底运算符。
 */
import { test, expect, type Page } from '../diagnose-fixture';
import {
  APP_ERROR_CODES,
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
  ensureTestEntities,
  failureCode,
  genCode,
  loginViaUI,
} from './helpers';

type Row = Record<string, unknown>;

/**
 * Decimal 出参折到指定标度的整数刻度。
 * 调用方：本文件的 eqAmount 与计划金额比对。入参：出参原值（字符串为主、兼容数字）、
 * 目标标度、断言位置标签。传给谁：仅参与比较，不外发、不落库。
 * 标度超出目标位且尾数非零即抛错点名原值，不做四舍五入。
 */
function toScaled(v: unknown, scale: number, label: string): bigint {
  const s = typeof v === 'string' ? v.trim() : typeof v === 'number' ? String(v) : '';
  if (s === '') throw new Error(`${label}：Decimal 出参非 string|number，raw=${JSON.stringify(v)}`);
  const m = /^(-?)(\d+)(?:\.(\d+))?$/.exec(s);
  if (!m) throw new Error(`${label}：Decimal 出参形态非法，raw="${s}"`);
  const [, sign, intPart, frac = ''] = m;
  if (frac.length > scale && frac.slice(scale).replace(/0+$/, '') !== '') {
    throw new Error(`${label}：出参标度超出分位精度且非零，raw="${s}"`);
  }
  const padded = (intPart + frac).padEnd(intPart.length + scale, '0');
  const value = BigInt(padded);
  return sign === '-' ? -value : value;
}

/** 两位小数金额字面量折分位（期望值侧唯一入口） */
function cents(want: string): bigint {
  return toScaled(want, 2, `期望金额 ${want}`);
}

/** 金额等值断言：实际出参折分位后与期望分位逐分对齐 */
function eqAmount(actual: unknown, expectedCents: bigint, label: string): void {
  expect(
    toScaled(actual, 2, label),
    `${label}：期望(分)=${expectedCents} 实际=${JSON.stringify(actual)}`
  ).toBe(expectedCents);
}

/** 取正整数主键，缺失或非正即抛错点名步骤 */
function requireId(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isInteger(n) || n <= 0) {
    throw new Error(`${label}：无有效整数 id，raw=${JSON.stringify(v)}`);
  }
  return n;
}

/** 取非负整数计数，非整数即抛错而不是当作 0 */
function requireCount(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isInteger(n) || n < 0) {
    throw new Error(`${label}：计数非整数，raw=${JSON.stringify(v)}`);
  }
  return n;
}

/** 相对今天的 UTC 日历日（后端付款计划与对账区间按 NaiveDate 比较，用例侧必须同基准） */
function utcDate(offsetDays: number): string {
  return new Date(Date.now() + offsetDays * 86_400_000).toISOString().slice(0, 10);
}

/**
 * 为本用例新建专属供应商并返回主键。
 * 调用方：本文件全部用例的第一步。入参：tag 为本用例唯一标记，写进供应商名称。
 * 传给谁：POST /purchase/suppliers，返回 id 作应付单与付款申请的 supplier_id
 * 以及所有列表与聚合出口的过滤键。存什么：名称、简称、联系电话；存哪里：suppliers 表。
 * 不复用 ctx.supplierId：同 worker 共用供应商会被前一个用例造数污染，绝对值断言即失效。
 */
async function createDedicatedSupplier(page: Page, tag: string): Promise<number> {
  const created = await apiCallRaw<Row>(page, 'POST', '/purchase/suppliers', {
    supplier_name: `E2E应付161-${tag}`,
    supplier_short_name: `E161${tag.slice(-4)}`,
    contact_phone: '13800000001',
  });
  return requireId(created.id, `专属供应商 ${tag} 创建`);
}

/** 建应付单（create_manual 六要素必填口径），返回主表整行 */
async function createInvoice(
  page: Page,
  supplierId: number,
  amount: string,
  dueDate: string,
  notes: string
): Promise<Row> {
  const created = await apiCallRaw<Row>(page, 'POST', '/ap/invoices', {
    supplier_id: supplierId,
    invoice_type: 'PURCHASE',
    invoice_date: utcDate(0),
    due_date: dueDate,
    amount,
    tax_amount: '0.00',
    notes,
  });
  return { ...created, id: requireId(created.id, `建应付单 ${notes}`) };
}

/** 审核应付单并读回（付款申请的明细只接受非草稿非取消的票） */
async function approveInvoice(page: Page, id: number): Promise<void> {
  await apiCall(page, 'POST', `/ap/invoices/${id}/approve`);
  const after = await apiCallRaw<Row>(page, 'GET', `/ap/invoices/${id}`);
  expect(String(after.invoice_status), `应付单 ${id} 审核回读状态`).toBe('AUDITED');
}

/** 建付款申请（可带明细），返回主表整行 */
async function createRequest(
  page: Page,
  supplierId: number,
  amount: string,
  expectedDate: string,
  notes: string,
  items?: Array<{ invoice_id: number; apply_amount: string }>
): Promise<Row> {
  const created = await apiCallRaw<Row>(page, 'POST', '/ap/payment-requests', {
    supplier_id: supplierId,
    request_date: utcDate(0),
    payment_type: 'PROGRESS',
    payment_method: 'TT',
    request_amount: amount,
    expected_payment_date: expectedDate,
    notes,
    ...(items ? { items } : {}),
  });
  return { ...created, id: requireId(created.id, `建付款申请 ${notes}`) };
}

/** 分页列表出口的 data 四键形态断言并取出 data */
async function readList(
  page: Page,
  path: string,
  label: string
): Promise<{ items: Row[]; total: number; page: number; page_size: number }> {
  const res = await apiCall<Row>(page, 'GET', path);
  expect(typeof res.code, `${label} 成功信封 code 类型`).toBe('number');
  expect(res.code, `${label} 成功信封 code 必须为数字 200`).toBe(200);
  expect(
    'total' in res,
    `${label} 成功信封顶层不得带 total（分页元数据只落在 data 内），实际键 ${JSON.stringify(Object.keys(res))}`
  ).toBe(false);
  const data = res.data;
  expect(data === undefined, `${label} 必须带 data 键`).toBe(false);
  expect(
    Array.isArray(data.items),
    `${label} data.items 必须是数组，实际 ${JSON.stringify(data)}`
  ).toBe(true);
  return {
    items: data.items as Row[],
    total: requireCount(data.total, `${label} data.total`),
    page: requireCount(data.page, `${label} data.page`),
    page_size: requireCount(data.page_size, `${label} data.page_size`),
  };
}

/** 在列表数组里按主键取恰一行，取不到即抛错点名，不把缺行当成空对象 */
function pickById(rows: Row[], id: number, label: string): Row {
  const hit = rows.filter(r => String(r.id) === String(id));
  expect(hit.length, `${label}：主键 ${id} 必须恰命中一行，实际 ${JSON.stringify(rows)}`).toBe(1);
  return hit[0];
}

test.describe.serial('161 付款申请链路 对账汇总与发票关联契约链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('161-01 付款申请建单初值与明细分页钳位逐列回读，超未付额申请被拒且零落库', async ({
    page,
  }) => {
    const tag = genCode('E161A');
    const supplierId = await createDedicatedSupplier(page, tag);

    const invoice = await createInvoice(page, supplierId, '1000.00', utcDate(-5), `${tag}-票`);
    const invoiceId = requireId(invoice.id, '夹具应付单');
    await approveInvoice(page, invoiceId);

    const req = await createRequest(page, supplierId, '1000.00', utcDate(15), `${tag}-申请`, [
      { invoice_id: invoiceId, apply_amount: '1000.00' },
    ]);
    const requestId = requireId(req.id, '付款申请');

    // 建单初值：单号形态、状态、服务端权威币种与汇率、未提交的可选列必须是 NULL
    expect(
      /^PRQ\d{11}$/.test(String(req.request_no)),
      `申请单号必须是 PRQ 加八位日期加流水，实际 ${String(req.request_no)}`
    ).toBe(true);
    expect(String(req.approval_status), '建单初值状态必须为 DRAFT').toBe('DRAFT');
    expect(String(req.currency), '币种缺省走服务端默认').toBe('CNY');
    expect(Number(req.exchange_rate), '本位币汇率必须被服务端短路为 1').toBe(1);
    eqAmount(req.request_amount, cents('1000.00'), '申请金额');
    expect(String(req.expected_payment_date), '期望付款日必须回显提交值').toBe(utcDate(15));
    expect(req.bank_name, '未提交收款银行必须存 NULL 而非空串').toBeNull();
    expect(req.bank_account, '未提交收款账号必须存 NULL').toBeNull();
    expect(req.approved_by, '建单不得写审批人').toBeNull();
    expect(req.approved_at, '建单不得写审批时间').toBeNull();
    expect(req.submitted_at, '建单不得写提交时间').toBeNull();
    const me = await apiCallRaw<Row>(page, 'GET', '/auth/me');
    expect(String(req.created_by), '建单人必须是会话用户').toBe(String(me.id));

    // 明细跨口回读：详情口带出 items 数组，行的申请额与引用票与主表关联逐字段对齐
    const detail = await apiCallRaw<Row>(page, 'GET', `/ap/payment-requests/${requestId}`);
    const items = detail.items as Row[];
    expect(
      Array.isArray(items),
      `详情口必须带出 items 数组，实际 ${JSON.stringify(detail.items)}`
    ).toBe(true);
    expect(items.length, '明细必须恰一行').toBe(1);
    expect(String(items[0].invoice_id), '明细行引用的应付单必须是夹具票').toBe(String(invoiceId));
    eqAmount(items[0].apply_amount, cents('1000.00'), '明细行申请额');
    expect(String(items[0].request_id), '明细行外键必须指向本申请').toBe(String(requestId));

    // 列表出口：分页元数据回显与钳位（page_size 上限 100、page 上限 1000）
    const paged = await readList(
      page,
      `/ap/payment-requests?supplier_id=${supplierId}&page=1&page_size=500`,
      '付款申请列表钳位页'
    );
    expect(paged.page_size, 'page_size 必须被钳位为 100 并回显').toBe(100);
    expect(paged.page, '页码原样回显').toBe(1);
    expect(paged.total, '本供应商名下申请计数').toBe(1);

    const clampedPage = await readList(
      page,
      `/ap/payment-requests?supplier_id=${supplierId}&page=99999&page_size=20`,
      '付款申请页码钳位'
    );
    expect(clampedPage.page, '越界页码必须被钳位为 1000').toBe(1000);
    expect(
      clampedPage.items.length,
      `钳位后页码越界应取空页，实际 ${JSON.stringify(clampedPage.items)}`
    ).toBe(0);
    expect(clampedPage.total, '钳位不得改写总数').toBe(1);

    // 状态过滤门：DRAFT 命中一行、APPROVED 空集（过滤是等值条件而非模糊匹配）
    const draftRows = await readList(
      page,
      `/ap/payment-requests?supplier_id=${supplierId}&approval_status=DRAFT&page=1&page_size=20`,
      '草稿态过滤'
    );
    expect(draftRows.total, 'DRAFT 过滤必须命中自建申请').toBe(1);
    pickById(draftRows.items, requestId, 'DRAFT 过滤命中行');
    const approvedRows = await readList(
      page,
      `/ap/payment-requests?supplier_id=${supplierId}&approval_status=APPROVED&page=1&page_size=20`,
      '已审批态过滤'
    );
    expect(approvedRows.total, 'APPROVED 过滤必须空集').toBe(0);
    expect(approvedRows.items.length, 'APPROVED 过滤的 items 必须为空数组').toBe(0);

    // 超未付额的明细申请被业务门拒：机器码 BUSINESS_ERROR 且脱敏文案，主表与明细零写入
    const overApply = await apiCallExpectFail(page, 'POST', '/ap/payment-requests', {
      supplier_id: supplierId,
      request_date: utcDate(0),
      payment_type: 'PROGRESS',
      payment_method: 'TT',
      request_amount: '1000.01',
      notes: `${tag}-超额`,
      items: [{ invoice_id: invoiceId, apply_amount: '1000.01' }],
    });
    expect(overApply.status, `超额申请应 400，实际 ${JSON.stringify(overApply)}`).toBe(400);
    expect(failureCode(overApply), '超额申请机器码应为 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    expect(overApply.message, 'BusinessError 走脱敏站点，文案必须是固定常量').toBe('业务处理失败');
    const afterReject = await readList(
      page,
      `/ap/payment-requests?supplier_id=${supplierId}&page=1&page_size=20`,
      '被拒后列表'
    );
    expect(afterReject.total, '被拒申请必须零落库').toBe(1);

    // 引用不存在应付单走 404 NOT_FOUND（明细校验先于落库）
    const ghostInvoice = await apiCallExpectFail(page, 'POST', '/ap/payment-requests', {
      supplier_id: supplierId,
      request_date: utcDate(0),
      payment_type: 'PROGRESS',
      payment_method: 'TT',
      request_amount: '10.00',
      notes: `${tag}-幽灵票`,
      items: [{ invoice_id: 999999999, apply_amount: '10.00' }],
    });
    expect(
      ghostInvoice.status,
      `引用不存在应付单应 404，实际 ${JSON.stringify(ghostInvoice)}`
    ).toBe(404);
    expect(failureCode(ghostInvoice), '幽灵票引用机器码应为 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
    const stillOne = await readList(
      page,
      `/ap/payment-requests?supplier_id=${supplierId}&page=1&page_size=20`,
      '幽灵票拒绝后列表'
    );
    expect(stillOne.total, '幽灵票拒绝必须零落库').toBe(1);
  });

  test('161-02 表头更新按 PATCH 语义逐列回读且不改明细，提交后更新与删除两门都被拒', async ({
    page,
  }) => {
    const tag = genCode('E161B');
    const supplierId = await createDedicatedSupplier(page, tag);
    const invoice = await createInvoice(page, supplierId, '800.00', utcDate(10), `${tag}-票`);
    const invoiceId = requireId(invoice.id, '夹具应付单');
    await approveInvoice(page, invoiceId);

    const req = await createRequest(page, supplierId, '800.00', utcDate(12), `${tag}-申请`, [
      { invoice_id: invoiceId, apply_amount: '800.00' },
    ]);
    const requestId = requireId(req.id, '付款申请');

    // 表头更新：提交的四列改写，未提交的 request_amount 与 payment_type 保持原值
    const updated = await apiCallRaw<Row>(page, 'PUT', `/ap/payment-requests/${requestId}`, {
      payment_method: 'WIRE',
      expected_payment_date: utcDate(20),
      bank_name: '测试收款行',
      bank_account: '6222000000000000',
      notes: `${tag}-更新后备注`,
    });
    expect(String(updated.payment_method), '付款方式必须改写').toBe('WIRE');
    expect(String(updated.expected_payment_date), '期望付款日必须改写').toBe(utcDate(20));
    expect(String(updated.bank_name), '收款银行必须落提交值').toBe('测试收款行');
    expect(String(updated.bank_account), '收款账号必须落提交值').toBe('6222000000000000');
    expect(String(updated.notes), '备注必须落提交值').toBe(`${tag}-更新后备注`);
    eqAmount(updated.request_amount, cents('800.00'), '未提交金额保持原值');
    expect(String(updated.payment_type), '未提交付款类型保持原值').toBe('PROGRESS');
    expect(String(updated.approval_status), '更新不改状态列').toBe('DRAFT');

    // 独立读回逐列对表（不只看更新出口回显），并钉明细未被表头更新波及
    const reread = await apiCallRaw<Row>(page, 'GET', `/ap/payment-requests/${requestId}`);
    expect(String(reread.notes), '详情读回备注与更新出口一致').toBe(String(updated.notes));
    expect(String(reread.bank_account), '详情读回收款账号与更新出口一致').toBe(
      String(updated.bank_account)
    );
    const me = await apiCallRaw<Row>(page, 'GET', '/auth/me');
    expect(String(reread.updated_by), '更新人列必须写会话用户').toBe(String(me.id));
    const rereadItems = reread.items as Row[];
    expect(rereadItems.length, '表头更新不得增删明细行').toBe(1);
    eqAmount(rereadItems[0].apply_amount, cents('800.00'), '明细行申请额不被表头更新改写');
    expect(String(rereadItems[0].invoice_id), '明细行引用票不变').toBe(String(invoiceId));

    // 提交推进 APPROVING 后，更新门与删除门都拒绝（仅 DRAFT 与 REJECTED 可删）
    await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/submit`);
    const submitted = await apiCallRaw<Row>(page, 'GET', `/ap/payment-requests/${requestId}`);
    expect(String(submitted.approval_status), '提交后状态必须为 APPROVING').toBe('APPROVING');
    expect(String(submitted.submitted_by), '提交人列必须写会话用户').toBe(String(me.id));
    expect(
      typeof submitted.submitted_at === 'string' &&
        !Number.isNaN(Date.parse(String(submitted.submitted_at))),
      `提交时间必须是可解析时间串，实际 ${JSON.stringify(submitted.submitted_at)}`
    ).toBe(true);

    const putBlocked = await apiCallExpectFail(page, 'PUT', `/ap/payment-requests/${requestId}`, {
      notes: `${tag}-提交后改写`,
    });
    expect(putBlocked.status, `提交后更新应 400，实际 ${JSON.stringify(putBlocked)}`).toBe(400);
    expect(failureCode(putBlocked), '提交后更新机器码应为 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    const afterBlockedPut = await apiCallRaw<Row>(page, 'GET', `/ap/payment-requests/${requestId}`);
    expect(String(afterBlockedPut.notes), '被拒更新不得改写备注').toBe(`${tag}-更新后备注`);
    expect(String(afterBlockedPut.approval_status), '被拒更新不得改状态').toBe('APPROVING');

    const deleteBlocked = await apiCallExpectFail(
      page,
      'DELETE',
      `/ap/payment-requests/${requestId}`
    );
    expect(deleteBlocked.status, `提交后删除应 400，实际 ${JSON.stringify(deleteBlocked)}`).toBe(
      400
    );
    expect(failureCode(deleteBlocked), '提交后删除机器码应为 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    const stillThere = await apiCallRaw<Row>(page, 'GET', `/ap/payment-requests/${requestId}`);
    expect(String(stillThere.id), '被拒删除后行必须仍在库').toBe(String(requestId));

    // 草稿态删除腿真实存在：另一张无明细草稿申请删得掉、删后详情 404
    const draftReq = await createRequest(page, supplierId, '50.00', utcDate(3), `${tag}-可删草稿`);
    const draftId = requireId(draftReq.id, '可删草稿申请');
    await apiCall(page, 'DELETE', `/ap/payment-requests/${draftId}`);
    const gone = await apiCallExpectFail(page, 'GET', `/ap/payment-requests/${draftId}`);
    expect(gone.status, `已删申请详情应 404，实际 ${JSON.stringify(gone)}`).toBe(404);
    expect(failureCode(gone), '已删申请详情机器码应为 NOT_FOUND').toBe(APP_ERROR_CODES.NOT_FOUND);
    const listAfterDelete = await readList(
      page,
      `/ap/payment-requests?supplier_id=${supplierId}&page=1&page_size=20`,
      '删除后列表'
    );
    expect(listAfterDelete.total, '本供应商名下应只剩提交前的那一张').toBe(1);

    // 无明细提交的门（可外显业务拒绝）：草稿申请不补明细即提交被拒
    const noItemsReq = await createRequest(page, supplierId, '30.00', utcDate(4), `${tag}-无明细`);
    const noItemsId = requireId(noItemsReq.id, '无明细申请');
    const submitFail = await apiCallExpectFail(
      page,
      'POST',
      `/ap/payment-requests/${noItemsId}/submit`
    );
    expect(submitFail.status, `无明细提交应 400，实际 ${JSON.stringify(submitFail)}`).toBe(400);
    expect(failureCode(submitFail), '无明细提交机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(submitFail.message, '无明细拒绝走可外显变体，文案原文可钉').toBe(
      '付款申请没有明细，不可提交'
    );
    const noItemsAfter = await apiCallRaw<Row>(page, 'GET', `/ap/payment-requests/${noItemsId}`);
    expect(String(noItemsAfter.approval_status), '被拒提交不得推进状态').toBe('DRAFT');
    // 清理腿：无明细草稿可直接删（DRAFT 放行），删后详情 404
    await apiCall(page, 'DELETE', `/ap/payment-requests/${noItemsId}`);
    const noItemsGone = await apiCallExpectFail(page, 'GET', `/ap/payment-requests/${noItemsId}`);
    expect(noItemsGone.status, '清理后无明细申请应 404').toBe(404);
  });

  test('161-03 付款计划只取已审批且期望付款日落闭区间，缺起止参数走抽取器拒绝收口为 VALIDATION_ERROR 信封', async ({
    page,
  }) => {
    const tag = genCode('E161C');
    const supplierId = await createDedicatedSupplier(page, tag);
    const invoice = await createInvoice(page, supplierId, '1200.00', utcDate(5), `${tag}-票`);
    const invoiceId = requireId(invoice.id, '夹具应付单');
    await approveInvoice(page, invoiceId);

    const expectedDate = utcDate(20);
    const req = await createRequest(page, supplierId, '1200.00', expectedDate, `${tag}-申请`, [
      { invoice_id: invoiceId, apply_amount: '1200.00' },
    ]);
    const requestId = requireId(req.id, '付款申请');

    // 审批前：状态门挡住，计划区间内不得出现本申请
    const windowStart = utcDate(10);
    const windowEnd = utcDate(25);
    const scheduleBefore = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/ap/payments/schedule?supplier_id=${supplierId}&start_date=${windowStart}&end_date=${windowEnd}`
    );
    expect(
      Array.isArray(scheduleBefore),
      `计划出口 data 必须是数组，实际 ${JSON.stringify(scheduleBefore)}`
    ).toBe(true);
    expect(
      scheduleBefore.length,
      `未审批申请不得进计划，实际 ${JSON.stringify(scheduleBefore)}`
    ).toBe(0);

    await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/submit`);
    await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/approve`);
    const approved = await apiCallRaw<Row>(page, 'GET', `/ap/payment-requests/${requestId}`);
    expect(String(approved.approval_status), '审批回读状态必须为 APPROVED').toBe('APPROVED');
    expect(String(approved.approved_by), '审批人列必须写会话用户').toBe(
      String((await apiCallRaw<Row>(page, 'GET', '/auth/me')).id)
    );

    // 审批后：计划按日期分桶，桶内金额与张数取该日已审批申请之和
    const schedule = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/ap/payments/schedule?supplier_id=${supplierId}&start_date=${windowStart}&end_date=${windowEnd}`
    );
    expect(schedule.length, `计划必须恰一桶，实际 ${JSON.stringify(schedule)}`).toBe(1);
    expect(String(schedule[0].payment_date), '计划桶日期必须是期望付款日').toBe(expectedDate);
    eqAmount(schedule[0].total_amount, cents('1200.00'), '计划桶金额');
    expect(requireCount(schedule[0].payment_count, '计划桶张数')).toBe(1);

    // 区间门是闭区间比较：把窗口整体推到期望付款日之后，桶即消失
    const outsideStart = utcDate(21);
    const outsideEnd = utcDate(40);
    const outside = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/ap/payments/schedule?supplier_id=${supplierId}&start_date=${outsideStart}&end_date=${outsideEnd}`
    );
    expect(outside.length, `区间外必须空数组，实际 ${JSON.stringify(outside)}`).toBe(0);

    // 区间左端正好等于期望付款日：闭区间命中（不是左开右闭）
    const leftEdgeEnd = utcDate(25);
    const leftEdge = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/ap/payments/schedule?supplier_id=${supplierId}&start_date=${expectedDate}&end_date=${leftEdgeEnd}`
    );
    expect(leftEdge.length, '左端点等值必须命中').toBe(1);
    eqAmount(leftEdge[0].total_amount, cents('1200.00'), '左端点命中桶金额');

    // 缺 start_date 与 end_date：非 Option 入参死在 Query 抽取层，被收口为统一信封
    const missingRange = await apiCallExpectFail(page, 'GET', '/ap/payments/schedule');
    expect(missingRange.status, `缺起止参数应 400，实际 ${JSON.stringify(missingRange)}`).toBe(400);
    expect(
      failureCode(missingRange),
      `缺参拒绝机器码应为 VALIDATION_ERROR，实际 ${JSON.stringify(missingRange)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      missingRange.message,
      `缺参拒绝文案应为固定公开规则串，实际 ${JSON.stringify(missingRange.message)}`
    ).toBe(
      '请求格式不正确：请求体或查询参数存在缺失的必填字段、字段类型错误或内容类型错误，请修正后重试'
    );

    // 只给一个端点同样被抽取层拒（两个日期入参都是必填），拒绝形态与全缺同信封
    const halfRange = await apiCallExpectFail(
      page,
      'GET',
      `/ap/payments/schedule?supplier_id=${supplierId}&start_date=${expectedDate}`
    );
    expect(halfRange.status, `缺区间止应 400，实际 ${JSON.stringify(halfRange)}`).toBe(400);
    expect(failureCode(halfRange), '缺区间止机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );

    // 已审批申请在 HTTP 面上没有删除通道（删除门只放行草稿与被拒），本用例如实不清理
    const deleteBlocked = await apiCallExpectFail(
      page,
      'DELETE',
      `/ap/payment-requests/${requestId}`
    );
    expect(
      deleteBlocked.status,
      `已审批申请删除应 400，实际 ${JSON.stringify(deleteBlocked)}`
    ).toBe(400);
    expect(failureCode(deleteBlocked), '已审批申请删除机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const stillApproved = await apiCallRaw<Row>(page, 'GET', `/ap/payment-requests/${requestId}`);
    expect(String(stillApproved.approval_status), '被拒删除后仍 APPROVED').toBe('APPROVED');
  });

  test('161-04 对账单列表与生成口同一行逐列互证，状态与日期两族过滤生效且本族无删除通道', async ({
    page,
  }) => {
    const tag = genCode('E161D');
    const supplierId = await createDedicatedSupplier(page, tag);
    const today = utcDate(0);

    // 两张票：区间内已审核的一张计入本期应付，区间外的草稿既不进本期也不进期初
    const inRange = await createInvoice(page, supplierId, '600.00', utcDate(20), `${tag}-区间内`);
    const inRangeId = requireId(inRange.id, '区间内票');
    await approveInvoice(page, inRangeId);
    const draftInvoice = await createInvoice(
      page,
      supplierId,
      '900.00',
      utcDate(25),
      `${tag}-草稿`
    );
    const draftInvoiceId = requireId(draftInvoice.id, '草稿票');

    const generated = await apiCallRaw<Row>(page, 'POST', '/ap/reconciliations/generate', {
      supplier_id: supplierId,
      start_date: utcDate(-1),
      end_date: utcDate(1),
      notes: `${tag}-对账`,
    });
    const recId = requireId(generated.id, '生成对账单');
    expect(
      /^REC\d{11}$/.test(String(generated.reconciliation_no)),
      `对账单号必须是 REC 加八位日期加流水，实际 ${String(generated.reconciliation_no)}`
    ).toBe(true);
    expect(String(generated.reconciliation_status), '生成初值状态必须为 PENDING').toBe('PENDING');
    eqAmount(generated.opening_balance, cents('0.00'), '期初余额（区间起前本供应商无票）');
    eqAmount(generated.total_invoice, cents('600.00'), '本期应付合计（草稿被排除）');
    eqAmount(generated.total_payment, cents('0.00'), '本期付款合计（无付款单）');
    eqAmount(generated.closing_balance, cents('600.00'), '期末余额=期初+本期应付-本期付款');

    // 列表出口逐列对表：同一行在两个出口的主键与全部金额列必须一致
    const list = await readList(
      page,
      `/ap/reconciliations?supplier_id=${supplierId}&page=1&page_size=20`,
      '对账单列表'
    );
    expect(list.total, '本供应商名下对账单计数').toBe(1);
    const listed = pickById(list.items, recId, '对账单列表命中行');
    expect(String(listed.reconciliation_no), '列表单号与生成口一致').toBe(
      String(generated.reconciliation_no)
    );
    expect(String(listed.reconciliation_status), '列表状态与生成口一致').toBe('PENDING');
    for (const col of ['opening_balance', 'total_invoice', 'total_payment', 'closing_balance']) {
      eqAmount(
        listed[col],
        toScaled(generated[col], 2, `生成口 ${col}`),
        `列表 ${col} 必须与生成口同源`
      );
    }
    expect(String(listed.start_date), '列表区间起与提交值一致').toBe(utcDate(-1));
    expect(String(listed.end_date), '列表区间止与提交值一致').toBe(utcDate(1));
    expect(String(listed.supplier_id), '列表供应商键一致').toBe(String(supplierId));
    expect(String(listed.notes), '列表备注与提交值一致').toBe(`${tag}-对账`);

    // 状态过滤：PENDING 命中、CONFIRMED 空集
    const pendingRows = await readList(
      page,
      `/ap/reconciliations?supplier_id=${supplierId}&reconciliation_status=PENDING&page=1&page_size=20`,
      '对账单状态过滤 PENDING'
    );
    expect(pendingRows.total, 'PENDING 过滤必须命中').toBe(1);
    const confirmedRows = await readList(
      page,
      `/ap/reconciliations?supplier_id=${supplierId}&reconciliation_status=CONFIRMED&page=1&page_size=20`,
      '对账单状态过滤 CONFIRMED'
    );
    expect(confirmedRows.total, 'CONFIRMED 过滤必须空集').toBe(0);

    // 日期过滤：start_date 是「对账区间起不早于该值」，把门槛推到本单区间起之后即空集
    const dateFiltered = await readList(
      page,
      `/ap/reconciliations?supplier_id=${supplierId}&start_date=${utcDate(2)}&page=1&page_size=20`,
      '对账单日期过滤'
    );
    expect(dateFiltered.total, '区间起晚于本单起点的过滤必须空集').toBe(0);
    const dateIncluded = await readList(
      page,
      `/ap/reconciliations?supplier_id=${supplierId}&end_date=${utcDate(1)}&page=1&page_size=20`,
      '对账单日期过滤含入'
    );
    expect(dateIncluded.total, '区间止不晚于本单终点的过滤必须命中').toBe(1);

    // 详情口第三次互证（列表与生成与详情三处同源），并如实记录本族没有删除通道
    const detail = await apiCallRaw<Row>(page, 'GET', `/ap/reconciliations/${recId}`);
    expect(String(detail.reconciliation_no), '详情单号与生成口一致').toBe(
      String(generated.reconciliation_no)
    );
    eqAmount(detail.closing_balance, cents('600.00'), '详情期末余额');

    // 争议门：对账单提出争议后状态推进 DISPUTED（本族唯一的合法状态推进通道之一），
    // 三个争议列写入并读回；争议不改写任何金额列，期末余额仍与生成口同源
    await apiCall(page, 'POST', `/ap/reconciliations/${recId}/dispute`, {
      reason: `${tag} 金额异议`,
    });
    const disputed = await apiCallRaw<Row>(page, 'GET', `/ap/reconciliations/${recId}`);
    expect(String(disputed.reconciliation_status), '争议后状态必须为 DISPUTED').toBe('DISPUTED');
    expect(String(disputed.disputed_reason), '争议原因列必须写提交值').toBe(`${tag} 金额异议`);
    const me = await apiCallRaw<Row>(page, 'GET', '/auth/me');
    expect(String(disputed.disputed_by), '争议人列必须写会话用户').toBe(String(me.id));
    expect(
      typeof disputed.disputed_at === 'string' &&
        !Number.isNaN(Date.parse(String(disputed.disputed_at))),
      `争议时间必须是可解析时间串，实际 ${JSON.stringify(disputed.disputed_at)}`
    ).toBe(true);
    eqAmount(disputed.closing_balance, cents('600.00'), '争议不改写期末余额');
    const disputedList = await readList(
      page,
      `/ap/reconciliations?supplier_id=${supplierId}&reconciliation_status=PENDING&page=1&page_size=20`,
      '争议后 PENDING 过滤'
    );
    expect(disputedList.total, '状态推进后 PENDING 过滤必须空集').toBe(0);

    // 对账单族没有删除通道（backend/src/routes/finance.rs 的 ap_reconciliation_routes 只注册
    // GET 与 POST），故本用例不清理对账单、也不打空清理；夹具草稿票有删除腿，删后回读 404
    await apiCall(page, 'DELETE', `/ap/invoices/${draftInvoiceId}`);
    const draftGone = await apiCallExpectFail(page, 'GET', `/ap/invoices/${draftInvoiceId}`);
    expect(draftGone.status, `已删草稿票详情应 404，实际 ${JSON.stringify(draftGone)}`).toBe(404);
    expect(failureCode(draftGone), '已删草稿票机器码应为 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
    const recUnchanged = await apiCallRaw<Row>(page, 'GET', `/ap/reconciliations/${recId}`);
    eqAmount(recUnchanged.closing_balance, cents('600.00'), '删除草稿票不得回改已生成的对账单金额');
  });

  test('161-05 供应商应付汇总把草稿计入而余额出口把草稿挡住，编码与名称来自供应商表富化', async ({
    page,
  }) => {
    const tag = genCode('E161E');
    const supplierId = await createDedicatedSupplier(page, tag);
    const emptySupplierId = await createDedicatedSupplier(page, `${tag}空`);

    const draftForSummary = await createInvoice(
      page,
      supplierId,
      '500.00',
      utcDate(-5),
      `${tag}-草稿票`
    );
    const audited = await createInvoice(
      page,
      supplierId,
      '1000.00',
      utcDate(-8),
      `${tag}-已审核票`
    );
    await approveInvoice(page, requireId(audited.id, '已审核票'));

    // 汇总出口：草稿同样计入（与余额出口的排除集合不同源）
    const summary = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/ap/reconciliations/summary?supplier_id=${supplierId}`
    );
    expect(
      Array.isArray(summary),
      `汇总出口 data 必须是数组，实际 ${JSON.stringify(summary)}`
    ).toBe(true);
    expect(summary.length, '按单一供应商过滤必须恰一行').toBe(1);
    const row = summary[0];
    expect(String(row.supplier_id), '汇总行供应商键必须是过滤入参').toBe(String(supplierId));
    expect(requireCount(row.total_invoice_count, '汇总应付张数（含草稿）')).toBe(2);
    eqAmount(row.total_invoice_amount, cents('1500.00'), '汇总应付合计（含草稿）');
    eqAmount(row.total_unpaid_amount, cents('1500.00'), '汇总未付合计');
    eqAmount(row.total_paid_amount, cents('0.00'), '汇总已付合计');
    expect(requireCount(row.paid_invoice_count, '汇总已付清张数')).toBe(0);
    expect(requireCount(row.partial_paid_invoice_count, '汇总部分付款张数')).toBe(0);
    expect(requireCount(row.overdue_invoice_count, '汇总逾期张数（两张到期日均已过）')).toBe(2);
    eqAmount(row.overdue_amount, cents('1500.00'), '汇总逾期金额取未付额');

    // 同一供应商在余额出口只计已审核那一张，两口的集合差被逐列钉出
    const balance = await apiCallRaw<Row>(
      page,
      'GET',
      `/ap/invoices/balance?supplier_id=${supplierId}`
    );
    expect(requireCount(balance.invoice_count, '余额出口张数（排除草稿）')).toBe(1);
    eqAmount(balance.total_invoice_amount, cents('1000.00'), '余额出口应付合计');
    const sumMinusBalance =
      toScaled(row.total_invoice_amount, 2, '汇总应付') -
      toScaled(balance.total_invoice_amount, 2, '余额应付');
    expect(sumMinusBalance, '两口差额必须恰是被排除的那张草稿金额').toBe(cents('500.00'));

    // 富化列取自供应商表：编码与名称与详情口逐字符相等，而不是应付表里的冗余列
    const supplierDetail = await apiCallRaw<Row>(page, 'GET', `/purchase/suppliers/${supplierId}`);
    expect(String(row.supplier_name), '汇总供应商名称必须与供应商详情口同源').toBe(
      String(supplierDetail.supplier_name)
    );
    expect(String(row.supplier_code), '汇总供应商编码必须与供应商详情口同源').toBe(
      String(supplierDetail.supplier_code)
    );

    // 无应付单的供应商：聚合 map 无键，出参空数组而非补一行零值
    const emptySummary = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/ap/reconciliations/summary?supplier_id=${emptySupplierId}`
    );
    expect(
      emptySummary.length,
      `无应付单供应商必须空数组，实际 ${JSON.stringify(emptySummary)}`
    ).toBe(0);

    // 清理腿（草稿票可删）：删掉那张被两口口径差涉及的草稿，删完回读确认从汇总里消失，
    // 用同一出口的前后两读把「草稿计入汇总」这一事实与「删除确实生效」一起钉死
    const draftId = requireId(draftForSummary.id, '草稿票');
    await apiCall(page, 'DELETE', `/ap/invoices/${draftId}`);
    const draftGone = await apiCallExpectFail(page, 'GET', `/ap/invoices/${draftId}`);
    expect(draftGone.status, `已删草稿票详情应 404，实际 ${JSON.stringify(draftGone)}`).toBe(404);
    expect(failureCode(draftGone), '已删草稿票机器码应为 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
    const summaryAfterDelete = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/ap/reconciliations/summary?supplier_id=${supplierId}`
    );
    expect(summaryAfterDelete.length, '删除后汇总仍恰一行').toBe(1);
    expect(requireCount(summaryAfterDelete[0].total_invoice_count, '删除草稿后汇总张数')).toBe(1);
    eqAmount(summaryAfterDelete[0].total_invoice_amount, cents('1000.00'), '删除草稿后汇总合计');
  });

  test('161-06 发票关联对纯手工票返回空集并在付款登记与确认后带出同供应商付款行', async ({
    page,
  }) => {
    const tag = genCode('E161F');
    const supplierId = await createDedicatedSupplier(page, tag);
    const invoice = await createInvoice(page, supplierId, '1000.00', utcDate(6), `${tag}-票`);
    const invoiceId = requireId(invoice.id, '夹具应付单');
    await approveInvoice(page, invoiceId);

    // 手工票无采购入库来源、该供应商尚无付款单：关联集合为空数组
    const relationsEmpty = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/ap/invoices/${invoiceId}/relations`
    );
    expect(
      Array.isArray(relationsEmpty),
      `关联出口 data 必须是数组，实际 ${JSON.stringify(relationsEmpty)}`
    ).toBe(true);
    expect(
      relationsEmpty.length,
      `手工票且无付款时必须空数组，实际 ${JSON.stringify(relationsEmpty)}`
    ).toBe(0);

    // 走完申请到付款的链路：付款单登记后关联口按供应商维度带出该付款行
    const req = await createRequest(page, supplierId, '1000.00', utcDate(18), `${tag}-申请`, [
      { invoice_id: invoiceId, apply_amount: '1000.00' },
    ]);
    const requestId = requireId(req.id, '付款申请');
    await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/submit`);
    await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/approve`);

    const payment = await apiCallRaw<Row>(page, 'POST', '/ap/payments', {
      request_id: requestId,
      payment_date: utcDate(0),
      notes: `${tag}-付款`,
    });
    const paymentId = requireId(payment.id, '付款单');
    expect(
      /^PAY\d{11}$/.test(String(payment.payment_no)),
      `付款单号必须是 PAY 加八位日期加流水，实际 ${String(payment.payment_no)}`
    ).toBe(true);
    expect(String(payment.payment_status), '付款单初值状态必须为 REGISTERED').toBe('REGISTERED');
    eqAmount(payment.payment_amount, cents('1000.00'), '付款金额取申请额而非请求原值');

    const relationsAfterPayment = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/ap/invoices/${invoiceId}/relations`
    );
    expect(
      relationsAfterPayment.length,
      `登记一张付款单后关联必须恰一行，实际 ${JSON.stringify(relationsAfterPayment)}`
    ).toBe(1);
    const rel = relationsAfterPayment[0];
    expect(String(rel.invoice_id), '关联行的发票主键必须是本票').toBe(String(invoiceId));
    expect(String(rel.invoice_no), '关联行的发票单号必须是本票').toBe(String(invoice.invoice_no));
    expect(String(rel.source_type), '付款行 source_type 必须写 PAYMENT').toBe('PAYMENT');
    expect(String(rel.source_id), '付款行 source_id 必须是付款单主键').toBe(String(paymentId));
    expect(String(rel.source_no), '付款行 source_no 必须是付款单号').toBe(
      String(payment.payment_no)
    );
    expect(String(rel.supplier_id), '关联行供应商键必须是本票供应商').toBe(String(supplierId));
    eqAmount(rel.amount, cents('1000.00'), '关联行金额取付款金额');
    expect(String(rel.status), '关联行状态取付款单状态').toBe('REGISTERED');

    // 确认付款（需先补交易流水号）后，关联行状态随付款单迁移，票面资金列同步回写
    await apiCall(page, 'PUT', `/ap/payments/${paymentId}`, { transaction_no: genCode('E161TXN') });
    const confirmed = await apiCallRaw<Row>(page, 'POST', `/ap/payments/${paymentId}/confirm`);
    expect(String(confirmed.payment_status), '确认付款后状态必须为 CONFIRMED').toBe('CONFIRMED');

    const relationsAfterConfirm = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/ap/invoices/${invoiceId}/relations`
    );
    expect(relationsAfterConfirm.length, '确认后关联仍恰一行').toBe(1);
    expect(String(relationsAfterConfirm[0].status), '关联行状态随付款单迁移为 CONFIRMED').toBe(
      'CONFIRMED'
    );
    const paidInvoice = await apiCallRaw<Row>(page, 'GET', `/ap/invoices/${invoiceId}`);
    eqAmount(paidInvoice.paid_amount, cents('1000.00'), '确认后票面已付列回写');
    eqAmount(paidInvoice.unpaid_amount, cents('0.00'), '确认后票面未付列归零');

    // 结清票退出口径互证：账龄两口把 PAID 挡住，汇总与余额两口仍计入（排除集合三处不同）
    const agingAfterPaid = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/ap/invoices/aging?supplier_id=${supplierId}`
    );
    expect(
      agingAfterPaid.length,
      `已付清票必须被账龄口排除，实际 ${JSON.stringify(agingAfterPaid)}`
    ).toBe(0);
    const balanceAfterPaid = await apiCallRaw<Row>(
      page,
      'GET',
      `/ap/invoices/balance?supplier_id=${supplierId}`
    );
    expect(requireCount(balanceAfterPaid.invoice_count, '余额口仍计入已付清票')).toBe(1);

    // 关联口对不存在的票走 404 NOT_FOUND（先查票再组关联，不在空票上编造行）
    const ghost = await apiCallExpectFail(page, 'GET', '/ap/invoices/999999999/relations');
    expect(ghost.status, `不存在发票的关联口应 404，实际 ${JSON.stringify(ghost)}`).toBe(404);
    expect(failureCode(ghost), '幽灵票关联机器码应为 NOT_FOUND').toBe(APP_ERROR_CODES.NOT_FOUND);

    // 同一申请重复建付款单被拒（一申请一付款的收口），关联行数不得因此增加
    const dupPayment = await apiCallExpectFail(page, 'POST', '/ap/payments', {
      request_id: requestId,
      payment_date: utcDate(0),
      notes: `${tag}-重复付款`,
    });
    expect(dupPayment.status, `重复建付款单应 400，实际 ${JSON.stringify(dupPayment)}`).toBe(400);
    expect(failureCode(dupPayment), '重复建付款单机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const relationsFinal = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/ap/invoices/${invoiceId}/relations`
    );
    expect(relationsFinal.length, '被拒付款单不得进关联集合').toBe(1);
    // 残留口径：付款单族在 backend/src/routes/finance.rs 只注册查询、创建、更新与 confirm，
    // 没有删除通道；已确认付款单与结清（PAID）票均留在库，本用例如实不清理，
    // 关联与账龄出口按专属供应商过滤，唯一 tag 保证不与其它用例互污
  });

  test('161-07 核销列表分页四键与钳位与三类过滤在无核销数据时都是空集而非补零行', async ({
    page,
  }) => {
    const tag = genCode('E161G');
    const supplierId = await createDedicatedSupplier(page, tag);

    const listed = await readList(
      page,
      `/ap/verifications?supplier_id=${supplierId}&page=1&page_size=20`,
      '核销列表'
    );
    expect(listed.total, '专属供应商名下无核销记录时 total 必须为 0').toBe(0);
    expect(listed.items.length, '空集必须出空数组而不是补一行零值').toBe(0);
    expect(listed.page, '页码回显').toBe(1);
    expect(listed.page_size, '每页条数回显').toBe(20);

    const clamped = await readList(
      page,
      `/ap/verifications?supplier_id=${supplierId}&page=1&page_size=500`,
      '核销列表页宽钳位'
    );
    expect(clamped.page_size, '每页条数必须被钳位为 100').toBe(100);

    const clampedPage = await readList(
      page,
      `/ap/verifications?supplier_id=${supplierId}&page=5000&page_size=20`,
      '核销列表页码钳位'
    );
    expect(clampedPage.page, '越界页码必须被钳位为 1000').toBe(1000);

    const typed = await readList(
      page,
      `/ap/verifications?supplier_id=${supplierId}&verification_type=AUTO&page=1&page_size=20`,
      '核销类型过滤'
    );
    expect(typed.total, '类型过滤在无数据时必须空集').toBe(0);

    const rangeStart = utcDate(-365);
    const rangeEnd = utcDate(365);
    const ranged = await readList(
      page,
      `/ap/verifications?supplier_id=${supplierId}&start_date=${rangeStart}&end_date=${rangeEnd}&page=1&page_size=20`,
      '核销日期区间过滤'
    );
    expect(ranged.total, '日期区间过滤在无数据时必须空集').toBe(0);

    // 非法日期串即使字段是 Option<NaiveDate> 也在抽取层反序列化失败，收口为统一信封
    const badDate = await apiCallExpectFail(
      page,
      'GET',
      `/ap/verifications?supplier_id=${supplierId}&start_date=2001-13-01`
    );
    expect(badDate.status, `非法核销起始日应 400，实际 ${JSON.stringify(badDate)}`).toBe(400);
    expect(failureCode(badDate), '非法核销起始日机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
  });
});
