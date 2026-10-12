// 交易域全流程契约级 E2E — 21 销售到收款（报价→转销售订单→发货→开票→收款）
//
// 定位：交易域 S2C 全链契约回读。靶心：
//   ①「创建保存不完整、再编辑显示不出来」→ 转订单后逐字段回读订单明细的数量/单价/金额/
//      **换算字段 quantity_meters/quantity_kg**（历史丢辅量字段回归防线，与 quotations/13 同源口径）；
//      AR 发票源单关联键（source_bill_id/source_bill_no/sales_order_no）逐一回读。
//   ②「提交/保存报请求错误」→ 每个负例锁具体 status + 具体机器码
//      （BUSINESS_ERROR/BAD_REQUEST/NOT_FOUND），禁"任意 4xx"。
//   ③「发货后库存真的扣减」→ UI 之外以 API 触发真实出库，四维行前后差值逐项断言，禁只断 toast。
// 取数口径：本例 API 自建唯一单（唯一 notes 标记 + 后端生成 order_no/quotation_no/invoice_no），
// 按单号回查精确锚定，afterEach 尽力清理；不依赖共享 seed 行。
//
// 契约真值源：
//   报价状态词表（小写）    backend/src/models/status/sales.rs:75-90（draft/approved/rejected/cancelled）
//     + :106-116 ext（pending_approval/converted/expired）；转订单门 services/quotation_convert_service.rs:121-124
//     （仅 approved 可转，business → 脱敏站点只锁 code）
//   转订单字段翻译         services/quotation_convert_service.rs::copy_quotation_items_to_order
//     （quantity/unit_price/subtotal/total_amount/quantity_meters/quantity_kg ← 报价行+DualUnitConverter；
//      quotations/13-01 已钉同源断言基准：qty40×12.5、米 40、公斤 40*180*150/100000=10.8）
//   销售订单状态词表（小写） models/status/sales.rs:14-33（draft/pending/approved/partial_shipped/shipped/completed…）
//   发货门/状态推导         services/so/delivery_ops/ship.rs:111（仅 approved 可发货）、
//     :462-482（全发 shipped / 部分 partial_shipped）、发货单 delivery status=shipped（:182，
//     sales_delivery 词表 sales.rs:46-52 小写 pending/shipped/cancelled）
//   四维出库扣减            services/so/delivery_ops/inventory.rs::reduce_inventory_four_dim
//     （quantity_available -= 出库量、quantity_shipped += 出库量、quantity_on_hand 守恒；
//      出库对染色布强制四维=缸号/色号/批次/匹号（用户 2026-10-02 口径），缺维由
//      require_outbound_dimensions 判 VALIDATION 族且外显真实原因（fix(outbound) 后），
//      匹号未命中真实可用匹属 BUSINESS 族）
//   路径-载荷一致性门       handlers/sales_order_handler.rs:418-420（不一致 400 BAD_REQUEST，真实 message 外显）
//   发货单回读形状          handlers/sales_order_handler.rs:687-702：data = {list,total}（**非** items！
//     信封键名以 handler 出参为准，前端读取键必须同源，列入契约表）
//   AR 发票                 handlers/ar_invoice_handler.rs:46-58 CreateArInvoiceRequestDto（无 tax_amount/
//     quantity_meters/quantity_kg 键 → 该域列无法录入，fullflow/11-ar 已钉，本文件不复断）；
//     状态门 services/ar_invoice_service.rs:330（update 仅 DRAFT）、:405-428（approve DRAFT→APPROVED）、
//     词表 models/status/general.rs:16-46（DRAFT/APPROVED/PAID/PARTIAL_PAID/CANCELLED 大写）
//   AR 收款                 handlers/ar_payment_handler.rs:31-46 CreateArPaymentRequest；
//     创建即关联发票按剩余未收量分摊 services/ar_ops/collection.rs:138-146/:267-367
//     （received_amount += allocate、unpaid 重算、derive 状态 PAID/PARTIAL_PAID）；
//     收款状态词表小写 pending/confirmed/cancelled models/status/finance.rs:11-19；
//     confirm 仅 pending（collection.rs:486-497）
// 诚实标注：AR 收款**不含核销端点联动**（received 累加发生在 create_payment 的 invoice_ids 分摊，
//   非 confirm 时），测试按该真实时序断言；发货走 API 动作端点（UI DeliveryDialog 已由
//   sales/04、sales/13 覆盖），本文件不重复脆弱 UI，专注权威状态回读。
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
  pickDyeableWarehouse,
  seedDyedOutboundBundle,
  readDyedPieceByNo,
  APP_ERROR_CODES,
} from '../flow/helpers';
import { pickListArray } from '../flow/ui-helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function expectKeyValue(
  obj: Record<string, unknown>,
  key: string,
  expected: unknown,
  label: string
) {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺少后端真实键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  expect(
    obj[key],
    `${label}：键 "${key}" 期望=${JSON.stringify(expected)} 实际=${JSON.stringify(obj[key])}`
  ).toEqual(expected);
}

