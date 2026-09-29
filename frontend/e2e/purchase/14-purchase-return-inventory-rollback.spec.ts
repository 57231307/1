// 采购 E2E 套件 — 14 采购退货过账 → 库存扣减回退 + 订单进度（联动现状锁）
//
// 补齐审计缺口「采购退货仅 toast、未回读库存回退与订单进度联动」(P1)。
// 本例「退货过账（approve）→ GET /inventory/stock 精确回读四维行 quantity_on_hand 减少」，禁只断 toast。
//
// 后端事实来源（写入方为准，逐字段核对）：
// - 退货过账：POST /purchase/returns/{id}/approve（purchase_return_service.rs::approve_return）：
//     ① 状态 SUBMITTED → APPROVED；
//     ② 事务内扣减库存 deduct_stock_for_return_items → update_stock_quantity_with_optimistic_lock_txn
//        把命中库存行的 quantity_meters 设为 (原 - 退货量)，同步刷新 quantity_on_hand/quantity_available
//        （inventory_stock_txn.rs:28-42），即退货使在库量回退。
//   ⚠ 关键契约事实：退货扣减仅按「退货单 warehouse_id + 明细 product_id」定位库存行
//     （purchase_return_service.rs:365-370，stock_map 以 product_id 为键、取该产品在该仓的第一行），
//     并非按四维精确定位；且 approve_return 全程未回写采购订单 received_quantity/状态。
//     故本例：
//       (a) 用「专用产品 ctx.productIds[1]」并在目标仓仅保留唯一一行四维库存，使「产品+仓库」定位
//           恰等价于四维回读，扣减可精确归因（多于 1 行则显式 skip，不放宽）；
//       (b) 如实断言当前契约：退货回读 quantity_on_hand 减少；采购订单 received_quantity **不因退货而回退**
//           ——此行为后端未实现「退货回退订单进度」的客观现状，本例锁定该现状并据实上报为覆盖/功能缺口，
//           绝不虚构「received_quantity 应减少」的断言去制造假绿或假红。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  tryCleanup,
  genCode,
} from '../flow/helpers';
import { pickListArray } from '../flow/ui-helpers';

interface PurchaseOrderLite {
  id: number;
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
}

const CREATED_ORDER_IDS: number[] = [];
const CREATED_RECEIPT_IDS: number[] = [];
const CREATED_RETURN_IDS: number[] = [];
test.afterEach(async ({ page }) => {
  while (CREATED_RETURN_IDS.length) {
    const id = CREATED_RETURN_IDS.pop();
    if (id != null) await tryCleanup(page, 'DELETE', `/purchase/returns/${id}`, `return#${id}`);
  }
  while (CREATED_RECEIPT_IDS.length) {
    const id = CREATED_RECEIPT_IDS.pop();
    if (id != null) await tryCleanup(page, 'DELETE', `/purchase/receipts/${id}`, `receipt#${id}`);
  }
  while (CREATED_ORDER_IDS.length) {
    const id = CREATED_ORDER_IDS.pop();
    if (id != null) await tryCleanup(page, 'DELETE', `/purchase/orders/${id}`, `po#${id}`);
  }
});

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

/** 为专用产品造 APPROVED 态采购订单。 */
async function seedApprovedPOForProduct(page: Page, productId: number): Promise<PurchaseOrderLite> {
  const ctx = getCtx();
  const created = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
    supplier_id: ctx.supplierId,
    order_date: today(),
    warehouse_id: ctx.warehouseIds[0],
    department_id: ctx.departmentIds[0],
    notes: `E2E-P14-${genCode('PO')}`,
    items: [{ material_id: productId, quantity_ordered: '100', unit_price: '12.00' }],
  });
  const id = created.data?.id;
  if (!id) throw new Error(`建 PO 未返回 id：${JSON.stringify(created)}`);
  CREATED_ORDER_IDS.push(id);
  await apiCall(page, 'POST', `/purchase/orders/${id}/submit`);
  await apiCall(page, 'POST', `/purchase/orders/${id}/approve`);
  const st = await apiCallRaw<PurchaseOrderLite>(page, 'GET', `/purchase/orders/${id}`);
  if (st.status !== 'APPROVED') throw new Error(`PO approve 后应为 APPROVED，实际 ${st.status}`);
  return { id, status: 'APPROVED' };
}

/** 为专用产品建全四维入库单并确认（过账入库），返回确认后的四维行快照。 */
async function seedConfirmedReceipt(
  page: Page,
  po: PurchaseOrderLite,
  productId: number,
  dims: { colorNo: string; dyeLotNo: string; batchNo: string; qty: string }
): Promise<StockRowLite> {
  const ctx = getCtx();
  const warehouseId = ctx.warehouseIds[0];
  const prod = await apiCallRaw<{ code?: string; name?: string; unit?: string }>(
    page,
    'GET',
    `/products/${productId}`
  );
  const rcv = await apiCall<{ id?: number }>(page, 'POST', '/purchase/receipts', {
    supplier_id: ctx.supplierId,
    order_id: po.id,
    receipt_date: today(),
    warehouse_id: warehouseId,
    department_id: ctx.departmentIds[0],
    notes: `E2E-P14-RCV-${dims.batchNo}`,
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
        unit_price: '12.00',
      },
    ],
  });
  const rcvId = rcv.data?.id;
  if (!rcvId) throw new Error(`建入库单未返回 id：${JSON.stringify(rcv)}`);
  CREATED_RECEIPT_IDS.push(rcvId);
  await apiCall(page, 'POST', `/purchase/receipts/${rcvId}/confirm`);
  const rows = await readStockByProduct(page, productId, warehouseId);
  const row = rows.find(r => r.batch_no === dims.batchNo);
  if (!row) throw new Error(`确认入库后按批次 ${dims.batchNo} 回读不到库存行`);
  return row;
}

