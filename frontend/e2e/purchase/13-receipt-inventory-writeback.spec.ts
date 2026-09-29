// 采购 E2E 套件 — 13 UI 采购收货/确认入库 → 库存四维回写端到端
//
// 补齐审计缺口「UI 收货库存回写：仅 toast、未回读 quantity_on_hand 四维增加」(P1)。
// 本 spec 走「UI 触发 + API/UI 回读」组合，断言基于 GET /inventory/stock 真实回读，禁只断 toast。
//
// 后端事实来源（写入方为准）：
// - 收货登记：PurchaseReceiveDialog → POST /purchase/receipts 建 DRAFT 入库单（purchase_receipt_service.rs:75
//   写 status::purchase_receipt::DRAFT），对话框仅采集批次号（usePurchRcv.ts buildReceiptPayload），色号/缸号留空。
// - 确认入库：POST /purchase/receipts/{id}/confirm（purchase_receipt_ops/state.rs::confirm_receipt）
//   同事务内 ① 累加采购订单行 received_quantity 并推进订单状态（PARTIAL_RECEIVED/COMPLETED），
//   ② 写库存四维行（product+batch+color+lot）：新行 quantity_on_hand=quantity_meters
//   （purchase_receipt_private.rs::upsert_stock_for_item + inventory_stock_txn.rs::create_stock_fabric_txn:98），
//   ③ 状态转 COMPLETED。confirm 前 lock_exclusive 且状态门只放 DRAFT → 二次 confirm 被业务拒（不重复入库）。
// - GET /inventory/stock（inventory_stock_handler.rs list）data.items[].quantity_on_hand/quantity_available/
//   quantity_shipped/color_no/dye_lot_no/batch_no 为回读口径；product_id/color_no/dye_lot_no/batch_no/warehouse_id 为 SQL 下推过滤。
//
// 覆盖三条真实回写：
//  13-01 UI 列表「审核」确认带全四维的入库单 → GET /inventory/stock 按 款号+色号+缸号+批次 回读 quantity_on_hand 增加；
//       并回读订单行 received_quantity 增加、订单状态离开 APPROVED、入库单转 COMPLETED。
//  13-02 纯 UI「收货」对话框（批次维度）建单 + 确认 → 按 款号+批次 回读 quantity_on_hand 增加（白坯空色号/缸号合法）。
//  13-02b 确认入库幂等防御：重复 confirm 被业务拒（4xx），库存 quantity_on_hand 不被二次累加（边界/非法值）。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  tryCleanup,
  genCode,
} from '../flow/helpers';
import { pickSelectIn, pickListArray } from '../flow/ui-helpers';

