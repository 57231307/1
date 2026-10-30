/**
 * 160 应付单余额·账龄·统计三口径与四张应付报表的契约真值链。
 *
 * 本文件守的口径面。手工建单的六要素必填门与金额精度门拒绝后必须零落库；草稿与已取消
 * 两个状态在余额、账龄两张出口被排除门挡住，而综合统计的第三块（状态分布）却把它们计入，
 * 同一张 ap_invoice 表在四张口径上的排除集合并不一致，此前无人钉；四张应付报表的期间回显、
 * 账龄六桶的固定序与占比算法、日报的等值日门、月报的月界与期初期末余额、以及基准日入参
 * 对分桶的迁移作用，同样无人钉。
 *
 * 覆盖的端点（全部取自路由快照在册的真实「方法 + 路径」组合，不做别名与拼装）。
 * GET /api/v1/erp/ap/invoices/aging 应付账龄分析
 * GET /api/v1/erp/ap/invoices/balance 应付余额汇总
 * GET /api/v1/erp/ap/invoices/statistics 应付综合统计
 * GET /api/v1/erp/ap/reports/aging 应付账龄分析报告
 * GET /api/v1/erp/ap/reports/daily 应付日报
 * GET /api/v1/erp/ap/reports/monthly 应付月报
 * GET /api/v1/erp/ap/reports/statistics 应付统计报表
 *
 * 契约真相（逐段读后端源，非推测）。
 * 路由装配在 backend/src/routes/finance.rs 的 ap_invoice_routes 与 ap_report_routes，
 * 前缀分别是 /ap/invoices 与 /ap/reports。出参统一走 backend/src/utils/response.rs 的
 * ApiResponse：成功信封为数字 code 加 data（写请求另带 message），顶层不带 total，
 * 分页元数据落在 data 内的 PaginatedResponse 四键里。
 * 建单入口 backend/src/handlers/ap_invoice_handler.rs 的 create_ap_invoice 先跑 validator
 * 字段校验，再由 backend/src/services/ap_invoice_ops/crud.rs 的 create_manual 逐键点名缺失：
 * 应付单缺少供应商ID、应付单缺少应付类型、应付单缺少发票日期、应付单缺少到期日期、
 * 应付单缺少发票金额、以及「应付单缺少税额，无进项税请显式提交 0」，这六条走可外显校验
 * 变体（HTTP 400 配 VALIDATION_ERROR 且文案原文外显）；金额精度门「应付单金额精度不能超过
 * 2 位小数」先于缺键检查执行。
 * 建单初值：状态 DRAFT、paid_amount 恒 0、unpaid_amount 由 amount 减 paid_amount 派生、
 * invoice_no 为 API 前缀加八位日期加流水、source_type 写 MANUAL，payment_terms 与 currency
 * 与 exchange_rate 缺省走服务端默认而非请求原值。
 * 状态门：仅 DRAFT 可审核（推进 AUDITED）、仅 DRAFT 可修改与删除、仅 AUDITED 或
 * PARTIAL_PAID 可取消且 paid_amount 必须为 0；被拒一律 HTTP 400 配 BUSINESS_ERROR，
 * 出参文案走脱敏常量「业务处理失败」。
 * 排除门。backend/src/services/ap_invoice_ops/report.rs 的 get_balance_summary 排除
 * CANCELLED 与 DRAFT；get_aging_analysis 在此之上再排除 PAID（只统计未付清），账龄桶按
 * due_date 相对当天（UTC 日历日）的逾期天数归类，桶标签固定为「未到期 / 逾期 1-30 天 /
 * 逾期 31-60 天 / 逾期 61-90 天 / 逾期 91-180 天 / 逾期 180 天以上」六档，且只输出有数据的
 * 桶（可能为空数组）；get_statistics 的第三块按状态分组，不排除草稿与已取消。
 * 报表侧（backend/src/services/ap_report_service.rs）走 SQL 聚合而非内存归类：
 * get_aging_report 恒输出六桶并按序补齐零值桶，percentage 为桶金额除总未付额乘 100，
 * 总未付额为 0 时全桶占比留 0（不做除零），baseline_date 缺省取当天、显式传入即改变全部桶
 * 的归属；get_daily_report 的新增门是 invoice_date 等值于 report_date 且状态不在 CANCELLED
 * 与 DRAFT 内，到期门是 due_date 等值于 report_date 且 unpaid_amount 大于 0，付款门取
 * ap_payment 的 payment_date 等值且状态 CONFIRMED；get_statistics_report 与
 * get_monthly_report 的排除门同为 CANCELLED 与 DRAFT，月报期初取 invoice_date 早于月首的
 * 未付合计、期末取不晚于月末的未付合计。
 *
 * 入参拒绝形态（按代码事实钉，不按直觉）。报表口的日期入参在
 * backend/src/handlers/ap_report_handler.rs 与 ap_payment_handler.rs 的查询结构体里是
 * 非 Option 的 NaiveDate 类型，缺参或日期串解析失败发生在 axum Query 抽取器的反序列化层，
 * 该纯文本拒绝由 middleware/trace_context.rs 的 normalize_extractor_rejection 收口为
 * 统一失败信封：HTTP 400 配 code=VALIDATION_ERROR，文案恒为 backend/src/utils/error.rs
 * 的公开规则常量 REQUEST_DECODING_PUBLIC（serde 原文只进服务端日志不外显）；因此断言钉
 * 「400 且机器码 VALIDATION_ERROR 且文案逐字为该公开规则串」。能进 handler 的非法月份
 * （month 取 13）走 service 的 bad_request 通道，出参是 400 配 BAD_REQUEST，文案是
 * backend/src/utils/messages.rs 的脱敏常量「请求参数错误」而非 service 原文。
 * 同族说明见 backend/src/handlers/purchase_receipt_handler.rs 的日期解析函数注释。
 *
 * 假绿防线。每个用例先为本 worker 另建专属供应商再在其名下造数，使按 supplier_id 过滤的
 * 聚合出口只有我自己的行，绝对值可直接断言，不靠全表漂移数凑数；每次写后一律独立读回
 * 逐字段对表；被拒建单用「该供应商名下应付单计数」反查零落库；金额一律按 Decimal 字符串
 * 折分位比较（出参标度超出两位且尾数非零即抛错，不静默舍入）；桶标签、状态词与机器码与
 * 后端常量表逐字符同源（DRAFT/AUDITED/CANCELLED）；已取消与已审核票没有删除腿这一事实，
 * 按用例内的状态门拒绝与读回原文钉出，不打空清理；全程不用可选链与兜底运算符，
 * 必出的键缺失即判红。
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

/** 后端账龄桶标签全集（与 ap_invoice_ops/report.rs 的 classify_aging_bucket 六档逐字同源） */
const AGING_BUCKET_ORDER = [
  '未到期',
  '逾期 1-30 天',
  '逾期 31-60 天',
  '逾期 61-90 天',
  '逾期 91-180 天',
  '逾期 180 天以上',
] as const;

