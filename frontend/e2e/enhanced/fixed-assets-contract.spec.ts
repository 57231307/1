// 固定资产契约面 — enhanced/fixed-assets-contract
// 功能（业务规则面，逐端点对后端源码核实后编写）：
//   1) 减值测试族：POST /fixed-assets/impairment-tests 由服务端计算减值损失
//      （可收回低于账面取差额，否则恒 0，绝不出现负数；判据在
//      backend/src/services/fixed_asset_service.rs::create_impairment_test 计算段）；
//      GET 按 asset_id 过滤回读；PUT 审批跃迁断言状态列与审批留痕列同源
//      （status、reviewed_by、reviewed_at 三列一次写入）。
//   2) 折旧政策变更族：同型生命周期（pending → approved，approved_by 与 approved_at
//      留痕），GET 按 asset_id 过滤，重复审批被逐字文案拒绝。
//   3) POST /fixed-assets/batch-depreciate 是计算视图不是写库通道：直线法
//      月折旧额 (原值-残值)/(年限*12) 保留两位、按购置月数封顶、减已计提取非负
//      （backend/src/services/fixed_asset_service.rs::calculate_asset_depreciation），
//      本轮资产原值 120000、年限 10、购置 2026-01-01、计算日 2026-07-31，
//      应计 6 个月共 6000；断言逐值相等且资产行前后零漂移。
//   4) 盘点计划族与列表读回：GET /fixed-assets/count-plans 的 data 为
//      「行数组与总数」二元组（handler 直出 tuple），本轮计划以唯一资产类别过滤，
//      total_items 必等于该类别自建资产数 1；停用类别过滤不产生他人明细行。
//   5) 打印族 GET /fixed-assets/{id}/print 与 GET /fixed-assets/count/{id}/print 出真件：
//      content-type 为 docx MIME、zip 魔数 PK 开头、attachment 文件名带 .docx。
// 调用方：CI e2e 分片 36（extras enhanced+production）。
// 入参：apiCall、apiCallRaw、apiCallExpectFail 与 page.request（二进制出件走直连）；
//   资产、计划、变更、减值全部自建带唯一后缀。
// 传给谁：断言按本轮主键精确回读。
// 存什么：asset_impairment_tests 的 carrying、recoverable、impairment_loss、status、
//   reviewed_by、reviewed_at；depreciation_policy_changes 的 old/new 方法、status、
//   approved_by、approved_at；fixed_asset_count 的 plan_name、status、total_items、count_no。
// 存哪里：表 asset_impairment_tests、depreciation_policy_changes、fixed_asset_count、
//   fixed_assets。
// 残量登记（不许算已覆盖）：POST /fixed-assets/auto-depreciation 为全库扫描计提通道
//   （backend/src/services/fixed_asset_service.rs::auto_monthly_depreciation 对全部
//   active 资产逐个 depreciate，period 是自由字符串、无资产范围参数、卫兵在写入之后），
//   在共享库上任何调用都会改写他人资产累计折旧列，无法隔离自测，需产品给范围参数口径
//   或测试库独占后才可覆盖；batch-depreciate 与折旧族的「真实落库腿」不在本面（本面钉的是
//   计算视图契约）。
import { test, expect } from '../diagnose-fixture';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  tryCleanup,
  failureCode,
  APP_ERROR_CODES,
  API_BASE,
  API_PREFIX,
} from '../flow/helpers';

interface AssetRow {
  id: number;
  asset_no: string;
  asset_name: string;
  asset_category: string | null;
  original_value: string | number;
  accumulated_depreciation: string | number;
  net_value: string | number | null;
  useful_life: number | null;
  depreciation_method: string | null;
  purchase_date: string | null;
  status: string;
}

interface ImpairmentTestRow {
  id: number;
  asset_id: number;
  test_date: string;
  carrying_amount: string | number;
  recoverable_amount: string | number;
  impairment_loss: string | number;
  test_basis: string;
  status: string;
  created_by: number;
  reviewed_by: number | null;
  reviewed_at: string | null;
}

