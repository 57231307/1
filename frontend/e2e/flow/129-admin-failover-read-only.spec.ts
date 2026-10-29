import { test, expect } from '../diagnose-fixture';
import {
  APP_ERROR_CODES,
  loginViaUI,
  apiCall,
  apiCallExpectFail,
  failureCode,
  API_BASE,
  API_PREFIX,
} from './helpers';

/**
 * 129 主备隔离端点族的只读面与入参校验回（admin 域，admin/failover 族）
 *
 * 功能：守住运维主备隔离面五个端点的契约可达与响应形状，
 * 覆盖以下五个真实端点，均由 failover_handler 产出：
 * - GET /admin/failover/status 取主备实时状态与最近事件，
 *   出参 ApiResponse<StatusResponse>，data.statuses 为 FailoverStatusDto 数组、
 *   data.events 为 FailoverEventDto 数组。
 * - GET /admin/failover/metrics 取 Prometheus 指标，
 *   返回 text/plain 格式（Prometheus exposition format），非 JSON。
 *   Content-Type 为 application/openmetrics-text 或 text/plain。
 * - GET /admin/failover/health 健康检查，出参 ApiResponse<Value>，
 *   data 含 database(bool) 与 cache(bool) 两个键。
 * - POST /admin/failover/test/switch 手动触发切换（仅管理员）：
 *   入参 SwitchRequest（function 必填，合法值为 database 或 cache）；
 *   本用例只测非法 function 的入参校验负例（返回 400 且 BAD_REQUEST），
 *   不触发真实主备切换，避免对共享数据库造成破坏。
 * - POST /admin/failover/failback 故障回切：
 *   入参 FailbackRequest（function 必填，合法值为 database 或 cache）；
 *   本用例只测非法 function 的入参校验负例，不触发真实回切。
 *
 * 调用方：routes/failover.rs 的 failover_routes 函数注册于 routes/mod.rs 的
 *   nest("/api/v1/erp/admin/failover")，handler 在 handlers/failover_handler.rs。
 * 入参：GET 无请求体；POST 取 JSON body {function}。
 * 传给谁：FailoverService → sea_orm / ArcSwap executor（主备切换执行器）。
 * 存什么：status 端点读 failover_status 与 failover_event 表；
 *   health 端点探测连接可用性不持久化；
 *   test/switch 与 failback 在真实执行时写切换事件与指标，本用例不触发。
 * 存哪里：路由前缀 /api/v1/erp/admin/failover。
 *
 * 写腿为何不测真实入参：POST /test/switch 与 POST /failback 会触发真实数据库连接切换，
 * 对共享测试库和其他并发分片产生不可逆影响，属于派工纪律明确禁止的危险写操作，
 * 只测入参校验负例（非法 function → 400）。
 *
 * 断言均为值判据（形状键存在、类型正确、机器码与状态码双钉），不用恒真、不只判 ok()。
 */

type FailoverStatusData = {
  statuses: Array<{
    function: string;
    active_target: string;
    last_switch_at: string | null;
    failover_count: number;
  }>;
  events: Array<{
    id: number;
    function: string;
    from_target: string;
    to_target: string;
    switched_at: string;
  }>;
};

test.describe.serial('129 主备隔离端点族只读面与入参校验', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('GET status 应返回 statuses 与 events 两个数组', async ({ page }) => {
    const result = await apiCall<FailoverStatusData>(page, 'GET', '/admin/failover/status');
    expect(result.code, '状态查询应 200').toBe(200);
    expect(Array.isArray(result.data.statuses), 'statuses 应为数组').toBe(true);
    expect(Array.isArray(result.data.events), 'events 应为数组').toBe(true);
  });

  test('GET health 应含 database 与 cache 两个布尔键', async ({ page }) => {
    const result = await apiCall<{ database: boolean; cache: boolean }>(
      page,
      'GET',
      '/admin/failover/health'
    );
    expect(result.code, '健康检查应 200').toBe(200);
    expect(typeof result.data.database, 'database 应为布尔').toBe('boolean');
    expect(typeof result.data.cache, 'cache 应为布尔').toBe('boolean');
  });

  test('GET metrics 应返回非 JSON 的 text 格式（Prometheus exposition）', async ({ page }) => {
    const csrfToken = await (async () => {
      const cookies = await page.context().cookies();
      const csrf = cookies.find(c => c.name === 'csrf_token');
      return csrf?.value ?? '';
    })();
    const resp = await page.request.get(`${API_BASE}${API_PREFIX}/admin/failover/metrics`, {
      headers: {
        'X-Requested-With': 'XMLHttpRequest',
        'X-CSRF-Token': csrfToken,
      },
    });
    expect(resp.status(), '指标端点应 200').toBe(200);
    const ct = resp.headers()['content-type'] ?? '';
    expect(
      ct,
      `Content-Type 应为 text 类（Prometheus 格式），实际：${ct}`
    ).toMatch(/text\/plain|application\/openmetrics/);
  });

  test('POST test/switch 非法 function 返回 400 且码为 BAD_REQUEST', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'POST', '/admin/failover/test/switch', {
      function: 'e2e_invalid_function',
    });
    expect(fail.status, '非法 function 应 400').toBe(400);
    expect(failureCode(fail), '码应为 BAD_REQUEST').toBe(APP_ERROR_CODES.BAD_REQUEST);
    expect(
      fail.message,
      `应点名合法取值 database/cache，实际：${fail.message}`
    ).toContain('database');
  });

  test('POST failback 非法 function 返回 400 且码为 BAD_REQUEST', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'POST', '/admin/failover/failback', {
      function: 'e2e_invalid_function',
    });
    expect(fail.status, '非法 function 应 400').toBe(400);
    expect(failureCode(fail), '码应为 BAD_REQUEST').toBe(APP_ERROR_CODES.BAD_REQUEST);
    expect(
      fail.message,
      `应点名合法取值 database/cache，实际：${fail.message}`
    ).toContain('cache');
  });
});
