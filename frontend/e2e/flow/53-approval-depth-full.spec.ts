import { test, expect, type Page } from '../diagnose-fixture';
import { pickListArray } from './ui-helpers';
import {
  loginViaUI,
  loginOnPage,
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
  tryCleanup,
  API_BASE,
  API_PREFIX,
  failureCode,
  APP_ERROR_CODES,
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

/**
 * 负例允许的 AppError 拒绝机器码（backend/src/utils/error.rs:461-476 error_code()）。
 * 原先写 `String(code ?? '')` 配大小写不敏感正则：既绕过类型收窄，又把机器码散进用例。
 * error_code() 只产出精确大写常量，故改为白名单精确成员判断（更严，不放宽）。
 */
const APP_REJECT_CODES: string[] = [
  APP_ERROR_CODES.BUSINESS_ERROR,
  APP_ERROR_CODES.VALIDATION_ERROR,
  APP_ERROR_CODES.BAD_REQUEST,
];
/**
 * 返回类型收窄为非可选：`expect(x).toBeTruthy()` 不产生类型收窄（Playwright expect 无
 * 断言签名），调用点取 sensitive.id / sensitive.code 会报 TS18048。
 * 找不到敏感角色本就是前置失败，在取数点抛错比在每个调用点写 `!` 更严格；
 * 调用点原有的 expect 断言全部保留，本函数只负责把"确实存在"写进类型。
 */
function pickSensitiveRole(roles: Array<{ id: number; code: string }>): {
  id: number;
  code: string;
} {
  const hit = roles.find(r => SENSITIVE_ROLE_CODES.includes(r.code));
  if (!hit) {
    throw new Error(
      `角色表里没有敏感角色（${SENSITIVE_ROLE_CODES.join('/')}），现有：${roles
        .map(r => r.code)
        .join(',')}`
    );
  }
  return hit;
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
    const usersData = await apiCallRaw<{ users?: Array<{ id: number; username: string }> }>(
      page,
      'GET',
      `/users?page=1&page_size=100&keyword=${APPROVER.username}`
    );
    // 后端 list_users（user_handler.rs:355-398）→ ApiResponse<UserListResponse>，
    // data = { users, total }（不是 items），单一形状直读 data.users。
    const userList = pickListArray<{ id: number; username: string }>(
      usersData,
      'users',
      '53 用户列表 /users'
    );
    const exists = userList.find(u => u.username === APPROVER.username);
    if (exists) {
      approverUserId = exists.id;
      console.log('[53-0] 审批人已存在 id=', exists.id);
      return;
    }
    // apiCall 返回完整 body {code,data,message}；RoleListResponse 在 data.roles 下
    const rolesResp = await apiCallRaw<{ roles: Array<{ id: number; code: string }> }>(
      page,
      'GET',
      '/roles?page=1&page_size=50'
    );
    // /roles -> RoleListResponse.roles（不是 items，也不是裸数组）
    const roleList = pickListArray<{ id: number; code: string }>(
      rolesResp,
      'roles',
      '53 角色列表 /roles'
    );
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
    const roleResp = await apiCallRaw<{ roles: Array<{ id: number; code: string }> }>(
      page,
      'GET',
      '/roles?page=1&page_size=50'
    );
    const roleList = pickListArray<{ id: number; code: string }>(
      roleResp,
      'roles',
      '53 敏感角色列表 /roles'
    );
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
    // 该防自审批文案构造点显式声明可外显（AppError::business_displayable，
    // role_change_approval_service.rs approve_l1），真实文案直接进 HTTP message；
    // code 与默认 business 一致（error.rs BusinessErrorDisplayable → BUSINESS_ERROR），两者都断。
    expect(APP_REJECT_CODES, 'code 应为业务拒绝而非系统故障').toContain(failureCode(self));
    expect(self.message, '应外显防自审批真实文案').toBe('审批人不能是申请人');

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
    // 双人约束文案构造点显式声明可外显（AppError::business_displayable，
    // role_change_approval_service.rs approve_l2）：message 应为真实文案；
    // 出参结构不变，code 仍为 BUSINESS_ERROR（error.rs 两个变体共用），两者都断。
    expect(String(l2.body?.code ?? ''), 'code 应为业务拒绝').toBe('BUSINESS_ERROR');
    expect(String(l2.body?.message ?? ''), '应外显双人约束真实文案').toBe(
      '二级审批人不能与一级审批人相同'
    );
  });

  test('53-3 敏感角色白名单：非敏感角色变更无需审批（:20 对照）', async ({ page }) => {
    await loginViaUI(page);
    // 后端 list_roles（role_handler.rs:113-145）→ ApiResponse<RoleListResponse>，
    // data = { roles, total }（不是 items），与同文件 :274 一致按 data.roles 单一读取。
    const rolesData = await apiCallRaw<{ roles?: Array<{ id: number; code: string }> }>(
      page,
      'GET',
      '/roles?page=1&page_size=50'
    );
    const roleList = pickListArray<{ id: number; code: string }>(
      rolesData,
      'roles',
      '53 角色列表 /roles'
    );
    const normal = roleList.find(
      r => !['admin', 'super_admin', 'finance', 'finance_admin'].includes(r.code)
    );
    expect(normal, '无非敏感角色可对照').toBeTruthy();
    if (!normal) throw new Error('无非敏感角色可对照');
    const r = await apiCallExpectFail(page, 'POST', '/role-change-approvals', {
      change_type: 'assign_role',
      target_role_id: normal.id,
      target_role_code: normal.code,
    });
    expect(r.status, '非敏感角色变更应直接拒绝（无需走审批）').toBeGreaterThanOrEqual(400);
    // 非敏感拒绝为 handler 的 AppError::business_displayable（role_change_approval_handler.rs:29-31），
    // 公开业务规则文案可外显；code 与默认 business 一致，两者都断
    expect(failureCode(r), 'code 应为业务拒绝（仅敏感角色需审批）').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    expect(r.message, '应外显敏感角色审批规则真实文案').toBe('只有敏感角色变更需要审批');
  });
});
