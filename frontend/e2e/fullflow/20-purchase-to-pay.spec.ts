// 交易域全流程契约级 E2E — 20 采购到付款（PO→收货→采购发票→付款申请→付款）
//
// 定位：补齐审计缺口「e2e 只走成功路径、从不回读后端权威状态」。本文件把用户点名的两类缺陷
// 钉在 P2P 链的每一跳上：
//   ①「创建时保存的数据不完整、再编辑显示不出来」→ 全键建 PO 后 GET 逐字段回读；收货后回读
//      订单行 received_quantity/received_quantity_alt（辅量历史丢字段靶心）；付款申请明细回读。
//   ②「提交/保存报请求错误」→ 每个 4xx 断具体 status + 具体机器码（BUSINESS_ERROR/
//      VALIDATION_ERROR/NOT_FOUND），可外显站点再断真实 message；脱敏站点只锁 code。
// 取数口径（与 sales/03-approve、purchase/03 同范式）：本例 API/UI 自建唯一单 → 后端生成的唯一
// order_no/receipt_no 精确锚定 → 操作 → toast + 回查后端状态字面量 → afterEach 尽力清理。
// 禁依赖共享 seed 行。
//
// 契约真值源（逐字段核对，非猜键名）：
//   PO 状态词表（大写）      backend/src/models/status/purchase_inventory.rs:13-32
//     （DRAFT/PENDING_APPROVAL/APPROVED/PARTIAL_RECEIVED/COMPLETED/REJECTED/CANCELLED/CLOSED/SUBMITTED）
//   收货词表（大写）         同上 :35-42（DRAFT/CONFIRMED/COMPLETED；confirm 直达 COMPLETED，
//     state.rs:53-59 build_completed_receipt_active_model）
//   CreatePurchaseOrderRequest        services/po/mod.rs:33-91（items 必填≥1 行 :88-90）
//   CreateOrderItemRequest            services/po/mod.rs:111-143（quantity_alt_ordered 辅量 :125）
//   UpdatePurchaseOrderRequest        services/po/mod.rs:94-108（单层 Option 全键）
//   收货确认三门           services/purchase_receipt_ops/state.rs:84-104（DRAFT 门 + 明细数门）
//   订单进度回写           services/purchase_receipt_private.rs:164-197
//     （received_quantity/received_quantity_alt 累加；determine_order_receipt_status
//      :172-196 → 全收 COMPLETED / 部分 PARTIAL_RECEIVED）
//   批次四维门             services/purchase_receipt_private.rs:183-196 require_receipt_batch
//     （空/空白批次整单业务拒，不落库存行）
//   收货自动生成应付       services/ap_invoice_ops/receipt.rs:71-175
//     （source_type="PURCHASE_RECEIPT" :43，amount=receipt.total_amount :172，初始 DRAFT）
//   AP 门与词表            services/ap_invoice_ops/crud.rs:112-118（update 仅 DRAFT）+
//     models/status/general.rs:14-33/finance.rs:47-50（DRAFT/AUDITED/PARTIAL_PAID/PAID；
//     付款申请 APPROVING/APPROVED/REJECTED）——与 fullflow/10-ap 同源
//   付款申请 submit 需明细  services/ap_payment_request_service.rs:336-338（10-ap 头注释同源）
//   付款金额取申请额       services/ap_payment_service.rs:752-766；确认 REGISTERED→CONFIRMED :223-226
// 诚实标注：PurchaseReceiveDialog 采集批次号 + 辅助数量（usePurchRcv buildReceiptPayload；
//   e2e/purchase/03 头注释），色号/缸号/等级留默认——收货对话框**无法录入**完整四维，
//   四维全量入库已由 purchase/13-01 以 API+UI 审核组合覆盖；本文件按对话框真实能力
//   收货，并断言由此产生的库存行四维默认口径（color/lot 空串、grade 一等品，
//   purchase_receipt_private.rs receipt_item_stock_key）。辅量已接入真实采集：
//   20-01 逐批录入非零辅量并回读 received_quantity_alt 累加，20-02 录显式 0（合法实收值）。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  tryCleanup,
  APP_ERROR_CODES,
  seedInspectionPass,
} from '../flow/helpers';
import { pickSelectIn, pickListArray } from '../flow/ui-helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 键存在且值严格相等（缺键即红并列出实际键集，杜绝静默丢字段=缺陷①）。 */
function expectKeyValue(
  obj: Record<string, unknown>,
  key: string,
  expected: unknown,
  label: string
): void {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺少后端真实键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  expect(
    obj[key],
    `${label}：键 "${key}" 期望=${JSON.stringify(expected)} 实际=${JSON.stringify(obj[key])}`
  ).toEqual(expected);
}

