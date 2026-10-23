// 本文件守的缺陷族：出库数量不回写订单行、出库头与出库行账实脱节、
// 交货容差门失真（超发被放行 / 允收区间内被误拒）、库存流水前后差与出库量对不上。
// 断言全部为数值级：数量与金额按 Decimal 字符串出参折算整数最小单位比较，
// 不经 JS number 相加，杜绝浮点掩盖；每笔写动作后都以独立 GET 回读逐字段比对（刷新语义）。
// 状态比较值与写入方逐字符同源：sales_order 小写 draft/pending/approved/partial_shipped/shipped，
// sales_delivery 小写 pending/shipped/cancelled，库存匹大写 AVAILABLE/SHIPPED。
// 契约形态：订单列表/详情出参键 snake_case；订单发货记录出参为 {list,total}（非 items）；
// 库存流水出参为 PaginatedResponse{items,total,page,page_size}；发货入参用 warehouse_code（非 id）。
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
  APP_ERROR_CODES,
  type DyedSeedPiece,
} from './helpers';
import { pickListArray } from './ui-helpers';

type Row = Record<string, unknown>;

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanupRow(page, c.path, c.label);
  CLEANUP.length = 0;
});

async function tryCleanupRow(page: import('@playwright/test').Page, path: string, label: string) {
  try {
    await apiCall(page, 'DELETE', path);
  } catch (e) {
    console.warn(`[90-cleanup] ${label} 清理失败: ${(e as Error).message}`);
  }
}

/**
 * Decimal 出参（字符串或数字）折算为指定小数位的整数刻度值；
 * 超出该位数的尾数若非零立即抛错（断分位场景下标度异常必须判红，四舍五入即掩盖）。
 */
function toScaled(v: unknown, scale: number, label: string): bigint {
  const s = typeof v === 'string' ? v.trim() : typeof v === 'number' ? String(v) : '';
  if (s === '') {
    throw new Error(`${label}：Decimal 出参非 string|number，raw=${JSON.stringify(v)}`);
  }
  const m = /^(-?)(\d+)(?:\.(\d+))?$/.exec(s);
  if (!m) throw new Error(`${label}：Decimal 出参形态非法，raw="${s}"`);
  const [, sign, intPart, frac = ''] = m;
  if (frac.length > scale && frac.slice(scale).replace(/0+$/, '') !== '') {
    throw new Error(
      `${label}：出参标度 ${frac.length} 位超出断言精度 ${scale} 位且非零，raw="${s}"`
    );
  }
  // 按字符串实际小数位定标：小数位多于目标标度时整除降位（降位尾数已由上面的非零检查拦下），
  // 少于目标标度时补零升位；全程 BigInt 十进制定点，不经过 JS number/parseFloat。
  const digits = BigInt(`${intPart}${frac}`);
  const shifted =
    scale >= frac.length
      ? digits * 10n ** BigInt(scale - frac.length)
      : digits / 10n ** BigInt(frac.length - scale);
  return sign === '-' ? -shifted : shifted;
}

/** 金额断言（2 位小数=分位） */
function expectCents(actual: unknown, expectedWhole: string, label: string): void {
  expect(
    toScaled(actual, 2, label),
    `${label}：期望=${expectedWhole}（分位精确） 实际=${JSON.stringify(actual)}`
  ).toBe(toScaled(expectedWhole, 2, `${label} 期望值`));
}

/** 数量断言（允许至多 4 位小数标度，按值比较） */
function expectQty(actual: unknown, expected: string, label: string): void {
  expect(
    toScaled(actual, 4, label),
    `${label}：期望=${expected} 实际=${JSON.stringify(actual)}`
  ).toBe(toScaled(expected, 4, `${label} 期望值`));
}

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0)
    throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

