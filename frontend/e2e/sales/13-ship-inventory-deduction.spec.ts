// 销售 E2E 套件 — 13 UI 发货出库 → 库存四维回写（扣减）+ 匹号消耗端到端
//
// 补齐审计缺口「UI 收货/发货库存回写仅 toast、未回读扣减」(P1) 的出库侧。
// 原 sales/04-04 发货仅断「发货成功」toast + 订单离开 approved，未回读库存量四维扣减。
// 本例以「UI 触发发货 + API 精确回读四维库存行 + 匹状态回读」组合，断言 quantity_available 减、
// quantity_shipped 增、被消耗匹 AVAILABLE→SHIPPED，禁只断 toast。
//
// 后端事实来源：
// - 出库扣减（services/so/delivery_ops/ship.rs → inventory.rs reduce_inventory_four_dim）：
//     按 款号+色号+缸号+批次 定位库存行，quantity_available -= 出库量、quantity_shipped += 出库量
//     （inventory.rs:310-348），quantity_on_hand 不变（on_hand=available+shipped 守恒）；
//     染色布（色号非空）另在同一事务内按四维 tuple CAS 消耗匹号（inventory.rs:350-366 →
//     piece_domain_service.rs:606 outbound_piece_filter，AVAILABLE→SHIPPED，未命中整单回滚）。
// - 发货对话框出库候选（sales/DeliveryDialog.vue）：stockRows[product_id] 来源 GET /inventory/stock
//     按 产品+仓库 下推查询（不按色号过滤）；匹号候选来源 GET /inventory/pieces 按
//     产品+仓库+缸号+批次+status=AVAILABLE 下推（useOlv.loadDeliveryPieces）。
//     本例库存行/匹全部落同一目标仓与唯一缸号，UI 按缸号/匹号文本精确锚定选定——
//     并发分片即使在同仓塞入别家库存行/匹，也无法与本例唯一缸号同 tuple，归因确定，
//     无需旧版"候选行数>1 即 skip"的脆弱前提（skip 已按四维口径改造移除，非放宽断言）。
// - GET /inventory/stock 回读 quantity_on_hand/quantity_available/quantity_shipped（admin 会话不脱敏）；
//   GET /inventory/pieces 回读匹 status（词表 models/status/purchase_inventory.rs::inventory_piece 大写）。
//
// 隔离策略：用 ctx.productIds[1]（专用产品，globalSeed 不为其备货）而非 productIds[0]，
// 仓库经 pickDyeableWarehouse 确定性选定并按名称在对话框选中（不再依赖下拉首项顺序）。
// seed 走 helpers.seedDyedOutboundBundle：batch=缸号 的四维库存行 + 委外染色真实链同维造匹。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelect, pickSelectIn } from '../flow/ui-helpers';
import {
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  tryCleanup,
  genCode,
  seedDyedOutboundBundle,
  pickDyeableWarehouse,
  fetchAvailableDyedPieces,
  readDyedPieceByNo,
} from '../flow/helpers';

interface SalesOrderLite {
  id: number;
  order_no: string;
  status: string;
}
interface StockRowLite {
  id: number;
  product_id: number;
  color_no: string;
  dye_lot_no: string | null;
  batch_no: string;
  quantity_on_hand: number | string;
  quantity_available: number | string;
  quantity_shipped: number | string;
}

const CREATED_ORDER_IDS: number[] = [];
test.afterEach(async ({ page }) => {
  while (CREATED_ORDER_IDS.length) {
    const id = CREATED_ORDER_IDS.pop();
    if (id != null) await tryCleanup(page, 'DELETE', `/sales/orders/${id}`, `so#${id}`);
  }
});