interface PolicyChangeRow {
  id: number;
  asset_id: number;
  old_method: string;
  new_method: string;
  change_date: string;
  reason: string;
  status: string;
  created_by: number;
  approved_by: number | null;
  approved_at: string | null;
}

interface DepreciationResultRow {
  asset_id: number;
  asset_no: string;
  original_value: string | number;
  accumulated_depreciation: string | number;
  current_depreciation: string | number;
  net_value: string | number;
  depreciation_method: string;
}

/**
 * 功能：自建一台直线法资产（原值 120000、年限 10、购置 2026-01-01），折旧对账的唯一基准行。
 * 调用方：本文件各用例。入参：page 与本轮唯一后缀 tag。
 * 传给谁：返回 id 进减值、变更、批量计算与打印端点。
 * 存什么：fixed_assets 一行（active、累计折旧 0、净值等于原值）。
 * 存哪里：表 fixed_assets，编号取唯一字面量防并发重号（该列无表级唯一约束）。
 */
async function seedStraightLineAsset(
  page: import('@playwright/test').Page,
  tag: string
): Promise<AssetRow> {
  const created = await apiCall<AssetRow>(page, 'POST', '/fixed-assets', {
    asset_no: `E2EFA${tag}`,
    asset_name: `契约资产${tag}`,
    asset_category: `契约类别${tag}`,
    location: `契约地点${tag}`,
    original_value: '120000',
    useful_life: 10,
    depreciation_method: 'straight_line',
    purchase_date: '2026-01-01',
  });
  const asset = created.data;
  expect(asset, `自建资产失败：${JSON.stringify(created)}`).toBeTruthy();
  expect(Number(asset!.original_value), '资产原值必须按提交落库').toBe(120000);
  expect(Number(asset!.accumulated_depreciation), '新建资产累计折旧必须为 0').toBe(0);
  return asset!;
}

