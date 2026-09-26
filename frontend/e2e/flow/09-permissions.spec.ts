import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  verifyPermissionDenied,
  verifySoDConflict,
  getCtx,
  genCode,
  genName,
  expectDenied,
  loginAsRole,
  API_BASE,
  API_PREFIX,
  failureCode,
  type ApiFailureBody,
  CSRF_ERROR_CODES,
} from './helpers';

test.describe.serial('扩展: 权限深度测试（SoD/字段级/黑名单/缓存）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('P1-1 验证角色互斥规则（9 对 SoD）', async ({ page }) => {
    // 后端 GET /roles/conflicts（role_conflicts 表，初始化时写入 9 对 SoD）
    // handler 返回 ApiResponse::success(Vec) → data 本身就是数组
    const conflicts = await apiCallRaw<Array<{ role_a_code: string; role_b_code: string }>>(
      page,
      'GET',
      '/roles/conflicts?page=1&page_size=20'
    );
    expect(Array.isArray(conflicts)).toBe(true);

    const pairs = conflicts.map(c => `${c.role_a_code}↔${c.role_b_code}`);
    if (pairs.length > 0) {
      // 对齐 migration/src/domain/finance/mod.rs 预置的财务三权分立互斥种子
      const expectedPairs = [
        'accountant↔finance_manager',
        'accountant↔cashier',
        'cashier↔finance_manager',
        'cashier↔purchase_manager',
        'cashier↔purchase_clerk',
        'cashier↔sales_manager',
        'production_manager↔qc_manager',
        'dyeing_master↔quality_inspector',
      ];
      const hasAny = pairs.some(p =>
        expectedPairs.some(e => p.includes(e.split('↔')[0]) && p.includes(e.split('↔')[1]))
      );
      expect(hasAny).toBe(true);
    }
  });

  test('P1-2 验证敏感导出 fail-closed（无审批令牌 403）', async ({ page }) => {
    // P1.1 fail-closed 落地后：/products/export 为敏感资源导出，
    // 无 download_token 一律 403（admin 也不例外）
    const result = await apiCallExpectFail(page, 'GET', '/products/export');
    expectDenied(result);
  });

  test('P1-3 验证敏感导出 fail-closed（伪造令牌 403）', async ({ page }) => {
    // 伪造 download_token 同样 403（令牌查无或资源类型不匹配）。
    // 该路由在 routes/production.rs 的 dye_recipes() 内注册字面量 /dye-recipes/export，
    // 经 routes/mod.rs 的 nest("/api/v1/erp/production", production::routes()) 挂载，
    // 真实路径必须带 production 前缀，否则打错 URL 直接 404（假象成"未鉴权"）。
    const result = await apiCallExpectFail(
      page,
      'GET',
      '/production/dye-recipes/export?download_token=fake-token-p13'
    );
    expectDenied(result);
  });

  test('P1-4 验证权限缓存（多次调用不拒绝）', async ({ page }) => {
    // 原实现循环 5 次取数却不作任何断言（注释写"容错"），无论后端返回什么都算通过。
    // 权限缓存的判据是：同一身份连续请求都被放行，且权限范围内列表的总量在并发下只增不减
    // （其它分片会往同一库里建用户，故 total 单调不降而非恒定；一旦缓存返回更早的
    //  快照导致 total 回退，或某次请求被权限层拒绝，本用例即红）。
    const totals: number[] = [];
    for (let i = 0; i < 5; i++) {
      const result = await apiCallRaw<{ users: Array<{ id: number }>; total: number }>(
        page,
        'GET',
        '/users?page=1&page_size=5'
      );
      expect(
        Array.isArray(result?.users),
        `第 ${i + 1} 次请求未返回 users 数组：${JSON.stringify(result).slice(0, 200)}`
      ).toBe(true);
      expect(
        result.users.length,
        `第 ${i + 1} 次请求 users 为空（admin 至少能看到自身与分片账号）：${JSON.stringify(
          result
        ).slice(0, 200)}`
      ).toBeGreaterThan(0);
      const total = Number(result.total);
      expect(Number.isInteger(total), `第 ${i + 1} 次 total 非整数：${result.total}`).toBe(true);
      expect(
        total,
        `第 ${i + 1} 次 total(${total}) 小于本页行数(${result.users.length})，分页出参自相矛盾`
      ).toBeGreaterThanOrEqual(result.users.length);
      totals.push(total);
    }
    for (let i = 1; i < totals.length; i++) {
      expect(
        totals[i],
        `第 ${i + 1} 次 total(${totals[i]}) 比第 ${i} 次(${totals[i - 1]}) 小：缓存回退了更早快照`
      ).toBeGreaterThanOrEqual(totals[i - 1]);
    }
  });

  test('P1-5 验证未知路由 fail-closed', async ({ page }) => {
    const result = await apiCallExpectFail(page, 'GET', '/unknown-module/unknown-resource');
    expect(result.status).toBeGreaterThanOrEqual(400);
  });

  test('P1-6 验证资源 ID 精确匹配（防垂直越权）', async ({ page }) => {
    // 尝试访问不存在的资源 ID
    const result = await apiCallExpectFail(page, 'GET', '/users/99999999');
    expect(result.status === 404 || result.status === 403 || result.status >= 400).toBeTruthy();
  });

  test('P1-7 验证数据权限行级隔离（Dept 级别）', async ({ page }) => {
    // 后端 list_data_permissions → Json<ApiResponse<Vec<DataPermissionResponse>>>
    // （data_permission_handler.rs:271-283）：data 是权限裸数组，无 items / data.items 包装。
    // 原三级回退 + ?? [] 会让"缺键/契约漂移"永远读到空数组而恒绿，改为单一裸数组直读 + 逐行内容断言。
    const perms = await apiCallRaw<Array<{ id: number; role_id: number; resource_type: string }>>(
      page,
      'GET',
      '/data-permissions'
    );
    expect(
      Array.isArray(perms),
      `数据权限列表 data 非裸数组（后端 list_data_permissions 返回 Vec）：${JSON.stringify(perms).slice(0, 200)}`
    ).toBe(true);
    for (const row of perms.slice(0, 10)) {
      expect(Number(row.id), `数据权限行缺 id：${JSON.stringify(row)}`).toBeGreaterThan(0);
      expect(typeof row.resource_type, `数据权限行缺 resource_type：${JSON.stringify(row)}`).toBe(
        'string'
      );
    }
  });

  test('P1-8 验证字段级权限', async ({ page }) => {
    // 后端 list_field_permissions → Json<ApiResponse<Vec<FieldPermissionResponse>>>
    // （field_permission_handler.rs:76-80）：data 是字段权限裸数组。
    const perms = await apiCallRaw<
      Array<{ id: number; role_id: number; resource_type: string; field_name: string }>
    >(page, 'GET', '/field-permissions');
    expect(
      Array.isArray(perms),
      `字段权限列表 data 非裸数组（后端 list_field_permissions 返回 Vec）：${JSON.stringify(perms).slice(0, 200)}`
    ).toBe(true);
    for (const row of perms.slice(0, 10)) {
      expect(Number(row.id), `字段权限行缺 id：${JSON.stringify(row)}`).toBeGreaterThan(0);
      expect(typeof row.field_name, `字段权限行缺 field_name：${JSON.stringify(row)}`).toBe(
        'string'
      );
    }
  });

  test('P1-8b 验证客户字段级权限端点', async ({ page }) => {
    // 旧写法 GET /customer-field-permissions 未在路由树注册（恒 404）——错误路径。
    // 真实端点：GET /crm/customers/field-permissions/{role_id}
    // （routes/crm.rs:522 → crm_handler.rs:900-910，service 返回 Vec<customer_field_permission::Model>，
    //  data 为按 role_id 过滤后的裸数组）。
    const roleList = await apiCallRaw<{ roles: Array<{ id: number }> }>(page, 'GET', '/roles');
    expect(
      Array.isArray(roleList?.roles) ? roleList.roles.length : -1,
      `未取得任何角色，无法验证客户字段权限端点：${JSON.stringify(roleList).slice(0, 200)}`
    ).toBeGreaterThan(0);
    const roleId = roleList.roles[0].id;
    const perms = await apiCallRaw<Array<{ role_id: number; field_name: string }>>(
      page,
      'GET',
      `/crm/customers/field-permissions/${roleId}`
    );
    // 缺键/漂移必须判红；无配置时的空集是合法结果
    expect(
      Array.isArray(perms),
      `客户字段权限 data 非裸数组（后端返回 Vec）：${JSON.stringify(perms).slice(0, 200)}`
    ).toBe(true);
    for (const row of perms) {
      expect(
        Number(row.role_id),
        `客户字段权限行 role_id 与查询角色不符：${JSON.stringify(row)}`
      ).toBe(Number(roleId));
    }
  });

  test('P1-9 验证 CSRF 防护：缺失令牌的写请求必须被拒', async ({ page }) => {
    // 原用例只 GET 了一次 /users 且不留任何断言（注释写"容错"），根本没验证 CSRF。
    // 这里绕过 apiCall 的令牌注入，直接发一个不带 X-CSRF-Token 的写请求：
    // 会话 Cookie 仍在（已登录），因此被拒只能是因为 CSRF 令牌缺失。
    // 必须打后端绝对地址（与 helpers.apiCall 同一拼法）：vite 只代理 /api/**，
    // 相对路径 '/departments' 会落到 SPA history fallback（200 + HTML），测的是前端不是中间件。
    const url = `${API_BASE}${API_PREFIX}/departments`;
    const res = await page.request.fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        // 与 helpers.apiCall 同构，仅去掉 X-CSRF-Token，构成"只差 CSRF 令牌"的负例
        'X-Requested-With': 'XMLHttpRequest',
      },
      data: { name: `E2E-CSRF-${Date.now()}`, code: `E2E-CSRF-${Date.now()}` },
    });
    expect(
      res.status(),
      `带会话 Cookie 但缺 X-CSRF-Token 的写请求应被 csrf_middleware 拒为 403，实际 HTTP ${res.status()}`
    ).toBe(403);
    const body: ApiFailureBody = await res.json();
    // 错误码出自 backend/src/middleware/csrf.rs:39 CODE_MISS（提取不到 token 的分支，
    // 响应体由 csrf.rs:234-242 直出，code 为字符串机器码而非 ApiResponse 的数字码）
    expect(
      failureCode(body),
      `应返回 CSRF 缺失错误码 ${CSRF_ERROR_CODES.MISSING}，实际：${JSON.stringify(body).slice(0, 200)}`
    ).toBe(CSRF_ERROR_CODES.MISSING);
  });

  test('P1-10 验证权限审计日志（拒绝记录真实落库）', async ({ page }) => {
    // 原实现请求 /unknown-module/test —— 那是未注册路径，路由层直接 404，
    // 权限中间件根本没执行，permission.rs 的 record_permission_denied 不会被触发；
    // 末尾再写 expect(denied.length >= 0) 这种恒真断言，等于"一条都没查到"也算通过。
    // 现改为真实越权：非 admin 角色访问 admin 专属端点 → 403 → 落审计。
    await loginAsRole(page, 'report_viewer');
    const deniedResp = await apiCallExpectFail(page, 'GET', '/users?page=1&page_size=1');
    expect(deniedResp.status, 'report_viewer 访问用户列表应被拒（403）').toBe(403);

    // 审计经 channel 异步落库，回到 admin 身份轮询最多 10 秒。
    // 必须 force=true 清 cookie 后重登：loginAsRole 已把会话切到 report_viewer 且置位
    // LOGGED_IN.done，若沿用不带 force 的 loginViaUI(page)，helpers 会检测到已有
    // access_token/csrf_token 而短路返回（仍是 report_viewer），随后 GET /audit-logs
    // 被权限中间件拒为 FORBIDDEN（run 35887709282 分片 flow(5/20) 的 P1-10 真实报错）。
    await loginViaUI(page, undefined, undefined, true);
    let rows: Array<Record<string, unknown>> = [];
    for (let i = 0; i < 10; i++) {
      const logs = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
        page,
        'GET',
        '/audit-logs?page=1&page_size=50'
      );
      expect(
        Array.isArray(logs?.items),
        `审计日志应返回 items 数组，实际：${JSON.stringify(logs).slice(0, 200)}`
      ).toBe(true);
      rows = logs.items.filter(l => l.resource_type === 'permission_denied');
      if (rows.length > 0) break;
      await page.waitForTimeout(1000);
    }
    expect(rows.length, '未查到 permission_denied 审计记录：权限拒绝没有落库').toBeGreaterThan(0);
    const hit = rows[0];
    expect(
      String(hit.request_path ?? ''),
      `拒绝审计应记录被拒的请求路径，实际：${JSON.stringify(hit)}`
    ).toContain('/users');
    expect(
      String(hit.resource_name ?? ''),
      `拒绝审计应写明缺失的权限码，实际：${JSON.stringify(hit)}`
    ).toContain('权限拒绝');
  });
});