/**
 * Decimal 出参折到指定标度的整数刻度。
 * 调用方：本文件的 eqAmount 与跨桶求和断言。入参：出参原值（字符串为主、兼容数字）、
 * 目标标度、断言位置标签。传给谁：仅参与比较，不外发、不落库。
 * 出参标度超出目标位且尾数非零时直接抛错点名原值，不做四舍五入，避免把精度漂移
 * 掩盖成「数值相等」。
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

/** 两位小数金额字面量折分位（期望值侧唯一入口，禁止在用例里散落换算） */
function cents(want: string): bigint {
  return toScaled(want, 2, `期望金额 ${want}`);
}

/** 金额等值断言：实际出参折分位后与期望分位逐分对齐，失败时把两侧原值全部打出来 */
function eqAmount(actual: unknown, expectedCents: bigint, label: string): void {
  expect(
    toScaled(actual, 2, label),
    `${label}：期望(分)=${expectedCents} 实际=${JSON.stringify(actual)}`
  ).toBe(expectedCents);
}

/** 取正整数主键，缺失或非正即抛错点名步骤，不假定后端一定回显该键 */
function requireId(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isInteger(n) || n <= 0) {
    throw new Error(`${label}：无有效整数 id，raw=${JSON.stringify(v)}`);
  }
  return n;
}

/** 取非负整数计数（张数与占比位数用），非整数即抛错而不是当作 0 */
function requireCount(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isInteger(n) || n < 0) {
    throw new Error(`${label}：计数非整数，raw=${JSON.stringify(v)}`);
  }
  return n;
}

/**
 * 相对今天的 UTC 日历日字符串（YYYY-MM-DD）。
 * 后端账龄与日报的「当天」取 Utc::now 的 naive_utc 日期，故用例侧同日基准必须按 UTC 计算，
 * 用本地时区会在跨日窗口造出差一天的桶归属。入参 offsetDays 为相对今天的整天数（负为过去）。
 */
function utcDate(offsetDays: number): string {
  const ms = Date.now() + offsetDays * 86_400_000;
  return new Date(ms).toISOString().slice(0, 10);
}

/** 从 YYYY-MM-DD 拆年月（月报入参用，避免把 Date 的零基月号带进断言） */
function yearMonthOf(dateStr: string): { year: number; month: number } {
  const parts = dateStr.split('-');
  return { year: Number(parts[0]), month: Number(parts[1]) };
}

/**
 * 为本用例新建专属供应商并返回主键。
 * 调用方：本文件全部用例的第一步。入参：tag 为本用例唯一标记，写进供应商名称。
 * 传给谁：POST /purchase/suppliers，返回的 id 随后作应付单 supplier_id 与所有聚合出口的过滤键。
 * 存什么：供应商名称、简称与联系电话；存哪里：purchase 域 suppliers 表。
 * 不复用 ctx.supplierId 的原因：同一 worker 内共用供应商会被前一个用例的造数污染，
 * 聚合口径就退化成增量比较；专属供应商让绝对值断言成立。
 */
async function createDedicatedSupplier(page: Page, tag: string): Promise<number> {
  const created = await apiCallRaw<Row>(page, 'POST', '/purchase/suppliers', {
    supplier_name: `E2E应付160-${tag}`,
    supplier_short_name: `E160${tag.slice(-4)}`,
    contact_phone: '13800000001',
  });
  return requireId(created.id, `专属供应商 ${tag} 创建`);
}

interface InvoiceSeed {
  amount: string;
  taxAmount: string;
  invoiceDate: string;
  dueDate: string;
  notes: string;
}

/** 手工建单（走 create_manual 的六要素必填口径，缺键不兜底），返回主表整行 */
async function createInvoice(page: Page, supplierId: number, seed: InvoiceSeed): Promise<Row> {
  const created = await apiCallRaw<Row>(page, 'POST', '/ap/invoices', {
    supplier_id: supplierId,
    invoice_type: 'PURCHASE',
    invoice_date: seed.invoiceDate,
    due_date: seed.dueDate,
    amount: seed.amount,
    tax_amount: seed.taxAmount,
    notes: seed.notes,
  });
  return { ...created, id: requireId(created.id, `建应付单 ${seed.notes}`) };
}

/** 审核应付单并读回确认状态推进；审核前后各读一次，用来钉「状态门拒绝无副作用」 */
async function approveInvoice(page: Page, id: number): Promise<Row> {
  await apiCall(page, 'POST', `/ap/invoices/${id}/approve`);
  const after = await apiCallRaw<Row>(page, 'GET', `/ap/invoices/${id}`);
  expect(String(after.invoice_status), `应付单 ${id} 审核回读状态`).toBe('AUDITED');
  return after;
}

/** GET 余额出口：data 即 BalanceSummary 四键（应付、已付、未付、张数） */
async function readBalance(page: Page, supplierId: number): Promise<Row> {
  const res = await apiCall<Row>(page, 'GET', `/ap/invoices/balance?supplier_id=${supplierId}`);
  expect(typeof res.code, `余额出口 code 类型，实际 ${JSON.stringify(res.code)}`).toBe('number');
  expect(res.code, '余额出口成功码必须为数字 200').toBe(200);
  expect(
    'total' in res,
    `余额出口顶层不得带 total（分页元数据只落在 data 内），实际键 ${JSON.stringify(Object.keys(res))}`
  ).toBe(false);
  return assertSuccessEnvelope(res);
}

/** GET 账龄出口：data 为 AgingAnalysisItem 数组，只含有数据的桶（可能为空数组） */
async function readInvoiceAging(page: Page, supplierId: number): Promise<Row[]> {
  const res = await apiCall<Row[]>(page, 'GET', `/ap/invoices/aging?supplier_id=${supplierId}`);
  expect(res.code, '账龄出口成功码必须为数字 200').toBe(200);
  expect(
    'total' in res,
    `账龄出口顶层不得带 total，实际键 ${JSON.stringify(Object.keys(res))}`
  ).toBe(false);
  const data = assertSuccessEnvelope(res);
  expect(Array.isArray(data), `账龄出口 data 必须是数组，实际 ${JSON.stringify(data)}`).toBe(true);
  return data;
}

/** GET 综合统计出口：data 为 ApInvoiceStatistics 三块（余额、账龄数组、状态分布数组） */
async function readInvoiceStatistics(page: Page, supplierId: number): Promise<Row> {
  const res = await apiCall<Row>(page, 'GET', `/ap/invoices/statistics?supplier_id=${supplierId}`);
  expect(res.code, '统计出口成功码必须为数字 200').toBe(200);
  expect(
    'total' in res,
    `统计出口顶层不得带 total，实际键 ${JSON.stringify(Object.keys(res))}`
  ).toBe(false);
  return assertSuccessEnvelope(res);
}

