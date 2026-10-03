// 数据范围行级隔离 E2E — enhanced/data-scope-isolation
// 前置说明：CI 不执行 #[ignore] 的 RLS 集成测试，本 spec 以真实 API 回读证明
//   应用层 data_scope 过滤（self/dept/all）按 user_id 正确限制可见行，
//   非 localStorage mock。
// 核心验证逻辑：
//   1. 两个不同 user_id 且 role.data_scope=self 的用户各自登录
//   2. User A 建线索（owner_id = A 的 user_id）
//   3. User B 列表 GET → 看不到 A 的私有行
//   4. User A 列表 GET → 能看到自己的行
//   5. admin (data_scope=all) 能看到所有行
//   6. 按 ID GET 越权 → 403 permission_denied（IDOR 防护验证）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  genCode,
  tryCleanup,
  getRoleCredential,
  BASE_URL,
  API_BASE,
  API_PREFIX,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/**
 * 以指定角色登录并在该上下文中创建一条线索，返回 lead ID。
 * 使用独立 BrowserContext 保证 cookie 隔离。
 */
async function createUserAndSeedLead(
  browser: import('@playwright/test').Browser,
  role: string,
  leadCompanyPrefix: string
): Promise<{
  context: import('@playwright/test').BrowserContext;
  page: import('@playwright/test').Page;
  userId: number;
  leadId: number;
  leadNo: string;
}> {
  const cred = getRoleCredential(role);
  // global-setup ensureRoleUsers 必须保证角色凭证就绪；缺失属 setup 缺陷，判红不 skip 掩盖
  if (!cred) {
    throw new Error(
      `角色 ${role} 凭证不存在——global-setup ensureRoleUsers 未正确执行，属 setup 缺陷判红`
    );
  }

  // storageState 显式置空：browser.newContext 会继承 testInfo.project.use 的
  // storageState（playwright.config.ts:66 = 分片主账号 e2e_admin_s{n} 的 cookie），
  // 不置空则本 context 实际带着 admin 身份，"不同用户隔离"前提被伪造成同一身份。
  // password 必须取 cred.password：角色账号初始密码是 ensureRoleUsers 的
  // DEFAULT_ROLE_PASSWORD，而非分片主账号 TEST_PASSWORD——
  // 只传 username 时 applyAuthMocks 用主账号密码登录，#4669 里整片 401
  // （backend.log rs31:17482 reason="无效的密码: 密码错误"）。
  const context = await browser.newContext({
    baseURL: BASE_URL,
    storageState: { cookies: [], origins: [] },
  });
  await applyAuthMocks(context, { username: cred.username, password: cred.password });
  const page = await context.newPage();

  // 获取该用户 ID
  const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');

  // 建一条线索
  const leadNo = genCode(leadCompanyPrefix);
  const created = await apiCall<{ id?: number }>(page, 'POST', '/crm/leads', {
    lead_no: leadNo,
    lead_status: 'new',
    company_name: `隔离测试 ${leadNo}`,
    contact_name: '测试联系人',
    lead_source: 'WEBSITE',
    mobile_phone: '13700137001',
    email: `${leadNo}@test.com`,
    priority: 'LOW',
  });
  if (!created.data?.id) {
    await context.close();
    throw new Error(`建线索失败：${JSON.stringify(created)}`);
  }

  return { context, page, userId: me.id, leadId: created.data.id, leadNo };
}

