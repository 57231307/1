import { test, expect } from '../diagnose-fixture';
import {
  APP_ERROR_CODES,
  loginViaUI,
  apiCall,
  apiCallExpectFail,
  deferCleanup,
  flushDeferredCleanups,
  failureCode,
  type DeferredCleanup,
} from './helpers';

/**
 * 125 行业基准配置端点族的增删改查读回（finance 域，industry-benchmarks 族）
 *
 * 功能：守住行业基准配置面（表 industry_benchmark_configs）确实把提交值落库并能读回，
 * 覆盖以下五个真实端点，均由 industry_benchmark_handler 产出：
 * - GET /industry-benchmarks 列表（返回 Vec，非分页信封；后端按 is_active=true 过滤，
 *   先按 industry_type 升序再按 metric_name 升序）。
 * - POST /industry-benchmarks 建基准，benchmark_name/industry_type/metric_name/metric_value 必填；
 *   可选 unit(String) data_source(String) data_year(i32) is_active(bool) remark(String)。
 *   is_active 由 handler:45 unwrap_or(true) 兜底，unit/data_source/data_year/remark 缺省 null。
 * - GET /industry-benchmarks/{id} 复用 list_industry_benchmarks 处理（handler 不提取路径 id），
 *   返回与 GET /industry-benchmarks 相同的全量启用列表。本用例仍向该路径发请求以满足端点覆盖。
 * - PUT /industry-benchmarks/{id} 部分更新，只覆盖请求体里出现的列，未出现的列保持原值；
 *   id 不存在返回 404 且码为 NOT_FOUND。
 * - DELETE /industry-benchmarks/{id} 硬删除（SeaORM active.delete），删后按 id 不再出现在列表中。
 *
 * 调用方：routes/finance.rs:1211-1224 注册，handler 在 handlers/industry_benchmark_handler.rs。
 * 入参：CreateIndustryBenchmarkDto 必填 benchmark_name(String) industry_type(String)
 *   metric_name(String) metric_value(Decimal(14,4))；
 *   可选 unit(String) data_source(String) data_year(i32) is_active(bool) remark(String)。
 * 传给谁：industry_benchmark_config::ActiveModel → sea_orm → PostgreSQL。
 * 存什么：id benchmark_name industry_type metric_name metric_value unit
 *   data_source data_year is_active remark created_at updated_at。
 * 存哪里：表 industry_benchmark_configs，路由前缀 /api/v1/erp/industry-benchmarks。
 *
 * 断言均为值判据，不用恒真、不只判 ok()。
 */

type BenchmarkRow = {
  id: number;
  benchmark_name: string;
  industry_type: string;
  metric_name: string;
  metric_value: string;
  unit: string | null;
  data_source: string | null;
  data_year: number | null;
  is_active: boolean;
  remark: string | null;
};