async function seedApprovedOrderForProduct(page: Page, productId: number): Promise<SalesOrderLite> {
  const ctx = getCtx();
  if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
  const created = await apiCall<SalesOrderLite>(page, 'POST', '/sales/orders', {
    customer_id: ctx.customerId,
    order_date: new Date().toISOString(),
    items: [{ product_id: productId, quantity: '10', unit_price: '25.00' }],
    notes: `E2E-S13-${genCode('X')}`,
  });
  const id = created.data?.id;
  const order_no = created.data?.order_no;
  if (!id || !order_no)
    throw new Error(`销售订单创建未返回 id/order_no：${JSON.stringify(created)}`);
  CREATED_ORDER_IDS.push(id);
  await apiCall(page, 'POST', `/sales/orders/${id}/submit`);
  await apiCall(page, 'POST', `/sales/orders/${id}/approve`);
  const after = await apiCallRaw<SalesOrderLite>(page, 'GET', `/sales/orders/${id}`);
  if (after.status !== 'approved') throw new Error(`approve 后应为 approved，实际 ${after.status}`);
  return { id, order_no, status: 'approved' };
}

async function readStockByDims(
  page: Page,
  productId: number,
  warehouseId: number,
  colorNo: string,
  dyeLotNo: string
): Promise<StockRowLite | null> {
  const res = await apiCallRaw<{ items?: unknown }>(
    page,
    'GET',
    `/inventory/stock?product_id=${productId}&warehouse_id=${warehouseId}` +
      `&color_no=${encodeURIComponent(colorNo)}&dye_lot_no=${encodeURIComponent(dyeLotNo)}&page=1&page_size=50`
  );
  const raw = (res as { items?: unknown }).items;
  const rows = Array.isArray(raw) ? (raw as StockRowLite[]) : [];
  // 本例 batch=缸号（写入方口径），色号+缸号两维下推后即可精确锁定本例行
  return rows.find(r => r.color_no === colorNo && String(r.dye_lot_no) === dyeLotNo) ?? null;
}

