/**
 * 多上下文隔离 + 多用户角色协作工具集
 *
 * 批次 262：针对项目 E2E 测试增强，提供以下能力：
 * - 多上下文隔离：为不同用户角色创建独立的 BrowserContext（cookie/localStorage 互不干扰）
 * - 多用户角色协作：模拟 admin / manager / operator 等角色在同一业务流程中的协作
 * - 并行会话：多个角色同时在各自上下文中操作，验证协作场景
 *
 * 设计原则：
 * - 每个角色一个独立 BrowserContext，互不干扰（模拟不同浏览器/不同设备）
 * - 角色凭据从环境变量注入（fail-secure 模式，禁止硬编码）
 * - 协作场景使用真实后端登录（与 color-card.spec.ts 一致）
 * - smoke 场景可使用 mock 凭据（与 auth.ts 一致）
 *
 * 会话态来源（authMode）分三档，缺省一律保持历史行为，既有 mocked 消费者零改动：
 * - anonymous：只建空 context，既不注入 mock 也不登录（createIsolatedSession 的历史语义）
 * - mocked：注入假 JWT 与 mock 用户信息，不连后端（createMockedIsolatedSession 的历史语义）
 * - real：按角色码取凭证并在该 context 内走后端的 auth 登录接口拿真 cookie，
 *   凭证来源与 loginAsRole 完全同源（env 优先，回退 role-credentials.json），
 *   登录腿复用 flow/helpers 的 loginInIsolatedContext，全仓只此一套真实登录实现
 */
import type { Browser, BrowserContext, Page } from '@playwright/test';
import { injectAuthToken, mockInitStatus } from './auth';
import { workerPasswordFromEnv, workerUsernameFromEnv } from '../worker-auth';
import { getRoleCredential, loginInIsolatedContext } from '../flow/helpers';

/**
 * 角色凭据配置
 */
export interface RoleCredentials {
  /** 角色名称（用于日志与断言） */
  role: string;
  /** 登录用户名 */
  username: string;
  /** 登录密码 */
  password: string;
}

/**
 * 会话鉴权来源档位。
 * 调用方：createIsolatedSession 与 createCollaborationContext 的 options。
 * 入参形态：字符串字面量联合，无运行时枚举对象，不参与后端契约。
 * anonymous 与 mocked 是历史既有语义；real 为新增能力，只有显式传入才会启用。
 */
export type SessionAuthMode = 'anonymous' | 'mocked' | 'real';

/**
 * 建会话的可选参数。
 * 调用方：createIsolatedSession、createMockedIsolatedSession、createCollaborationContext。
 * authMode 缺省为 anonymous（保持既有裸 context 语义）；permissions 仅在 mocked 档生效；
 * credentialRole 仅在 real 档生效，用于把"会话显示名"与"取凭证用的角色码"解耦
 * （同一角色码在多角色场景里需要各自独立显示名时传 role，取凭证用 credentialRole）。
 */
export interface IsolatedSessionOptions {
  authMode?: SessionAuthMode;
  /** mocked 档写入 mock 用户信息的权限码集合，缺省为通配一枚 */
  permissions?: string[];
  /** real 档取凭证用的角色码，缺省用 role */
  credentialRole?: string;
}

/**
 * 隔离会话：一个 BrowserContext + 一个 Page + 角色信息
 */
export interface IsolatedSession {
  /** 角色名称 */
  role: string;
  /**
   * 该会话绑定的登录账号名。
   * real 档为凭证文件里的真账号名，mocked 档为 mock 用户名，
   * anonymous 档不绑定任何身份故为 undefined（不伪装成已登录）。
   */
  username?: string;
  /** 隔离的浏览器上下文（cookie/localStorage 独立） */
  context: BrowserContext;
  /** 会话页面 */
  page: Page;
  /** 关闭会话（关闭 context） */
  close: () => Promise<void>;
}

/**
 * 从环境变量读取角色凭据（fail-secure 模式）
 *
 * 环境变量命名约定：
 * - E2E_ADMIN_USERNAME / E2E_ADMIN_PASSWORD
 * - E2E_MANAGER_USERNAME / E2E_MANAGER_PASSWORD
 * - E2E_OPERATOR_USERNAME / E2E_OPERATOR_PASSWORD
 *
 * 缺失时的取身份口径：
 * - admin 角色 → 本 worker 的会话账号（由 worker 作用域 fixture 注入本进程 env）。
 *   绝不回落到分片主账号或基础管理员账号：整片共用一个账号 + 多并发 worker 会互相打死
 *   一次性消费的 CSRF 凭据；会话未绑定时由 worker-auth 显式抛错点名注入来源。
 * - 其它角色 → 必须显式给 env，否则抛错（fail-secure，禁止硬编码凭据）。
 */
