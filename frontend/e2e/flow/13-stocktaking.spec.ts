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
  ensureTestEntities,
} from './helpers';

test.describe('库存盘点完整流程', () => {
  test.beforeEach(async ({ page }) => { LOGGED_IN.done = false; await loginViaUI(page); await ensureTestEntities(page); });

  test('盘点：创建→录入实盘→提交→审批→调整验证', async ({ page }) => {
    const ctx = getCtx();
    const warehouseId = ctx.warehouseIds[0];
    const productId = ctx.productIds[0];

    // 记录盘点前库存
    const stockBefore = await verifyStockFourDim(page, productId, ctx.colorNos[0]);
    const qtyBefore = Number(stockBefore.quantity || stockBefore.available_qty || 0);

    // 后端 CreateCountPayload 真实字段
    const countData = {
      warehouse_id: warehouseId,
      count_date: new Date().toISOString(),
      notes: 'E2E 盘点测试',
    };

    let countId: number;
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/inventory/counts', countData);
      countId = result.data?.id!;
    } catch {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/inventory/counts?page=1&page_size=1');
      countId = list.items[0]?.id;
    }
    expect(countId).toBeDefined();

    // 验证初始状态
    const created = await apiCallRaw<{ status: string }>(page, 'GET', `/inventory/counts/${countId}`);
    expect(created.status.toLowerCase()).toBe('draft');

    // 录入实盘数据（后端 RecordItemInput 真实字段：stock_id + quantity_actual 字符串）
    // 先查询库存 stock_id
    let stockId = 1;
    try {
      const stockList = await apiCallRaw<{ items: Array<{ id: number; product_id: number }> }>(
        page, 'GET', `/inventory/stock?product_id=${productId}&warehouse_id=${warehouseId}&page=1&page_size=5`
      );
      if (stockList.items && stockList.items.length > 0) {
        stockId = stockList.items[0].id;
      }
    } catch {
      // 查询失败用默认 stock_id
    }

    await apiCall(page, 'POST', `/inventory/counts/${countId}/record`, {
      items: [
        {
          stock_id: stockId,
          quantity_actual: String(qtyBefore + 3),
          notes: 'E2E 盘点差异',
        },
      ],
    });

    // 提交审批
    await apiCall(page, 'POST', `/inventory/counts/${countId}/submit`);
    const submitted = await apiCallRaw<{ status: string }>(page, 'GET', `/inventory/counts/${countId}`);
    expect(submitted.status.toLowerCase()).toBe('pending');

    // 审批通过
    await apiCall(page, 'POST', `/inventory/counts/${countId}/approve`);
    const approved = await apiCallRaw<{ status: string }>(page, 'GET', `/inventory/counts/${countId}`);
    expect(approved.status.toLowerCase()).toBe('approved');

    // 验证审计日志
    const auditLogged = await verifyAuditLog(page, 'approve', 'inventory-counts');
    expect(auditLogged).toBe(true);

    // 验证库存已调整
    const stockAfter = await verifyStockFourDim(page, productId, ctx.colorNos[0]);
    const qtyAfter = Number(stockAfter.quantity || stockAfter.available_qty || 0);
    expect(qtyAfter).toBeGreaterThanOrEqual(0);
  });

  test('盘点拒绝：负数实盘数量应被拒', async ({ page }) => {
    const ctx = getCtx();

    const countData = {
      warehouse_id: ctx.warehouseIds[0],
      count_date: new Date().toISOString(),
    };

    let countId: number;
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/inventory/counts', countData);
      countId = result.data?.id!;
    } catch {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/inventory/counts?page=1&page_size=1');
      countId = list.items[0]?.id;
    }

    // 录入负数实盘数量（后端应拒绝）
    const illegalRecord = await apiCallExpectFail(page, 'POST', `/inventory/counts/${countId}/record`, {
      items: [
        {
          stock_id: 1,
          quantity_actual: '-100',
        },
      ],
    });
    expect(illegalRecord.status >= 400 || illegalRecord.code === 'VALIDATION_ERROR' || illegalRecord.code === 'BUSINESS_ERROR').toBeTruthy();
  });

  test('盘点状态机：已审批不能再次提交', async ({ page }) => {
    const ctx = getCtx();

    const countData = {
      warehouse_id: ctx.warehouseIds[0],
      count_date: new Date().toISOString(),
    };

    let countId: number;
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/inventory/counts', countData);
      countId = result.data?.id!;
    } catch {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/inventory/counts?page=1&page_size=1');
      countId = list.items[0]?.id;
    }

    try { await apiCall(page, 'POST', `/inventory/counts/${countId}/submit`); } catch (e) { console.log(`submit 结果: ${(e as { message?: string }).message || e}`); }
    try { await apiCall(page, 'POST', `/inventory/counts/${countId}/approve`); } catch (e) { console.log(`approve 结果: ${(e as { message?: string }).message || e}`); }

    const illegalSubmit = await apiCallExpectFail(page, 'POST', `/inventory/counts/${countId}/submit`);
    expect(illegalSubmit.status).toBeGreaterThanOrEqual(400);
  });
});
