// 财务管理 E2E 套件 — 12 发票管理（端点 /finance/invoices*，页面 /invoice-details）
//
// 契约真值来源（全部以写入方为准）：
//   - 路由：routes/finance.rs 的 finance_payment_invoice_routes 挂 /invoices 的
//     GET/POST/GET{id}/PUT/DELETE 与 /{id}/approve、/{id}/verify，经 routes/mod.rs nest 到
//     /api/v1/erp/finance，故调用路径为 /finance/invoices*。
//   - 状态词表唯一真相＝写入方站点三处：create_invoice 落 pending、approve_invoice 落 approved、
//     verify_invoice 落 verified 并同批写 paid_date；approve 的状态门只放行 pending。
//     三个值均为小写，比较点一律引用本文件的 INVOICE_STATUS 常量，逐字符相同，禁止散写裸字面量。
//   - 前端中文文案词表：views/invoice-details/index.vue 的 statusTextMap
//     （pending 待审批 / approved 已审批 / verified 已核销 / rejected 已驳回），中文词表禁英文化，
//     UI 断言一律引用本文件的 INVOICE_STATUS_LABEL 映射。
//   - 金额标度：finance_invoices 三列是 DECIMAL(18,4)，create handler 先做 round_dp(2) 归一再落库，
//     故提交 3 位小数必须回读为 2 位值补零到 4 位标度的形态；本套件按 4 位标度逐字符比金额。
//   - 失败信封只有 AppError 一种形状：状态门走 AppError::business（脱敏，出参 message 恒为
//     utils/messages.rs 的权限族外常量 BUSINESS_PUBLIC），金额负值走 AppError::validation_displayable
//     （可外显真实文案）。两类都断恰 400 + 真实机器码 + message 逐字符，不断小于 500。
//   - 列表端点契约：出参 { invoices, total }，total 取本次返回的行数（handler 全量返回，无分页），
//     不属于 PaginatedResponse 一族，故同时硬断 total 与数组长度相等。
// 回读纪律：每步写后必 GET /finance/invoices/{id} 回读落库真值（状态 + 三金额 + paid_date +
//   未补录字段的 NULL 语义），不信任写请求的响应体；被拒的写请求做零变化回查（状态、金额、
//   行数都不动，且不得留下引用本例发票号的记录）。
import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCallRaw,
  apiCallExpectFail,
  tryCleanup,
  failureCode,
  APP_ERROR_CODES,
  genCode,
  BASE_URL,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 发票状态词表（写入方＝finance_invoice_service.rs 的三处落库站点，小写）。 */
const INVOICE_STATUS = {
  PENDING: 'pending',
  APPROVED: 'approved',
  VERIFIED: 'verified',
} as const;

/** 前端状态中文文案词表（写入方＝views/invoice-details/index.vue 的 statusTextMap）。 */
const INVOICE_STATUS_LABEL: Record<string, string> = {
  [INVOICE_STATUS.PENDING]: '待审批',
  [INVOICE_STATUS.APPROVED]: '已审批',
  [INVOICE_STATUS.VERIFIED]: '已核销',
};

/** 脱敏状态门站点的固定出参文案（utils/messages.rs BUSINESS_PUBLIC，BusinessError 分支）。 */
const SANITIZED_BUSINESS = '业务处理失败';

/** 可外显的金额负值校验文案（create handler 用 validation_displayable 装配，逐字符钉死）。 */
const NEGATIVE_AMOUNT_MESSAGE = '发票金额/税额/价税合计不能为负';

/** 表头列名（写入方＝views/invoice-details/index.vue 的 el-table-column label，中文逐字符）。 */
const INVOICE_COLUMN = {
  INVOICE_NO: '发票号',
  ORDER: '关联订单',
  AMOUNT: '发票金额',
  TAX: '税额',
  TOTAL: '价税合计',
  STATUS: '状态',
} as const;

