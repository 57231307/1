import { test, expect } from '@playwright/test';
import { loginAsRole, apiCallExpectFail } from './helpers';

/**
 * P5.3 垂直越权端点矩阵
 *
 * admin 专属端点族（require_admin_role 保护，grep backend handlers 确认）：
 * - users（用户管理：195/312 行 admin 校验）
 * - roles（角色管理：192/267/373 行）
 * - data-permissions（137/173/191 行）
 * - field-permissions（81/114/142 行）
 * - system-update（126/183/303 行）
 * - audit-logs（146/237/364 行）
 * - login-security 管理（锁定列表）
 *
 * 验证：非 admin 角色（report_viewer / cashier）逐端点断言 403，
 *       admin 自身 200 对照（区分"端点不存在"与"权限拒绝"）。
 * 修复 09-permissions 恒真断言后的真实越权矩阵（本 spec 承接详尽断言）。
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

/** admin 专属只读端点（GET，越权测试主要面） */
const ADMIN_READONLY_ENDPOINTS = [
  '/users?page=1&page_size=5',
  '/roles?page=1&page_size=5',
  '/data-permissions?page=1&page_size=5',
  '/field-permissions?page=1&page_size=5',
  '/system-update/version',
  '/system-update/status',
  '/audit-logs?page=1&page_size=5',
];

/** admin 专属写端点（POST/PUT，非 admin 必须 403） */
const ADMIN_WRITE_ENDPOINTS: Array<{ method: string; path: string; body?: Record<string, unknown> }> = [
  { method: 'POST', path: '/users', body: { username: 'e2e_vpriv_user', password: 'Xk9#mQ2$vL8pW4nR', role_id: 1 } },
  { method: 'POST', path: '/roles', body: { code: 'e2e_vpriv_role', name: 'VPriv 测试角色' } },
  { method: 'POST', path: '/data-permissions', body: { scope_type: 'self' } },
];

/** 非 admin 角色（走 role-credentials.json 或 env） */
const NON_ADMIN_ROLES = ['report_viewer', 'cashier'];

test.describe('P5.3 垂直越权矩阵', () => {
  for (const role of NON_ADMIN_ROLES) {
    test.describe(`角色 ${role}`, () => {
      for (const ep of ADMIN_READONLY_ENDPOINTS) {
        test(`GET ${ep} → 403`, async ({ page }) => {
          await loginAsRole(page, role);

          const resp = await page.request
            .get(`${API_BASE}${API_PREFIX}${ep}`)
            .catch((e) => { console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`); return null; });
          if (!resp) throw new Error(`网络错误: ${ep}`);

          const status = resp.status();
          // 非 admin 对 admin 端点：403=权限拒绝（正确）；401=会话问题（真缺陷）；200=越权（真缺陷）
          expect(
            status,
            `${role} 访问 ${ep} 应 403，实际 ${status}（${status === 200 ? '越权可达——权限缺陷' : status === 401 ? '401 会话异常' : '其他状态'}`,
          ).toBe(403);
        });
      }

      for (const ep of ADMIN_WRITE_ENDPOINTS) {
        test(`${ep.method} ${ep.path} → 403`, async ({ page }) => {
          await loginAsRole(page, role);

          // 带 CSRF 头消除歧义：403 只能来自权限拒绝（CSRF 缺失同样返回 403）
          const cookies = await page.context().cookies();
          const csrf = cookies.find((c) => c.name === 'csrf_token');

          const resp = await page.request
            .post(`${API_BASE}${API_PREFIX}${ep.path}`, {
              data: ep.body ?? {},
              headers: {
                'X-Requested-With': 'XMLHttpRequest',
                'Content-Type': 'application/json',
                ...(csrf ? { 'X-CSRF-Token': csrf.value } : {}),
              },
            })
            .catch((e) => { console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`); return null; });
          if (!resp) throw new Error(`网络错误: ${ep.path}`);

          const status = resp.status();
          expect(status, `${role} 写操作 ${ep.path} 应 403，实际 ${status}`).toBe(403);
        });
      }
    });
  }

  test('admin 对照：同端点可达（区分端点缺失与权限拒绝）', async ({ page }) => {
    await loginAsRole(page, 'admin');

    const reachable: string[] = [];
    const unexpected: string[] = [];
    for (const ep of ADMIN_READONLY_ENDPOINTS) {
      const resp = await page.request.get(`${API_BASE}${API_PREFIX}${ep}`).catch((e) => { console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`); return null; });
      if (!resp) continue;
      const status = resp.status();
      if (status < 400) reachable.push(ep);
      else if (status !== 404 && status !== 400) unexpected.push(`${ep}=${status}`);
    }

    // admin 至少可达一半端点（种子库 users/roles/audit-logs 必在）
    expect(
      reachable.length,
      `admin 可达端点数 ${reachable.length} 过少（${reachable.join(', ')}），端点族可能整体失效`,
    ).toBeGreaterThanOrEqual(4);
  });
});
