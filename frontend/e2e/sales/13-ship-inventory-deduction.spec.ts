// 销售 E2E 套件 — 13 UI 发货出库 → 库存四维回写（扣减）端到端
//
// 补齐审计缺口「UI 收货/发货库存回写仅 toast、未回读扣减」(P1) 的出库侧。
// 原 sales/04-04 发货仅断「发货成功」toast + 订单离开 approved，未回读库存量四维扣减。
// 本例以「UI 触发发货 + API 精确回读四维库存行」组合，断言 quantity_available 减、quantity_shipped 增，禁只断 toast。
//
// 后端事实来源：
// - 出库扣减（services/so/delivery_ops/ship.rs → reduce_inventory_four_dim）：
//     按 款号+色号+缸号+批次 定位库存行，quantity_available -= 出库量、quantity_shipped += 出库量
//     （inventory.rs:287-305），quantity_on_hand 不变（on_hand=available+shipped 守恒）。
// - 发货对话框出库候选（sales/DeliveryDialog.vue）：stockRows[product_id] 来源 GET /inventory/stock
//     按 产品+仓库 下推查询（不按色号过滤），故本例为专用产品在目标仓仅备一行四维库存，令候选唯一、可精确归因。
// - GET /inventory/stock 回读 quantity_on_hand/quantity_available/quantity_shipped（admin 会话不脱敏）。
//
// 隔离策略：用 ctx.productIds[1]（专用产品，globalSeed 不为其备货）而非 productIds[0]，
// 若目标仓内该产品出现多于 1 条库存行（并发分片污染），显式 skip + 中文原因（真实环境前提不成立，非放宽断言）。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelect, pickSelectIn } from '../flow/ui-helpers';
import {
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  tryCleanup,
  seedFourDimStockIn,
  genCode,
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

async function locateStockRows(
  page: Page,
  productId: number,
  warehouseId: number
): Promise<StockRowLite[]> {
  const res = await apiCallRaw<{ items?: unknown }>(
    page,
    'GET',
    `/inventory/stock?product_id=${productId}&warehouse_id=${warehouseId}&page=1&page_size=50`
  );
  const raw = (res as { items?: unknown }).items;
  return Array.isArray(raw) ? (raw as StockRowLite[]) : [];
}

async function readStockByDims(
  page: Page,
  productId: number,
  warehouseId: number,
  colorNo: string,
  batchNo: string
): Promise<StockRowLite | null> {
  const rows = await locateStockRows(page, productId, warehouseId);
  return rows.find(r => r.color_no === colorNo && r.batch_no === batchNo) ?? null;
}

test.describe('13 UI 发货出库 → 库存四维回写扣减', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
    await ensureTestEntities(page);
  });

  test('13-01 UI 发货确定后回读四维库存行 available 减、shipped 增', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[1];
    const warehouseId = ctx.warehouseIds[0];
    if (!productId) throw new Error('前置缺失：ctx.productIds[1] 未就绪');
    if (!warehouseId) throw new Error('前置缺失：ctx.warehouseIds[0] 未就绪');

    const tag = genCode('S13');
    const colorNo = `C-${tag}`;
    const batchNo = `B-${tag}`;
    // 为该专用产品在目标仓灌入唯一一行完整四维库存（100 米），作为出库候选与扣减回读基准
    const seeded = (await seedFourDimStockIn(page, {
      productId,
      warehouseId,
      colorNo,
      dyeLotNo: `L-${tag}`,
      batchNo,
      quantityMeters: '100',
    })) as unknown as StockRowLite;
    const beforeAvailable = Number(seeded.quantity_available);
    const beforeShipped = Number(seeded.quantity_shipped);
    const beforeOnHand = Number(seeded.quantity_on_hand);

    // 前提校验：目标仓内该产品应仅有刚灌入的一行候选（出库对话框按产品+仓库列候选，唯一则可精确归因）
    const candidates = await locateStockRows(page, productId, warehouseId);
    test.skip(
      candidates.length > 1,
      `[并发隔离] 目标仓库 ${warehouseId} 下专用产品 ${productId} 库存行数=${candidates.length}（>1），` +
        `出库对话框候选非唯一，无法把扣减精确归因到本例库存行——真实共享库多分片污染，非本用例可放宽，跳过以免假绿`
    );

    const order = await seedApprovedOrderForProduct(page, productId);

    // UI 发货：/sales 列表按唯一订单号筛本例行 → 「发货」→ 选仓库→选库存行→填数量→确定发货
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
    await pickSelectIn(dialog, page, '仓库');
    await dialog.getByPlaceholder('选择日期').fill('2026-12-31');
    await page.keyboard.press('Enter');
    // 库存行下拉（仓库之后第 2 个 el-select），候选唯一即本例四维行
    await pickSelect(page, dialog.locator('.el-select').nth(1));
    await dialog.getByRole('spinbutton').first().fill('1');
    await dialog.getByRole('button', { name: '确定发货' }).click();
    await expect(page.getByText('发货成功')).toBeVisible({ timeout: 30000 });

    // 核心回读：按 款号+色号+缸号+批次 精确取行，available 减 1、shipped 增 1、on_hand 守恒
    const after = await readStockByDims(page, productId, warehouseId, colorNo, batchNo);
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

    // 订单侧联动：出库量应记入订单行 shipped_quantity
    const orderDetail = await apiCallRaw<{
      items?: Array<{ product_id: number; shipped_quantity: number | string }>;
    }>(page, 'GET', `/sales/orders/${order.id}`);
    const oline = (orderDetail.items ?? []).find(i => i.product_id === productId);
    expect(Number(oline?.shipped_quantity ?? -1), '订单行 shipped_quantity 应累加至 1').toBe(1);
  });
});