function uniqTag(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

test.describe.serial('125 行业基准配置增删改查读回', () => {
  const cleanup: DeferredCleanup[] = [];

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test.afterEach(async ({ page }) => {
    await flushDeferredCleanups(page, cleanup);
  });

  test('POST 建基准后由列表读回逐键持久，未提交列落后端默认 null', async ({ page }) => {
    const tag = uniqTag();
    const name = `e2e基准-${tag}`;

    const created = await apiCall<BenchmarkRow>(page, 'POST', '/industry-benchmarks', {
      benchmark_name: name,
      industry_type: '纺织',
      metric_name: '流动比率',
      metric_value: '1.5000',
    });
    const id = created.data.id;
    expect(Number.isInteger(id) && id > 0, `创建应返回正整数 id，实际：${String(id)}`).toBe(true);
    deferCleanup(cleanup, 'DELETE', `/industry-benchmarks/${id}`, `ib-${tag}`);

    expect(created.data.benchmark_name).toBe(name);
    expect(created.data.industry_type).toBe('纺织');
    expect(created.data.metric_name).toBe('流动比率');
    expect(Number(created.data.metric_value), 'metric_value 应持久 1.5').toBeCloseTo(1.5, 4);
    expect(created.data.unit, '未提交 unit 应为 null').toBeNull();
    expect(created.data.data_source, '未提交 data_source 应为 null').toBeNull();
    expect(created.data.data_year, '未提交 data_year 应为 null').toBeNull();
    expect(created.data.is_active, '缺省 is_active 应为 true').toBe(true);
    expect(created.data.remark, '未提交 remark 应为 null').toBeNull();

    // 从列表读回（证明落库，不只 POST 回显）。
    const listed = await apiCall<BenchmarkRow[]>(page, 'GET', '/industry-benchmarks');
    const hit = listed.data.find(r => r.id === id);
    expect(hit, `列表应包含刚建基准 id=${id}`).toBeDefined();
    expect(hit!.benchmark_name).toBe(name);
    expect(hit!.industry_type).toBe('纺织');
    expect(hit!.metric_name).toBe('流动比率');
  });

  test('GET /industry-benchmarks/{id} 仍返回启用基准列表', async ({ page }) => {
    const tag = uniqTag();
    const created = await apiCall<BenchmarkRow>(page, 'POST', '/industry-benchmarks', {
      benchmark_name: `e2e-byid-${tag}`,
      industry_type: '化纤',
      metric_name: '速动比率',
      metric_value: '0.8500',
      unit: '倍',
      data_source: '统计局',
      data_year: 2024,
      remark: `e2e-remark-${tag}`,
    });
    const id = created.data.id;
    deferCleanup(cleanup, 'DELETE', `/industry-benchmarks/${id}`, `ib-byid-${tag}`);

    // 该路径复用 list 处理器（handler 不提取 id），返回启用基准数组。
    const byId = await apiCall<BenchmarkRow[]>(page, 'GET', `/industry-benchmarks/${id}`);
    expect(Array.isArray(byId.data), 'GET /{id} 应返回数组').toBe(true);
    const hit = byId.data.find(r => r.id === id);
    expect(hit, `列表应包含刚建基准 id=${id}`).toBeDefined();
    expect(hit!.benchmark_name).toBe(`e2e-byid-${tag}`);
    expect(hit!.unit).toBe('倍');
    expect(hit!.data_source).toBe('统计局');
    expect(hit!.data_year).toBe(2024);
    expect(hit!.remark).toBe(`e2e-remark-${tag}`);
  });

  test('PUT 部分更新后旧值变新值，缺席列保持原值', async ({ page }) => {
    const tag = uniqTag();
    const created = await apiCall<BenchmarkRow>(page, 'POST', '/industry-benchmarks', {
      benchmark_name: `原始基准-${tag}`,
      industry_type: '印染',
      metric_name: '资产负债率',
      metric_value: '65.0000',
      unit: '%',
      data_year: 2023,
      remark: '原始备注',
    });
    const id = created.data.id;
    deferCleanup(cleanup, 'DELETE', `/industry-benchmarks/${id}`, `ib-upd-${tag}`);

    const updated = await apiCall<BenchmarkRow>(page, 'PUT', `/industry-benchmarks/${id}`, {
      benchmark_name: `更新基准-${tag}`,
      metric_value: '70.5000',
    });
    expect(updated.data.benchmark_name).toBe(`更新基准-${tag}`);
    expect(Number(updated.data.metric_value), '更新后值应为 70.5').toBeCloseTo(70.5, 4);
    // 缺席列保持原值
    expect(updated.data.industry_type, '缺席的 industry_type 应保持原值 印染').toBe('印染');
    expect(updated.data.metric_name, '缺席的 metric_name 应保持原值').toBe('资产负债率');
    expect(updated.data.unit, '缺席的 unit 应保持原值 %').toBe('%');
    expect(updated.data.data_year, '缺席的 data_year 应保持原值 2023').toBe(2023);

    // 从列表读回确认持久
    const listed = await apiCall<BenchmarkRow[]>(page, 'GET', '/industry-benchmarks');
    const hit = listed.data.find(r => r.id === id);
    expect(hit!.benchmark_name).toBe(`更新基准-${tag}`);
    expect(Number(hit!.metric_value)).toBeCloseTo(70.5, 4);
  });

  test('DELETE 硬删后 id 不再出现在列表中', async ({ page }) => {
    const tag = uniqTag();
    const created = await apiCall<BenchmarkRow>(page, 'POST', '/industry-benchmarks', {
      benchmark_name: `待删基准-${tag}`,
      industry_type: '服装',
      metric_name: '毛利率',
      metric_value: '30.0000',
    });
    const id = created.data.id;

    const delRes = await apiCall<string>(page, 'DELETE', `/industry-benchmarks/${id}`);
    expect(delRes.code, '删除应返回 200').toBe(200);

    // 列表不应包含已删行
    const listed = await apiCall<BenchmarkRow[]>(page, 'GET', '/industry-benchmarks');
    const gone = listed.data.find(r => r.id === id);
    expect(gone, `删除后 id=${id} 不应再出现在列表中`).toBeUndefined();
  });

  test('PUT 不存在的 id 返回 404 且码为 NOT_FOUND', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'PUT', '/industry-benchmarks/999999999', {
      benchmark_name: 'ghost',
    });
    expect(fail.status).toBe(404);
    expect(failureCode(fail)).toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('DELETE 不存在的 id 返回 404 且码为 NOT_FOUND', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'DELETE', '/industry-benchmarks/999999998');
    expect(fail.status).toBe(404);
    expect(failureCode(fail)).toBe(APP_ERROR_CODES.NOT_FOUND);
  });
});