test.describe('固定资产 - 减值变更折旧与出件契约面', () => {
  test('减值测试族 - 服务端算差额、审批留痕同源与拒绝腿零落库', async ({ page }) => {
    const tag = genCode('FAS');
    const asset = await seedStraightLineAsset(page, tag);
    const userId = (await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me')).id;
    try {
      const lossy = await apiCall<ImpairmentTestRow>(
        page,
        'POST',
        '/fixed-assets/impairment-tests',
        {
          asset_id: asset.id,
          test_date: '2026-06-30',
          carrying_amount: '500',
          recoverable_amount: '300',
          test_basis: `可收回低于账面${tag}`,
        }
      );
      expect(
        Number(lossy.data!.impairment_loss),
        `减值额必须等于 500-300=200，实际 ${lossy.data!.impairment_loss}`
      ).toBe(200);
      expect(lossy.data!.status, '新建减值行必须落 pending').toBe('pending');
      expect(lossy.data!.created_by, 'created_by 必须为真实操作人').toBe(userId);

      const noLoss = await apiCall<ImpairmentTestRow>(
        page,
        'POST',
        '/fixed-assets/impairment-tests',
        {
          asset_id: asset.id,
          test_date: '2026-06-30',
          carrying_amount: '400',
          recoverable_amount: '600',
          test_basis: `可收回高于账面${tag}`,
        }
      );
      expect(
        Number(noLoss.data!.impairment_loss),
        `可收回高于账面时减值必须为 0 而非负数，实际 ${noLoss.data!.impairment_loss}`
      ).toBe(0);

      const list = await apiCallRaw<ImpairmentTestRow[]>(
        page,
        'GET',
        `/fixed-assets/impairment-tests/${asset.id}`
      );
      expect(Array.isArray(list), '减值列表必须为数组').toBe(true);
      const lossRow = list.find(r => r.id === lossy.data!.id);
      expect(lossRow, 'GET 按 asset_id 必须回读到本轮损失行').toBeTruthy();
      expect(Number(lossRow!.impairment_loss)).toBe(200);
      expect(lossRow!.status).toBe('pending');
      expect(lossRow!.test_basis, 'test_basis 必须逐字回读').toBe(`可收回低于账面${tag}`);

      const approved = await apiCall<ImpairmentTestRow>(
        page,
        'PUT',
        `/fixed-assets/impairment-tests/${lossy.data!.id}/approve`
      );
      expect(approved.data!.status, '审批后状态列翻为 approved').toBe('approved');
      expect(approved.data!.reviewed_by, '留痕列与审批人同源').toBe(userId);
      expect(approved.data!.reviewed_at, '留痕时间列必须同次写入非空').not.toBeNull();

      const listAfter = await apiCallRaw<ImpairmentTestRow[]>(
        page,
        'GET',
        `/fixed-assets/impairment-tests/${asset.id}`
      );
      const rowAfter = listAfter.find(r => r.id === lossy.data!.id)!;
      expect(rowAfter.status, '列表回读与审批响应同源').toBe('approved');

      const reApprove = await apiCallExpectFail(
        page,
        'PUT',
        `/fixed-assets/impairment-tests/${lossy.data!.id}/approve`
      );
      expect(reApprove.status, '重复审批必须 400').toBe(400);
      expect(failureCode(reApprove)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
      expect(reApprove.message, '重复审批文案必须逐字').toBe('只有待审批状态的记录才能审批');

      const countBefore = (
        await apiCallRaw<ImpairmentTestRow[]>(
          page,
          'GET',
          `/fixed-assets/impairment-tests/${asset.id}`
        )
      ).length;
      const badCreate = await apiCallExpectFail(page, 'POST', '/fixed-assets/impairment-tests', {
        asset_id: asset.id,
        test_date: '2026-06-30',
        carrying_amount: '1',
        recoverable_amount: '1',
        test_basis: '',
      });
      expect(badCreate.status).toBe(400);
      expect(failureCode(badCreate)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
      expect(badCreate.message).toBe('测试依据不能为空');
      const countAfter = (
        await apiCallRaw<ImpairmentTestRow[]>(
          page,
          'GET',
          `/fixed-assets/impairment-tests/${asset.id}`
        )
      ).length;
      expect(countAfter, '被拒的减值行不得落库').toBe(countBefore);
    } finally {
      await tryCleanup(page, 'DELETE', `/fixed-assets/${asset.id}`, 'asset');
    }
  });

  test('折旧政策变更族 - 资产维度过滤与审批跃迁及越界资产空列表', async ({ page }) => {
    const tag = genCode('FAP');
    const asset = await seedStraightLineAsset(page, tag);
    const other = await seedStraightLineAsset(page, `${tag}O`);
    const userId = (await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me')).id;
    try {
      const change = await apiCall<PolicyChangeRow>(
        page,
        'POST',
        '/fixed-assets/depreciation-policy-changes',
        {
          asset_id: asset.id,
          change_date: '2026-07-01',
          old_method: 'straight_line',
          new_method: 'declining_balance',
          old_useful_life: 10,
          new_useful_life: 8,
          reason: `契约变更原因${tag}`,
        }
      );
      expect(change.message, '创建信封文案必须逐字').toBe('折旧政策变更创建成功');
      expect(change.data!.status, '新建变更行必须落 pending').toBe('pending');

      const mine = await apiCallRaw<PolicyChangeRow[]>(
        page,
        'GET',
        `/fixed-assets/depreciation-policy-changes/${asset.id}`
      );
      expect(mine.length, '变更列表应恰含本轮 1 行').toBe(1);
      expect(mine[0].id).toBe(change.data!.id);
      expect(mine[0].old_method).toBe('straight_line');
      expect(mine[0].new_method, '新旧方法必须逐字回读').toBe('declining_balance');
      expect(mine[0].reason).toBe(`契约变更原因${tag}`);

      const otherList = await apiCallRaw<PolicyChangeRow[]>(
        page,
        'GET',
        `/fixed-assets/depreciation-policy-changes/${other.id}`
      );
      expect(otherList.length, 'asset_id 过滤失效会把变更行混入他机列表').toBe(0);

      const approved = await apiCall<PolicyChangeRow>(
        page,
        'PUT',
        `/fixed-assets/depreciation-policy-changes/${change.data!.id}/approve`
      );
      expect(approved.data!.status).toBe('approved');
      expect(approved.data!.approved_by, '审批留痕与操作人同源').toBe(userId);
      expect(approved.data!.approved_at, '跃迁列与留痕列必须同次写入').not.toBeNull();

      const reApprove = await apiCallExpectFail(
        page,
        'PUT',
        `/fixed-assets/depreciation-policy-changes/${change.data!.id}/approve`
      );
      expect(reApprove.status).toBe(400);
      expect(reApprove.message).toBe('只有待审批状态的记录才能审批');

      const badCreate = await apiCallExpectFail(
        page,
        'POST',
        '/fixed-assets/depreciation-policy-changes',
        {
          asset_id: asset.id,
          change_date: '2026-07-01',
          old_method: '',
          new_method: 'declining_balance',
          reason: `空方法${tag}`,
        }
      );
      expect(badCreate.status).toBe(400);
      expect(badCreate.message, '原方法必填文案必须逐字').toBe('原折旧方法不能为空');
      const listAfterBad = await apiCallRaw<PolicyChangeRow[]>(
        page,
        'GET',
        `/fixed-assets/depreciation-policy-changes/${asset.id}`
      );
      expect(listAfterBad.length, '被拒的变更行不得落库').toBe(1);
    } finally {
      await tryCleanup(page, 'DELETE', `/fixed-assets/${asset.id}`, 'asset');
      await tryCleanup(page, 'DELETE', `/fixed-assets/${other.id}`, 'asset-other');
    }
  });

  test('批量折旧计算视图 - 直线法逐值对账且资产行零漂移', async ({ page }) => {
    const tag = genCode('FAB');
    const asset = await seedStraightLineAsset(page, tag);
    try {
      const calc = await apiCallRaw<DepreciationResultRow[]>(
        page,
        'POST',
        '/fixed-assets/batch-depreciate',
        {
          asset_ids: [asset.id],
          calculation_date: '2026-07-31',
        }
      );
      expect(calc.length, '单资产入参应得单行结果').toBe(1);
      const row = calc[0];
      expect(row.asset_id).toBe(asset.id);
      expect(row.asset_no, '资产编号必须逐字回显').toBe(`E2EFA${tag}`);
      expect(
        Number(row.current_depreciation),
        `月折旧 1000 计 6 个月应得 6000，实际 ${row.current_depreciation}`
      ).toBe(6000);
      expect(Number(row.accumulated_depreciation), '响应累计列为已计提加本次应计').toBe(6000);
      expect(Number(row.net_value), '净值列必须等于原值减应计').toBe(114000);
      expect(row.depreciation_method).toBe('straight_line');

      // 计算视图不写库：资产行的累计与净值保持基线
      const after = await apiCallRaw<AssetRow>(page, 'GET', `/fixed-assets/${asset.id}`);
      expect(Number(after.accumulated_depreciation), '批量计算不得回写资产累计折旧列').toBe(0);
      expect(Number(after.net_value), '批量计算不得回写净值列').toBe(120000);

      const empty = await apiCallRaw<DepreciationResultRow[]>(
        page,
        'POST',
        '/fixed-assets/batch-depreciate',
        {
          asset_ids: [],
          calculation_date: '2026-07-31',
        }
      );
      expect(empty.length, '空 id 列表必须短路为空结果而非全表').toBe(0);

      const ghost = await apiCallExpectFail(page, 'POST', '/fixed-assets/batch-depreciate', {
        asset_ids: [2147000998],
        calculation_date: '2026-07-31',
      });
      expect(ghost.status, '幽灵资产必须 404 而非静默跳过').toBe(404);
      expect(failureCode(ghost)).toBe(APP_ERROR_CODES.NOT_FOUND);

      const badDate = await apiCallExpectFail(page, 'POST', '/fixed-assets/batch-depreciate', {
        asset_ids: [asset.id],
        calculation_date: 'not-a-date',
      });
      expect(badDate.status).toBe(400);
      expect(failureCode(badDate)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
      expect(badDate.message, '日期族校验外显必须为脱敏常量').toBe('请求参数验证失败');
    } finally {
      await tryCleanup(page, 'DELETE', `/fixed-assets/${asset.id}`, 'asset');
    }
  });

  test('盘点计划列表与打印出件 - 二元组信封、类别过滤计数与 docx 真件', async ({ page }) => {
    const tag = genCode('FAC');
    const asset = await seedStraightLineAsset(page, tag);
    try {
      const plan = await apiCall<{
        id: number;
        count_no: string;
        plan_name: string;
        status: string;
        total_items: number;
        counted_items: number;
      }>(page, 'POST', '/fixed-assets/count-plans', {
        plan_name: `契约盘点${tag}`,
        asset_category: `契约类别${tag}`,
        count_date: '2026-08-01',
      });
      expect(plan.data!.status, '新建计划必须为 DRAFT').toBe('DRAFT');
      expect(plan.data!.total_items, '计划明细数必须等于该唯一类别下的资产数（本轮 1 台）').toBe(1);
      expect(plan.data!.counted_items, '未开始盘点时进度列必须为 0').toBe(0);

      const listed = await apiCallRaw<
        [Array<{ id: number; plan_name: string; status: string; total_items: number }>, number]
      >(page, 'GET', '/fixed-assets/count-plans?page=1&page_size=100');
      expect(
        Array.isArray(listed) && listed.length === 2,
        '列表 data 必须是「行数组与总数」二元组'
      ).toBe(true);
      const rows = listed[0];
      expect(Array.isArray(rows)).toBe(true);
      const total = listed[1];
      expect(typeof total, '总数必须为数字').toBe('number');
      const hit = rows.find(r => r.id === plan.data!.id);
      expect(hit, 'GET count-plans 必须回读到本轮计划').toBeTruthy();
      expect(hit!.plan_name).toBe(`契约盘点${tag}`);
      expect(hit!.status).toBe('DRAFT');
      expect(hit!.total_items).toBe(1);
      expect(total, '信封总数不得小于已回读页内命中').toBeGreaterThanOrEqual(1);

      // 资产打印出件：真 docx（zip 魔数 + 出件 MIME + attachment 文件名）
      const assetPrint = await page.request.get(
        `${API_BASE}${API_PREFIX}/fixed-assets/${asset.id}/print`
      );
      expect(assetPrint.status(), `资产打印必须 200，实际 ${assetPrint.status()}`).toBe(200);
      expect(assetPrint.headers()['content-type'], '资产打印必须回 docx MIME').toContain(
        'wordprocessingml.document'
      );
      const assetBytes = await assetPrint.body();
      expect(assetBytes[0], 'docx 必须是 zip 容器（PK 魔数首位）').toBe(0x50);
      expect(assetBytes[1], 'docx 必须是 zip 容器（PK 魔数次位）').toBe(0x4b);
      expect(assetPrint.headers()['content-disposition'], '出件文件名必须带 .docx').toContain(
        '.docx'
      );

      const countPrint = await page.request.get(
        `${API_BASE}${API_PREFIX}/fixed-assets/count/${plan.data!.id}/print`
      );
      expect(countPrint.status(), `盘点单打印必须 200，实际 ${countPrint.status()}`).toBe(200);
      expect(countPrint.headers()['content-type']).toContain('wordprocessingml.document');
      const countBytes = await countPrint.body();
      expect(countBytes[0]).toBe(0x50);
      expect(countBytes[1]).toBe(0x4b);
    } finally {
      await tryCleanup(page, 'DELETE', `/fixed-assets/${asset.id}`, 'asset');
    }
  });
});
