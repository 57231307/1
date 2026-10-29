// 财务分析契约面 — enhanced/financial-analysis-contract
// 功能（业务规则面，逐端点对后端源码核实后编写）：
//   1) POST indicators 字段校验与默认值：名称长度经 validator 逐字外显；缺 code 由服务层
//      自动生成（backend/src/handlers/financial_analysis_handler.rs::create_indicator 的
//      indicator_code 兜底段）；被拒请求必须零落库（用唯一 indicator_type 反查为 0 证伪）。
//   2) POST trends 派生量：variance=value-target、variance_rate=(value-target)/target*100、
//      trend 方向词表（backend/src/services/financial_analysis_service.rs::
//      create_analysis_result 计算段），断言逐值相等而非仅可达。
//   3) GET trends 的 period 精确过滤与 indicator_id 必填：缺参走可外显校验族，文案逐字
//      「财务分析请求缺少指标ID」。
//   4) GET trend-analysis 线性回归与移动平均：全等值序列的 slope 必为 0、r_squared 必为 0
//      （ss_yy 为零分支）、ma3 末位为均值、trend_direction 落「平稳」（同文件
//      get_trend_analysis 计算段）。
//   5) 报告族（复用 financial_analysis 表）：POST reports 把期间与关联指标与描述拼进 remark；
//      PUT 部分更新必须保留「期间」「关联指标」段仅替换「描述」段
//      （backend/src/handlers/financial_analysis_handler.rs::upsert_remark_description）；
//      DELETE 后 GET 与二次 DELETE 均 404。
//   6) POST reports/{id}/execute 的非预定义指标透明回语：status 必为 no_data、message 逐字、
//      period 回显提交值（缺省才会落当前年月，回显即证明参数传播）。
//   7) POST cash-flow-ratios 零余额期间：响应 self-consistent（total 等于 indicators 行数），
//      存在行时分子为 0 故 indicator_value 必为 0，target 落在三个常量之一。
// 调用方：CI e2e 分片 36（extras enhanced+production）。
// 入参：apiCall、apiCallRaw、apiCallExpectFail 真实后端真实库；期间一律用 2099 与 1999 的
//   专属窗，与运行期业务数据正交。
// 传给谁：断言仅按本轮自建 indicator_id 与唯一 indicator_type 精确回读。
// 存什么：financial_analysis 的名称、编码、类型、单位、remark 拼装段、status；
//   financial_analysis_results 的 value、target、variance、variance_rate、trend、period。
// 存哪里：表 financial_analysis、financial_analysis_results。
// 残量登记：cash-flow-ratios 的非零分子路径需要该期间真实现金流量科目余额，HTTP 面无法
//   在隔离窗内造出，只断言零余额分支；execute 的 completed 分支同理（需预定义指标在有数期间）。
import { test, expect } from '../diagnose-fixture';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  tryCleanup,
  failureCode,
  APP_ERROR_CODES,
} from '../flow/helpers';

interface FaIndicatorRow {
  id: number;
  indicator_name: string;
  indicator_code: string;
  indicator_type: string;
  unit: string | null;
  status: string;
  remark: string | null;
}

interface FaTrendRow {
  id: number;
  analysis_type: string;
  period: string;
  indicator_id: number;
  indicator_value: string | number;
  target_value: string | number | null;
  variance: string | number | null;
  variance_rate: string | number | null;
  trend: string | null;
}

