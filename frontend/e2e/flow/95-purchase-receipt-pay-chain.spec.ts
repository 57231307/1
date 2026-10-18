// 采购链深度契约 E2E — 收货 → 质检 → 入库 → 应付 → 凭证 → 付款（数值级）
//
// 本文件守的是同一族缺陷：**跨环数量/金额不一致却无人报错、写成功但未落库、
// 状态门与词表不同源、部分与分批分支没人测**。与既有用例的分工：既有用例已覆盖
// 「UI 收货按钮可见 + 状态字面量 + 单据字段回读」，本文件只补它没断的那一半——
// 把每一环的数量与金额按十进制字符串逐值对账，并证明「被拒的写入真的没落库」。
//
// 断言纪律：
// - 所有 Decimal 出参一律按十进制字符串用 BigInt 定标比较，禁止 JS number 累加
//   （rust_decimal 未开 serde-arbitrary-precision，出参本就是字符串）。
// - 每个写动作后 GET 读回逐字段比对；负例同时断 HTTP 码、机器码与"零漂移"，
//   拒绝原因可外显的站点逐字符锁文案，被脱敏的站点只锁机器码并在失败信息里点名根因。
// - 数量/金额的基准全部由本用例自建自收的单据导出，不引用库中历史行、不写死绝对值。
// - 夹具唯一命名、finally 只删自建行。
import { test, expect } from '../diagnose-fixture';
import { type Page } from '@playwright/test';
import {
  loginViaUI,
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  APP_ERROR_CODES,
  genCode,
  tryCleanup,
  seedInspectionPass,
} from './helpers';
import { pickListArray } from './ui-helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

// ───────────────────────────── 十进制字符串对账（禁浮点）─────────────────────────────

/** 定标十进制值：v = 去掉小数点后的整数（含符号），s = 小数位数 */
interface Dec {
  v: bigint;
  s: number;
}

/**
 * 供日志/报错模板安全序列化：定标十进制值的整数部分是 BigInt，
 * 直接 JSON.stringify 会抛 TypeError: Do not know how to serialize a BigInt，
 * 把真实断言原因盖掉。这里统一把 BigInt 转成字符串再交给 JSON.stringify。
 */
function jsonText(raw: unknown): string {
  return JSON.stringify(raw, (_key, value: unknown) =>
    typeof value === 'bigint' ? value.toString() : value
  );
}

function decFromRaw(raw: unknown, label: string): Dec {
  const text = typeof raw === 'string' ? raw.trim() : typeof raw === 'number' ? String(raw) : null;
  if (text === null) {
    throw new Error(
      `${label}：后端 Decimal 出参必须是字符串（rust_decimal 默认序列化口径），` +
        `实际类型=${typeof raw} 值=${jsonText(raw)}`
    );
  }
  const m = /^(-?)(\d+)(?:\.(\d+))?$/.exec(text);
  if (!m) {
    throw new Error(`${label}：无法按十进制解析，raw=${jsonText(raw)}`);
  }
  const sign = m[1] === '-' ? -1n : 1n;
  return { v: sign * BigInt(`${m[2]}${m[3] ?? ''}`), s: (m[3] ?? '').length };
}

function decLiteral(text: string, label: string): Dec {
  return decFromRaw(text, label);
}

function alignDec(a: Dec, b: Dec): [bigint, bigint, number] {
  const s = Math.max(a.s, b.s);
  return [a.v * 10n ** BigInt(s - a.s), b.v * 10n ** BigInt(s - b.s), s];
}

function decAdd(a: Dec, b: Dec): Dec {
  const [x, y, s] = alignDec(a, b);
  return { v: x + y, s };
}

function decCmp(a: Dec, b: Dec): number {
  const [x, y] = alignDec(a, b);
  return x < y ? -1 : x > y ? 1 : 0;
}

function decText(d: Dec): string {
  const sign = d.v < 0n ? '-' : '';
  const digits = (d.v < 0n ? -d.v : d.v).toString().padStart(d.s + 1, '0');
  return d.s === 0 ? `${sign}${digits}` : `${sign}${digits.slice(0, -d.s)}.${digits.slice(-d.s)}`;
}

/** 十进制值逐值相等（定标对齐后比整数，无浮点误差）；actual 必须是后端原始出参 */
function expectDecEquals(actual: unknown, expected: Dec, label: string): void {
  const a = decFromRaw(actual, label);
  expectDecMatch(a, expected, label);
}

/** 两侧都已解析为定标十进制值时的逐值相等（派生量比对走这里，不得回喂原始出参解析口） */
function expectDecMatch(actual: Dec, expected: Dec, label: string): void {
  expect(
    decCmp(actual, expected) === 0,
    `${label}：十进制期望=${decText(expected)} 实际=${decText(actual)}`
  ).toBe(true);
}

/** 出参必须是十进制字符串（Decimal 序列化契约；退化成 JSON number 即判红） */
function expectDecimalString(raw: unknown, label: string): Dec {
  expect(
    typeof raw,
    `${label}：Decimal 出参应为 JSON 字符串，实际类型=${typeof raw} 值=${jsonText(raw)}`
  ).toBe('string');
  return decFromRaw(raw, label);
}

/** 按行累加某个十进制键（字符串 → BigInt 定标相加，全程不经过 JS number） */
function decSumOf(rows: Array<Record<string, unknown>>, key: string, label: string): Dec {
  let acc: Dec = { v: 0n, s: 0 };
  for (const [i, row] of rows.entries()) {
    if (!Object.prototype.hasOwnProperty.call(row, key)) {
      throw new Error(
        `${label} 第 ${i + 1} 行缺十进制键 "${key}"，实际键=${Object.keys(row).join(',')}`
      );
    }
    acc = decAdd(acc, decFromRaw(row[key], `${label} 第 ${i + 1} 行 ${key}`));
  }
  return acc;
}

function requireId(row: Record<string, unknown>, label: string): number {
  const id = Number(row?.id);
  if (!Number.isFinite(id) || id <= 0) {
    throw new Error(`${label}：未取到有效 id，实际键=${Object.keys(row ?? {}).join(',')}`);
  }
  return id;
}

/**
 * 读凭证详情（GET /vouchers/{id}）的分录数组。
 * 该端点出参是「凭证主表字段 flatten + entries 数组」，entries 不是分页信封的 items，
 * 因此不走通用列表择形工具，直接按真实键名取并判形：缺键或非数组即判红并列出实际键集。
 */
function pickEntries(detail: Record<string, unknown>, context: string): Array<Record<string, unknown>> {
  const rows = detail.entries;
  if (!Array.isArray(rows)) {
    throw new Error(
      `[${context}] 凭证详情出参缺 entries 数组，实际键=${Object.keys(detail).join(',')}`
    );
  }
  return rows as Array<Record<string, unknown>>;
}