test.describe('13 UI 发货出库 → 库存四维回写扣减 + 匹号消耗', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
    await ensureTestEntities(page);
  });

  test('13-01 UI 发货选定匹号确定后：四维库存 available 减 shipped 增、匹回读 SHIPPED', async ({
    page,
  }) => {
    test.setTimeout(240_000);
    const ctx = getCtx();
    const productId = ctx.productIds[1];
    if (!productId) throw new Error('前置缺失：ctx.productIds[1] 未就绪');
    // 确定性目标仓（可承载染色匹）替代旧"warehouseIds[0]+首项下拉"双不确定性
    const target = await pickDyeableWarehouse(page);

    // 为该专用产品在目标仓灌：一行 batch=缸号 的完整四维库存（100 米）+ 同 tuple 真实
    // AVAILABLE 染色匹（委外染色真实链生成，helpers.seedDyedOutboundBundle），
    // 作为出库扣减与匹消耗的精确回读基准。
    const bundle = await seedDyedOutboundBundle(page, {
      productId,
      warehouseId: target.id,
      quantityMeters: '100',
      pieceCount: 1,
      context: 'S13-01',
    });
    const piece = bundle.pieces[0];
    const seeded = bundle.stockRow as unknown as StockRowLite;
    const beforeAvailable = Number(seeded.quantity_available);
    const beforeShipped = Number(seeded.quantity_shipped);
    const beforeOnHand = Number(seeded.quantity_on_hand);

    const order = await seedApprovedOrderForProduct(page, productId);

    // UI 发货：/sales 列表按唯一订单号筛本例行 → 「发货」→ 选仓→选库存行(缸号锚定)→
    // 选匹号(第四维)→填数量→确定发货
    await page.goto('/sales');
    await page.getByPlaceholder('订单号').fill(order.order_no);
    await page.getByRole('button', { name: '查询', exact: true }).click();
    await expect(
      page.getByRole('row').filter({ hasText: order.order_no }).first(),
      `销售列表应筛出 ${order.order_no}`
    ).toBeVisible({ timeout: 30000 });
    await page.getByRole('button', { name: '发货', exact: true }).first().click();
    const dialog = page.getByRole('dialog', { name: '销售发货' });
    await expect(dialog).toBeVisible();
    await pickSelectIn(dialog, page, '仓库', { optionText: target.name });
    await dialog.getByPlaceholder('选择日期').fill('2026-12-31');
    await page.keyboard.press('Enter');
    // 库存行下拉（第 2 个 el-select）：本例唯一缸号锚定
    await pickSelect(page, dialog.locator('.el-select').nth(1), bundle.dyeLotNo);
    // 匹候选发货前显式判红（与 UI 同口径下推查询）：为空即真实链断裂，不许 skip/放宽
    const candidates = await fetchAvailableDyedPieces(page, {
      productId,
      warehouseId: target.id,
      dyeLotNo: bundle.dyeLotNo,
    });
    if (!candidates.some(p => p.piece_no === piece.piece_no)) {
      throw new Error(
        `[13-01] 发货对话框匹候选为空/未含本例匹 ${piece.piece_no}（回读命中 ${candidates.length} 匹）` +
          '——第四维候选链路真实断裂，判红不 skip'
      );
    }
    const pieceSel = dialog.locator('.el-select').nth(2);
    await pickSelect(page, pieceSel, piece.piece_no);
    await expect(
      pieceSel.locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)'),
      `匹号第四维应选中本例真实匹 ${piece.piece_no}`
    ).toContainText(piece.piece_no);
    await dialog.getByRole('spinbutton').first().fill('1');
    await dialog.getByRole('button', { name: '确定发货' }).click();
    await expect(page.getByText('发货成功')).toBeVisible({ timeout: 30000 });

    // 核心回读①：按 款号+色号+缸号+批次 精确取行，available 减 1、shipped 增 1、on_hand 守恒
    const after = await readStockByDims(
      page,
      productId,
      target.id,
      bundle.colorNo,
      bundle.dyeLotNo
    );
    expect(after, '发货后应能按四维回读到库存行').toBeTruthy();
    expect(
      Number(after!.quantity_available),
      `quantity_available 应扣减 1：${beforeAvailable} → ${beforeAvailable - 1}（实际 ${after!.quantity_available}）`
    ).toBe(beforeAvailable - 1);
    expect(
      Number(after!.quantity_shipped),
      `quantity_shipped 应累加 1：${beforeShipped} → ${beforeShipped + 1}（实际 ${after!.quantity_shipped}）`
    ).toBe(beforeShipped + 1);
    expect(
      Number(after!.quantity_on_hand),
      `on_hand 应守恒不变（available+shipped）：${beforeOnHand}（实际 ${after!.quantity_on_hand}）`
    ).toBe(beforeOnHand);

    // 核心回读②（第四维）：被选定匹必须同事务 CAS 消耗 AVAILABLE→SHIPPED，
    // 杜绝"账面扣了米、匹还挂着可用"的假绿。
    const consumed = await readDyedPieceByNo(page, {
      productId,
      warehouseId: target.id,
      dyeLotNo: bundle.dyeLotNo,
      batchNo: bundle.dyeLotNo,
      pieceNo: piece.piece_no,
    });
    expect(consumed, `发货后应能按四维 tuple 回读到匹 ${piece.piece_no}`).toBeTruthy();
    expect(
      String(consumed!.status),
      `匹状态应流转为 SHIPPED（词表 models/status/purchase_inventory.rs::inventory_piece，大写），实际 ${consumed!.status}`
    ).toBe('SHIPPED');

    // 订单侧联动：出库量应记入订单行 shipped_quantity
    const orderDetail = await apiCallRaw<{
      items?: Array<{ product_id: number; shipped_quantity: number | string }>;
    }>(page, 'GET', `/sales/orders/${order.id}`);
    const oline = (orderDetail.items ?? []).find(i => i.product_id === productId);
    expect(Number(oline?.shipped_quantity ?? -1), '订单行 shipped_quantity 应累加至 1').toBe(1);
  });
});
