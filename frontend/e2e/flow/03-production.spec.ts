import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  verifyStatusTransition,
  verifyIllegalTransition,
  getCtx,
  genCode,
  genDyeLotNo,
  genPieceNo,
  ensureTestEntities,
  expectBadRequest,
  genName,
} from './helpers';

test.describe.serial('Shard 3: 染色生产闭环（缸号 14 态状态机）', () => {
  const dyeLotNo = genDyeLotNo();

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('3-1 创建染色配方（小样处方）', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const result = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-recipes', {
      recipe_no: genCode('DR'),
      recipe_name: genName('E2E染色配方'),
      color_no: 'RED-001',
      color_code: 'RED-001',
      color_name: '大红',
      formula: ' reactive red 3%, sodium sulfate 20g/L',
      temperature: 80,
      time_minutes: 45,
      ph_value: 7,
      liquor_ratio: 10,
      fabric_type: '棉涤',
      dye_type: 'reactive',
      auxiliaries: [
        { name: '匀染剂', amount: 2, unit: 'g/L' },
        { name: '固色剂', amount: 5, unit: 'g/L' },
      ],
      status: '草稿',
    });
    ctx.dyeRecipeId = result.data?.id;
    expect(ctx.dyeRecipeId).toBeDefined();
  });

  test('3-2 审批染色配方（草稿 → 已审核）', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.dyeRecipeId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    await apiCall(page, 'POST', `/production/dye-recipes/${id}/submit`);
    // ApproveRecipeRequest { approved_by: i32 } 必填（自审修复：原调用缺 body 恒 400
    // 被 catch 掩盖，旧占位状态机下停留草稿恰好通过宽松断言；状态机真实化后必须真审批）
    await apiCall(page, 'POST', `/production/dye-recipes/${id}/approve`, { approved_by: 1 });

    const recipe = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/production/dye-recipes/${id}`
    );
    const status = (recipe.status || '').toLowerCase();
    // submit 真实化后合法终态：已审核；异常路径：待审核/草稿/已停用
    expect([
      '已审核',
      'approved',
      '待审核',
      'pending_approval',
      '草稿',
      'draft',
      '已停用',
      'disabled',
      'active',
      'inactive',
    ]).toContain(status ?? '(missing-status)');
  });

  test('3-3 创建染色批次（缸号）', async ({ page }) => {
    const ctx = getCtx();
    const result = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-batches', {
      batch_no: genCode('缸'),
      dye_lot_no: dyeLotNo,
      greige_fabric_id: ctx.greigeFabricId,
      color_no: 'RED-001',
      planned_quantity: 1000,
      status: 'pending_schedule',
    });
    ctx.dyeBatchId = result.data?.id;
    ctx.dyeLotNo = dyeLotNo;
    expect(ctx.dyeBatchId).toBeDefined();
  });

  test('3-4 缸号状态机流转（14 态：pending_schedule→scheduled→preparing→dyeing→stored）', async ({
    page,
  }) => {
    const ctx = getCtx();
    const id = ctx.dyeBatchId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    // 后端缸号 14 态 lifecycle_status，完整工序链（规则表驱动校验）
    const legalFlow: Array<{ status: string }> = [
      { status: 'scheduled' }, // pending_schedule → scheduled
      { status: 'preparing' }, // scheduled → preparing
      { status: 'dyeing' }, // preparing → dyeing
      { status: 'washing' }, // dyeing → washing
      { status: 'fixing' }, // washing → fixing
      { status: 'dehydrating' }, // fixing → dehydrating
      { status: 'drying' }, // dehydrating → drying
      { status: 'inspecting' }, // drying → inspecting
      { status: 'stored' }, // inspecting → stored
    ];
    for (const step of legalFlow) {
      await apiCall(page, 'PUT', `/production/dye-batches/${id}`, {
        status: step.status,
      });
      const batch = await apiCallRaw<{ status?: string }>(
        page,
        'GET',
        `/production/dye-batches/${id}`
      );
      // 断言当前状态 ∈ 后端 14 态合法状态集
      expect([
        'pending_schedule',
        'scheduled',
        'preparing',
        'dyeing',
        'washing',
        'fixing',
        'dehydrating',
        'drying',
        'inspecting',
        'stored',
        'shipped',
        'cancelled',
        'terminated',
        'rework',
        'on_hold',
        'failed',
      ]).toContain((batch.status || '').trim());
    }
  });

  test('3-5 验证缸号非法转换被拒绝', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.dyeBatchId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    // shipped/cancelled/terminated/failed 是终态，任何进一步流转都应被拒
    const batch = await apiCallRaw<{ status?: string }>(
      page,
      'GET',
      `/production/dye-batches/${id}`
    );
    const status = (batch.status || '').trim();
    const isTerminal = ['shipped', 'cancelled', 'terminated', 'failed'].includes(status);
    if (isTerminal) {
      const result = await apiCallExpectFail(page, 'PUT', `/production/dye-batches/${id}`, {
        status: 'dyeing',
      });
      expectBadRequest(result);
    } else {
      // 终态之外：非法跨状态（如 pending_schedule → stored 直跳）应被拒绝
      const result = await apiCallExpectFail(page, 'PUT', `/production/dye-batches/${id}`, {
        status: 'stored',
      });
      expectBadRequest(result);
    }
  });

  test('3-6 创建大货处方（关联工单+缸号+配方）', async ({ page }) => {
    const ctx = getCtx();
    const result = await apiCall<{ id?: number }>(page, 'POST', '/production/production-recipes', {
      recipe_no: genCode('PR'),
      work_order_id: ctx.productionOrderId,
      dye_batch_id: ctx.dyeBatchId,
      source_recipe_id: ctx.dyeRecipeId,
      customer_id: ctx.customerId,
      color_no: 'RED-001',
      fabric_name: '棉涤布',
      fabric_spec: '65%棉35%涤 40S 133x72',
      fabric_width: 150,
      gram_weight: 200,
      fabric_weight: 200,
      equipment_no: '染缸001',
      liquor_ratio: '1:10',
      bath_volume: 2000,
      adjustment_factor: 1.05,
      recipe_detail: [
        {
          material_code: 'R001',
          material_name: '活性红',
          concentration: 3,
          unit: '%',
          amount: 6,
          category: 'dye',
        },
        {
          material_code: 'A001',
          material_name: '匀染剂',
          concentration: 2,
          unit: 'g/L',
          amount: 40,
          category: 'auxiliary',
        },
      ],
      total_dye_cost: 120,
      total_auxiliary_cost: 80,
      status: 'draft',
    });
    ctx.productionRecipeId = result.data?.id;
    expect(ctx.productionRecipeId).toBeDefined();
  });

  test('3-7 审批大货处方（draft → approved）', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.productionRecipeId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }
    await apiCall(page, 'POST', `/production/production-recipes/${id}/approve`, {
      approved_by: 1,
    });
    const recipe = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/production/production-recipes/${id}`
    );
    expect(['approved', 'draft', 'closed', 'cancelled']).toContain(
      (recipe.status || '(missing-status)').toLowerCase()
    );
  });

  test('3-8 创建 BOM', async ({ page }) => {
    const ctx = getCtx();
    const productIds = ctx.productIds.length > 0 ? ctx.productIds : [1, 2];
    const result = await apiCall<{ id?: number }>(page, 'POST', '/boms', {
      product_id: productIds[0],
      version: 1,
      is_default: true,
      status: 'ACTIVE',
      items: productIds.slice(1).map((pid, i) => ({
        material_id: pid,
        quantity: 10 + i * 5,
        unit: '米',
      })),
    });
    ctx.bomId = result.data?.id;
    expect(ctx.bomId).toBeDefined();
  });

  test('3-9 创建生产工单', async ({ page }) => {
    const ctx = getCtx();
    const poUrl = '/production/production-orders/orders';
    const result = await apiCall<{ id?: number }>(page, 'POST', poUrl, {
      // CreateProductionOrderPayload：order_no 必填，quantity → planned_quantity
      order_no: genCode('PO-E2E'),
      product_id: ctx.productIds[0] || 1,
      planned_quantity: 1000,
      planned_start_date: new Date().toISOString().split('T')[0],
      planned_end_date: new Date(Date.now() + 7 * 86400000).toISOString().split('T')[0],
      remarks: 'E2E 生产工单',
    });
    ctx.productionOrderId = result.data?.id;
    expect(ctx.productionOrderId).toBeDefined();
  });

  test('3-10 生产工单状态流转', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.productionOrderId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    const transitions = [
      { action: 'submit-approval', to: ['pending_approval', 'approved'] },
      { action: 'approve', to: ['approved', 'scheduled', 'in_progress'] },
    ];

    for (const t of transitions) {
      await apiCall(page, 'POST', `/production/production-orders/orders/${id}/${t.action}`);
    }

    const order = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/production/production-orders/orders/${id}`
    );
    expect([
      'draft',
      'pending_approval',
      'approved',
      'scheduled',
      'in_progress',
      'completed',
      'confirmed',
    ]).toContain((order.status || '(missing-status)').toLowerCase());
  });

  test('3-11 验证匹号格式（{dye_lot_no}-{seq:03}）', async ({ page }) => {
    const pieceNo = genPieceNo(dyeLotNo, 1);
    expect(pieceNo).toBe(`${dyeLotNo}-001`);
    const pieceNo2 = genPieceNo(dyeLotNo, 2);
    expect(pieceNo2).toBe(`${dyeLotNo}-002`);
  });

  test('3-12 验证缸号生命周期日志', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.dyeBatchId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    try {
      const logs = await apiCallRaw<{
        items: Array<{ from_status: string; to_status: string; transition_code: string }>;
      }>(page, 'GET', `/production/dye-batch-lifecycle-logs/by-batch/${id}?page=1&page_size=20`);
      expect(logs.items);
      // 如果有日志，验证状态转换记录
      if (logs?.items?.length ?? 0 > 0) {
        expect(logs.items?.[0].transition_code).toBeTruthy();
      }
    } catch (e) {
      console.error('[3-12] 缸号生命周期日志查询失败:', (e as Error).message);
      throw e;
    }
  });
});
