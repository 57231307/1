// 销售发货到收款的深度链路用例，与 02 号 O2C 冒烟用例互补，02 只断链路走通与终态取值
//
// 本文件把四件事钉在真实后端读回上，全部打真实接口，不用路由拦截一类伪造响应的手段。
//   一、发货单按行数量与库存扣减读回。发货写入方是服务层 so 域 delivery_ops 的 ship 模块，
//       create_shipment_delivery 把发货单头 total_quantity 写为本次发货明细数量求和，
//       库存侧 reduce_inventory_four_dim 对命中的库存行做 quantity_available 精确减、
//       quantity_shipped 精确增，quantity_on_hand 不动，订单行 shipped_quantity 逐行回写。
//       本用例对这三处读回做前后精确比对，不接受只断接口返回 200。
//   二、发货状态词表逐字符比对。权威词表在 models 状态目录的销售模块，
//       sales_delivery 模块三值全为小写，pending、shipped、cancelled。
//       sales_order 模块的流转值也是全小写，draft、pending、approved、partial_shipped、shipped。
//       动作发货落 shipped，手动建单口 create_delivery 落 pending，两个写入方的读回值
//       都用 toBe 逐字符比较，不用 toLowerCase 归一，不放行词表外取值。
//   三、应收单据金额与订单金额同源。全额发货时 ship 流程在同一事务内调用
//       ar 域 inv 模块 create_receivable，发票 invoice_amount 直接取订单头 total_amount，
//       初始 received_amount 为零、unpaid_amount 等于全额、状态落大写 DRAFT、
//       审批态落大写 PENDING、source_type 为 SALES_ORDER、source_bill_no 与 sales_order_no
//       都等于订单号。本用例断言发票读回金额与订单详情读回金额逐分对齐，
//       而不是拿算式重算一个"应该的值"去比对，同源链路上任何一处漂移都会判红。
//   四、收款后核销金额读回。POST 收款携带 invoice_ids 时，collection 模块在同一事务内
//       逐张按剩余收款与发票未收取小分配并写核销单，核销单走与手工核销同一个写账本入口，
//       reconciliation_status 落小写 closed，total_invoices 与 total_collections 都等于分配额。
//       明细两行，INVOICE 行金额为正分配额、RECEIPT 行金额为负分配额，
//       两行 matched_amount 都等于分配额、match_status 落大写 MATCHED。
//       发票侧 received 等于分配额、unpaid 归零、状态落大写 PAID。
//       收款自身状态是小写 pending，确认动作才会转 confirmed，本用例不越过确认环节。
//
// 造数纪律。白坯口径出库免缸号免匹号，只强制批次，因此每支用例用 seedGreigeStockIn
// 建一行批次取值本轮唯一的白坯库存行，出库按该批次精确命中，共享库里其它库存行
// 不在四维候选集内，前后比对不受并发用例干扰。清理依赖批次唯一取值隔离，不做删除动作。
// 销售订单、发货仓库、客户全部来自本 worker 前置实体与真实列表读回，不硬编码任何实体 ID，
// 发货仓库编码从仓库列表按仓库 ID 反查真实 warehouse_code，取不到直接抛错不回落默认值。
//
// 已知真实行为声明。发货单头 total_amount 由写入方落固定零值且全链路无任何回填，
// 属未修复缺陷，已另行登记跟进。本用例对发货单只断数量维与状态维，
// 金额同源断言走应收发票与订单头 total_amount 这条链路，不拿缺陷列凑断言。
import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  ensureTestEntities,
  apiCallRaw,
  apiCallExpectFail,
  getCtx,
  genCode,
  seedGreigeStockIn,
  failureCode,
  APP_ERROR_CODES,
} from './helpers';
import { pickListArray } from './ui-helpers';

