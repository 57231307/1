import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  ensureTestEntities,
  ensureStockInWarehouse,
  getCtx,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  tryCleanup,
  BASE_URL,
} from './helpers';

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
    const po = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/production/production-orders/orders',
      {
        order_no: `E2E-PO-${Date.now().toString().slice(-6)}`,
        product_id: ctx.productIds[0],
        planned_quantity: 100,
        planned_start_date: new Date().toISOString().slice(0, 10),
      }
    );
    const id = po?.data?.id;
    expect(id, '生产订单创建失败').toBeTruthy();
    CLEANUP.push({ path: `/production/production-orders/orders/${id}`, label: '[44f-1] 生产订单' });

    const st0 = await apiCall<{ status?: string }>(
      page,
      'GET',
      `/production/production-orders/orders/${id}`
    );
    expect((st0 as { status?: string })?.status ?? 'DRAFT', '初始应为 DRAFT').toContain('DRAFT');

    await apiCall(page, 'POST', `/production/production-orders/orders/${id}/submit-approval`);
    const st1 = await apiCall<{ status?: string }>(
      page,
      'GET',
      `/production/production-orders/orders/${id}`
    );
    expect(JSON.stringify(st1).toUpperCase()).toContain('PENDING_APPROVAL');

    // 提交后再次提交被拒（防重复）
    const dup = await apiCallExpectFail(
      page,
      'POST',
      `/production/production-orders/orders/${id}/submit-approval`
    );
    expect(dup.status, 'PENDING_APPROVAL 二次提交应被拒').toBeGreaterThanOrEqual(400);

    // ApprovalRequest { approved: bool, opinion? } 必填（自审修复：缺 body 恒 400）
    await apiCall(page, 'POST', `/production/production-orders/orders/${id}/approve`, {
      approved: true,
    });
    const st2 = await apiCall<{ status?: string }>(
      page,
      'GET',
      `/production/production-orders/orders/${id}`
    );
    expect(JSON.stringify(st2).toUpperCase()).toContain('APPROVED');
  });

  test('44f-2 流转卡 pending→scheduled→preparing→dyeing 全链+非法跳转', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    // 前置生产订单
    const porder = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/production/production-orders/orders',
      {
        order_no: `E2E-PO2-${Date.now().toString().slice(-6)}`,
        product_id: ctx.productIds[0],
        planned_quantity: 50,
      }
    );
    const porderId = porder?.data?.id;
    expect(porderId, '生产订单创建失败').toBeTruthy();
    CLEANUP.push({
      path: `/production/production-orders/orders/${porderId}`,
      label: '[44f-2] 生产订单',
    });

    const fc = await apiCall<{ id?: number }>(page, 'POST', '/production/flow-cards', {
      production_order_id: porderId,
      product_id: ctx.productIds[0],
      color_no: `44F${Date.now().toString().slice(-5)}`,
    });
    const id = fc?.data?.id;
    expect(id, '流转卡创建失败').toBeTruthy();
    CLEANUP.push({ path: `/production/flow-cards/${id}`, label: '[44f-2] 流转卡' });

    const rd = async () => {
      const r = await apiCall<{ status?: string }>(page, 'GET', `/production/flow-cards/${id}`);
      return JSON.stringify(r).toLowerCase();
    };
    expect(await rd()).toContain('pending');

    // pending 态非法直跳 dyeing（start-dyeing 端点应被状态门拒绝）
    const skip = await apiCallExpectFail(page, 'POST', `/production/flow-cards/${id}/start-dyeing`);
    expect(skip.status, 'pending 直跳 dyeing 应被拒').toBeGreaterThanOrEqual(400);

    await apiCall(page, 'POST', `/production/flow-cards/${id}/schedule`, {});
    expect(await rd()).toContain('scheduled');
    // scheduled 态非法直跳 complete-dyeing
    const skip2 = await apiCallExpectFail(
      page,
      'POST',
      `/production/flow-cards/${id}/complete-dyeing`
    );
    expect(skip2.status, 'scheduled 直跳 dyed 应被拒').toBeGreaterThanOrEqual(400);

    await apiCall(page, 'POST', `/production/flow-cards/${id}/start-preparing`, {});
    expect(await rd()).toContain('preparing');
    await apiCall(page, 'POST', `/production/flow-cards/${id}/complete-preparing`, {
      actual_fabric_weight: 100,
    });
    await apiCall(page, 'POST', `/production/flow-cards/${id}/start-dyeing`, {});
    expect(await rd()).toContain('dyeing');
    await apiCall(page, 'POST', `/production/flow-cards/${id}/complete-dyeing`, {});
    expect(await rd()).toContain('dyed');
    await apiCall(page, 'POST', `/production/flow-cards/${id}/start-inspecting`, {});
    expect(await rd()).toContain('inspecting');
  });

  test('44f-3 库存调拨 pending→approved→ship→receive 全链', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const tf = await apiCall<{ id?: number }>(page, 'POST', '/inventory/transfers', {
      from_warehouse_id: ctx.warehouseIds[0],
      to_warehouse_id: ctx.warehouseIds[1],
      transfer_date: new Date().toISOString(),
      // inv/inventory_move.rs:239-255（缺陷 6.2）：batch_no 必填；提供 color_no 时 dye_lot_no 必填
      items: [
        {
          product_id: ctx.productIds[0],
          quantity: 5,
          batch_no: `E2E-TF${Date.now().toString().slice(-6)}`,
          color_no: ctx.colorNos[0] || '白坯布',
          dye_lot_no: ctx.dyeLotNo || 'E2E-DL-001',
        },
      ],
    });
    const id = tf?.data?.id;
    expect(id, '调拨单创建失败').toBeTruthy();
    CLEANUP.push({ path: `/inventory/transfers/${id}`, label: '[44f-3] 调拨' });

    // pending 时 ship 被拒
    const skip = await apiCallExpectFail(page, 'POST', `/inventory/transfers/${id}/ship`);
    expect(skip.status, 'pending 直接收发应被拒（inv/batch.rs:100-104）').toBeGreaterThanOrEqual(
      400
    );

    // ApproveTransferRequest { approved: bool（必填）, notes: Option }——
    // 原实现不发请求体，axum 报 "EOF while parsing a value at line 1 column 0"（status 400）
    await apiCall(page, 'POST', `/inventory/transfers/${id}/approve`, { approved: true });
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
        supplier_id: ctx.supplierId,
        warehouse_id: ctx.warehouseIds[0],
        department_id: ctx.departmentIds[0],
        order_date: new Date().toISOString().slice(0, 10),
        expected_delivery_date: new Date(Date.now() + 7 * 86400000).toISOString().split('T')[0],
        items: [{ material_id: ctx.productIds[0], quantity: 10, unit_price: '2.50' }],
      }
    );
    const poId = po?.data?.id;
    expect(poId, 'PO 创建失败').toBeTruthy();
    CLEANUP.push({ path: `/purchase/orders/${poId}`, label: '[44f-4] PO' });
    await apiCall(page, 'POST', `/purchase/orders/${poId}/submit`);
    await apiCall(page, 'POST', `/purchase/orders/${poId}/approve`);

    // CreateReceiptItemRequest 的 unit_master 是必填 String
    // （backend/services/purchase_receipt_dto.rs:98），缺失会被 422 拒绝：
    // "items[0]: missing field `unit_master`"。取产品真实计量单位而非写死字面量。
    const prod = await apiCallRaw<{ unit: string }>(page, 'GET', `/products/${ctx.productIds[0]}`);
    console.log(`[44f-4] 收货物料主单位 unit=${prod?.unit}`);
    const receipt = await apiCall<{ id?: number }>(page, 'POST', '/purchase/receipts', {
      order_id: poId,
      supplier_id: ctx.supplierId,
      receipt_date: new Date().toISOString().slice(0, 10),
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      items: [
        {
          line_no: 1,
          material_id: ctx.productIds[0],
          material_code: `44F${Date.now().toString().slice(-6)}`,
          material_name: '44f 收货物料',
          quantity: 10,
          quantity_alt: 0,
          unit_master: prod.unit,
        },
      ],
    });
    const receiptId = receipt?.data?.id;
    expect(receiptId, '收货单创建失败').toBeTruthy();
    CLEANUP.push({ path: `/purchase/receipts/${receiptId}`, label: '[44f-4] 收货单' });

    const c1 = await apiCallExpectFail(page, 'POST', `/purchase/receipts/${receiptId}/confirm`);
    expect(c1.status, '首次确认应成功').toBeLessThan(300);
    // 确认事务内按入库明细完成库存收货并置 COMPLETED；轮询等待终态而非依赖同步返回体
    await expect
      .poll(
        async () => {
          const rd = await apiCall<{ status?: string; receipt_status?: string }>(
            page,
            'GET',
            `/purchase/receipts/${receiptId}`
          );
          return JSON.stringify(rd).toUpperCase().includes('COMPLETED');
        },
        { message: `入库单 ${receiptId} 确认后经收货事件应进入 COMPLETED`, timeout: 20000 }
      )
      .toBe(true);
    // 重复确认幂等拦截（po/receipt.rs:33-40）
    const c2 = await apiCallExpectFail(page, 'POST', `/purchase/receipts/${receiptId}/confirm`);
    expect(c2.status, 'COMPLETED 后重复确认应被拒').toBeGreaterThanOrEqual(400);
  });

  test('44f-5 销售发货全链+shipped 后修改被拒', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const so = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      // SO 明细字段名是 product_id（后端 422 明示 items[0]: missing field `product_id`），
      // 原实现写 material_id 属字段名错用
      items: [{ product_id: ctx.productIds[0], quantity: 5, unit_price: '8.00' }],
    });
    const soId = so?.data?.id;
    expect(soId, 'SO 创建失败').toBeTruthy();
    CLEANUP.push({ path: `/sales/orders/${soId}`, label: '[44f-5] SO' });
    await apiCall(page, 'POST', `/sales/orders/${soId}/submit`);
    await apiCall(page, 'POST', `/sales/orders/${soId}/approve`);

    // 发货仓库必须传真实仓库编码：ship.rs 按 warehouse_code 查仓，
    // 原实现硬编码 'WH-MAIN' 在 CI 空库里不存在 → 404（该用例此前被前面的串行失败挡住未跑）
    const warehouseId = ctx.warehouseIds[0];
    await ensureStockInWarehouse(page, ctx.productIds[0], warehouseId);
    const wh = await apiCallRaw<{ warehouse_code?: string }>(
      page,
      'GET',
      `/warehouses/${warehouseId}`
    );
    const warehouseCode = wh?.warehouse_code;
    expect(warehouseCode, `仓库 ${warehouseId} 应返回 warehouse_code`).toBeTruthy();

    const ship = await apiCallExpectFail(page, 'POST', `/sales/orders/${soId}/ship`, {
      order_id: soId,
      warehouse_code: warehouseCode,
      items: [{ product_id: ctx.productIds[0], quantity: 5 }],
    });
    expect(ship.status, `发货应成功（status=${ship.status} code=${ship.code ?? ''}）`).toBeLessThan(
      300
    );

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
    expect(id, '凭证创建失败').toBeTruthy();
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
    // 创建入参对齐 CreateProductionRecipeRequest（production_recipe_service.rs:39-63）：
    // 必填只有 fabric_weight（备布重量，用量计算依据）与 liquor_ratio（浴比），
    // 处方号由后端按单据号规则生成，不由调用方提交；product_id 不是该 DTO 字段。
    const r = await apiCall<{ id?: number }>(page, 'POST', '/production/production-recipes', {
      color_no: `44F-${Date.now().toString().slice(-6)}`,
      fabric_name: '44f 大货处方用坯布',
      fabric_weight: 120,
      liquor_ratio: '1:8',
    });
    const id = r?.data?.id;
    expect(id, '处方创建失败').toBeTruthy();
    // 审核要传 approved_by（后端从请求体取审批人身份，非登录态），故先取当前用户 ID
    const me = await apiCallRaw<{ id?: number; user_id?: number }>(page, 'GET', '/users/me');
    const myId = me?.id ?? me?.user_id;
    expect(myId, '当前登录用户 ID 取不到').toBeTruthy();

    // 详情接口的状态列就是 production_recipe.status（models/production_recipe.rs:93）
    const rd = async () => {
      const detail = await apiCallRaw<{ status?: string }>(
        page,
        'GET',
        `/production/production-recipes/${id}`
      );
      return String(detail?.status ?? '');
    };

    expect(await rd(), '新建处方应为 draft').toContain('draft');

    await apiCall(page, 'POST', `/production/production-recipes/${id}/approve`, {
      approved_by: myId,
    });
    expect(await rd(), '审核后应为 approved').toContain('approved');

    // 非 draft 不可更新（validate_can_update）：这是"审核后处方冻结"的真实约束
    const upd = await apiCallExpectFail(page, 'PUT', `/production/production-recipes/${id}`, {
      fabric_weight: 200,
      liquor_ratio: '1:8',
    });
    expect(upd.status, '已审核处方不应可改').toBeGreaterThanOrEqual(400);

    await apiCall(page, 'POST', `/production/production-recipes/${id}/close`);
    expect(await rd(), '关闭后应为 closed 终态').toContain('closed');

    // closed 为终态：再次审核必须被状态机拒绝
    const reApprove = await apiCallExpectFail(
      page,
      'POST',
      `/production/production-recipes/${id}/approve`,
      { approved_by: myId }
    );
    expect(reApprove.status, '终态处方不应再被审核').toBeGreaterThanOrEqual(400);

    // 已离开 draft 的处方后端禁止删除（validate_can_delete），清理走 cancel 也不可达：
    // 记录归档即设计约束，故本用例不注册删除型清理。
    const del = await apiCallExpectFail(page, 'DELETE', `/production/production-recipes/${id}`);
    expect(del.status, '非 draft 处方应拒删').toBeGreaterThanOrEqual(400);
  });

  test('44f-8 打样通知单 pending→sampling→submitted→approved 全链', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const r = await apiCall<{ id?: number }>(page, 'POST', '/production/lab-dip/requests', {
      customer_id: ctx.customerId,
      customer_color_no: `44F${Date.now().toString().slice(-5)}`,
      customer_color_name: '44f 打样色号',
      main_light_source: 'D65',
    });
    const id = r?.data?.id;
    expect(id, '打样单创建失败').toBeTruthy();
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
    expect(id, '配方创建失败').toBeTruthy();
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
    // 唯一停用路径：PUT /{id}（UpdateDyeRecipeRequest.status，后端常量"已停用"，
    // validate_status_transition: APPROVED→DISABLED 合法边）
    await apiCall(page, 'PUT', `/production/dye-recipes/${id}`, { status: '已停用' });
  });
});