// 分位换算自证表：toScaled 必须按字符串实际小数位定标（值×10^scale），
// 覆盖标度 0 到 6、整数出参、2 位与 4 位小数出参、负数与纯小数、非法形态与截尾守卫。
test('90-00 工具自证：toScaled 按实际小数位定标（scale 0 到 6 逐值）', () => {
  const exactCases: Array<readonly [string | number, number, bigint]> = [
    ['120', 0, 120n],
    ['120', 1, 1200n],
    ['120', 2, 12000n],
    ['120', 3, 120000n],
    ['120', 4, 1200000n],
    ['120', 5, 12000000n],
    ['120', 6, 120000000n],
    ['220.00', 0, 220n],
    ['220.00', 1, 2200n],
    ['220.00', 2, 22000n],
    ['220.00', 3, 220000n],
    ['220.00', 4, 2200000n],
    ['220.00', 6, 220000000n],
    ['200.0000', 2, 20000n],
    ['200.0000', 4, 2000000n],
    ['120.0000', 2, 12000n],
    ['-1.5', 1, -15n],
    ['-1.5', 2, -150n],
    ['-1.5', 4, -15000n],
    ['-1.5', 6, -1500000n],
    ['0.000123', 6, 123n],
    [200, 2, 20000n],
    [200.5, 2, 20050n],
  ];
  for (const [input, scale, expected] of exactCases) {
    expect(
      toScaled(input, scale, `自证 ${JSON.stringify(input)}@${scale}`),
      `换算 ${JSON.stringify(input)} scale=${scale}：期望=${expected}`
    ).toBe(expected);
  }
  // 降位尾数非零必须抛错（静默截断=掩盖标度异常）
  const truncationCases: Array<readonly [string, number]> = [
    ['1.2345', 2],
    ['0.000123', 4],
    ['-1.5', 0],
  ];
  for (const [input, scale] of truncationCases) {
    expect(() => toScaled(input, scale, `自证截尾 ${input}@${scale}`)).toThrow(/非零/);
  }
  // 出参形态守卫：非十进制字符串与非 string|number 一律抛错
  expect(() => toScaled('1e3', 2, '自证形态')).toThrow(/形态非法/);
  expect(() => toScaled({ v: '1' }, 2, '自证形态')).toThrow(/非 string\|number/);
});

/** 按四维元组回读库存行（缺行即红，不做兜底空对象） */
async function readStockRow(
  page: import('@playwright/test').Page,
  dims: { productId: number; warehouseId: number; colorNo: string; dyeLotNo: string }
): Promise<Row> {
  const qs =
    `product_id=${dims.productId}&warehouse_id=${dims.warehouseId}` +
    `&color_no=${encodeURIComponent(dims.colorNo)}&dye_lot_no=${encodeURIComponent(dims.dyeLotNo)}` +
    `&batch_no=${encodeURIComponent(dims.dyeLotNo)}&page=1&page_size=20`;
  const res = await apiCallRaw<Row>(page, 'GET', `/inventory/stock?${qs}`);
  const rows = pickListArray<Row>(res, 'items', `产品 ${dims.productId} 四维库存行`);
  const row = rows.find(
    r =>
      r.color_no === dims.colorNo && r.dye_lot_no === dims.dyeLotNo && r.batch_no === dims.dyeLotNo
  );
  if (!row)
    throw new Error(`四维库存行缺失：dims=${JSON.stringify(dims)} 实际行=${JSON.stringify(rows)}`);
  return row;
}

/** 按订单号锚定该产品名下 SALES_DELIVERY 库存流水行（source_bill_no 必须逐字符等于订单号） */
async function readSalesDeliveryTxns(
  page: import('@playwright/test').Page,
  productId: number,
  dyeLotNo: string,
  orderNo: string
): Promise<Row[]> {
  const qs =
    `product_id=${productId}&batch_no=${encodeURIComponent(dyeLotNo)}` +
    `&transaction_type=SALES_DELIVERY&page=1&page_size=100`;
  const res = await apiCallRaw<Row>(page, 'GET', `/inventory/stock/transactions?${qs}`);
  const rows = pickListArray<Row>(res, 'items', `产品 ${productId} 出库库存流水`);
  return rows.filter(r => r.source_bill_no === orderNo);
}

