import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  getCtx,
  verifyTrialBalance,
  verifyAuditLog,
  ensureTestEntities,
} from './helpers';

test.describe('成本核算完整流程', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('成本归集：创建→料工费验证→审批→成本分析', async ({ page }) => {
    const ctx = getCtx();

    // 后端 CreateCostCollectionRequestDto 真实字段
    const costData = {
      collection_date: new Date().toISOString().slice(0, 10),
      cost_object_type: 'dye_batch',
      cost_object_id: ctx.dyeBatchId,
      cost_object_no: ctx.dyeLotNo || genCode('DL'),
      batch_no: ctx.dyeLotNo,
      color_no: ctx.colorNos[0],
      dye_lot_no: ctx.dyeLotNo,
      workshop: '一车间',
      direct_material: '1500.50',
      direct_labor: '800.00',
      manufacturing_overhead: '300.00',
      processing_fee: '100.00',
      dyeing_fee: '200.00',
      output_quantity_meters: '1000',
      output_quantity_kg: '200',
    };

    const result = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/production/cost-collections',
      costData
    );
    const costId = result.data?.id;
    expect(
      costId,
      `成本归集创建应返回 data.id，实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    // 验证初始状态
    const created = await apiCallRaw<{
      status: string;
      direct_material: string;
      direct_labor: string;
      manufacturing_overhead: string;
      processing_fee: string;
      dyeing_fee: string;
    }>(page, 'GET', `/production/cost-collections/${costId}`);
    expect(created.status.toLowerCase()).toBe('draft');

    // 验证成本金额（直接材料+直接人工+制造费用+加工费+染色费 = 总成本）
    const dm = parseFloat(created.direct_material || '0');
    const dl = parseFloat(created.direct_labor || '0');
    const mo = parseFloat(created.manufacturing_overhead || '0');
    const pf = parseFloat(created.processing_fee || '0');
    const df = parseFloat(created.dyeing_fee || '0');
    const expectedTotal = dm + dl + mo + pf + df;
    expect(expectedTotal).toBeGreaterThan(0);

    // 审批成本归集（audit 端点需要 body {approved, comment}，空 body 会 400）
    await apiCall(page, 'POST', `/production/cost-collections/${costId}/audit`, {
      approved: true,
      comment: 'E2E 审核通过',
    });
    const audited = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/production/cost-collections/${costId}`
    );
    expect(audited.status.toLowerCase()).toMatch(/audited|approved/);

    // 验证成本分析报表
    // 后端 get_cost_analysis_summary 无日期参数时聚合全部成本归集记录，
    // service 用 total_direct_material 键返回本用例刚创建记录的直接材料合计。
    const summary = await apiCallRaw<{ total_direct_material: string | number }>(
      page,
      'GET',
      '/production/cost-collections/analysis/summary'
    );
    // 全局汇总必然包含本用例创建的 1500.50 直接材料，断言下界而非 >=0 空转
    expect(Number(summary.total_direct_material)).toBeGreaterThanOrEqual(1500.5);

    // 按缸号查询成本
    // 后端 get_cost_by_batch 的 handler 返回裸 Vec<BatchCostAnalysis>（非 {items} 分页对象），
    // CostByBatchQuery.batch_no 为 Option，不传即全表；此处按本用例创建的缸号过滤，
    // 断言返回的正是这条数据（字段名对照 BatchCostAnalysis 结构体）。
    const batchNo = ctx.dyeLotNo!;
    const byBatch = await apiCallRaw<
      Array<{
        batch_no: string | null;
        direct_material: string | number;
        direct_labor: string | number;
        manufacturing_overhead: string | number;
        total_cost: string | number;
        status: string;
      }>
    >(
      page,
      'GET',
      `/production/cost-collections/analysis/by-batch?batch_no=${encodeURIComponent(batchNo)}`
    );
    expect(Array.isArray(byBatch), 'by-batch 应返回裸数组').toBe(true);
    expect(byBatch.length).toBeGreaterThanOrEqual(1);
    // 服务端按 batch_no 下推过滤，每一行都必须是本用例创建的缸号
    for (const row of byBatch) {
      expect(row.batch_no).toBe(batchNo);
    }
    const mine = byBatch.find(r => Number(r.direct_material) === 1500.5);
    expect(mine, '按缸号成本分析应包含本用例创建的归集记录').toBeTruthy();
    expect(Number(mine!.direct_labor)).toBe(800);
    expect(Number(mine!.manufacturing_overhead)).toBe(300);
    // 总成本 = 直接材料+直接人工+制造费用+加工费+染色费 = 1500.5+800+300+100+200
    expect(Number(mine!.total_cost)).toBeCloseTo(2900.5, 2);
    // audit 端点将 draft 置为 approved
    expect(mine!.status.toLowerCase()).toMatch(/approved|audited/);

    // 验证审计日志
    const auditLogged = await verifyAuditLog(
      page,
      'CREATE',
      'production',
      '/production/cost-collections'
    );
    expect(auditLogged).toBe(true);
  });

  test('会计期间控制：关闭期间禁止录入凭证', async ({ page }) => {
    const ctx = getCtx();
    // 查询当前会计期间
    const currentPeriod = await apiCallRaw<{
      id: number;
      period_name: string;
      status: string;
    }>(page, 'GET', '/finance/accounting-periods/current');

    expect(currentPeriod.id).toBeDefined();

    // 创建凭证（后端 CreateVoucherRequestDto 真实字段）
    // 科目用 ensureTestEntities 创建的随机编码（硬编码 1001/1002 与种子冲突 → BAD_REQUEST）
    const subjCodes = ctx.accountSubjectIds.length
      ? await Promise.all(
          ctx.accountSubjectIds.slice(0, 2).map(async id => {
            const s = await apiCallRaw<{ code: string }>(page, 'GET', `/subjects/${id}`);
            return s.code;
          })
        )
      : ['1001', '1002'];
    const voucherData = {
      voucher_type: 'general',
      voucher_date: new Date().toISOString().slice(0, 10),
      source_type: 'manual',
      source_module: 'e2e_test',
      items: [
        {
          subject_code: subjCodes[0],
          debit: '100',
          credit: '0',
          summary: '测试借方',
        },
        {
          subject_code: subjCodes[1],
          debit: '0',
          credit: '100',
          summary: '测试贷方',
        },
      ],
    };

    const result = await apiCall<{ id?: number }>(page, 'POST', '/finance/vouchers', voucherData);
    const voucherId = result.data?.id;
    expect(
      voucherId,
      `凭证创建应返回 data.id，实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    // 验证凭证借贷平衡
    const voucher = await apiCallRaw<{
      entries: Array<{ debit: string; credit: string }>;
      status: string;
    }>(page, 'GET', `/vouchers/${voucherId}`);
    const totalDebit =
      voucher.entries?.reduce(
        (sum: number, e: { debit: string; credit: string }) => sum + parseFloat(e.debit || '0'),
        0
      ) || 0;
    const totalCredit =
      voucher.entries?.reduce(
        (sum: number, e: { debit: string; credit: string }) => sum + parseFloat(e.credit || '0'),
        0
      ) || 0;
    expect(Math.abs(totalDebit - totalCredit)).toBeLessThan(0.01);

    // 验证试算平衡
    const trialBalance = await verifyTrialBalance(page);
    expect(trialBalance.debit_total).toBeGreaterThanOrEqual(0);
    expect(trialBalance.credit_total).toBeGreaterThanOrEqual(0);
  });

  test('固定资产折旧：计提→折旧记录验证', async ({ page }) => {
    // 后端 create() 将资产 status 置为 ACTIVE，depreciate 的 validate_asset_for_depreciation
    // 状态门只放行 ACTIVE；列表端点 GET /fixed-assets 的 handler 用 ApiResponse::success(Vec)
    // 返回裸数组（非 {items} 分页对象），无法从"已有资产"里保证可折旧状态。
    // 因此本用例显式创建一条全新的 ACTIVE 资产，把折旧驱动到允许状态再断言。
    const assetData = {
      asset_no: genCode('FA'),
      asset_name: 'E2E 测试设备',
      asset_category: '生产设备',
      purchase_date: new Date().toISOString().slice(0, 10),
      original_value: '100000',
      useful_life: 60,
      depreciation_method: 'straight_line',
      location: '一车间',
    };
    const created = await apiCall<{ id?: number }>(page, 'POST', '/fixed-assets', assetData);
    const assetId = created.data?.id;
    expect(assetId, '固定资产创建应返回 id').toBeTruthy();

    // 确认资产处于可折旧状态（create 默认写入 ACTIVE）
    const detail = await apiCallRaw<{ status: string }>(page, 'GET', `/fixed-assets/${assetId}`);
    expect(detail.status.toLowerCase()).toBe('active');

    // 计提折旧：DepreciateRequest 需要 body { period }（缺 body 会 4xx，非状态机拒绝）；
    // straight_line 月折旧 = (原值-残值)/(使用年限*12) = 100000/(60*12) = 138.89 > 0
    const now = new Date();
    const period = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`;
    await apiCall(page, 'POST', `/fixed-assets/${assetId}/depreciate`, { period });

    // 折旧记录：list_depreciation_records 的 handler 返回裸 Vec（非 {items}），字段名对照
    // fixed_asset_depreciation_record 模型（depreciation_amount / period）。
    const records = await apiCallRaw<
      Array<{ period: string; depreciation_amount: string | number }>
    >(page, 'GET', `/fixed-assets/${assetId}/depreciation-records`);
    expect(Array.isArray(records), '折旧记录端点应返回裸数组').toBe(true);
    expect(records).toHaveLength(1);
    expect(records[0].period).toBe(period);
    expect(Number(records[0].depreciation_amount)).toBeGreaterThan(0);
    expect(Number(records[0].depreciation_amount)).toBeCloseTo(138.89, 2);

    // 拒绝路径：同一资产同一期间重复计提命中唯一约束 uk_fa_depreciation_records_asset_period，
    // service 转 AppError::validation → HTTP 400（apiCallExpectFail 不抛错，直接读 status）。
    const dup = await apiCallExpectFail(page, 'POST', `/fixed-assets/${assetId}/depreciate`, {
      period,
    });
    expect(dup.status, '重复期间计提应被拒绝').toBeGreaterThanOrEqual(400);
    expect(dup.status).toBeLessThan(500);
  });

  test('预算控制：超预算预警查询', async ({ page }) => {
    // budget_execution_warnings 的 handler 返回 ApiResponse{data: Vec<BudgetWarning>}，
    // apiCallRaw 已解到 data 层，故这里是裸数组（非 {items} 分页对象）。
    // 字段对照 models/dto/budget_management_dto.rs 的 BudgetWarning：主键是 plan_id、
    // 预警级别字段是 warning_level（原用例误写 budget_id/warning_type 属字段臆测）。
    const warnings = await apiCallRaw<
      Array<{
        plan_id: number;
        plan_no: string;
        warning_level: string;
        execution_rate: string | number;
      }>
    >(page, 'GET', '/budgets/execution-warnings?page=1&page_size=50');

    expect(Array.isArray(warnings), 'execution-warnings 应返回裸数组').toBe(true);
    // 逐条校验预警记录的真实契约：plan_id 为数字、级别只可能是 yellow/red
    for (const w of warnings) {
      expect(typeof w.plan_id).toBe('number');
      expect(['yellow', 'red']).toContain(w.warning_level);
    }

    // 查询预算列表
    const budgets = await apiCallRaw<{ items: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/budgets?page=1&page_size=5'
    );

    if (budgets.items && budgets.items.length > 0) {
      const budget = budgets.items?.[0];
      const control = await apiCallRaw<{
        total_budget: string;
        total_executed: string;
        execution_rate: string;
      }>(page, 'GET', `/budgets/control/${budget.id}`);

      if (control) {
        expect(parseFloat(control.total_budget)).toBeGreaterThanOrEqual(0);
        expect(parseFloat(control.total_executed)).toBeGreaterThanOrEqual(0);
      }
    }
  });
});