function expectDecimal(obj: Record<string, unknown>, key: string, expected: number, label: string) {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：缺金额/数量键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n)) {
    throw new Error(`${label}：键 "${key}" 不可解析，raw=${JSON.stringify(obj[key])}`);
  }
  expect(Math.abs(n - expected), `${label}：${key} 期望=${expected} 实际=${n}`).toBeLessThan(0.005);
}

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0)
    throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

/**
 * 自建「值可辨识」草稿报价单（qty=40 / 单价 12.5 / 税 0，与 quotations/13 同基准），
 * 推进到 approved（真实状态机），返回 {id, quotation_no, status}。
 * 报价行 unit 必须逐字符等于产品交易单位（后端 validate_item_units_against_products），
 * 故一律引用 ctx.quotationProductId/quotationProductUnit。
 */
async function seedApprovedQuotation(page: Page): Promise<{ id: number; quotation_no: string }> {
  const ctx = getCtx();
  if (!ctx.quotationProductId) throw new Error('前置缺失：ctx.quotationProductId 未就绪');
  if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
  if (!ctx.userIds[0]) throw new Error('前置缺失：ctx.userIds[0] 未就绪');
  const marker = genCode('F21');

  const res = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/quotations', {
    customer_id: ctx.customerId,
    sales_user_id: ctx.userIds[0],
    quotation_date: new Date().toISOString().slice(0, 10),
    valid_until: new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10),
    currency: 'CNY',
    exchange_rate: '1',
    base_currency: 'CNY',
    price_terms: 'FOB',
    tax_inclusive: false,
    tax_rate: '0',
    items: [
      {
        product_id: ctx.quotationProductId,
        unit: ctx.quotationProductUnit,
        quantity: '40',
        unit_price: '12.5',
        unit_price_with_tax: '12.5',
      },
    ],
    notes: `E2E-F21-${marker}`,
  });
  const id = requireNum(res.id, `建报价单（${marker}）`);
  CLEANUP.push({ path: `/quotations/${id}`, label: `quotation#${id}` });

  await apiCall(page, 'POST', `/quotations/${id}/submit`);
  const after = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/quotations/${id}`);
  // 词表：sales.rs:75-90 + :106-116（小写 draft/pending_approval/approved/…）
  const st = String(after.status ?? '');
  if (st !== 'approved') {
    // 走 BPM 审批态则补 approve（真实流转，非跳过）
    await apiCall(page, 'POST', `/quotations/${id}/approve`);
    const r = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/quotations/${id}`);
    if (String(r.status) !== 'approved') {
      throw new Error(
        `报价单 submit/approve 后应为 approved（实际 ${JSON.stringify(r.status)}，id=${id}）`
      );
    }
  }
  const quotation_no = String(after.quotation_no ?? res.quotation_no ?? '');
  if (!quotation_no)
    throw new Error(`报价单缺 quotation_no，实际键=${Object.keys(after).join(',')}`);
  return { id, quotation_no };
}

