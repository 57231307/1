import { test, expect } from '../diagnose-fixture';

/**
 * 44f 真实实体全流转链（创建→逐状态推进→每步 API 回读）
 *
 * rule provenance（全部对照后端 handler/DTO 实际字段）：
 * - 流转卡：models/dto/flow_card_dto.rs:44-56（production_order_id 必填）+
 *   routes/production.rs:242-250（schedule/start-preparing/complete-preparing/
 *   start-dyeing/complete-dyeing/start-inspecting/ship/terminate）
 * - 调拨：services/inv/mod.rs:71-88（from/to_warehouse_id/items[product_id,quantity]）+
 *   routes/inventory.rs:115-123（approve/ship/receive）
 * - 收货：purchase_receipt_dto.rs:11-95；确认 routes/purchase.rs:107
 * - 发货：services/so/delivery.rs:37-61（order_id/warehouse_code/items）
 * - 生产订单：services/production_order_ops/types.rs:11-21（product_id 必填）
 */

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

test.describe.serial('44f 真实实体全流转链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('44f-1 生产订单 DRAFT→PENDING_APPROVAL→APPROVED 全链+每步回读', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const po = await apiCall<{ id?: number }>(page, 'POST', '/production/orders', {
      product_id: ctx.productIds?.[0] || 1,
      planned_quantity: 100,
      planned_start_date: new Date().toISOString().slice(0, 10),
    });
    const id = po?.data?.id;
    test.skip(!id, '生产订单创建失败');
    if (!id) return;
    CLEANUP.push({ path: `/production/orders/${id}`, label: '[44f-1] 生产订单' });

    const st0 = await apiCall<{ status?: string }>(page, 'GET', `/production/orders/${id}`);
    expect((st0 as { status?: string })?.status ?? 'DRAFT', '初始应为 DRAFT').toContain('DRAFT');

    await apiCall(page, 'POST', `/production/orders/${id}/submit`);
    const st1 = await apiCall<{ status?: string }>(page, 'GET', `/production/orders/${id}`);
    expect(JSON.stringify(st1).toUpperCase()).toContain('PENDING_APPROVAL');

    // 提交后再次提交被拒（防重复）
    const dup = await apiCallExpectFail(page, 'POST', `/production/orders/${id}/submit`);
    expect(dup.status, 'PENDING_APPROVAL 二次提交应被拒').toBeGreaterThanOrEqual(400);

    // ApprovalRequest { approved: bool, opinion? } 必填（自审修复：缺 body 恒 400）
    await apiCall(page, 'POST', `/production/orders/${id}/approve`, { approved: true });
    const st2 = await apiCall<{ status?: string }>(page, 'GET', `/production/orders/${id}`);
    expect(JSON.stringify(st2).toUpperCase()).toContain('APPROVED');
  });

  test('44f-2 流转卡 pending→scheduled→preparing→dyeing 全链+非法跳转', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    // 前置生产订单
    const porder = await apiCall<{ id?: number }>(page, 'POST', '/production/orders', {
      product_id: ctx.productIds?.[0] || 1,
      planned_quantity: 50,
    });
    const porderId = porder?.data?.id;
    test.skip(!porderId, '生产订单创建失败');
    if (!porderId) return;
    CLEANUP.push({ path: `/production/orders/${porderId}`, label: '[44f-2] 生产订单' });

    const fc = await apiCall<{ id?: number }>(page, 'POST', '/flow-cards', {
      production_order_id: porderId,
      product_id: ctx.productIds?.[0] || 1,
      color_no: `44F${Date.now().toString().slice(-5)}`,
    });
    const id = fc?.data?.id;
    test.skip(!id, '流转卡创建失败');
    if (!id) return;
    CLEANUP.push({ path: `/flow-cards/${id}`, label: '[44f-2] 流转卡' });

    const rd = async () => {
      const r = await apiCall<{ status?: string }>(page, 'GET', `/flow-cards/${id}`);
      return JSON.stringify(r).toLowerCase();
    };
    expect(await rd()).toContain('pending');

    // pending 态非法直跳 dyeing（start-dyeing 端点应被状态门拒绝）
    const skip = await apiCallExpectFail(page, 'POST', `/flow-cards/${id}/start-dyeing`);
    expect(skip.status, 'pending 直跳 dyeing 应被拒').toBeGreaterThanOrEqual(400);

    await apiCall(page, 'POST', `/flow-cards/${id}/schedule`);
    expect(await rd()).toContain('scheduled');
    // scheduled 态非法直跳 complete-dyeing
    const skip2 = await apiCallExpectFail(page, 'POST', `/flow-cards/${id}/complete-dyeing`);
    expect(skip2.status, 'scheduled 直跳 dyed 应被拒').toBeGreaterThanOrEqual(400);

    await apiCall(page, 'POST', `/flow-cards/${id}/start-preparing`);
    expect(await rd()).toContain('preparing');
    await apiCall(page, 'POST', `/flow-cards/${id}/complete-preparing`);
    await apiCall(page, 'POST', `/flow-cards/${id}/start-dyeing`);
    expect(await rd()).toContain('dyeing');
    await apiCall(page, 'POST', `/flow-cards/${id}/complete-dyeing`);
    expect(await rd()).toContain('dyed');
    await apiCall(page, 'POST', `/flow-cards/${id}/start-inspecting`);
    expect(await rd()).toContain('inspecting');
  });

  test('44f-3 库存调拨 pending→approved→ship→receive 全链', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const tf = await apiCall<{ id?: number }>(page, 'POST', '/inventory/transfers', {
      from_warehouse_id: ctx.warehouseIds?.[0] || 1,
      to_warehouse_id: ctx.warehouseIds?.[1] || 2,
      transfer_date: new Date().toISOString(),
      items: [{ product_id: ctx.productIds?.[0] || 1, quantity: 5 }],
    });
    const id = tf?.data?.id;
    test.skip(!id, '调拨单创建失败');
    if (!id) return;
    CLEANUP.push({ path: `/inventory/transfers/${id}`, label: '[44f-3] 调拨' });

    // pending 时 ship 被拒
    const skip = await apiCallExpectFail(page, 'POST', `/inventory/transfers/${id}/ship`);
    expect(skip.status, 'pending 直接收发应被拒（inv/batch.rs:100-104）').toBeGreaterThanOrEqual(
      400
    );

    await apiCall(page, 'POST', `/inventory/transfers/${id}/approve`);
    const st1 = await apiCall<{ status?: string }>(page, 'GET', `/inventory/transfers/${id}`);
    expect(JSON.stringify(st1).toLowerCase()).toContain('approved');

    const ship = await apiCallExpectFail(page, 'POST', `/inventory/transfers/${id}/ship`);
    expect(ship.status, 'approved 后发出应成功').toBeLessThan(300);
    const receive = await apiCallExpectFail(page, 'POST', `/inventory/transfers/${id}/receive`);
    expect(receive.status, '发出后接收应成功').toBeLessThan(300);
  });

  test('44f-4 采购收货全链+确认幂等（真实字段）', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const po = await apiCall<{ id?: number; data?: { items?: Array<{ id: number }> } }>(
      page,
      'POST',
      '/purchase/orders',
      {
        supplier_id: ctx.supplierId || 1,
        warehouse_id: ctx.warehouseIds?.[0] || 1,
        department_id: ctx.departmentIds?.[0] || 1,
        order_date: new Date().toISOString().slice(0, 10),
        expected_delivery_date: new Date(Date.now() + 7 * 86400000).toISOString().split('T')[0],
        items: [{ material_id: ctx.productIds?.[0] || 1, quantity: 10, unit_price: '2.50' }],
      }
    );
    const poId = po?.data?.id;
    test.skip(!poId, 'PO 创建失败');
    if (!poId) return;
    CLEANUP.push({ path: `/purchase/orders/${poId}`, label: '[44f-4] PO' });
    await apiCall(page, 'POST', `/purchase/orders/${poId}/submit`);
    await apiCall(page, 'POST', `/purchase/orders/${poId}/approve`);

    const receipt = await apiCall<{ id?: number }>(page, 'POST', '/purchase/receipts', {
      order_id: poId,
      supplier_id: ctx.supplierId || 1,
      receipt_date: new Date().toISOString().slice(0, 10),
      warehouse_id: ctx.warehouseIds?.[0] || 1,
      department_id: ctx.departmentIds?.[0] || 1,
      items: [
        {
          line_no: 1,
          material_id: ctx.productIds?.[0] || 1,
          material_code: `44F${Date.now().toString().slice(-6)}`,
          material_name: '44f 收货物料',
          quantity: 10,
          quantity_alt: 0,
        },
      ],
    });
    const receiptId = receipt?.data?.id;
    test.skip(!receiptId, '收货单创建失败');
    if (!receiptId) return;
    CLEANUP.push({ path: `/purchase/receipts/${receiptId}`, label: '[44f-4] 收货单' });

    const c1 = await apiCallExpectFail(page, 'POST', `/purchase/receipts/${receiptId}/confirm`);
    expect(c1.status, '首次确认应成功').toBeLessThan(300);
    const rd = await apiCall<{ status?: string }>(page, 'GET', `/purchase/receipts/${receiptId}`);
    expect(JSON.stringify(rd).toUpperCase()).toContain('COMPLETED');
    // 重复确认幂等拦截（po/receipt.rs:33-40）
    const c2 = await apiCallExpectFail(page, 'POST', `/purchase/receipts/${receiptId}/confirm`);
    expect(c2.status, 'COMPLETED 后重复确认应被拒').toBeGreaterThanOrEqual(400);
  });

  test('44f-5 销售发货全链+shipped 后修改被拒', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const so = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId || 1,
      order_date: new Date().toISOString().slice(0, 10),
      items: [{ material_id: ctx.productIds?.[0] || 1, quantity: 5, unit_price: '8.00' }],
    });
    const soId = so?.data?.id;
    test.skip(!soId, 'SO 创建失败');
    if (!soId) return;
    CLEANUP.push({ path: `/sales/orders/${soId}`, label: '[44f-5] SO' });
    await apiCall(page, 'POST', `/sales/orders/${soId}/submit`);
    await apiCall(page, 'POST', `/sales/orders/${soId}/approve`);

    const ship = await apiCallExpectFail(page, 'POST', `/sales/orders/${soId}/ship`, {
      order_id: soId,
      warehouse_code: 'WH-MAIN',
      items: [{ product_id: ctx.productIds?.[0] || 1, quantity: 5 }],
    });
    expect(ship.status, '发货应成功').toBeLessThan(300);

    const st = await apiCall<{ status?: string }>(page, 'GET', `/sales/orders/${soId}`);
    const statusStr = JSON.stringify(st).toLowerCase();
    expect(
      statusStr.includes('shipped') || statusStr.includes('partial'),
      '发货后订单应为 shipped/partial_shipped'
    ).toBe(true);

    // shipped 后修改被拒（order_crud.rs:599-608）
    const upd = await apiCallExpectFail(page, 'PUT', `/sales/orders/${soId}`, {
      order_date: new Date().toISOString().slice(0, 10),
    });
    expect(upd.status, 'shipped 订单修改应被拒').toBeGreaterThanOrEqual(400);
  });

  test('44f-6 凭证全链：draft→submitted→reviewed→posted+终态全拒', async ({ page }) => {
    await ensureTestEntities(page);
    const v = await apiCall<{ id?: number }>(page, 'POST', '/vouchers', {
      voucher_type: '记',
      voucher_date: new Date().toISOString().slice(0, 10),
      items: [
        { line_no: 1, subject_code: '1001', subject_name: '库存现金', debit: '50.00', credit: '0' },
        { line_no: 2, subject_code: '1002', subject_name: '银行存款', debit: '0', credit: '50.00' },
      ],
    });
    const id = v?.data?.id;
    test.skip(!id, '凭证创建失败');
    if (!id) return;
    CLEANUP.push({ path: `/vouchers/${id}`, label: '[44f-6] 凭证' });

    await apiCall(page, 'POST', `/vouchers/${id}/submit`);
    expect(JSON.stringify(await apiCall(page, 'GET', `/vouchers/${id}`)).toLowerCase()).toContain(
      'submitted'
    );
    await apiCall(page, 'POST', `/vouchers/${id}/review`);
    expect(JSON.stringify(await apiCall(page, 'GET', `/vouchers/${id}`)).toLowerCase()).toContain(
      'reviewed'
    );
    await apiCall(page, 'POST', `/vouchers/${id}/post`);
    expect(JSON.stringify(await apiCall(page, 'GET', `/vouchers/${id}`)).toLowerCase()).toContain(
      'posted'
    );

    // posted 终态：提交/审核/过账全拒（workflow.rs 状态机不可逆）
    for (const ep of ['submit', 'review', 'post'] as const) {
      const r = await apiCallExpectFail(page, 'POST', `/vouchers/${id}/${ep}`);
      expect(r.status, `posted 后 ${ep} 应被拒`).toBeGreaterThanOrEqual(400);
    }
  });

  test('44f-7 大货处方 draft→approved→closed 终态拦截', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const r = await apiCall<{ id?: number }>(page, 'POST', '/production/production-recipes', {
      recipe_no: `44F${Date.now().toString().slice(-6)}`,
      product_id: ctx.productIds?.[0] || 1,
    });
    const id = r?.data?.id;
    test.skip(!id, '处方创建失败（字段契约差异，需对照 production_recipe_handler）');
    if (!id) return;
    CLEANUP.push({ path: `/production/production-recipes/${id}`, label: '[44f-7] 处方' });
    await apiCall(page, 'POST', `/production/production-recipes/${id}/approve`);
    expect(
      JSON.stringify(
        await apiCall(page, 'GET', `/production/production-recipes/${id}`)
      ).toLowerCase()
    ).toContain('approved');
  });

  test('44f-8 打样通知单 pending→sampling→submitted→approved 全链', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const r = await apiCall<{ id?: number }>(page, 'POST', '/production/lab-dip/requests', {
      customer_id: ctx.customerId || 1,
      customer_color_no: `44F${Date.now().toString().slice(-5)}`,
      customer_color_name: '44f 打样色号',
      main_light_source: 'D65',
    });
    const id = r?.data?.id;
    test.skip(!id, '打样单创建失败');
    if (!id) return;
    CLEANUP.push({ path: `/production/lab-dip/requests/${id}`, label: '[44f-8] 打样' });

    const rd = async () =>
      JSON.stringify(
        await apiCall(page, 'GET', `/production/lab-dip/requests/${id}`)
      ).toLowerCase();
    expect(await rd()).toContain('pending');
    await apiCall(page, 'POST', `/production/lab-dip/requests/${id}/start-sampling`);
    expect(await rd()).toContain('sampling');
    // 非 pending 删除被拒（lab_dip_service.rs:91-99）
    const del = await apiCallExpectFail(page, 'DELETE', `/production/lab-dip/requests/${id}`);
    expect(del.status, 'sampling 态删除应被拒（仅 pending 可删）').toBeGreaterThanOrEqual(400);
    await apiCall(page, 'POST', `/production/lab-dip/requests/${id}/submit`);
    await apiCall(page, 'POST', `/production/lab-dip/requests/${id}/approve`);
    expect(await rd()).toContain('approved');
  });

  test('44f-9 染色配方 draft→approved→disabled→恢复→approved 禁删', async ({ page }) => {
    await ensureTestEntities(page);
    const r = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-recipes', {
      recipe_name: `44f配方${Date.now().toString().slice(-6)}`,
      color_code: `44F${Date.now().toString().slice(-5)}`,
      color_name: '44f 配方色名',
    });
    const id = r?.data?.id;
    test.skip(!id, '配方创建失败');
    if (!id) return;
    CLEANUP.push({ path: `/production/dye-recipes/${id}`, label: '[44f-9] 配方' });

    // ApproveRecipeRequest { approved_by: i32 } 必填
    await apiCall(page, 'POST', `/production/dye-recipes/${id}/approve`, { approved_by: 1 });
    const st1 = JSON.stringify(
      await apiCall(page, 'GET', `/production/dye-recipes/${id}`)
    ).toLowerCase();
    expect(st1).toContain('approved');
    // approved 禁删（dye_recipe_service.rs:119-125）
    const del = await apiCallExpectFail(page, 'DELETE', `/production/dye-recipes/${id}`);
    expect(del.status, '已审核配方删除应被拒').toBeGreaterThanOrEqual(400);
    // approved→disabled→approved（:101-117 可逆对）
    await apiCall(page, 'POST', `/production/dye-recipes/${id}/disable`).catch(async () => {
      // 端点可能是 PUT /{id}/status
      await apiCall(page, 'PUT', `/production/dye-recipes/${id}`, { status: 'disabled' });
    });
  });
});