/** 读回响应里的一个键并转成数字，键缺席或非有限值直接抛错，不用缺省值掩盖缺键。 */
function readNumber(obj: Record<string, unknown>, key: string, label: string): number {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label} 响应缺少后端真实键 ${key}，实际键=${Object.keys(obj).join(',')}`);
  }
  const value = Number(obj[key]);
  if (!Number.isFinite(value)) {
    throw new Error(`${label} 键 ${key} 不可解析为有限数字，raw=${String(obj[key])}`);
  }
  return value;
}

/** 金额与数量按货币精度比较，容差百分之一，失败信息带期望与实际。 */
function expectQuantity(actual: number, expected: number, label: string): void {
  expect(Math.abs(actual - expected), `${label}，期望=${expected} 实际=${actual}`).toBeLessThan(
    0.01
  );
}

/** 状态逐字符断言，先把读回值 String 化再 toBe，绝不做大小写归一。 */
function expectStatusChar(actual: unknown, expected: string, label: string): void {
  expect(String(actual), `${label}应逐字符等于写入方词表值 ${expected}，实际 ${String(actual)}`).toBe(
    expected
  );
}

/** 白坯库存播种的唯一批次取值生成，配合回读定位本用例专属库存行。 */
function greigeBatchNo(tag: string): string {
  return `E2E-SSHP-B${tag}`;
}

/** 按款号加仓库加批次回读白坯库存行，批次为每轮唯一值，命中行必属本用例。 */
async function rereadGreigeRow(
  page: import('@playwright/test').Page,
  productId: number,
  warehouseId: number,
  batchNo: string
): Promise<Record<string, unknown>> {
  const path =
    `/inventory/stock?product_id=${productId}` +
    `&warehouse_id=${warehouseId}` +
    `&batch_no=${encodeURIComponent(batchNo)}` +
    `&page=1&page_size=50`;
  const res = await apiCallRaw<Record<string, unknown>>(page, 'GET', path);
  const rows = pickListArray<Record<string, unknown>>(res, 'items', '白坯库存行回读');
  const row = rows.find(r => !r.color_no || String(r.color_no) === '');
  if (!row) {
    throw new Error(
      `白坯库存行回读未命中，product=${productId} warehouse=${warehouseId} batch=${batchNo}，` +
        `候选行数=${rows.length}，候选色号=${rows.map(r => String(r.color_no)).join(',')}`
    );
  }
  return row;
}

/** 按仓库 ID 从仓库列表反查真实编码，取不到即抛错，绝不回落默认编码。 */
async function resolveWarehouseCode(
  page: import('@playwright/test').Page,
  warehouseId: number
): Promise<string> {
  const res = await apiCallRaw<Record<string, unknown>>(page, 'GET', '/warehouses?page=1&page_size=200');
  const rows = pickListArray<Record<string, unknown>>(res, 'items', '仓库列表反查发货编码');
  const hit = rows.find(w => Number(w.id) === warehouseId);
  if (!hit) {
    throw new Error(`仓库列表里没有 ID=${warehouseId} 的仓库行，无法确定发货仓库编码`);
  }
  const code = String(hit.warehouse_code ?? hit.code ?? '');
  if (code.trim() === '') {
    throw new Error(`仓库 ${warehouseId} 的编码键 warehouse_code 读回为空，无法发货`);
  }
  return code;
}

/** 读回销售订单详情，出参为 SalesOrderDetail，主表列与 items 明细同层。 */
async function readOrderDetail(
  page: import('@playwright/test').Page,
  orderId: number
): Promise<Record<string, unknown>> {
  return apiCallRaw<Record<string, unknown>>(page, 'GET', `/sales/orders/${orderId}`);
}

/** 读回订单的发货单列表，本端点出参是手搓信封，数组键为 list。 */
async function readDeliveries(
  page: import('@playwright/test').Page,
  orderId: number
): Promise<Record<string, unknown>[]> {
  const res = await apiCallRaw<Record<string, unknown>>(
    page,
    'GET',
    `/sales/orders/${orderId}/deliveries`
  );
  const rows = pickListArray<Record<string, unknown>>(res, 'list', '订单发货单列表');
  const total = readNumber(res, 'total', '订单发货单列表总数');
  expect(total, `发货单列表 total 应等于数组行数，实际 total=${total} 行数=${rows.length}`).toBe(
    rows.length
  );
  return rows;
}

/**
 * 按订单号在应收单列表里定位本用例的发票，覆盖必须完整。
 *
 * 列表端点只支持按客户与状态过滤，不支持按来源单号过滤，因此先按状态 DRAFT 与
 * 本客户读回总数，再逐页扫到累计行数覆盖总数为止，覆盖不满即抛错而不是默认"没有就是对"。
 * 命中多于一张同样抛错，同源链路上一个订单只允许一张应收单。
 */
async function findDraftInvoiceByOrderNo(
  page: import('@playwright/test').Page,
  customerId: number,
  orderNo: string,
  label: string
): Promise<Record<string, unknown> | null> {
  const pageSize = 100;
  let collected = 0;
  let total = -1;
  const hits: Record<string, unknown>[] = [];
  for (let p = 1; p <= 20; p++) {
    const res = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/invoices?customer_id=${customerId}&status=DRAFT&page=${p}&page_size=${pageSize}`
    );
    total = readNumber(res, 'total', `${label}应收草稿总数`);
    const rows = pickListArray<Record<string, unknown>>(res, 'items', `${label}应收草稿列表`);
    collected += rows.length;
    for (const r of rows) {
      if (String(r.source_type) === 'SALES_ORDER' && String(r.source_bill_no) === orderNo) {
        hits.push(r);
      }
    }
    if (collected >= total) {
      break;
    }
  }
  if (collected < total) {
    throw new Error(`${label} 应收草稿扫描未覆盖全量，total=${total} 已扫=${collected}，拒绝给出缺判结论`);
  }
  if (hits.length > 1) {
    throw new Error(
      `${label} 订单号 ${orderNo} 命中 ${hits.length} 张应收单，同源链路上同一订单至多一张`
    );
  }
  return hits.length === 1 ? hits[0]! : null;
}