/** 自建 approved 销售订单（qty=10，产品=productIds[1] 专用产品，避免并发分片污染库存行） */
async function seedApprovedSO(
  page: Page,
  productId: number,
  qty: string
): Promise<{ id: number; order_no: string }> {
  const ctx = getCtx();
  if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
  const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/sales/orders', {
    customer_id: ctx.customerId,
    order_date: new Date().toISOString(),
    items: [{ product_id: productId, quantity: qty, unit_price: '25.00' }],
    notes: `E2E-F21-SO-${genCode('S1')}`,
  });
  const id = requireNum(created.id, '建销售订单');
  CLEANUP.push({ path: `/sales/orders/${id}`, label: `so#${id}` });
  await apiCall(page, 'POST', `/sales/orders/${id}/submit`);
  const sub = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/sales/orders/${id}`);
  expectKeyValue(sub, 'status', 'pending', 'submit 回读（词表 sales.rs:14-33 小写）');
  await apiCall(page, 'POST', `/sales/orders/${id}/approve`);
  const app = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/sales/orders/${id}`);
  expectKeyValue(app, 'status', 'approved', 'approve 回读');
  return { id, order_no: String(app.order_no) };
}

test.describe('21 销售到收款全流程契约链', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await ensureTestEntities(page);
  });

  test('21-01 主链：报价 approved→convert→订单/换算字段回读→发货两笔→库存四维扣减→AR 发票→收款分摊→PAID', async ({
    page,
  }) => {
    const ctx = getCtx();
    const q = await seedApprovedQuotation(page);

    // ── 转销售订单 ──
    await apiCall(page, 'POST', `/quotations/${q.id}/convert`);
    const qAfter = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/quotations/${q.id}`);
    expectKeyValue(qAfter, 'status', 'converted', '转单后报价状态词值（sales.rs:111）');
    const soId = requireNum(qAfter.converted_sales_order_id, '报价单 converted_sales_order_id');
    CLEANUP.push({ path: `/sales/orders/${soId}`, label: `so(converted)#${soId}` });

    // ①缺陷靶心：订单明细逐字段回读（含换算字段，历史 quantity_alt/辅量丢字段回归防线）
    const order0 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/sales/orders/${soId}`);
    expectKeyValue(order0, 'status', 'draft', '转生成的订单应为 draft（真实链路起点）');
    const lines0 = pickListArray<Record<string, unknown>>(
      { items: order0.items },
      'items',
      `订单 ${soId} 明细`
    );
    expect(lines0.length, '转单应带出 1 行明细').toBe(1);
    expectDecimal(lines0[0], 'quantity', 40, '订单行数量翻译自报价行 40');
    expectDecimal(lines0[0], 'unit_price', 12.5, '订单行单价翻译自报价行');
    expectDecimal(lines0[0], 'subtotal', 500, '订单行小计=40×12.5=500');
    expectDecimal(lines0[0], 'total_amount', 500, '税 0 时总额=小计');
    expectDecimal(
      lines0[0],
      'quantity_meters',
      40,
      '换算米数（丢字段即红，quotation_convert_service.rs::copy_quotation_items_to_order）'
    );
    expectDecimal(
      lines0[0],
      'quantity_kg',
      10.8,
      '换算公斤数=40×180×150/100000=10.8（DualUnitConverter 口径同 quotations/13-01）'
    );
    expectKeyValue(lines0[0], 'product_id', ctx.quotationProductId, '订单行产品外键未被翻译丢失');
    expectDecimal(order0, 'total_amount', 500, '订单头总额翻译自报价总额');

    // 订单状态机推进到 approved（发货前置）
    await apiCall(page, 'POST', `/sales/orders/${soId}/submit`);
    await apiCall(page, 'POST', `/sales/orders/${soId}/approve`);

    // ── 四维库存备货（专用：quotationProductId + 唯一色/缸/批，本例可精确归因）──
    // 出库对染色布强制四维=缸号/色号/批次/匹号（用户 2026-10-02 口径）：批次必须等于缸号
    // ——写入方 piece_domain_service.rs:540 生成染色匹恒 batch_no=dye_lot_no=缸号，旧写法
    // batch=`E21B…` 独立值按该 tuple 造不出真实匹。改用 helpers.seedDyedOutboundBundle：
    // batch=缸号 库存行 40 米 + 委外染色真实链同维 2 匹 AVAILABLE（发料逐匹/回仓确认，flow/07 先例）。
    const target = await pickDyeableWarehouse(page);
    const bundle = await seedDyedOutboundBundle(page, {
      productId: ctx.quotationProductId as number,
      warehouseId: target.id,
      quantityMeters: '40',
      pieceCount: 2,
      context: 'E21-01',
    });
    if (bundle.pieces.length < 2)
      throw new Error(
        `[21-01] 前置失败：真实链未产出 2 匹可出库染色匹（实际 ${bundle.pieces.length}）`
      );
    const dim = {
      colorNo: bundle.colorNo,
      dyeLotNo: bundle.dyeLotNo,
      batchNo: bundle.dyeLotNo, // 写入方口径：染色匹 批次=缸号
    };
    const warehouseId = target.id;
    const whCode = target.code;
    if (!whCode) throw new Error(`仓库 ${warehouseId} 缺 warehouse_code（models/warehouse.rs:11）`);

    const readStock = async (): Promise<Record<string, unknown>> => {
      const rows = pickListArray<Record<string, unknown>>(
        await apiCallRaw<Record<string, unknown>>(
          page,
          'GET',
          `/inventory/stock?product_id=${ctx.quotationProductId}&color_no=${dim.colorNo}&dye_lot_no=${dim.dyeLotNo}&batch_no=${dim.batchNo}&warehouse_id=${warehouseId}&page=1&page_size=10`
        ),
        'items',
        '四维库存行'
      );
      const row = rows.find(
        r =>
          r.color_no === dim.colorNo && r.batch_no === dim.batchNo && r.dye_lot_no === dim.dyeLotNo
      );
      if (!row) throw new Error(`四维库存行缺失：实际=${JSON.stringify(rows)}`);
      return row;
    };
    const before = await readStock();
    expectDecimal(before, 'quantity_available', 40, '发货前可用量');

    // ── 发货两笔：15 → partial_shipped；再 25 → shipped；库存与匹状态逐项回读 ──
    // 染色布出库第四维=匹号（用户 2026-10-02 口径）：两笔各带真实链产出的一匹，
    // 出库后该匹必须 AVAILABLE→SHIPPED（piece_domain_service.rs:674 CAS 消耗）。
    await apiCall(page, 'POST', `/sales/orders/${soId}/ship`, {
      order_id: soId,
      warehouse_code: whCode,
      items: [
        {
          product_id: ctx.quotationProductId,
          quantity: '15',
          batch_no: dim.batchNo,
          color_no: dim.colorNo,
          dye_lot_no: dim.dyeLotNo,
          piece_no: bundle.pieces[0].piece_no,
        },
      ],
    });
    const o1 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/sales/orders/${soId}`);
    expectKeyValue(o1, 'status', 'partial_shipped', '部分发货词值（ship.rs:462-465）');
    const s1 = await readStock();
    expectDecimal(
      s1,
      'quantity_available',
      25,
      '发货 15 后可用量=40-15（reduce_inventory_four_dim）'
    );
    expectDecimal(s1, 'quantity_shipped', 15, '发货 15 后已发量 +=15');
    expectDecimal(
      s1,
      'quantity_on_hand',
      Number(before.quantity_on_hand),
      '现存量守恒（on_hand=available+shipped，不因出库变动）'
    );
    const p1 = await readDyedPieceByNo(page, {
      productId: ctx.quotationProductId as number,
      warehouseId,
      dyeLotNo: dim.dyeLotNo,
      batchNo: dim.batchNo,
      pieceNo: bundle.pieces[0].piece_no,
    });
    expect(p1, `第一笔发货后应回读到匹 ${bundle.pieces[0].piece_no}`).toBeTruthy();
    expect(
      String(p1!.status),
      `匹 ${bundle.pieces[0].piece_no} 应被第一笔出库消耗为 SHIPPED（词表 inventory_piece 大写），实际 ${p1!.status}`
    ).toBe('SHIPPED');

    // 发货单回读：信封键 = {list,total}（sales_order_handler.rs:687-702，**非** items——
    // 前端若按 items 读该端点将恒空，列入契约表）
    const dlv = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/sales/orders/${soId}/deliveries`
    );
    for (const k of ['list', 'total']) {
      expect(
        Object.prototype.hasOwnProperty.call(dlv, k),
        `发货单回读应含信封键 "${k}"（handler:696-699），实际键=${Object.keys(dlv).join(',')}`
      ).toBe(true);
    }
    const dlvRows = pickListArray<Record<string, unknown>>(dlv, 'list', '发货单列表');
    expect(dlvRows.length, '第一笔发货后应恰有 1 张发货单').toBe(1);
    expectKeyValue(
      dlvRows[0],
      'status',
      'shipped',
      '发货单词值（sales_delivery.shipped，ship.rs:182）'
    );

    await apiCall(page, 'POST', `/sales/orders/${soId}/ship`, {
      order_id: soId,
      warehouse_code: whCode,
      items: [
        {
          product_id: ctx.quotationProductId,
          quantity: '25',
          batch_no: dim.batchNo,
          color_no: dim.colorNo,
          dye_lot_no: dim.dyeLotNo,
          piece_no: bundle.pieces[1].piece_no,
        },
      ],
    });
    const o2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/sales/orders/${soId}`);
    expectKeyValue(o2, 'status', 'shipped', '全发后订单词值 shipped（ship.rs:462-463）');
    const s2 = await readStock();
    expectDecimal(s2, 'quantity_available', 0, '全发后可用量归零');
    expectDecimal(s2, 'quantity_shipped', 40, '全发后已发量=40');
    const p2 = await readDyedPieceByNo(page, {
      productId: ctx.quotationProductId as number,
      warehouseId,
      dyeLotNo: dim.dyeLotNo,
      batchNo: dim.batchNo,
      pieceNo: bundle.pieces[1].piece_no,
    });
    expect(p2, `第二笔发货后应回读到匹 ${bundle.pieces[1].piece_no}`).toBeTruthy();
    expect(
      String(p2!.status),
      `匹 ${bundle.pieces[1].piece_no} 应被第二笔出库消耗为 SHIPPED，实际 ${p2!.status}`
    ).toBe('SHIPPED');

    // ── 开票：AR 发票（源单关联键全数回读）──
    const orderNo = String(o2.order_no);
    const inv = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/invoices', {
      customer_id: ctx.customerId,
      invoice_date: new Date().toISOString().slice(0, 10),
      due_date: new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10),
      invoice_amount: 500,
      source_type: 'SALES_ORDER',
      source_bill_id: soId,
      source_bill_no: orderNo,
      sales_order_no: orderNo,
    });
    const invId = requireNum(inv.id, '建 AR 发票');
    CLEANUP.push({ path: `/ar/invoices/${invId}`, label: `ar_invoice#${invId}` });
    const invD = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/invoices/${invId}`);
    expectKeyValue(invD, 'customer_id', ctx.customerId, 'AR 发票客户');
    expectDecimal(invD, 'invoice_amount', 500, 'AR 发票金额=订单总额（③金额一致）');
    expectKeyValue(invD, 'source_bill_id', soId, 'AR 发票源单 id（缺陷①靶心：关联键必须可回读）');
    expectKeyValue(invD, 'source_bill_no', orderNo, 'AR 发票源单号');
    expectKeyValue(invD, 'sales_order_no', orderNo, 'AR 发票销售单号');
    expectKeyValue(invD, 'status', 'DRAFT', 'AR 发票初始大写词值 DRAFT（general.rs:16）');
    expect(
      typeof invD.invoice_no === 'string' && (invD.invoice_no as string).length > 0,
      `invoice_no 应为后端生成非空串，实际=${JSON.stringify(invD.invoice_no)}`
    ).toBe(true);
    await apiCall(page, 'POST', `/ar/invoices/${invId}/approve`);
    const invA = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/invoices/${invId}`);
    expectKeyValue(invA, 'status', 'APPROVED', 'approve 回读（ar_invoice_service.rs:405-428）');

    // ── 收款：300 → PARTIAL_PAID；200 → PAID；金额与发票逐值一致 ──
    const pay1 = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/payments', {
      customer_id: ctx.customerId,
      amount: 300,
      payment_method: '银行转账',
      payment_date: new Date().toISOString().slice(0, 10),
      invoice_ids: [invId],
      remark: `E2E-F21-PAY1-${orderNo}`,
    });
    const pay1Id = requireNum(pay1.id, '建收款1');
    const pay1D = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/payments/${pay1Id}`);
    expectKeyValue(pay1D, 'status', 'pending', '收款单初始小写词值（finance.rs:13）');
    expectDecimal(pay1D, 'amount', 300, '收款金额 300');
    const invP1 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/invoices/${invId}`);
    expectDecimal(
      invP1,
      'received_amount',
      300,
      'create_payment 分摊即累加已收（collection.rs:138-146/:348）'
    );
    expectDecimal(invP1, 'unpaid_amount', 200, '未收=500-300');
    expectKeyValue(invP1, 'status', 'PARTIAL_PAID', '部分收讫词值（general.rs:46）');

    await apiCall(page, 'POST', `/ar/payments/${pay1Id}/confirm`);
    const pay1C = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/payments/${pay1Id}`);
    expectKeyValue(pay1C, 'status', 'confirmed', 'confirm 回读（collection.rs:505-511）');

    const pay2 = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/payments', {
      customer_id: ctx.customerId,
      amount: 200,
      payment_method: '银行转账',
      payment_date: new Date().toISOString().slice(0, 10),
      invoice_ids: [invId],
      remark: `E2E-F21-PAY2-${orderNo}`,
    });
    const pay2Id = requireNum(pay2.id, '建收款2');
    const invFinal = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/invoices/${invId}`
    );
    expectDecimal(
      invFinal,
      'received_amount',
      500,
      '两笔收款合计=发票额 500（③付款金额与发票一致）'
    );
    expectDecimal(invFinal, 'unpaid_amount', 0, '未收归零');
    expectKeyValue(invFinal, 'status', 'PAID', '全额收讫词值（general.rs:43）');
    void pay2Id;
  });

  test('21-02 负例：未 approved 报价转单被拒、draft 订单发货被拒、路径/载荷不一致 400、四维缺失与库存不足均业务拒且无痕', async ({
    page,
  }) => {
    const ctx = getCtx();

    // A) draft 报价直接 convert → 400 BUSINESS_ERROR（quotation_convert_service.rs:121-124）
    if (!ctx.quotationProductId || !ctx.customerId || !ctx.userIds[0]) {
      throw new Error('前置缺失：报价域 ctx 未就绪');
    }
    const qDraft = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/quotations', {
      customer_id: ctx.customerId,
      sales_user_id: ctx.userIds[0],
      quotation_date: new Date().toISOString().slice(0, 10),
      valid_until: new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10),
      currency: 'CNY',
      exchange_rate: '1',
      base_currency: 'CNY',
      price_terms: 'FOB',
      tax_inclusive: false,
      tax_rate: '0',
      items: [
        {
          product_id: ctx.quotationProductId,
          unit: ctx.quotationProductUnit,
          quantity: '10',
          unit_price: '5',
          unit_price_with_tax: '5',
        },
      ],
      notes: `E2E-F21-NEG-${genCode('N')}`,
    });
    const qId = requireNum(qDraft.id, '建草稿报价');
    CLEANUP.push({ path: `/quotations/${qId}`, label: `quotation(neg)#${qId}` });
    const fConv = await apiCallExpectFail(page, 'POST', `/quotations/${qId}/convert`);
    expect(
      fConv.status,
      `draft 转单应 400，实际=${fConv.status} body=${JSON.stringify(fConv)}`
    ).toBe(400);
    expect(failureCode(fConv), 'draft 转单机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const qStill = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/quotations/${qId}`);
    expectKeyValue(qStill, 'status', 'draft', '转单被拒后报价状态应无痕');

    // B) draft 销售订单发货 → 400 BUSINESS_ERROR（ship.rs:111 仅 approved）
    const so = await seedApprovedSO(page, ctx.productIds[1] ?? ctx.productIds[0], '10');
    // 先回退不可行，改为另建一张仅 submit 到 pending 的订单再拒发货？pending 也不能发货（同门）。
    // 用新建未提交订单（draft）：
    const draftSo = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      items: [
        { product_id: ctx.productIds[1] ?? ctx.productIds[0], quantity: '5', unit_price: '9' },
      ],
      notes: `E2E-F21-NEGSO-${genCode('N2')}`,
    });
    const draftId = requireNum(draftSo.id, '建 draft 销售订单');
    CLEANUP.push({ path: `/sales/orders/${draftId}`, label: `so(neg)#${draftId}` });
    const fShip = await apiCallExpectFail(page, 'POST', `/sales/orders/${draftId}/ship`, {
      order_id: draftId,
      warehouse_code: 'WH-IGNORE',
      items: [{ product_id: ctx.productIds[0], quantity: '1' }],
    });
    expect(fShip.status, `draft 订单发货应 400，实际=${fShip.status}`).toBe(400);
    expect(failureCode(fShip), 'draft 发货机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const dStill = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/sales/orders/${draftId}`
    );
    expectKeyValue(dStill, 'status', 'draft', '发货被拒后订单状态无痕');

    // C) approved 订单：路径 id 与 payload.order_id 不一致 → 400 BAD_REQUEST（handler:418-420）
    const whCode = String(
      (await apiCallRaw<Record<string, unknown>>(page, 'GET', `/warehouses/${ctx.warehouseIds[0]}`))
        .warehouse_code ?? ''
    );
    if (!whCode) throw new Error('仓库缺 warehouse_code');
    const fMismatch = await apiCallExpectFail(page, 'POST', `/sales/orders/${so.id}/ship`, {
      order_id: so.id + 1,
      warehouse_code: whCode,
      items: [{ product_id: ctx.productIds[1] ?? ctx.productIds[0], quantity: '1' }],
    });
    expect(fMismatch.status, '路径/载荷不一致应 400').toBe(400);
    expect(failureCode(fMismatch), '不一致机器码 BAD_REQUEST').toBe(APP_ERROR_CODES.BAD_REQUEST);

    // D) 缺维发货 → 400 VALIDATION_ERROR 且外显真实原因（inventory_deduction.rs:242
    //    require_outbound_dimensions → fabric_class.rs:42-58 批次必填；fix(outbound) 已把
    //    缺维拒绝改回可外显——旧期望 BUSINESS_ERROR 是"包装层把 VALIDATION 降级"时代的口径，
    //    新口径缺维属字段必填族 VALIDATION，状态门/充足性才是 BUSINESS；此处如实对齐口径，
    //    并钉文案非脱敏常量）
    const fDim = await apiCallExpectFail(page, 'POST', `/sales/orders/${so.id}/ship`, {
      order_id: so.id,
      warehouse_code: whCode,
      items: [{ product_id: ctx.productIds[1] ?? ctx.productIds[0], quantity: '1' }],
    });
    expect(fDim.status, `缺四维发货应 400，实际=${fDim.status}`).toBe(400);
    expect(
      failureCode(fDim),
      `缺维机器码应为 VALIDATION_ERROR（字段必填族），实际 code=${fDim.code ?? ''} message=${fDim.message ?? ''}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const fDimMsg = String(fDim.message ?? '');
    expect(fDimMsg, `缺维拒绝文案不得是脱敏常量，实际=${fDimMsg}`).not.toBe('请求参数验证失败');
    expect(fDimMsg, `缺维拒绝文案应点名缺失维度（批次/批号），实际=${fDimMsg}`).toMatch(/批/);

    // E) 四维行+匹齐备但数量不足（备货 3 < 发货 10）→ 400 BUSINESS_ERROR 且不得把可用量扣成负。
    //    出库对染色布强制四维=缸/色/批/匹（用户 2026-10-02 口径）：旧写法不带 piece_no，
    //    拒绝会先落在"缺维 VALIDATION"上，本命题（可用量门控）根本没跑到——假绿。
    //    现按写入方口径备货（batch=缸号 库存行 + 委外染色真实链同维 AVAILABLE 匹，
    //    helpers.seedDyedOutboundBundle），请求带真实匹号，让拒绝只能来自数量不足。
    const shortTarget = await pickDyeableWarehouse(page);
    const shortBundle = await seedDyedOutboundBundle(page, {
      productId: ctx.productIds[1] as number,
      warehouseId: shortTarget.id,
      quantityMeters: '3',
      pieceCount: 1,
      context: 'E21-02E',
    });
    const shortColor = shortBundle.colorNo;
    const shortLot = shortBundle.dyeLotNo;
    const shortBatch = shortBundle.dyeLotNo; // 写入方口径：染色匹 批次=缸号
    const fShort = await apiCallExpectFail(page, 'POST', `/sales/orders/${so.id}/ship`, {
      order_id: so.id,
      warehouse_code: shortTarget.code,
      items: [
        {
          product_id: ctx.productIds[1],
          quantity: '10',
          batch_no: shortBatch,
          color_no: shortColor,
          dye_lot_no: shortLot,
          piece_no: shortBundle.pieces[0].piece_no,
        },
      ],
    });
    expect(fShort.status, '四维合计不足发货应 400，实际响应=' + JSON.stringify(fShort)).toBe(400);
    expect(
      failureCode(fShort),
      `四维合计不足机器码（数量门控属 BUSINESS 族，非缺维 VALIDATION），实际 code=${fShort.code ?? ''} message=${fShort.message ?? ''}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const shortRows = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/inventory/stock?product_id=${ctx.productIds[1]}&color_no=${shortColor}&dye_lot_no=${shortLot}&batch_no=${shortBatch}&warehouse_id=${shortTarget.id}&page=1&page_size=10`
      ),
      'items',
      '不足发货后的库存行'
    );
    const shortRow = shortRows.find(r => r.batch_no === shortBatch);
    expect(shortRow, '不足发货不应吞掉原库存行').toBeTruthy();
    expectDecimal(
      shortRow!,
      'quantity_available',
      3,
      '发货被拒后可用量应原样为 3（fail-closed 无半扣）'
    );
    // 被拒的写无痕：匹未被消耗（不足拒绝发生在米数扣减规划期，匹 CAS 未执行/随事务回滚）
    const ePiece = await readDyedPieceByNo(page, {
      productId: ctx.productIds[1] as number,
      warehouseId: shortTarget.id,
      dyeLotNo: shortLot,
      batchNo: shortBatch,
      pieceNo: shortBundle.pieces[0].piece_no,
    });
    expect(ePiece, '不足发货被拒后应仍能回读到该匹').toBeTruthy();
    expect(String(ePiece!.status), '不足发货被拒后匹应仍 AVAILABLE（消耗未发生）').toBe(
      'AVAILABLE'
    );
    const oStill = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/sales/orders/${so.id}`);
    expectKeyValue(oStill, 'status', 'approved', '全部失败发货后订单应仍 approved（无半途漂移）');
  });

  test('21-03 AR 发票门控：非 DRAFT 不可改；DRAFT 不可 mark-as-paid（状态门+词值回读，负例无痕）', async ({
    page,
  }) => {
    const ctx = getCtx();
    if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
    const inv = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/invoices', {
      customer_id: ctx.customerId,
      invoice_date: new Date().toISOString().slice(0, 10),
      due_date: new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10),
      invoice_amount: 888.88,
    });
    const invId = requireNum(inv.id, '建 AR 发票');
    CLEANUP.push({ path: `/ar/invoices/${invId}`, label: `ar_invoice(gate)#${invId}` });

    // DRAFT 直接 mark-as-paid 被拒（白名单 APPROVED/PARTIAL_PAID，ar_invoice_service.rs:461-471，
    // 该处用 bad_request → 锁 BAD_REQUEST）
    const fPaid = await apiCallExpectFail(page, 'POST', `/ar/invoices/${invId}/mark-as-paid`);
    expect(fPaid.status, 'DRAFT mark-as-paid 应 400').toBe(400);
    expect(failureCode(fPaid), 'DRAFT mark-as-paid 机器码').toBe(APP_ERROR_CODES.BAD_REQUEST);

    // approve 后 PUT 被拒（update 仅 DRAFT，service:330）
    await apiCall(page, 'POST', `/ar/invoices/${invId}/approve`);
    const fUpd = await apiCallExpectFail(page, 'PUT', `/ar/invoices/${invId}`, {
      invoice_amount: 1,
    });
    expect(fUpd.status, 'APPROVED 后 PUT 应 400').toBe(400);
    const still = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/invoices/${invId}`);
    expectKeyValue(still, 'status', 'APPROVED', '改被拒后状态无痕迹');
    expectDecimal(still, 'invoice_amount', 888.88, '改被拒后金额原样（半改即红）');

    // 不存在站点 404 NOT_FOUND
    const gone = await apiCallExpectFail(page, 'GET', '/ar/invoices/2147483647');
    expect(gone.status, '不存在发票 GET 应 404').toBe(404);
    expect(failureCode(gone), '不存在机器码').toBe('NOT_FOUND');
  });
});