export function loadRoleCredentials(role: string): RoleCredentials {
  const envPrefix = `E2E_${role.toUpperCase()}`;
  const username = process.env[`${envPrefix}_USERNAME`];
  const password = process.env[`${envPrefix}_PASSWORD`];

  // 回退：admin 角色用本 worker 的会话身份（与 context 初始会话态同源）
  if (!username || !password) {
    if (role === 'admin') {
      return { role, username: workerUsernameFromEnv(), password: workerPasswordFromEnv() };
    }
    throw new Error(
      `E2E 多角色测试需要环境变量 ${envPrefix}_USERNAME / ${envPrefix}_PASSWORD` +
        `（fail-secure 模式，禁止硬编码凭据）`
    );
  }

  return { role, username, password };
}

/**
 * 给会话套上 mock 鉴权（注入假 token、mock 用户信息、mock 初始化状态）。
 * 调用方：createIsolatedSession 的 mocked 档，以及经它转调的 createMockedIsolatedSession。
 * 入参：会话、角色名、权限码集合。传给谁：fixtures/auth 的 addInitScript 与 context.route。
 * 存什么：只存浏览器上下文内的假 token 与拦截规则，不落任何持久化数据。
 * 三步的注册先后与改动前逐字一致（假 token、用户信息拦截、初始化状态拦截），
 * 刻意不调整：既有 mock 消费者依赖这一份规则集，顺序变动会让拦截面与历史行为不再等价。
 */
async function applyMockedAuth(
  session: IsolatedSession,
  role: string,
  permissions: string[]
): Promise<void> {
  // 注入 mock token（绕过鉴权）
  await injectAuthToken(session.context);

  // 拦截 /auth/me 返回该角色的 mock 用户信息
  await session.context.route('**/api/v1/erp/auth/me**', route => {
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        code: 200,
        message: 'success',
        data: {
          id: Math.floor(Math.random() * 1000) + 1,
          username: `mock_${role}`,
          real_name: `测试${role}`,
          role_id: Math.floor(Math.random() * 10) + 1,
          role_name: role,
          permissions,
        },
        timestamp: new Date().toISOString(),
      }),
    });
  });

  // 拦截 init/status 返回 initialized: true
  await mockInitStatus(session.context);
}

/**
 * 取一个真实登录的角色会话：凭证来源与 loginAsRole 同源，登录腿复用既有隔离会话实现。
 * 调用方：createIsolatedSession 的 real 档（接力夹具与多角色协作场景）。
 * 入参：browser、角色显示名、取凭证用的角色码。传给谁：getRoleCredential 读 env 与
 * role-credentials.json，loginInIsolatedContext 向后端登录接口发请求并把 cookie 落进新 context。
 * 存什么：该 context 的 access_token、refresh_token、csrf_token 三条真 cookie。
 * 存哪里：只存在这个 BrowserContext 的 cookie jar 里，不写会话态文件，避免与其他 worker 争抢。
 * 凭证缺失即抛错点名角色码与两处来源，绝不退回其它账号（退号会让接力退化成同账号自问自答）。
 */
async function createRealLoginSession(
  browser: Browser,
  role: string,
  credentialRole: string
): Promise<IsolatedSession> {
  const cred = getRoleCredential(credentialRole);
  if (!cred) {
    throw new Error(
      `real 档会话取凭证失败：角色码 ${credentialRole} 在 env ` +
        `E2E_${credentialRole.toUpperCase()}_USERNAME 与 e2e/.auth/role-credentials.json 两处都不存在。` +
        `禁止改用其它账号顶替，接力链上两个身份必须各自真实存在`
    );
  }
  const authed = await loginInIsolatedContext(browser, cred.username, cred.password);
  return {
    role,
    username: authed.username,
    context: authed.context,
    page: authed.page,
    close: authed.close,
  };
}

/**
 * 创建隔离的浏览器会话（独立 context）
 *
 * 使用场景：
 * - 多用户角色协作测试（每个角色一个 context，互不干扰）
 * - 多设备/多浏览器会话模拟（不同 context 模拟不同设备）
 *
 * 缺省第三参时行为与历史完全一致：只建空 context，不做任何鉴权。
 * 需要真登录时显式传 authMode real 或字符串 real。
 *
 * @example
 * const adminSession = await createIsolatedSession(browser, 'admin')
 * const managerSession = await createIsolatedSession(browser, 'manager', { authMode: 'real' })
 */
export async function createIsolatedSession(
  browser: Browser,
  role: string,
  options: SessionAuthMode | IsolatedSessionOptions = {}
): Promise<IsolatedSession> {
  const opts: IsolatedSessionOptions =
    typeof options === 'string' ? { authMode: options } : options;
  const mode: SessionAuthMode = opts.authMode ?? 'anonymous';

  if (mode === 'real') {
    return createRealLoginSession(browser, role, opts.credentialRole ?? role);
  }

  const context = await browser.newContext();
  const page = await context.newPage();
  const session: IsolatedSession = {
    role,
    context,
    page,
    close: async () => {
      await context.close();
    },
  };

  if (mode === 'mocked') {
    await applyMockedAuth(session, role, opts.permissions ?? ['*']);
    session.username = `mock_${role}`;
  }

  return session;
}