interface ShipChain {
  orderId: number;
  orderNo: string;
  productId: number;
  customerId: number;
  warehouseId: number;
  batchNo: string;
  shipQty: number;
  availableBefore: number;
  shippedBefore: number;
  onHandBefore: number;
}

/**
 * 建单到发货的公共前段，逐段读回并钉住状态词表。
 *
 * 白坯行批次唯一，出库按批次精确命中。订单行数量即发货数量由调用方指定，
 * 指定等于订单量则全额发货并触发生成应收，小于订单量则落部分发货态。
 * 提交前状态应为草稿，提交后待审，审批后已审，四步全部用读回值逐字符比对，
 * 库存基线快照取在发货调用前一刻，前后差值只归本次发货解释。
 */
async function runShipChain(
  page: import('@playwright/test').Page,
  opts: {
    productId: number;
    customerId: number;
    warehouseId: number;
    orderQty: number;
    shipQty: number;
    unitPrice: number;
  }
): Promise<ShipChain> {
  const tag = genCode('SSHP');
  const batchNo = greigeBatchNo(tag);
  await seedGreigeStockIn(page, {
    productId: opts.productId,
    warehouseId: opts.warehouseId,
    batchNo,
    quantityMeters: '200',
  });

  const warehouseCode = await resolveWarehouseCode(page, opts.warehouseId);

  const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/sales/orders', {
    customer_id: opts.customerId,
    items: [
      {
        product_id: opts.productId,
        quantity: opts.orderQty,
        unit_price: opts.unitPrice,
      },
    ],
    remarks: `E2E 发货深度链路 ${tag}`,
  });
  const orderId = readNumber(created, 'id', '销售建单');
  const orderNo = String(created.order_no);
  expect(orderNo, `建单读回的订单号不应为空，实际 ${orderNo}`).not.toBe('undefined');
  expectStatusChar(created.status, 'draft', '建单后订单状态');

  await apiCallRaw(page, 'POST', `/sales/orders/${orderId}/submit`);
  const submitted = await readOrderDetail(page, orderId);
  expectStatusChar(submitted.status, 'pending', '提交后订单状态');

  await apiCallRaw(page, 'POST', `/sales/orders/${orderId}/approve`);
  const approved = await readOrderDetail(page, orderId);
  expectStatusChar(approved.status, 'approved', '审批后订单状态');

  const stockBefore = await rereadGreigeRow(page, opts.productId, opts.warehouseId, batchNo);
  const availableBefore = readNumber(stockBefore, 'quantity_available', '发货前可用量');
  const shippedBefore = readNumber(stockBefore, 'quantity_shipped', '发货前已发量');
  const onHandBefore = readNumber(stockBefore, 'quantity_on_hand', '发货前在库量');
  expect(
    availableBefore >= opts.shipQty,
    `发货前本用例专属库存行可用量 ${availableBefore} 应不小于拟发货量 ${opts.shipQty}，seed 即失败`
  ).toBe(true);

  await apiCallRaw(page, 'POST', `/sales/orders/${orderId}/ship`, {
    order_id: orderId,
    warehouse_code: warehouseCode,
    items: [
      {
        product_id: opts.productId,
        quantity: opts.shipQty,
        batch_no: batchNo,
      },
    ],
    remarks: `E2E 白坯发货 ${tag}`,
  });

  return {
    orderId,
    orderNo,
    productId: opts.productId,
    customerId: opts.customerId,
    warehouseId: opts.warehouseId,
    batchNo,
    shipQty: opts.shipQty,
    availableBefore,
    shippedBefore,
    onHandBefore,
  };
}