/** 发票记录（键名取自 finance_invoice_handler.rs 的 InvoiceResponse，snake_case）。 */
type InvoiceRecord = Record<string, unknown>;

interface InvoiceListData {
  invoices: InvoiceRecord[];
  total: number;
}

/** 金额三件套的本例真值：提交 3 位小数，经 handler round_dp(2) 归一后应回读为 2 位值。 */
const SEEDED = {
  amount: 1000.234,
  taxAmount: 60.016,
  totalAmount: 1060.25,
  amountAtScale: '1000.2300',
  taxAmountAtScale: '60.0200',
  totalAmountAtScale: '1060.2500',
} as const;

/** 金额按 DECIMAL(18,4) 标度转字符串比较（Decimal 常以字符串回喂），非数值形态立即抛错不放宽。 */
function moneyAtScale(value: unknown, label: string): string {
  const n = typeof value === 'string' || typeof value === 'number' ? Number(value) : NaN;
  if (!Number.isFinite(n)) {
    throw new Error(`${label} 金额形态非数值（契约漂移？）：${JSON.stringify(value)}`);
  }
  return n.toFixed(4);
}

/** 取指定键（缺失即抛错并点名实际键，不用 ?? 0 兜底掩盖缺键）。 */
function pick(obj: InvoiceRecord, key: string, label: string): unknown {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺少后端真实键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  return obj[key];
}

/** 逐步回读并比对状态与金额三件套（每步写后的真值判据）。 */
function assertInvoiceTruth(
  rec: InvoiceRecord,
  ctx: string,
  status: string,
  opts: { paidDateNull: boolean }
): void {
  expect(String(pick(rec, 'status', ctx)), `${ctx}：状态应等于写入方词表值`).toBe(status);
  expect(
    moneyAtScale(pick(rec, 'amount', ctx), `${ctx}：发票金额`),
    `${ctx}：发票金额标度真值`
  ).toBe(SEEDED.amountAtScale);
  expect(moneyAtScale(pick(rec, 'tax_amount', ctx), `${ctx}：税额`), `${ctx}：税额标度真值`).toBe(
    SEEDED.taxAmountAtScale
  );
  expect(
    moneyAtScale(pick(rec, 'total_amount', ctx), `${ctx}：价税合计`),
    `${ctx}：价税合计标度真值`
  ).toBe(SEEDED.totalAmountAtScale);
  // 价税合计恒等：后端按提交值分别落三列，归一后仍须满足合计等于明细之和。
  const amount = Number(pick(rec, 'amount', ctx));
  const tax = Number(pick(rec, 'tax_amount', ctx));
  const total = Number(pick(rec, 'total_amount', ctx));
  expect(Math.abs(total - (amount + tax)), `${ctx}：价税合计应等于金额加税额`).toBeLessThan(0.005);
  if (opts.paidDateNull) {
    expect(pick(rec, 'paid_date', ctx), `${ctx}：未核销时 paid_date 应为 NULL`).toBeNull();
  }
}

/** 建一张发票并登记清理，返回后端落库记录（创建响应与 GET 回读同值由调用方另验）。 */
async function createInvoice(
  page: import('@playwright/test').Page,
  invoiceNo: string,
  amounts: { amount: number; tax_amount: number; total_amount: number }
): Promise<InvoiceRecord> {
  const created = await apiCallRaw<InvoiceRecord>(page, 'POST', '/finance/invoices', {
    invoice_no: invoiceNo,
    amount: amounts.amount,
    tax_amount: amounts.tax_amount,
    total_amount: amounts.total_amount,
  });
  const id = Number(created.id);
  if (!Number.isInteger(id) || id <= 0) {
    throw new Error(`建发票无有效 id：${JSON.stringify(created).slice(0, 200)}`);
  }
  CLEANUP.push({ path: `/finance/invoices/${id}`, label: 'finance_invoice' });
  return created;
}