/** 在桶或分组数组里按标签键取恰一行，取不到或取多即抛错点名标签，不用可选链把缺桶当成 0 */
function pickRowByLabel(rows: Row[], wanted: string, labelKey: string, label: string): Row {
  const hit = rows.filter(r => String(r[labelKey]) === wanted);
  expect(
    hit.length,
    `${label}：${labelKey}=${wanted} 必须恰有一条，实际 ${JSON.stringify(rows)}`
  ).toBe(1);
  return hit[0];
}

/**
 * 断言成功信封形态并取出 data。
 * 入参：apiCall 的整封响应（数字 code、data、可选 message、可选顶层 total）。
 * 判据：code 必须是数字 200；顶层出现 total 即出参形状漂移（backend/src/utils/response.rs 的
 * ApiResponse 在 success 构造下 total 恒为 None 并被 skip_serializing_if 摘除，
 * 分页 total 只允许落在 data 内的 PaginatedResponse 里）。
 */
function assertSuccessEnvelope<T>(res: { code: number; data: T; total?: number }): T {
  expect(typeof res.code, `成功信封 code 类型，实际 ${JSON.stringify(res.code)}`).toBe('number');
  expect(res.code, '成功信封 code 必须为数字 200').toBe(200);
  expect(
    'total' in res,
    `成功信封顶层不得带 total，实际键 ${JSON.stringify(Object.keys(res))}`
  ).toBe(false);
  expect(res.data === undefined, '成功信封必须带 data 键').toBe(false);
  return res.data;
}