/**
 * 创建带 mock 鉴权的隔离会话（smoke 测试用，不依赖真实后端）
 *
 * 使用场景：
 * - smoke 测试中多角色协作（mock 不同角色的权限）
 * - 无后端环境下的多角色 UI 交互测试
 *
 * @param role 角色名称（用于日志）
 * @param permissions 该角色的权限列表（写入 mock /auth/me 响应）
 */
export async function createMockedIsolatedSession(
  browser: Browser,
  role: string,
  permissions: string[] = ['*']
): Promise<IsolatedSession> {
  return createIsolatedSession(browser, role, { authMode: 'mocked', permissions });
}

/**
 * 执行真实登录流程（在指定会话中）
 *
 * 使用场景：
 * - 多角色协作测试中，每个角色在自己的 context 中真实登录
 * - 与 color-card.spec.ts 的 login 函数一致，但作用于指定 page
 *
 * @param session 隔离会话
 * @param credentials 角色凭据
 * @param baseURL 应用基础地址
 */
export async function loginSession(
  session: IsolatedSession,
  credentials: RoleCredentials,
  baseURL: string
): Promise<void> {
  const { page } = session;
  await page.goto(`${baseURL}/login`);
  await page.waitForSelector('input[name="username"]', { state: 'visible' });
  await page.waitForSelector('input[name="password"]', { state: 'visible' });

  await page.fill('input[name="username"]', credentials.username);
  await page.fill('input[name="password"]', credentials.password);
  await page.click('button[type="submit"]');

  // 等待登录成功后跳转
  await page.waitForURL(/dashboard|\/$/, { timeout: 30_000 });
}

/**
 * 并行执行多个角色的会话操作
 *
 * 使用场景：
 * - 多用户角色协作场景（admin 创建 → manager 审批 → operator 查看）
 * - 并行测试多个角色的 UI 交互（互不干扰）
 *
 * @example
 * await runParallelSessions(browser, [
 *   { role: 'admin', action: async (session) => { await adminCreatesOrder(session) } },
 *   { role: 'manager', action: async (session) => { await managerApprovesOrder(session) } },
 * ])
 */
export async function runParallelSessions(
  browser: Browser,
  sessions: Array<{
    role: string;
    action: (session: IsolatedSession) => Promise<void>;
  }>
): Promise<void> {
  // 创建所有会话
  const isolated = await Promise.all(
    sessions.map(s => createMockedIsolatedSession(browser, s.role))
  );

  try {
    // 并行执行各会话的操作
    await Promise.all(isolated.map((session, index) => sessions[index].action(session)));
  } finally {
    // 关闭所有会话
    await Promise.all(isolated.map(session => session.close()));
  }
}

/**
 * 创建多角色协作上下文（一次性创建多个隔离会话）
 *
 * 使用场景：
 * - 业务流程协作测试（一个角色建单，另一个角色审批，再一个角色查看，逐棒传递）
 * - 需要多个角色同时在线的复杂场景
 *
 * 鉴权档位口径（第三参保持历史布尔语义，第四参是新增的显式覆盖）：
 * - 只给第三参：true 走 mocked、false 走 anonymous，与改动前逐字一致
 * - 给了 options.authMode：以它为准，real 档即"多个角色各自真登录"
 *
 * @example
 * const { sessions, close } = await createCollaborationContext(browser, ['admin', 'manager'])
 * const { sessions, close } = await createCollaborationContext(
 *   browser,
 *   ['sales_manager', 'warehouse_keeper'],
 *   false,
 *   { authMode: 'real' }
 * )
 * try {
 *   await sessions.admin.page.goto('/admin')
 *   await sessions.manager.page.goto('/dashboard')
 * } finally {
 *   await close()
 * }
 */
export async function createCollaborationContext(
  browser: Browser,
  roles: string[],
  useMock = true,
  options: IsolatedSessionOptions = {}
): Promise<{
  sessions: Record<string, IsolatedSession>;
  close: () => Promise<void>;
}> {
  const mode: SessionAuthMode = options.authMode ?? (useMock ? 'mocked' : 'anonymous');
  const sessionList = await Promise.all(
    roles.map(role => createIsolatedSession(browser, role, { ...options, authMode: mode }))
  );

  const sessions: Record<string, IsolatedSession> = {};
  sessionList.forEach(session => {
    sessions[session.role] = session;
  });

  return {
    sessions,
    close: async () => {
      await Promise.all(sessionList.map(session => session.close()));
    },
  };
}
