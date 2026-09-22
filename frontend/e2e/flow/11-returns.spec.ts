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
  seedFourDimStockIn,
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

  test('销售退货：追溯三列回写自原出库行（出库→退货→入库四维命中）', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0] || 1;
    const warehouseId = ctx.warehouseIds[0];
    expect(warehouseId, '需要真实仓库用于出库回写').toBeTruthy();

    // 本轮唯一四维：出库明细将如实记录这组维度，作为退货追溯断言的基准
    const stamp = Date.now().toString().slice(-6);
    const colorNo = `E2E-RET-C${stamp}`;
    const dyeLotNo = `E2E-RET-D${stamp}`;
    const batchNo = `E2E-RET-B${stamp}`;

    // 1. 按四维入库一行专属库存（发货扣减来源，qty 足够不触发跨缸拆分）
    await seedFourDimStockIn(page, {
      productId,
      warehouseId,
      colorNo,
      dyeLotNo,
      batchNo,
      quantityMeters: '1000',
    });

    // 2. 取真实仓库编码（ship 按 warehouse_code 匹配，硬编码在空库必失败）
    const wh = await apiCallRaw<{ warehouse_code?: string }>(
      page,
      'GET',
      `/warehouses/${warehouseId}`
    );
    const warehouseCode = wh?.warehouse_code;
    expect(warehouseCode, `仓库 ${warehouseId} 应返回 warehouse_code`).toBeTruthy();

    // 3. 建销售订单 → 提交 → 审批
    const so = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      items: [{ product_id: productId, quantity: 5, unit_price: '20.00' }],
    });
    const soId = so.data?.id;
    expect(soId, `销售订单创建失败：${JSON.stringify(so).slice(0, 200)}`).toBeTruthy();
    await apiCall(page, 'POST', `/sales/orders/${soId}/submit`);
    await apiCall(page, 'POST', `/sales/orders/${soId}/approve`);

    // 4. 四维发货：落库出库明细行 = 上面那组维度（实际被扣库存行的真实维度）
    await apiCall(page, 'POST', `/sales/orders/${soId}/ship`, {
      order_id: soId,
      warehouse_code: warehouseCode,
      items: [
        {
          product_id: productId,
          quantity: 5,
          color_no: colorNo,
          dye_lot_no: dyeLotNo,
          batch_no: batchNo,
        },
      ],
    });
    // 出库单必须真实生成（否则回写无来源、断言失去前提）
    const deliveries = await apiCallRaw<{ list?: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/sales/orders/${soId}/deliveries`
    );
    expect(
      deliveries?.list?.length ?? 0,
      `订单 ${soId} 应已生成出库单，作为退货追溯权威来源`
    ).toBeGreaterThan(0);

    // 5. 建退货单（关联销售订单）
    const rtn = await apiCall<{ id?: number }>(page, 'POST', '/sales/sales-returns', {
      order_id: soId,
      customer_id: ctx.customerId,
      return_date: new Date().toISOString().slice(0, 10),
      warehouse_id: warehouseId,
      reason_type: 'customer_cancel',
      reason_detail: '退回原出库缸',
    });
    const returnId = rtn.data?.id;
    expect(returnId, `退货单创建失败：${JSON.stringify(rtn).slice(0, 200)}`).toBeTruthy();

    // 6. 添加退货明细：不传追溯字段，系统必须从原出库行回写
    await apiCall(page, 'POST', `/sales/sales-returns/${returnId}/items`, {
      product_id: productId,
      quantity: '3',
      unit_price: '20.00',
    });

    // 7. 回读退货明细，断言追溯三列等于原出库行（真实断言，不吞错、不 skip）
    const items = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      `/sales/sales-returns/${returnId}/items`
    );
    expect(Array.isArray(items) && items.length, '退货明细应至少一行').toBe(true);
    const it = items[0];
    expect(String(it.color_no), '退货明细色号应回写自原出库行').toBe(colorNo);
    expect(String(it.dye_lot_no), '退货明细缸号应回写自原出库行').toBe(dyeLotNo);
    expect(String(it.batch_no), '退货明细批次应回写自原出库行').toBe(batchNo);
    expect(it.color_no, '退货明细色号不得为空串（必须由权威来源回写）').not.toBe('');

    // 8. 后段流转：提交→审批→执行入库
    await apiCall(page, 'POST', `/sales/sales-returns/${returnId}/submit`);
    const submitted = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/sales/sales-returns/${returnId}`
    );
    expect(submitted.status.toLowerCase()).toBe('submitted');

    await apiCall(page, 'POST', `/sales/sales-returns/${returnId}/approve`);
    const approved = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/sales/sales-returns/${returnId}`
    );
    expect(approved.status.toLowerCase()).toBe('approved');

    await apiCall(page, 'POST', `/sales/sales-returns/${returnId}/execute`);
    const executed = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/sales/sales-returns/${returnId}`
    );
    expect(executed.status.toLowerCase()).toBe('completed');

    // 9. 退货再入库必须命中真实色号行（四维口径），而非回写失败落到的空色号行
    const stockAfter = await verifyStockFourDim(page, productId, colorNo, dyeLotNo, {
      batchNo,
      warehouseId,
    });
    expect(stockAfter, `退货入库应命中四维库存行（色号 ${colorNo}）`).toBeTruthy();
    expect(String(stockAfter!.color_no), '命中库存行色号应等于退货明细色号').toBe(colorNo);
    expect(
      Number(stockAfter!.quantity_on_hand),
      `退货入库后在库量应大于 0（实际 ${stockAfter!.quantity_on_hand}）`
    ).toBeGreaterThan(0);

    // 10. 非法操作：已完成退货不能再次审批
    const illegalApprove = await apiCallExpectFail(
      page,
      'POST',
      `/sales/sales-returns/${returnId}/approve`
    );
    expect(illegalApprove.status).toBeGreaterThanOrEqual(400);
  });
});
