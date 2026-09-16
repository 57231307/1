import { test, expect, type Page } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallExpectFail,
  tryCleanup,
  API_BASE,
  API_PREFIX,
} from './helpers';

/**
 * 53 L4 审批纵深完整版（跨用户装置）
 *
 * rule provenance（role_change_approval_service.rs）：
 * - :20 敏感角色 admin/super_admin/finance/finance_admin 才需审批
 * - :2 申请人==审批人拒绝（防自审批）
 * - :43 二级审批人不能与一级审批人相同（双人约束）
 * - 端点 routes/system.rs / handlers/role_change_approval_handler.rs:20-117
 *   POST /role-change-approvals、/{id}/approve-l1、/{id}/approve-l2、/{id}/reject
 *
 * 跨用户装置：context A = admin（创建人）；context B = 新建 approver 用户
 * （admin 角色）API 登录后执行审批——真实双身份。
 */

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 在独立 context 中以用户名密码 API 登录，返回可用于 apiCall 风格请求的函数 */
async function loginSecondUser(
  browser: import('@playwright/test').Browser,
  username: string,
  password: string
): Promise<{
  ctx: import('@playwright/test').BrowserContext;
  call: (
    method: string,
    path: string,
    body?: Record<string, unknown>
  ) => Promise<{ status: number; body: Record<string, unknown> }>;
}> {
  const ctx = await browser.newContext();
  const page = await ctx.newPage();
  await page.goto(`${process.env.BASE_URL || 'http://localhost:3000'}/login`);
  await page.fill('input[name="username"]', username);
  await page.fill('input[name="password"]', password);
  await page.click('button[type="submit"]');
  await page.waitForURL(/dashboard|\/$/, { timeout: 30_000 });
  const call = async (method: string, path: string, body?: Record<string, unknown>) => {
    const cookies = await ctx.cookies();
    const csrf = cookies.find(c => c.name === 'csrf_token');
    const resp = await page.request.fetch(`${API_BASE}${API_PREFIX}${path}`, {
      method,
      headers: {
        'Content-Type': 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
        ...(csrf ? { 'X-CSRF-Token': csrf.value } : {}),
      },
      data: body ? JSON.stringify(body) : undefined,
      timeout: 60_000,
    });
    const text = await resp.text();
    let parsed: Record<string, unknown> = {};
    try {
      parsed = JSON.parse(text);
    } catch {
      /* 非 JSON 容错 */
    }
    return { status: resp.status(), body: parsed };
  };
  return { ctx, call };
}

