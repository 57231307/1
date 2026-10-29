import { test, expect } from '../diagnose-fixture';
import {
  APP_ERROR_CODES,
  loginViaUI,
  apiCall,
  apiCallExpectFail,
  deferCleanup,
  flushDeferredCleanups,
  failureCode,
  type ApiResponse,
  type DeferredCleanup,
} from './helpers';

/**
 * 122 账龄预警规则端点族的增删改查读回（finance 域，aging-alert-rules 族）
 *
 * 功能：守住账龄预警规则面（表 aging_alert_rules）确实把提交值落库并能逐键读回，
 * 覆盖以下五个真实端点，均由 aging_alert_rule_handler 产出：
 * - GET /aging-alert-rules 分页列表（页码基数 0，入参 page/page_size/aging_bucket/alert_level/is_active），
 *   出参 ApiResponse<PaginatedResponse<Model>>，data.items 为规则对象数组、data.total 在分页信封顶层。
 * - POST /aging-alert-rules 建规则，rule_name/rule_code/aging_bucket/threshold_days/alert_level/notify_method 必填，
 *   缺必填字段返回 400 且码为 VALIDATION_ERROR；rule_code 唯一约束兜底并发。
 *   未提交的列取后端默认：threshold_amount 为 null、notify_roles 为 null、is_active 为 true、remarks 为 null。
 * - GET /aging-alert-rules/{id} 取单条，id 不存在返回 404 且码为 NOT_FOUND。
 * - PUT /aging-alert-rules/{id} 部分更新，只覆盖请求体里出现的列，未出现的列保持原值；
 *   可空列三态（对齐 RFC 7386）：键缺席=保持原值、显式 null=清空为 NULL、有值=覆盖。
 *   id 不存在返回 404 且码为 NOT_FOUND。
 * - DELETE /aging-alert-rules/{id} 硬删除，删后按 id 再查返回 404。
 *
 * 调用方：routes/finance.rs 的 aging_alert_rules 路由函数注册，handler 在 handlers/aging_alert_rule_handler.rs。
 * 入参：CreateAlertRuleDto 必填 rule_name(String) rule_code(String) aging_bucket(String)
 *   threshold_days(i32) alert_level(String) notify_method(String)；可选 threshold_amount(Decimal)
 *   notify_roles(Vec<String>) is_active(bool) remarks(String)。
 * 传给谁：AgingAlertRuleService → sea_orm Entity → PostgreSQL。
 * 存什么：id rule_name rule_code aging_bucket threshold_days threshold_amount
 *   alert_level notify_method notify_roles is_active remarks created_at updated_at。
 * 存哪里：表 aging_alert_rules，路由前缀 /api/v1/erp/aging-alert-rules。
 *
 * 断言均为值判据（写什么读回什么、更新后旧值变新值、删除后 404），
 * 不用恒真、不只判 ok()、不把 URL 藏进变量让判定器解析不出位点。
 */

type AlertRuleRow = {
  id: number;
  rule_name: string;
  rule_code: string;
  aging_bucket: string;
  threshold_days: number;
  threshold_amount: string | null;
  alert_level: string;
  notify_method: string;
  notify_roles: string[] | null;
  is_active: boolean;
  remarks: string | null;
};