interface PurchaseOrderLite {
  id: number;
  order_no: string;
  status: string;
}
interface PurchaseReceiptLite {
  id: number;
  order_id: number | null;
  receipt_no: string;
  receipt_status: string;
  total_quantity: number | string;
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
const CREATED_RECEIPT_IDS: number[] = [];
test.afterEach(async ({ page }) => {
  // 先删入库单再删订单（入库单引用订单；已 COMPLETED 的删除失败仅告警属预期）
  while (CREATED_RECEIPT_IDS.length) {
    const id = CREATED_RECEIPT_IDS.pop();
    if (id != null) await tryCleanup(page, 'DELETE', `/purchase/receipts/${id}`, `receipt#${id}`);
  }
  while (CREATED_ORDER_IDS.length) {
    const id = CREATED_ORDER_IDS.pop();
    if (id != null) await tryCleanup(page, 'DELETE', `/purchase/orders/${id}`, `po#${id}`);
  }
});

/** API 造一张本例专属 APPROVED 态采购订单（qty 默认 20），回查每步真实状态字面量。 */
async function seedApprovedPO(page: Page, qtyOrdered = '20'): Promise<PurchaseOrderLite> {
  const ctx = getCtx();
  if (!ctx.supplierId) throw new Error('前置缺失：ctx.supplierId 未就绪');
  if (!ctx.productIds[0]) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
  if (!ctx.warehouseIds[0]) throw new Error('前置缺失：ctx.warehouseIds[0] 未就绪');
  if (!ctx.departmentIds[0]) throw new Error('前置缺失：ctx.departmentIds[0] 未就绪');
  const marker = genCode('P13');
  const created = await apiCall<PurchaseOrderLite>(page, 'POST', '/purchase/orders', {
    supplier_id: ctx.supplierId,
    order_date: new Date().toISOString().slice(0, 10),
    warehouse_id: ctx.warehouseIds[0],
    department_id: ctx.departmentIds[0],
    items: [{ material_id: ctx.productIds[0], quantity_ordered: qtyOrdered, unit_price: '15.00' }],
    notes: `E2E-P13-${marker}`,
  });
  const id = created.data?.id;
  const order_no = created.data?.order_no;
  if (!id || !order_no)
    throw new Error(`采购订单创建未返回 id/order_no：${JSON.stringify(created)}`);
  CREATED_ORDER_IDS.push(id);
  await apiCall(page, 'POST', `/purchase/orders/${id}/submit`);
  await apiCall(page, 'POST', `/purchase/orders/${id}/approve`);
  const after = await apiCallRaw<PurchaseOrderLite>(page, 'GET', `/purchase/orders/${id}`);
  if (after.status !== 'APPROVED') throw new Error(`approve 后应为 APPROVED，实际 ${after.status}`);
  return { id, order_no, status: 'APPROVED' };
}

/** API 造一张本例专属、带全四维（色号/缸号/批次）的 DRAFT 入库单（引用订单，不确认）。 */
async function seedFourDimDraftReceipt(
  page: Page,
  po: PurchaseOrderLite,
  dims: { colorNo: string; dyeLotNo: string; batchNo: string; qty: string }
): Promise<PurchaseReceiptLite> {
  const ctx = getCtx();
  const productId = ctx.productIds[0];
  const prod = await apiCallRaw<{ code?: string; name?: string; unit?: string }>(
    page,
    'GET',
    `/products/${productId}`
  );
  const created = await apiCall<PurchaseReceiptLite>(page, 'POST', '/purchase/receipts', {
    supplier_id: ctx.supplierId,
    order_id: po.id,
    receipt_date: new Date().toISOString().slice(0, 10),
    warehouse_id: ctx.warehouseIds[0],
    department_id: ctx.departmentIds[0],
    notes: `E2E-P13-RCV-${dims.batchNo}`,
    items: [
      {
        line_no: 1,
        material_id: productId,
        material_code: prod.code,
        material_name: prod.name,
        batch_no: dims.batchNo,
        color_code: dims.colorNo,
        lot_no: dims.dyeLotNo,
        grade: '一等品',
        quantity: dims.qty,
        quantity_alt: dims.qty,
        unit_master: prod.unit ?? '米',
        unit_price: '15.00',
      },
    ],
  });
  const id = created.data?.id;
  const receipt_no = created.data?.receipt_no;
  if (!id || !receipt_no)
    throw new Error(`入库单创建未返回 id/receipt_no：${JSON.stringify(created)}`);
  CREATED_RECEIPT_IDS.push(id);
  const draft = await apiCallRaw<PurchaseReceiptLite>(page, 'GET', `/purchase/receipts/${id}`);
  expect(draft.receipt_status, '新建入库单应为 DRAFT').toBe('DRAFT');
  return { id, order_id: po.id, receipt_no, receipt_status: 'DRAFT', total_quantity: dims.qty };
}

/** 按 product+color+dye_lot+batch+warehouse 精确回读四维库存行；无行返回 null（返回 null 而非 {} 防假绿）。 */
async function readStockFourDim(
  page: Page,
  productId: number,
  dims: { colorNo?: string; dyeLotNo?: string; batchNo: string; warehouseId: number }
): Promise<StockRowLite | null> {
  let path = `/inventory/stock?product_id=${productId}&batch_no=${encodeURIComponent(
    dims.batchNo
  )}&warehouse_id=${dims.warehouseId}&page=1&page_size=50`;
  if (dims.colorNo) path += `&color_no=${encodeURIComponent(dims.colorNo)}`;
  if (dims.dyeLotNo) path += `&dye_lot_no=${encodeURIComponent(dims.dyeLotNo)}`;
  const res = await apiCallRaw<{ items?: unknown }>(page, 'GET', path);
  const items = pickListArray<StockRowLite>(res, 'items', `四维库存回读 ${path}`);
  return items.find(r => r.batch_no === dims.batchNo) ?? null;
}

/** 筛选入库单列表到本例 receipt_no 行并返回该行定位器（唯一单号收敛，杜绝共享行）。 */
async function locateReceiptRow(page: Page, receiptNo: string) {
  await page.goto('/purchase-receipt');
  await page.getByPlaceholder('入库单号').fill(receiptNo);
  await page.getByRole('button', { name: '查询', exact: true }).click();
  const row = page.getByRole('row').filter({ hasText: receiptNo }).first();
  await expect(row, `按入库单号 ${receiptNo} 应筛出本例行`).toBeVisible({ timeout: 30000 });
  return row;
}

test.describe('13 UI 采购收货确认 → 库存四维回写', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
    await ensureTestEntities(page);
  });

  test('13-01 UI 审核确认带全四维入库单 → GET /inventory/stock 回读 quantity_on_hand 增加 + 订单进度联动', async ({
    page,
  }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    const tag = genCode('P13A');
    const dims = { colorNo: `C-${tag}`, dyeLotNo: `L-${tag}`, batchNo: `B-${tag}`, qty: '30' };

    // 回写前基线：该四维组合应无库存行（新维度，先证基线为 0，非事后无法归因）
    expect(
      await readStockFourDim(page, productId, { ...dims, warehouseId }),
      '确认前该四维组合不应已有库存行'
    ).toBeNull();

    const po = await seedApprovedPO(page, '50');
    const receipt = await seedFourDimDraftReceipt(page, po, dims);

    // UI 确认入库：/purchase-receipt 行「审核」→ ElMessageBox 确认 → 成功提示
    const row = await locateReceiptRow(page, receipt.receipt_no);
    await row.getByRole('button', { name: '审核', exact: true }).click();
    await page.locator('.el-message-box__btns .el-button--primary').click();
    await expect(page.locator('.el-message--success').filter({ hasText: '审核成功' })).toBeVisible({
      timeout: 30000,
    });

    // 回读①：入库单转 COMPLETED（写入方原值大写）
    const afterReceipt = await apiCallRaw<PurchaseReceiptLite>(
      page,
      'GET',
      `/purchase/receipts/${receipt.id}`
    );
    expect(afterReceipt.receipt_status, '确认后入库单应为 COMPLETED').toBe('COMPLETED');

    // 回读②（核心）：按 款号+色号+缸号+批次 精确取库存行，quantity_on_hand 增加至收货量 30
    const stock = await readStockFourDim(page, productId, { ...dims, warehouseId });
    expect(stock, '确认后应能按四维精确回读到库存行').toBeTruthy();
    expect(Number(stock!.quantity_on_hand), '四维行 quantity_on_hand 应等于收货量 30').toBe(30);
    expect(Number(stock!.quantity_available), '可用量应同步 30').toBe(30);
    expect(stock!.color_no, '库存行色号应为入库录入色号').toBe(dims.colorNo);
    expect(stock!.dye_lot_no, '库存行缸号应为入库录入缸号').toBe(dims.dyeLotNo);
    expect(stock!.product_id, '库存行产品应为订单产品').toBe(productId);

    // 回读③：订单进度联动 —— 行 received_quantity 增加至 30，订单状态离开 APPROVED（部分/完成收货）
    const orderItems = pickListArray<{ product_id: number; received_quantity: number | string }>(
      await apiCallRaw<unknown>(page, 'GET', `/purchase/orders/${po.id}/items`),
      'bare',
      '订单明细回读'
    );
    const line = orderItems.find(i => i.product_id === productId);
    expect(line, '订单明细应含本产品行').toBeTruthy();
    expect(Number(line!.received_quantity), '订单行已收量应累加至 30').toBe(30);
    const afterPo = await apiCallRaw<PurchaseOrderLite>(page, 'GET', `/purchase/orders/${po.id}`);
    expect(
      ['PARTIAL_RECEIVED', 'COMPLETED'],
      `收货确认后订单状态应推进为部分/完成收货（实际 ${afterPo.status}）`
    ).toContain(afterPo.status);
  });

  test('13-02 纯 UI 收货对话框（批次维度）收货+确认 → 回读 quantity_on_hand 增加', async ({
    page,
  }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    const batchNo = `B-${genCode('P13B')}`;

    const po = await seedApprovedPO(page, '20');

    // UI 收货：/purchase 列表按唯一订单号筛本例行 → 点「收货」→ 选仓库/填数量/批次 → 确定收货
    await page.goto('/purchase');
    await page.getByPlaceholder('订单号/供应商名').fill(po.order_no);
    await page.getByRole('button', { name: '查询', exact: true }).click();
    const poRow = page.getByRole('row').filter({ hasText: po.order_no }).first();
    await expect(poRow, `订单列表应筛出 ${po.order_no}`).toBeVisible({ timeout: 30000 });
    await poRow.getByRole('button', { name: '收货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '采购收货' });
    await expect(dialog).toBeVisible();
    await pickSelectIn(dialog, page, '仓库');
    await dialog.getByRole('spinbutton').first().fill('5');
    await dialog.locator('input[placeholder="收货批次号"]').first().fill(batchNo);
    await dialog.getByRole('button', { name: '确定收货' }).click();
    await expect(page.getByText('收货成功')).toBeVisible({ timeout: 30000 });

    // 取回本单刚生成的 DRAFT 入库单（按 order_id 筛，非共享行）
    const list = pickListArray<PurchaseReceiptLite>(
      await apiCallRaw<unknown>(
        page,
        'GET',
        `/purchase/receipts?order_id=${po.id}&page=1&page_size=10`
      ),
      'items',
      '按单入库单列表'
    );
    const draft = list.find(r => r.order_id === po.id);
    expect(draft, `订单 ${po.order_no} 收货应生成 DRAFT 入库单`).toBeTruthy();
    expect(draft!.receipt_status, '收货登记只建 DRAFT').toBe('DRAFT');
    CREATED_RECEIPT_IDS.push(draft!.id);

    // 确认入库（同 confirm 端点；UI 收货对话框仅采批次，色号/缸号合法留空——白坯口径）
    await apiCall(page, 'POST', `/purchase/receipts/${draft!.id}/confirm`);

    // 回读：按 款号+批次 取行，quantity_on_hand 增加至 5（收货量）
    const stock = await readStockFourDim(page, productId, { batchNo, warehouseId });
    expect(stock, '确认后应按批次回读到库存行').toBeTruthy();
    expect(Number(stock!.quantity_on_hand), 'UI 收货确认后 quantity_on_hand 应增加至 5').toBe(5);
    expect(Number(stock!.quantity_available), '可用量应增加至 5').toBe(5);
  });

  test('13-02b 重复确认入库被业务拒且 quantity_on_hand 不被二次累加', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    const tag = genCode('P13C');
    const dims = { colorNo: `C-${tag}`, dyeLotNo: `L-${tag}`, batchNo: `B-${tag}`, qty: '12' };

    const po = await seedApprovedPO(page, '30');
    const receipt = await seedFourDimDraftReceipt(page, po, dims);

    // 首次确认成功
    await apiCall(page, 'POST', `/purchase/receipts/${receipt.id}/confirm`);
    const first = await readStockFourDim(page, productId, { ...dims, warehouseId });
    expect(first, '首次确认应回读到库存行').toBeTruthy();
    expect(Number(first!.quantity_on_hand), '首次确认后 on_hand 应为 12').toBe(12);

    // 二次确认应被业务拒（confirm 前 lock + 状态门只放 DRAFT）
    const fail = await apiCallExpectFail(page, 'POST', `/purchase/receipts/${receipt.id}/confirm`);
    expect(fail.status, `二次确认应被拒为 4xx，实际 ${fail.status}`).toBeGreaterThanOrEqual(400);
    expect(fail.status, '二次确认不应为 5xx').toBeLessThan(500);

    // 回读：on_hand 仍为 12（未被二次累加），证明幂等防御生效
    const after = await readStockFourDim(page, productId, { ...dims, warehouseId });
    expect(
      Number(after!.quantity_on_hand),
      `重复确认后 on_hand 不得翻倍，应仍为 12（实际 ${after!.quantity_on_hand}）`
    ).toBe(12);
  });
});