async function seedApprovedOrder(
  page: import('@playwright/test').Page,
  items: Array<{
    product_id: number;
    quantity: string;
    unit_price: string;
    tax_percent: string;
    quantity_tolerance_pct: string;
  }>
): Promise<{ id: number; order_no: string }> {
  const ctx = getCtx();
  if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
  const created = await apiCallRaw<Row>(page, 'POST', '/sales/orders', {
    customer_id: ctx.customerId,
    order_date: new Date().toISOString(),
    items,
    notes: `E2E-SHIP-${genCode('S9')}`,
  });
  const id = requireNum(created.id, '建销售订单');
  CLEANUP.push({ path: `/sales/orders/${id}`, label: `so#${id}` });
  await apiCall(page, 'POST', `/sales/orders/${id}/submit`);
  await apiCall(page, 'POST', `/sales/orders/${id}/approve`);
  const detail = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${id}`);
  expect(detail.status, 'approve 回读应为 approved（词表小写）').toBe('approved');
  return { id, order_no: String(detail.order_no) };
}

function orderLine(detail: Row, productId: number): Row {
  const lines = detail.items as Row[];
  const hit = lines.find(l => Number(l.product_id) === productId);
  if (!hit) throw new Error(`订单明细缺产品 ${productId} 的行，实际=${JSON.stringify(lines)}`);
  return hit;
}

test.describe('90 销售订单→出库 逐行账实链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('90-01 两行订单分两笔部分发货：行级 shipped_quantity 逐行累加不串写、出库头归属与源单逐字段相等、库存流水前后差=出库量', async ({
    page,
  }) => {
    const ctx = getCtx();
    const pA = ctx.productIds[0];
    const pB = ctx.productIds[1];
    if (!pA || !pB) throw new Error('前置缺失：ctx.productIds 不足两个产品');
    if (pA === pB) throw new Error('前置失败：ctx.productIds[0]/[1] 是同一产品，无法验证逐行分配');

    // 行级容差显式写 0：上下界=行数量，杜绝产品单位默认容差引入的不确定区间
    const order = await seedApprovedOrder(page, [
      {
        product_id: pA,
        quantity: '100',
        unit_price: '37.50',
        tax_percent: '13',
        quantity_tolerance_pct: '0',
      },
      {
        product_id: pB,
        quantity: '50',
        unit_price: '210.00',
        tax_percent: '6',
        quantity_tolerance_pct: '0',
      },
    ]);

    const target = await pickDyeableWarehouse(page);
    const bundleA = await seedDyedOutboundBundle(page, {
      productId: pA,
      warehouseId: target.id,
      quantityMeters: '200',
      pieceCount: 2,
      context: 'E90-01A',
    });
    const bundleB = await seedDyedOutboundBundle(page, {
      productId: pB,
      warehouseId: target.id,
      quantityMeters: '200',
      pieceCount: 2,
      context: 'E90-01B',
    });
    if (bundleA.pieces.length < 2 || bundleB.pieces.length < 2)
      throw new Error('[90-01] 前置失败：真实链未产出足量可出库染色匹');

    const stockA0 = await readStockRow(page, {
      productId: pA,
      warehouseId: target.id,
      colorNo: bundleA.colorNo,
      dyeLotNo: bundleA.dyeLotNo,
    });
    const stockB0 = await readStockRow(page, {
      productId: pB,
      warehouseId: target.id,
      colorNo: bundleB.colorNo,
      dyeLotNo: bundleB.dyeLotNo,
    });
    expectCents(stockA0.quantity_available, '200', 'A 发货前可用量');
    expectCents(stockB0.quantity_available, '200', 'B 发货前可用量');

    const shipItems = (
      aQty: string,
      aPiece: DyedSeedPiece,
      bQty: string,
      bPiece: DyedSeedPiece
    ) => ({
      order_id: order.id,
      warehouse_code: target.code,
      items: [
        {
          product_id: pA,
          quantity: aQty,
          batch_no: bundleA.dyeLotNo,
          color_no: bundleA.colorNo,
          dye_lot_no: bundleA.dyeLotNo,
          piece_no: aPiece.piece_no,
        },
        {
          product_id: pB,
          quantity: bQty,
          batch_no: bundleB.dyeLotNo,
          color_no: bundleB.colorNo,
          dye_lot_no: bundleB.dyeLotNo,
          piece_no: bPiece.piece_no,
        },
      ],
    });

    // ── 第一笔：A 发 60、B 发 20 ──
    await apiCall(
      page,
      'POST',
      `/sales/orders/${order.id}/ship`,
      shipItems('60', bundleA.pieces[0], '20', bundleB.pieces[0])
    );

    const d1 = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${order.id}`);
    expect(d1.status, '部分发货订单状态词值（写入方小写 partial_shipped）').toBe('partial_shipped');
    // 行级回写不串写：发 A60/B20 若被整笔加到每一行（历史缺陷形态），这里读到的是 80/80 而非 60/20
    expectQty(orderLine(d1, pA).shipped_quantity, '60', '第一笔后 A 行 shipped_quantity');
    expectQty(orderLine(d1, pB).shipped_quantity, '20', '第一笔后 B 行 shipped_quantity');
    expectQty(orderLine(d1, pA).quantity, '100', 'A 行订单数量未被发货动作改写');
    expectQty(orderLine(d1, pB).quantity, '50', 'B 行订单数量未被发货动作改写');
    // 金额恒等（分位）：行 subtotal = 行数量×行单价，发货不回改订单行金额
    expectCents(orderLine(d1, pA).subtotal, '3750.00', 'A 行 subtotal=100×37.50');
    expectCents(orderLine(d1, pB).subtotal, '10500.00', 'B 行 subtotal=50×210');

    const stockA1 = await readStockRow(page, {
      productId: pA,
      warehouseId: target.id,
      colorNo: bundleA.colorNo,
      dyeLotNo: bundleA.dyeLotNo,
    });
    expectCents(stockA1.quantity_available, '140', 'A 库存行可用量=200-60');
    expectCents(stockA1.quantity_shipped, '60', 'A 库存行已发量+=60');
    expectQty(
      stockA1.quantity_on_hand,
      String(stockA0.quantity_on_hand),
      'A 库存行现存量守恒（出库只动可用/已发）'
    );
    const stockB1 = await readStockRow(page, {
      productId: pB,
      warehouseId: target.id,
      colorNo: bundleB.colorNo,
      dyeLotNo: bundleB.dyeLotNo,
    });
    expectCents(stockB1.quantity_available, '180', 'B 库存行可用量=200-20');
    expectCents(stockB1.quantity_shipped, '20', 'B 库存行已发量+=20');

    // 出库库存流水：本订单名下每产品恰 1 行，前后差必须逐分等于本次出库量（流水即出库行的可读账本）
    const txA1 = await readSalesDeliveryTxns(page, pA, bundleA.dyeLotNo, order.order_no);
    expect(txA1.length, '第一笔后 A 出库流水应恰 1 行（本订单名下）').toBe(1);
    expectQty(txA1[0].quantity_meters, '60', 'A 出库流水行出库量');
    expectQty(txA1[0].quantity_before_meters, '200', 'A 出库流水行扣前量');
    expect(toScaled(txA1[0].quantity_after_meters, 4, 'A 流水扣后量'), '流水前后差=出库量').toBe(
      toScaled(txA1[0].quantity_before_meters, 4, 'before') -
        toScaled(txA1[0].quantity_meters, 4, 'qty')
    );
    const txB1 = await readSalesDeliveryTxns(page, pB, bundleB.dyeLotNo, order.order_no);
    expect(txB1.length, '第一笔后 B 出库流水应恰 1 行').toBe(1);
    expectQty(txB1[0].quantity_meters, '20', 'B 出库流水行出库量');

    // 发货单回读：信封键 {list,total}（缺键即红，不得 ?? [] 吞掉）
    const dlv1 = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${order.id}/deliveries`);
    for (const k of ['list', 'total']) {
      expect(Object.prototype.hasOwnProperty.call(dlv1, k), `发货记录回读应含信封键 ${k}`).toBe(
        true
      );
    }
    const dlvRows1 = pickListArray<Row>(dlv1, 'list', '发货单列表');
    expect(dlvRows1.length, '第一笔发货后应恰 1 张发货单').toBe(1);
    const dlv1Row = dlvRows1[0];
    expect(dlv1Row.status, '发货单状态词值（写入方小写 shipped）').toBe('shipped');
    // 出库头归属来自源单：customer_id/warehouse_id 逐值相等（不是断非空）
    expect(dlv1Row.customer_id, '发货单 customer_id 必须等于订单客户').toBe(d1.customer_id);
    expect(Number(dlv1Row.warehouse_id), '发货单 warehouse_id 必须等于实际出库仓库').toBe(
      target.id
    );
    expect(Number(dlv1Row.order_id), '发货单 order_id 必须等于源订单').toBe(order.id);
    expectQty(dlv1Row.total_quantity, '80', '发货单头 total_quantity=60+20（请求量合计如实回写）');
    expect(String(dlv1Row.delivery_no), '发货单号应由后端生成且带 DN 前缀').toMatch(/^DN/);

    // 匹消耗回读（第四维）：两笔各消耗一匹，AVAILABLE→SHIPPED
    const pieceA = bundleA.pieces[0];
    const pieceARows = await apiCallRaw<Row>(
      page,
      'GET',
      `/inventory/pieces?product_id=${pA}&warehouse_id=${target.id}&dye_lot_no=${encodeURIComponent(bundleA.dyeLotNo)}&batch_no=${encodeURIComponent(bundleA.dyeLotNo)}&piece_no=${encodeURIComponent(pieceA.piece_no)}&page=1&page_size=10`
    );
    const pieceAHit = pickListArray<Row>(pieceARows, 'items', 'A 匹回读').find(
      r => r.piece_no === pieceA.piece_no
    );
    expect(pieceAHit, `应回读到匹 ${pieceA.piece_no}`).toBeTruthy();
    expect(String(pieceAHit!.status), '被出库的匹必须 AVAILABLE→SHIPPED').toBe('SHIPPED');

    // ── 第二笔：A 补发 40、B 补发 30 → 全额发货 ──
    await apiCall(
      page,
      'POST',
      `/sales/orders/${order.id}/ship`,
      shipItems('40', bundleA.pieces[1], '30', bundleB.pieces[1])
    );
    const d2 = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${order.id}`);
    expect(d2.status, '全部行发满后订单词值 shipped（不是 partial_shipped）').toBe('shipped');
    expectQty(orderLine(d2, pA).shipped_quantity, '100', '全额后 A 行 shipped_quantity=行数量');
    expectQty(orderLine(d2, pB).shipped_quantity, '50', '全额后 B 行 shipped_quantity=行数量');
    // 行数量与已发量的业务不变式：每行 shipped ≤ quantity（容差 0 时恰为等号于全额行）
    for (const pid of [pA, pB]) {
      const line = orderLine(d2, pid);
      expect(
        toScaled(line.shipped_quantity, 4, 'shipped') <= toScaled(line.quantity, 4, 'qty'),
        `行 ${pid} shipped_quantity(${line.shipped_quantity}) 不得超过 quantity(${line.quantity})`
      ).toBe(true);
    }
    const stockA2 = await readStockRow(page, {
      productId: pA,
      warehouseId: target.id,
      colorNo: bundleA.colorNo,
      dyeLotNo: bundleA.dyeLotNo,
    });
    expectCents(stockA2.quantity_available, '100', 'A 全额后可用量=200-100');
    expectCents(stockA2.quantity_shipped, '100', 'A 全额后已发量=100');
    const dlv2 = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${order.id}/deliveries`);
    const dlvRows2 = pickListArray<Row>(dlv2, 'list', '第二笔后发货单列表');
    expect(dlvRows2.length, '两笔发货应有两张发货单（分批留痕，不是覆盖）').toBe(2);
    const firstDlv = dlvRows2.find(r => Number(r.id) === Number(dlv1Row.id));
    expect(firstDlv, '第一张发货单必须仍在列表（分批不覆盖既有出库单）').toBeTruthy();
    expectQty(firstDlv!.total_quantity, '80', '第一张发货单总量不被第二笔污染');
    // 刷新语义：详情两次读一致（假成功未落库在这里表现为两次读漂移或状态回跳）
    const d2Again = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${order.id}`);
    expect(String(d2Again.status), '重复读回状态必须稳定').toBe(String(d2.status));
    expectQty(orderLine(d2Again, pA).shipped_quantity, '100', '重复读回行 shipped 量稳定');
  });

  test('90-02 容差门双向：0% 容差超 1 米即拒且零副作用，5% 容差恰好及界放行、越界拒', async ({
    page,
  }) => {
    const ctx = getCtx();
    const pA = ctx.productIds[0];
    const pB = ctx.productIds[2] ?? ctx.productIds[1];
    if (!pA || !pB || pA === pB) throw new Error('前置缺失：需要两个不同产品做双订单');

    const target = await pickDyeableWarehouse(page);

    // 订单一：行数量 100、容差 0 —— 上界恰为 100
    const order0 = await seedApprovedOrder(page, [
      {
        product_id: pA,
        quantity: '100',
        unit_price: '12.00',
        tax_percent: '0',
        quantity_tolerance_pct: '0',
      },
    ]);
    const bundle0 = await seedDyedOutboundBundle(page, {
      productId: pA,
      warehouseId: target.id,
      quantityMeters: '120',
      pieceCount: 1,
      context: 'E90-02A',
    });
    const ship0 = (qty: string, pieceNo: string) => ({
      order_id: order0.id,
      warehouse_code: target.code,
      items: [
        {
          product_id: pA,
          quantity: qty,
          batch_no: bundle0.dyeLotNo,
          color_no: bundle0.colorNo,
          dye_lot_no: bundle0.dyeLotNo,
          piece_no: pieceNo,
        },
      ],
    });
    const f0 = await apiCallExpectFail(
      page,
      'POST',
      `/sales/orders/${order0.id}/ship`,
      ship0('101', bundle0.pieces[0].piece_no)
    );
    expect(f0.status, '容差 0 超发 1 米应 400').toBe(400);
    expect(failureCode(f0), '超发机器码 BUSINESS_ERROR').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    // 被拒零副作用（fail-closed，不允许半扣库存/半回写）
    const d0 = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${order0.id}`);
    expect(d0.status, '超发被拒后订单不得漂出 approved').toBe('approved');
    expectQty(orderLine(d0, pA).shipped_quantity, '0', '超发被拒后行 shipped_quantity 不得被写入');
    const stock0 = await readStockRow(page, {
      productId: pA,
      warehouseId: target.id,
      colorNo: bundle0.colorNo,
      dyeLotNo: bundle0.dyeLotNo,
    });
    expectCents(stock0.quantity_available, '120', '超发被拒后可用量分毫未动');
    expectCents(stock0.quantity_shipped, '0', '超发被拒后已发量不得预埋');
    const dlv0 = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${order0.id}/deliveries`);
    expect(
      pickListArray<Row>(dlv0, 'list', '被拒后的发货单列表').length,
      '被拒发货不得留下发货单'
    ).toBe(0);

    // 订单二：行数量 100、容差 5 —— 上界 105；发 105 恰好及界放行
    const order5 = await seedApprovedOrder(page, [
      {
        product_id: pB,
        quantity: '100',
        unit_price: '12.00',
        tax_percent: '0',
        quantity_tolerance_pct: '5',
      },
    ]);
    const bundle5 = await seedDyedOutboundBundle(page, {
      productId: pB,
      warehouseId: target.id,
      quantityMeters: '120',
      pieceCount: 1,
      context: 'E90-02B',
    });
    const ship5 = (qty: string) => ({
      order_id: order5.id,
      warehouse_code: target.code,
      items: [
        {
          product_id: pB,
          quantity: qty,
          batch_no: bundle5.dyeLotNo,
          color_no: bundle5.colorNo,
          dye_lot_no: bundle5.dyeLotNo,
          piece_no: bundle5.pieces[0].piece_no,
        },
      ],
    });
    await apiCall(page, 'POST', `/sales/orders/${order5.id}/ship`, ship5('105'));
    const d5 = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${order5.id}`);
    expect(d5.status, '及界发货 105 应放行并推进 shipped（行已发满）').toBe('shipped');
    expectQty(
      orderLine(d5, pB).shipped_quantity,
      '105',
      '及界发货行回写实发 105（不是行数量 100）'
    );

    // 累计越界：105 + 1 > 105 → 拒，且此前实发 105 无痕保留
    const f5 = await apiCallExpectFail(page, 'POST', `/sales/orders/${order5.id}/ship`, ship5('1'));
    expect(f5.status, '累计超容差上界应 400').toBe(400);
    expect(failureCode(f5), '累计超发机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const d5b = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${order5.id}`);
    expect(d5b.status, '终态 shipped 不被被拒请求改动').toBe('shipped');
    expectQty(orderLine(d5b, pB).shipped_quantity, '105', '被拒续发不得累加行 shipped_quantity');
    const stock5 = await readStockRow(page, {
      productId: pB,
      warehouseId: target.id,
      colorNo: bundle5.colorNo,
      dyeLotNo: bundle5.dyeLotNo,
    });
    expectCents(stock5.quantity_available, '15', '被拒续发后可用量仍为 120-105');

    // 订单一（容差 0）此刻精确发满 100 → 放行，证明上一拒绝不是"永远发不出"的假门
    await apiCall(
      page,
      'POST',
      `/sales/orders/${order0.id}/ship`,
      ship0('100', bundle0.pieces[0].piece_no)
    );
    const d0b = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${order0.id}`);
    expect(d0b.status, '精确按行数量发货应放行至 shipped').toBe('shipped');
    expectQty(orderLine(d0b, pA).shipped_quantity, '100', '行精确发满回写 100');
  });
});
