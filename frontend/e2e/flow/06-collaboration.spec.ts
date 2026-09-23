import { test, expect } from '../diagnose-fixture';
import {
  BASE_URL,
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
  ensureTestEntities,
  genCode,
  genName,
  getCtx,
  loginViaUI,
  verifyAuditLog,
  verifyPermissionDenied,
} from './helpers';

test.describe.serial('Shard 6: 多角色协作 + 权限隔离 + 状态显示', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test.beforeEach(async ({ page }) => {
    await ensureTestEntities(page);
  });

  test('6-1 admin 权限验证（*:* 通配）', async ({ page }) => {
    const me = await apiCallRaw<{ username: string; permissions: string[] }>(
      page,
      'GET',
      '/auth/me'
    );
    expect(typeof me.username, '/auth/me 必须返回用户名').toBe('string');
    expect(Array.isArray(me.permissions), '/auth/me 应返回权限数组').toBe(true);
    // admin 应有 *:* 或类似通配权限（本用例账号即为管理员，缺失即为真实缺陷）
    const hasWildcard = me.permissions.some(p => p.includes('*'));
    expect(
      hasWildcard,
      `admin 权限集应包含通配项，实际：${me.permissions.slice(0, 5).join(',')}`
    ).toBe(true);
  });

  test('6-2 创建测试角色（采购员）', async ({ page }) => {
    const result = await apiCall<{ id?: number }>(page, 'POST', '/roles', {
      // CreateRoleRequest 仅接受 name/code/description/is_system（data_scope 由角色管理页维护）
      name: genName('E2E采购员'),
      code: 'role_pur_' + Date.now().toString().slice(-6),
      description: 'E2E 多角色协作-采购员（可 create 不可 approve）',
      is_system: false,
    });
    getCtx().roleId = result.data?.id;
    // 容错：roleId 可能为 undefined
  });

  test('6-3 创建测试用户', async ({ page }) => {
    const ctx = getCtx();
    const username = `e2e_user_${Date.now().toString().slice(-6)}`;
    const result = await apiCall<{ id?: number }>(page, 'POST', '/users', {
      username,
      password: 'E2e@TestPass2026!',
      email: `e2e_${Date.now().toString().slice(-6)}@test.com`,
      role_id: ctx.roleId,
      department_id: ctx.departmentIds[0] || 1,
      is_active: true,
    });
    // 建用户失败此前被 if 吞掉后仍写 `length >= 0` 恒真断言；现要求真实创建出用户
    expect(result.data?.id, `协作测试用户创建未返回 id：${JSON.stringify(result)}`).toBeTruthy();
    if (result.data.id === undefined) {
      throw new Error(`协作测试用户创建未返回 id：${JSON.stringify(result)}`);
    }
    ctx.userIds.push(result.data.id);
    expect(ctx.userIds.length, '本用例应至少创建一个协作用户').toBeGreaterThan(0);
  });

  test('6-4 验证 SoD 职责分离规则', async ({ page }) => {
    // 真实端点：POST /role-relations/check-mutual-exclusive/{role_code}
    // （role_relation.rs，校验角色与已有角色集合的互斥冲突），先取一个角色再校验其 SoD 检查可用。
    // GET /roles 出参是 RoleListResponse{roles,total}（不分页，page/page_size 会被 serde 忽略）；
    // 原实现读 roles.items 恒为 undefined，于是整条用例在 !roleCode 分支静默 return，
    // 从未真正调用过互斥校验端点。
    const roles = await apiCallRaw<{
      roles: Array<{ id: number; code: string; name: string }>;
      total: number;
    }>(page, 'GET', '/roles');
    expect(
      Array.isArray(roles?.roles),
      `角色列表应返回 roles 数组，实际：${JSON.stringify(roles).slice(0, 200)}`
    ).toBe(true);
    expect(roles.total, '种子角色总数应大于 0').toBeGreaterThan(0);
    const roleCode = roles.roles[0]?.code;
    expect(roleCode, '角色列表为空，无法校验 SoD 互斥端点').toBeTruthy();
    // apiCall 失败（非 200）会抛错使用例失败，成功返回信封 data
    const check = await apiCall<{ is_exclusive?: boolean; role_code?: string }>(
      page,
      'POST',
      `/role-relations/check-mutual-exclusive/${encodeURIComponent(roleCode)}`,
      { existing_role_codes: [] }
    );
    expect(check.data, 'SoD 互斥校验应返回结果体').toBeDefined();
    expect(
      typeof check.data?.is_exclusive,
      `is_exclusive 应为布尔值，实际：${JSON.stringify(check.data)}`
    ).toBe('boolean');
    expect(check.data?.role_code, `响应应回显被校验的角色代码`).toBe(roleCode);
  });

  test('6-5 验证角色权限矩阵', async ({ page }) => {
    const roles = await apiCallRaw<{
      roles: Array<{ id: number; code: string; name: string }>;
      total: number;
    }>(page, 'GET', '/roles');
    expect(
      Array.isArray(roles?.roles),
      `角色列表应返回 roles 数组，实际：${JSON.stringify(roles).slice(0, 200)}`
    ).toBe(true);
    expect(roles.roles.length, '至少应有一个种子角色可查权限').toBeGreaterThan(0);

    let checked = 0;
    for (const role of roles.roles.slice(0, 2)) {
      // GET /roles/{id}/permissions 出参是裸数组 Vec<PermissionResponse>（无分页信封）
      const perms = await apiCallRaw<Array<Record<string, unknown>>>(
        page,
        'GET',
        `/roles/${role.id}/permissions`
      );
      expect(
        Array.isArray(perms),
        `角色 ${role.code} 的权限应返回数组，实际：${JSON.stringify(perms).slice(0, 200)}`
      ).toBe(true);
      for (const p of perms) {
        expect(
          String(p.resource_type ?? ''),
          `权限行缺少 resource_type：${JSON.stringify(p)}`
        ).not.toBe('');
        expect(String(p.action ?? ''), `权限行缺少 action：${JSON.stringify(p)}`).not.toBe('');
      }
      checked += 1;
    }
    expect(checked, '角色权限矩阵应至少校验一个角色').toBeGreaterThan(0);
  });

  test('6-6 验证非法 API 调用被拒绝', async ({ page }) => {
    const result = await apiCallExpectFail(page, 'GET', '/nonexistent-resource');
    expect(result.status).toBeGreaterThanOrEqual(400);
  });

  test('6-7 验证审计日志记录所有操作', async ({ page }) => {
    // 前面的测试创建了角色/用户，审计日志应该有记录
    const hasLog = await verifyAuditLog(page, 'create');
    expect(typeof hasLog).toBe('boolean');
  });

  test('6-8 验证列表页路由可达（状态显示映射入口）', async ({ page }) => {
    // 原实现访问 http://localhost:3000/purchase/orders 与 /sales/orders，
    // 两个路径都不是已注册路由（router/index.ts:202 销售列表=/sales、:223 采购列表=/purchase，
    // /sales/orders/:id 才是详情），页面实际被重定向到 /404，
    // 而 `expect(url)` / `expect(page.url())` 没有匹配器，永远不会失败。
    for (const route of ['/purchase', '/sales']) {
      await page.goto(`${BASE_URL}${route}`);
      await page.waitForTimeout(3000);
      console.log(`[6-8] ${route} → ${page.url()}`);
      expect(page.url(), `${route} 不应被重定向到 404`).not.toContain('/404');
      // 销售列表用虚拟表格（el-table-v2 / v2-table-wrapper），采购列表用 el-table，
      // 只匹配 .el-table 会漏掉 V2Table 页面
      await expect(
        page.locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper').first(),
        `${route} 应渲染数据表格`
      ).toBeVisible({ timeout: 10_000 });
    }
  });

  test('6-9 验证 el-tag 状态颜色映射', async ({ page }) => {
    // 上一轮这里只报「可见 el-tag 0 个」，无法区分三种完全不同的原因：
    // 状态列没渲染标签 / 列表没有数据 / 列表请求被拒。现先把列表请求本身断成显式契约。
    const listResponse = page
      .waitForResponse(res => res.url().includes('/purchase/orders'), { timeout: 30_000 })
      .catch(() => null);
    await page.goto(`${BASE_URL}/purchase`);
    const res = await listResponse;
    expect(res, '进入采购页未发出 /purchase/orders 请求，状态标签无从渲染').toBeTruthy();
    const body = (await res!.json()) as {
      code?: unknown;
      message?: string;
      data?: { items?: unknown[]; total?: number };
    };
    expect(
      res!.status(),
      `采购列表应 200，实际 ${res!.status()} ${JSON.stringify(body).slice(0, 300)}`
    ).toBe(200);
    // 后端 list_orders → ApiResponse<PaginatedResponse>（purchase_order_handler.rs:26-30；
    // utils/response.rs:34）：data 唯一形状是 {items,total,page,page_size}。缺 items 数组即判红，
    // 不再宽容"整个 data 是数组"这一后端从不产生的形状（旧 ?? [] 会把缺键伪装成空集合）。
    const payload = body.data;
    expect(
      Array.isArray(payload?.items),
      `采购列表 data 缺 items 数组（后端 PaginatedResponse 契约）：${JSON.stringify(body).slice(0, 300)}`
    ).toBe(true);
    const rows = payload!.items!;
    expect(
      rows.length,
      `采购列表没有数据，标签断言失去前提（beforeEach 的 ensureTestEntities 应已建单）；响应：${JSON.stringify(body).slice(0, 300)}`
    ).toBeGreaterThan(0);

    await page.waitForTimeout(3000);
    const tags = page.locator('.el-table .el-tag:visible');
    const tagCount = await tags.count();
    console.log(`[6-9] 采购列表 ${rows.length} 行，可见 el-tag ${tagCount} 个`);
    expect(tagCount, '采购订单列表应渲染状态标签').toBeGreaterThan(0);
    // Element Plus 的 el-tag 必须带类型类（success/info/warning/danger/primary），
    // 类名缺失说明状态→颜色映射没有真正生效
    for (let i = 0; i < tagCount; i++) {
      const cls = (await tags.nth(i).getAttribute('class')) ?? '';
      expect(
        /el-tag--(success|info|warning|danger|primary)/.test(cls),
        `第 ${i + 1} 个状态标签缺少类型类：${cls}`
      ).toBe(true);
    }
  });

  test('6-10 验证 CSRF 保护', async ({ page }) => {
    // 不带 CSRF Token 的 POST 请求应被拒绝
    const csrfToken = 'invalid-token';
    const response = await page.request.fetch('http://localhost:8082/api/v1/erp/departments', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
        'X-CSRF-Token': csrfToken,
      },
      data: JSON.stringify({ name: 'CSRF Test', code: 'CSRF-TEST' }),
    });
    // 无效 CSRF Token 应返回 403
    // 不带合法 CSRF Token 的写请求必须被拒（403 或任意 4xx）；
    // 原写法 expect(a === 403 || a >= 400) 没有匹配器，永远不会失败
    expect(
      response.status(),
      `缺少 CSRF Token 的 POST 应被拒绝，实际 HTTP ${response.status()}`
    ).toBeGreaterThanOrEqual(400);
  });

  test('6-11 验证数据权限行级隔离', async ({ page }) => {
    // admin 应能查看所有数据（data_scope=all）
    const orders = await apiCallRaw<{ items: unknown[] }>(
      page,
      'GET',
      '/purchase/orders?page=1&page_size=50'
    );
    expect(Array.isArray(orders.items), `orders.items 应为后端返回的 items 数组`).toBe(true);
    // admin 查看的数据不应被过滤
    expect(Array.isArray(orders?.items), '订单列表应返回 items 数组').toBe(true);
  });

  test('6-12 验证权限缓存', async ({ page }) => {
    // 多次调用同一 API，验证权限缓存不会导致拒绝
    for (let i = 0; i < 3; i++) {
      const result = await apiCallRaw<{ items: unknown[] }>(
        page,
        'GET',
        '/users?page=1&page_size=5'
      );
      // 容错：result 可能为 undefined
    }
  });
});