/** GET 回读单张发票真值。 */
async function readInvoice(
  page: import('@playwright/test').Page,
  id: number
): Promise<InvoiceRecord> {
  return apiCallRaw<InvoiceRecord>(page, 'GET', `/finance/invoices/${id}`);
}

/** 推进发票到目标状态（走真实端点，逐步 GET 回读；每步都断状态与金额与 paid_date 真值），返回末态回读。 */
async function advanceTo(
  page: import('@playwright/test').Page,
  id: number,
  target: 'approved' | 'verified'
): Promise<InvoiceRecord> {
  await apiCallRaw<InvoiceRecord>(page, 'POST', `/finance/invoices/${id}/approve`);
  const approved = await readInvoice(page, id);
  assertInvoiceTruth(approved, '审批后', INVOICE_STATUS.APPROVED, { paidDateNull: true });
  if (target === 'approved') return approved;
  await apiCallRaw<InvoiceRecord>(page, 'POST', `/finance/invoices/${id}/verify`);
  const verified = await readInvoice(page, id);
  expect(String(pick(verified, 'status', '核销后')), '核销后状态应为 verified').toBe(
    INVOICE_STATUS.VERIFIED
  );
  expect(
    pick(verified, 'paid_date', '核销后'),
    '核销后 paid_date 应已由 NULL 落为真实时间'
  ).not.toBeNull();
  return verified;
}

/** 列表端点按发票号取行（列表为全量返回，发票号本例唯一，不受并发串扰）。 */
async function findListRow(
  page: import('@playwright/test').Page,
  invoiceNo: string
): Promise<InvoiceRecord | undefined> {
  const list = await apiCallRaw<InvoiceListData>(page, 'GET', '/finance/invoices');
  expect(Array.isArray(list.invoices), '列表响应应含数组键 invoices').toBe(true);
  expect(typeof list.total, '列表响应应含数值键 total').toBe('number');
  expect(
    list.total,
    `列表 total 应等于返回行数（handler 全量返回，total=行数）：total=${list.total} rows=${list.invoices.length}`
  ).toBe(list.invoices.length);
  return list.invoices.find(o => String(o.invoice_no) === invoiceNo);
}