function uniqTag(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

test.describe.serial('122 账龄预警规则增删改查读回', () => {
  const cleanup: DeferredCleanup[] = [];

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test.afterEach(async ({ page }) => {
    await flushDeferredCleanups(page, cleanup);
  });

  test('POST 建规则后由单条读回逐键持久，未提交列落后端默认', async ({ page }) => {
    const tag = uniqTag();
    const code = `e2e-aar-${tag}`;
    const name = `e2e预警规则-${tag}`;

    const created = await apiCall<AlertRuleRow>(page, 'POST', '/aging-alert-rules', {
      rule_name: name,
      rule_code: code,
      aging_bucket: '30_days',
      threshold_days: 30,
      alert_level: 'warning',
      notify_method: 'system',
    });
    const id = created.data.id;
    expect(Number.isInteger(id) && id > 0, `创建应返回正整数 id，实际：${String(id)}`).toBe(true);
    deferCleanup(cleanup, 'DELETE', `/aging-alert-rules/${id}`, `aar-${tag}`);

    expect(created.data.rule_name, '建接口应回显 rule_name').toBe(name);
    expect(created.data.rule_code).toBe(code);
    expect(created.data.aging_bucket).toBe('30_days');
    expect(created.data.threshold_days).toBe(30);
    expect(created.data.alert_level).toBe('warning');
    expect(created.data.notify_method).toBe('system');
    expect(created.data.threshold_amount, '未提交金额应为 null').toBeNull();
    expect(created.data.notify_roles, '未提交角色应为 null').toBeNull();
    expect(created.data.is_active, '缺省 is_active 应为 true').toBe(true);
    expect(created.data.remarks, '未提交备注应为 null').toBeNull();

    // 从数据库经单条读回（证明落库，不只是 POST 回显）。
    const fetched = await apiCall<AlertRuleRow>(page, 'GET', `/aging-alert-rules/${id}`);
    expect(fetched.data.id).toBe(id);
    expect(fetched.data.rule_name).toBe(name);
    expect(fetched.data.rule_code).toBe(code);
    expect(fetched.data.aging_bucket).toBe('30_days');
    expect(fetched.data.threshold_days).toBe(30);
    expect(fetched.data.alert_level).toBe('warning');
    expect(fetched.data.notify_method).toBe('system');
  });

  test('GET 列表分页读回包含刚建规则', async ({ page }) => {
    const tag = uniqTag();
    const code = `e2e-aar-list-${tag}`;
    const name = `e2e列表规则-${tag}`;

    const created = await apiCall<AlertRuleRow>(page, 'POST', '/aging-alert-rules', {
      rule_name: name,
      rule_code: code,
      aging_bucket: '60_days',
      threshold_days: 60,
      alert_level: 'critical',
      notify_method: 'email',
      threshold_amount: '9999.50',
      is_active: true,
      remarks: `e2e-remark-${tag}`,
    });
    const id = created.data.id;
    deferCleanup(cleanup, 'DELETE', `/aging-alert-rules/${id}`, `aar-list-${tag}`);

    // 本链页码基数 0（list_rules unwrap_or(0)），page=0 取首页。
    const listed = await apiCall<{
      items: AlertRuleRow[];
      total: number;
      page: number;
      page_size: number;
    }>(page, 'GET', '/aging-alert-rules?page=0&page_size=100');

    const hit = listed.data.items.find(r => r.id === id);
    expect(hit, `列表应包含刚建规则 id=${id}`).toBeDefined();
    expect(hit!.rule_code).toBe(code);
    expect(hit!.aging_bucket).toBe('60_days');
    expect(hit!.threshold_days).toBe(60);
    expect(hit!.alert_level).toBe('critical');
    expect(hit!.notify_method).toBe('email');
    expect(hit!.remarks).toBe(`e2e-remark-${tag}`);
    expect(Number(hit!.threshold_amount), '阈值金额应持久 9999.50').toBeCloseTo(9999.50, 2);
  });

  test('PUT 部分更新后旧值变新值，缺席列保持原值', async ({ page }) => {
    const tag = uniqTag();
    const code = `e2e-aar-upd-${tag}`;

    const created = await apiCall<AlertRuleRow>(page, 'POST', '/aging-alert-rules', {
      rule_name: `原始名称-${tag}`,
      rule_code: code,
      aging_bucket: '90_days',
      threshold_days: 90,
      alert_level: 'info',
      notify_method: 'system',
      threshold_amount: '5000.00',
      remarks: '原始备注',
    });
    const id = created.data.id;
    deferCleanup(cleanup, 'DELETE', `/aging-alert-rules/${id}`, `aar-upd-${tag}`);

    const updated = await apiCall<AlertRuleRow>(page, 'PUT', `/aging-alert-rules/${id}`, {
      rule_name: `更新名称-${tag}`,
      threshold_days: 45,
    });
    expect(updated.data.rule_name).toBe(`更新名称-${tag}`);
    expect(updated.data.threshold_days).toBe(45);
    // 缺席列保持原值
    expect(updated.data.aging_bucket, '缺席的 aging_bucket 应保持原值').toBe('90_days');
    expect(updated.data.alert_level, '缺席的 alert_level 应保持原值').toBe('info');
    expect(updated.data.rule_code, 'rule_code 不在更新 DTO 中，应保持原值').toBe(code);

    // 从数据库读回确认
    const fetched = await apiCall<AlertRuleRow>(page, 'GET', `/aging-alert-rules/${id}`);
    expect(fetched.data.rule_name).toBe(`更新名称-${tag}`);
    expect(fetched.data.threshold_days).toBe(45);
  });

  test('DELETE 硬删后按 id 再查返回 404', async ({ page }) => {
    const tag = uniqTag();
    const code = `e2e-aar-del-${tag}`;

    const created = await apiCall<AlertRuleRow>(page, 'POST', '/aging-alert-rules', {
      rule_name: `待删规则-${tag}`,
      rule_code: code,
      aging_bucket: 'current',
      threshold_days: 1,
      alert_level: 'info',
      notify_method: 'system',
    });
    const id = created.data.id;

    const delRes = await apiCall<string>(page, 'DELETE', `/aging-alert-rules/${id}`);
    expect(delRes.code, `删除应返回成功码 200`).toBe(200);
    expect(delRes.data, '删除返回应含规则 id').toContain(String(id));

    // 删后读回必须 404
    const afterDel = await apiCallExpectFail(page, 'GET', `/aging-alert-rules/${id}`);
    expect(afterDel.status, `删除后 GET 应 404，实际 ${afterDel.status}`).toBe(404);
    expect(failureCode(afterDel), '删除后应返回 NOT_FOUND 码').toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('POST 缺必填字段返回 400 且码为 VALIDATION_ERROR', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'POST', '/aging-alert-rules', {
      rule_name: 'x',
    });
    expect(fail.status, '缺必填应 400').toBe(400);
    expect(failureCode(fail), '码应为 VALIDATION_ERROR').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
  });

  test('GET 不存在的 id 返回 404 且码为 NOT_FOUND', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'GET', '/aging-alert-rules/999999999');
    expect(fail.status).toBe(404);
    expect(failureCode(fail)).toBe(APP_ERROR_CODES.NOT_FOUND);
  });
});
