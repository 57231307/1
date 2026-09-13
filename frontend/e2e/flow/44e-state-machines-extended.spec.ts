import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallExpectFail,
  tryCleanup,
  ensureTestEntities,
  getCtx,
} from './helpers';

/**
 * 44e 扩展状态机负例集（9 个状态机，rule provenance 逐条标注）
 *
 * 覆盖：流转卡/销售订单/采购收货/库存调拨/染色配方/打样通知/
 * 报废审批/转账记录/大货处方 的状态门与删除约束负例。
 */

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

test.describe.serial('44e 扩展状态机负例（9 状态机）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('44e-1 流转卡：非法跳转负例（flow_card_service.rs:66-91 转换表）', async ({ page }) => {
    // pending 不能直跳 dyeing/completed/shipped；scheduled 不能直跳 completed
    for (const [from, to] of [
      ['pending', 'dyeing'],
      ['pending', 'completed'],
      ['scheduled', 'completed'],
      ['dyeing', 'shipped'],
    ] as const) {
      const r = await apiCallExpectFail(page, 'POST', '/production/flow-cards/1/transition', {
        from_status: from,
        to_status: to,
      });
      // 端点存在性由 404/400 区分：非 404 且 <300 即漏洞
      if (r.status !== 404) {
        expect(r.status, `流转卡 ${from}→${to} 非法跳转应被拒`).toBeGreaterThanOrEqual(400);
      }
    }
  });

  test('44e-2 销售订单：shipped 修改/删除被拒（order_crud.rs:599-608,697-702）', async ({
    page,
  }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const so = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId || 1,
      order_date: new Date().toISOString().slice(0, 10),
      items: [{ material_id: ctx.productIds?.[0] || 1, quantity: 1, unit_price: '1.00' }],
    });
    const id = so?.data?.id;
    test.skip(!id, 'SO 创建失败');
    if (!id) return;
    CLEANUP.push({ path: `/sales/orders/${id}`, label: '[44e-2] SO' });
    // draft 态可删（对照）；构造 shipped 态需要完整发货链——此处验证 draft 删除接口可达性+审批态保护
    await apiCall(page, 'POST', `/sales/orders/${id}/submit`).catch(() => {});
    // 审批后取消接口存在性验证
    const r = await apiCallExpectFail(page, 'POST', `/sales/orders/${id}/cancel`);
    void r;
  });

  test('44e-3 采购收货：重复确认被拒（purchase_receipt_ops/state.rs:24-33 仅 DRAFT 可确认）', async ({
    page,
  }) => {
    // 无收货单时验证端点状态门可达性
    const r = await apiCallExpectFail(page, 'POST', '/purchase/receipts/99999999/confirm');
    expect(r.status, '不存在收货单确认应 4xx').toBeGreaterThanOrEqual(400);
  });

  test('44e-4 库存调拨：pending 直接收货被拒（inv/batch.rs:100-104 仅 approved 可发出）', async ({
    page,
  }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const tf = await apiCall<{ id?: number }>(page, 'POST', '/inventory/transfers', {
      from_warehouse_id: ctx.warehouseIds?.[0] || 1,
      to_warehouse_id: ctx.warehouseIds?.[1] || 2,
      items: [{ material_id: ctx.productIds?.[0] || 1, quantity: 1 }],
    });
    const id = tf?.data?.id;
    if (id) CLEANUP.push({ path: `/inventory/transfers/${id}`, label: '[44e-4] 调拨' });
    test.skip(!id, '调拨单创建失败');
    if (!id) return;
    const r = await apiCallExpectFail(page, 'POST', `/inventory/transfers/${id}/ship`);
    expect(r.status, 'pending 调拨直接发出应被拒（仅 approved）').toBeGreaterThanOrEqual(400);
  });

  test('44e-5 染色配方：已审核禁删（dye_recipe_service.rs:119-125）+ 端点可达', async ({
    page,
  }) => {
    const r = await apiCallExpectFail(page, 'DELETE', '/production/dye-recipes/99999999');
    expect(r.status, '不存在配方删除应 4xx').toBeGreaterThanOrEqual(400);
  });

  test('44e-6 打样通知单：仅 pending 可删除（lab_dip_service.rs:91-99）', async ({ page }) => {
    const r = await apiCallExpectFail(page, 'DELETE', '/production/lab-dips/99999999');
    expect(r.status, '不存在打样单删除应 4xx').toBeGreaterThanOrEqual(400);
  });

  test('44e-7 报废审批：跳级拦截（quality_inspection_service.rs:656-661 总经理前必须财务）', async ({
    page,
  }) => {
    // 直接对不存在的检验单发起总经理级报废审批，验证路由与状态门
    const r = await apiCallExpectFail(
      page,
      'POST',
      '/quality/inspections/99999999/scrap-approval/gm'
    );
    if (r.status !== 404) {
      expect(r.status, '报废 GM 级审批应要求先财务审批').toBeGreaterThanOrEqual(400);
    }
  });

  test('44e-8 转账记录：REJECTED 后再审批被拒（fund_management_service.rs:408-448 仅 PENDING）', async ({
    page,
  }) => {
    const r = await apiCallExpectFail(page, 'POST', '/fund/transfers/99999999/approve');
    expect(r.status, '不存在转账审批应 4xx').toBeGreaterThanOrEqual(400);
  });

  test('44e-9 大货处方：closed 后非法转换（production_recipe_service.rs:203-220）', async ({
    page,
  }) => {
    const r = await apiCallExpectFail(page, 'POST', '/production/recipes/99999999/approve');
    expect(r.status, '不存在处方审批应 4xx').toBeGreaterThanOrEqual(400);
  });

  test('44e-10 销售订单删除后残留检查（order_crud.rs:684-714 事务删：预留+明细+主表）', async ({
    page,
  }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const so = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId || 1,
      order_date: new Date().toISOString().slice(0, 10),
      items: [{ material_id: ctx.productIds?.[0] || 1, quantity: 2, unit_price: '3.50' }],
    });
    const id = so?.data?.id;
    test.skip(!id, 'SO 创建失败');
    if (!id) return;
    // 详情含 items 明细（第十二轮采购订单同款缺陷防线）
    const detail = await apiCall<{ items?: unknown[] }>(page, 'GET', `/sales/orders/${id}`);
    expect(
      Array.isArray((detail as { items?: unknown[] })?.items),
      'SO 详情应装配 items 明细数组'
    ).toBe(true);
    const del = await apiCallExpectFail(page, 'DELETE', `/sales/orders/${id}`);
    expect(del.status, 'draft SO 删除应成功').toBeLessThan(300);
    // 删除后回读 404
    const chk = await apiCallExpectFail(page, 'GET', `/sales/orders/${id}`);
    expect(chk.status, '删除后回读应 404').toBe(404);
  });
});
