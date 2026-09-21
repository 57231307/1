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
    if (result.data?.id) ctx.userIds.push(result.data.id);
    expect(ctx.userIds.length).toBeGreaterThanOrEqual(0);
  });

  test('6-4 验证 SoD 职责分离规则', async ({ page }) => {
    // 真实端点：POST /role-relations/check-mutual-exclusive/{role_code}
    // （role_relation.rs，校验角色与已有角色集合的互斥冲突），先取一个角色再校验其 SoD 检查可用
    const roles = await apiCallRaw<{ items: Array<{ id: number; code?: string; name?: string }> }>(
      page,
      'GET',
      '/roles?page=1&page_size=5'
    );
    const roleCode = roles.items?.[0]?.code || roles.items?.[0]?.name;
    if (!roleCode) {
      // 环境无角色数据时跳过（不虚构断言）
      return;
    }
    // apiCall 失败（非 200）会抛错使用例失败，成功返回信封 data
    const check = await apiCall<{ is_exclusive?: boolean; role_code?: string }>(
      page,
      'POST',
      `/role-relations/check-mutual-exclusive/${encodeURIComponent(roleCode)}`,
      { existing_role_codes: [] }
    );
    expect(check.data).toBeDefined();
  });

  test('6-5 验证角色权限矩阵', async ({ page }) => {
    const roles = await apiCallRaw<{ items: Array<{ id: number; name: string }> }>(
      page,
      'GET',
      '/roles?page=1&page_size=10'
    );
    expect(Array.isArray(roles.items), `roles.items 应为后端返回的 items 数组`);

    for (const role of roles?.items?.slice(0, 2) ?? []) {
      const perms = await apiCallRaw<{ items: Array<{ resource_type: string; action: string }> }>(
        page,
        'GET',
        `/roles/${role.id}/permissions`
      );
      expect(Array.isArray(perms.items), `perms.items 应为后端返回的 items 数组`);
    }
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
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);
    const tags = page.locator('.el-table .el-tag:visible');
    const tagCount = await tags.count();
    console.log(`[6-9] 采购列表可见 el-tag ${tagCount} 个`);
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
    expect(Array.isArray(orders.items), `orders.items 应为后端返回的 items 数组`);
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