test.describe.serial('160 应付余额账龄统计与四张应付报表契约链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('160-01 建单六要素必填门与精度门拒绝后零落库，草稿态被余额与账龄两口排除而统计第三块照计', async ({
    page,
  }) => {
    const tag = genCode('E160A');
    const supplierId = await createDedicatedSupplier(page, tag);
    const today = utcDate(0);

    // 缺税额：create_manual 的点名文案原文外显（可外显校验变体），HTTP 400 配 VALIDATION_ERROR
    const missingTax = await apiCallExpectFail(page, 'POST', '/ap/invoices', {
      supplier_id: supplierId,
      invoice_type: 'PURCHASE',
      invoice_date: today,
      due_date: today,
      amount: '100.00',
      notes: `缺税额${tag}`,
    });
    expect(missingTax.status, `缺税额建单应 400，实际 ${JSON.stringify(missingTax)}`).toBe(400);
    expect(failureCode(missingTax), '缺税额建单机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    expect(
      missingTax.message,
      `缺税额文案必须逐字等于后端点名串，实际 ${JSON.stringify(missingTax.message)}`
    ).toBe('应付单缺少税额，无进项税请显式提交 0');

    // 金额精度三位小数：精度门先于缺键检查，故六要素齐全仍按精度拒绝
    const overScale = await apiCallExpectFail(page, 'POST', '/ap/invoices', {
      supplier_id: supplierId,
      invoice_type: 'PURCHASE',
      invoice_date: today,
      due_date: today,
      amount: '100.001',
      tax_amount: '13.00',
      notes: `超精度${tag}`,
    });
    expect(overScale.status, `超精度建单应 400，实际 ${JSON.stringify(overScale)}`).toBe(400);
    expect(failureCode(overScale), '超精度建单机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    expect(overScale.message, `超精度文案实际 ${JSON.stringify(overScale.message)}`).toBe(
      '应付单金额精度不能超过 2 位小数'
    );

    // 两笔被拒建单零落库：专属供应商名下计数为 0，绝对值可直接断言
    const emptyList = await apiCall<Row>(
      page,
      'GET',
      `/ap/invoices?supplier_id=${supplierId}&page=1&page_size=20`
    );
    const emptyData = assertSuccessEnvelope(emptyList);
    expect(requireCount(emptyData.total, '被拒建单后的列表 total')).toBe(0);
    expect(Array.isArray(emptyData.items), '分页 data.items 必须是数组').toBe(true);

    // 两张草稿票：金额 3000 的到期日已过（落逾期 1-30 天），金额 1000 的到期日未至
    const draftA = await createInvoice(page, supplierId, {
      amount: '3000.00',
      taxAmount: '390.00',
      invoiceDate: today,
      dueDate: utcDate(-10),
      notes: `${tag}-A`,
    });
    const draftB = await createInvoice(page, supplierId, {
      amount: '1000.00',
      taxAmount: '0.00',
      invoiceDate: today,
      dueDate: utcDate(30),
      notes: `${tag}-B`,
    });
    const draftIdA = requireId(draftA.id, '草稿票 A');

    // 建单初值逐列回读：单号形态、状态、派生金额、来源标识与服务端默认币种汇率
    expect(
      /^API\d{11}$/.test(String(draftA.invoice_no)),
      `建单单号必须是 API 加八位日期加流水，实际 ${String(draftA.invoice_no)}`
    ).toBe(true);
    expect(String(draftA.invoice_status), '建单初值状态必须为 DRAFT').toBe('DRAFT');
    expect(String(draftA.source_type), '手工建单 source_type 必须写 MANUAL').toBe('MANUAL');
    eqAmount(draftA.paid_amount, cents('0.00'), '建单已付初值');
    eqAmount(draftA.unpaid_amount, cents('3000.00'), '建单未付派生值');
    expect(String(draftA.currency), '币种缺省走服务端默认而非请求原值').toBe('CNY');
    expect(Number(draftA.exchange_rate), '本位币汇率缺省必须为 1').toBe(1);
    expect(String(draftA.supplier_id), '供应商外键必须回显专属供应商').toBe(String(supplierId));

    // 草稿排除门：余额三口为零、账龄出口为空数组
    const balanceDraft = await readBalance(page, supplierId);
    eqAmount(balanceDraft.total_invoice_amount, cents('0.00'), '草稿期余额-应付');
    eqAmount(balanceDraft.total_paid_amount, cents('0.00'), '草稿期余额-已付');
    eqAmount(balanceDraft.total_unpaid_amount, cents('0.00'), '草稿期余额-未付');
    expect(requireCount(balanceDraft.invoice_count, '草稿期余额张数')).toBe(0);

    const agingDraft = await readInvoiceAging(page, supplierId);
    expect(agingDraft.length, `草稿期账龄必须空数组，实际 ${JSON.stringify(agingDraft)}`).toBe(0);

    // 统计出口第三块不排除草稿：前两块与两个单口同为空，状态分布却计入两张草稿共 4000
    const statistics = await readInvoiceStatistics(page, supplierId);
    const draftBalanceBlock = statistics.balance_summary as Row;
    expect(requireCount(draftBalanceBlock.invoice_count, '统计内余额张数')).toBe(0);
    eqAmount(draftBalanceBlock.total_unpaid_amount, cents('0.00'), '统计内余额未付');
    const draftAgingBlock = statistics.aging_analysis as Row[];
    expect(Array.isArray(draftAgingBlock), '统计内账龄块必须是数组').toBe(true);
    expect(draftAgingBlock.length, '统计内账龄块与账龄出口同为空（草稿被两口排除）').toBe(0);
    const draftStatusRows = statistics.status_distribution as Row[];
    expect(
      draftStatusRows.length,
      `状态分布必须恰一行 DRAFT，实际 ${JSON.stringify(draftStatusRows)}`
    ).toBe(1);
    expect(String(draftStatusRows[0].status), '状态分布行状态必须为 DRAFT').toBe('DRAFT');
    expect(requireCount(draftStatusRows[0].invoice_count, '状态分布行张数')).toBe(2);
    eqAmount(draftStatusRows[0].total_amount, cents('4000.00'), '状态分布行金额');

    // 草稿的删除腿真实存在：删一张后详情读回 404，列表只剩另一张
    await apiCall(page, 'DELETE', `/ap/invoices/${draftIdA}`);
    const gone = await apiCallExpectFail(page, 'GET', `/ap/invoices/${draftIdA}`);
    expect(gone.status, `已删草稿详情应 404，实际 ${JSON.stringify(gone)}`).toBe(404);
    expect(failureCode(gone), '已删草稿详情机器码应为 NOT_FOUND').toBe(APP_ERROR_CODES.NOT_FOUND);
    const afterDelete = assertSuccessEnvelope(
      await apiCall<Row>(page, 'GET', `/ap/invoices?supplier_id=${supplierId}&page=1&page_size=20`)
    );
    expect(requireCount(afterDelete.total, '删一张后的列表 total')).toBe(1);
    const leftRow = (afterDelete.items as Row[])[0];
    expect(String(leftRow.id), '剩余行必须是未被删除的那张').toBe(
      String(requireId(draftB.id, '草稿票 B'))
    );
  });

  test('160-02 审核后余额与账龄与统计三口互证，桶标签与张数金额逐字段对齐', async ({ page }) => {
    const tag = genCode('E160B');
    const supplierId = await createDedicatedSupplier(page, tag);
    const today = utcDate(0);

    const invA = await createInvoice(page, supplierId, {
      amount: '3000.00',
      taxAmount: '390.00',
      invoiceDate: today,
      dueDate: utcDate(-10),
      notes: `${tag}-A`,
    });
    const invB = await createInvoice(page, supplierId, {
      amount: '1000.00',
      taxAmount: '0.00',
      invoiceDate: today,
      dueDate: utcDate(30),
      notes: `${tag}-B`,
    });
    const idA = requireId(invA.id, '票 A');
    const idB = requireId(invB.id, '票 B');
    const approvedA = await approveInvoice(page, idA);
    await approveInvoice(page, idB);

    // 余额出口：两张有效票合计 4000，已付 0，未付 4000
    const balance = await readBalance(page, supplierId);
    expect(requireCount(balance.invoice_count, '余额张数')).toBe(2);
    eqAmount(balance.total_invoice_amount, cents('4000.00'), '余额-应付合计');
    eqAmount(balance.total_paid_amount, cents('0.00'), '余额-已付合计');
    eqAmount(balance.total_unpaid_amount, cents('4000.00'), '余额-未付合计');

    // 账龄出口：只有两桶出现，桶标签与后端分档文案逐字一致
    const aging = await readInvoiceAging(page, supplierId);
    expect(aging.length, `账龄必须恰两桶，实际 ${JSON.stringify(aging)}`).toBe(2);
    const bucketOverdue = pickRowByLabel(aging, '逾期 1-30 天', 'aging_bucket', '账龄逾期桶');
    const bucketNotDue = pickRowByLabel(aging, '未到期', 'aging_bucket', '账龄未到期桶');
    expect(requireCount(bucketOverdue.invoice_count, '逾期桶张数')).toBe(1);
    eqAmount(bucketOverdue.total_amount, cents('3000.00'), '逾期桶金额');
    expect(requireCount(bucketNotDue.invoice_count, '未到期桶张数')).toBe(1);
    eqAmount(bucketNotDue.total_amount, cents('1000.00'), '未到期桶金额');

    // 跨出口恒等式：账龄各桶金额合计必须等于余额出口的未付合计
    const agingSum = aging.reduce(
      (acc, b) => acc + toScaled(b.total_amount, 2, '账龄桶金额求和'),
      0n
    );
    eqAmount(balance.total_unpaid_amount, agingSum, '余额未付合计必须等于账龄桶金额合计');

    // 统计出口：前两块必须与两个单口出参整块相等（同一 service 函数的不同出口）
    const statistics = await readInvoiceStatistics(page, supplierId);
    expect(statistics.balance_summary, '统计内余额块必须等于余额出口整块').toEqual(balance);
    expect(statistics.aging_analysis, '统计内账龄块必须等于账龄出口整块').toEqual(aging);
    const auditedRows = (statistics.status_distribution as Row[]).filter(
      r => String(r.status) === 'AUDITED'
    );
    expect(
      auditedRows.length,
      `状态分布必须恰一行 AUDITED，实际 ${JSON.stringify(statistics.status_distribution)}`
    ).toBe(1);
    expect(requireCount(auditedRows[0].invoice_count, 'AUDITED 行张数')).toBe(2);
    eqAmount(auditedRows[0].total_amount, cents('4000.00'), 'AUDITED 行金额');

    // 审核写两列并保留建单身份列：审批人与审批时间落值、created_by 仍是建单人
    const me = await apiCallRaw<Row>(page, 'GET', '/auth/me');
    expect(String(approvedA.approved_by), '审核回读审批人必须是本 worker 会话用户').toBe(
      String(me.id)
    );
    expect(
      typeof approvedA.approved_at === 'string' &&
        !Number.isNaN(Date.parse(String(approvedA.approved_at))),
      `审批时间必须是可解析时间串，实际 ${JSON.stringify(approvedA.approved_at)}`
    ).toBe(true);
    expect(String(approvedA.created_by), '审核不得覆盖建单人列').toBe(String(me.id));
    expect(String(approvedA.notes), '审核不得改写备注列').toBe(`${tag}-A`);
    expect(String(approvedA.due_date), '审核不得改写到期日列').toBe(utcDate(-10));

    // 重复审核被状态门拒（仅 DRAFT 可审核），且拒绝无副作用：状态与审批时间不被二次写
    const twice = await apiCallExpectFail(page, 'POST', `/ap/invoices/${idA}/approve`);
    expect(twice.status, `重复审核应 400，实际 ${JSON.stringify(twice)}`).toBe(400);
    expect(failureCode(twice), '重复审核机器码应为 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    expect(twice.message, 'BusinessError 走脱敏站点，出参文案必须是固定常量').toBe('业务处理失败');
    const unchanged = await apiCallRaw<Row>(page, 'GET', `/ap/invoices/${idA}`);
    expect(String(unchanged.invoice_status), '重复审核后状态仍 AUDITED').toBe('AUDITED');
    expect(String(unchanged.approved_at), '重复审核后审批时间不被覆盖').toBe(
      String(approvedA.approved_at)
    );
    // 残留口径：删除门只放行 DRAFT（backend/src/services/ap_invoice_ops/crud.rs），
    // AUDITED 夹具票在 HTTP 面上无删除通道，本用例如实不清理，靠专属供应商与唯一 tag 隔离
  });

  test('160-03 报表账龄恒六桶并按序补零，占比算法与基准日迁移和单口账龄互证', async ({ page }) => {
    const tag = genCode('E160C');
    const supplierId = await createDedicatedSupplier(page, tag);
    const today = utcDate(0);

    const invA = await createInvoice(page, supplierId, {
      amount: '3000.00',
      taxAmount: '390.00',
      invoiceDate: today,
      dueDate: utcDate(-10),
      notes: `${tag}-A`,
    });
    const invB = await createInvoice(page, supplierId, {
      amount: '1000.00',
      taxAmount: '0.00',
      invoiceDate: today,
      dueDate: utcDate(30),
      notes: `${tag}-B`,
    });
    await approveInvoice(page, requireId(invA.id, '票 A'));
    await approveInvoice(page, requireId(invB.id, '票 B'));

    // 默认基准日（当天）：六桶恒在且按固定序输出，两桶有值四桶补零
    const agingReport = await apiCallRaw<Row>(
      page,
      'GET',
      `/ap/reports/aging?supplier_id=${supplierId}`
    );
    expect(
      agingReport.report_date,
      `报表日期默认取当天，实际 ${JSON.stringify(agingReport.report_date)}`
    ).toBe(today);
    expect(String(agingReport.supplier_id), '供应商过滤键必须回显').toBe(String(supplierId));
    eqAmount(agingReport.total_unpaid_amount, cents('4000.00'), '账龄报表未付合计');
    const buckets = agingReport.aging_buckets as Row[];
    expect(buckets.length, `报表账龄必须恒六桶（含零值桶），实际 ${JSON.stringify(buckets)}`).toBe(
      6
    );
    expect(
      buckets.map(b => String(b.bucket_name)),
      '六桶名称与顺序必须与后端分档常量逐字同源'
    ).toEqual([...AGING_BUCKET_ORDER]);

    const bOverdue = pickRowByLabel(buckets, '逾期 1-30 天', 'bucket_name', '报表逾期桶');
    const bNotDue = pickRowByLabel(buckets, '未到期', 'bucket_name', '报表未到期桶');
    expect(requireCount(bOverdue.invoice_count, '报表逾期桶张数')).toBe(1);
    eqAmount(bOverdue.total_amount, cents('3000.00'), '报表逾期桶金额');
    expect(Number(bOverdue.percentage), '逾期桶占比必须为 75').toBe(75);
    expect(requireCount(bNotDue.invoice_count, '报表未到期桶张数')).toBe(1);
    eqAmount(bNotDue.total_amount, cents('1000.00'), '报表未到期桶金额');
    expect(Number(bNotDue.percentage), '未到期桶占比必须为 25').toBe(25);
    for (const name of ['逾期 31-60 天', '逾期 61-90 天', '逾期 91-180 天', '逾期 180 天以上']) {
      const zeroBucket = pickRowByLabel(buckets, name, 'bucket_name', '报表零值桶');
      expect(requireCount(zeroBucket.invoice_count, `零值桶 ${name} 张数`)).toBe(0);
      eqAmount(zeroBucket.total_amount, cents('0.00'), `零值桶 ${name} 金额`);
      eqAmount(zeroBucket.percentage, cents('0.00'), `零值桶 ${name} 占比`);
    }
    const pctSum = buckets.reduce((acc, b) => acc + Number(b.percentage), 0);
    expect(pctSum, `六桶占比合计必须为 100，实际 ${pctSum}`).toBe(100);

    // 报表桶与单口账龄桶同源互证：张数与金额逐桶相等（两套实现同一事实）
    const invoiceAging = await readInvoiceAging(page, supplierId);
    for (const name of ['逾期 1-30 天', '未到期']) {
      const fromReport = pickRowByLabel(buckets, name, 'bucket_name', '报表侧桶');
      const fromSingle = pickRowByLabel(invoiceAging, name, 'aging_bucket', '单口侧桶');
      expect(String(fromReport.invoice_count), `两侧 ${name} 张数必须同源`).toBe(
        String(fromSingle.invoice_count)
      );
      eqAmount(
        fromReport.total_amount,
        toScaled(fromSingle.total_amount, 2, `单口 ${name} 金额`),
        `两侧 ${name} 金额必须同源`
      );
    }

    // 显式基准日迁移分桶：基准推到 T+25（不越过票 B 到期日 T+30），票 A 逾期 35 天滑到 31-60 天桶，原 1-30 桶整桶归零
    const baseline = utcDate(25);
    const shifted = await apiCallRaw<Row>(
      page,
      'GET',
      `/ap/reports/aging?supplier_id=${supplierId}&baseline_date=${baseline}`
    );
    expect(String(shifted.report_date), '基准日入参必须回显为报表日期').toBe(baseline);
    const shiftedBuckets = shifted.aging_buckets as Row[];
    expect(
      shiftedBuckets.length,
      `显式基准日下仍必须恒六桶，实际 ${JSON.stringify(shiftedBuckets)}`
    ).toBe(6);
    const shiftedOverdue = pickRowByLabel(
      shiftedBuckets,
      '逾期 31-60 天',
      'bucket_name',
      '迁移后逾期桶'
    );
    expect(requireCount(shiftedOverdue.invoice_count, '迁移后逾期桶张数')).toBe(1);
    eqAmount(shiftedOverdue.total_amount, cents('3000.00'), '迁移后逾期桶金额');
    const shiftedCleared = pickRowByLabel(
      shiftedBuckets,
      '逾期 1-30 天',
      'bucket_name',
      '迁移后原桶'
    );
    expect(requireCount(shiftedCleared.invoice_count, '基准日前移后原逾期桶张数必须归零')).toBe(0);
    eqAmount(shiftedCleared.total_amount, cents('0.00'), '基准日前移后原逾期桶金额必须归零');
    const shiftedNotDue = pickRowByLabel(shiftedBuckets, '未到期', 'bucket_name', '迁移后未到期桶');
    expect(requireCount(shiftedNotDue.invoice_count, '迁移后未到期桶张数')).toBe(1);
    eqAmount(shiftedNotDue.total_amount, cents('1000.00'), '迁移后未到期桶金额');
    eqAmount(shifted.total_unpaid_amount, cents('4000.00'), '基准日不改未付合计');
    // 残留口径：AUDITED 夹具票无删除通道（删除门只放行 DRAFT），本用例如实不清理，
    // 聚合出口按专属供应商过滤，唯一 tag 保证不与其它用例互污。
    // 防复发：基准日窗口不得覆盖非目标票的到期日，否则未到期票会被拖进逾期桶
  });

  test('160-04 应付日报按 invoice_date 与 due_date 精确命中，缺参与非法日期走抽取器拒绝收口为 VALIDATION_ERROR 信封', async ({
    page,
  }) => {
    const tag = genCode('E160D');
    const supplierId = await createDedicatedSupplier(page, tag);
    const today = utcDate(0);
    const dueDay = utcDate(15);

    const inv = await createInvoice(page, supplierId, {
      amount: '500.00',
      taxAmount: '65.00',
      invoiceDate: today,
      dueDate: dueDay,
      notes: `${tag}-A`,
    });
    const invoiceId = requireId(inv.id, '日报夹具票');

    // 草稿期：新增门与到期门都挂着 CANCELLED 与 DRAFT 的排除集合，故日报全零
    const dailyDraft = await apiCallRaw<Row>(
      page,
      'GET',
      `/ap/reports/daily?report_date=${today}&supplier_id=${supplierId}`
    );
    expect(String(dailyDraft.report_date), '日报日期必须回显入参').toBe(today);
    expect(String(dailyDraft.supplier_id), '日报供应商键必须回显入参').toBe(String(supplierId));
    expect(requireCount(dailyDraft.new_invoice_count, '草稿期新增张数')).toBe(0);
    eqAmount(dailyDraft.new_invoice_amount, cents('0.00'), '草稿期新增金额');

    await approveInvoice(page, invoiceId);

    // 审核后：新增门按 invoice_date 等值命中，到期与付款两门当日仍为零
    const daily = await apiCallRaw<Row>(
      page,
      'GET',
      `/ap/reports/daily?report_date=${today}&supplier_id=${supplierId}`
    );
    expect(requireCount(daily.new_invoice_count, '新增张数')).toBe(1);
    eqAmount(daily.new_invoice_amount, cents('500.00'), '新增金额');
    expect(requireCount(daily.due_invoice_count, '建单日到期张数')).toBe(0);
    eqAmount(daily.due_invoice_amount, cents('0.00'), '建单日到期金额');
    expect(requireCount(daily.payment_count, '当日付款张数（本供应商无付款单）')).toBe(0);
    eqAmount(daily.payment_amount, cents('0.00'), '当日付款金额');

    // 到期门命中日：report_date 推到到期日，新增归零、到期亮起（两门各自独立）
    const dailyDueDay = await apiCallRaw<Row>(
      page,
      'GET',
      `/ap/reports/daily?report_date=${dueDay}&supplier_id=${supplierId}`
    );
    expect(requireCount(dailyDueDay.new_invoice_count, '到期日当天新增张数必须回零')).toBe(0);
    expect(requireCount(dailyDueDay.due_invoice_count, '到期日张数')).toBe(1);
    eqAmount(dailyDueDay.due_invoice_amount, cents('500.00'), '到期金额取未付额');

    // 等值门而非区间门：前一日不得把今日建单算进去
    const dailyPrev = await apiCallRaw<Row>(
      page,
      'GET',
      `/ap/reports/daily?report_date=${utcDate(-1)}&supplier_id=${supplierId}`
    );
    expect(requireCount(dailyPrev.new_invoice_count, '前一日的新增张数')).toBe(0);
    eqAmount(dailyPrev.new_invoice_amount, cents('0.00'), '前一日的新增金额');

    // 缺 report_date：非 Option 的日期字段在 Query 抽取层反序列化失败，
    // 拒绝体已被 trace_context 收口为统一信封：400 + VALIDATION_ERROR + 固定公开规则文案
    const missingDate = await apiCallExpectFail(page, 'GET', '/ap/reports/daily');
    expect(missingDate.status, `缺 report_date 应 400，实际 ${JSON.stringify(missingDate)}`).toBe(
      400
    );
    expect(
      failureCode(missingDate),
      `缺参拒绝机器码应为 VALIDATION_ERROR，实际 ${JSON.stringify(missingDate)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      missingDate.message,
      `缺参拒绝文案应为固定公开规则串，实际 ${JSON.stringify(missingDate.message)}`
    ).toBe(
      '请求格式不正确：请求体或查询参数存在缺失的必填字段、字段类型错误或内容类型错误，请修正后重试'
    );

    // 非法日期串同样死在反序列化层（月份十三无法解析为日期），收口后同为 VALIDATION_ERROR 信封
    const badDate = await apiCallExpectFail(
      page,
      'GET',
      '/ap/reports/daily?report_date=2001-13-01'
    );
    expect(badDate.status, `非法日期应 400，实际 ${JSON.stringify(badDate)}`).toBe(400);
    expect(failureCode(badDate), '非法日期机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    expect(
      badDate.message,
      `非法日期文案应为固定公开规则串，实际 ${JSON.stringify(badDate.message)}`
    ).toBe(
      '请求格式不正确：请求体或查询参数存在缺失的必填字段、字段类型错误或内容类型错误，请修正后重试'
    );
  });

  test('160-05 统计报表期间与分组同主聚合互证，月报月界与期初期末，非法月份走 BAD_REQUEST 信封', async ({
    page,
  }) => {
    const tag = genCode('E160E');
    const supplierId = await createDedicatedSupplier(page, tag);
    const today = utcDate(0);
    const periodStart = utcDate(-1);
    const periodEnd = utcDate(1);

    const invA = await createInvoice(page, supplierId, {
      amount: '3000.00',
      taxAmount: '390.00',
      invoiceDate: today,
      dueDate: utcDate(-10),
      notes: `${tag}-A`,
    });
    const invB = await createInvoice(page, supplierId, {
      amount: '1000.00',
      taxAmount: '0.00',
      invoiceDate: today,
      dueDate: utcDate(30),
      notes: `${tag}-B`,
    });
    await approveInvoice(page, requireId(invA.id, '票 A'));
    await approveInvoice(page, requireId(invB.id, '票 B'));

    // 统计报表：期间回显、主聚合九项、两块分组的合计与主聚合互证
    const report = await apiCallRaw<Row>(
      page,
      'GET',
      `/ap/reports/statistics?supplier_id=${supplierId}&start_date=${periodStart}&end_date=${periodEnd}`
    );
    expect(String(report.period_start), '期间起必须回显入参').toBe(periodStart);
    expect(String(report.period_end), '期间止必须回显入参').toBe(periodEnd);
    expect(requireCount(report.total_invoice_count, '统计报表应付张数')).toBe(2);
    eqAmount(report.total_invoice_amount, cents('4000.00'), '统计报表应付合计');
    eqAmount(report.total_paid_amount, cents('0.00'), '统计报表已付合计');
    eqAmount(report.total_unpaid_amount, cents('4000.00'), '统计报表未付合计');
    expect(requireCount(report.paid_invoice_count, '已付清张数')).toBe(0);
    expect(requireCount(report.partial_paid_count, '部分付款张数')).toBe(0);
    expect(requireCount(report.unpaid_count, '未付款张数')).toBe(2);
    expect(requireCount(report.overdue_count, '逾期张数（仅到期已过的一张）')).toBe(1);
    eqAmount(report.overdue_amount, cents('3000.00'), '逾期金额取未付额');

    const byStatus = report.by_status as Row[];
    expect(byStatus.length, `按状态分组必须恰一行 AUDITED，实际 ${JSON.stringify(byStatus)}`).toBe(
      1
    );
    expect(String(byStatus[0].status), '状态分组行状态').toBe('AUDITED');
    expect(requireCount(byStatus[0].count, '状态分组张数')).toBe(2);
    eqAmount(byStatus[0].amount, cents('4000.00'), '状态分组金额');
    const statusCountSum = byStatus.reduce(
      (acc, r) => acc + requireCount(r.count, '状态分组张数求和'),
      0
    );
    expect(statusCountSum, '按状态分组张数合计必须等于主聚合总张数').toBe(
      requireCount(report.total_invoice_count, '主聚合总张数')
    );

    const byType = report.by_type as Row[];
    expect(byType.length, `按类型分组必须恰一行 PURCHASE，实际 ${JSON.stringify(byType)}`).toBe(1);
    expect(String(byType[0].invoice_type), '类型分组行键名与状态值').toBe('PURCHASE');
    expect(requireCount(byType[0].count, '类型分组张数')).toBe(2);
    eqAmount(byType[0].amount, cents('4000.00'), '类型分组金额');
    eqAmount(byType[0].unpaid_amount, cents('4000.00'), '类型分组未付金额');

    // 倒序区间：期间条件不成立即空集，计数归零而不报错（后端无区间顺序门）
    const reversed = await apiCallRaw<Row>(
      page,
      'GET',
      `/ap/reports/statistics?supplier_id=${supplierId}&start_date=${periodEnd}&end_date=${periodStart}`
    );
    expect(requireCount(reversed.total_invoice_count, '倒序区间张数必须为 0')).toBe(0);
    eqAmount(reversed.total_invoice_amount, cents('0.00'), '倒序区间金额必须为 0');
    expect(
      (reversed.by_status as Row[]).length,
      `倒序区间分组必须空数组，实际 ${JSON.stringify(reversed.by_status)}`
    ).toBe(0);

    // 缺 start_date：与日报同族，死在 Query 抽取层后被收口为统一信封：400 + VALIDATION_ERROR
    const missingStart = await apiCallExpectFail(page, 'GET', '/ap/reports/statistics');
    expect(missingStart.status, `缺期间起应 400，实际 ${JSON.stringify(missingStart)}`).toBe(400);
    expect(failureCode(missingStart), '缺期间起机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );

    // 月报：月界由年月推得，期初取月首前、期末取不晚于月末，两余额差即本月净额
    const { year, month } = yearMonthOf(today);
    const monthly = await apiCallRaw<Row>(
      page,
      'GET',
      `/ap/reports/monthly?year=${year}&month=${month}&supplier_id=${supplierId}`
    );
    expect(Number(monthly.year), '月报年份必须回显').toBe(year);
    expect(Number(monthly.month), '月报月份必须回显').toBe(month);
    expect(String(monthly.supplier_id), '月报供应商键必须回显').toBe(String(supplierId));
    const monthStart = `${year}-${String(month).padStart(2, '0')}-01`;
    const monthlyStats = monthly.statistics as Row;
    expect(String(monthlyStats.period_start), '月报内统计期间起必须是月首').toBe(monthStart);
    expect(requireCount(monthlyStats.total_invoice_count, '月报内统计张数')).toBe(2);
    eqAmount(monthly.opening_balance, cents('0.00'), '月初余额（本供应商月首前无票）');
    eqAmount(monthly.closing_balance, cents('4000.00'), '月末余额（本月未付合计）');
    eqAmount(
      monthlyStats.total_unpaid_amount,
      toScaled(monthly.closing_balance, 2, '月末余额'),
      '月末余额必须等于月内未付合计'
    );

    // 无数据年月：计数与两笔余额全零，不报错
    const emptyMonth = await apiCallRaw<Row>(
      page,
      'GET',
      `/ap/reports/monthly?year=1999&month=6&supplier_id=${supplierId}`
    );
    expect(Number(emptyMonth.year), '无数据月年份回显').toBe(1999);
    expect(Number(emptyMonth.month), '无数据月月份回显').toBe(6);
    expect(requireCount((emptyMonth.statistics as Row).total_invoice_count, '无数据月张数')).toBe(
      0
    );
    eqAmount(emptyMonth.opening_balance, cents('0.00'), '无数据月月初余额');
    eqAmount(emptyMonth.closing_balance, cents('0.00'), '无数据月月末余额');

    // month 取十三能过反序列化层，进 service 的月界计算被判无效日期：400 配 BAD_REQUEST，
    // 出参文案是 messages.rs 的 BAD_REQUEST_PUBLIC 脱敏常量，service 原文「无效的日期参数」不外显
    const badMonth = await apiCallExpectFail(
      page,
      'GET',
      `/ap/reports/monthly?year=${year}&month=13&supplier_id=${supplierId}`
    );
    expect(badMonth.status, `非法月份应 400，实际 ${JSON.stringify(badMonth)}`).toBe(400);
    expect(failureCode(badMonth), '非法月份机器码应为 BAD_REQUEST').toBe(
      APP_ERROR_CODES.BAD_REQUEST
    );
    expect(
      badMonth.message,
      `非法月份文案应为脱敏常量，实际 ${JSON.stringify(badMonth.message)}`
    ).toBe('请求参数错误');

    // 缺 year：整条查询串不合法，死在抽取层后被收口为统一信封：400 + VALIDATION_ERROR
    const missingYear = await apiCallExpectFail(page, 'GET', '/ap/reports/monthly?month=6');
    expect(missingYear.status, `缺年份应 400，实际 ${JSON.stringify(missingYear)}`).toBe(400);
    expect(failureCode(missingYear), '缺年份机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    // 残留口径：AUDITED 夹具票无删除通道（删除门只放行 DRAFT），本用例如实不清理，
    // 统计与月报出口按专属供应商过滤，唯一 tag 保证不与其它用例互污
  });

  test('160-06 取消票退出口径并留在库：余额与账龄剔除而状态分布照计，已取消与已审核票无删除腿', async ({
    page,
  }) => {
    const tag = genCode('E160F');
    const supplierId = await createDedicatedSupplier(page, tag);
    const today = utcDate(0);

    const invA = await createInvoice(page, supplierId, {
      amount: '3000.00',
      taxAmount: '390.00',
      invoiceDate: today,
      dueDate: utcDate(-10),
      notes: `${tag}-A`,
    });
    const invB = await createInvoice(page, supplierId, {
      amount: '1000.00',
      taxAmount: '0.00',
      invoiceDate: today,
      dueDate: utcDate(30),
      notes: `${tag}-B`,
    });
    const idA = requireId(invA.id, '票 A');
    const idB = requireId(invB.id, '票 B');
    await approveInvoice(page, idA);
    await approveInvoice(page, idB);

    // 取消一张（AUDITED 且未付才放行），三个取消列写入并读回
    const me = await apiCallRaw<Row>(page, 'GET', '/auth/me');
    const cancelReason = `${tag} 测试取消`;
    await apiCall(page, 'POST', `/ap/invoices/${idA}/cancel`, { reason: cancelReason });
    const cancelled = await apiCallRaw<Row>(page, 'GET', `/ap/invoices/${idA}`);
    expect(String(cancelled.invoice_status), '取消回读状态必须为 CANCELLED').toBe('CANCELLED');
    expect(String(cancelled.cancelled_by), '取消人列必须写会话用户').toBe(String(me.id));
    expect(String(cancelled.cancelled_reason), '取消原因列必须写提交值').toBe(cancelReason);
    expect(
      typeof cancelled.cancelled_at === 'string' &&
        !Number.isNaN(Date.parse(String(cancelled.cancelled_at))),
      `取消时间必须是可解析时间串，实际 ${JSON.stringify(cancelled.cancelled_at)}`
    ).toBe(true);

    // 余额与账龄两口把已取消剔除：只剩一张 1000 的未到期票
    const balance = await readBalance(page, supplierId);
    expect(requireCount(balance.invoice_count, '取消后余额张数')).toBe(1);
    eqAmount(balance.total_invoice_amount, cents('1000.00'), '取消后应付合计');
    eqAmount(balance.total_unpaid_amount, cents('1000.00'), '取消后未付合计');
    const aging = await readInvoiceAging(page, supplierId);
    expect(aging.length, `取消后账龄必须只剩未到期一桶，实际 ${JSON.stringify(aging)}`).toBe(1);
    expect(String(aging[0].aging_bucket), '取消后唯一桶必须是未到期').toBe('未到期');
    eqAmount(aging[0].total_amount, cents('1000.00'), '取消后桶金额');

    // 统计第三块不排除已取消：已审核与已取消两行并存，前两块与两个单口仍然相等
    const statistics = await readInvoiceStatistics(page, supplierId);
    expect(statistics.balance_summary, '取消后统计内余额块仍等于余额出口').toEqual(balance);
    expect(statistics.aging_analysis, '取消后统计内账龄块仍等于账龄出口').toEqual(aging);
    const statusRows = statistics.status_distribution as Row[];
    const cancelledRow = pickRowByLabel(statusRows, 'CANCELLED', 'status', '状态分布已取消行');
    expect(requireCount(cancelledRow.invoice_count, '已取消行张数')).toBe(1);
    eqAmount(cancelledRow.total_amount, cents('3000.00'), '已取消行金额');
    const auditedRow = pickRowByLabel(statusRows, 'AUDITED', 'status', '状态分布已审核行');
    expect(requireCount(auditedRow.invoice_count, '已审核行张数')).toBe(1);
    eqAmount(auditedRow.total_amount, cents('1000.00'), '已审核行金额');

    // 已取消票的三个门：不可重复取消、不可再审核、不可删除（删除腿只在草稿存在）
    const cancelTwice = await apiCallExpectFail(page, 'POST', `/ap/invoices/${idA}/cancel`, {
      reason: `${tag} 二次取消`,
    });
    expect(cancelTwice.status, `重复取消应 400，实际 ${JSON.stringify(cancelTwice)}`).toBe(400);
    expect(failureCode(cancelTwice), '重复取消机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const stillCancelled = await apiCallRaw<Row>(page, 'GET', `/ap/invoices/${idA}`);
    expect(String(stillCancelled.invoice_status), '重复取消不得改写状态').toBe('CANCELLED');
    expect(String(stillCancelled.cancelled_reason), '重复取消不得覆盖取消原因').toBe(
      `${tag} 测试取消`
    );

    const reApprove = await apiCallExpectFail(page, 'POST', `/ap/invoices/${idA}/approve`);
    expect(reApprove.status, `已取消票再审核应 400，实际 ${JSON.stringify(reApprove)}`).toBe(400);
    expect(failureCode(reApprove), '已取消票再审核机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    const deleteCancelled = await apiCallExpectFail(page, 'DELETE', `/ap/invoices/${idA}`);
    expect(
      deleteCancelled.status,
      `已取消票删除应 400（仅草稿可删），实际 ${JSON.stringify(deleteCancelled)}`
    ).toBe(400);
    expect(failureCode(deleteCancelled), '已取消票删除机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const stillThere = await apiCallRaw<Row>(page, 'GET', `/ap/invoices/${idA}`);
    expect(String(stillThere.id), '被拒删除后行必须仍在库').toBe(String(idA));
    expect(String(stillThere.invoice_status), '被拒删除不得改写状态').toBe('CANCELLED');

    // 已审核票同样无删除腿：本族审核后的票在 HTTP 面上没有终止通道，如实钉出
    const deleteAudited = await apiCallExpectFail(page, 'DELETE', `/ap/invoices/${idB}`);
    expect(deleteAudited.status, `已审核票删除应 400，实际 ${JSON.stringify(deleteAudited)}`).toBe(
      400
    );
    expect(failureCode(deleteAudited), '已审核票删除机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const auditedStill = await apiCallRaw<Row>(page, 'GET', `/ap/invoices/${idB}`);
    expect(String(auditedStill.invoice_status), '被拒删除后已审核票保持 AUDITED').toBe('AUDITED');
  });
});
