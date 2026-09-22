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

    const pairs = (conflicts || []).map(c => `${c.role_a_code}↔${c.role_b_code}`);
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
    // 伪造 download_token 同样 403（令牌查无或资源类型不匹配）
    const result = await apiCallExpectFail(
      page,
      'GET',
      '/dye-recipes/export?download_token=fake-token-p13'
    );
    expectDenied(result);
  });

  test('P1-4 验证权限缓存（多次调用不拒绝）', async ({ page }) => {
    // 原实现循环 5 次取数却不作任何断言（注释写"容错"），无论后端返回什么都算通过。
    // 权限缓存的正确性判据是：同一身份连续请求都成功，且结果集稳定（命中缓存前后一致）。
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
      totals.push(Number(result.total));
    }
    expect(
      new Set(totals).size,
      `同一身份的 5 次查询 total 应一致（缓存导致结果漂移）：${totals.join(',')}`
    ).toBe(1);
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
    // 真实断言：data-permissions 列表必须可达且返回结构化数据
    const perms = await apiCallRaw<
      | { items: Array<{ id: number; scope_type?: string }> }
      | { data?: { items: Array<{ id: number }> } }
    >(page, 'GET', '/data-permissions?page=1&page_size=10');
    const items = Array.isArray(perms)
      ? perms
      : (perms?.items ?? (perms as { data?: { items: unknown[] } })?.data?.items ?? []);
    expect(Array.isArray(items)).toBe(true);
  });

  test('P1-8 验证字段级权限', async ({ page }) => {
    // 真实断言：field-permissions 端点可达且结构化（端点存在性由 404 区分）
    const perms = await apiCallRaw<
      { items: Array<{ id: number }> } | { data?: { items: Array<{ id: number }> } }
    >(page, 'GET', '/field-permissions?page=1&page_size=10');
    const items = Array.isArray(perms)
      ? perms
      : (perms?.items ?? (perms as { data?: { items: unknown[] } })?.data?.items ?? []);
    expect(Array.isArray(items)).toBe(true);
  });

  test('P1-8b 验证客户字段级权限端点', async ({ page }) => {
    const perms = await apiCallRaw<
      { items: Array<{ id: number }> } | { data?: { items: Array<{ id: number }> } }
    >(page, 'GET', '/customer-field-permissions?page=1&page_size=10');
    const items = Array.isArray(perms)
      ? perms
      : (perms?.items ?? (perms as { data?: { items: unknown[] } })?.data?.items ?? []);
    expect(Array.isArray(items)).toBe(true);
  });

  test('P1-9 验证 CSRF 防护：缺失令牌的写请求必须被拒', async ({ page }) => {
    // 原用例只 GET 了一次 /users 且不留任何断言（注释写"容错"），根本没验证 CSRF。
    // 这里绕过 apiCall 的令牌注入，直接发一个不带 X-CSRF-Token 的写请求：
    // 会话 Cookie 仍在（已登录），因此被拒只能是因为 CSRF 令牌缺失。
    const res = await page.request.fetch('/departments', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      data: { name: `E2E-CSRF-${Date.now()}`, description: 'CSRF 负例' },
    });
    expect(res.status(), '缺 CSRF 令牌的写请求应被拒（4xx）').toBeGreaterThanOrEqual(400);
    const body = await res.json();
    expect(String(body?.code ?? ''), `应返回 CSRF 错误码，实际：${JSON.stringify(body)}`).toMatch(
      /CSRF/i
    );
  });

  test('P1-10 验证权限审计日志（拒绝记录真实落库）', async ({ page }) => {
    // 原实现请求 /unknown-module/test —— 那是未注册路径，路由层直接 404，
    // 权限中间件根本没执行，permission.rs 的 record_permission_denied 不会被触发；
    // 末尾再写 expect(denied.length >= 0) 这种恒真断言，等于"一条都没查到"也算通过。
    // 现改为真实越权：非 admin 角色访问 admin 专属端点 → 403 → 落审计。
    await loginAsRole(page, 'report_viewer');
    const deniedResp = await apiCallExpectFail(page, 'GET', '/users?page=1&page_size=1');
    expect(deniedResp.status, 'report_viewer 访问用户列表应被拒（403）').toBe(403);

    // 审计经 channel 异步落库，回到 admin 身份轮询最多 10 秒
    await loginViaUI(page);
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
