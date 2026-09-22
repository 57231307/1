import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  getCtx,
  verifyStockFourDim,
  verifyAuditLog,
  ensureTestEntities,
  ensureStockInWarehouse,
} from './helpers';

test.describe('库存调拨完整流程', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('调拨：创建→审批→出库→在途→入库→双仓库库存变化验证', async ({ page }) => {
    const ctx = getCtx();
    expect(ctx.warehouseIds.length).toBeGreaterThanOrEqual(2);

    const productId = ctx.productIds[0] || 1;

    // 调出仓库必须有库存：以实际库存行为准（修 ctx.warehouseIds 漂移），
    // 调入仓库用 ctx 的另一个仓库
    const stockRow = await ensureStockInWarehouse(
      page,
      productId,
      ctx.warehouseIds[0],
      ctx.colorNos[0]
    );
    const fromWarehouseId = Number(stockRow.warehouse_id) || ctx.warehouseIds[0];
    const toWarehouseId =
      ctx.warehouseIds.find(id => id !== fromWarehouseId) || ctx.warehouseIds[1];

    // 调拨前：来源仓必须已有该产品+色号的库存行
    // （原实现读不存在的 quantity/available_qty 字段算出 qtyBefore，且该值后续从未参与断言）
    const stockBefore = await verifyStockFourDim(page, productId, ctx.colorNos[0], undefined, {
      warehouseId: fromWarehouseId,
    });
    expect(
      stockBefore,
      `调拨前来源仓 ${fromWarehouseId} 应存在产品 ${productId} / 色号 ${ctx.colorNos[0]} 的库存行`
    ).toBeTruthy();

    const transferDyeLot = ctx.dyeLotNo || `E2E-DL-${Date.now().toString().slice(-6)}`;
    const transferBatchNo = 'E2E-BATCH-12';
    const transferQty = 5;

    // 后端 CreateInventoryTransferRequest 真实字段
    const transferData = {
      from_warehouse_id: fromWarehouseId,
      to_warehouse_id: toWarehouseId,
      transfer_date: new Date().toISOString(),
      notes: 'E2E 调拨测试',
      items: [
        {
          product_id: productId,
          quantity: String(transferQty),
          color_no: ctx.colorNos[0],
          dye_lot_no: transferDyeLot,
          batch_no: transferBatchNo,
        },
      ],
    };

    const result = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/inventory/transfers',
      transferData
    );
    const transferId = result.data?.id;
    expect(
      transferId,
      `调拨建单应返回 data.id，实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    // 验证初始状态：build_transfer_active_model 默认写 PENDING
    // （inventory_move.rs:210-212；调拨词表只有 pending/approved/rejected/shipped/completed，
    //  无 draft——见 purchase_inventory.rs:63-78）
    const created = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/inventory/transfers/${transferId}`
    );
    expect(created.status.toLowerCase()).toBe('pending');

    // 审批调拨
    // ApproveTransferRequest { approved: bool, notes? } 必填
    await apiCall(page, 'POST', `/inventory/transfers/${transferId}/approve`, { approved: true });
    const approved = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/inventory/transfers/${transferId}`
    );
    expect(approved.status.toLowerCase()).toBe('approved');

    // 出库：update_transfer_to_shipped 写 "shipped"（batch.rs:457，词表无 in_transit）
    await apiCall(page, 'POST', `/inventory/transfers/${transferId}/ship`);
    const shipped = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/inventory/transfers/${transferId}`
    );
    expect(shipped.status.toLowerCase()).toBe('shipped');

    // 验证非法操作：在途状态不能再次出库
    const illegalShip = await apiCallExpectFail(
      page,
      'POST',
      `/inventory/transfers/${transferId}/ship`
    );
    expect(illegalShip.status).toBeGreaterThanOrEqual(400);

    // 入库
    await apiCall(page, 'POST', `/inventory/transfers/${transferId}/receive`);
    const received = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/inventory/transfers/${transferId}`
    );
    expect(received.status.toLowerCase()).toBe('completed');

    // 调拨入库必须落到目标仓的四维库存行（产品+色号+缸号+批次+目标仓）
    const stockTo = await verifyStockFourDim(page, productId, ctx.colorNos[0], transferDyeLot, {
      batchNo: transferBatchNo,
      warehouseId: toWarehouseId,
    });
    expect(
      stockTo,
      `调拨完成后目标仓 ${toWarehouseId} 应存在缸号 ${transferDyeLot} / 批次 ${transferBatchNo} 的库存行`
    ).toBeTruthy();
    expect(
      Number(stockTo!.quantity_on_hand),
      `目标仓在库量应不少于调拨数量 ${transferQty}（实际 ${stockTo!.quantity_on_hand}）`
    ).toBeGreaterThanOrEqual(transferQty);

    // 验证审计日志
    const auditLogged = await verifyAuditLog(page, 'UPDATE', 'inventory');
    expect(auditLogged).toBe(true);

    // UI 验证：访问调拨列表页
    await page.goto('/inventory-transfer');
    await page.waitForTimeout(2000);
    const tableVisible = await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first()
      .isVisible();
    expect(tableVisible).toBe(true);
  });

  test('调拨状态机非法转换验证', async ({ page }) => {
    const ctx = getCtx();

    const transferData = {
      from_warehouse_id: ctx.warehouseIds[0],
      to_warehouse_id: ctx.warehouseIds[1] || ctx.warehouseIds[0],
      transfer_date: new Date().toISOString(),
      items: [
        {
          product_id: ctx.productIds[0],
          quantity: '1',
          color_no: ctx.colorNos[0],
          // 染色布建单为 fail-closed 校验：color_no 非白坯时必须同时提供缸号与批号，
          // 否则 create_transfer_items_and_compute_total 直接 400
          // （inventory_move.rs:247-258）。缸号取前置数据真实值，批号为必填的非空追溯串。
          dye_lot_no: ctx.dyeLotNo,
          batch_no: 'E2E-BATCH-12NEG',
        },
      ],
    };

    const result = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/inventory/transfers',
      transferData
    );
    const transferId = result.data?.id;
    expect(
      transferId,
      `负例前置：调拨建单应返回 data.id，否则 URL 退化为 /undefined 会让非法转换断言假绿；实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    // pending 状态直接入库应被拒
    const illegalReceive = await apiCallExpectFail(
      page,
      'POST',
      `/inventory/transfers/${transferId}/receive`
    );
    expect(illegalReceive.status).toBeGreaterThanOrEqual(400);

    // pending 状态直接出库应被拒
    const illegalShip = await apiCallExpectFail(
      page,
      'POST',
      `/inventory/transfers/${transferId}/ship`
    );
    expect(illegalShip.status).toBeGreaterThanOrEqual(400);
  });

  // ============================================================
  // 白坯布正常出库链路（真实入库→调拨出库）：
  // 业务口径：白坯布 = 没有颜色的布（color_no 为空），免缸号、批次必填。
  // 入库：走真实采购收货，明细不带 color_code / lot_no（→ inventory_stocks.color_no=''、
  //       dye_lot_no IS NULL），带批次；出库：调拨按 产品+仓库+批次+无色号 精确扣该白坯行。
  // 断言：出库后白坯库存行按批次精确减少（扣减量=出库量），出库明细色号为空、缸号为空。
  // ============================================================
  test('白坯布：真实入库后调拨出库按批次精确扣减（免缸号）', async ({ page }) => {
    const ctx = getCtx();
    expect(ctx.warehouseIds.length, '需要至少两个仓库（调出/调入）').toBeGreaterThanOrEqual(2);
    const productId = ctx.productIds[0];
    const supplierId = ctx.supplierId;
    const departmentId = ctx.departmentIds[0];
    const warehouseId = ctx.warehouseIds[0];
    expect(productId, '前置产品缺失').toBeTruthy();
    expect(supplierId, '前置供应商缺失').toBeTruthy();

    const whiteBatch = genCode('BN-WHITE');
    const inboundQty = 30;
    const outboundQty = 12;

    // ---- 1. 建采购单（白坯产品）并审批 ----
    const po = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
      supplier_id: supplierId,
      warehouse_id: warehouseId,
      department_id: departmentId,
      order_date: new Date().toISOString().slice(0, 10),
      items: [{ material_id: productId, quantity_ordered: String(inboundQty), unit_price: '1' }],
    });
    const poId = po.data?.id;
    expect(poId, `采购单创建应返回 id，实际：${JSON.stringify(po).slice(0, 200)}`).toBeTruthy();
    await apiCall(page, 'POST', `/purchase/orders/${poId}/submit`);
    await apiCall(page, 'POST', `/purchase/orders/${poId}/approve`);
    console.warn(`[白坯出库] 采购单已审批 poId=${poId}`);

    // ---- 2. 建白坯入库单（不带 color_code / lot_no）并确认入库 ----
    const receipt = await apiCall<{ id?: number }>(page, 'POST', '/purchase/receipts', {
      order_id: poId,
      supplier_id: supplierId,
      warehouse_id: warehouseId,
      receipt_date: new Date().toISOString().slice(0, 10),
      items: [
        {
          line_no: 1,
          material_id: productId,
          material_name: 'E2E 白坯布',
          unit_master: 'm',
          quantity: String(inboundQty),
          quantity_alt: '0',
          batch_no: whiteBatch,
          // 白坯：故意不提供 color_code / lot_no
        },
      ],
    });
    const receiptId = receipt.data?.id;
    expect(receiptId, `白坯入库单创建失败：${JSON.stringify(receipt).slice(0, 200)}`).toBeTruthy();
    await apiCall(page, 'POST', `/purchase/receipts/${receiptId}/confirm`);
    await expect
      .poll(
        async () => {
          const r = await apiCallRaw<{ receipt_status?: string; status?: string }>(
            page,
            'GET',
            `/purchase/receipts/${receiptId}`
          );
          return (r.receipt_status || r.status || '').toUpperCase();
        },
        { message: `白坯入库单 ${receiptId} 应进入 COMPLETED 终态` }
      )
      .toBe('COMPLETED');

    // ---- 3. 定位入库后的白坯库存行（color_no 空 + dye_lot_no 空 + 指定批次）----
    const readWhiteStock = async () => {
      const res = await apiCallRaw<{ items?: Array<Record<string, unknown>> }>(
        page,
        'GET',
        `/inventory/stock?product_id=${productId}&warehouse_id=${warehouseId}&batch_no=${whiteBatch}&page=1&page_size=50`
      );
      const rows = res.items ?? [];
      console.warn(
        `[白坯出库] 批次 ${whiteBatch} 命中库存行 ${rows.length} 条: ${rows
          .map(
            r =>
              `id=${r.id} color=${JSON.stringify(r.color_no)} dye=${JSON.stringify(r.dye_lot_no)} avail=${r.quantity_available}`
          )
          .join(' | ')}`
      );
      return rows.find(r => !r.color_no && !r.dye_lot_no) ?? null;
    };
    const whiteBefore = await readWhiteStock();
    expect(
      whiteBefore,
      `白坯入库后应在仓库 ${warehouseId} 生成 color_no 空 + dye_lot_no 空 + 批次 ${whiteBatch} 的库存行`
    ).toBeTruthy();
    expect(
      Number(whiteBefore!.quantity_available),
      `白坯入库后可用量应为 ${inboundQty}`
    ).toBeCloseTo(inboundQty, 2);

    // ---- 4. 建白坯调拨单（色号空、无缸号、带批次）→ 审批 → 出库 ----
    const toWarehouseId = ctx.warehouseIds.find(id => id !== warehouseId) || ctx.warehouseIds[1];
    const transfer = await apiCall<{ id?: number }>(page, 'POST', '/inventory/transfers', {
      from_warehouse_id: warehouseId,
      to_warehouse_id: toWarehouseId,
      transfer_date: new Date().toISOString(),
      notes: 'E2E 白坯出库',
      items: [
        {
          product_id: productId,
          quantity: String(outboundQty),
          color_no: '',
          // 白坯免缸号：不提供 dye_lot_no
          batch_no: whiteBatch,
        },
      ],
    });
    const transferId = transfer.data?.id;
    expect(
      transferId,
      `白坯调拨建单应成功，实际：${JSON.stringify(transfer).slice(0, 200)}`
    ).toBeTruthy();
    console.warn(`[白坯出库] 白坯调拨建单成功 transferId=${transferId}`);

    // 出库明细色号为空、缸号为空（如实回显白坯维度）
    const detail = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/inventory/transfers/${transferId}`
    );
    expect(detail.items.length, '白坯调拨明细应可读回').toBeGreaterThan(0);
    expect(String(detail.items[0].color_no ?? ''), '白坯出库明细色号应为空').toBe('');
    expect(
      detail.items[0].dye_lot_no === null ||
        detail.items[0].dye_lot_no === undefined ||
        detail.items[0].dye_lot_no === '',
      `白坯出库明细缸号应为空（实际 ${JSON.stringify(detail.items[0].dye_lot_no)}）`
    ).toBe(true);

    await apiCall(page, 'POST', `/inventory/transfers/${transferId}/approve`, { approved: true });
    await apiCall(page, 'POST', `/inventory/transfers/${transferId}/ship`);
    const shipped = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/inventory/transfers/${transferId}`
    );
    expect(shipped.status.toLowerCase(), '白坯调拨出库后状态应为 shipped').toBe('shipped');

    // ---- 5. 断言白坯库存行按批次精确减少（扣减量 = 出库量），未被跨缸回退动到其他行 ----
    const whiteAfter = await readWhiteStock();
    expect(whiteAfter, '出库后白坯库存行应仍存在（按批次定位）').toBeTruthy();
    expect(
      Number(whiteAfter!.quantity_available),
      `白坯库存行可用量应精确减少 ${outboundQty}（${inboundQty} → ${inboundQty - outboundQty}），实际 ${whiteAfter!.quantity_available}`
    ).toBeCloseTo(inboundQty - outboundQty, 2);
  });

  // ============================================================
  // 对照：染色布（色号非空）缺缸号建单必须被拒（4xx），证明白坯免缸号是"布种差异"而非
  // 放宽所有校验——与白坯出库测试同一判定源（fabric_class）双向锁死。
  // ============================================================
  test('染色布缺缸号：调拨建单仍被拒（白坯免缸号的对照）', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const rejected = await apiCallExpectFail(page, 'POST', '/inventory/transfers', {
      from_warehouse_id: ctx.warehouseIds[0],
      to_warehouse_id: ctx.warehouseIds[1] || ctx.warehouseIds[0],
      transfer_date: new Date().toISOString(),
      items: [
        {
          product_id: productId,
          quantity: '1',
          color_no: ctx.colorNos[0], // 非空色号 = 染色布
          // 故意不提供 dye_lot_no
          batch_no: genCode('BN-DYED-NEG'),
        },
      ],
    });
    console.warn(
      `[染色对照] 染色布缺缸号建单 status=${rejected.status} message=${rejected.message}`
    );
    expect(rejected.status, '染色布缺缸号建单应被拒（>=400）').toBeGreaterThanOrEqual(400);
    expect(rejected.status, '应为客户端校验错误而非 5xx').toBeLessThan(500);
  });
});