async function readStockByProduct(
  page: Page,
  productId: number,
  warehouseId: number
): Promise<StockRowLite[]> {
  const res = await apiCallRaw<{ items?: unknown }>(
    page,
    'GET',
    `/inventory/stock?product_id=${productId}&warehouse_id=${warehouseId}&page=1&page_size=50`
  );
  return pickListArray<StockRowLite>(res, 'items', `库存回读 product=${productId}`);
}

test.describe('14 采购退货过账 → 库存回退', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
    await ensureTestEntities(page);
  });

  test('14-01 退货过账后回读 quantity_on_hand 回退 + 退货单状态机联动（并锁定订单进度现状）', async ({
    page,
  }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[1];
    const warehouseId = ctx.warehouseIds[0];
    if (!productId) throw new Error('前置缺失：ctx.productIds[1] 未就绪');
    if (!warehouseId) throw new Error('前置缺失：ctx.warehouseIds[0] 未就绪');
    if (!ctx.supplierId) throw new Error('前置缺失：ctx.supplierId 未就绪');

    const tag = genCode('P14');
    const dims = { colorNo: `C-${tag}`, dyeLotNo: `L-${tag}`, batchNo: `B-${tag}`, qty: '60' };

    const po = await seedApprovedPOForProduct(page, productId);
    const before = await seedConfirmedReceipt(page, po, productId, dims);
    expect(Number(before.quantity_on_hand), '入库确认后 on_hand 基线应为 60').toBe(60);

    // 隔离前提：退货按「产品+仓库」定位库存行，若该产品在目标仓多于 1 行（并发污染）则无法精确归因
    const candidates = await readStockByProduct(page, productId, warehouseId);
    test.skip(
      candidates.length > 1,
      `[并发隔离] 仓库 ${warehouseId} 下专用产品 ${productId} 库存行数=${candidates.length}（>1），` +
        `退货按产品+仓库定位库存行会命中非本例行，扣减无法精确归因——真实共享库多分片污染，非可放宽，跳过以免假绿`
    );

    // 建退货单（draft）→ 加明细（退货 20）→ submit → approve（过账，触发库存回退）
    const ret = await apiCall<{ id?: number }>(page, 'POST', '/purchase/returns', {
      supplier_id: ctx.supplierId,
      order_id: po.id,
      warehouse_id: warehouseId,
      department_id: ctx.departmentIds[0],
      return_date: today(),
      reason_type: '色差',
      reason_detail: 'E2E 退货过账库存回退用例',
      notes: `E2E-P14-RET-${tag}`,
    });
    const returnId = ret.data?.id;
    if (!returnId) throw new Error(`建退货单未返回 id：${JSON.stringify(ret)}`);
    CREATED_RETURN_IDS.push(returnId);

    await apiCall(page, 'POST', `/purchase/returns/${returnId}/items`, {
      line_no: 1,
      material_id: productId,
      quantity_returned: '20',
      unit_price: '12.00',
    });

    const draftDetail = await apiCallRaw<{ return_status: string }>(
      page,
      'GET',
      `/purchase/returns/${returnId}`
    );
    expect(draftDetail.return_status, '新建退货单应为 draft').toBe('draft');

    await apiCall(page, 'POST', `/purchase/returns/${returnId}/submit`);
    const submitted = await apiCallRaw<{ return_status: string }>(
      page,
      'GET',
      `/purchase/returns/${returnId}`
    );
    expect(submitted.return_status, '提交后应为 submitted').toBe('submitted');

    await apiCall(page, 'POST', `/purchase/returns/${returnId}/approve`);
    const approved = await apiCallRaw<{ return_status: string }>(
      page,
      'GET',
      `/purchase/returns/${returnId}`
    );
    expect(approved.return_status, '审批过账后应为 approved').toBe('approved');

    // 核心回读：按四维行回读 quantity_on_hand 回退（60 - 20 = 40）
    const after = (await readStockByProduct(page, productId, warehouseId)).find(
      r => r.batch_no === dims.batchNo
    );
    expect(after, '过账后仍应能按批次回读到库存行').toBeTruthy();
    expect(
      Number(after!.quantity_on_hand),
      `退货过账后 on_hand 应回退 60→40（实际 ${after!.quantity_on_hand}）`
    ).toBe(40);
    expect(
      Number(after!.quantity_available),
      `退货过账后 available 应回退 60→40（实际 ${after!.quantity_available}）`
    ).toBe(40);

    // 订单进度联动现状锁：后端 approve_return 不回写采购订单 received_quantity，
    // 此断言如实锁定「退货不联动回退订单进度」这一当前实现事实（已作为功能缺口上报，非放宽掩盖）。
    const orderItems = pickListArray<{ product_id: number; received_quantity: number | string }>(
      await apiCallRaw<unknown>(page, 'GET', `/purchase/orders/${po.id}/items`),
      'bare',
      '订单明细回读'
    );
    const line = orderItems.find(i => i.product_id === productId);
    expect(
      Number(line!.received_quantity),
      '当前契约：退货过账不回退订单 received_quantity，应仍为入库量 60'
    ).toBe(60);
    const poAfter = await apiCallRaw<PurchaseOrderLite>(page, 'GET', `/purchase/orders/${po.id}`);
    expect(
      ['COMPLETED', 'PARTIAL_RECEIVED'].includes(poAfter.status),
      `退货过账不改变订单收货态，应维持入库确认后的收货进度态（实际 ${poAfter.status}）`
    ).toBe(true);
  });
});