test.describe('数据范围行级隔离（self scope）', () => {
  test('self-01 不同 self 用户互不可见私有行', async ({ browser }) => {
    // User A: salesperson（data_scope=self，由 ensureRoleUsers 建角色默认 self）
    const userA = await createUserAndSeedLead(browser, 'salesperson', 'E2E-ISO-A');
    // User B: customer_service（data_scope=self，与 A 不同 user_id）
    const userB = await createUserAndSeedLead(browser, 'customer_service', 'E2E-ISO-B');

    try {
      // 两个用户应有不同的 user_id（前提条件）
      expect(
        userA.userId,
        `User A(${userA.userId}) 与 User B(${userB.userId}) 必须为不同用户`
      ).not.toBe(userB.userId);

      // User B 列表：不应看到 User A 的线索
      const listB = await apiCallRaw<{ data?: Array<{ id: number; lead_no: string }> }>(
        userB.page,
        'GET',
        '/crm/leads?page=1&page_size=100'
      );
      const itemsB = listB?.data ?? [];
      const aLeadInBList = itemsB.find(l => l.id === userA.leadId);
      expect(
        aLeadInBList,
        `User B(self scope, id=${userB.userId}) 不应看到 User A(id=${userA.userId}) 的私有线索 ${userA.leadId}`
      ).toBeUndefined();

      // User A 列表：应看到自己的线索
      const listA = await apiCallRaw<{ data?: Array<{ id: number; lead_no: string }> }>(
        userA.page,
        'GET',
        '/crm/leads?page=1&page_size=100'
      );
      const itemsA = listA?.data ?? [];
      const ownLeadInAList = itemsA.find(l => l.id === userA.leadId);
      expect(
        ownLeadInAList,
        `User A(self scope) 应能看到自己创建的线索 ${userA.leadId}`
      ).toBeDefined();
    } finally {
      // 清理
      await tryCleanup(userA.page, 'DELETE', `/crm/leads/${userA.leadId}`, 'crm_lead');
      await tryCleanup(userB.page, 'DELETE', `/crm/leads/${userB.leadId}`, 'crm_lead');
      await userA.context.close();
      await userB.context.close();
    }
  });

  test('self-02 self 用户按 ID GET 越权访问他人线索返回 403', async ({ browser }) => {
    // User A 建线索
    const userA = await createUserAndSeedLead(browser, 'salesperson', 'E2E-IDOR');

    // User B（不同 self 用户）尝试直接按 ID 获取 User A 的线索
    const credB = getRoleCredential('customer_service');
    // global-setup ensureRoleUsers 必须保证角色凭证就绪；缺失属 setup 缺陷，判红不 skip 掩盖
    if (!credB) {
      throw new Error(
        'customer_service 角色凭证不存在——global-setup ensureRoleUsers 未正确执行，属 setup 缺陷判红'
      );
    }
    // 独立空 storageState：不置空则 newContext 继承 use.storageState 的 admin cookie，
    // B 的请求会以 admin(all scope) 身份发出，越权 403 断言将被伪造成 200 假绿
    const contextB = await browser.newContext({
      baseURL: BASE_URL,
      storageState: { cookies: [], origins: [] },
    });
    await applyAuthMocks(contextB, { username: credB.username, password: credB.password });
    const pageB = await contextB.newPage();

    try {
      // User B GET User A 的线索 → 应被 check_resource_owner 拒绝（data_scope=self, A≠B）
      const resp = await pageB.request.fetch(`${API_BASE}${API_PREFIX}/crm/leads/${userA.leadId}`, {
        method: 'GET',
        headers: { 'X-Requested-With': 'XMLHttpRequest' },
      });
      expect(
        resp.status(),
        `User B(self) 按 ID 访问 User A 的私有线索应返回 403（实际 ${resp.status()}）`
      ).toBe(403);
    } finally {
      await tryCleanup(userA.page, 'DELETE', `/crm/leads/${userA.leadId}`, 'crm_lead');
      await userA.context.close();
      await contextB.close();
    }
  });

  test('self-03 admin(all scope) 可看到任意 self 用户的私有行', async ({ page, browser }) => {
    // User A (self scope) 建线索
    const userA = await createUserAndSeedLead(browser, 'salesperson', 'E2E-ADMIN-VIEW');

    try {
      // 主测试 page/context 使用默认 e2e_admin（all scope）
      // 直接 GET User A 的线索 → admin 应通过 check_resource_owner（All 始终 true）
      const lead = await apiCallRaw<{ id: number; lead_no: string; owner_id: number }>(
        page,
        'GET',
        `/crm/leads/${userA.leadId}`
      );
      expect(lead.id).toBe(userA.leadId);
      expect(lead.owner_id).toBe(userA.userId);

      // 列表搜索也能看到（keyword 匹配 company_name 含前缀）
      const list = await apiCallRaw<{ data?: Array<{ id: number }> }>(
        page,
        'GET',
        `/crm/leads?keyword=${encodeURIComponent('E2E-ADMIN-VIEW')}&page=1&page_size=50`
      );
      const items = list?.data ?? [];
      const found = items.find(l => l.id === userA.leadId);
      expect(found, 'admin(all scope) 列表应包含 User A 的行').toBeDefined();
    } finally {
      await tryCleanup(userA.page, 'DELETE', `/crm/leads/${userA.leadId}`, 'crm_lead');
      await userA.context.close();
    }
  });

  test('self-04 隔离验证基于真实 DB 而非 localStorage mock（负向：无 cookie 应 401）', async ({
    browser,
  }) => {
    // 创建一个未认证的裸请求，访问线索列表 → 应被 auth 中间件 401 拦截
    // #4669 假绿前兆实证：本行原写法 newContext 不指定 storageState 时，会继承
    // playwright.config.ts:66 的 use.storageState（分片主账号 cookie），"匿名"请求
    // 实际带着 e2e_admin_s31 的 access_token 到达后端（backend.log rs31 22:58:14
    // "从 access_token Cookie 获取Token … 认证成功 user_id=2 username=e2e_admin_s31"）
    // ⇒ 返回 200 而非 401。断言本身正确且必须保持精确 401，修的是"匿名"这个前置。
    const anonymousContext = await browser.newContext({
      baseURL: BASE_URL,
      storageState: { cookies: [], origins: [] },
    });
    const anonPage = await anonymousContext.newPage();

    try {
      const resp = await anonPage.request.fetch(
        `${API_BASE}${API_PREFIX}/crm/leads?page=1&page_size=1`,
        { method: 'GET', headers: { 'X-Requested-With': 'XMLHttpRequest' } }
      );
      // 无 auth cookie → 401
      expect(
        resp.status(),
        `无认证态访问业务 API 应返回 401（证明隔离依赖真实后端鉴权而非前端 mock）`
      ).toBe(401);
    } finally {
      await anonymousContext.close();
    }
  });
});