test.describe.serial('53 审批纵深：防自审批+双人约束（跨用户）', () => {
  let browser: import('@playwright/test').Browser;
  let secondCtx: import('@playwright/test').BrowserContext | null = null;
  const APPROVER = { username: `approver53`, password: 'Appr53!Sec#2026x' };

  test.beforeAll(async ({ browser: b }) => {
    browser = b;
  });

  test.afterAll(async () => {
    await secondCtx?.close();
  });

  test('53-0 装置：创建二级审批人用户（admin 角色）', async ({ page }) => {
    await loginViaUI(page);
    // 幂等：先查是否已存在
    const list = await apiCall<{ items?: Array<{ id: number; username: string }> }>(
      page,
      'GET',
      `/users?page=1&page_size=100&keyword=${APPROVER.username}`
    );
    const exists = list?.items?.find(u => u.username === APPROVER.username);
    if (exists) {
      console.log('[53-0] 审批人已存在 id=', exists.id);
      return;
    }
    // apiCall 返回完整 body {code,data,message}；RoleListResponse 在 data.roles 下
    const rolesResp = await apiCallRaw<{ roles?: Array<{ id: number; code: string }> }>(
      page,
      'GET',
      '/roles?page=1&page_size=50'
    );
    const roleList = rolesResp?.roles ?? [];
    const adminRole = roleList.find(r => r.code === 'admin') ?? roleList[0];
    expect(adminRole, '无可用角色').toBeTruthy();
    const r = await apiCall<{ id?: number }>(page, 'POST', '/users', {
      username: APPROVER.username,
      password: APPROVER.password,
      email: `${APPROVER.username}@test.com`,
      role_id: adminRole.id,
    });
    const uid = r?.data?.id;
    if (uid) CLEANUP.push({ path: `/users/${uid}`, label: '[53-0] 审批人' });
    expect(uid, '审批人创建应成功').toBeTruthy();
  });

  test('53-1 角色变更：A 创建→A 自审 approve-l1 被拒（防自审批）→B 审批通过', async ({
    page,
    browser: b,
  }) => {
    await loginViaUI(page);
    // 查一个敏感角色 id（admin/finance）
    const roles = await apiCall<{ items?: Array<{ id: number; code: string }> }>(
      page,
      'GET',
      '/roles?page=1&page_size=50'
    );
    const sensitive =
      roles?.items?.find(r =>
        ['admin', 'finance', 'finance_admin', 'super_admin'].includes(r.code)
      ) ?? roles?.items?.[0];
    expect(sensitive, '无敏感角色').toBeTruthy();

    const me = await apiCall<{ id?: number }>(page, 'GET', '/users/me');
    const myId =
      (me as { id?: number; data?: { id?: number } })?.id ??
      (me as { data?: { id?: number } })?.data?.id;

    // A 创建角色变更申请（change_type 需为有效值，grant 常用）
    const created = await apiCall<{ id?: number; data?: { id?: number } }>(
      page,
      'POST',
      '/role-change-approvals',
      {
        change_type: 'grant',
        target_user_id: myId,
        target_role_id: sensitive.id,
        target_role_code: sensitive.code,
      }
    );
    const apId =
      (created as { id?: number })?.id ?? (created as { data?: { id?: number } })?.data?.id;
    expect(apId, '角色变更申请创建失败').toBeTruthy();
    CLEANUP.push({ path: `/role-change-approvals/${apId}`, label: '[53-1] 申请' });

    // A 自己审批 L1 → 必须被拒（:2 防自审批）
    const self = await apiCallExpectFail(page, 'POST', `/role-change-approvals/${apId}/approve-l1`);
    expect(self.status, '申请人自己审批必须被拒').toBeGreaterThanOrEqual(400);
    expect(String(self.message ?? ''), '拒绝消息应提示本人/自己').toMatch(
      /本人|自己|self|applicant/i
    );

    // B（独立 context）审批 L1 → 通过
    const { ctx: ctxB, call: callB } = await loginSecondUser(
      b,
      APPROVER.username,
      APPROVER.password
    );
    secondCtx = ctxB;
    const bApprove = await callB('POST', `/role-change-approvals/${apId}/approve-l1`);
    expect(bApprove.status, 'B 审批 L1 应通过').toBeLessThan(300);
  });

  test('53-2 角色变更：二级审批人≠一级审批人（:43 双人约束）', async ({ page, browser: b }) => {
    await loginViaUI(page);
    const roles = await apiCall<{ items?: Array<{ id: number; code: string }> }>(
      page,
      'GET',
      '/roles?page=1&page_size=50'
    );
    const sensitive =
      roles?.items?.find(r => ['finance', 'finance_admin'].includes(r.code)) ?? roles?.items?.[0];
    expect(sensitive, '无敏感角色').toBeTruthy();

    const me = await apiCall<{ id?: number }>(page, 'GET', '/users/me');
    const myId =
      (me as { id?: number; data?: { id?: number } })?.id ??
      (me as { data?: { id?: number } })?.data?.id;

    const created = await apiCall<{ id?: number; data?: { id?: number } }>(
      page,
      'POST',
      '/role-change-approvals',
      {
        change_type: 'grant',
        target_user_id: myId,
        target_role_id: sensitive.id,
        target_role_code: sensitive.code,
      }
    );
    const apId =
      (created as { id?: number })?.id ?? (created as { data?: { id?: number } })?.data?.id;
    expect(apId, '申请创建失败').toBeTruthy();
    CLEANUP.push({ path: `/role-change-approvals/${apId}`, label: '[53-2] 申请' });

    // B 完成 L1
    const { ctx: ctxB, call: callB } = await loginSecondUser(
      b,
      APPROVER.username,
      APPROVER.password
    );
    secondCtx = ctxB;
    const l1 = await callB('POST', `/role-change-approvals/${apId}/approve-l1`);
    expect(l1.status, 'B 一级审批应通过').toBeLessThan(300);

    // L1 审批人 B 尝试继续做 L2 → 双人约束拒绝
    const l2 = await callB('POST', `/role-change-approvals/${apId}/approve-l2`);
    expect(l2.status, '二级审批人不能与一级相同').toBeGreaterThanOrEqual(400);
    expect(String(l2.body?.message ?? '')).toContain('一级审批人相同');
  });

  test('53-3 敏感角色白名单：非敏感角色变更无需审批（:20 对照）', async ({ page }) => {
    await loginViaUI(page);
    const roles = await apiCall<{ items?: Array<{ id: number; code: string }> }>(
      page,
      'GET',
      '/roles?page=1&page_size=50'
    );
    const normal = roles?.items?.find(
      r => !['admin', 'super_admin', 'finance', 'finance_admin'].includes(r.code)
    );
    expect(normal, '无非敏感角色可对照').toBeTruthy();
    const r = await apiCallExpectFail(page, 'POST', '/role-change-approvals', {
      change_type: 'grant',
      target_role_id: normal.id,
      target_role_code: normal.code,
    });
    expect(r.status, '非敏感角色变更应直接拒绝（无需走审批）').toBeGreaterThanOrEqual(400);
    expect(String(r.message ?? ''), '消息应提示仅敏感角色').toContain('敏感');
  });
});
