// 本文件守的缺陷族：出库后应收未自动生成（提示成功未落库）、应收金额与订单行金额
// 在链路各环不一致却无人报错、源单关联键丢失导致发票与订单对不上账、应收状态门被绕过。
// 金额一律断到分位：Decimal 走字符串出参，折算整数分后精确比较，不经 JS number 相加，
// 期望值从订单行出参实时推导（行 subtotal+tax_amount），不写死会随造数漂移的业务数值。
// 状态比较值与写入方逐字符同源：ar_invoice 大写 DRAFT/APPROVED/PARTIAL_PAID/PAID/CANCELLED，
// approval_status 大写 PENDING/APPROVED；销售订单小写 shipped。
// 出参形状：/ar/invoices 列表为 PaginatedResponse{items,...}；详情为裸 Model（snake_case 键）。
import { test, expect } from '../diagnose-fixture';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  failureCode,
  genCode,
  getCtx,
  loginViaUI,
  pickDyeableWarehouse,
  seedDyedOutboundBundle,
  tryCleanup,
  APP_ERROR_CODES,
} from './helpers';
import { pickListArray } from './ui-helpers';

type Row = Record<string, unknown>;

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function toScaled(v: unknown, scale: number, label: string): bigint {
  const s = typeof v === 'string' ? v.trim() : typeof v === 'number' ? String(v) : '';
  if (s === '') throw new Error(`${label}：Decimal 出参非 string|number，raw=${JSON.stringify(v)}`);
  const m = /^(-?)(\d+)(?:\.(\d+))?$/.exec(s);
  if (!m) throw new Error(`${label}：Decimal 出参形态非法，raw="${s}"`);
  const [, sign, intPart, frac = ''] = m;
  if (frac.length > scale && frac.slice(scale).replace(/0+$/, '') !== '') {
    throw new Error(`${label}：出参标度超出断言精度且非零，raw="${s}"`);
  }
  const padded = (intPart + frac).padEnd(intPart.length + scale, '0');
  const value = BigInt(padded);
  return sign === '-' ? -value : value;
}

/** 分位精确金额比较 */
function eqAmount(actual: unknown, expectedCents: bigint, label: string): void {
  expect(
    toScaled(actual, 2, label),
    `${label}：期望(分)=${expectedCents} 实际=${JSON.stringify(actual)}`
  ).toBe(expectedCents);
}

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0) throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

async function findInvoiceBySource(
  page: import('@playwright/test').Page,
  customerId: number,
  orderId: number
): Promise<Row | undefined> {
  const env = await apiCallRaw<Row>(
    page,
    'GET',
    `/ar/invoices?customer_id=${customerId}&page=1&page_size=100`
  );
  const rows = pickListArray<Row>(env, 'items', 'AR 应收列表');
  return rows.find(r => Number(r.source_bill_id) === orderId && r.source_type === 'SALES_ORDER');
}