/**
 * 按来源单据号定位凭证。GET /vouchers 是服务端分页（page_size 默认 20、上限 clamp 100）且出参
 * 为当页裸数组，同一凭证类型在全库维度可能跨多页；凭证的 source_bill_no 也不是该列表的可信
 * 过滤维度（不同来源类型的凭证号命名规则各异）。故逐页拉全（每页取后端允许的上界 100），
 * 在完整集合上按单据号定位，禁止只读第一页就判定凭证不存在后再整条判红。
 * 翻页设硬上限防越界：达到上限仍未取完即显式抛错交判责，不静默截断。
 */
async function findVoucherBySourceBillNo(
  page: Page,
  sourceBillNo: string,
  context: string
): Promise<Record<string, unknown> | undefined> {
  const pageSize = 100;
  const maxPages = 50;
  const seen = new Set<number>();
  const hits: Array<Record<string, unknown>> = [];
  let sawShortPage = false;
  for (let pageNo = 1; pageNo <= maxPages; pageNo += 1) {
    const rows = pickListArray<Record<string, unknown>>(
      await apiCallRaw<unknown>(page, 'GET', `/vouchers?page=${pageNo}&page_size=${pageSize}`),
      'bare',
      `${context} 凭证全量翻页第 ${pageNo} 页`
    );
    for (const r of rows) {
      const id = Number(r.id);
      // 同页内按 id 去重：翻页间隙若有新凭证落库可能造成窗口漂移，重复行不构成多命中
      if (Number.isFinite(id) && !seen.has(id)) {
        seen.add(id);
        if (String(r.source_bill_no ?? '') === sourceBillNo) hits.push(r);
      }
    }
    if (rows.length < pageSize) {
      sawShortPage = true;
      break;
    }
  }
  if (!sawShortPage) {
    throw new Error(
      `${context}：凭证列表翻页达上限 ${maxPages} 页（${maxPages * pageSize} 行）仍未到尾，判为取数异常`
    );
  }
  if (hits.length > 1) {
    throw new Error(
      `${context}：来源单据号 ${sourceBillNo} 命中 ${hits.length} 张凭证，跨环锚点不唯一，需判责`
    );
  }
  return hits[0];
}

// ───────────────────────────── 本链自建夹具 ─────────────────────────────

interface SeedLine {
  lineNo: number;
  productId: number;
  materialCode: string;
  materialName: string;
  unit: string;
  orderedQty: Dec;
  unitPrice: Dec;
  orderItemId: number;
}

/**
 * 建一张已审批采购订单并返回逐行契约值（订购量/单价/订单明细 id）。
 * 行级允差显式送 0：把"允许收货上界"钉成订购量本身，超收分支不随产品单位默认容差漂移。
 */
async function seedApprovedOrder(
  page: Page,
  lines: Array<{ product: { id: number; code: string; name: string; unit: string }; qty: string; price: string; taxRate: string }>
): Promise<{ poId: number; orderNo: string; seeded: SeedLine[] }> {
  const ctx = getCtx();
  if (!ctx.supplierId) throw new Error('前置缺失：ctx.supplierId 未就绪');
  if (!ctx.warehouseIds[0]) throw new Error('前置缺失：ctx.warehouseIds[0] 未就绪');
  if (!ctx.departmentIds[0]) throw new Error('前置缺失：ctx.departmentIds[0] 未就绪');
  const marker = genCode('F95');

  const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/orders', {
    supplier_id: ctx.supplierId,
    order_date: todayStr(),
    expected_delivery_date: new Date(Date.now() + 14 * 86400000).toISOString().slice(0, 10),
    warehouse_id: ctx.warehouseIds[0],
    department_id: ctx.departmentIds[0],
    currency: 'USD',
    exchange_rate: '7.15',
    notes: `E2E-F95-${marker}`,
    items: lines.map((l, i) => ({
      line_no: i + 1,
      material_id: l.product.id,
      quantity_ordered: l.qty,
      quantity_alt_ordered: '0',
      unit_price: l.price,
      tax_rate: l.taxRate,
      quantity_tolerance_pct: '0',
      notes: `E2E-F95-L${i + 1}-${marker}`,
    })),
  });
  const poId = requireId(created, `建 PO（${marker}）`);
  const orderNo = String(created.order_no ?? '');
  if (!orderNo) throw new Error(`建 PO 未返回 order_no，实际键=${Object.keys(created).join(',')}`);
  CLEANUP.push({ path: `/purchase/orders/${poId}`, label: `purchase_order#${poId}` });

  await apiCall(page, 'POST', `/purchase/orders/${poId}/submit`);
  await apiCall(page, 'POST', `/purchase/orders/${poId}/approve`);

  const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/orders/${poId}`);
  expect(detail.status, 'approve 回读状态词值（大写词表）').toBe('APPROVED');
  const items = pickListArray<Record<string, unknown>>(
    { items: detail.items },
    'items',
    `PO ${poId} 明细`
  );
  expect(items.length, `PO 明细行数应等于提交行数`).toBe(lines.length);

  const seeded: SeedLine[] = [];
  for (const [i, l] of lines.entries()) {
    const row = items[i];
    expectDecEquals(row.quantity, decLiteral(l.qty, '提交订购量'), `PO行${i + 1} 订购量回读`);
    expectDecEquals(row.unit_price, decLiteral(l.price, '提交单价'), `PO行${i + 1} 单价回读`);
    expectDecEquals(row.received_quantity, { v: 0n, s: 0 }, `PO行${i + 1} 初始已收量`);
    seeded.push({
      lineNo: i + 1,
      productId: l.product.id,
      materialCode: l.product.code,
      materialName: l.product.name,
      unit: l.product.unit,
      orderedQty: decLiteral(l.qty, '提交订购量'),
      unitPrice: decLiteral(l.price, '提交单价'),
      orderItemId: requireId(row, `PO行${i + 1} 明细 id`),
    });
  }
  return { poId, orderNo, seeded };
}

/** 取一个本用例专属产品（自建，避免引用库中 unit/编码未知的历史产品） */
async function ownProduct(page: Page, tag: string): Promise<{
  id: number;
  code: string;
  name: string;
  unit: string;
}> {
  const ctx = getCtx();
  if (!ctx.productCategoryIds[0]) throw new Error('前置缺失：ctx.productCategoryIds[0] 未就绪');
  const code = genCode(`E95${tag}`);
  const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/products', {
    code,
    name: `E2E采购链${tag}${code}`,
    unit: '米',
    category_id: ctx.productCategoryIds[0],
  });
  const id = requireId(created, `建产品 ${code}`);
  CLEANUP.push({ path: `/products/${id}`, label: `product#${id}` });
  const readBack = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/products/${id}`);
  expect(readBack.code, '产品编码回读').toBe(code);
  expect(readBack.unit, '产品计量单位回读（决定交货容差品类默认，必须已知）').toBe('米');
  return { id, code, name: String(readBack.name ?? ''), unit: String(readBack.unit ?? '米') };
}

/**
 * 按订单行建一张收货单（DRAFT）。批次/等级为本用例唯一值，库存行按同维度可精确定位。
 * 不送色号：色号非空即染色布、必须带缸号（布种判定唯一实现），本链按白坯收货。
 */
async function createReceipt(
  page: Page,
  opts: {
    poId: number;
    line: SeedLine;
    quantity: string;
    batchNo: string;
    grade: string;
    noteTag: string;
  }
): Promise<Record<string, unknown>> {
  const ctx = getCtx();
  return await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/receipts', {
    supplier_id: ctx.supplierId,
    order_id: opts.poId,
    receipt_date: todayStr(),
    warehouse_id: ctx.warehouseIds[0],
    department_id: ctx.departmentIds[0],
    notes: `E2E-F95-${opts.noteTag}`,
    items: [
      {
        line_no: 1,
        material_id: opts.line.productId,
        material_code: opts.line.materialCode,
        material_name: opts.line.materialName,
        batch_no: opts.batchNo,
        grade: opts.grade,
        quantity: opts.quantity,
        quantity_alt: '0',
        unit_master: opts.line.unit,
        unit_price: decText(opts.line.unitPrice),
      },
    ],
  });
}

/** 按「产品+仓库+批次+等级」定位唯一库存行；无行返回 null（不是空数组兜底） */
async function findStockRow(
  page: Page,
  opts: { productId: number; warehouseId: number; batchNo: string; grade: string }
): Promise<Record<string, unknown> | null> {
  const path =
    `/inventory/stock?product_id=${opts.productId}&warehouse_id=${opts.warehouseId}` +
    `&batch_no=${encodeURIComponent(opts.batchNo)}&grade=${encodeURIComponent(opts.grade)}` +
    '&page=1&page_size=50';
  const data = await apiCallRaw<Record<string, unknown>>(page, 'GET', path);
  const rows = pickListArray<Record<string, unknown>>(data, 'items', `库存行查询 ${path}`);
  const hit = rows.find(
    r => String(r.batch_no) === opts.batchNo && String(r.grade) === opts.grade
  );
  return hit ? (hit as Record<string, unknown>) : null;
}

async function readOrderLine(page: Page, poId: number, lineNo: number): Promise<Record<string, unknown>> {
  const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/orders/${poId}`);
  const items = pickListArray<Record<string, unknown>>(
    { items: detail.items },
    'items',
    `PO ${poId} 明细`
  );
  const row = items.find(r => Number(r.line_no) === lineNo);
  if (!row) throw new Error(`PO ${poId} 明细未找到行号 ${lineNo}，实际行号=${items.map(r => r.line_no).join(',')}`);
  return row;
}