/** Decimal 出参（rust_decimal 默认字符串如 "120.00"）Number() 归一后按货币精度比。 */
function expectDecimal(obj: Record<string, unknown>, key: string, expected: number, label: string) {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺少金额/数量键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n)) {
    throw new Error(`${label}：键 "${key}" 不可解析为数字，raw=${JSON.stringify(obj[key])}`);
  }
  expect(Math.abs(n - expected), `${label}：${key} 期望=${expected} 实际=${n}`).toBeLessThan(0.005);
}

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0) {
    throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  }
  return n;
}

/** GET /purchase/orders/{id} 详情（handler 内嵌 items，purchase_order_handler.rs:100-127） */
interface PoLite {
  id: number;
  order_no: string;
  status: string;
}

/**
 * 全键建 PO（CreatePurchaseOrderRequest 全部可送键，services/po/mod.rs:33-91），
 * 推进到目标状态并逐段回查后端状态字面量；不达预期立即抛错（禁兜底）。
 * 明细含 quantity_alt_ordered=200 辅量（缺陷①历史丢字段靶心）。
 */
async function seedPo(
  page: Page,
  target: 'DRAFT' | 'PENDING_APPROVAL' | 'APPROVED'
): Promise<PoLite> {
  const ctx = getCtx();
  if (!ctx.supplierId) throw new Error('前置缺失：ctx.supplierId 未就绪');
  if (!ctx.productIds[0]) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
  if (!ctx.warehouseIds[0]) throw new Error('前置缺失：ctx.warehouseIds[0] 未就绪');
  if (!ctx.departmentIds[0]) throw new Error('前置缺失：ctx.departmentIds[0] 未就绪');
  const marker = genCode('F20');

  const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/orders', {
    supplier_id: ctx.supplierId,
    order_date: new Date().toISOString().slice(0, 10),
    expected_delivery_date: new Date(Date.now() + 14 * 86400000).toISOString().slice(0, 10),
    warehouse_id: ctx.warehouseIds[0],
    department_id: ctx.departmentIds[0],
    currency: 'USD',
    exchange_rate: '7.15',
    payment_terms: 'NET30 电汇',
    shipping_terms: 'FOB',
    notes: `E2E-F20-${marker}`,
    attachment_urls: ['https://example.com/po-a.pdf'],
    items: [
      {
        line_no: 1,
        material_id: ctx.productIds[0],
        quantity_ordered: '20',
        quantity_alt_ordered: '200',
        unit_price: '15.00',
        tax_rate: '0',
        color_no: 'E2E-C1',
        notes: `E2E-F20-L1-${marker}`,
      },
    ],
  });
  const id = requireNum(created.id, `建 PO 响应（${marker}）`);
  const orderNo = String(created.order_no ?? '');
  if (!orderNo) throw new Error(`建 PO 未返回 order_no，实际键=${Object.keys(created).join(',')}`);
  CLEANUP.push({ path: `/purchase/orders/${id}`, label: `purchase_order#${id}` });

  // ①全字段回读（缺陷①靶心）：详情逐键对齐提交值
  const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/orders/${id}`);
  expectKeyValue(detail, 'id', id, 'PO详情');
  expectKeyValue(detail, 'order_no', orderNo, 'PO详情');
  expectKeyValue(detail, 'supplier_id', ctx.supplierId, 'PO详情');
  expectKeyValue(detail, 'notes', `E2E-F20-${marker}`, 'PO详情');
  expectKeyValue(
    detail,
    'status',
    'DRAFT',
    'PO详情初始（PurchaseOrderDto order_status 经 serde rename 为 status，services/po/order.rs:38-39）'
  );
  // 明细内嵌（purchase_order_handler.rs:100-127 单查挂 order_json["items"]）
  const items = pickListArray<Record<string, unknown>>(
    { items: detail.items },
    'items',
    `PO ${id} 详情明细`
  );
  expect(items.length, `PO 详情应含 1 行自建明细，实际 ${items.length}`).toBe(1);
  expectDecimal(items[0], 'quantity', 20, 'PO明细');
  expectDecimal(
    items[0],
    'quantity_alt',
    200,
    'PO明细（辅量 quantity_alt，模型列 purchase_order_item.rs:32）'
  );
  expectDecimal(items[0], 'unit_price', 15, 'PO明细');
  expectDecimal(items[0], 'received_quantity', 0, 'PO明细初始已收');
  expectDecimal(items[0], 'received_quantity_alt', 0, 'PO明细初始已收辅量');

  // 真实状态机推进（非改库），每步回查字面量
  // 词表：purchase_order.status 写入方 DRAFT→submit→PENDING_APPROVAL→approve→APPROVED
  let status = String(detail.status ?? created.status ?? 'DRAFT');
  if (target !== 'DRAFT') {
    await apiCall(page, 'POST', `/purchase/orders/${id}/submit`);
    const after = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/orders/${id}`);
    status = String(after.status ?? '');
    if (status !== 'PENDING_APPROVAL') {
      throw new Error(`submit 后应为 PENDING_APPROVAL（实际 ${status}，id=${id}）`);
    }
  }
  if (target === 'APPROVED') {
    await apiCall(page, 'POST', `/purchase/orders/${id}/approve`);
    const after = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/orders/${id}`);
    status = String(after.status ?? '');
    if (status !== 'APPROVED') {
      throw new Error(`approve 后应为 APPROVED（实际 ${status}，id=${id}）`);
    }
  }
  return { id, order_no: orderNo, status };
}

/** 按唯一 order_no 把 /purchase 列表收敛到本例行（purchase/03 locateRowByOrderNo 范式） */
async function locatePoRow(page: Page, orderNo: string) {
  await page.goto('/purchase');
  await page.getByPlaceholder('订单号/供应商名').fill(orderNo);
  await page.getByRole('button', { name: '查询', exact: true }).click();
  const row = page.getByRole('row').filter({ hasText: orderNo });
  await expect(row, `按订单号 ${orderNo} 应筛出本例专属行`).toHaveCount(1, { timeout: 30000 });
  return row;
}

/**
 * UI 收货（PurchaseReceiveDialog 真实能力：仓库下拉 + 本次收货数量 + 辅助数量 + 批次号）。
 * 辅量是创建契约必填键，留空会被对话框提交拦截，必须随每批录入实测值（0 为合法实收值）。
 * 捕获 POST /purchase/receipts 响应取本例入库单 id（禁靠列表顺序猜行）。
 */
async function receiveViaUI(
  page: Page,
  orderNo: string,
  qty: string,
  altQty: string,
  batchNo: string
): Promise<number> {
  const row = await locatePoRow(page, orderNo);
  await row.getByRole('button', { name: '收货', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: '采购收货' });
  await expect(dialog).toBeVisible({ timeout: 30000 });
  await pickSelectIn(dialog, page, '仓库');
  await dialog.getByRole('spinbutton').first().fill(qty);
  await dialog.locator('input[placeholder="请输入辅助数量，无辅量填0"]').first().fill(altQty);
  await dialog.locator('input[placeholder="收货批次号"]').first().fill(batchNo);
  // 只绑 200：CSRF token 一次性消费下 UI「确定收货」首个 POST 可能 403（csrf.rs:110 consume
  // + :216-224 轮换，前端 axios 用恢复头静默重放，request.ts:197-223）。不过滤状态码时
  // waitForResponse 命中 data=null 的 403 中间态，"收货响应入库单 id"落空即真红假象
  //（同 purchase/11-03 族）；toast「收货成功」已先断，真被拒时本行与 toast 双判红，不放宽。
  const createdResp = page
    .waitForResponse(
      r =>
        r.request().method() === 'POST' &&
        /\/purchase\/receipts(\?|$)/.test(r.url()) &&
        r.status() === 200,
      { timeout: 30000 }
    )
    .catch(() => null);
  await dialog.getByRole('button', { name: '确定收货' }).click();
  // createPurchaseReceipt 成功 → msg.success('receiveSuccess') = '收货成功'（purchase/03 头注释）
  await expect(page.getByText('收货成功')).toBeVisible({ timeout: 30000 });
  const resp = await createdResp;
  expect(resp, `UI 收货未捕获 POST /purchase/receipts 响应（单号 ${orderNo}）`).not.toBeNull();
  const body = (await resp!.json()) as { data?: { id?: number } };
  return requireNum(body.data?.id, `收货响应入库单 id（${orderNo}）`);
}

test.describe('20 采购到付款全流程契约链', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await ensureTestEntities(page);
  });

  test('20-01 主链：全键建PO→submit/approve→UI收货→确认入库→进度/库存/辅量回读→自动生成采购发票→付款申请→付款→金额一致', async ({
    page,
  }) => {
    const { id: poId, order_no: orderNo } = await seedPo(page, 'APPROVED');

    // ── UI 收货 8 件 / 辅量 80（对话框送批次 + 辅量两维）──
    const batch1 = `E2E-B1${genCode('RCV')}`;
    const rcptId = await receiveViaUI(page, orderNo, '8', '80', batch1);

    // 入库单回读：收货登记只建 DRAFT（purchase_receipt_service.rs build_receipt_active_model
    // 写 status::purchase_receipt::DRAFT，state.rs 头注释）
    const rcpt1 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${rcptId}`
    );
    expectKeyValue(rcpt1, 'order_id', poId, '入库单1（应关联本例 PO）');
    expectKeyValue(rcpt1, 'receipt_status', 'DRAFT', '入库单1初始');
    expectDecimal(rcpt1, 'total_quantity', 8, '入库单1数量=本次收货 8');
    expectDecimal(rcpt1, 'total_quantity_alt', 80, '入库单1辅量合计=对话框实录 80');
    CLEANUP.push({ path: `/purchase/receipts/${rcptId}`, label: `purchase_receipt#${rcptId}` });

    // 明细回读：对话框录入的批次号原样落库（purchase_receipt_item 模型键）
    const lines1 = pickListArray<Record<string, unknown>>(
      await apiCallRaw<unknown>(page, 'GET', `/purchase/receipts/${rcptId}/items`),
      'bare',
      `入库单 ${rcptId} 明细`
    );
    const line1 = lines1.find(l => l.batch_no === batch1);
    expect(
      line1,
      `入库明细应带回收货录入的批次号 ${batch1}，实际=${JSON.stringify(lines1)}`
    ).toBeTruthy();
    expectDecimal(line1!, 'quantity', 8, '入库明细数量');
    expectDecimal(line1!, 'quantity_alt', 80, '入库明细辅量=对话框实录 80（十进制出参归一）');

    // 门控前置：整单质检 complete(pass) 并回读 PASSED（helpers.seedInspectionPass）。
    // 确认事务内会连带触发应付自动生成，此时收货单已是 PASSED，应付侧同一门控
    // （ap_invoice_ops/receipt.rs）不再拦——下面那些 AP/库存/进度回读才有因果可依。
    await seedInspectionPass(page, {
      receiptId: rcptId,
      supplierId: getCtx().supplierId!,
      context: '20-01 首批收货单',
    });

    // ── 确认入库（API 触发动作，回读全部后端权威字段）──
    await apiCall(page, 'POST', `/purchase/receipts/${rcptId}/confirm`);
    const rcpt1After = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${rcptId}`
    );
    expectKeyValue(
      rcpt1After,
      'receipt_status',
      'COMPLETED',
      'confirm 直达 COMPLETED（state.rs:53-59）'
    );

    // ②进度回写靶心：订单行 received_quantity=8、received_quantity_alt=80（对话框实录辅量累加）
    const poAfter1 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/orders/${poId}`
    );
    const poItems1 = pickListArray<Record<string, unknown>>(
      { items: poAfter1.items },
      'items',
      `PO ${poId} 收货后明细`
    );
    expectDecimal(poItems1[0], 'received_quantity', 8, 'PO行已收主量（private.rs:154-158 累加）');
    expectDecimal(
      poItems1[0],
      'received_quantity_alt',
      80,
      'PO行已收辅量=对话框实录 80（辅量断链靶心：收货采集→订单行累加）'
    );
    expectKeyValue(
      poAfter1,
      'status',
      'PARTIAL_RECEIVED',
      `部分收货状态词值（determine_order_receipt_status private.rs:172-196，8<20）`
    );

    // ③库存四维行回写（warehouse 取自入库单权威值，按 款号+批次 过滤命中本例行）
    const whId = requireNum(rcpt1After.warehouse_id, '入库单仓库');
    const stockRows = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/inventory/stock?product_id=${getCtx().productIds[0]}&batch_no=${batch1}&warehouse_id=${whId}&page=1&page_size=10`
      ),
      'items',
      '收货后四维库存行'
    );
    const stock = stockRows.find(r => r.batch_no === batch1);
    expect(
      stock,
      `确认入库后应存在批次 ${batch1} 的库存行，实际=${JSON.stringify(stockRows)}`
    ).toBeTruthy();
    expectDecimal(stock!, 'quantity_on_hand', 8, '库存现存量=收货 8（upsert_stock_for_item）');
    expectKeyValue(
      stock!,
      'grade',
      '一等品',
      '库存行等级默认值（receipt_item_stock_key 缺省一等品）'
    );

    // ④采购发票＝入库确认自动生成应付（ap_invoice_ops/receipt.rs:71-175）
    const apList1 = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/ap/invoices?supplier_id=${getCtx().supplierId}&page=1&page_size=100`
      ),
      'items',
      '应付发票列表'
    );
    const apInv1 = apList1.find(r => Number(r.source_id) === rcptId);
    expect(
      apInv1,
      `入库单 ${rcptId} 确认后应自动生成应付单（receipt.rs:71），实际列表 source_id=${apList1
        .map(r => r.source_id)
        .join(',')}`
    ).toBeTruthy();
    expectKeyValue(apInv1!, 'source_type', 'PURCHASE_RECEIPT', '自动应付来源类型（receipt.rs:43）');
    expectDecimal(
      apInv1!,
      'amount',
      Number(rcpt1After.total_amount),
      '应付金额=入库单总额（receipt.rs:172）'
    );
    expectDecimal(apInv1!, 'amount', 120, '自动应付金额=8×15.00=120');
    expectKeyValue(apInv1!, 'invoice_status', 'DRAFT', '自动应付初始 DRAFT');
    const apInvId1 = requireNum(apInv1!.id, '自动应付单 id');

    // ── UI 收货 12 件 / 辅量 120（收满 20）+ 确认 → 全收 COMPLETED ──
    const batch2 = `E2E-B2${genCode('RCV')}`;
    const rcptId2 = await receiveViaUI(page, orderNo, '12', '120', batch2);
    CLEANUP.push({ path: `/purchase/receipts/${rcptId2}`, label: `purchase_receipt#${rcptId2}` });
    await seedInspectionPass(page, {
      receiptId: rcptId2,
      supplierId: getCtx().supplierId!,
      context: '20-01 末批收货单',
    });
    await apiCall(page, 'POST', `/purchase/receipts/${rcptId2}/confirm`);
    const poAfter2 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/orders/${poId}`
    );
    const poItems2 = pickListArray<Record<string, unknown>>(
      { items: poAfter2.items },
      'items',
      `PO ${poId} 收满后明细`
    );
    expectDecimal(poItems2[0], 'received_quantity', 20, '全收后已收主量=20');
    expectDecimal(
      poItems2[0],
      'received_quantity_alt',
      200,
      '全收后已收辅量=80+120，与订购辅量 200（quantity_alt_ordered）逐值对齐'
    );
    expectKeyValue(
      poAfter2,
      'status',
      'COMPLETED',
      '全收状态词值（is_fully_received → COMPLETED）'
    );

    // ── 付款申请（10-ap 同源词表：DRAFT→APPROVING→APPROVED）──
    await apiCall(page, 'POST', `/ap/invoices/${apInvId1}/approve`);
    const req1 = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payment-requests', {
      supplier_id: getCtx().supplierId,
      request_date: new Date().toISOString().slice(0, 10),
      payment_type: '货款',
      payment_method: '银行转账',
      request_amount: 120,
      currency: 'CNY',
      exchange_rate: 1,
      notes: `E2E-F20-REQ-${orderNo}`,
      items: [{ invoice_id: apInvId1, apply_amount: 120 }],
    });
    const reqId = requireNum(req1.id, '建付款申请');
    CLEANUP.push({ path: `/ap/payment-requests/${reqId}`, label: `ap_request#${reqId}` });
    const reqDetail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payment-requests/${reqId}`
    );
    expectKeyValue(reqDetail, 'approval_status', 'DRAFT', '申请初始');
    expectDecimal(reqDetail, 'request_amount', 120, '申请金额=发票额 120（③金额一致前置）');
    await apiCall(page, 'POST', `/ap/payment-requests/${reqId}/submit`);
    const reqSub = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payment-requests/${reqId}`
    );
    expectKeyValue(reqSub, 'approval_status', 'APPROVING', 'submit 回读');
    await apiCall(page, 'POST', `/ap/payment-requests/${reqId}/approve`);
    const reqApp = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payment-requests/${reqId}`
    );
    expectKeyValue(reqApp, 'approval_status', 'APPROVED', 'approve 回读');

    // ── 付款单 + 确认 → 金额与发票一致、发票联动 PAID ──
    const pay = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payments', {
      request_id: reqId,
      payment_date: new Date().toISOString().slice(0, 10),
      notes: `E2E-F20-PAY-${orderNo}`,
    });
    const payId = requireNum(pay.id, '建付款单');
    CLEANUP.push({ path: `/ap/payments/${payId}`, label: `ap_payment#${payId}` });
    const payDetail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payments/${payId}`
    );
    expectKeyValue(
      payDetail,
      'payment_status',
      'REGISTERED',
      '付款单初始（ap_payment_service.rs）'
    );
    expectDecimal(payDetail, 'payment_amount', 120, '付款金额取申请额=发票额（服务 :752-766）');
    // ③"付款金额与发票一致"：跨三个权威源同值锁死
    const invBeforePay = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/invoices/${apInvId1}`
    );
    expectDecimal(
      invBeforePay,
      'amount',
      Number(payDetail.payment_amount),
      '发票额与付款额逐值一致'
    );
    await apiCall(page, 'POST', `/ap/payments/${payId}/confirm`);
    const payAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payments/${payId}`
    );
    expectKeyValue(
      payAfter,
      'payment_status',
      'CONFIRMED',
      'confirm 回读（ap_payment_service.rs:223-226）'
    );
    const invAfterPay = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/invoices/${apInvId1}`
    );
    expectDecimal(invAfterPay, 'paid_amount', 120, 'confirm 后发票已付=全额');
    expectKeyValue(invAfterPay, 'invoice_status', 'PAID', '已付满额后发票词值 PAID');
  });

  test('20-02 门控与幂等负例：非 DRAFT 重复确认被拒且库存/进度无漂移；空批次整单拒且不落库存；申请无明细 submit 被拒', async ({
    page,
  }) => {
    const { id: poId, order_no: orderNo } = await seedPo(page, 'APPROVED');
    const batch = `E2E-BX${genCode('RCV')}`;
    // 辅量录显式 0：合法实收值（本批仅按主单位计量），同时锁 0 不被后续门控误伤
    const rcptId = await receiveViaUI(page, orderNo, '5', '0', batch);
    // 门控前置：先质检合格，首次确认才会成功；否则下面的"重复确认被 DRAFT 状态门拒"
    // 会被质检门代劳，两条断言形状相同 → 因错误的原因通过。
    await seedInspectionPass(page, {
      receiptId: rcptId,
      supplierId: getCtx().supplierId!,
      context: '20-02 收货单',
    });

    await apiCall(page, 'POST', `/purchase/receipts/${rcptId}/confirm`);
    const rcpt = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${rcptId}`
    );
    expectKeyValue(rcpt, 'receipt_status', 'COMPLETED', '首次确认后');
    CLEANUP.push({ path: `/purchase/receipts/${rcptId}`, label: `purchase_receipt#${rcptId}` });

    // 重复 confirm：DRAFT 门（state.rs:84-88 business → 脱敏站点只锁 code）
    const dup = await apiCallExpectFail(page, 'POST', `/purchase/receipts/${rcptId}/confirm`);
    expect(dup.status, `重复确认应 400，实际=${dup.status} body=${JSON.stringify(dup)}`).toBe(400);
    expect(failureCode(dup), '重复确认机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      dup.message,
      `重复确认的拒绝原因不应是质检门文案（说明首次确认未成功）：${dup.message}`
    ).not.toMatch(/质检尚未完成|质检不合格/);
    // 无漂移：库存行与订单已收量不得被二次累加（幂等防御，purchase/13-02b 同源契约）
    const poRows = pickListArray<Record<string, unknown>>(
      {
        items: (await apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/orders/${poId}`))
          .items,
      },
      'items',
      '重复确认后 PO 明细'
    );
    expectDecimal(poRows[0], 'received_quantity', 5, '重复确认被拒后已收量不应翻倍');

    // 空批次入库单：建单可过（CreateReceiptItemRequest 维度为可空列），确认整单业务拒
    // （require_receipt_batch private.rs:183-196 fail-closed，不落任何库存行）
    const ctx = getCtx();
    const prod = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/products/${ctx.productIds[0]}`
    );
    const bad = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/receipts', {
      supplier_id: ctx.supplierId,
      order_id: poId,
      receipt_date: new Date().toISOString().slice(0, 10),
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      notes: `E2E-F20-NOBATCH-${genCode('NB')}`,
      items: [
        {
          line_no: 1,
          material_id: ctx.productIds[0],
          material_code: prod.code,
          material_name: prod.name,
          batch_no: '   ',
          quantity: '3',
          quantity_alt: '0',
          unit_master: prod.unit ?? '米',
          unit_price: '15.00',
        },
      ],
    });
    const badId = requireNum(bad.id, '建空批次入库单');
    CLEANUP.push({ path: `/purchase/receipts/${badId}`, label: `purchase_receipt#${badId}` });
    // 空批次负例的归因前置：这张单必须"已质检合格但批次为空白"。
    // 否则确认会先被质检门（PENDING）拒，而质检门与批次门的出参形状完全相同
    // （400 + BUSINESS_ERROR + 仍 DRAFT），用例表面绿、实际验的已不是空批次 fail-closed
    // （require_receipt_batch，purchase_receipt_private.rs:231-244）。
    await seedInspectionPass(page, {
      receiptId: badId,
      supplierId: ctx.supplierId!,
      context: '20-02 空批次收货单',
    });

    const failConfirm = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/receipts/${badId}/confirm`
    );
    expect(failConfirm.status, `空批次确认应 400，实际=${failConfirm.status}`).toBe(400);
    expect(failureCode(failConfirm), '空批次确认机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      failConfirm.message,
      `空批次确认的拒绝原因不应是质检门文案：${failConfirm.message}`
    ).not.toMatch(/质检尚未完成|质检不合格/);
    const badAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${badId}`
    );
    expectKeyValue(
      badAfter,
      'receipt_status',
      'DRAFT',
      '确认被拒后入库单应仍 DRAFT（fail-closed 无痕）'
    );

    // 付款申请无明细 submit 被拒（ap_payment_request_service.rs:336-338，10-ap 头注释同源）
    const noItem = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payment-requests', {
      supplier_id: ctx.supplierId,
      request_date: new Date().toISOString().slice(0, 10),
      payment_type: '货款',
      payment_method: '银行转账',
      request_amount: 50,
    });
    const noItemId = requireNum(noItem.id, '建无明细申请');
    CLEANUP.push({ path: `/ap/payment-requests/${noItemId}`, label: `ap_request#${noItemId}` });
    const failSub = await apiCallExpectFail(
      page,
      'POST',
      `/ap/payment-requests/${noItemId}/submit`
    );
    expect(failSub.status, `无明细 submit 应 400，实际=${failSub.status}`).toBe(400);
    expect(failureCode(failSub), '无明细 submit 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const noItemAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payment-requests/${noItemId}`
    );
    expectKeyValue(
      noItemAfter,
      'approval_status',
      'DRAFT',
      'submit 被拒后应无痕（不得漂移到 APPROVING）'
    );

    // 不存在站点：GET 404 + NOT_FOUND（统一失败信封，utils/error.rs）
    const gone = await apiCallExpectFail(page, 'GET', '/purchase/receipts/2147483647');
    expect(gone.status, '不存在入库单 GET 应 404').toBe(404);
    expect(failureCode(gone), '不存在机器码').toBe('NOT_FOUND');
  });

  test('20-03 再编辑回读（缺陷①）：DRAFT PO 全键 PUT 后逐字段回读，币种/汇率/附件不丢', async ({
    page,
  }) => {
    const { id: poId } = await seedPo(page, 'DRAFT');
    const marker = genCode('F20U');

    // UpdatePurchaseOrderRequest 全键（services/po/mod.rs:94-108，单层 Option：全送避免 None 语义歧义）
    await apiCall(page, 'PUT', `/purchase/orders/${poId}`, {
      supplier_id: getCtx().supplierId,
      order_date: new Date().toISOString().slice(0, 10),
      expected_delivery_date: new Date(Date.now() + 21 * 86400000).toISOString().slice(0, 10),
      warehouse_id: getCtx().warehouseIds[0],
      department_id: getCtx().departmentIds[0],
      currency: 'EUR',
      exchange_rate: '6.55',
      payment_terms: 'NET60',
      shipping_terms: 'CIF',
      notes: `E2E-F20-UPD-${marker}`,
      attachment_urls: ['https://example.com/po-updated.pdf'],
    });
    const detail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/orders/${poId}`
    );
    expectKeyValue(detail, 'currency', 'EUR', 'PO 改后币种（丢字段=缺陷①）');
    expectDecimal(detail, 'exchange_rate', 6.55, 'PO 改后汇率');
    expectKeyValue(detail, 'payment_terms', 'NET60', 'PO 改后付款条件');
    expectKeyValue(detail, 'shipping_terms', 'CIF', 'PO 改后运输条款');
    expectKeyValue(detail, 'notes', `E2E-F20-UPD-${marker}`, 'PO 改后备注');
    // 缺陷①回归钉（本波源码修复后应为绿）：attachment_urls 创建/更新真实落库
    // （purchase_order.rs:87 列；写入 services/po/order_ops/crud.rs:268 / :591-592），
    // 出参 PurchaseOrderDto（services/po/order.rs）已补同名键（键名与模型列同源
    // snake_case，Entity::find() 全列 SELECT 按列名直映，无二次查询）。
    // 本断言再红即回归（DTO 丢键 ⇒ 再编辑附件显示不出来），禁止放宽为"键缺失容忍"。
    if (!Array.isArray(detail.attachment_urls)) {
      throw new Error(
        `PO 详情缺 attachment_urls 数组（保存数据不完整/再编辑显示不出来，缺陷①回归）；` +
          `根因应为 backend/src/services/po/order.rs PurchaseOrderDto 丢失 attachment_urls 键，` +
          `而写入侧 crud.rs:268/591-592 仍落库。实际键=${Object.keys(detail).join(',')}`
      );
    }
    expectKeyValue(
      detail,
      'attachment_urls',
      ['https://example.com/po-updated.pdf'],
      'PO 改后附件'
    );
    // 改头不动行：明细仍在（防止 PUT 全量替换语义误伤行的历史回归）
    const items = pickListArray<Record<string, unknown>>(
      { items: detail.items },
      'items',
      `PO ${poId} 改后明细`
    );
    expect(items.length, 'PUT 表头不应吞掉明细行').toBe(1);
    expectDecimal(items[0], 'quantity_alt', 200, 'PUT 后辅量保持 200');
    expectKeyValue(detail, 'status', 'DRAFT', 'PUT 不应推进状态');
  });
});
