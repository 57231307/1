// 采购 E2E 套件 — 14 采购退货过账 → 库存扣减回退 + 订单进度（联动现状锁）
//
// 补齐审计缺口「采购退货仅 toast、未回读库存回退与订单进度联动」(P1)。
// 本例「退货过账（approve）→ GET /inventory/stock 精确回读四维行 quantity_on_hand 减少」，禁只断 toast。
//
// 后端事实来源（写入方为准，逐字段核对）：
// - 退货过账：POST /purchase/returns/{id}/approve（purchase_return_service.rs::approve_return）：
//     ① 状态 SUBMITTED → APPROVED；
//     ② 事务内扣减库存 deduct_stock_for_return_items：按退货明细真实携带的四维
//        (产品+色号+缸号+批次，StockDimKey；仓库=退货单 warehouse_id) **精确命中**库存行
//        （purchase_return_service.rs::return_item_stock_key/stock_row_key，缸号两侧 trim 归一空串），
//        四维命中不到库存行 → 400 BUSINESS_ERROR 显式拒绝（不兜底、不任选行）；同键多行（仅等级
//        不同，退货明细不携等级）→ 歧义显式拒绝。命中后 update_stock_quantity_with_optimistic_lock_txn
//        把该行 quantity_meters 设为 (原 - 退货量)，同步刷新 quantity_on_hand/quantity_available，
//        即退货使在库量按维度真实回退。
//     ③ 同一事务内 writeback_source_order_received_quantity 按产品归集退货量、在该订单同产品
//        明细行按 line_no 升序逐行把 received_quantity 减回（min 保下限 0），并重算 PO 状态
//        （全收 COMPLETED / 有收 PARTIAL_RECEIVED / 归零回 APPROVED）。
// ⚠ CI E11 判责更新：旧版本文件头注「仅按产品+仓库定位第一行、approve 不回写订单进度」
//     是旧契约描述，现源码两条都已推翻——退货明细必须携带与入库一致的色号/缸号/批次
//     （CreateReturnItemRequest.color_no/dye_lot_no/batch_no），扣减才按四维真实发生；
//     且退货过账**会**回退来源 PO received_quantity 并重算状态。本例据此如实断言新契约，
//     维度值取自本用例自建入库后回读的真实库存行（不硬编码、不塞 TEST 假值）。
//     四维口径核对：本退货链路后端定位键 = 产品+色号+缸号+批次（三维追溯 + 产品），
//     不含匹号——出库销售侧的「缸号/色号/批次/匹号」四维强制不适用于采购退货，未擅自扩维。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  tryCleanup,
  genCode,
  seedInspectionPass,
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
  // GET /products/{id} 返回 product::Model（name/code/unit 均 NOT NULL 真实键），直取真键，
  // 禁 `?? 默认值` 掩盖缺键。
  const prod = await apiCallRaw<{ code: string; name: string; unit: string }>(
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
        unit_master: prod.unit,
        unit_price: '12.00',
      },
    ],
  });
  const rcvId = rcv.data?.id;
  if (!rcvId) throw new Error(`建入库单未返回 id：${JSON.stringify(rcv)}`);
  CREATED_RECEIPT_IDS.push(rcvId);
  // 「质检合格方可入库」门控前置（本链确实需要已入库库存做退货回退，故必须真确认入库；
  // 见 helpers.seedInspectionPass —— 未质检直接 confirm 会被 400 拒）
  await seedInspectionPass(page, {
    receiptId: rcvId,
    supplierId: ctx.supplierId!,
    context: `14 入库单#${rcvId}`,
  });
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

  test('14-01 退货过账后回读 quantity_on_hand 四维回退 + 状态机联动 + 来源订单进度回写', async ({
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

    // 四维定位契约（return_item_stock_key vs stock_row_key）下，同产品同仓存在其它批次行
    // 不构成歧义——扣减按 (产品+色号+缸号+批次) 唯一命中，仅当同四维键多行（差异只在等级）
    // 才触发后端歧义拒绝；本用例维度值带唯一 tag，天然排除该形态。
    // 旧版此处 test.skip(行数>1) 的前提是「按产品+仓库取第一行」的旧契约，已被源码推翻，
    // 条件跳过会制造假绿盲区，故删除 skip，如实走四维扣减。

    // 建退货单（draft）→ 加明细（退货 20，**带与入库一致的真实三维**）→ submit → approve（过账）
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

    // CI E11 修复：退货明细补传 color_no/dye_lot_no/batch_no——CreateReturnItemRequest
    // 本就支持这三键（purchase_return_service.rs::CreateReturnItemRequest，落库 NOT NULL DEFAULT ''）。
    // 取值全部来自本用例自建入库后 GET /inventory/stock 回读的那一行真实维度（before 行），
    // 不硬编码假值、不塞 TEST；缺维度时后端按空串四维匹配不到入库行，
    // approve 会 400 BUSINESS_ERROR fail-closed（正是 的红因）。
    const returnDims = {
      colorNo: before.color_no,
      dyeLotNo: before.dye_lot_no ?? '',
      batchNo: before.batch_no,
    };
    await apiCall(page, 'POST', `/purchase/returns/${returnId}/items`, {
      line_no: 1,
      material_id: productId,
      quantity_returned: '20',
      unit_price: '12.00',
      color_no: returnDims.colorNo,
      dye_lot_no: returnDims.dyeLotNo,
      batch_no: returnDims.batchNo,
    });

    const draftDetail = await apiCallRaw<{ return_status: string }>(
      page,
      'GET',
      `/purchase/returns/${returnId}`
    );
    expect(draftDetail.return_status, '新建退货单应为 draft').toBe('draft');

    // 写后必回读（过账前）：GET items 为裸数组信封（list_purchase_return_items 返回
    // Vec<PurchaseReturnItemDto>），明细落库的三维必须与入库库存行逐字段一致——
    // 这是四维扣减将要真实发生的前提证据，而非只信 POST 返回。
    const returnItems = pickListArray<{
      line_no: number;
      material_id: number;
      color_no: string;
      dye_lot_no: string;
      batch_no: string;
      quantity_returned: number | string;
    }>(
      await apiCallRaw<unknown>(page, 'GET', `/purchase/returns/${returnId}/items`),
      'bare',
      '退货明细回读'
    );
    const retLine = returnItems.find(i => i.line_no === 1);
    expect(retLine, '退货明细回读应含 line_no=1 行').toBeTruthy();
    expect(retLine!.material_id, '明细产品应为入库产品').toBe(productId);
    expect(retLine!.color_no, '明细色号应与入库库存行一致').toBe(returnDims.colorNo);
    expect(retLine!.dye_lot_no, '明细缸号应与入库库存行一致').toBe(returnDims.dyeLotNo);
    expect(retLine!.batch_no, '明细批次应与入库库存行一致').toBe(returnDims.batchNo);
    expect(Number(retLine!.quantity_returned), '明细退货量应落库为 20').toBe(20);

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
    // 四维回填：证明扣减命中的仍是当初入库那一行（退货只回退在库量，维度身份 product+batch+color+lot 不变）
    expect(after!.product_id, '四维回填：命中行产品应为退货产品').toBe(productId);
    expect(after!.batch_no, '四维回填：命中行批次应为入库批次').toBe(dims.batchNo);
    expect(after!.color_no, '四维回填：命中行色号应为入库色号').toBe(dims.colorNo);
    expect(after!.dye_lot_no, '四维回填：命中行缸号应为入库缸号').toBe(dims.dyeLotNo);
    expect(
      Number(after!.quantity_on_hand),
      `退货过账后 on_hand 应回退 60→40（实际 ${after!.quantity_on_hand}）`
    ).toBe(40);
    expect(
      Number(after!.quantity_available),
      `退货过账后 available 应回退 60→40（实际 ${after!.quantity_available}）`
    ).toBe(40);

    // 订单进度回写（新契约，非"现状锁"）：approve_return 与库存扣减同一事务内执行
    // writeback_source_order_received_quantity——按产品归集退货量、同产品订单行按 line_no
    // 升序逐行把 received_quantity 减回（60 - 20 = 40），并重算 PO 状态（40 < 订购 100 → PARTIAL_RECEIVED）。
    // 出参键以 PurchaseOrderItemDto 为准（services/po/order.rs:65 product_id 经 serde 改名 material_id），
    // 按真实键 material_id 定位本品行。
    const orderItems = pickListArray<{ material_id: number; received_quantity: number | string }>(
      await apiCallRaw<unknown>(page, 'GET', `/purchase/orders/${po.id}/items`),
      'bare',
      '订单明细回读'
    );
    const line = orderItems.find(i => i.material_id === productId);
    expect(
      Number(line!.received_quantity),
      `退货过账应回写来源订单 received_quantity 60→40（实际 ${line!.received_quantity}）`
    ).toBe(40);
    const poAfter = await apiCallRaw<PurchaseOrderLite>(page, 'GET', `/purchase/orders/${po.id}`);
    expect(
      poAfter.status,
      `退货回写后 40<订购100 且 >0，PO 应按 determine_order_status_after_return 判 PARTIAL_RECEIVED（实际 ${poAfter.status}）`
    ).toBe('PARTIAL_RECEIVED');
  });
});