/** 质检完成（结论走采购质检域英文词表）并把收货单推进到 COMPLETED */
async function inspectAndConfirm(
  page: Page,
  opts: { receiptId: number; supplierId: number; passQuantity?: string; context: string }
): Promise<void> {
  await seedInspectionPass(page, {
    receiptId: opts.receiptId,
    supplierId: opts.supplierId,
    passQuantity: opts.passQuantity,
    context: opts.context,
  });
  await apiCall(page, 'POST', `/purchase/receipts/${opts.receiptId}/confirm`);
}

test.describe('95 采购收货→入库→应付→付款 数值对账链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('95-01 分批续收数值守恒：库存增量精确等于本行收货量、订单已收累计、超收整单拒且零漂移', async ({
    page,
  }) => {
    const ctx = getCtx();
    const line = await ownProduct(page, 'A');
    // 订购 30 米、行级允差 0（上界=30），单价 12.50；分三批收 12/7/11 收满
    const { poId, seeded } = await seedApprovedOrder(page, [
      { product: line, qty: '30', price: '12.50', taxRate: '0' },
    ]);
    const orderLine = seeded[0];
    const grade = '一等品';
    const batchA = `E95A${genCode('B')}`;
    const whId = ctx.warehouseIds[0];

    // 收货前基线：本用例唯一批次维度下不应存在任何库存行（有则说明夹具维度不是自建的）
    const baseRow = await findStockRow(page, {
      productId: orderLine.productId,
      warehouseId: whId,
      batchNo: batchA,
      grade,
    });
    expect(baseRow, `批次 ${batchA} 收货前就存在库存行，基准不唯一`).toBeNull();

    // ── 第一批 12 ──
    const r1 = await createReceipt(page, {
      poId,
      line: orderLine,
      quantity: '12',
      batchNo: batchA,
      grade,
      noteTag: `R1-${batchA}`,
    });
    const r1Id = requireId(r1, '第一批收货单');
    CLEANUP.push({ path: `/purchase/receipts/${r1Id}`, label: `purchase_receipt#${r1Id}` });
    expect(r1.receipt_status, '收货登记初始态（大写词表）').toBe('DRAFT');
    await inspectAndConfirm(page, {
      receiptId: r1Id,
      supplierId: ctx.supplierId!,
      passQuantity: '12',
      context: '95-01 第一批',
    });

    const stock1 = await findStockRow(page, {
      productId: orderLine.productId,
      warehouseId: whId,
      batchNo: batchA,
      grade,
    });
    expect(stock1, `第一批确认入库后应生成批次 ${batchA} 的库存行`).not.toBeNull();
    expect(Number(stock1!.warehouse_id), '库存行必须落在收货单指定的仓库（入对仓位）').toBe(whId);
    expect(stock1!.product_id, '库存行产品维度').toBe(orderLine.productId);
    const stockOnHand1 = expectDecimalString(stock1!.quantity_on_hand, '第一批后库存现存量');
    expect(
      decCmp(stockOnHand1, decLiteral('12', '第一批收货量')) === 0,
      `第一批入库增量必须精确等于收货量：期望=12 实际=${decText(stockOnHand1)}`
    ).toBe(true);

    const lineAfter1 = await readOrderLine(page, poId, 1);
    expectDecEquals(lineAfter1.received_quantity, decLiteral('12', '第一批后已收'), '第一批后订单行已收量');
    const poAfter1 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/orders/${poId}`);
    expect(poAfter1.status, '部分收货状态词值（分批续收前提）').toBe('PARTIAL_RECEIVED');

    // ── 第二批同批次号续收 7：库存必须在既有行上累加，增量精确等于 7 ──
    const r2 = await createReceipt(page, {
      poId,
      line: orderLine,
      quantity: '7',
      batchNo: batchA,
      grade,
      noteTag: `R2-${batchA}`,
    });
    const r2Id = requireId(r2, '第二批收货单');
    CLEANUP.push({ path: `/purchase/receipts/${r2Id}`, label: `purchase_receipt#${r2Id}` });
    await inspectAndConfirm(page, {
      receiptId: r2Id,
      supplierId: ctx.supplierId!,
      passQuantity: '7',
      context: '95-01 第二批',
    });

    const stock2 = await findStockRow(page, {
      productId: orderLine.productId,
      warehouseId: whId,
      batchNo: batchA,
      grade,
    });
    expect(stock2, '第二批入库后库存行仍在同一维度').not.toBeNull();
    expect(stock2!.id, '同维度续收必须复用同一库存行，不得另起一行（否则四维追溯分裂）').toBe(
      stock1!.id
    );
    const before = decFromRaw(stock1!.quantity_on_hand, '第一批后现存量');
    const after = decFromRaw(stock2!.quantity_on_hand, '第二批后现存量');
    const delta = { v: after.v - before.v, s: after.s };
    expectDecMatch(delta, decLiteral('7', '第二批收货量'), '第二批库存增量精确等于本行收货量');

    const lineAfter2 = await readOrderLine(page, poId, 1);
    expectDecEquals(
      lineAfter2.received_quantity,
      decLiteral('19', '两批累计已收'),
      '分批续收后订单行已收量按批累计'
    );

    // ── 超收被拒：上界 30（允差 0），已收 19，本批 12 → 累计 31 越界，整单拒绝 ──
    const overQty = '12';
    const batchOver = `E95X${genCode('B')}`;
    const overFail = await apiCallExpectFail(page, 'POST', '/purchase/receipts', {
      supplier_id: ctx.supplierId,
      order_id: poId,
      receipt_date: todayStr(),
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      notes: `E2E-F95-OVER-${batchOver}`,
      items: [
        {
          line_no: 1,
          material_id: orderLine.productId,
          material_code: orderLine.materialCode,
          material_name: orderLine.materialName,
          batch_no: batchOver,
          grade,
          quantity: overQty,
          quantity_alt: '0',
          unit_master: orderLine.unit,
          unit_price: decText(orderLine.unitPrice),
        },
      ],
    });
    expect(
      overFail.status,
      `超收建单应被拒（HTTP 400），实际=${overFail.status} code=${overFail.code ?? '(none)'}`
    ).toBe(400);
    expect(
      failureCode(overFail),
      `超收拒绝必须归因到业务门机器码，实际 code=${JSON.stringify(overFail.code)}；` +
        '注意：该拒绝文案由后端脱敏族产出（真实原因"入库量超过允许收货上界"只进日志），' +
        'e2e 只能锁机器码，用户侧看不到超收原因属源码待判责项，勿在此放宽成 >=400'
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 零漂移三证：超收批次无库存行、订单已收量不动、同 PO 收货单行数不动
    const overStock = await findStockRow(page, {
      productId: orderLine.productId,
      warehouseId: whId,
      batchNo: batchOver,
      grade,
    });
    expect(overStock, `被拒的超收批次 ${batchOver} 不应留下库存行（整单未落库）`).toBeNull();
    const lineAfterOver = await readOrderLine(page, poId, 1);
    expectDecEquals(
      lineAfterOver.received_quantity,
      decLiteral('19', '超收被拒后已收量'),
      '超收被拒后订单行已收量不得漂移'
    );
    const receiptsAfterOver = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/purchase/receipts?order_id=${poId}&page=1&page_size=50`
      ),
      'items',
      `PO ${poId} 收货单列表（超收被拒后）`
    );
    expect(
      receiptsAfterOver.length,
      `超收被拒不得留下收货单，实际行数=${receiptsAfterOver.length}（本例应只有 2 张）`
    ).toBe(2);

    // ── 收满 30：剩余 11 合法续收，状态推进 COMPLETED ──
    const r3 = await createReceipt(page, {
      poId,
      line: orderLine,
      quantity: '11',
      batchNo: batchA,
      grade,
      noteTag: `R3-${batchA}`,
    });
    const r3Id = requireId(r3, '第三批收货单');
    CLEANUP.push({ path: `/purchase/receipts/${r3Id}`, label: `purchase_receipt#${r3Id}` });
    await inspectAndConfirm(page, {
      receiptId: r3Id,
      supplierId: ctx.supplierId!,
      passQuantity: '11',
      context: '95-01 第三批',
    });
    const lineFinal = await readOrderLine(page, poId, 1);
    expectDecEquals(lineFinal.received_quantity, orderLine.orderedQty, '收满后已收量=订购量');
    const poFinal = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/orders/${poId}`);
    expect(poFinal.status, '全收状态词值').toBe('COMPLETED');
    const stockFinal = await findStockRow(page, {
      productId: orderLine.productId,
      warehouseId: whId,
      batchNo: batchA,
      grade,
    });
    expectDecEquals(
      stockFinal!.quantity_on_hand,
      orderLine.orderedQty,
      '三批入库后库存现存量必须与订购量逐值相等（跨环数量一致）'
    );
  });

  test('95-02 质检词表同源与不合格拦截：中文结论逐字符、REJECTED 回写、入库被拦点名原因且零漂移、退货数量=不合格数', async ({
    page,
  }) => {
    const ctx = getCtx();
    const line = await ownProduct(page, 'Q');
    const { poId, seeded } = await seedApprovedOrder(page, [
      { product: line, qty: '20', price: '12.50', taxRate: '0' },
    ]);
    const orderLine = seeded[0];
    const grade = '一等品';
    const batch = `E95Q${genCode('B')}`;

    const receipt = await createReceipt(page, {
      poId,
      line: orderLine,
      quantity: '20',
      batchNo: batch,
      grade,
      noteTag: `BAD-${batch}`,
    });
    const receiptId = requireId(receipt, '不合格场景收货单');
    CLEANUP.push({ path: `/purchase/receipts/${receiptId}`, label: `purchase_receipt#${receiptId}` });

    // 待检初值：新建收货单检验状态恒 PENDING（词表 purchase_receipt_inspection 大写）
    const head0 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${receiptId}`
    );
    expect(head0.inspection_status, '新建收货单检验状态初值').toBe('PENDING');

    // 质检结论「不合格」→ 收货单检验状态必须映射为 REJECTED（两表两套词表，映射唯一实现）
    const insp = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/inspections', {
      receipt_id: receiptId,
      supplier_id: ctx.supplierId,
      inspection_date: todayStr(),
    });
    const inspId = requireId(insp, '建质检单');
    CLEANUP.push({ path: `/purchase/inspections/${inspId}`, label: `purchase_inspection#${inspId}` });
    await apiCall(page, 'POST', `/purchase/inspections/${inspId}/complete`, {
      pass_quantity: '0',
      reject_quantity: '20',
      inspection_result: 'fail',
    });
    const headRejected = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${receiptId}`
    );
    expect(headRejected.inspection_status, '质检不合格回写检验状态（大写词表）').toBe('REJECTED');
    const inspReadBack = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/inspections/${inspId}`
    );
    expect(inspReadBack.inspection_result, '采购质检结论词表（英文小写码）').toBe('fail');
    expectDecEquals(
      inspReadBack.reject_quantity,
      decLiteral('20', '不合格数量'),
      '不合格数量回读=收货全数'
    );

    // 不合格不得入库：确认被状态门拒，且拒绝原因可外显 → 逐字符锁文案
    const confirmFail = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/receipts/${receiptId}/confirm`
    );
    expect(confirmFail.status, `不合格收货单确认入库应 400，实际=${confirmFail.status}`).toBe(400);
    expect(failureCode(confirmFail), '不合格拦截机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      confirmFail.message,
      '不合格拦截的可外显文案必须逐字符命中门控实现'
    ).toBe('质检不合格的收货单不能确认入库，请先处理不合格品');

    // 拦截必须无痕：库存零行、订单已收零、收货单仍 DRAFT
    const stockAfterReject = await findStockRow(page, {
      productId: orderLine.productId,
      warehouseId: ctx.warehouseIds[0],
      batchNo: batch,
      grade,
    });
    expect(stockAfterReject, `被拦下的收货单批次 ${batch} 不得产生库存行`).toBeNull();
    const lineAfterReject = await readOrderLine(page, poId, 1);
    expectDecEquals(lineAfterReject.received_quantity, { v: 0n, s: 0 }, '被拦后订单行已收量仍为 0');
    const headAfterReject = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${receiptId}`
    );
    expect(headAfterReject.receipt_status, '被拦后收货单状态不得半推进').toBe('DRAFT');

    // 通用质检记录域的结论词表是中文（待检/合格/不合格），禁止英文化：
    // 中文「不合格」经映射写大写 REJECTED；送英文码必须被拒并点名允许清单。
    const cnRecord = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/production/quality-inspection/records',
      {
        inspection_no: genCode('E95QIR'),
        inspection_type: 'incoming',
        related_type: 'PURCHASE_RECEIPT',
        related_id: receiptId,
        product_id: orderLine.productId,
        inspection_date: todayStr(),
        total_qty: '20',
        inspected_qty: '20',
        qualified_qty: '0',
        unqualified_qty: '20',
        qualification_rate: '0',
        inspection_result: '不合格',
      }
    );
    const cnRecordId = requireId(cnRecord, '中文结论质检记录');
    CLEANUP.push({
      path: `/production/quality-inspection/records/${cnRecordId}`,
      label: `quality_inspection_record#${cnRecordId}`,
    });
    const cnReadBack = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/quality-inspection/records/${cnRecordId}`
    );
    expect(cnReadBack.inspection_result, '中文结论必须原样落库（禁止被转成英文码）').toBe('不合格');
    const headAfterCn = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${receiptId}`
    );
    expect(
      headAfterCn.inspection_status,
      '中文「不合格」映射到的入库单检验状态必须仍是词表原值 REJECTED'
    ).toBe('REJECTED');

    const enFail = await apiCallExpectFail(page, 'POST', '/production/quality-inspection/records', {
      inspection_no: genCode('E95QEN'),
      inspection_type: 'incoming',
      product_id: orderLine.productId,
      inspection_date: todayStr(),
      total_qty: '1',
      inspected_qty: '1',
      inspection_result: 'fail',
    });
    expect(enFail.status, `英文化结论应 400，实际=${enFail.status}`).toBe(400);
    expect(failureCode(enFail), '英文化结论机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      enFail.message,
      '结论越界文案必须逐字符回显允许清单（中文词表本体，不得被脱敏成通用文案）'
    ).toBe('检验结果 fail 不是合法取值，允许值：待检/合格/不合格');

    // 不合格 → 退货：退货数量必须与不合格数量逐值一致（跨环数量一致）
    const rtn = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/returns', {
      receipt_id: receiptId,
      order_id: poId,
      supplier_id: ctx.supplierId,
      return_date: todayStr(),
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      reason_type: '品质瑕疵',
      reason_detail: `E2E-F95-${batch}`,
    });
    const rtnId = requireId(rtn, '建退货单');
    CLEANUP.push({ path: `/purchase/returns/${rtnId}`, label: `purchase_return#${rtnId}` });
    const rtnLine = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/purchase/returns/${rtnId}/items`,
      {
        line_no: 1,
        material_id: orderLine.productId,
        quantity_returned: '20',
        unit_price: '12.50',
        batch_no: batch,
      }
    );
    expect(rtnLine, '退货明细创建应回显落库行').toBeTruthy();
    const rtnLines = pickListArray<Record<string, unknown>>(
      await apiCallRaw<unknown>(page, 'GET', `/purchase/returns/${rtnId}/items`),
      'bare',
      `退货单 ${rtnId} 明细`
    );
    expect(rtnLines.length, '退货明细行数').toBe(1);
    const returnedQty = decFromRaw(rtnLines[0].quantity_returned, '退货明细退货量');
    const inspReject = decFromRaw(inspReadBack.reject_quantity, '质检不合格数量');
    expect(
      decCmp(returnedQty, inspReject) === 0,
      `退货数量必须与质检不合格数量逐值相等：退货=${decText(returnedQty)} 不合格=${decText(inspReject)}`
    ).toBe(true);
    const rtnHead = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/returns/${rtnId}`);
    expect(rtnHead.return_status, '退货初始状态词值（小写词表）').toBe('draft');
    expect(rtnHead.receipt_id, '退货必须回指被判不合格的收货单').toBe(receiptId);
    expectDecEquals(
      rtnHead.total_quantity,
      inspReject,
      '退货主表合计=明细退货量=质检不合格量（三环闭合）'
    );
  });

  test('95-03 收货→应付→付款金额账：应付金额=收货行金额合计（十进制串逐值），部分付款余额闭合，应付凭证存在且借贷平衡', async ({
    page,
  }) => {
    const ctx = getCtx();
    const pA = await ownProduct(page, 'M1');
    const pB = await ownProduct(page, 'M2');
    // 两行不同单价、税率 0，把「收货行金额合计 → 应付金额」这条账做成不含税的纯金额闭合
    const { poId, seeded } = await seedApprovedOrder(page, [
      { product: pA, qty: '10', price: '20.00', taxRate: '0' },
      { product: pB, qty: '4', price: '25.00', taxRate: '0' },
    ]);
    const batch = `E95M${genCode('B')}`;
    const grade = '一等品';
    const receipt = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/receipts', {
      supplier_id: ctx.supplierId,
      order_id: poId,
      receipt_date: todayStr(),
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      notes: `E2E-F95-PAY-${batch}`,
      items: [
        {
          line_no: 1,
          material_id: pA.id,
          material_code: pA.code,
          material_name: pA.name,
          batch_no: batch,
          grade,
          quantity: '6',
          quantity_alt: '0',
          unit_master: pA.unit,
          unit_price: '20.00',
        },
        {
          line_no: 2,
          material_id: pB.id,
          material_code: pB.code,
          material_name: pB.name,
          batch_no: batch,
          grade,
          quantity: '4',
          quantity_alt: '0',
          unit_master: pB.unit,
          unit_price: '25.00',
        },
      ],
    });
    const receiptId = requireId(receipt, '多行收货单');
    CLEANUP.push({ path: `/purchase/receipts/${receiptId}`, label: `purchase_receipt#${receiptId}` });
    expect(seeded.length, '前置：PO 应为 2 行').toBe(2);

    const linesBeforeConfirm = pickListArray<Record<string, unknown>>(
      await apiCallRaw<unknown>(page, 'GET', `/purchase/receipts/${receiptId}/items`),
      'bare',
      `收货单 ${receiptId} 明细（确认前）`
    );
    expect(linesBeforeConfirm.length, '收货明细应落 2 行').toBe(2);
    // 行金额 = 数量 × 单价，逐行独立算再累加（十进制串，不用 JS number）
    for (const [i, row] of linesBeforeConfirm.entries()) {
      const qty = decFromRaw(row.quantity, `收货明细${i + 1} 数量`);
      const price = decFromRaw(row.unit_price, `收货明细${i + 1} 单价`);
      const amount = decFromRaw(row.amount, `收货明细${i + 1} 行金额`);
      const expected = { v: qty.v * price.v, s: qty.s + price.s };
      expectDecMatch(amount, expected, `收货明细${i + 1} 行金额=数量×单价`);
    }
    const lineAmountSum = decSumOf(linesBeforeConfirm, 'amount', '收货行金额合计');

    await seedInspectionPass(page, {
      receiptId,
      supplierId: ctx.supplierId!,
      context: '95-03 多行收货单',
    });
    await apiCall(page, 'POST', `/purchase/receipts/${receiptId}/confirm`);

    const rcpt = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/receipts/${receiptId}`);
    expect(rcpt.receipt_status, '确认后收货单状态').toBe('COMPLETED');
    expectDecEquals(
      rcpt.total_amount,
      lineAmountSum,
      '收货单总额必须等于明细行金额合计（同额两条路径）'
    );

    // 应付单：来源锚定本收货单，金额逐值等于收货行金额合计
    const apList = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/ap/invoices?supplier_id=${ctx.supplierId}&page=1&page_size=100`
      ),
      'items',
      '应付发票列表'
    );
    const invoice = apList.find(r => Number(r.source_id) === receiptId);
    expect(invoice, `收货单 ${receiptId} 确认后应自动生成应付单`).toBeTruthy();
    const invoiceId = requireId(invoice!, '应付单 id');
    expect(invoice!.source_type, '应付来源类型锚定').toBe('PURCHASE_RECEIPT');
    expectDecimalString(invoice!.amount, '应付单金额');
    expectDecEquals(invoice!.amount, lineAmountSum, '应付金额=收货行金额合计');
    expectDecEquals(
      invoice!.unpaid_amount,
      lineAmountSum,
      '未付余额初值=应付金额（全额未付）'
    );
    expect(invoice!.invoice_status, '自动应付初始态').toBe('DRAFT');

    // 应付凭证：入库确认链路必须真生成，且借贷平衡、金额与应付单同口径
    const invoiceNo = String(invoice!.invoice_no ?? '');
    if (!invoiceNo) throw new Error(`应付单缺 invoice_no，实际键=${Object.keys(invoice!).join(',')}`);
    const invHead = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ap/invoices/${invoiceId}`);
    const invTax = decFromRaw(invHead.tax_amount, '应付单税额');
    // GET /vouchers 列表不服务端过滤 source_type、且分页返回，按来源单据号翻页全量定位
    const apVoucher = await findVoucherBySourceBillNo(page, invoiceNo, '采购入库应付凭证');
    expect(
      apVoucher,
      `应付单 ${invoiceNo} 必须已生成应付凭证（凭证缺失=跨环金额无人对账的缺陷族）`
    ).toBeTruthy();
    const apVoucherId = requireId(apVoucher!, '应付凭证 id');
    const apVoucherDetail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/vouchers/${apVoucherId}`
    );
    expect(apVoucherDetail.status, '凭证过账态（小写词表）').toBe('posted');
    const apEntries = pickEntries(apVoucherDetail, `凭证 ${apVoucherId} 分录`);
    expect(apEntries.length >= 2, `应付凭证分录至少 2 行，实际=${apEntries.length}`).toBe(true);
    // 出参形态锁：凭证分录借贷列必须逐行是十进制字符串，退化成 JSON number 即判红
    for (const [i, row] of apEntries.entries()) {
      expectDecimalString(row.debit, `应付凭证 ${apVoucherId} 第 ${i + 1} 行 debit`);
      expectDecimalString(row.credit, `应付凭证 ${apVoucherId} 第 ${i + 1} 行 credit`);
    }
    const apDebit = decSumOf(apEntries, 'debit', '应付凭证借方合计');
    const apCredit = decSumOf(apEntries, 'credit', '应付凭证贷方合计');
    expect(
      decCmp(apDebit, apCredit) === 0,
      `应付凭证借贷必须平衡：借=${decText(apDebit)} 贷=${decText(apCredit)}`
    ).toBe(true);
    const payableEntry = apEntries.find(r => String(r.subject_code) === '2202');
    expect(payableEntry, '应付凭证必须有应付账款（2202）贷方分录').toBeTruthy();
    const payableHead = invHead.amount as unknown;
    const expectedPayable = decAdd(decFromRaw(payableHead, '应付单金额'), invTax);
    expectDecEquals(
      payableEntry!.credit,
      expectedPayable,
      '应付账款贷方发生额=应付单价税合计'
    );
    expectDecMatch(apCredit, expectedPayable, '应付凭证贷方合计=应付单价税合计');

    // 审核应付 → 付款申请（先只申请 60%，走部分付款分支）
    await apiCall(page, 'POST', `/ap/invoices/${invoiceId}/approve`);
    const audited = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ap/invoices/${invoiceId}`);
    expect(audited.invoice_status, '审核后的应付状态词值（大写词表）').toBe('AUDITED');
    const invoiceAmount = decFromRaw(audited.amount, '应付单金额（审核后回读）');
    const firstApply = decAdd(invoiceAmount, { v: 0n, s: 0 });
    // 部分付款额：按十进制取整到元级再减，避免浮点
    const partialPay = { v: firstApply.v / 2n, s: firstApply.s };
    const applyText = decText(partialPay);
    const restPay = decAdd(
      { v: -partialPay.v, s: partialPay.s },
      firstApply
    );
    const restText = decText(restPay);

    const req1 = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payment-requests', {
      supplier_id: ctx.supplierId,
      request_date: todayStr(),
      payment_type: '货款',
      payment_method: '银行转账',
      request_amount: applyText,
      currency: 'USD',
      exchange_rate: '1',
      notes: `E2E-F95-REQ1-${batch}`,
      items: [{ invoice_id: invoiceId, apply_amount: applyText }],
    });
    const req1Id = requireId(req1, '建部分付款申请');
    CLEANUP.push({ path: `/ap/payment-requests/${req1Id}`, label: `ap_request#${req1Id}` });
    await apiCall(page, 'POST', `/ap/payment-requests/${req1Id}/submit`);
    await apiCall(page, 'POST', `/ap/payment-requests/${req1Id}/approve`);

    // 非法流转：未审批通过的申请不得直接生成付款单 —— 可外显站点必须点名原因
    const req2Raw = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payment-requests', {
      supplier_id: ctx.supplierId,
      request_date: todayStr(),
      payment_type: '货款',
      payment_method: '银行转账',
      request_amount: restText,
      currency: 'USD',
      exchange_rate: '1',
      notes: `E2E-F95-REQ2-${batch}`,
      items: [{ invoice_id: invoiceId, apply_amount: restText }],
    });
    const req2Id = requireId(req2Raw, '建第二笔付款申请');
    CLEANUP.push({ path: `/ap/payment-requests/${req2Id}`, label: `ap_request#${req2Id}` });
    const payTooEarly = await apiCallExpectFail(page, 'POST', '/ap/payments', {
      request_id: req2Id,
      payment_date: todayStr(),
      notes: `E2E-F95-EARLY-${batch}`,
    });
    expect(payTooEarly.status, `未审批申请建付款单应 400，实际=${payTooEarly.status}`).toBe(400);
    expect(failureCode(payTooEarly), '未审批申请机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(payTooEarly.message, '未审批申请的可外显拒绝文案').toBe(
      '付款申请尚未审批通过，请先完成审批后再创建付款单'
    );
    const req2Unchanged = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payment-requests/${req2Id}`
    );
    expect(req2Unchanged.approval_status, '被拒后申请状态不得半推进').toBe('DRAFT');
    const earlyPayments = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(page, 'GET', '/ap/payments?page=1&page_size=100'),
      'items',
      '付款单列表（未审批申请被拒后）'
    );
    expect(
      earlyPayments.some(p => Number(p.request_id) === req2Id),
      '未审批申请被拒后不得留下付款单（假成功未落库的反向验证）'
    ).toBe(false);

    const pay1 = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payments', {
      request_id: req1Id,
      payment_date: todayStr(),
      notes: `E2E-F95-PAY1-${batch}`,
    });
    const pay1Id = requireId(pay1, '建第一笔付款单');
    CLEANUP.push({ path: `/ap/payments/${pay1Id}`, label: `ap_payment#${pay1Id}` });
    const pay1Head = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ap/payments/${pay1Id}`);
    expect(pay1Head.payment_status, '付款单登记态').toBe('REGISTERED');
    expectDecEquals(pay1Head.payment_amount, partialPay, '付款金额取申请额');

    // 确认门前置：缺交易流水号必须被拦并点名缺失字段（用户自提字段规则，可外显）
    const noTxn = await apiCallExpectFail(page, 'POST', `/ap/payments/${pay1Id}/confirm`);
    expect(noTxn.status, `缺交易流水号确认付款应 400，实际=${noTxn.status}`).toBe(400);
    expect(failureCode(noTxn), '缺流水号机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(noTxn.message, '缺流水号可外显文案').toBe('付款单必须填写交易流水号才能确认');
    const payStillRegistered = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payments/${pay1Id}`
    );
    expect(payStillRegistered.payment_status, '被拦后付款单状态不动').toBe('REGISTERED');

    await apiCall(page, 'PUT', `/ap/payments/${pay1Id}`, {
      transaction_no: `E2E-F95-TXN1-${batch}`,
    });
    await apiCall(page, 'POST', `/ap/payments/${pay1Id}/confirm`);
    const pay1After = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ap/payments/${pay1Id}`);
    expect(pay1After.payment_status, '确认后付款单态').toBe('CONFIRMED');

    // 部分付款分支对账：已付+未付=应付金额，状态 PARTIAL_PAID
    const invPartial = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/invoices/${invoiceId}`
    );
    expectDecimalString(invPartial.paid_amount, '应付已付额');
    expectDecEquals(invPartial.paid_amount, partialPay, '部分付款后已付额=本笔付款额');
    expect(invPartial.invoice_status, '部分付款状态词值').toBe('PARTIAL_PAID');
    const paidNow = decFromRaw(invPartial.paid_amount, '已付额');
    const unpaidNow = decFromRaw(invPartial.unpaid_amount, '未付额');
    const totalNow = decFromRaw(invPartial.amount, '应付额');
    expect(
      decCmp(decAdd(paidNow, unpaidNow), totalNow) === 0,
      `部分付款余额闭合：已付 ${decText(paidNow)} + 未付 ${decText(unpaidNow)} ≠ 应付 ${decText(totalNow)}`
    ).toBe(true);

    // 付款凭证：借应付账款、贷银行存款，两行金额都等于本次付款额且平衡
    const pay1No = String(pay1After.payment_no ?? '');
    if (!pay1No) throw new Error(`付款单缺 payment_no，实际键=${Object.keys(pay1After).join(',')}`);
    const payVoucher = await findVoucherBySourceBillNo(page, pay1No, '付款凭证');
    expect(
      payVoucher,
      `付款单 ${pay1No} 确认后必须生成付款凭证`
    ).toBeTruthy();
    const payVoucherId = requireId(payVoucher!, '付款凭证 id');
    const payVoucherDetail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/vouchers/${payVoucherId}`
    );
    expect(payVoucherDetail.voucher_type, '付款凭证字（中文单字常量）').toBe('付');
    expect(payVoucherDetail.status, '付款凭证过账态').toBe('posted');
    const payEntries = pickEntries(payVoucherDetail, `付款凭证 ${payVoucherId} 分录`);
    expect(payEntries.length, '付款凭证应恰为一借一贷两行').toBe(2);
    const debitPayable = payEntries.find(r => String(r.subject_code) === '2202');
    const creditBank = payEntries.find(r => String(r.subject_code) === '1002');
    expect(debitPayable, '付款凭证必须有应付账款（2202）借方').toBeTruthy();
    expect(creditBank, '非现金付款方式必须贷银行存款（1002）').toBeTruthy();
    expectDecEquals(debitPayable!.debit, partialPay, '付款凭证借方=本次付款额');
    expectDecEquals(debitPayable!.credit, { v: 0n, s: 0 }, '借方行的贷方列必须为 0（不得双记）');
    expectDecEquals(creditBank!.credit, partialPay, '付款凭证贷方=本次付款额');
    expectDecEquals(creditBank!.debit, { v: 0n, s: 0 }, '贷方行的借方列必须为 0（不得双记）');
  });

  test('95-04 含税链闭合：应付账款科目在付清后必须归零（价税合计口径跨环一致）', async ({
    page,
  }) => {
    const ctx = getCtx();
    const prod = await ownProduct(page, 'T');
    const { poId, seeded } = await seedApprovedOrder(page, [
      { product: prod, qty: '10', price: '20.00', taxRate: '13' },
    ]);
    const orderLine = seeded[0];
    const batch = `E95T${genCode('B')}`;
    const grade = '一等品';

    const receipt = await createReceipt(page, {
      poId,
      line: orderLine,
      quantity: '10',
      batchNo: batch,
      grade,
      noteTag: `TAX-${batch}`,
    });
    const receiptId = requireId(receipt, '含税收货单');
    CLEANUP.push({ path: `/purchase/receipts/${receiptId}`, label: `purchase_receipt#${receiptId}` });

    await seedInspectionPass(page, {
      receiptId,
      supplierId: ctx.supplierId!,
      passQuantity: '10',
      context: '95-04 含税收货单',
    });
    await apiCall(page, 'POST', `/purchase/receipts/${receiptId}/confirm`);

    const apList = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/ap/invoices?supplier_id=${ctx.supplierId}&page=1&page_size=100`
      ),
      'items',
      '含税应付列表'
    );
    const invoice = apList.find(r => Number(r.source_id) === receiptId);
    expect(invoice, '含税收货确认后应自动生成应付单').toBeTruthy();
    const invoiceId = requireId(invoice!, '含税应付单 id');
    const invoiceNo = String(invoice!.invoice_no ?? '');
    const invHead = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ap/invoices/${invoiceId}`);
    const netAmount = decFromRaw(invHead.amount, '应付不含税额');
    const taxAmount = decFromRaw(invHead.tax_amount, '应付税额');
    const gross = decAdd(netAmount, taxAmount);

    // 应付凭证（价税合计挂应付账款贷方）
    const apVoucher = await findVoucherBySourceBillNo(page, invoiceNo, '含税应付凭证');
    expect(apVoucher, `应付单 ${invoiceNo} 必须有应付凭证`).toBeTruthy();
    const apDetail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/vouchers/${requireId(apVoucher!, '应付凭证')}`
    );
    const apEntries = pickEntries(apDetail, '含税应付凭证分录');
    // 出参形态锁：凭证分录借贷列必须逐行是十进制字符串，退化成 JSON number 即判红
    for (const [i, row] of apEntries.entries()) {
      expectDecimalString(row.debit, `含税应付凭证第 ${i + 1} 行 debit`);
      expectDecimalString(row.credit, `含税应付凭证第 ${i + 1} 行 credit`);
    }
    const payableCredit = decSumOf(
      apEntries.filter(r => String(r.subject_code) === '2202'),
      'credit',
      '应付账款贷方合计'
    );
    expectDecMatch(payableCredit, gross, '应付账款贷方发生额=价税合计');

    // 付清路径：申请额=账面未付额（不含税口径），确认后应付结清
    await apiCall(page, 'POST', `/ap/invoices/${invoiceId}/approve`);
    const req = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payment-requests', {
      supplier_id: ctx.supplierId,
      request_date: todayStr(),
      payment_type: '货款',
      payment_method: '银行转账',
      request_amount: decText(netAmount),
      currency: 'USD',
      exchange_rate: '1',
      notes: `E2E-F95-TAXREQ-${batch}`,
      items: [{ invoice_id: invoiceId, apply_amount: decText(netAmount) }],
    });
    const reqId = requireId(req, '含税付清申请');
    CLEANUP.push({ path: `/ap/payment-requests/${reqId}`, label: `ap_request#${reqId}` });
    await apiCall(page, 'POST', `/ap/payment-requests/${reqId}/submit`);
    await apiCall(page, 'POST', `/ap/payment-requests/${reqId}/approve`);
    const pay = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payments', {
      request_id: reqId,
      payment_date: todayStr(),
      notes: `E2E-F95-TAXPAY-${batch}`,
    });
    const payId = requireId(pay, '含税付款单');
    CLEANUP.push({ path: `/ap/payments/${payId}`, label: `ap_payment#${payId}` });
    await apiCall(page, 'PUT', `/ap/payments/${payId}`, {
      transaction_no: `E2E-F95-TXTN-${batch}`,
    });
    await apiCall(page, 'POST', `/ap/payments/${payId}/confirm`);

    const payNo = String(
      (await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ap/payments/${payId}`)).payment_no ?? ''
    );
    const payVoucher = await findVoucherBySourceBillNo(page, payNo, '含税付款凭证');
    expect(payVoucher, `付款单 ${payNo} 确认后必须生成付款凭证`).toBeTruthy();
    const payDetail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/vouchers/${requireId(payVoucher!, '付款凭证')}`
    );
    const payEntries = pickEntries(payDetail, '含税付款凭证分录');
    const payableDebit = decSumOf(
      payEntries.filter(r => String(r.subject_code) === '2202'),
      'debit',
      '应付账款借方合计'
    );

    // 业务闭合：同一张应付发票付清后，应付账款科目发生额必须轧平（贷-借=0）。
    // 当前实现里收货凭证按「价税合计」贷 2202、付款凭证按账面未付额（不含税）借 2202，
    // 两者相差整笔进项税额 → 科目永久挂账。这是跨环金额口径不一致的真实缺陷，
    // 保持判红并交回源码判责，禁止把本断言改成"两边各断各的"来换绿。
    const residue = { v: payableCredit.v - payableDebit.v, s: payableCredit.s };
    expect(
      decCmp(payableDebit, payableCredit) === 0,
      `应付账款科目未随付清轧平：贷方 ${decText(payableCredit)} − 借方 ${decText(payableDebit)} = ` +
        `${decText(residue)}（差额恰为进项税额 ${decText(taxAmount)}）。` +
        '根因：应付单 amount 按不含税落库、收货凭证按价税合计贷记、付款凭证只按账面未付额借记，' +
        '三处价税口径不同源，需源码统一口径后本用例才会转绿'
    ).toBe(true);

    const invPaid = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ap/invoices/${invoiceId}`);
    expect(invPaid.invoice_status, '付清后应付状态词值').toBe('PAID');
    expectDecEquals(invPaid.unpaid_amount, { v: 0n, s: 0 }, '付清后账面未付额归零');
  });
});
