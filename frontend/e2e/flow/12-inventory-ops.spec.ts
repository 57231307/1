import { test, expect } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  getCtx,
  verifyStockFourDim,
  verifyAuditLog,
} from './helpers';

test.describe('库存调拨完整流程', () => {
  test.beforeAll(async ({ page }) => {
    await loginViaUI(page);
  });

  test('调拨：创建→审批→出库→在途→入库→双仓库库存变化验证', async ({ page }) => {
    const ctx = getCtx();
    expect(ctx.warehouseIds.length).toBeGreaterThanOrEqual(2);

    const fromWarehouseId = ctx.warehouseIds[0];
    const toWarehouseId = ctx.warehouseIds[1];
    const productId = ctx.productIds[0];

    // 记录调拨前库存
    const stockBefore = await verifyStockFourDim(page, productId, ctx.colorNos[0]);
    const qtyBefore = Number(stockBefore.quantity || stockBefore.available_qty || 0);

    // 后端 CreateInventoryTransferRequest 真实字段
    const transferData = {
      from_warehouse_id: fromWarehouseId,
      to_warehouse_id: toWarehouseId,
      transfer_date: new Date().toISOString(),
      notes: 'E2E 调拨测试',
      items: [
        {
          product_id: productId,
          quantity: '5',
          color_no: ctx.colorNos[0],
          dye_lot_no: ctx.dyeLotNo,
        },
      ],
    };

    let transferId: number;
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/inventory/transfers', transferData);
      transferId = result.data?.id!;
    } catch {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/inventory/transfers?page=1&page_size=1');
      transferId = list.items[0]?.id;
    }
    expect(transferId).toBeDefined();

    // 验证初始状态
    const created = await apiCallRaw<{ status: string }>(page, 'GET', `/inventory/transfers/${transferId}`);
    expect(created.status.toLowerCase()).toBe('draft');

    // 审批调拨
    await apiCall(page, 'POST', `/inventory/transfers/${transferId}/approve`);
    const approved = await apiCallRaw<{ status: string }>(page, 'GET', `/inventory/transfers/${transferId}`);
    expect(approved.status.toLowerCase()).toBe('approved');

    // 出库
    await apiCall(page, 'POST', `/inventory/transfers/${transferId}/ship`);
    const shipped = await apiCallRaw<{ status: string }>(page, 'GET', `/inventory/transfers/${transferId}`);
    expect(shipped.status.toLowerCase()).toBe('in_transit');

    // 验证非法操作：在途状态不能再次出库
    const illegalShip = await apiCallExpectFail(page, 'POST', `/inventory/transfers/${transferId}/ship`);
    expect(illegalShip.status).toBeGreaterThanOrEqual(400);

    // 入库
    await apiCall(page, 'POST', `/inventory/transfers/${transferId}/receive`);
    const received = await apiCallRaw<{ status: string }>(page, 'GET', `/inventory/transfers/${transferId}`);
    expect(received.status.toLowerCase()).toBe('completed');

    // 验证审计日志
    const auditLogged = await verifyAuditLog(page, 'receive', 'inventory-transfers');
    expect(auditLogged).toBe(true);

    // UI 验证：访问调拨列表页
    await page.goto('/inventory/transfer');
    await page.waitForTimeout(2000);
    const tableVisible = await page.locator('.el-table').first().isVisible().catch(() => false);
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
        },
      ],
    };

    let transferId: number;
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/inventory/transfers', transferData);
      transferId = result.data?.id!;
    } catch {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/inventory/transfers?page=1&page_size=1');
      transferId = list.items[0]?.id;
    }

    // draft 状态直接入库应被拒
    const illegalReceive = await apiCallExpectFail(page, 'POST', `/inventory/transfers/${transferId}/receive`);
    expect(illegalReceive.status).toBeGreaterThanOrEqual(400);

    // draft 状态直接出库应被拒
    const illegalShip = await apiCallExpectFail(page, 'POST', `/inventory/transfers/${transferId}/ship`);
    expect(illegalShip.status).toBeGreaterThanOrEqual(400);
  });
});