test.describe('财务分析 - 指标趋势报告契约面', () => {
  test('指标创建校验 - 逐字文案与缺省编码兜底和被拒零落库', async ({ page }) => {
    const tag = genCode('FAI');
    const created = await apiCall<FaIndicatorRow>(page, 'POST', '/financial-analysis/indicators', {
      name: `契约指标${tag}`,
      indicator_type: `e2e_fa_${tag}`,
      unit: '倍',
      description: `单位校验样本${tag}`,
    });
    const indicatorId = Number(created.data?.id);
    expect(indicatorId, `创建指标未返回 id：${JSON.stringify(created.data)}`).toBeGreaterThan(0);
    expect(
      String(created.data!.indicator_code),
      `缺 code 时应由服务端兜底生成 IND_ 前缀，实际 ${created.data!.indicator_code}`
    ).toMatch(/^IND_/);
    expect(created.data!.indicator_type, '缺省类型必须显式落 ratio 之外提交的值').toBe(
      `e2e_fa_${tag}`
    );
    try {
      // 同请求体但名称为空 → validator 逐字外显；用唯一 indicator_type 反查封零落库
      const badType = `e2e_fa_bad_${tag}`;
      const denied = await apiCallExpectFail(page, 'POST', '/financial-analysis/indicators', {
        name: '',
        indicator_type: badType,
      });
      expect(denied.status, `空名称必须 400，实际 ${JSON.stringify(denied)}`).toBe(400);
      expect(failureCode(denied)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
      expect(denied.message, `校验文案必须逐字，实际 ${denied.message}`).toBe(
        '指标名称长度必须在1到100字符之间'
      );
      const zeroList = await apiCallRaw<{ items: FaIndicatorRow[]; total: number }>(
        page,
        'GET',
        `/financial-analysis/indicators?page=1&page_size=100&indicator_type=${badType}`
      );
      expect(zeroList.total, '被拒的指标不得落库（唯一类型反查必 0）').toBe(0);
      expect(zeroList.items.length, '被拒的指标不得落库（items 必空）').toBe(0);

      // 同类型缺 code 的默认类型分支：indicator_type 缺省提交即落 ratio
      const defaulted = await apiCall<FaIndicatorRow>(
        page,
        'POST',
        '/financial-analysis/indicators',
        {
          name: `默认类型${tag}`,
          code: `E2EFADEF${tag}`,
        }
      );
      expect(defaulted.data!.indicator_type, '缺省 indicator_type 必须为 ratio').toBe('ratio');
      await tryCleanup(
        page,
        'DELETE',
        `/financial-analysis/reports/${defaulted.data!.id}`,
        'fa-indicator-default'
      );
    } finally {
      await tryCleanup(
        page,
        'DELETE',
        `/financial-analysis/reports/${indicatorId}`,
        'fa-indicator'
      );
    }
  });

  test('趋势创建派生量与查询过滤 - 逐值断言和被拒零落库', async ({ page }) => {
    const tag = genCode('FAT');
    const ind = await apiCall<FaIndicatorRow>(page, 'POST', '/financial-analysis/indicators', {
      name: `趋势宿主指标${tag}`,
      code: `E2EFAT${tag}`,
      indicator_type: `e2e_fa_trend_${tag}`,
    });
    const indicatorId = Number(ind.data?.id);
    expect(indicatorId).toBeGreaterThan(0);
    try {
      const first = await apiCall<FaTrendRow>(page, 'POST', '/financial-analysis/trends', {
        indicator_id: indicatorId,
        period: '2099-11',
        value: '100',
        target_value: '80',
      });
      expect(first.data!.analysis_type, '缺省 analysis_type 必须为 trend').toBe('trend');
      expect(Number(first.data!.indicator_value)).toBe(100);
      expect(
        Number(first.data!.variance),
        `variance 必须等于 value-target=20，实际 ${first.data!.variance}`
      ).toBe(20);
      expect(Number(first.data!.variance_rate), 'variance_rate 必须等于 20/80*100=25').toBe(25);
      expect(first.data!.trend, '值高于目标必须判「上升」').toBe('上升');

      const second = await apiCall<FaTrendRow>(page, 'POST', '/financial-analysis/trends', {
        indicator_id: indicatorId,
        period: '2099-11',
        value: '100',
      });
      expect(second.data!.variance, '无目标时 variance 必为 null，不得兜底 0').toBeNull();
      expect(second.data!.trend).toBeNull();

      // 期间过滤传播：命中窗 2 行、错窗 0 行
      const hit = await apiCallRaw<{ items: FaTrendRow[]; total: number }>(
        page,
        'GET',
        `/financial-analysis/trends?indicator_id=${indicatorId}&period=2099-11`
      );
      expect(hit.total, '信封 total 必须等于 items 行数').toBe(hit.items.length);
      expect(hit.items.length, 'period=2099-11 应恰含本轮 2 行').toBe(2);
      expect(
        hit.items.every(r => r.period === '2099-11' && r.indicator_id === indicatorId),
        'period 精确过滤失效即红'
      ).toBe(true);
      const miss = await apiCallRaw<{ items: FaTrendRow[]; total: number }>(
        page,
        'GET',
        `/financial-analysis/trends?indicator_id=${indicatorId}&period=1999-01`
      );
      expect(miss.items.length, '错窗必须整体排除').toBe(0);

      // 缺 indicator_id 必填门 - 可外显逐字
      const noId = await apiCallExpectFail(page, 'GET', '/financial-analysis/trends');
      expect(noId.status, '缺指标ID必须 400').toBe(400);
      expect(failureCode(noId)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
      expect(noId.message, `必填门外显必须逐字，实际 ${noId.message}`).toBe(
        '财务分析请求缺少指标ID'
      );

      // 非法周期被拒且零落库：错窗前后再查行数不变
      const badPeriod = await apiCallExpectFail(page, 'POST', '/financial-analysis/trends', {
        indicator_id: indicatorId,
        period: '',
        value: '1',
      });
      expect(badPeriod.status).toBe(400);
      expect(failureCode(badPeriod)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
      expect(badPeriod.message, '周期校验文案必须逐字').toBe('周期长度必须在1到20字符之间');
      const afterDenied = await apiCallRaw<{ items: FaTrendRow[] }>(
        page,
        'GET',
        `/financial-analysis/trends?indicator_id=${indicatorId}&period=2099-11`
      );
      expect(afterDenied.items.length, '被拒的非法周期行不得落库').toBe(2);
    } finally {
      await tryCleanup(
        page,
        'DELETE',
        `/financial-analysis/reports/${indicatorId}`,
        'fa-indicator'
      );
    }
  });

  test('趋势分析回归计算 - 全等值序列的斜率与移动平均逐项对账', async ({ page }) => {
    const tag = genCode('FAX');
    const ind = await apiCall<FaIndicatorRow>(page, 'POST', '/financial-analysis/indicators', {
      name: `回归宿主指标${tag}`,
      code: `E2EFAX${tag}`,
      indicator_type: `e2e_fa_reg_${tag}`,
    });
    const indicatorId = Number(ind.data?.id);
    try {
      for (let i = 0; i < 3; i++) {
        await apiCall(page, 'POST', '/financial-analysis/trends', {
          indicator_id: indicatorId,
          period: '2099-13',
          value: '100',
        });
      }
      const analysis = await apiCallRaw<{
        data_points: number;
        linear_regression: {
          slope: string | number;
          intercept: string | number;
          r_squared: string | number;
        };
        moving_average: { ma3: Array<number | null>; ma5: Array<number | null> };
        trend_direction: string;
        latest_value: number | null;
        period_range: { start: string; end: string };
      }>(page, 'GET', `/financial-analysis/trend-analysis?indicator_id=${indicatorId}`);
      expect(analysis.data_points, '数据点数必须等于本轮种入的 3').toBe(3);
      expect(
        Math.abs(Number(analysis.linear_regression.slope)),
        `全等值序列 slope 必为 0，实际 ${analysis.linear_regression.slope}`
      ).toBeLessThan(1e-9);
      expect(
        Math.abs(Number(analysis.linear_regression.r_squared)),
        'ss_yy 为零时 r_squared 必须走 0 分支而非除零'
      ).toBeLessThan(1e-9);
      expect(
        Number(analysis.linear_regression.intercept),
        'intercept 应等于常数值 100'
      ).toBeCloseTo(100, 6);
      expect(analysis.moving_average.ma3.length, 'ma3 长度必须等于数据点数').toBe(3);
      expect(analysis.moving_average.ma3[0]).toBeNull();
      expect(analysis.moving_average.ma3[1]).toBeNull();
      expect(Number(analysis.moving_average.ma3[2]), 'ma3 末位必须等于三值均值 100').toBe(100);
      expect(
        analysis.moving_average.ma5.every(v => v === null),
        '样本不足 5 时 ma5 必须全 null'
      ).toBe(true);
      expect(analysis.trend_direction, '全等值序列方向必须判「平稳」').toBe('平稳');
      expect(Number(analysis.latest_value)).toBe(100);
      expect(analysis.period_range.start).toBe('2099-13');
      expect(analysis.period_range.end).toBe('2099-13');

      const noId = await apiCallExpectFail(page, 'GET', '/financial-analysis/trend-analysis');
      expect(noId.status).toBe(400);
      expect(noId.message, '趋势分析必填门外显必须逐字').toBe('财务分析请求缺少指标ID');
    } finally {
      await tryCleanup(
        page,
        'DELETE',
        `/financial-analysis/reports/${indicatorId}`,
        'fa-indicator'
      );
    }
  });

  test('报告族 remark 拼装与部分更新保留段和删除族 404', async ({ page }) => {
    const tag = genCode('FAR');
    const created = await apiCall<{ id: number; indicator_code: string }>(
      page,
      'POST',
      '/financial-analysis/reports',
      {
        name: `契约报告${tag}`,
        report_type: 'e2e_rpt',
        period_start: '2099-01-01',
        period_end: '2099-01-31',
        indicators: [7, 42],
        description: `原始描述${tag}`,
      }
    );
    const reportId = Number(created.data?.id);
    expect(reportId, `建报告未返回 id：${JSON.stringify(created.data)}`).toBeGreaterThan(0);
    expect(String(created.data!.indicator_code), '报告编码必须以 RPT- 前缀落库').toMatch(/^RPT-/);
    try {
      const detail = await apiCallRaw<FaIndicatorRow>(
        page,
        'GET',
        `/financial-analysis/reports/${reportId}`
      );
      expect(detail.indicator_name).toBe(`契约报告${tag}`);
      expect(detail.indicator_type, 'report_type 落 indicator_type 列').toBe('e2e_rpt');
      expect(detail.remark, `remark 必须含期间段，实际 ${detail.remark}`).toContain(
        '期间: 2099-01-01 ~ 2099-01-31'
      );
      expect(detail.remark).toContain('关联指标: 7,42');
      expect(detail.remark).toContain(`描述: 原始描述${tag}`);

      // 部分更新只提交 description：期间段与关联指标段必须原样保留
      const updated = await apiCall<{ remark?: string; name?: string }>(
        page,
        'PUT',
        `/financial-analysis/reports/${reportId}`,
        { description: `新描述${tag}` }
      );
      expect(String(updated.data!.remark), '更新响应 remark 必须仍含期间段').toContain(
        '期间: 2099-01-01 ~ 2099-01-31'
      );
      expect(String(updated.data!.remark)).toContain('关联指标: 7,42');
      expect(String(updated.data!.remark)).toContain(`描述: 新描述${tag}`);
      expect(
        String(updated.data!.remark).includes('原始描述'),
        '旧描述段必须被替换而不是叠加'
      ).toBe(false);

      // 未提交列保持原值
      const afterPut = await apiCallRaw<FaIndicatorRow>(
        page,
        'GET',
        `/financial-analysis/reports/${reportId}`
      );
      expect(afterPut.indicator_name, '未提交 name 时不得改写名称列').toBe(`契约报告${tag}`);
      expect(afterPut.indicator_type).toBe('e2e_rpt');

      // 非法名称被拒且零改写
      const badPut = await apiCallExpectFail(
        page,
        'PUT',
        `/financial-analysis/reports/${reportId}`,
        { name: '' }
      );
      expect(badPut.status).toBe(400);
      expect(badPut.message, '名称校验文案必须逐字').toBe('报告名称长度必须在1到100字符之间');
      const afterBadPut = await apiCallRaw<FaIndicatorRow>(
        page,
        'GET',
        `/financial-analysis/reports/${reportId}`
      );
      expect(afterBadPut.indicator_name, '被拒的 PUT 不得改写名称').toBe(`契约报告${tag}`);

      // 幽灵 id 的 PUT 与 DELETE 与详情均 404（脱敏常量）
      // 幽灵 id 取 i32 上界内的高位区间，保证解析通过且行必不存在
      const ghost = 2_147_000_000 + (Date.now() % 1000);
      const ghostPut = await apiCallExpectFail(
        page,
        'PUT',
        `/financial-analysis/reports/${ghost}`,
        {
          name: '幽灵',
        }
      );
      expect(ghostPut.status).toBe(404);
      expect(failureCode(ghostPut)).toBe(APP_ERROR_CODES.NOT_FOUND);
      expect(ghostPut.message).toBe('资源未找到');
    } finally {
      await tryCleanup(page, 'DELETE', `/financial-analysis/reports/${reportId}`, 'fa-report');
    }

    // 删除后 GET 与二次 DELETE 同族 404，证明真删
    const gone = await apiCallExpectFail(page, 'GET', `/financial-analysis/reports/${reportId}`);
    expect(gone.status).toBe(404);
    expect(failureCode(gone)).toBe(APP_ERROR_CODES.NOT_FOUND);
    const reDelete = await apiCallExpectFail(
      page,
      'DELETE',
      `/financial-analysis/reports/${reportId}`
    );
    expect(reDelete.status, '二次删除必须 404 而非幂等吞掉').toBe(404);
    expect(failureCode(reDelete)).toBe(APP_ERROR_CODES.NOT_FOUND);
    expect(reDelete.message).toBe('资源未找到');
  });

  test('报告执行 - 非预定义指标透明 no_data 回语与期间参数传播', async ({ page }) => {
    const tag = genCode('FAE');
    const created = await apiCall<{ id: number }>(page, 'POST', '/financial-analysis/reports', {
      name: `执行靶报告${tag}`,
      report_type: 'e2e_rpt_exec',
      period_start: '2099-06-01',
      period_end: '2099-06-30',
    });
    const reportId = Number(created.data?.id);
    expect(reportId).toBeGreaterThan(0);
    try {
      const executed = await apiCallRaw<{
        id: number;
        name: string;
        status: string;
        period: string;
        executed_at: string;
        total_indicators_computed: number;
        latest_result: unknown;
      }>(page, 'POST', `/financial-analysis/reports/${reportId}/execute?period=2099-12`);
      expect(executed.id, '执行响应 id 必须回显目标报告').toBe(reportId);
      expect(executed.name).toBe(`执行靶报告${tag}`);
      expect(executed.period, 'period 必须回显提交值而非回落当前年月').toBe('2099-12');
      expect(executed.status, '非预定义指标必须如实回 no_data 而非假 completed').toBe('no_data');
      expect(executed.latest_result, 'no_data 时 latest_result 必须为 null').toBeNull();
      expect(new Date(executed.executed_at).toString(), 'executed_at 必须是合法时间戳').not.toBe(
        'Invalid Date'
      );
      expect(Number.isInteger(executed.total_indicators_computed), '计算指标数必须为整数').toBe(
        true
      );

      const ghost = await apiCallExpectFail(
        page,
        'POST',
        `/financial-analysis/reports/2147000999/execute?period=2099-12`
      );
      expect(ghost.status, '幽灵报告执行必须 404').toBe(404);
      expect(failureCode(ghost)).toBe(APP_ERROR_CODES.NOT_FOUND);
    } finally {
      await tryCleanup(page, 'DELETE', `/financial-analysis/reports/${reportId}`, 'fa-report');
    }
  });

  test('现金流比率 - 专属期间回显与零余额分支对账', async ({ page }) => {
    const res = await apiCallRaw<{
      period: string;
      indicators: Array<{
        indicator_id: number;
        indicator_value: string | number;
        target_value: string | number | null;
      }>;
      total: number;
    }>(page, 'POST', '/financial-analysis/cash-flow-ratios?period=2099-12');
    expect(res.period, 'period 必须回显提交值').toBe('2099-12');
    expect(Array.isArray(res.indicators), 'indicators 必须为数组').toBe(true);
    expect(res.total, 'total 必须等于 indicators 行数').toBe(res.indicators.length);
    for (const row of res.indicators) {
      expect(Number.isInteger(row.indicator_id), '行必须携带指标 id').toBe(true);
      expect(
        Number(row.indicator_value),
        `零余额期间分子恒 0，可算出的比率必须为 0，实际 ${row.indicator_value}`
      ).toBe(0);
      const target = row.target_value === null ? null : Number(row.target_value);
      expect([null, 0.4, 0.2, 1], '目标常量必须来自现金流定义表').toContain(target);
    }
  });
});