test.describe('12 发票管理建→审批→核销全链回读', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('12-01 建→审→验每步 GET 回读状态词表与四标度金额，未补录字段保持 NULL', async ({ page }) => {
    const invoiceNo = genCode('E2E12-INV');
    const created = await createInvoice(page, invoiceNo, {
      amount: SEEDED.amount,
      tax_amount: SEEDED.taxAmount,
      total_amount: SEEDED.totalAmount,
    });
    const id = Number(created.id);

    // 创建态回读：初始状态与未补录字段（order_id / payment_method / notes / paid_date 一律 NULL，不写 0 或空串）。
    expect(String(pick(created, 'status', '创建响应')), '创建响应状态应为 pending').toBe(
      INVOICE_STATUS.PENDING
    );
    const g0 = await readInvoice(page, id);
    expect(Number(g0.id), '回读 id 应与建单返回一致').toBe(id);
    expect(String(pick(g0, 'invoice_no', '创建回读')), '发票号应原样落库').toBe(invoiceNo);
    assertInvoiceTruth(g0, '创建回读', INVOICE_STATUS.PENDING, { paidDateNull: true });
    expect(pick(g0, 'order_id', '创建回读'), '未关联订单时 order_id 应为 NULL').toBeNull();
    expect(pick(g0, 'payment_method', '创建回读'), '未填写付款方式应为 NULL').toBeNull();
    expect(pick(g0, 'notes', '创建回读'), '未填写备注应为 NULL').toBeNull();

    // invoice_date 由后端 Utc::now() 写入（请求不携带该字段），必须是可解析的真实时间戳且属于今天。
    const invoiceDate = String(pick(g0, 'invoice_date', '创建回读'));
    expect(
      /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/.test(invoiceDate),
      `invoice_date 应为 RFC3339 时间文本，实际=${invoiceDate}`
    ).toBe(true);
    const invoiceDateObj = new Date(invoiceDate);
    expect(
      Number.isNaN(invoiceDateObj.getTime()),
      `invoice_date 应可解析，实际=${invoiceDate}`
    ).toBe(false);
    const todayUtc = new Date().toISOString().slice(0, 10);
    expect(
      invoiceDate.slice(0, 10),
      `invoice_date 日期部分应为后端写入当天（UTC）=${todayUtc}，实际=${invoiceDate}`
    ).toBe(todayUtc);

    // 审批：pending → approved，金额三件套与未核销语义保持不动。
    await apiCallRaw<InvoiceRecord>(page, 'POST', `/finance/invoices/${id}/approve`);
    const g1 = await readInvoice(page, id);
    assertInvoiceTruth(g1, '审批后', INVOICE_STATUS.APPROVED, { paidDateNull: true });

    // 核销：approved → verified，paid_date 由空落为真实时间戳，金额三件套不动。
    await apiCallRaw<InvoiceRecord>(page, 'POST', `/finance/invoices/${id}/verify`);
    const g2 = await readInvoice(page, id);
    assertInvoiceTruth(g2, '核销后', INVOICE_STATUS.VERIFIED, { paidDateNull: false });
    const paidDateRaw = pick(g2, 'paid_date', '核销后');
    expect(
      typeof paidDateRaw === 'string' && paidDateRaw.length > 0,
      `核销后 paid_date 应为非空时间文本，实际=${JSON.stringify(paidDateRaw)}`
    ).toBe(true);
    const paidDate = String(paidDateRaw);
    expect(
      /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/.test(paidDate),
      `paid_date 应为 RFC3339 时间文本，实际=${paidDate}`
    ).toBe(true);
    expect(
      new Date(paidDate).getTime() >= new Date(invoiceDate).getTime(),
      `paid_date 不应早于 invoice_date：paid=${paidDate} invoice=${invoiceDate}`
    ).toBe(true);

    // 列表端点回读：同一行的状态与金额必须与详情逐值一致（两条装配路径同源同值）。
    const row = await findListRow(page, invoiceNo);
    if (!row) throw new Error(`列表 GET 未回读到发票 ${invoiceNo}`);
    expect(String(row.status), '列表中该发票状态应为 verified').toBe(INVOICE_STATUS.VERIFIED);
    for (const key of ['amount', 'tax_amount', 'total_amount'] as const) {
      expect(moneyAtScale(row[key], `列表行 ${key}`), `列表行 ${key} 应与详情回读同值`).toBe(
        moneyAtScale(g2[key], `详情 ${key}`)
      );
    }
    expect(String(row.paid_date), '列表行 paid_date 应与详情同值').toBe(String(g2.paid_date));
    expect(String(row.invoice_no), '列表行发票号应与提交原值逐字符一致').toBe(invoiceNo);
  });

  test('12-02 已核销再审批被拒 400 + BUSINESS_ERROR + 脱敏常量逐字符 + 零变化回查', async ({
    page,
  }) => {
    const invoiceNo = genCode('E2E12-APV');
    const created = await createInvoice(page, invoiceNo, {
      amount: SEEDED.amount,
      tax_amount: SEEDED.taxAmount,
      total_amount: SEEDED.totalAmount,
    });
    const id = Number(created.id);
    await advanceTo(page, id, 'verified');
    // 被拒前的基线回读（含 updated_at），供零变化回查逐项比对，不用两次拒后回读互比。
    const beforeReject = await readInvoice(page, id);

    // 状态门：approve_invoice 仅放行 pending（写入方词表 pending），已核销必须被拒。
    const fail = await apiCallExpectFail(page, 'POST', `/finance/invoices/${id}/approve`);
    expect(
      fail.status,
      `verified 后重复 approve 应恰为 400，实际=${fail.status} code=${fail.code}`
    ).toBe(400);
    expect(failureCode(fail), `机器码应为 BUSINESS_ERROR，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    // 该站点文案回显了记录内部状态 token，按 error.rs 安全边界走脱敏 business：
    // 出参 message 恒为常量，逐字符等于常量即同时证明未外显内部 token（不是只断非空）。
    expect(
      fail.message,
      `脱敏站点 message 应为固定业务失败常量，实际=${JSON.stringify(fail.message)}`
    ).toBe(SANITIZED_BUSINESS);

    // 零变化回查：状态、三个金额、paid_date、updated_at 全部不得被被拒请求改动。
    const after = await readInvoice(page, id);
    assertInvoiceTruth(after, '重复审批被拒后', INVOICE_STATUS.VERIFIED, { paidDateNull: false });
    expect(String(after.updated_at), '被拒请求不得改写 updated_at').toBe(
      String(beforeReject.updated_at)
    );
    expect(String(after.paid_date), '被拒请求不得改写 paid_date').toBe(
      String(beforeReject.paid_date)
    );
  });

  test('12-03 首次审批放行、已审批再审批被拒（状态门仅 pending 的双向自证）+ 金额与 paid_date 零变化', async ({
    page,
  }) => {
    const invoiceNo = genCode('E2E12-RPT');
    const created = await createInvoice(page, invoiceNo, {
      amount: SEEDED.amount,
      tax_amount: SEEDED.taxAmount,
      total_amount: SEEDED.totalAmount,
    });
    const id = Number(created.id);

    // 首次审批放行（同一端点、同一账号，唯一差别是状态），反证 12-02 的拒绝确由状态门触发。
    await apiCallRaw<InvoiceRecord>(page, 'POST', `/finance/invoices/${id}/approve`);
    const approvedOnce = await readInvoice(page, id);
    assertInvoiceTruth(approvedOnce, '首次审批后', INVOICE_STATUS.APPROVED, {
      paidDateNull: true,
    });

    // 第二次审批：approved 不满足仅 pending 的门 → 400 + BUSINESS_ERROR + 脱敏常量。
    const fail = await apiCallExpectFail(page, 'POST', `/finance/invoices/${id}/approve`);
    expect(fail.status, `approved 后重复 approve 应恰为 400，实际=${fail.status}`).toBe(400);
    expect(failureCode(fail), '机器码应为 BUSINESS_ERROR').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(fail.message, 'message 应逐字符等于脱敏常量').toBe(SANITIZED_BUSINESS);

    const after = await readInvoice(page, id);
    assertInvoiceTruth(after, '重复审批被拒后', INVOICE_STATUS.APPROVED, { paidDateNull: true });
  });

  test('12-04 负金额建单被拒 400 + VALIDATION_ERROR + 可外显文案逐字符 + 零记录回查', async ({
    page,
  }) => {
    const invoiceNo = genCode('E2E12-NEG');
    const fail = await apiCallExpectFail(page, 'POST', '/finance/invoices', {
      invoice_no: invoiceNo,
      amount: -1,
      tax_amount: SEEDED.taxAmount,
      total_amount: SEEDED.totalAmount,
    });
    expect(fail.status, `负金额建单应恰为 400，实际=${fail.status} code=${fail.code}`).toBe(400);
    expect(failureCode(fail), `机器码应为 VALIDATION_ERROR，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    // 该站点走 validation_displayable：文案只涉及用户自己提交的金额，允许外显，故逐字符钉死真实文案。
    expect(
      fail.message,
      `金额负值文案应逐字符等于写入方真实文案，实际=${JSON.stringify(fail.message)}`
    ).toBe(NEGATIVE_AMOUNT_MESSAGE);

    // 零变化回查：被拒的建单不得留下该发票号。
    const leaked = await findListRow(page, invoiceNo);
    expect(leaked, `负金额建单被拒后不应存在发票 ${invoiceNo}`).toBeUndefined();
  });

  test('12-05 页面按表头列定位回读核销状态中文文案与金额原值（不整行 hasText）', async ({
    page,
  }) => {
    const invoiceNo = genCode('E2E12-UI');
    const created = await createInvoice(page, invoiceNo, {
      amount: SEEDED.amount,
      tax_amount: SEEDED.taxAmount,
      total_amount: SEEDED.totalAmount,
    });
    const id = Number(created.id);
    await advanceTo(page, id, 'verified');
    const detail = await readInvoice(page, id);

    await page.goto(`${BASE_URL}/invoice-details`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });
    const headerCells = page.locator('.el-table__header-wrapper th .cell');
    await expect(headerCells.first(), '发票列表表头应已渲染').toBeVisible({ timeout: 30000 });
    const headerTexts = (await headerCells.allTextContents()).map(t => t.trim());
    const idxStatus = headerTexts.indexOf(INVOICE_COLUMN.STATUS);
    const idxAmount = headerTexts.indexOf(INVOICE_COLUMN.AMOUNT);
    const idxTax = headerTexts.indexOf(INVOICE_COLUMN.TAX);
    const idxTotal = headerTexts.indexOf(INVOICE_COLUMN.TOTAL);
    const idxOrder = headerTexts.indexOf(INVOICE_COLUMN.ORDER);
    for (const [name, idx] of [
      ['状态', idxStatus],
      ['发票金额', idxAmount],
      ['税额', idxTax],
      ['价税合计', idxTotal],
      ['关联订单', idxOrder],
    ] as const) {
      expect(
        idx,
        `表头应含列 ${name}，实际表头=${JSON.stringify(headerTexts)}`
      ).toBeGreaterThanOrEqual(0);
    }

    const row = page.getByRole('row').filter({ has: page.getByText(invoiceNo, { exact: true }) });
    await expect(row, `页面应渲染出该发票行`).toHaveCount(1);
    // 按列下标取格子前先硬断格子数与表头列数一致（防止错位取格把别列文本当判据）。
    const tdCount = await row.locator('td').count();
    expect(
      tdCount,
      `行格子数应等于表头列数：td=${tdCount} th=${headerTexts.length}，表头=${JSON.stringify(headerTexts)}`
    ).toBe(headerTexts.length);

    // 状态格子内的标签文本必须逐字符等于前端词表对 verified 的中文文案（中文词表禁英文化）。
    await expect(
      row.locator('td').nth(idxStatus).locator('.el-tag'),
      `核销状态应渲染为 ${INVOICE_STATUS_LABEL[INVOICE_STATUS.VERIFIED]}`
    ).toHaveText(INVOICE_STATUS_LABEL[INVOICE_STATUS.VERIFIED]);

    // 金额格文本等于后端出参原值（视图直接插值 row.amount，不额外格式化），证 UI 未被兜底掩盖。
    const expectedAmountText = String(detail.amount);
    await expect(
      row.locator('td').nth(idxAmount),
      `发票金额格应等于后端出参原值 ${expectedAmountText}`
    ).toHaveText(expectedAmountText);
    await expect(
      row.locator('td').nth(idxTax),
      `税额格应等于后端出参原值 ${String(detail.tax_amount)}`
    ).toHaveText(String(detail.tax_amount));
    await expect(
      row.locator('td').nth(idxTotal),
      `价税合计格应等于后端出参原值 ${String(detail.total_amount)}`
    ).toHaveText(String(detail.total_amount));
    // order_id 为 NULL 时视图渲染短横，不得被兜底成 0 或空。
    await expect(
      row.locator('td').nth(idxOrder),
      '未关联订单的格子应渲染短横而非 0 或空'
    ).toHaveText('-');
  });
});