test.describe('91 出库→开票（应收自动生成与金额一致性）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('91-01 全额发货自动生成应收：金额逐分等于订单行含税合计、源单关联键全部可回读、发货前无应收假落库', async ({
    page,
  }) => {
    const ctx = getCtx();
    if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
    const productId = ctx.productIds[1] ?? ctx.productIds[0];
    if (!productId) throw new Error('前置缺失：ctx.productIds 为空');

    const created = await apiCallRaw<Row>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      items: [
        {
          product_id: productId,
          quantity: '80',
          unit_price: '45.67',
          tax_percent: '13',
          quantity_tolerance_pct: '0',
        },
      ],
      notes: `E2E-AR-${genCode('A1')}`,
    });
    const orderId = requireNum(created.id, '建销售订单');
    CLEANUP.push({ path: `/sales/orders/${orderId}`, label: `so#${orderId}` });
    await apiCall(page, 'POST', `/sales/orders/${orderId}/submit`);
    await apiCall(page, 'POST', `/sales/orders/${orderId}/approve`);

    // 发货前：该订单名下不允许已存在销售来源应收（未出库先落应收=账实两头空的前兆）
    const beforeShip = await findInvoiceBySource(page, ctx.customerId, orderId);
    expect(beforeShip, '发货前不应存在该订单的销售来源应收单').toBeUndefined();

    const target = await pickDyeableWarehouse(page);
    const bundle = await seedDyedOutboundBundle(page, {
      productId,
      warehouseId: target.id,
      quantityMeters: '80',
      pieceCount: 1,
      context: 'E91-01',
    });
    await apiCall(page, 'POST', `/sales/orders/${orderId}/ship`, {
      order_id: orderId,
      warehouse_code: target.code,
      items: [
        {
          product_id: productId,
          quantity: '80',
          batch_no: bundle.dyeLotNo,
          color_no: bundle.colorNo,
          dye_lot_no: bundle.dyeLotNo,
          piece_no: bundle.pieces[0].piece_no,
        },
      ],
    });
    const order = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${orderId}`);
    expect(order.status, '全额发货后订单词值 shipped（应收生成的状态前提）').toBe('shipped');
    const line = (order.items as Row[])[0];
    // 期望金额从订单行出参实时推导：含税合计 = 行 subtotal + 行 tax_amount（均为分位落库值），
    // 不写死业务数值，避免造数漂移；分位相加走 BigInt，杜绝 JS number 浮点误差。
    const lineSubtotalCents = toScaled(line.subtotal, 2, '订单行 subtotal');
    const lineTaxCents = toScaled(line.tax_amount, 2, '订单行 tax_amount');
    const expectedTotalCents = lineSubtotalCents + lineTaxCents;
    // 订单头总额与行合计的账内自洽（发票金额一致性以此为锚）
    eqAmount(order.total_amount, expectedTotalCents, '订单头 total_amount=行含税合计');

    const inv = await findInvoiceBySource(page, ctx.customerId, orderId);
    expect(inv, '全额发货必须自动生成销售来源应收单（提示成功未落库在这里现形）').toBeTruthy();
    const invRow = inv as Row;
    eqAmount(invRow.invoice_amount, expectedTotalCents, '应收金额=订单行含税合计（出库→开票金额一致）');
    eqAmount(invRow.unpaid_amount, expectedTotalCents, '初值未收=发票全额');
    eqAmount(invRow.received_amount, 0n, '初值已收=0');
    expect(invRow.source_type, '源单类型').toBe('SALES_ORDER');
    expect(Number(invRow.source_bill_id), '源单 ID 关联键必须可回读且等于订单 id').toBe(orderId);
    expect(String(invRow.source_bill_no), '源单单号逐字符等于订单号').toBe(String(order.order_no));
    expect(String(invRow.sales_order_no), '销售单号冗余列逐字符等于订单号').toBe(String(order.order_no));
    expect(invRow.status, '自动应收初始大写词值 DRAFT').toBe('DRAFT');
    expect(invRow.approval_status, '自动应收审批初始大写词值 PENDING').toBe('PENDING');
    // 客户名从客户主档带出（不是 #id 假名、不是空占位）
    const cust = await apiCallRaw<Row>(page, 'GET', `/crm/customers/${ctx.customerId}`);
    expect(String(invRow.customer_name), '应收客户名必须逐字符等于客户主档名').toBe(
      String(cust.customer_name)
    );
    // 到期日=发票日+客户账期（payment_terms<=0 回退 30 天）
    const termsRaw = Number(cust.payment_terms ?? 0);
    const terms = termsRaw > 0 ? termsRaw : 30;
    const invDate = new Date(String(invRow.invoice_date));
    const dueDate = new Date(String(invRow.due_date));
    const dayDiff = Math.round((dueDate.getTime() - invDate.getTime()) / 86_400_000);
    expect(dayDiff, `到期日与发票日差值应等于客户账期 ${terms} 天`).toBe(terms);

    const invId = requireNum(invRow.id, '自动应收 id');
    // 审批推进 + 回读（刷新语义二次读一致）
    await apiCall(page, 'POST', `/ar/invoices/${invId}/approve`);
    const approved = await apiCallRaw<Row>(page, 'GET', `/ar/invoices/${invId}`);
    expect(approved.status, 'approve 回读 APPROVED').toBe('APPROVED');
    expect(approved.approval_status, 'approval_status 同步 APPROVED').toBe('APPROVED');
    const me = await apiCallRaw<Row>(page, 'GET', '/auth/me');
    expect(Number(approved.reviewed_by), '审核人落真实操作人').toBe(Number(me.id));
    expect(approved.reviewed_at, 'reviewed_at 必须写入非空').toBeTruthy();
    const approvedAgain = await apiCallRaw<Row>(page, 'GET', `/ar/invoices/${invId}`);
    expect(String(approvedAgain.status), '二次读回状态稳定').toBe('APPROVED');
  });

  test('91-02 应收创建与修改负例族：非法金额点名原因、精确机器码、被拒零落库', async ({ page }) => {
    const ctx = getCtx();
    if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
    const d = new Date().toISOString().slice(0, 10);

    // 金额为零 → VALIDATION_ERROR + 文案点名「大于零」（可外显族，钉文案不是脱敏常量）
    const fZero = await apiCallExpectFail(page, 'POST', '/ar/invoices', {
      customer_id: ctx.customerId,
      invoice_amount: '0',
      invoice_date: d,
    });
    expect(fZero.status, '0 元应收应 400').toBe(400);
    expect(failureCode(fZero), '0 元机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const zeroMsg = String(fZero.message ?? '');
    expect(zeroMsg, '0 元拒绝文案必须点名原因，不得是脱敏常量').not.toBe('请求参数验证失败');
    expect(zeroMsg, `0 元拒绝文案应含「大于零」，实际=${zeroMsg}`).toContain('大于零');

    // 金额 3 位小数 → 精度拒绝，文案点名精度
    const fPrec = await apiCallExpectFail(page, 'POST', '/ar/invoices', {
      customer_id: ctx.customerId,
      invoice_amount: '100.005',
      invoice_date: d,
    });
    expect(fPrec.status, '超精度应收应 400').toBe(400);
    expect(failureCode(fPrec), '超精度机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(String(fPrec.message ?? ''), '超精度拒绝应点名「精度」').toContain('精度');

    // 缺客户 → 拒绝点名客户
    const fNoCust = await apiCallExpectFail(page, 'POST', '/ar/invoices', {
      invoice_amount: '10',
      invoice_date: d,
    });
    expect(fNoCust.status, '缺客户 ID 应 400').toBe(400);
    expect(failureCode(fNoCust), '缺客户机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 客户不存在 → 404 NOT_FOUND（不是 500、不是空成功）
    const fGhost = await apiCallExpectFail(page, 'POST', '/ar/invoices', {
      customer_id: 2147483647,
      invoice_amount: '10',
      invoice_date: d,
    });
    expect(fGhost.status, '幽灵客户建应收应 404').toBe(404);
    expect(failureCode(fGhost), '幽灵客户机器码').toBe(APP_ERROR_CODES.NOT_FOUND);

    // DRAFT 修改：非法精度被拒后金额原样；合法修改后 unpaid 按「新额-已收」联动重算（分位）
    const inv = await apiCallRaw<Row>(page, 'POST', '/ar/invoices', {
      customer_id: ctx.customerId,
      invoice_amount: '666.66',
      invoice_date: d,
    });
    const invId = requireNum(inv.id, '建应收(DRAFT)');
    CLEANUP.push({ path: `/ar/invoices/${invId}`, label: `ar_invoice#${invId}` });
    const fBadUpd = await apiCallExpectFail(page, 'PUT', `/ar/invoices/${invId}`, {
      invoice_amount: '7.777',
    });
    expect(fBadUpd.status, 'DRAFT 改非法精度应 400').toBe(400);
    expect(failureCode(fBadUpd), 'DRAFT 改非法精度机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const still666 = await apiCallRaw<Row>(page, 'GET', `/ar/invoices/${invId}`);
    eqAmount(still666.invoice_amount, 66666n, '非法修改被拒后金额分毫未动');
    eqAmount(still666.unpaid_amount, 66666n, '非法修改被拒后未收分毫未动');

    await apiCall(page, 'PUT', `/ar/invoices/${invId}`, { invoice_amount: '77.77' });
    const upd = await apiCallRaw<Row>(page, 'GET', `/ar/invoices/${invId}`);
    eqAmount(upd.invoice_amount, 7777n, '合法修改金额落库');
    eqAmount(upd.unpaid_amount, 7777n, '已收为 0 时未收联动=新金额');

    // 审批后不可再改：状态门拒绝并点名「草稿」，金额零漂移
    await apiCall(page, 'POST', `/ar/invoices/${invId}/approve`);
    const fAfterApprove = await apiCallExpectFail(page, 'PUT', `/ar/invoices/${invId}`, {
      invoice_amount: '1',
    });
    expect(fAfterApprove.status, 'APPROVED 后修改应 400').toBe(400);
    expect(failureCode(fAfterApprove), 'APPROVED 后修改机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(String(fAfterApprove.message ?? ''), '拒绝文案应点名草稿门').toContain('草稿');
    const frozen = await apiCallRaw<Row>(page, 'GET', `/ar/invoices/${invId}`);
    eqAmount(frozen.invoice_amount, 7777n, '审批后改被拒金额不得半途写入');

    // 重复审批被拒（仅草稿可审批），状态不漂
    const fDupApprove = await apiCallExpectFail(page, 'POST', `/ar/invoices/${invId}/approve`);
    expect(fDupApprove.status, '重复审批应 400').toBe(400);
    expect(failureCode(fDupApprove), '重复审批机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const noDrift = await apiCallRaw<Row>(page, 'GET', `/ar/invoices/${invId}`);
    expect(noDrift.status, '重复审批后状态仍 APPROVED').toBe('APPROVED');

    // 删除仅草稿：APPROVED 删除被拒且单据仍在
    const fDel = await apiCallExpectFail(page, 'DELETE', `/ar/invoices/${invId}`);
    expect(fDel.status, 'APPROVED 删除应 400').toBe(400);
    expect(failureCode(fDel), 'APPROVED 删除机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const alive = await apiCallRaw<Row>(page, 'GET', `/ar/invoices/${invId}`);
    expect(Number(alive.id), '删除被拒后单据必须仍可读到').toBe(invId);
  });

  test('91-03 应收终态门：已取消不可再取消、已取消/终态读回逐字段一致，删除草稿后 404 真实消失', async ({
    page,
  }) => {
    const ctx = getCtx();
    if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
    const d = new Date().toISOString().slice(0, 10);

    const inv = await apiCallRaw<Row>(page, 'POST', '/ar/invoices', {
      customer_id: ctx.customerId,
      invoice_amount: '120.30',
      invoice_date: d,
    });
    const invId = requireNum(inv.id, '建应收');
    await apiCall(page, 'POST', `/ar/invoices/${invId}/approve`);
    await apiCall(page, 'POST', `/ar/invoices/${invId}/cancel`, { reason: 'E2E 取消测试单' });
    const cancelled = await apiCallRaw<Row>(page, 'GET', `/ar/invoices/${invId}`);
    expect(cancelled.status, '取消回读 CANCELLED（大写词表）').toBe('CANCELLED');
    eqAmount(cancelled.invoice_amount, 12030n, '取消不改发票金额');
    eqAmount(cancelled.unpaid_amount, 12030n, '取消不改未收额');

    const fRecancel = await apiCallExpectFail(page, 'POST', `/ar/invoices/${invId}/cancel`, {
      reason: '再取消一次',
    });
    expect(fRecancel.status, '已取消再取消应 400').toBe(400);
    expect(failureCode(fRecancel), '已取消再取消机器码 BAD_REQUEST').toBe(APP_ERROR_CODES.BAD_REQUEST);
    const stillCancelled = await apiCallRaw<Row>(page, 'GET', `/ar/invoices/${invId}`);
    expect(stillCancelled.status, '重复取消被拒后仍是 CANCELLED，不得漂到别的态').toBe('CANCELLED');

    // 草稿删除真实落库：删后 GET 必须 404（不是只回 toast）
    const draft = await apiCallRaw<Row>(page, 'POST', '/ar/invoices', {
      customer_id: ctx.customerId,
      invoice_amount: '10.00',
      invoice_date: d,
    });
    const draftId = requireNum(draft.id, '建草稿应收');
    await apiCall(page, 'DELETE', `/ar/invoices/${draftId}`);
    const fGone = await apiCallExpectFail(page, 'GET', `/ar/invoices/${draftId}`);
    expect(fGone.status, '删除后回读应 404').toBe(404);
    expect(failureCode(fGone), '删除后机器码 NOT_FOUND').toBe(APP_ERROR_CODES.NOT_FOUND);
    const env = await apiCallRaw<Row>(
      page,
      'GET',
      `/ar/invoices?customer_id=${ctx.customerId}&page=1&page_size=100`
    );
    const rows = pickListArray<Row>(env, 'items', 'AR 列表（删除后）');
    expect(
      rows.some(r => Number(r.id) === draftId),
      '已删草稿不得仍出现在列表（软删不可见性=删除语义真实生效）'
    ).toBe(false);
  });
});
