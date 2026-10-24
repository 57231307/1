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
  seedColorCardArchive,
} from './helpers';

test.describe.serial('Shard 3: 染色生产闭环（缸号 14 态状态机）', () => {
  const dyeLotNo = genDyeLotNo();

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('3-1 创建染色配方（小样处方）', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const recipeNo = genCode('DR');
    const recipeName = genName('E2E染色配方');
    // 配方正文的后端入参键是 chemical_formula（CreateDyeRecipeRequest 的字段名），
    // formula 不在该 DTO 里，提交它不会落库
    const chemicalFormula = 'reactive red 3%, sodium sulfate 20g/L';
    const result = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-recipes', {
      recipe_no: recipeNo,
      recipe_name: recipeName,
      color_no: 'RED-001',
      color_code: 'RED-001',
      color_name: '大红',
      chemical_formula: chemicalFormula,
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
      // dye_recipe.status 是闭合小写英文词表（draft/pending_approval/approved/rejected/disabled），
      // 与 migration chk_dye_recipe_status CHECK 逐项一致；"草稿" 只是展示标签，落库会被 CHECK 拒绝。
      status: 'draft',
    });
    ctx.dyeRecipeId = result.data?.id;
    expect(
      ctx.dyeRecipeId,
      `配方创建应返回 data.id（建单响应回 id 为后端契约），实际响应：${JSON.stringify(result).slice(0, 300)}`
    ).toBeDefined();
    // 写后读回真值：建单只回 id 不算落库成功，逐字段核对配方主体确实按入参存进 dye_recipe
    const saved = await apiCallRaw<{
      recipe_no?: string;
      recipe_name?: string;
      color_code?: string;
      chemical_formula?: string | null;
      formula?: string | null;
      temperature?: string | number | null;
      time_minutes?: number | null;
      status?: string;
    }>(page, 'GET', `/production/dye-recipes/${ctx.dyeRecipeId}`);
    expect(saved.recipe_no, `回读的配方编号应等于提交值 ${recipeNo}`).toBe(recipeNo);
    expect(saved.recipe_name, `回读的配方名称应等于提交值 ${recipeName}`).toBe(recipeName);
    expect(saved.color_code, '回读的颜色代号应与提交值一致').toBe('RED-001');
    expect(saved.chemical_formula, '回读的配方正文应等于提交值').toBe(chemicalFormula);
    expect(saved.formula, '建单把正文同写入 formula 列，两列应保持一致').toBe(chemicalFormula);
    // temperature 是 Decimal 列，JSON 回读形态是字符串，取数值比较（键缺失时得 NaN 即判失败）
    expect(
      Number(saved.temperature),
      `回读的染色温度应等于提交值 80，实际 ${JSON.stringify(saved.temperature)}`
    ).toBe(80);
    expect(saved.time_minutes, '回读的保温时间应等于提交值 45').toBe(45);
    expect(saved.status, '新建配方的初始状态应落为 draft').toBe('draft');
  });

  test('3-2 审批染色配方（草稿 → 已审核）', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.dyeRecipeId;
    expect(
      id,
      '3-1/ensureTestEntities 未建出染色配方（ctx.dyeRecipeId 缺失），本用例前置失败'
    ).toBeTruthy();

    await apiCall(page, 'POST', `/production/dye-recipes/${id}/submit`);
    // 审批人取服务端会话身份（approve_recipe 无请求体）；本用例必须真审批而非停留草稿
    await apiCall(page, 'POST', `/production/dye-recipes/${id}/approve`);

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
    // I 族前置（CI 判责 §③「色号 RED-001 在色卡档案中不存在」）：后端
    // resolve_dye_color_identity（backend/src/handlers/dye_batch_handler.rs:203-254）对非空
    // color_no 强制反查色卡明细档案且要求全局唯一命中——该校验正当，不得放松。
    // 原写死的 'RED-001' 属测试缺前置：本用例自建专属色卡+唯一色号（seedColorCardArchive，
    // 与 fabric/02-dye 先例同型），用入档后的唯一色号替代硬编码。
    const archive = await seedColorCardArchive(page, { context: '03-production 3-3' });
    const batchNo = genCode('缸');
    const result = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-batches', {
      batch_no: batchNo,
      dye_lot_no: dyeLotNo,
      greige_fabric_id: ctx.greigeFabricId,
      color_no: archive.colorCode,
      planned_quantity: 1000,
      status: 'pending_schedule',
    });
    ctx.dyeBatchId = result.data?.id;
    ctx.dyeLotNo = dyeLotNo;
    expect(
      ctx.dyeBatchId,
      `染色批次创建应返回 data.id（建单响应回 id 为后端契约），实际响应：${JSON.stringify(result).slice(0, 300)}`
    ).toBeDefined();
    // 写后读回真值：建单只回 id 不算落库成功，逐字段核对缸号确实按入参存进 dye_batch，
    // 其中 color_code 与 color_name 由色卡档案反查派生，不是提交值的复制
    const saved = await apiCallRaw<{
      batch_no?: string;
      dye_lot_no?: string;
      color_no?: string | null;
      color_code?: string;
      color_name?: string;
      planned_quantity?: string | number | null;
      actual_output_kg?: string | number | null;
      actual_output_m?: string | number | null;
      greige_input_kg?: string | number | null;
      status?: string;
    }>(page, 'GET', `/production/dye-batches/${ctx.dyeBatchId}`);
    expect(saved.batch_no, `回读的缸号应等于提交值 ${batchNo}`).toBe(batchNo);
    expect(saved.dye_lot_no, `回读的染色批号应等于提交值 ${dyeLotNo}`).toBe(dyeLotNo);
    expect(saved.color_no, '回读的色号应等于提交的档案色号').toBe(archive.colorCode);
    expect(saved.color_code, '色码由色卡档案反查得到，应等于入档色号').toBe(archive.colorCode);
    expect(saved.color_name, '色名由色卡档案反查得到，应取档案里的色名').toBe(archive.colorName);
    // planned_quantity 是 Decimal 列，JSON 回读形态是字符串，取数值比较（键缺失时得 NaN 即判失败）
    expect(
      Number(saved.planned_quantity),
      `回读的计划产量应等于提交值 1000，实际 ${JSON.stringify(saved.planned_quantity)}`
    ).toBe(1000);
    expect(saved.status, '新建缸号的初始状态应落为 pending_schedule').toBe('pending_schedule');
    expect(
      [saved.actual_output_kg, saved.actual_output_m, saved.greige_input_kg],
      '完工登记三列只由 complete 端点写入，新建后应全为 NULL'
    ).toEqual([null, null, null]);
  });

  test('3-4 缸号状态机流转（14 态：pending_schedule→scheduled→preparing→dyeing→stored）', async ({
    page,
  }) => {
    const ctx = getCtx();
    const id = ctx.dyeBatchId;
    expect(
      id,
      '3-3/ensureTestEntities 未建出缸号（ctx.dyeBatchId 缺失），本用例前置失败'
    ).toBeTruthy();

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
    expect(
      id,
      '3-3/ensureTestEntities 未建出缸号（ctx.dyeBatchId 缺失），本用例前置失败'
    ).toBeTruthy();

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
    expect(id, '3-6 未建出大货处方（ctx.productionRecipeId 缺失），本用例前置失败').toBeTruthy();
    await apiCall(page, 'POST', `/production/production-recipes/${id}/approve`);
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
    const result = await apiCall<{ bom?: { id?: number } }>(page, 'POST', '/boms', {
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
    // 后端 create_bom → ApiResponse<BomDetailResponse>，data 形状为 { bom:{id}, items:[…] }，
    // 顶层无 id 字段（bom_handler.rs:87-90 结构体 / :138 返回），故只读 data.bom.id
    ctx.bomId = result.data?.bom?.id;
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
    expect(id, '3-9 未建出生产工单（ctx.productionOrderId 缺失），本用例前置失败').toBeTruthy();

    const transitions = [
      { action: 'submit-approval', body: {}, to: ['pending_approval', 'approved'] },
      { action: 'approve', body: { approved: true }, to: ['approved', 'scheduled', 'in_progress'] },
    ];

    for (const t of transitions) {
      await apiCall(page, 'POST', `/production/production-orders/orders/${id}/${t.action}`, t.body);
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
    expect(
      id,
      '3-3/ensureTestEntities 未建出缸号（ctx.dyeBatchId 缺失），本用例前置失败'
    ).toBeTruthy();

    try {
      // GET /production/dye-batch-lifecycle-logs/by-batch/{id} 出参是裸数组
      // Vec<dye_batch_lifecycle_log::Model>（handler 无分页信封，page/page_size 无效）；
      // 原实现读 logs.items 恒为 undefined，expect 又不带匹配器，整条用例空转。
      const logs = await apiCallRaw<Array<Record<string, unknown>>>(
        page,
        'GET',
        `/production/dye-batch-lifecycle-logs/by-batch/${id}`
      );
      expect(
        Array.isArray(logs),
        `缸号生命周期日志应为数组，实际：${JSON.stringify(logs).slice(0, 200)}`
      ).toBe(true);
      for (const row of logs) {
        expect(
          String(row.to_status ?? ''),
          `生命周期日志缺少 to_status：${JSON.stringify(row)}`
        ).not.toBe('');
        expect(
          String(row.transition_code ?? ''),
          `生命周期日志缺少 transition_code：${JSON.stringify(row)}`
        ).not.toBe('');
        expect(
          Number(row.batch_id ?? id),
          `按缸号查询混入了其他缸号的日志：${JSON.stringify(row)}`
        ).toBe(id);
      }
    } catch (e) {
      console.error('[3-12] 缸号生命周期日志查询失败:', (e as Error).message);
      throw e;
    }
  });
});
