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

test.describe('采购退货完整流程', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('采购退货：创建→提交→审批→关联原采购单验证', async ({ page }) => {
    const ctx = getCtx();
    const poId = ctx.purchaseOrderId;
    expect(poId).toBeDefined();

    // 退货入库依赖库存：以实际库存行为准（修 ctx.warehouseIds 漂移）
    const stockRow = await ensureStockInWarehouse(
      page,
      ctx.productIds[0] || 1,
      ctx.warehouseIds[0],
      ctx.colorNos[0]
    );
    const warehouseId = Number(stockRow.warehouse_id) || ctx.warehouseIds[0] || 1;

    // 后端 CreatePurchaseReturnRequest 真实字段
    const returnData = {
      order_id: poId,
      supplier_id: ctx.supplierId,
      return_date: new Date().toISOString().slice(0, 10),
      warehouse_id: warehouseId,
      reason_type: 'quality',
      reason_detail: '布面疵点超标，客户拒收',
    };

    const result = await apiCall<{ id?: number }>(page, 'POST', '/purchase/returns', returnData);
    const returnId = result.data?.id;
    expect(
      returnId,
      `采购退货创建应返回 data.id，实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    // 添加退货明细（后端需要独立端点添加 items）
    // 明细缺失会导致 approve 撞"退货单至少需要一行明细"，失败必须暴露
    await apiCall(page, 'POST', `/purchase/returns/${returnId}/items`, {
      line_no: 1,
      material_id: Number(stockRow.product_id) || ctx.productIds[0] || 1,
      quantity_returned: '10',
      unit_price: '15.50',
    });

    // 验证退货单状态
    const created = await apiCallRaw<{ return_status?: string; supplier_id: number }>(
      page,
      'GET',
      `/purchase/returns/${returnId}`
    );
    expect((created.return_status ?? '').toLowerCase()).toBe('draft');
    expect(created.supplier_id).toBe(ctx.supplierId);

    // 提交退货单
    await apiCall(page, 'POST', `/purchase/returns/${returnId}/submit`);
    const submitted = await apiCallRaw<{ return_status?: string }>(
      page,
      'GET',
      `/purchase/returns/${returnId}`
    );
    expect((submitted.return_status ?? '').toLowerCase()).toBe('submitted');

    // 审批退货单
    await apiCall(page, 'POST', `/purchase/returns/${returnId}/approve`);
    const approved = await apiCallRaw<{ return_status?: string }>(
      page,
      'GET',
      `/purchase/returns/${returnId}`
    );
    expect((approved.return_status ?? '').toLowerCase()).toBe('approved');

    // 验证非法转换：已审批的退货单不能再次提交
    const illegalSubmit = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/returns/${returnId}/submit`
    );
    expect(illegalSubmit.status).toBeGreaterThanOrEqual(400);

    // 验证审计日志
    const auditLogged = await verifyAuditLog(page, 'UPDATE', 'purchase');
    expect(auditLogged).toBe(true);

    // UI 验证：访问采购退货列表页
    await page.goto('/purchase-return');
    await page.waitForTimeout(2000);
    const tableVisible = await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first()
      .isVisible();
    expect(tableVisible).toBe(true);
  });

  test('销售退货：创建→提交→审批→执行→入库验证', async ({ page }) => {
    const ctx = getCtx();
    const soId = ctx.salesOrderId;
    expect(soId).toBeDefined();

    // 销售退货入库依赖库存：以实际库存行为准（修 ctx.warehouseIds 漂移）
    const stockRow = await ensureStockInWarehouse(
      page,
      ctx.productIds[0] || 1,
      ctx.warehouseIds[0],
      ctx.colorNos[0]
    );
    const warehouseId = Number(stockRow.warehouse_id) || ctx.warehouseIds[0] || 1;

    // 后端 CreateSalesReturnRequest 真实字段
    const returnData = {
      order_id: soId,
      customer_id: ctx.customerId,
      return_date: new Date().toISOString().slice(0, 10),
      warehouse_id: warehouseId,
      reason_type: 'customer_cancel',
      reason_detail: '客户取消订单',
    };

    const result = await apiCall<{ id?: number }>(page, 'POST', '/sales/sales-returns', returnData);
    const returnId = result.data?.id;
    expect(
      returnId,
      `销售退货创建应返回 data.id，实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    // 添加退货明细
    // 明细缺失会导致 submit 撞"至少一行明细"校验，失败必须暴露
    // tax_percent 显式提供：NOT NULL 税率列由请求值落地（无请求值时后端回落到关联
    // 销售订单同商品明细税率；本用例复用的历史订单未必含该产品，故显式传）
    await apiCall(page, 'POST', `/sales/sales-returns/${returnId}/items`, {
      product_id: Number(stockRow.product_id) || ctx.productIds[0] || 1,
      quantity: '5',
      unit_price: '20.00',
      tax_percent: '13',
    });

    // 提交
    await apiCall(page, 'POST', `/sales/sales-returns/${returnId}/submit`);
    const submitted = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/sales/sales-returns/${returnId}`
    );
    expect(submitted.status.toLowerCase()).toBe('submitted');

    // 审批
    await apiCall(page, 'POST', `/sales/sales-returns/${returnId}/approve`);
    const approved = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/sales/sales-returns/${returnId}`
    );
    expect(approved.status.toLowerCase()).toBe('approved');

    // 执行退货（触发入库）
    await apiCall(page, 'POST', `/sales/sales-returns/${returnId}/execute`);
    const executed = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/sales/sales-returns/${returnId}`
    );
    expect(executed.status.toLowerCase()).toBe('executed');

    // 验证库存增加：退货执行会入库，必须命中该四维库存行且在库量大于 0
    // （原实现读不存在的 quantity/available_qty 字段，恒为 0 后断言 >= 0 空转）
    const stockAfter = await verifyStockFourDim(page, ctx.productIds[0], ctx.colorNos[0]);
    expect(
      stockAfter,
      `退货执行后应命中库存行（产品 ${ctx.productIds[0]} / 色号 ${ctx.colorNos[0]}）`
    ).toBeTruthy();
    expect(String(stockAfter!.color_no), '命中库存行的色号应与退货明细色号一致').toBe(
      ctx.colorNos[0]
    );
    expect(
      Number(stockAfter!.quantity_on_hand),
      `退货入库后在库量应大于 0（实际 ${stockAfter!.quantity_on_hand}）`
    ).toBeGreaterThan(0);

    // 验证非法操作：已执行的退货不能再次审批
    const illegalApprove = await apiCallExpectFail(
      page,
      'POST',
      `/sales/sales-returns/${returnId}/approve`
    );
    expect(illegalApprove.status).toBeGreaterThanOrEqual(400);
  });
});