/** 取订单详情里本用例产品的那一行明细，定位不到即抛错而不是拿首行蒙过。 */
function pickOrderItem(
  detail: Record<string, unknown>,
  productId: number,
  label: string
): Record<string, unknown> {
  const items = pickListArray<Record<string, unknown>>(detail, 'items', label);
  const row = items.find(it => Number(it.product_id) === productId);
  if (!row) {
    throw new Error(
      `${label} 明细里没有产品 ${productId} 的行，实际行产品=${items
        .map(it => String(it.product_id))
        .join(',')}`
    );
  }
  return row;
}

/** 取核销明细里指定 item_type 的那一行，缺席即抛错。 */
function pickVerificationItem(
  items: Record<string, unknown>[],
  itemType: string,
  label: string
): Record<string, unknown> {
  const row = items.find(it => String(it.item_type) === itemType);
  if (!row) {
    throw new Error(
      `${label} 核销明细里没有 ${itemType} 行，实际 item_type=${items
        .map(it => String(it.item_type))
        .join(',')}`
    );
  }
  return row;
}

test.describe('销售发货到收款深度链路', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('全额发货，发货单数量、库存四维扣减与订单行回写三处读回一致，状态逐字符落词表，已发货订单再发货被拒且零副作用', async ({
    page,
  }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    const customerId = ctx.customerId;
    expect(productId, '前置产品缺失').toBeTruthy();
    expect(warehouseId, '前置仓库缺失').toBeTruthy();
    expect(customerId, '前置客户缺失').toBeTruthy();

    const chain = await runShipChain(page, {
      productId: productId!,
      customerId: customerId!,
      warehouseId: warehouseId!,
      orderQty: 40,
      shipQty: 40,
      unitPrice: 25.5,
    });

    // 订单侧，全额发货终态与小写词表逐字符一致，行级发货量精确回写
    const detail = await readOrderDetail(page, chain.orderId);
    expectStatusChar(detail.status, 'shipped', '全额发货后订单状态');
    const line = pickOrderItem(detail, chain.productId, '全额发货后订单详情');
    expectQuantity(
      readNumber(line, 'quantity', '订单行数量'),
      40,
      '订单行 quantity 应保持建单值'
    );
    expectQuantity(
      readNumber(line, 'shipped_quantity', '订单行已发量'),
      40,
      '全额发货后订单行 shipped_quantity 应精确等于发货量'
    );

    // 发货单侧，动作发货只生成一张单，状态落小写 shipped，数量维等于明细求和
    const deliveries = await readDeliveries(page, chain.orderId);
    expect(deliveries.length, `全额发货应只生成一张发货单，实际 ${deliveries.length} 张`).toBe(1);
    const delivery = deliveries[0]!;
    expectStatusChar(delivery.status, 'shipped', '动作发货生成的发货单状态');
    expectQuantity(
      readNumber(delivery, 'total_quantity', '发货单总数量'),
      40,
      '发货单头 total_quantity 应等于本次发货明细数量求和'
    );
    expect(
      typeof delivery.delivery_no === 'string' && (delivery.delivery_no as string).trim() !== '',
      `发货单号应为写入方生成的非空串，实际 ${String(delivery.delivery_no)}`
    ).toBe(true);
    expect(
      readNumber(delivery, 'order_id', '发货单归属订单'),
      `发货单 order_id 应指向本订单 ${chain.orderId}`
    ).toBe(chain.orderId);
    expect(
      readNumber(delivery, 'customer_id', '发货单客户'),
      `发货单 customer_id 应等于订单客户 ${chain.customerId}`
    ).toBe(chain.customerId);
    expect(
      readNumber(delivery, 'warehouse_id', '发货单仓库'),
      `发货单 warehouse_id 应等于按真实编码命中的仓库 ${chain.warehouseId}`
    ).toBe(chain.warehouseId);

    // 库存侧，可用量精确减、已发量精确增、在库量不动，三点同时成立才算账实一致
    const stockAfter = await rereadGreigeRow(page, chain.productId, chain.warehouseId, chain.batchNo);
    const availableAfter = readNumber(stockAfter, 'quantity_available', '发货后可用量');
    const shippedAfter = readNumber(stockAfter, 'quantity_shipped', '发货后已发量');
    const onHandAfter = readNumber(stockAfter, 'quantity_on_hand', '发货后在库量');
    expectQuantity(
      chain.availableBefore - availableAfter,
      40,
      '发货后可用量减少量应恰为发货量 40'
    );
    expectQuantity(
      shippedAfter - chain.shippedBefore,
      40,
      '发货后已发量增加量应恰为发货量 40'
    );
    expectQuantity(
      onHandAfter,
      chain.onHandBefore,
      '发货只把可用转已发，在库量 quantity_on_hand 不得动'
    );

    // 已发货订单再次发货，状态门拒绝，拒绝必须零副作用
    const reject = await apiCallExpectFail(page, 'POST', `/sales/orders/${chain.orderId}/ship`, {
      order_id: chain.orderId,
      warehouse_code: await resolveWarehouseCode(page, chain.warehouseId),
      items: [{ product_id: chain.productId, quantity: 1, batch_no: chain.batchNo }],
    });
    expect(
      reject.status,
      `已发货订单再发货应恰为 400 状态门拒绝，实际 status=${reject.status} code=${failureCode(reject)}`
    ).toBe(400);
    expect(
      failureCode(reject),
      `再发货拒绝属业务前置不满足族，实际 code=${failureCode(reject)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      typeof reject.message === 'string' && reject.message.trim().length > 0,
      `状态门拒绝应带非空可读文案，实际 message=${String(reject.message)}`
    ).toBe(true);

    const deliveriesAfterReject = await readDeliveries(page, chain.orderId);
    expect(
      deliveriesAfterReject.length,
      `被拒的再发货不得新增发货单，实际发货单数 ${deliveriesAfterReject.length}`
    ).toBe(1);
    const stockAfterReject = await rereadGreigeRow(
      page,
      chain.productId,
      chain.warehouseId,
      chain.batchNo
    );
    expectQuantity(
      readNumber(stockAfterReject, 'quantity_available', '被拒再发货后可用量'),
      availableAfter,
      '被拒的再发货不得再扣可用量'
    );
    expectQuantity(
      readNumber(stockAfterReject, 'quantity_shipped', '被拒再发货后已发量'),
      shippedAfter,
      '被拒的再发货不得再加已发量'
    );
    const detailAfterReject = await readOrderDetail(page, chain.orderId);
    expectStatusChar(detailAfterReject.status, 'shipped', '被拒再发货后订单状态');
  });

  test('部分发货，订单落 partial_shipped，动作发货单逐字符落 shipped，手动建单口逐字符落 pending，两个写入方互不混用且不发应收', async ({
    page,
  }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    const customerId = ctx.customerId;
    expect(productId, '前置产品缺失').toBeTruthy();
    expect(warehouseId, '前置仓库缺失').toBeTruthy();
    expect(customerId, '前置客户缺失').toBeTruthy();

    const chain = await runShipChain(page, {
      productId: productId!,
      customerId: customerId!,
      warehouseId: warehouseId!,
      orderQty: 40,
      shipQty: 25,
      unitPrice: 25.5,
    });

    const detail = await readOrderDetail(page, chain.orderId);
    expectStatusChar(detail.status, 'partial_shipped', '部分发货后订单状态');
    const line = pickOrderItem(detail, productId!, '部分发货后订单详情');
    expectQuantity(
      readNumber(line, 'shipped_quantity', '部分发货后订单行已发量'),
      25,
      '订单行 shipped_quantity 应精确等于已发数量，未发满不得凑整'
    );

    const deliveries = await readDeliveries(page, chain.orderId);
    expect(deliveries.length, `动作发货应只有一张发货单，实际 ${deliveries.length}`).toBe(1);
    expectStatusChar(deliveries[0]!.status, 'shipped', '动作发货单状态');
    expectQuantity(
      readNumber(deliveries[0]!, 'total_quantity', '动作发货单数量'),
      25,
      '动作发货单 total_quantity 应等于本次发货量 25'
    );

    // 手动建单口是同一词表的另一个写入方，落 pending 而不是动作发货的 shipped
    const manual = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/sales/orders/${chain.orderId}/deliveries`,
      { warehouse_id: warehouseId }
    );
    expectStatusChar(manual.status, 'pending', '手动创建的发货单状态');
    expectQuantity(
      readNumber(manual, 'total_quantity', '手动发货单数量'),
      0,
      '手动建单只建空壳单，数量应落初值零'
    );
    const manualId = readNumber(manual, 'id', '手动发货单');

    const allDeliveries = await readDeliveries(page, chain.orderId);
    expect(allDeliveries.length, `两张写入方各落一张，发货单总数应为 2`).toBe(2);
    const statuses = allDeliveries
      .map(d => String(d.status))
      .sort()
      .join(',');
    expect(
      statuses,
      '两张发货单的状态集合应恰好是 shipped 与 pending 两个小写词表值'
    ).toBe('pending,shipped');
    const manualRow = allDeliveries.find(d => Number(d.id) === manualId);
    if (!manualRow) {
      throw new Error(`手动发货单 ${manualId} 建后在列表里读不回，实际 id=${allDeliveries
        .map(d => String(d.id))
        .join(',')}`);
    }

    // 库存只按真实出库的 25 变化，手动空壳单不得动库存
    const stockAfter = await rereadGreigeRow(page, productId!, warehouseId!, chain.batchNo);
    expectQuantity(
      chain.availableBefore - readNumber(stockAfter, 'quantity_available', '手动建单后可用量'),
      25,
      '手动空壳发货单不得再扣库存，可用量减幅应仍为 25'
    );
    expectQuantity(
      readNumber(stockAfter, 'quantity_on_hand', '手动建单后在库量'),
      chain.onHandBefore,
      '手动空壳发货单不得动在库量'
    );

    // 部分发货不生成应收，写入方门控是全额发货才触发生成，缺判必须全量覆盖
    const invoice = await findDraftInvoiceByOrderNo(
      page,
      customerId!,
      chain.orderNo,
      '部分发货应收圈定'
    );
    expect(
      invoice,
      `订单 ${chain.orderNo} 处于部分发货态，扫描全量草稿应收后不应存在同源发票`
    ).toBeNull();
  });

  test('全额发货后应收单据与订单金额同源，来源键、草稿态与大写审批态逐字符落词表', async ({
    page,
  }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    const customerId = ctx.customerId;
    expect(productId, '前置产品缺失').toBeTruthy();
    expect(warehouseId, '前置仓库缺失').toBeTruthy();
    expect(customerId, '前置客户缺失').toBeTruthy();

    const chain = await runShipChain(page, {
      productId: productId!,
      customerId: customerId!,
      warehouseId: warehouseId!,
      orderQty: 60,
      shipQty: 60,
      unitPrice: 18,
    });
    const orderDetail = await readOrderDetail(page, chain.orderId);
    const orderTotal = readNumber(orderDetail, 'total_amount', '订单头总额读回');

    const found = await findDraftInvoiceByOrderNo(page, customerId!, chain.orderNo, '全额发货应收圈定');
    if (!found) {
      throw new Error(
        `订单 ${chain.orderNo} 全额发货后应收草稿扫描零命中，生成链断裂，见后端日志`
      );
    }
    const invoiceId = readNumber(found, 'id', '应收单列表命中行');

    // 详情读回逐字段断言，列表行与详情行必须同源同值，防止两个出参面各写各的
    const invoice = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/invoices/${invoiceId}`);
    expect(readNumber(invoice, 'id', '应收单详情'), '详情与列表应命中同一张发票').toBe(invoiceId);
    expectQuantity(
      readNumber(invoice, 'invoice_amount', '应收单金额'),
      orderTotal,
      '同源断言，发票 invoice_amount 必须逐分等于订单头 total_amount，而非算式重算值'
    );
    expectQuantity(
      readNumber(invoice, 'invoice_amount', '应收单金额'),
      readNumber(found, 'invoice_amount', '应收单列表行金额'),
      '列表行与详情行的金额必须一致'
    );
    expectQuantity(
      readNumber(invoice, 'received_amount', '应收单已收'),
      0,
      '未收款时 received_amount 应为零'
    );
    expectQuantity(
      readNumber(invoice, 'unpaid_amount', '应收单未收'),
      orderTotal,
      '未收款时 unpaid_amount 应等于全额'
    );
    expectStatusChar(invoice.status, 'DRAFT', '应收单初始状态');
    expectStatusChar(invoice.approval_status, 'PENDING', '应收单初始审批状态');
    expect(String(invoice.source_type), `来源类型应逐字符等于 SALES_ORDER`).toBe('SALES_ORDER');
    expect(
      String(invoice.source_bill_no),
      `来源单号应逐字符等于销售订单号 ${chain.orderNo}`
    ).toBe(chain.orderNo);
    expect(
      String(invoice.sales_order_no),
      `销售订单号冗余键应逐字符等于 ${chain.orderNo}`
    ).toBe(chain.orderNo);
    expect(
      readNumber(invoice, 'customer_id', '应收单客户'),
      `应收单客户应等于订单客户 ${chain.customerId}`
    ).toBe(chain.customerId);
    expect(
      String(invoice.invoice_no).trim() !== '' && String(invoice.invoice_no) !== 'undefined',
      `发票号应为写入方生成的非空串，实际 ${String(invoice.invoice_no)}`
    ).toBe(true);
  });

  test('收款直连核销，核销单状态与两行明细金额读回一致，发票结清与收款待确认态逐字符落词表', async ({
    page,
  }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    const customerId = ctx.customerId;
    expect(productId, '前置产品缺失').toBeTruthy();
    expect(warehouseId, '前置仓库缺失').toBeTruthy();
    expect(customerId, '前置客户缺失').toBeTruthy();

    const chain = await runShipChain(page, {
      productId: productId!,
      customerId: customerId!,
      warehouseId: warehouseId!,
      orderQty: 50,
      shipQty: 50,
      unitPrice: 12.75,
    });
    const found = await findDraftInvoiceByOrderNo(page, customerId!, chain.orderNo, '收款前应收圈定');
    if (!found) {
      throw new Error(`订单 ${chain.orderNo} 全额发货后应收草稿零命中，收款环节无从挂账`);
    }
    const invoiceId = readNumber(found, 'id', '收款前应收列表命中行');
    const invoiceBefore = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/invoices/${invoiceId}`
    );
    const paid = readNumber(invoiceBefore, 'invoice_amount', '收款前应收全额');

    const paymentData = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/payments', {
      customer_id: customerId,
      amount: paid,
      payment_method: 'bank_transfer',
      payment_date: new Date().toISOString().slice(0, 10),
      invoice_ids: [invoiceId],
      remark: `E2E 直连核销收款 ${chain.orderNo}`,
    });
    const collectionId = readNumber(paymentData, 'id', '收款创建回读');
    expectStatusChar(paymentData.status, 'pending', '收款创建后状态');
    expect(
      String(paymentData.payment_no).trim() !== '' && String(paymentData.payment_no) !== 'undefined',
      `收款单号应为写入方生成的非空串，实际 ${String(paymentData.payment_no)}`
    ).toBe(true);
    expectQuantity(
      readNumber(paymentData, 'amount', '收款金额回读'),
      paid,
      '收款金额应等于提交的全额'
    );

    // 发票侧结清，金额与状态两个维度同时读回
    const invoiceAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/invoices/${invoiceId}`
    );
    expectQuantity(
      readNumber(invoiceAfter, 'received_amount', '收款后已收'),
      paid,
      '直连核销后 received_amount 应精确等于分配额'
    );
    expectQuantity(
      readNumber(invoiceAfter, 'unpaid_amount', '收款后未收'),
      0,
      '全额收款核销后 unpaid_amount 应归零'
    );
    expectStatusChar(invoiceAfter.status, 'PAID', '结清后发票状态');

    // 核销单侧，按发票反查应恰好一张，主记录两个金额键都等于分配额
    const listByInvoice = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/verifications?invoice_id=${invoiceId}&page=1&page_size=10`
    );
    const recRows = pickListArray<Record<string, unknown>>(listByInvoice, 'list', '按发票反查核销列表');
    expect(
      readNumber(listByInvoice, 'total', '按发票反查核销总数'),
      `本发票应恰好一张核销单，实际 ${recRows.length} 行`
    ).toBe(1);
    expect(recRows.length, '核销列表按发票反查应命中一行').toBe(1);
    const recRow = recRows[0]!;
    const recId = readNumber(recRow, 'id', '核销单列表行');
    expectStatusChar(recRow.reconciliation_status, 'closed', '核销单状态');
    expectStatusChar(recRow.status, 'closed', '核销单状态别名键');
    expectQuantity(
      readNumber(recRow, 'total_invoices', '核销单发票侧合计'),
      paid,
      'total_invoices 应等于分配额'
    );
    expectQuantity(
      readNumber(recRow, 'total_collections', '核销单收款侧合计'),
      paid,
      'total_collections 应等于分配额'
    );
    expect(
      readNumber(recRow, 'customer_id', '核销单客户'),
      `核销单客户应等于 ${chain.customerId}`
    ).toBe(chain.customerId);

    // 核销明细两行，正负镜像，matched_amount 同为分配额，大写词表逐字符比对
    const recDetail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/verifications/${recId}`
    );
    const items = pickListArray<Record<string, unknown>>(recDetail, 'items', '核销单明细');
    expect(items.length, `核销明细应恰好两行，实际 ${items.length}`).toBe(2);
    const invoiceItem = pickVerificationItem(items, 'INVOICE', '核销单详情');
    const receiptItem = pickVerificationItem(items, 'RECEIPT', '核销单详情');

    expect(
      readNumber(invoiceItem, 'document_id', '核销发票明细指向'),
      `INVOICE 行 document_id 应指向发票 ${invoiceId}`
    ).toBe(invoiceId);
    expect(String(invoiceItem.document_type), 'INVOICE 行单据类型应逐字符等于 SALES_INVOICE').toBe(
      'SALES_INVOICE'
    );
    expectQuantity(
      readNumber(invoiceItem, 'amount', '核销发票明细金额'),
      paid,
      'INVOICE 行金额应为正分配额'
    );
    expectQuantity(
      readNumber(invoiceItem, 'matched_amount', '核销发票明细匹配额'),
      paid,
      'INVOICE 行 matched_amount 应等于分配额'
    );
    expectStatusChar(invoiceItem.match_status, 'MATCHED', '核销发票明细匹配态');

    expect(
      readNumber(receiptItem, 'document_id', '核销收款明细指向'),
      `RECEIPT 行 document_id 应指向收款单 ${collectionId}`
    ).toBe(collectionId);
    expect(String(receiptItem.document_type), 'RECEIPT 行单据类型应逐字符等于 AR_COLLECTION').toBe(
      'AR_COLLECTION'
    );
    expectQuantity(
      readNumber(receiptItem, 'amount', '核销收款明细金额'),
      -paid,
      'RECEIPT 行金额应为负分配额，与发票行镜像'
    );
    expectQuantity(
      readNumber(receiptItem, 'matched_amount', '核销收款明细匹配额'),
      paid,
      'RECEIPT 行 matched_amount 应为正分配额'
    );
    expectStatusChar(receiptItem.match_status, 'MATCHED', '核销收款明细匹配态');

    // 反向按收款单过滤，应命中同一张核销单，双向圈定同源
    const listByPayment = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/verifications?payment_id=${collectionId}&page=1&page_size=10`
    );
    const recRowsByPay = pickListArray<Record<string, unknown>>(
      listByPayment,
      'list',
      '按收款反查核销列表'
    );
    expect(
      readNumber(listByPayment, 'total', '按收款反查核销总数'),
      '按收款单反查也应恰好一张核销单'
    ).toBe(1);
    expect(
      readNumber(recRowsByPay[0]!, 'id', '按收款反查命中核销单'),
      `双向反查应命中同一张核销单 ${recId}`
    ).toBe(recId);

    // 收款确认动作不属于本链路，收款状态必须仍是小写 pending，不被核销连带改写
    const paymentReadBack = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/payments/${collectionId}`
    );
    expectStatusChar(paymentReadBack.status, 'pending', '核销完成后的收款单状态');
  });
});
