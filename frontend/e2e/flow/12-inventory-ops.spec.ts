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
});
