import { test, expect, type Page } from '../diagnose-fixture';
import {
  loginViaUI,
  loginOnPage,
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
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
 *   POST /role-change-approvals、/{id}/approve-l1、/{id}/approve-l2、/{id}/reject、/{id}/cancel
 * - approve-l1/l2 的请求体是 Json<ApproveRoleChangeRequest>（{comments}），
 *   空请求体在 Json 提取阶段即 400，永远到不了业务校验；
 * - 审批记录没有 DELETE 路由（405），清理只能走 /cancel
 *
 * 跨用户装置：context A = admin（创建人）；context B = 新建 approver 用户
 * （admin 角色）API 登录后执行审批——真实双身份。
 */

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'POST', c.path, c.label);
  CLEANUP.length = 0;
});

/**
 * 与 backend role_change_approval_service.rs SENSITIVE_ROLES 同序的敏感角色码。
 * create_approval 第一步就是 is_sensitive_role 判定，越界码直接 400「只有敏感角色变更需要审批」，
 * 所以用例不能再 `?? roles[0]` 回退到任意角色——那会把「找不到敏感角色」伪装成业务失败。
 */
const SENSITIVE_ROLE_CODES = ['admin', 'super_admin', 'finance', 'finance_admin'];
function pickSensitiveRole(roles: Array<{ id: number; code: string }>) {
  return roles.find(r => SENSITIVE_ROLE_CODES.includes(r.code));
}

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
  await page.goto(`${process.env.BASE_URL || 'http://localhost:3000'}/login`, {
    waitUntil: 'domcontentloaded',
  });
  // 登录表单没有 name 属性（Element Plus el-input 只有 placeholder），且必须勾选用户协议
  // 才能提交——原实现用 input[name="username"] 等待，永远等不到（CI: 60s 超时）。
  // 复用 loginViaUI 的同一套表单操作流程，避免两处选择器各自漂移。
  await loginOnPage(page, username, password);
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
  // 审批人是整个 describe 的共享前置账号，必须活到最后一个用例；
  // 放进逐用例 CLEANUP 会被 afterEach 删掉，后续用例登录 401（CI 53-1 之因）
  let approverUserId: number | undefined;
  const APPROVER = { username: `approver53`, password: 'Appr53!Sec#2026x' };

  test.beforeAll(async ({ browser: b }) => {
    browser = b;
  });

  test.afterAll(async () => {
    await secondCtx?.close();
    if (approverUserId) {
      const ctx = await browser.newContext();
      const page = await ctx.newPage();
      await loginViaUI(page);
      await apiCall(page, 'DELETE', `/users/${approverUserId}`);
      await ctx.close();
    }
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
      approverUserId = exists.id;
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
    approverUserId = uid;
    expect(uid, '审批人创建应成功').toBeTruthy();
  });

  test('53-1 角色变更：A 创建→A 自审 approve-l1 被拒（防自审批）→B 审批通过', async ({
    page,
    browser: b,
  }) => {
    await loginViaUI(page);
    // 查一个敏感角色 id（backend 只对 SENSITIVE_ROLES 内的码开审批流）
    const roleResp = await apiCallRaw<{ roles?: Array<{ id: number; code: string }> }>(
      page,
      'GET',
      '/roles?page=1&page_size=50'
    );
    const roleList = roleResp?.roles ?? [];
    const sensitive = pickSensitiveRole(roleList);
    expect(
      sensitive,
      `角色表里没有敏感角色（${SENSITIVE_ROLE_CODES.join('/')}），现有：${roleList
        .map(r => r.code)
        .join(',')}`
    ).toBeTruthy();

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
        change_type: 'assign_role',
        target_user_id: myId,
        target_role_id: sensitive.id,
        target_role_code: sensitive.code,
      }
    );
    const apId =
      (created as { id?: number })?.id ?? (created as { data?: { id?: number } })?.data?.id;
    expect(apId, '角色变更申请创建失败').toBeTruthy();
    CLEANUP.push({ path: `/role-change-approvals/${apId}/cancel`, label: '[53-1] 申请' });

    // A 自己审批 L1 → 必须被拒（:2 防自审批）
    // 必须传 body：后端签名是 Json<ApproveRoleChangeRequest>，空请求体在 Json 提取阶段
    // 就被 axum 以 422 拦截，永远到不了"审批人不能是申请人"的业务校验。
    const self = await apiCallExpectFail(
      page,
      'POST',
      `/role-change-approvals/${apId}/approve-l1`,
      { comments: '53-1 自审批负例：申请人审批自己的申请必须被拒' }
    );
    expect(self.status, '申请人自己审批必须被拒').toBeGreaterThanOrEqual(400);
    // 后端 HTTP 响应统一脱敏（utils/error.rs:96 走 public_message），真实文案"审批人不能是申请人"
    // 只进 tracing 日志，断言 code 而非 message 才能稳定命中业务拒绝分支。
    expect(String(self.code ?? ''), 'code 应为业务拒绝而非系统故障').toMatch(
      /BUSINESS|VALIDATION|BAD_REQUEST/i
    );

    // B（独立 context）审批 L1 → 通过
    const { ctx: ctxB, call: callB } = await loginSecondUser(
      b,
      APPROVER.username,
      APPROVER.password
    );
    secondCtx = ctxB;
    const bApprove = await callB('POST', `/role-change-approvals/${apId}/approve-l1`, {
      comments: '53-1 B 审批一级',
    });
    expect(bApprove.status, 'B 审批 L1 应通过').toBeLessThan(300);
  });

  test('53-2 角色变更：二级审批人≠一级审批人（:43 双人约束）', async ({ page, browser: b }) => {
    await loginViaUI(page);
    // GET /roles 出参是 RoleListResponse{roles,total}（既不是 items 也不是分页结构，
    // role_handler.rs 注明全量返回无分页，故不传 page/page_size）；
    // 原用例按 roles.items 取值恒为 undefined，被断成"无敏感角色"。
    const roles = await apiCallRaw<{ roles: Array<{ id: number; code: string }>; total: number }>(
      page,
      'GET',
      '/roles'
    );
    expect(
      Array.isArray(roles?.roles),
      `角色列表应在 data.roles，实际响应：${JSON.stringify(roles).slice(0, 200)}`
    ).toBe(true);
    const sensitive = pickSensitiveRole(roles.roles);
    expect(
      sensitive,
      `角色表里没有敏感角色（${SENSITIVE_ROLE_CODES.join('/')}），现有：${roles.roles
        .map(r => r.code)
        .join(',')}（total=${roles.total}）`
    ).toBeTruthy();

    const me = await apiCall<{ id?: number }>(page, 'GET', '/users/me');
    const myId =
      (me as { id?: number; data?: { id?: number } })?.id ??
      (me as { data?: { id?: number } })?.data?.id;

    const created = await apiCall<{ id?: number; data?: { id?: number } }>(
      page,
      'POST',
      '/role-change-approvals',
      {
        change_type: 'assign_role',
        target_user_id: myId,
        target_role_id: sensitive.id,
        target_role_code: sensitive.code,
      }
    );
    const apId =
      (created as { id?: number })?.id ?? (created as { data?: { id?: number } })?.data?.id;
    expect(apId, '申请创建失败').toBeTruthy();
    CLEANUP.push({ path: `/role-change-approvals/${apId}/cancel`, label: '[53-2] 申请' });

    // B 完成 L1
    const { ctx: ctxB, call: callB } = await loginSecondUser(
      b,
      APPROVER.username,
      APPROVER.password
    );
    secondCtx = ctxB;
    const l1 = await callB('POST', `/role-change-approvals/${apId}/approve-l1`, {
      comments: '53-2 B 审批一级',
    });
    expect(l1.status, 'B 一级审批应通过').toBeLessThan(300);

    // L1 审批人 B 尝试继续做 L2 → 双人约束拒绝
    const l2 = await callB('POST', `/role-change-approvals/${apId}/approve-l2`, {
      comments: '53-2 B 审批二级',
    });
    expect(l2.status, '二级审批人不能与一级相同').toBeGreaterThanOrEqual(400);
    // 业务错误文案经 utils/error.rs:96 public_message 脱敏，响应 message 恒为固定文案，
    // 真实文案只进 tracing；approve_l2 该分支为 AppError::business
    // （role_change_approval_service.rs:186-188）→ 稳定 code = BUSINESS_ERROR（error.rs:413）
    expect(String(l2.body?.code ?? ''), 'code 应为业务拒绝').toBe('BUSINESS_ERROR');
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
      change_type: 'assign_role',
      target_role_id: normal.id,
      target_role_code: normal.code,
    });
    expect(r.status, '非敏感角色变更应直接拒绝（无需走审批）').toBeGreaterThanOrEqual(400);
    // 非敏感拒绝是 handler 的 AppError::business（role_change_approval_handler.rs:29-31），
    // message 被脱敏（utils/error.rs:94-99），只能断稳定 code=BUSINESS_ERROR
    expect(String(r.code ?? ''), 'code 应为业务拒绝（仅敏感角色需审批）').toBe('BUSINESS_ERROR');
  });
});
