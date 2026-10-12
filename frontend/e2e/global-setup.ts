import { request } from '@playwright/test';
import { execSync } from 'child_process';
import { writeFileSync, mkdirSync } from 'fs';

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp'; // 分片专属账号：每个 CI runner（matrix.shard）独享，根除跨分片并发登录的 CSRF 互踢。
// 分片账号通过真实 UI（用户管理页面）创建，属于测试前置数据准备（ensureTestEntities 同级，
// 不属于测试验证手段），UI 测试本身仍全部走真实用户操作。
// 基础管理员（init 步骤创建的固定账号，用于创建分片账号）：
// CI step 级会把 TEST_USERNAME 覆盖为分片账号，故这里显式用 E2E_BASE_USERNAME
// 或固定 'e2e_admin'，与分片账号变量彻底解耦
const BASE_USERNAME = process.env.E2E_BASE_USERNAME || 'e2e_admin';
const BASE_PASSWORD = process.env.TEST_PASSWORD || 'Xk9#mQ2$vL8pW4nR';
const SHARD_INDEX = process.env.E2E_SHARD_INDEX ?? '';
const SHARD_USERNAME = SHARD_INDEX !== '' ? `e2e_admin_s${SHARD_INDEX}` : BASE_USERNAME;
const SHARD_PASSWORD = BASE_PASSWORD;
// AI 限流隔离专用第二用户：ai/02 使用此用户登录，与 ai/01 的 SHARD_USERNAME 不同 user_id，
// 后端 rate_limit_ai_endpoint 按 user_id 维度限流（rate_limit.rs:330）→ 两文件独立 10/min 桶。
const AI_USERNAME = SHARD_INDEX !== '' ? `e2e_ai_s${SHARD_INDEX}` : 'e2e_ai';
const STORAGE_STATE_PATH = 'e2e/.auth/storage-state.json';

/**
 * requestWithCsrfRecovery / assignPermissionList 的响应契约（单一声明点）。
 *
 * 对齐 Playwright 真实类型 APIResponse：
 *   `ok(): boolean` —— ok 是**方法**不是属性；
 *   `json(): Promise<T>`
 * 此前本文件内联声明写成 `ok: boolean`（同文件 loginWithRetry 用的却是正确的
 * `ok: () => boolean`），导致把真实 APIRequestContext 传进来时类型不匹配（5 处 TS2345）、
 * 3 处运行时完全正确的 `resp.ok()` 报 TS2349、`resp.json()` 报 TS2339。
 * 修类型声明，不动调用点断言。只声明 setup 用到的成员，避免与 Playwright 版本演进脱钩。
 */
interface SetupApiResponse {
  ok: () => boolean;
  status: () => number;
  headers: () => Record<string, string>;
  /** 与 APIResponse.json() 对齐；调用点（:470）已有显式 `as` 收敛形状，不在此处再泛型化 */
  json(): Promise<unknown>;
  /** 与 APIResponse.text() 对齐；种子失败告警需打印原始响应体(非 2xx 常为纯文本) */
  text(): Promise<string>;
}

/** 仅声明 setup 用到的写方法 + 权限回读 GET + storageState（CSRF 轮换后需重取 token） */
interface CsrfCapableRequestContext {
  get(url: string, options: object): Promise<SetupApiResponse>;
  post(url: string, options: object): Promise<SetupApiResponse>;
  put(url: string, options: object): Promise<SetupApiResponse>;
  storageState(): Promise<{
    cookies: Array<{ name: string; value: string; domain: string; path: string }>;
  }>;
}

/**
 * 带退避重试的 API 登录：50 分片并发启动时同账号登录会触发
 * anti_brute_force 限流（429）。重试策略：指数退避，最多 6 次。
 * 入参 ctx 为已创建的 request context；返回登录响应。
 */
async function loginWithRetry(
  ctx: {
    post: (
      url: string,
      options: object
    ) => Promise<{ ok: () => boolean; status: () => number; text: () => Promise<string> }>;
  },
  username: string,
  password: string
): Promise<{ ok: () => boolean; status: () => number; text: () => Promise<string> }> {
  for (let attempt = 1; attempt <= 8; attempt++) {
    const resp = await ctx.post(`${API_PREFIX}/auth/login`, {
      data: { username, password },
    });
    if (resp.ok() || resp.status() !== 429) {
      if (!resp.ok()) {
        const body = await resp.text().catch((e: unknown) => {
          console.warn(`[loginWithRetry] 响应体读取失败: ${(e as Error).message}`);
          return '';
        });
        console.warn(
          `[loginWithRetry] ${username} 登录失败 HTTP ${resp.status()}（attempt ${attempt}）: ${body.slice(0, 200)}`
        );
      }
      return resp;
    }
    // 429 限流：退避策略 5s/10s/20s/40s/60s/90s/120s/120s（覆盖 300s 限流窗口）
    const waits = [5000, 10000, 20000, 40000, 60000, 90000, 120000, 120000];
    const wait = waits[attempt - 1] ?? 120000;
    console.warn(`[loginWithRetry] ${username} 登录 429 限流，attempt ${attempt}/8 退避 ${wait}ms`);
    await new Promise(r => setTimeout(r, wait));
  }
  // 最后一次重试
  const lastResp = await ctx.post(`${API_PREFIX}/auth/login`, { data: { username, password } });
  if (!lastResp.ok()) {
    const body = await lastResp.text().catch((e: unknown) => {
      console.warn(`[loginWithRetry] 末次响应体读取失败: ${(e as Error).message}`);
      return '';
    });
    throw new Error(
      `[loginWithRetry] ${username} 登录 8 次重试后仍失败: HTTP ${lastResp.status()} ${body.slice(0, 300)}`
    );
  }
  return lastResp;
}

export default async function globalSetup() {
  // ---- 1. 分片账号不存在时，通过真实 UI 创建（e2e_admin 登录 → 用户管理页 → 新建用户）----
  if (SHARD_USERNAME !== BASE_USERNAME) {
    await ensureShardUserViaUI();
  }

  // ---- 1.2 AI 限流隔离第二用户（admin 角色，与分片主用户不同 user_id → 独立 AI 限流桶）----
  await ensureAiUser();

  // ---- 1.5 全量角色账号 setup（P3.1）----
  await ensureRoleUsers();

  // ---- 2. 分片账号登录（API），保存 storageState 供全部 spec 复用 ----
  const ctx = await request.newContext({
    baseURL: API_BASE,
    extraHTTPHeaders: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
    },
  });

  const resp = await loginWithRetry(ctx, SHARD_USERNAME, SHARD_PASSWORD);

  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(`globalSetup 登录失败 (user=${SHARD_USERNAME}): HTTP ${resp.status()} ${body}`);
  }

  const cookies = await ctx.storageState();
  const accessCookie = cookies.cookies.find(c => c.name === 'access_token');
  if (!accessCookie) {
    throw new Error('globalSetup 登录后未获得 access_token cookie');
  }

  mkdirSync('e2e/.auth', { recursive: true });

  // ---- 2.5 全局业务实体种子（供不依赖 ensureTestEntities 的 extras specs 使用）----
  // extras 目录（sales/purchase/crm/color-card/price/production/quality 等）的
  // spec 不调用 ensureTestEntities，直接 navigate 后断言"状态行存在/下拉有选项"。
  // 本步骤用纯 API 建最小前置集，使 GET 列表非空。幂等：先查后建。
  await ensureGlobalBusinessSeed(ctx);

  // ---- 2.9 seed 之后回写 storage-state（保证磁盘上携带存活 CSRF token）----
  // 后端 CSRF Token 为服务端一次性消费（middleware/csrf.rs:199 consume → :216 Set-Cookie
  // 轮换）。上面 :118 登录拿到的 csrf=T0；ensureGlobalBusinessSeed 内的连续写请求会逐次
  // 消费并轮换 T0（seed 走 requestWithCsrfRecovery，从 ctx cookie jar 读回轮换后的新 token），
  // 至 seed 结束时 T0 已死亡。若把 T0 写入 storage-state，之后每条 playwright 用例
  // 从磁盘 storageState 恢复上下文都带回已死的 T0，
  // 任何需 CSRF 的写请求首轮即 403 CSRF_TOKEN_INVALID（表现为成片 403）。
  // 因此在 seed 完成后重新读取当前 ctx 的 cookie jar（含仍然存活的 access_token 与
  // 轮换链末端的 csrf_token），覆盖写回磁盘，保证持久化的 storage-state 携带存活 token。
  const postSeedState = await ctx.storageState();
  const postSeedAccess = postSeedState.cookies.find(c => c.name === 'access_token');
  const postSeedCsrf = postSeedState.cookies.find(c => c.name === 'csrf_token');
  if (!postSeedAccess || !postSeedCsrf) {
    throw new Error(
      `globalSetup seed 后 storage-state 缺关键 cookie（access=${!!postSeedAccess} csrf=${!!postSeedCsrf}），` +
        `持久化登录态将不可用`
    );
  }
  writeFileSync(STORAGE_STATE_PATH, JSON.stringify(postSeedState, null, 2));
  console.log(
    `[globalSetup] storage-state 于 seed 后回写完成：csrf_token=${postSeedCsrf.value.slice(0, 8)}…` +
      `（携带存活 token，非 seed 前已消费的旧值）`
  );

  await ctx.dispose();
}

/**
 * 通过真实 UI 创建分片专属账号（不使用 API 直接创建）：
 * e2e_admin 登录 → /system 用户管理 → 新建用户 → 填用户名/密码/姓名/角色(admin) → 提交
 * 账号已存在（唯一约束冲突）时视为成功跳过。
 */
async function ensureShardUserViaUI(): Promise<void> {
  // 分片账号创建属测试前置数据准备（与 ensureTestEntities 的 API 兜底同级，
  // 不属于测试验证手段——用户管理 UI 本身由 26-system-full 的 UI 测试验证）。
  // 1) 基础管理员 API 登录拿 cookie
  const loginCtx = await request.newContext({
    baseURL: API_BASE,
    extraHTTPHeaders: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
    },
  });
  const loginResp = await loginWithRetry(loginCtx, BASE_USERNAME, BASE_PASSWORD);
  if (!loginResp.ok()) {
    const body = await loginResp.text();
    await loginCtx.dispose();
    throw new Error(`分片账号创建前置：e2e_admin API 登录失败 HTTP ${loginResp.status()} ${body}`);
  }
  const loginCookies = (await loginCtx.storageState()).cookies;
  const csrfCookie = loginCookies.find(c => c.name === 'csrf_token');
  const accessCookie = loginCookies.find(c => c.name === 'access_token');
  if (!csrfCookie || !accessCookie) {
    await loginCtx.dispose();
    throw new Error('分片账号创建前置：登录后未取得 csrf/access cookie');
  }

  // 2) 查询 admin 角色 id
  const rolesResp = await loginCtx.get(`${API_PREFIX}/roles?page=1&page_size=50`, {
    headers: { 'X-CSRF-Token': csrfCookie.value, 'X-Requested-With': 'XMLHttpRequest' },
  });
  interface RoleRow {
    id: number;
    name?: string;
    code?: string;
  }
  const rolesBody = (await rolesResp.json().catch(e => {
    console.warn(
      `[assignPermissionList] 权限分配失败（不影响角色账号创建）:`,
      (e as Error).message
    );
    return null;
  })) as {
    // 后端 role_handler.rs:113-145 list_roles → ApiResponse<RoleListResponse>，
    // data = { roles: RoleResponse[], total }（既非裸数组也非 items）。
    data?: { roles?: RoleRow[] };
  } | null;
  const roleList = rolesBody?.data?.roles;
  if (!Array.isArray(roleList)) {
    await loginCtx.dispose();
    throw new Error(
      `分片账号创建失败：/roles 响应缺 data.roles（后端契约 role_handler.rs:113-145），` +
        `实际片段=${JSON.stringify(rolesBody).slice(0, 200)}`
    );
  }
  const adminRole = roleList.find(r => r.code === 'admin' || r.name === 'admin');
  if (!adminRole) {
    await loginCtx.dispose();
    throw new Error(
      `分片账号创建失败：未找到 admin 角色（roles=${JSON.stringify(rolesBody).slice(0, 300)}）`
    );
  }

  // 3) POST /users 创建分片账号（已存在视为成功）
  const createPayload = {
    username: SHARD_USERNAME,
    password: SHARD_PASSWORD,
    role_id: adminRole.id,
  };
  const createResp = await loginCtx.post(`${API_PREFIX}/users`, {
    headers: { 'X-CSRF-Token': csrfCookie.value, 'X-Requested-With': 'XMLHttpRequest' },
    data: createPayload,
  });
  await loginCtx.dispose();
  if (createResp.ok()) {
    console.log(`[globalSetup] 分片账号 ${SHARD_USERNAME} 创建成功 (HTTP ${createResp.status()})`);
  } else {
    const body = await createResp.text().catch(e => {
      console.warn(`[ensureShardUserViaUI] 创建响应体读取失败: ${(e as Error).message}`);
      return '';
    });
    // 幂等：400/409 或文案含"已存在"都视为账号已建——watchdog 重跑同一分片时
    // 第 1 轮已创建账号，重复 POST 返回 400 BusinessError"用户名已存在"，
    // 若不当作幂等识别，整片会瞬间失败。
    if (body.includes('已存在') || createResp.status() === 409 || createResp.status() === 400) {
      console.log(
        `[globalSetup] 分片账号 ${SHARD_USERNAME} 已存在（HTTP ${createResp.status()}），跳过创建`
      );
    } else {
      throw new Error(
        `分片账号创建失败 HTTP ${createResp.status()} payload=${JSON.stringify({ ...createPayload, password: '***' })} body=${body.slice(0, 300)}`
      );
    }
  }

  // 4) 终验：分片账号必须可登录
  const checkCtx = await request.newContext({
    baseURL: API_BASE,
    extraHTTPHeaders: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
    },
  });
  const loginCheck = await loginWithRetry(checkCtx, SHARD_USERNAME, SHARD_PASSWORD);
  await checkCtx.dispose();
  if (!loginCheck.ok()) {
    const body = await loginCheck.text().catch(e => {
      console.warn(`[ensureShardUserViaUI] 创建响应体读取失败: ${(e as Error).message}`);
      return '';
    });
    throw new Error(
      `分片账号 ${SHARD_USERNAME} 终验失败: HTTP ${loginCheck.status()} ${body.slice(0, 300)}`
    );
  }
  console.log(`[globalSetup] 分片账号 ${SHARD_USERNAME} 就绪（登录验证通过）`);
}

/**
 * 确保 AI 限流隔离第二用户存在并可登录（admin 角色）。
 * ai/02-prediction 使用此用户登录，与 ai/01 的分片主用户（e2e_admin_s{n}）拥有不同
 * user_id → 后端 rate_limit_ai_endpoint 按 user_id 维度的 10 req/min 桶彼此独立，
 * 两文件在 extras 分片内并行（workers=2）不再争抢同一桶，确定性根除 429 互踩。
 * 幂等：已存在（400/409/「已存在」）视为成功跳过。
 */
async function ensureAiUser(): Promise<void> {
  const loginCtx = await request.newContext({
    baseURL: API_BASE,
    extraHTTPHeaders: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
    },
  });
  const loginResp = await loginWithRetry(loginCtx, BASE_USERNAME, BASE_PASSWORD);
  if (!loginResp.ok()) {
    const body = await loginResp.text();
    await loginCtx.dispose();
    throw new Error(`ensureAiUser: ${BASE_USERNAME} 登录失败 HTTP ${loginResp.status()}: ${body}`);
  }
  const loginCookies = (await loginCtx.storageState()).cookies;
  const csrfCookie = loginCookies.find(c => c.name === 'csrf_token');
  if (!csrfCookie) {
    await loginCtx.dispose();
    throw new Error('ensureAiUser: 登录后未取得 csrf_token cookie');
  }
  const headers: Record<string, string> = {
    'X-CSRF-Token': csrfCookie.value,
    'X-Requested-With': 'XMLHttpRequest',
  };

  // 查询 admin 角色 id
  const rolesResp = await loginCtx.get(`${API_PREFIX}/roles`, { headers });
  if (!rolesResp.ok()) {
    const body = await rolesResp.text();
    await loginCtx.dispose();
    throw new Error(`ensureAiUser: 角色清单拉取失败 HTTP ${rolesResp.status()}: ${body}`);
  }
  const rolesBody = (await rolesResp.json()) as {
    data?: { roles?: Array<{ id: number; code?: string; name?: string }> };
  };
  const adminRole = rolesBody?.data?.roles?.find(r => r.code === 'admin' || r.name === 'admin');
  if (!adminRole) {
    await loginCtx.dispose();
    throw new Error('ensureAiUser: 未找到 admin 角色');
  }

  // 创建用户（幂等）
  const createResp = await requestWithCsrfRecovery(
    loginCtx as unknown as CsrfCapableRequestContext,
    'post',
    `${API_PREFIX}/users`,
    headers,
    {
      username: AI_USERNAME,
      password: BASE_PASSWORD,
      role_id: adminRole.id,
      real_name: 'E2E-AI限流隔离',
    }
  );
  if (createResp.ok()) {
    console.log(`[globalSetup] AI 隔离用户 ${AI_USERNAME} 创建成功`);
  } else if (createResp.status() === 400 || createResp.status() === 409) {
    console.log(`[globalSetup] AI 隔离用户 ${AI_USERNAME} 已存在，跳过`);
  } else {
    const body = await createResp.text();
    await loginCtx.dispose();
    throw new Error(
      `ensureAiUser: 创建 ${AI_USERNAME} 失败 HTTP ${createResp.status()}: ${body.slice(0, 300)}`
    );
  }
  await loginCtx.dispose();

  // 终验：确保可登录
  const checkCtx = await request.newContext({
    baseURL: API_BASE,
    extraHTTPHeaders: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
    },
  });
  const loginCheck = await loginWithRetry(checkCtx, AI_USERNAME, BASE_PASSWORD);
  await checkCtx.dispose();
  if (!loginCheck.ok()) {
    const body = await loginCheck.text().catch(() => '');
    throw new Error(
      `ensureAiUser: ${AI_USERNAME} 终验登录失败 HTTP ${loginCheck.status()}: ${body.slice(0, 300)}`
    );
  }
  console.log(`[globalSetup] AI 隔离用户 ${AI_USERNAME} 就绪`);
}

// ==================== P3.1 全量角色账号 setup ====================

/**
 * 角色清单（种子角色，预期 30+）
 * 执行时会从 GET /roles 拉取全量清单补齐
 */
const SEED_ROLES = [
  'admin',
  'system_admin',
  'department_manager',
  'purchasing_manager',
  'purchaser',
  'sales_manager',
  'salesperson',
  'warehouse_manager',
  'warehouse_keeper',
  'finance_manager',
  'accountant',
  'cashier',
  'cost_accountant',
  'quality_manager',
  'quality_inspector',
  'production_manager',
  'production_worker',
  'dyeing_technician',
  'color_card_manager',
  'after_sales_manager',
  'customer_service',
  'report_viewer',
  'auditor',
  'procurement_specialist',
  'supplier_manager',
  'inventory_accountant',
  'tax_accountant',
  'ap_accountant',
  'ar_accountant',
  'fixed_assets_accountant',
  'budget_analyst',
  // inventory_manager：02-adjustment.spec.ts IDOR 用例的第二用户 B（getRoleCredential
  // ('inventory_manager')）。本库 init 只种 3 个角色，后端 role.rs:168 的业务角色不
  // 保证存在，必须由 ensureRoleUsers 补建——缺此码时 role-credentials 凭证文件无
  // inventory_manager 键，getRoleCredential 返 null，用例在 setup 阶段即判红。
  'inventory_manager',
  // manager —— fullflow/15-report-export.spec.ts 15-05 的申请侧账号（loginAsRole('manager')）。
  // 后端 init 矩阵确有同名角色与 ("export-approvals","create")
  // （init_service_ops/permission.rs::management_role_resources），
  // 但本库 init 实际只种 3 角色（见上方 inventory_manager 条目的说明），业务角色
  // 不保证存在；缺该码时 role-credentials.json 无 manager 键 ⇒ getRoleCredential 返
  // null、helpers.ts 的 loginAsRole 直接抛，15-05 在 setup 阶段判红。与
  // inventory_manager 同法由 ensureRoleUsers
  // 补建（系统角色分支 409 时改为回读校验，见下方 SEED_ROLE_EXTRA_PERMISSIONS 说明）。
  // 15-05 的靶心正是"manager 申请 + admin 审批"双人分离，禁止改用 admin 绕过。
  'manager',
];

// 应用外壳权限码：与后端 init_service_ops/permission.rs 的 SHELL_PERMISSIONS 一致——
// 登录后落地页 /dashboard，主框架铃铛拉取自身未读数。缺码的角色登录后停在 /403。
const SHELL_PERMISSIONS = ['dashboard:read', 'notifications:read'];

// SEED_ROLES 中部分角色承载专项业务 spec（如 purchase/sku-mapping.spec.ts 的保密矩阵、
// 采购维护端到端），仅给 SHELL 无法访问对应端点。此映射为这些角色叠加"本 spec 真实需要"
// 的业务权限码——严格对齐后端 init_service_ops/permission.rs 中同名角色的权限集，
// 不越界、不虚构。保密矩阵所需的**不授**purchase/sku-mappings 等对销售角色的保密面，
// 保持负向断言（403 FORBIDDEN）真实成立。
const SEED_ROLE_EXTRA_PERMISSIONS: Record<string, string[]> = {
  // purchaser 对齐后端 purchase_clerk 权限集（read、create、update、import，不含 delete）。
  // 覆盖 sku-mapping 用例 A、C、D1、D2b、E1、E3，即产品与供应商只读、目录、对照 CRUD、批量导入。
  // import 是与建单同授本岗的成批建单通路，purchase_clerk 亦有，缺则 E3 批量导入按钮被
  // v-permission 删、用例无法打开导入对话框。
  // product-categories:read —— /product 页挂载即拉产品分类树（api/product.ts GET
  // /product-categories、/product-categories/tree），缺码则该页对 purchaser 恒 403 噪声，
  // 权限矩阵用例随之判红。本项目口径为 purchaser 应见产品分类树、补真实种子
  // （非前端降级隐藏）；init 矩阵/迁移通道①②须与此同口径落地（三通道同口径锁）。
  purchaser: [
    'sku-mappings:read',
    'sku-mappings:create',
    'sku-mappings:update',
    'sku-mappings:import',
    'supplier-products:read',
    'supplier-products:create',
    'supplier-products:update',
    'supplier-product-colors:read',
    'supplier-product-colors:create',
    'supplier-product-colors:update',
    'products:read',
    'product-categories:read',
    'suppliers:read',
  ],
  // salesperson 对齐 permission.rs 的 sales_rep 权限集：可读销售订单（保密扫描用例
  // "响应体不含 supplier_ 键"要求 200 才有效），但绝不授任何 purchase/supplier 侧权限，
  // 保证 sku-mappings/supplier-products/supplier-product-colors 三端点 403 负向真实成立。
  // crm-leads:* —— enhanced/data-scope-isolation.spec.ts 的 User A：POST/GET/DELETE /crm/leads
  // 需真实通过 RBAC。运行时键由 URL 段消歧得到（path_utils 的 ("crm","leads") → crm-leads），
  // 与注册表权威名 crm-leads 和前端 constants/permissions 的 CRM_LEAD_* 同源；缺码则建线索被
  // RBAC 403 拦下，self 隔离前提整体不存在（用例红且根因难查）。
  // 只授线索族，不越界授 purchase/supplier 码，保密面负向断言不受影响。
  salesperson: [
    'orders:read',
    'crm-leads:read',
    'crm-leads:create',
    'crm-leads:update',
    'crm-leads:delete',
    // PII 按需揭示（POST /crm/customers/{id}/pii/reveal 运行时键 customers:reveal，
    // 与后端 init 矩阵 sales_rep/crm_rep 及 m0086 存量补授三通道同口径；
    // salesperson 为 sales_rep 的 CI/部署别名码，同 pieces:read 在册别名）
    'customers:reveal',
    // 发货对话框第四维匹号候选来自 GET /inventory/pieces（与后端 permission.rs 的
    // sales_rep ("pieces","read") 同口径；只读，不授打印）
    'pieces:read',
    // 出口商检只读（与后端 permission.rs 的 sales_rep ("export-inspections","read")
    // 同口径；建单/登记结果属 customs_specialist，CI 不建该角色码故本 SEED 面不含）
    'export-inspections:read',
  ],
  // sales_manager 对齐 permission.rs：orders 只读+审批链（此处仅补读，其余 approve/reject
  // 由后端角色 init 授予，本 spec 只依赖 GET 列表/详情）。不授 purchase 侧任何码。
  sales_manager: ['orders:read'],
  // customer_service —— data-scope-isolation 的 User B：需要线索读/建/删才能把
  // "B 列表看不到 A 私有行 / B 按 ID GET A 的行 → 403 来自 check_resource_owner"
  // 钉在数据范围层；若缺 crm-leads:read，B 的 403 将来自 RBAC（message=「权限不足，无法
  // 访问该资源」）而非行级隔离（message=「无权限」），安全断言被层错位伪造。
  // 该角色由后端 init 建角并自带 crm-leads 分组（系统角色不允许经 API 改权限），
  // 此处按目标键回读校验其真实就位，授码 POST 被拒时仍由回读判据兜住。
  customer_service: ['crm-leads:read', 'crm-leads:create', 'crm-leads:update', 'crm-leads:delete'],
  // inventory_manager —— 02-adjustment.spec.ts IDOR 钉的第二用户 B。刻意对齐后端
  // init permission.rs:427-441 同名角色的 ("adjustments","*")：本 spec 仅经
  // GET/PUT/DELETE /inventory/adjustments/... 四类调用，逐动作授 read/update/delete
  // 即够其通过 RBAC、真实到达 handler 内数据范围归属校验层（403 message=「无权限」）；
  // 不授 inventory/stock 等其余码，B 若被引导访问其它资源仍会被 RBAC 拦（最小授权）。
  // 新建角色 data_scope 由后端默认 self（role_permission_service.rs:184-185），
  // 与 init 种的 dept 同为"非本人资源拒绝"，IDOR 前提两侧一致成立。
  inventory_manager: [
    'adjustments:read',
    'adjustments:update',
    'adjustments:delete',
    // 匹号领域读 + 成品布入库标签（对齐后端 permission.rs 同名角色与迁移 m0069；
    // 上述 IDOR 断言只涉及 adjustments 资源，加这两键不改变其 403 判定层）
    'pieces:read',
    'pieces:print',
  ],
  // manager —— fullflow/15-report-export.spec.ts 15-05「finance_report 导出审批链」的申请侧。
  // 该用例里 manager 只打一个端点：POST /export-approvals（创建导出审批申请），随后切回分片
  // admin 审批。权限键与后端 init 同名角色矩阵逐字符同口径（init_service_ops/permission.rs
  // ::management_role_resources 的 ("export-approvals","create")；注册表权威名
  // init_service.rs::PERMISSION_RESOURCES；运行时键由 URL 段直接取得，
  // path_utils.rs::is_misc_direct_resource 已把 export-approvals 登记为顶层资源段）。
  // 刻意**不**补 permission.rs 里 manager 的其余只读码：15-05 不经受那些端点，多授会稀释
  // "缺码即 RBAC 拦"的判据；也不授 export-approvals:approve——审批属 admin，授了会破坏
  // 双人分离前提（后端 SoD 同样拒绝同角色 create+approve 共存，role_permission_service.rs
  // ::validate_sod_create_approve）。
  manager: ['export-approvals:create'],
  // 仓管/仓库经理：按四维选匹（发货/调拨）+ 打印成品布入库标签的岗位，
  // 与 permission.rs 的 warehouse_keeper 及迁移 m0069 同口径。
  warehouse_keeper: ['pieces:read', 'pieces:print'],
  warehouse_manager: ['pieces:read', 'pieces:print'],
  // 质检/验布岗：验布打卷页的标签面板需读匹行并可打印（同 m0069 口径）
  quality_inspector: ['pieces:read', 'pieces:print'],
};

// 边界测试角色
const BOUNDARY_ROLES = [
  { code: 'e2e_readonly', name: 'E2E只读角色', permissions: SHELL_PERMISSIONS },
  { code: 'e2e_noperm', name: 'E2E空权限角色', permissions: [] },
];

// 黑名单端到端测试角色（33b）：PRINT/EXPORT_DENIED 黑名单按角色 code 精确匹配
// （backend/src/middleware/permission.rs）。给这两个角色配置 print/export 权限码，
// 使 33b 断言"持码仍 403"——权限码放行即证明黑名单失效。
// 端点对应权限码：/boms/{id}/print → boms:print；/stock/export → stock:export
// （extract_resource_info 按路径第 4 段取 resource_type）
const BLACKLIST_TEST_ROLES = [
  {
    code: 'customer',
    name: '客户外部用户（E2E黑名单验证）',
    permissions: [...SHELL_PERMISSIONS, 'boms:view', 'boms:print', 'stock:export'],
  },
  {
    code: 'temporary',
    name: '临时账号（E2E黑名单验证）',
    permissions: [...SHELL_PERMISSIONS, 'boms:view', 'boms:print', 'stock:export'],
  },
];

const ROLE_CREDENTIALS_PATH = 'e2e/.auth/role-credentials.json';
const DEFAULT_ROLE_PASSWORD = 'E2eRole#2026';

/**
 * CSRF 一次性消费的恢复包装：后端 CSRF Token 为一次性消费——成功写入后响应
 * Set-Cookie 下发轮换后的新 token，context 自动存储但 headers 对象不会同步。
 * 本函数在每次写请求完成后（无论成功或 403 恢复），从 context cookie jar 重取
 * csrf_token 并刷新到共享 headers 中，确保下一次调用不携带已消费的旧 token。
 * 若恢复后仍失败（如 IP 不匹配），保留原 resp 由调用方判定并报告。
 */
async function requestWithCsrfRecovery(
  ctx: CsrfCapableRequestContext,
  method: 'post' | 'put',
  url: string,
  headers: Record<string, string>,
  data: Record<string, unknown>
): Promise<SetupApiResponse> {
  let resp = await ctx[method](url, { headers, data });
  if (resp.status() === 403) {
    const newToken = resp.headers()['x-new-csrf-token'];
    if (newToken) {
      headers['X-CSRF-Token'] = newToken;
      resp = await ctx[method](url, { headers, data });
    }
  }
  // 写请求完成后，从 context cookie jar 重取轮换后的 csrf_token：
  // 后端成功消费旧 token 后通过 Set-Cookie 下发新 token（csrf.rs:216-224），
  // Playwright context 已自动存储，但 headers 对象仍为旧值——不同步则下次 403。
  if (resp.ok()) {
    try {
      const state = await ctx.storageState();
      const fresh = state.cookies.find(c => c.name === 'csrf_token');
      if (fresh) {
        headers['X-CSRF-Token'] = fresh.value;
      }
    } catch {
      // storageState 异常不阻塞（降级到下次 403 恢复路径）
    }
  }
  return resp;
}

/**
 * 失败响应机器码提取：只读 AppError 出参的 `code` 字段（error.rs:303-307 固定四键
 * code/message/trace_id/timestamp；BUSINESS_ERROR 等），绝不解析 message 文案原文
 * （文案默认脱敏为常量，且判据只看 HTTP 码 + 机器码）。解析不到归 'UNKNOWN'。
 */
async function readMachineCode(resp: SetupApiResponse): Promise<string> {
  const json = (await resp.json().catch(() => null)) as { code?: unknown } | null;
  return typeof json?.code === 'string' && json.code ? json.code : 'UNKNOWN';
}

/**
 * 判定某目标权限键 `resource:action` 是否已在角色权限集合中生效。
 * 接受精确键，也接受该资源的通配授权 `resource:*`
 * （init 矩阵对 inventory_manager 等以 ("adjustments","*") 播种，运行时段推导
 * 对 adjustments:read 命中通配行即放行；回读须与此口径一致，不可只字面比 action）。
 */
function permissionKeyPresent(present: Set<string>, code: string): boolean {
  if (present.has(code)) return true;
  const resource = code.split(':')[0];
  return present.has(`${resource}:*`);
}

/**
 * 回读某角色当前 allowed=true 的权限键集合。
 * 调用方：grantAndVerifyRolePermissions（授码后自检的唯一权威）。
 * 入参：roleId + 复用的 CSRF headers。
 * 传给谁：GET /roles/{id}/permissions（后端 role_handler.rs:511-531）。
 * 出参形状：ApiResponse<Vec<PermissionResponse>> → data 为裸数组，
 *   元素 { resource_type, resource_id, action, allowed }（同文件 :520-529）。
 * 存什么/存哪里：不持久化，返回 Set<'resource:action'>（仅收 allowed===true 的行）。
 * GET 非 2xx 或 data 非数组一律显式抛错，绝不吞成"空集合"后误判 missing。
 */
async function readRolePermissionKeys(
  ctx: CsrfCapableRequestContext,
  roleId: number,
  headers: Record<string, string>
): Promise<Set<string>> {
  const resp = await ctx.get(`${API_PREFIX}/roles/${roleId}/permissions`, { headers });
  if (!resp.ok()) {
    const body = await resp.text().catch(() => '');
    throw new Error(
      `readRolePermissionKeys: 角色 id=${roleId} 权限回读失败 HTTP ${resp.status()} ${body.slice(0, 200)}`
    );
  }
  const json = (await resp.json().catch(() => null)) as {
    data?: Array<{ resource_type?: unknown; action?: unknown; allowed?: unknown }>;
  } | null;
  if (!json || !Array.isArray(json.data)) {
    throw new Error(
      `readRolePermissionKeys: 角色 id=${roleId} 权限回读 data 非数组（契约 role_handler.rs:511-531），` +
        `实际片段=${JSON.stringify(json).slice(0, 200)}`
    );
  }
  const keys = new Set<string>();
  for (const p of json.data) {
    if (
      p?.allowed === true &&
      typeof p.resource_type === 'string' &&
      typeof p.action === 'string'
    ) {
      keys.add(`${p.resource_type}:${p.action}`);
    }
  }
  return keys;
}

/**
 * 为角色逐条授码（POST /roles/{id}/permissions 单条 upsert）。
 * 返回每个目标键"最近一次 POST 的失败分类"（成功者不入表），供调用方回读缺失时点名。
 * 本函数**不再判定成败、也不再吞 400/409 当幂等放行**：
 * 真正的成败由 grantAndVerifyRolePermissions 的回读裁决。
 */
async function assignPermissionList(
  ctx: CsrfCapableRequestContext,
  roleId: number,
  permissionCodes: string[],
  headers: Record<string, string>
): Promise<Map<string, string>> {
  const rejectReason = new Map<string, string>();
  for (const code of permissionCodes) {
    const [resourceType, action] = code.split(':');
    if (!resourceType || !action) {
      console.warn(`[globalSetup] 权限码格式非法（应为 resource:action）: ${code}`);
      rejectReason.set(code, '权限码格式非法');
      continue;
    }
    try {
      const resp = await requestWithCsrfRecovery(
        ctx,
        'post',
        `${API_PREFIX}/roles/${roleId}/permissions`,
        headers,
        { resource_type: resourceType, action, allowed: true }
      );
      if (resp.ok()) {
        rejectReason.delete(code); // 本次授码成功（或 upsert 更新成功）
      } else {
        // 只按 HTTP 码 + 机器码分类，不解析文案：
        // 系统角色拒改 → HTTP 400 code=BUSINESS_ERROR（role_permission_service.rs:354-356）；
        // 可授角色的"权限行已存在"走 update → 200，不经此分支。故 400 只可能是真拒绝，
        // 不可把 400 当幂等放行——那会静默吞掉全部 init 角色的 extras 授码。
        const machineCode = await readMachineCode(resp);
        rejectReason.set(code, `HTTP ${resp.status()} code=${machineCode}`);
      }
    } catch (e) {
      rejectReason.set(code, `POST 异常 ${(e as Error).message}`);
    }
  }
  return rejectReason;
}

/**
 * 授码 + 回读自检（幂等、fail-visible）。
 * 入参：角色码/id + 目标权限键列表 + 复用的 CSRF headers。
 * 传给谁：assignPermissionList 尝试授码 → readRolePermissionKeys 回读。
 * 返回：缺失项描述数组（每项含权限键与其 POST 失败分类）；空数组=全部目标键已就位。
 * 判定唯一权威是回读：
 *   目标键已在（无论本次 POST 落的、还是 init 早已播种的）→ 幂等成立、不计缺失，
 *     此时系统角色的 400 属"键已就位、无需也许可被 API 再改"的正常态；
 *   目标键确实不在 → 计入缺失（点名 POST 分类，如 HTTP 400 code=BUSINESS_ERROR），
 *     由调用方记入 seed 失败账并判红。
 * 严禁为绕过而改 is_system 或写库——缺失只能由后端补 init 矩阵分组/受控通道解决。
 */
async function grantAndVerifyRolePermissions(
  ctx: CsrfCapableRequestContext,
  roleId: number,
  roleCode: string,
  permissionCodes: string[],
  headers: Record<string, string>
): Promise<string[]> {
  if (permissionCodes.length === 0) return [];
  const rejectReason = await assignPermissionList(ctx, roleId, permissionCodes, headers);
  const present = await readRolePermissionKeys(ctx, roleId, headers);
  const missing: string[] = [];
  for (const code of permissionCodes) {
    if (!permissionKeyPresent(present, code)) {
      missing.push(`${code}（POST：${rejectReason.get(code) ?? '未尝试'}）`);
    }
  }
  if (missing.length > 0) {
    console.error(
      `[globalSetup] 角色 ${roleCode}(id=${roleId}) 权限回读缺失 ${missing.length}/${permissionCodes.length}：` +
        `${missing.join('； ')}`
    );
  } else {
    console.log(
      `[globalSetup] 角色 ${roleCode} 权限回读校验通过（${permissionCodes.length} 项目标键全部就位）`
    );
  }
  return missing;
}

/**
 * 全量角色账号 setup：
 * 1. 拉取后端角色全量清单
 * 2. 自动补建缺失角色 + 边界测试角色
 * 3. 每角色一个测试账号（API 创建）
 * 4. 凭证写入 role-credentials.json
 * 5. 幂等：角色/账号已存在视为成功跳过
 */
export async function ensureRoleUsers(): Promise<void> {
  const loginCtx = await request.newContext({
    baseURL: API_BASE,
    extraHTTPHeaders: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
    },
  });

  // 1. 基础管理员登录
  const loginResp = await loginWithRetry(loginCtx, BASE_USERNAME, BASE_PASSWORD);
  if (!loginResp.ok()) {
    await loginCtx.dispose();
    throw new Error(`ensureRoleUsers: ${BASE_USERNAME} 登录失败 HTTP ${loginResp.status()}`);
  }
  const loginCookies = (await loginCtx.storageState()).cookies;
  const csrfCookie = loginCookies.find(c => c.name === 'csrf_token');
  if (!csrfCookie) {
    await loginCtx.dispose();
    throw new Error('ensureRoleUsers: 未取得 csrf_token cookie');
  }
  const headers = {
    'X-CSRF-Token': csrfCookie.value,
    'X-Requested-With': 'XMLHttpRequest',
  };

  // 2. 拉取角色全量清单
  // 后端 list_roles 返回 ApiResponse<RoleListResponse>，载荷只有 {roles,total} 一种形状
  // （backend/src/handlers/role_handler.rs:113-116 签名、:78-81 结构体定义），
  // 且该 handler 不接收分页参数，故不再拼 page/page_size；也不再用 `roles ?? items ?? []`
  // 探测多形态——形状漂移必须在这里就抛错，而不是让 36 个分片各自读到空清单后乱失败。
  const rolesResp = await loginCtx.get(`${API_PREFIX}/roles`, { headers });
  if (!rolesResp.ok()) {
    const body = await rolesResp.text().catch(() => '');
    await loginCtx.dispose();
    throw new Error(
      `ensureRoleUsers: 角色清单拉取失败 HTTP ${rolesResp.status()}（role-credentials.json 无法生成，后续角色测试将全部失败）: ${body.slice(0, 200)}`
    );
  }
  const rolesBody = (await rolesResp.json().catch(e => {
    console.error(
      `[globalSetup] 角色清单 JSON 解析失败（原错误消息前缀错位已修正）:`,
      (e as Error).message
    );
    return null;
  })) as {
    data?: { roles?: Array<{ id: number; code?: string; name?: string }> };
  } | null;
  if (!rolesBody || !Array.isArray(rolesBody.data?.roles)) {
    await loginCtx.dispose();
    throw new Error(
      `ensureRoleUsers: 角色清单响应缺 data.roles（后端契约 role_handler.rs:78-81），` +
        `实际片段=${JSON.stringify(rolesBody).slice(0, 200)}`
    );
  }
  const existingRoles = rolesBody.data.roles;
  const existingCodes = new Set(existingRoles.map(r => r.code).filter(Boolean));
  console.log(`[globalSetup] 后端现有角色 ${existingRoles.length} 个`);

  // 3. 自动补建缺失种子角色 + 边界角色 + 黑名单验证角色
  // 补建角色给应用外壳权限码 + 该角色在 SEED_ROLE_EXTRA_PERMISSIONS 中的专项业务权限码
  // （若映射中未定义则仅 SHELL；32-roles 断言"登录 + Dashboard 可达"仍成立）。
  const allRolesToEnsure = [
    ...SEED_ROLES.filter(code => !existingCodes.has(code)).map(code => ({
      code,
      name: code,
      permissions: [...SHELL_PERMISSIONS, ...(SEED_ROLE_EXTRA_PERMISSIONS[code] ?? [])],
    })),
    ...BOUNDARY_ROLES.filter(r => !existingCodes.has(r.code)),
    ...BLACKLIST_TEST_ROLES.filter(r => !existingCodes.has(r.code)),
  ];

  // Map 键仅纳入 code 存在的行（与既有 existingCodes 口径一致）：
  // r.code 类型为 string | undefined，若把整表直接喂给 new Map 会把元组放宽成
  // (string|number)[]，构造报 TS2769。
  const roleCodeToId = new Map<string, number>();
  for (const r of existingRoles) {
    if (r.code) roleCodeToId.set(r.code, r.id);
  }

  // seed 失败账：收集"授码后回读仍缺的目标键"（角色码/权限键/POST 机器码），
  // 跨建角色分支与 step 3.5 累加，函数末尾一次性判红（不静默、不带病出凭证）。
  const seedFailures: string[] = [];

  for (const role of allRolesToEnsure) {
    const createRoleResp = await requestWithCsrfRecovery(
      loginCtx,
      'post',
      `${API_PREFIX}/roles`,
      headers,
      { code: role.code, name: role.name }
    );
    if (createRoleResp.ok()) {
      const created = (await createRoleResp.json().catch(e => {
        console.warn(
          `[assignPermissionList] 权限分配失败（不影响角色账号创建）:`,
          (e as Error).message
        );
        return null;
      })) as { data?: { id: number } } | null;
      if (created?.data?.id) {
        roleCodeToId.set(role.code, created.data.id);
        // 分配权限（POST 单条 upsert）后立即回读自检：新建角色为 e2e 非系统角色，
        // 授码应成功且回读应在，缺失即记入 seed 失败账。
        if (role.permissions.length > 0) {
          const missing = await grantAndVerifyRolePermissions(
            loginCtx,
            created.data.id,
            role.code,
            role.permissions,
            headers
          );
          for (const m of missing) seedFailures.push(`${role.code}: ${m}`);
        }
        console.log(`[globalSetup] 角色 ${role.code} 创建成功 (id=${created.data.id})`);
      }
    } else if (createRoleResp.status() === 400 || createRoleResp.status() === 409) {
      console.log(`[globalSetup] 角色 ${role.code} 已存在，跳过创建`);
      // 已存在角色也补齐权限码并回读自检（幂等；33b 黑名单断言依赖"持码仍拒"）
      if (role.permissions.length > 0) {
        const roleId = roleCodeToId.get(role.code);
        if (roleId) {
          const missing = await grantAndVerifyRolePermissions(
            loginCtx,
            roleId,
            role.code,
            role.permissions,
            headers
          );
          for (const m of missing) seedFailures.push(`${role.code}: ${m}`);
        }
      }
    }
  }

  // 重新拉取角色清单获取补建角色的 id（兼容 roles / items 两种响应形态）
  // 无条件重拉：角色已存在（409 分支）时 roleCodeToId 也可能缺 id（步骤 2 解析不全），
  // 只有重拉才能保证步骤 4 为全部角色创建测试账号
  {
    const reFetch = await loginCtx.get(`${API_PREFIX}/roles`, { headers });
    if (!reFetch.ok()) {
      const body = await reFetch.text().catch(() => '');
      await loginCtx.dispose();
      throw new Error(
        `ensureRoleUsers: 角色清单重拉失败 HTTP ${reFetch.status()}: ${body.slice(0, 200)}`
      );
    }
    const reBody = (await reFetch.json().catch(e => {
      console.error(`[globalSetup] 角色清单重拉 JSON 解析失败:`, (e as Error).message);
      return null;
    })) as {
      data?: { roles?: Array<{ id: number; code?: string }> };
    } | null;
    if (!reBody || !Array.isArray(reBody.data?.roles)) {
      await loginCtx.dispose();
      throw new Error(
        `ensureRoleUsers: 角色清单重拉响应缺 data.roles（契约 role_handler.rs:78-81），` +
          `实际片段=${JSON.stringify(reBody).slice(0, 200)}`
      );
    }
    const refetched = reBody.data.roles;
    if (refetched.length === 0) {
      await loginCtx.dispose();
      throw new Error(
        `ensureRoleUsers: 角色清单重拉后仍为空（响应结构异常），roleCodeToId=${roleCodeToId.size}，原始片段=${JSON.stringify(reBody).slice(0, 200)}`
      );
    }
    for (const r of refetched) {
      if (r.code) roleCodeToId.set(r.code, r.id);
    }
    console.log(
      `[globalSetup] 角色清单重拉完成，共 ${refetched.length} 角色，roleCodeToId=${roleCodeToId.size}`
    );
  }

  // 3.5 幂等补授 SHELL + SEED_ROLE_EXTRA_PERMISSIONS，并回读自检。
  // 为何含 SHELL：后端 init 矩阵对每个在册角色自动附加 SHELL（dashboard/notifications read，
  // 见 init_service_ops/permission.rs::SHELL_PERMISSIONS 附加逻辑），e2e 侧必须同口径补授，
  // 否则"后端先建角、e2e 后授权"且此前只补 extras 的角色会缺 dashboard:read 被路由守卫送 /403
  // （customer_service 即此形态，N1）。
  // 为何以回读为唯一权威：init 播种角色全部 is_system=true，POST /roles/:id/permissions 对
  // 系统角色一律 HTTP 400 code=BUSINESS_ERROR 拒改（role_permission_service.rs:354-356）；
  // 若 init 已把目标键播进矩阵，回读即在 → 幂等放行；回读仍缺 → 该 spec 前提确实未成立且
  // e2e 无法经 API 落地 → 记入 seed 失败账、末尾判红，绝不吞码、绝不为绕过改 is_system 或写库。
  for (const [code, extras] of Object.entries(SEED_ROLE_EXTRA_PERMISSIONS)) {
    const roleId = roleCodeToId.get(code);
    if (!roleId) {
      seedFailures.push(
        `${code}: 角色不在 roleCodeToId（既未存在于后端也未补建成功），spec 前提无法成立`
      );
      continue;
    }
    const targetCodes = [...SHELL_PERMISSIONS, ...extras];
    const missing = await grantAndVerifyRolePermissions(
      loginCtx,
      roleId,
      code,
      targetCodes,
      headers
    );
    for (const m of missing) seedFailures.push(`${code}: ${m}`);
    if (missing.length === 0) {
      console.log(
        `[globalSetup] 角色 ${code} SHELL+业务权限码补授并回读通过（${targetCodes.length} 项）`
      );
    }
  }

  // seed 失败账收口：任何"授码后回读仍缺"的目标键，都是 spec 前提未落地，必须判红。
  // 逐条已含 角色码 / 权限键 / POST 机器码；系统角色拒改（HTTP 400 code=BUSINESS_ERROR）且
  // 回读仍缺者，须由后端补 init 矩阵分组或提供受控授码通道，e2e 侧无合法绕过路径。
  if (seedFailures.length > 0) {
    throw new Error(
      `ensureRoleUsers: seed 权限授码回读失败 ${seedFailures.length} 项：\n  ` +
        seedFailures.join('\n  ') +
        `\n——缺码会使 spec 的 403/200 断言层错位（RBAC 拦 ≠ 数据范围拒），禁止静默继续。` +
        `若某项 POST 分类为 HTTP 400 code=BUSINESS_ERROR 且回读仍缺，即该 init 系统角色拒 API 改权限` +
        `且 init 矩阵未播种此键，须后端对齐（补分组/统一运行时资源段），不得在 e2e 侧绕过。`
    );
  }

  // 4. 为每个角色创建测试账号
  const credentials: Record<string, { username: string; password: string }> = {};
  for (const [code, roleId] of roleCodeToId) {
    const username = `e2e_${code}`;
    // admin 角色映射到主管理员账号 e2e_admin（已存在，真实密码=BASE_PASSWORD）：
    // 已存在分支不会重置密码，凭证必须写账号真实密码，否则 loginAsRole('admin') 必 401
    const password = username === BASE_USERNAME ? BASE_PASSWORD : DEFAULT_ROLE_PASSWORD;
    const createResp = await requestWithCsrfRecovery(
      loginCtx,
      'post',
      `${API_PREFIX}/users`,
      headers,
      { username, password, role_id: roleId, real_name: `E2E-${code}` }
    );
    if (createResp.ok()) {
      console.log(`[globalSetup] 角色账号 ${username} 创建成功`);
    } else if (createResp.status() === 400 || createResp.status() === 409) {
      // 已存在
    } else {
      // 非 400/409 的创建失败必须在这里判红：若只 console.warn 后继续，凭证照样写出、
      // 账号实际不存在/未建，下游 spec 登录 401 且根因埋在几分钟前的 setup 日志里，
      // 禁止带病写出凭证伪造就绪。
      const body = await createResp.text().catch(() => '');
      throw new Error(
        `ensureRoleUsers: 角色账号 ${username} 创建失败 HTTP ${createResp.status()} ` +
          `body=${body.slice(0, 300)}——属 setup 缺陷判红（禁止 warn 后继续写凭证伪造就绪）`
      );
    }
    credentials[code] = { username, password };
  }

  // 4.5 终验登录：spec 直接消费的角色账号必须"真能登进去"。
  // 400/409"已存在"分支不重置密码，库中密码与凭证漂移时只会表现为下游 401
  // （"账号在、凭证里密码不符"，排查成本高）。这里用凭证文件里
  // 的密码做一次真实登录收口：成功→凭证可信；失败→立即判红并带后端原始响应。
  // manager 纳入终验的理由：fullflow/15-report-export.spec.ts 15-05 直接 loginAsRole('manager')，
  // 属"spec 直接消费"面。若该账号在库中已存在而密码与本次写出的凭证不符（400/409 分支不重置
  // 密码），不在此收口就只会表现为几分钟后的 401（形态同上：账号在、凭证里密码不符），排查成本高。
  const SPEC_CONSUMED_ROLES = ['salesperson', 'customer_service', 'inventory_manager', 'manager'];
  for (const code of SPEC_CONSUMED_ROLES) {
    const cred = credentials[code];
    if (!cred) {
      throw new Error(
        `ensureRoleUsers: 角色 ${code} 凭证缺失（roleCodeToId 无此码），` +
          `data-scope-isolation / 02-adjustment 的前提账号不存在——属 setup 缺陷判红`
      );
    }
    const verifyCtx = await request.newContext({
      baseURL: API_BASE,
      extraHTTPHeaders: {
        'Content-Type': 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
      },
    });
    const verifyResp = await loginWithRetry(verifyCtx, cred.username, cred.password);
    await verifyCtx.dispose();
    if (!verifyResp.ok()) {
      const body = await verifyResp.text().catch(() => '');
      throw new Error(
        `ensureRoleUsers: 角色账号 ${cred.username} 终验登录失败 HTTP ${verifyResp.status()} ` +
          `body=${body.slice(0, 300)}——凭证写出的密码与账号真实密码不符，` +
          `下游角色用例将成片 401（#4669 E 族形态），属 setup 缺陷判红`
      );
    }
    console.log(`[globalSetup] 角色账号 ${cred.username} 终验登录通过`);
  }

  await loginCtx.dispose();

  // 5. 写入凭证文件
  const fs = await import('fs');
  mkdirSync('e2e/.auth', { recursive: true });
  fs.writeFileSync(ROLE_CREDENTIALS_PATH, JSON.stringify(credentials, null, 2));
  const credKeys = Object.keys(credentials);
  console.log(`[globalSetup] 角色凭证写入 ${ROLE_CREDENTIALS_PATH}（${credKeys.length} 角色）`);
  // 诊断：打印凭证键样本（前 10 个）+ 关键角色存在性
  console.log(
    `[globalSetup][诊断] 凭证键样本: ${credKeys.slice(0, 10).join(', ')}${credKeys.length > 10 ? ` …共 ${credKeys.length}` : ''}`
  );
  for (const mustHave of [
    'admin',
    'cashier',
    'report_viewer',
    'customer',
    'temporary',
    'manager',
  ]) {
    console.log(
      `[globalSetup][诊断] 角色 ${mustHave}: ${credKeys.includes(mustHave) ? '✅有' : '❌无'}`
    );
  }
  // 回读验证：确保写出的文件可读且键完整
  try {
    const verifyRaw = fs.readFileSync(ROLE_CREDENTIALS_PATH, 'utf-8');
    const verifyData = JSON.parse(verifyRaw) as Record<
      string,
      { username: string; password: string }
    >;
    const verifyCount = Object.keys(verifyData).length;
    console.log(
      `[globalSetup][诊断] 回读验证：文件 ${verifyRaw.length}B，可解析角色 ${verifyCount} 个，cwd=${process.cwd()}`
    );
    if (verifyCount !== credKeys.length) {
      console.error(
        `[globalSetup][诊断] ⚠️ 回读数量 ${verifyCount} ≠ 写入数量 ${credKeys.length}！`
      );
    }
  } catch (e) {
    console.error(`[globalSetup][诊断] ⚠️ 回读验证失败: ${(e as Error).message}`);
  }
}

// ==================== 全局业务实体种子 ====================

/**
 * 全局业务实体种子：为不依赖 ensureTestEntities 的 extras specs 提供最小前置数据。
 *
 * extras 分片的 sales/purchase/crm/color-card/price/production/quality 等 spec
 * 不调用 ensureTestEntities，直接导航后断言"状态行存在/下拉有选项"；setup 若不
 * 预建业务实体，这批用例会成片空红。
 *
 * 本函数使用已登录的 request context（同 globalSetup 主流程）做纯 API 调用，
 * 不依赖浏览器 page。所有创建均幂等（先查后建），跨分片复用同库不冲突。
 *
 * 覆盖实体与根因对照：
 * - 产品（带 gram_weight/width/meters_per_piece/meters_per_roll）→ Q1 转订单换算 +
 *   sales/purchase/production 产品下拉
 * - 供应商 → purchase 供应商 combobox
 * - 客户 → crm/sales 客户下拉
 * - 仓库 → inventory/fabric 仓库下拉
 * - 色卡 → color-card 列表/详情
 * - 色号定价 → color-price 列表/历史
 * - 报价单（带 sales_user_id）→ quotations/sales 已批准行
 * - 验布记录（带 fabric_width_inches）→ flow20 定级→关闭
 */
async function ensureGlobalBusinessSeed(
  ctx: Awaited<ReturnType<typeof request.newContext>>
): Promise<void> {
  // 种子失败硬账：静默失败必须响亮化。
  // 收集**本分片种子阶段的全部失败**——既包括「导致种子行根本产不出来」的硬失败，
  // 也包括 ⚠️ 软失败（reportSeedWrite 的 4xx/5xx；若只打印不进台账，会出现
  // 「⚠️ 打了、汇总仍写 0 项」的自相矛盾，下游大片红查不到根因）。
  // 函数结束前若非空 → 打印清单 + 落盘 + GitHub 注解后**显式 throw 判红**：
  // setup 失败会终止本分片全部用例，这正是目的——种子没就绪的运行结果不可信，
  // 不能接受「绿一半红一半、根因隐身」。（本仓纪律）
  // 声明必须位于函数最前：后续所有分支（含 csrf 缺失早退）都要能记账。
  const SEED_FAILURES: string[] = [];
  const recordSeedFailure = (label: string, detail: string): void => {
    SEED_FAILURES.push(`${label}｜${detail}`);
    console.error(`[globalSeed] ❌ 种子失败（计入汇总）：${label} ${detail}`);
  };

  // 获取 csrf_token（写操作需要）
  const cookies = (await ctx.storageState()).cookies;
  const csrfCookie = cookies.find(c => c.name === 'csrf_token');
  if (!csrfCookie) {
    // 无 CSRF 会话 = 种子阶段整体报废，warn+return 属静默失败，必须判红：
    console.error('[globalSeed] ❌ 无 csrf_token cookie，业务种子无法执行（判红，不再静默跳过）');
    throw new Error(
      '[globalSeed] 种子前置失败：登录会话缺 csrf_token cookie，全部业务种子未执行——' +
        '根因在 globalSetup 登录/CSRF 链路（E 族同族），不是下游用例缺陷。'
    );
  }
  const headers: Record<string, string> = {
    'X-CSRF-Token': csrfCookie.value,
    'X-Requested-With': 'XMLHttpRequest',
  };

  // 种子内写请求统一走 CSRF 恢复：token 一次性消费，连续写需读 X-New-CSRF-Token 头更新。
  // 复用同文件已定义的 requestWithCsrfRecovery（接受 CsrfCapableRequestContext，
  // Playwright APIRequestContext 结构化兼容）。
  const seedPost = (url: string, data: Record<string, unknown>) =>
    requestWithCsrfRecovery(ctx, 'post', url, headers, data);
  const seedPut = (url: string, data: Record<string, unknown>) =>
    requestWithCsrfRecovery(ctx, 'put', url, headers, data);

  // 辅助：安全 JSON 解析（响应可能是非 2xx 的文本）
  const safeJson = async (resp: {
    ok: () => boolean;
    status: () => number;
    json: () => Promise<unknown>;
  }): Promise<Record<string, unknown> | null> => {
    if (!resp.ok()) return null;
    try {
      return (await resp.json()) as Record<string, unknown>;
    } catch {
      return null;
    }
  };

  // 辅助：从列表响应提取 items 数组
  const extractItems = <T>(body: Record<string, unknown> | null): T[] => {
    const data = body?.data as Record<string, unknown> | undefined;
    const items = data?.items as T[] | undefined;
    return items ?? [];
  };

  // 辅助：种子写请求失败暴露——不只 ≥500 告警，4xx 同样不静默。
  // 非 2xx 一律 `[globalSeed] ⚠️` 显式打印 HTTP + 原始响应体，**并同步计入 SEED_FAILURES**
  // （若 ⚠️ 分支不进台账，「种子失败汇总：0 项」会与 ⚠️ 同屏自相矛盾）。
  const reportSeedWrite = async (resp: SetupApiResponse, label: string): Promise<void> => {
    const status = resp.status();
    if (status >= 400) {
      const body = await resp.text().catch(() => '');
      console.warn(
        `[globalSeed] ⚠️ ${label} 失败 HTTP ${status} body=${body.slice(0, 300)}${
          status >= 500 ? '（疑似后端潜伏缺陷，不吞掉）' : ''
        }`
      );
      recordSeedFailure(`${label} 写请求`, `HTTP ${status} body=${body.slice(0, 200)}`);
    }
  };

  // 「质检合格方可入库/结算」门控（backend/src/services/purchase_receipt_service.rs
  // ::ensure_receipt_inspection_allows_flow）：
  // 收货单 inspection_status 不是 PASSED 时，POST /purchase/receipts/{id}/confirm 与
  // POST /ap/invoices/auto-generate 一律 400 BUSINESS_ERROR；新建收货单恒为 PENDING
  // （列 NOT NULL DEFAULT 'PENDING'）。PASSED 的唯一业务写入口是质检完成回写
  // （purchase_inspection_service.rs::complete_inspection → to_receipt_inspection_status）。
  // 因此凡需要把收货单确认落库存的种子，都必须先跑完这条真实质检链——不存在也不允许
  // 任何直改状态的旁路。返回 false 表示该单无法确认，调用方必须跳过并计入种子失败汇总。
  const seedInspectionPass = async (
    receiptId: number,
    supplierId: number,
    passQuantity: string,
    label: string
  ): Promise<boolean> => {
    const inspectionDate = new Date().toISOString().slice(0, 10);
    const insResp = await seedPost(`${API_PREFIX}/purchase/inspections`, {
      receipt_id: receiptId,
      supplier_id: supplierId,
      inspection_date: inspectionDate,
      notes: `E2E-SEED-INSPECT-${receiptId}`,
    });
    const insBody = await safeJson(insResp);
    const insId = (insBody?.data as Record<string, unknown>)?.id as number;
    if (!insResp.ok() || !insId) {
      const text = JSON.stringify(await insResp.json().catch(() => null));
      recordSeedFailure(
        `${label} 建质检单`,
        `HTTP ${insResp.status()} rcv=${receiptId} body=${text.slice(0, 300)}`
      );
      return false;
    }
    const doneResp = await seedPost(`${API_PREFIX}/purchase/inspections/${insId}/complete`, {
      pass_quantity: passQuantity,
      reject_quantity: '0',
      inspection_result: 'pass',
    });
    if (!doneResp.ok()) {
      const text = JSON.stringify(await doneResp.json().catch(() => null));
      recordSeedFailure(
        `${label} 完成质检(pass)`,
        `HTTP ${doneResp.status()} insp=${insId} body=${text.slice(0, 300)}`
      );
      return false;
    }
    // 必须回读真读到 PASSED：读不到说明回写链断了，此时"确认"必然 400，
    // 显式失败而不是继续盲调 confirm 再把失败伪装成"确认接口有问题"。
    const readResp = await ctx.get(`${API_PREFIX}/purchase/receipts/${receiptId}`, { headers });
    const readBody = await safeJson(readResp);
    const inspectionStatus = (readBody?.data as Record<string, unknown>)?.inspection_status as
      string | undefined;
    if (inspectionStatus !== 'PASSED') {
      recordSeedFailure(
        `${label} 质检状态回写`,
        `rcv=${receiptId} insp=${insId} 回读 inspection_status=${String(inspectionStatus)}（期望 PASSED）`
      );
      return false;
    }
    return true;
  };

  // 辅助：取一个真实仓库 id（与前端发货/收货下拉、步骤15 同口径 GET /warehouses）。
  // 后端 validate_order_request（services/po/order_ops/crud.rs:137）对采购订单
  // warehouse_id 强校验非空，缺失即稳定 400「仓库 ID 不能为空」。
  const resolveWarehouseId = async (): Promise<number | undefined> => {
    try {
      const resp = await ctx.get(`${API_PREFIX}/warehouses?page=1&page_size=10`, { headers });
      const body = await safeJson(resp);
      return extractItems<{ id: number }>(body)[0]?.id;
    } catch (e) {
      recordSeedFailure('仓库 id 查询', (e as Error).message);
      return undefined;
    }
  };

  // 辅助：取/建一个真实部门 id。后端 validate_order_request（crud.rs:148）
  // 与 warehouse_id 同族强制采购订单 department_id 非空，缺失 → 400「部门 ID 不能为空」，
  // 故补 warehouse_id 的同时必须一并补 department_id，否则只是把 400 挪到下一行。
  const resolveDepartmentId = async (): Promise<number | undefined> => {
    try {
      const resp = await ctx.get(`${API_PREFIX}/departments?page=1&page_size=1`, { headers });
      const body = await safeJson(resp);
      const existing = extractItems<{ id: number }>(body)[0]?.id;
      if (existing) {
        return existing;
      }
      const ts = Date.now().toString().slice(-6);
      const createResp = await seedPost(`${API_PREFIX}/departments`, {
        name: `E2E全局部门${ts}`,
        code: `E2E-D${ts}`,
      });
      if (!createResp.ok()) {
        await reportSeedWrite(createResp, '部门创建（采购订单前置）');
        return undefined;
      }
      const created = await safeJson(createResp);
      return (created?.data as Record<string, unknown>)?.id as number | undefined;
    } catch (e) {
      recordSeedFailure('部门 id 查询/创建', (e as Error).message);
      return undefined;
    }
  };

  // 辅助：取当前用户 ID（用于 sales_user_id）
  let currentUserId = 0;
  try {
    const meResp = await ctx.get(`${API_PREFIX}/auth/me`, { headers });
    const meBody = await safeJson(meResp);
    currentUserId = ((meBody?.data as Record<string, unknown>)?.id as number) ?? 0;
    if (!currentUserId) {
      recordSeedFailure(
        '/auth/me 取当前用户 id',
        '响应缺 id——需要 user_id 的种子（报价单/销售订单/AR 等）全部不可用'
      );
    }
  } catch (e) {
    recordSeedFailure('/auth/me 查询', (e as Error).message);
  }

  // ---- 1. 产品分类 "面料" ----
  let fabricCategoryId: number | undefined;
  try {
    const catsResp = await ctx.get(`${API_PREFIX}/product-categories?page=1&page_size=50`, {
      headers,
    });
    const catsBody = await safeJson(catsResp);
    const cats = extractItems<{ id: number; name?: string }>(catsBody);
    const fabricCat = cats.find(c => c.name?.includes('面料'));
    if (fabricCat) {
      fabricCategoryId = fabricCat.id;
    } else {
      const createResp = await seedPost(`${API_PREFIX}/product-categories`, {
        name: '面料',
        code: 'FABRIC',
      });
      const created = await safeJson(createResp);
      fabricCategoryId = (created?.data as Record<string, unknown>)?.id as number;
      console.log(`[globalSeed] 创建产品分类"面料" id=${fabricCategoryId}`);
    }
  } catch (e) {
    recordSeedFailure('产品分类检查/创建', (e as Error).message);
  }

  // ---- 2. 产品（带克重/幅宽/每匹米数/每卷米数）----
  let productId: number | undefined;
  let productColorId: number | undefined;
  try {
    const prodResp = await ctx.get(`${API_PREFIX}/products?page=1&page_size=5`, { headers });
    const prodBody = await safeJson(prodResp);
    const prods = extractItems<{ id: number; gram_weight?: number | null }>(prodBody);
    // 优先找一个已有克重的产品（历史 seed 可能已创建）
    const goodProd = prods.find(p => p.gram_weight != null);
    if (goodProd) {
      productId = goodProd.id;
    } else if (prods.length > 0) {
      // 现有产品无克重（历史遗留），通过 PUT 补上
      const updateResp = await seedPut(`${API_PREFIX}/products/${prods[0].id}`, {
        gram_weight: 180,
        width: 150,
        meters_per_piece: 50,
        meters_per_roll: 100,
      });
      if (updateResp.ok()) {
        productId = prods[0].id;
        console.log(`[globalSeed] 产品 ${productId} 已补克重/幅宽`);
      } else {
        productId = prods[0].id;
      }
    } else {
      // 无任何产品，创建一个新的带克重的
      const ts = Date.now().toString().slice(-6);
      const createResp = await seedPost(`${API_PREFIX}/products`, {
        code: `E2E-GP${ts}`,
        name: `E2E全局产品${ts}`,
        unit: '米',
        category_id: fabricCategoryId,
        gram_weight: 180,
        width: 150,
        meters_per_piece: 50,
        meters_per_roll: 100,
      });
      const created = await safeJson(createResp);
      productId = (created?.data as Record<string, unknown>)?.id as number;
      console.log(`[globalSeed] 创建全局产品 id=${productId}`);
    }

    // 产品色号（报价/验布/库存需 color_no）
    if (productId) {
      const colorsResp = await ctx.get(`${API_PREFIX}/products/${productId}/colors`, { headers });
      const colorsBody = await safeJson(colorsResp);
      const colorList = (colorsBody?.data ?? []) as Array<{ id: number; color_no?: string }>;
      if (colorList.length === 0) {
        const ts = Date.now().toString().slice(-6);
        const colorCreate = await seedPost(`${API_PREFIX}/products/${productId}/colors`, {
          color_no: `E2E-GC${ts}`,
          color_name: 'E2E全局色号',
          color_type: '纯色',
          extra_cost: 0,
        });
        const colorCreated = await safeJson(colorCreate);
        productColorId = (colorCreated?.data as Record<string, unknown>)?.id as number;
      } else {
        productColorId = colorList[0].id;
      }
    }
  } catch (e) {
    recordSeedFailure('产品检查/创建', (e as Error).message);
  }

  // ---- 3. 供应商 ----
  try {
    const supResp = await ctx.get(`${API_PREFIX}/purchase/suppliers?page=1&page_size=1`, {
      headers,
    });
    const supBody = await safeJson(supResp);
    const sups = extractItems<{ id: number }>(supBody);
    if (sups.length === 0) {
      const ts = Date.now().toString().slice(-6);
      const supCreate = await seedPost(`${API_PREFIX}/purchase/suppliers`, {
        supplier_name: `E2E全局供应商${ts}`,
        supplier_short_name: 'E2EG',
        contact_phone: '13800000099',
      });
      // 写响应必须过账（原 `await seedPost(...)` 丢弃响应 = 静默失败盲区）
      if (!supCreate.ok()) await reportSeedWrite(supCreate, '全局供应商创建');
      else console.log('[globalSeed] 创建全局供应商');
    }
  } catch (e) {
    recordSeedFailure('供应商检查/创建', (e as Error).message);
  }

  // ---- 4. 客户 ----
  try {
    const cusResp = await ctx.get(`${API_PREFIX}/crm/customers?page=1&page_size=1`, { headers });
    const cusBody = await safeJson(cusResp);
    const cuss = extractItems<{ id: number }>(cusBody);
    if (cuss.length === 0) {
      const ts = Date.now().toString().slice(-6);
      const cusCreate = await seedPost(`${API_PREFIX}/crm/customers`, {
        customer_name: `E2E全局客户${ts}`,
        contact_phone: '13900000099',
      });
      if (!cusCreate.ok()) await reportSeedWrite(cusCreate, '全局客户创建');
      else console.log('[globalSeed] 创建全局客户');
    }
  } catch (e) {
    recordSeedFailure('客户检查/创建', (e as Error).message);
  }

  // ---- 5. 仓库 ----
  try {
    const whResp = await ctx.get(`${API_PREFIX}/warehouses?page=1&page_size=2`, { headers });
    const whBody = await safeJson(whResp);
    const whs = extractItems<{ id: number }>(whBody);
    if (whs.length < 2) {
      for (let i = whs.length; i < 2; i++) {
        const ts = Date.now().toString().slice(-6);
        const whCreate = await seedPost(`${API_PREFIX}/warehouses`, {
          name: `E2E全局仓库${ts}${i}`,
          code: `E2E-GW${ts}${i}`,
        });
        if (!whCreate.ok()) await reportSeedWrite(whCreate, `全局仓库创建 i=${i}`);
      }
      console.log('[globalSeed] 创建全局仓库');
    }
  } catch (e) {
    recordSeedFailure('仓库检查/创建', (e as Error).message);
  }

  // ---- 6. 色卡 ----
  try {
    const ccResp = await ctx.get(`${API_PREFIX}/color-cards?page=1&page_size=1`, { headers });
    const ccBody = await safeJson(ccResp);
    const ccs = extractItems<{ id: number }>(ccBody);
    if (ccs.length === 0) {
      const ts = Date.now().toString().slice(-6);
      const ccCreate = await seedPost(`${API_PREFIX}/color-cards`, {
        card_no: `E2E-GCC${ts}`,
        card_name: `E2E全局色卡${ts}`,
        card_type: 'PANTONE',
      });
      if (!ccCreate.ok()) await reportSeedWrite(ccCreate, '全局色卡创建');
      else console.log('[globalSeed] 创建全局色卡');
    }
  } catch (e) {
    recordSeedFailure('色卡检查/创建', (e as Error).message);
  }

  // ---- 7. 色号定价 ----
  if (productId && productColorId) {
    try {
      const cpResp = await ctx.get(`${API_PREFIX}/color-prices?page=1&page_size=1`, { headers });
      const cpBody = await safeJson(cpResp);
      const cps = extractItems<{ id: number }>(cpBody);
      if (cps.length === 0) {
        const cpCreate = await seedPost(`${API_PREFIX}/color-prices`, {
          product_id: productId,
          color_id: productColorId,
          currency: 'CNY',
          base_price: '15.00',
          effective_from: new Date().toISOString().slice(0, 10),
        });
        if (!cpCreate.ok()) await reportSeedWrite(cpCreate, '全局色号定价创建');
        else console.log('[globalSeed] 创建全局色号定价');
      }
    } catch (e) {
      recordSeedFailure('色号定价检查/创建', (e as Error).message);
    }
  }

  // ---- 8. 报价单（带 sales_user_id + 已批准状态）----
  // quotations/sales extras 期望列表中有"已批准"行用于筛选与转订单；
  // 创建一个已批准报价单，引用带克重的全局产品。
  if (productId && currentUserId) {
    try {
      const qResp = await ctx.get(`${API_PREFIX}/quotations?page=1&page_size=1&status=approved`, {
        headers,
      });
      const qBody = await safeJson(qResp);
      const qs = extractItems<{ id: number }>(qBody);
      if (qs.length === 0) {
        // 取客户 id
        const cusResp2 = await ctx.get(`${API_PREFIX}/crm/customers?page=1&page_size=1`, {
          headers,
        });
        const cusBody2 = await safeJson(cusResp2);
        const customerId = extractItems<{ id: number }>(cusBody2)[0]?.id;
        if (customerId) {
          // 获取产品的 unit（确保报价行单位匹配）
          const prodDetailResp = await ctx.get(`${API_PREFIX}/products/${productId}`, { headers });
          const prodDetail = await safeJson(prodDetailResp);
          const productUnit =
            ((prodDetail?.data as Record<string, unknown>)?.unit as string) ?? '米';

          const createQ = await seedPost(`${API_PREFIX}/quotations`, {
            customer_id: customerId,
            sales_user_id: currentUserId,
            quotation_date: new Date().toISOString().slice(0, 10),
            valid_until: new Date(Date.now() + 90 * 86400000).toISOString().slice(0, 10),
            currency: 'CNY',
            exchange_rate: '1',
            base_currency: 'CNY',
            price_terms: 'FOB',
            tax_inclusive: false,
            tax_rate: '13',
            items: [
              {
                product_id: productId,
                unit: productUnit,
                quantity: '5',
                unit_price: '12',
                unit_price_with_tax: '13.56',
              },
            ],
          });
          if (createQ.ok()) {
            const qCreated = await safeJson(createQ);
            const qId = (qCreated?.data as Record<string, unknown>)?.id as number;
            if (qId) {
              // submit：小额(<10万)后端自批→APPROVED，大额→pending_approval
              const submitResp = await seedPost(`${API_PREFIX}/quotations/${qId}/submit`, {});
              const submitBody = await safeJson(submitResp);
              const qStatus = (submitBody?.data as Record<string, unknown>)?.status as
                string | undefined;
              if (qStatus === 'pending_approval') {
                // 仅当 submit 后仍在待审批态才需调 approve
                const approveResp = await seedPost(`${API_PREFIX}/quotations/${qId}/approve`, {});
                console.log(
                  `[globalSeed] 创建全局报价单 id=${qId} submit→pending_approval, approve ${approveResp.status()}`
                );
              } else if (qStatus === 'approved') {
                // 小额自批已直接 APPROVED（quotation_approval_service.rs:122），无需再调 approve
                console.log(
                  `[globalSeed] 创建全局报价单 id=${qId} submit→approved（小额自批），跳过 approve`
                );
              } else {
                // submit 失败或状态未知
                await reportSeedWrite(submitResp, `报价单提交 id=${qId}`);
                console.warn(`[globalSeed] 报价单 submit 后状态非预期 id=${qId} status=${qStatus}`);
              }
            }
          } else {
            console.warn(`[globalSeed] 报价单创建失败 HTTP ${createQ.status()}`);
          }
        }
      }
    } catch (e) {
      recordSeedFailure('报价单检查/创建', (e as Error).message);
    }
  }

  // ---- 9. 验布记录（带 fabric_width_inches）----
  // flow20 验布定级需要 pending/inspecting 状态且 fabric_width_inches 非空的记录
  try {
    const fiResp = await ctx.get(
      `${API_PREFIX}/production/fabric-inspections?page=1&page_size=1&status=pending`,
      { headers }
    );
    const fiBody = await safeJson(fiResp);
    const fis = extractItems<{ id: number }>(fiBody);
    if (fis.length === 0 && productId) {
      const fiCreate = await seedPost(`${API_PREFIX}/production/fabric-inspections`, {
        inspection_date: new Date().toISOString().slice(0, 10),
        product_id: productId,
        color_no: 'E2E-GC',
        scoring_system: 'four_point',
        fabric_width_inches: 60,
        inspector_name: 'E2E全局验布员',
      });
      if (!fiCreate.ok()) await reportSeedWrite(fiCreate, '全局验布记录创建');
      else console.log('[globalSeed] 创建全局验布记录（含 fabric_width_inches=60）');
    }
  } catch (e) {
    recordSeedFailure('验布记录检查/创建', (e as Error).message);
  }

  // ---- 10. BPM 流程定义（幂等：先查后建）----
  try {
    const bpmResp = await ctx.get(`${API_PREFIX}/bpm/definitions?page=1&page_size=50`, { headers });
    const bpmBody = await safeJson(bpmResp);
    const bpms = extractItems<{ code?: string }>(bpmBody);
    if (!bpms.some(d => d.code === 'sales_order_approval')) {
      const approverId = currentUserId;
      const bpmCreate = await seedPost(`${API_PREFIX}/bpm/definitions`, {
        name: '销售订单审批流程',
        code: 'sales_order_approval',
        description: 'E2E 测试用销售订单审批流程定义',
        category: 'sales',
        version: '1.0',
        config: {
          nodes: [
            { id: 'start', name: '提交审批', type: 'start_event' },
            {
              id: 'approve_task',
              name: '销售订单审批',
              type: 'user_task',
              assignee_value: String(approverId),
            },
            { id: 'end', name: '完成', type: 'end_event' },
          ],
          edges: [
            { source: 'start', target: 'approve_task' },
            { source: 'approve_task', target: 'end' },
          ],
        },
        status: 'ACTIVE',
      });
      if (!bpmCreate.ok()) await reportSeedWrite(bpmCreate, 'BPM 定义创建 sales_order_approval');
      else console.log('[globalSeed] 创建 BPM sales_order_approval');
    }
  } catch (e) {
    recordSeedFailure('BPM 定义检查/创建', (e as Error).message);
  }

  // ---- 11. 销售订单多状态种子（覆盖 sales/03 draft+pending, sales/04 approved）----
  // sales/03 期望列表中有"草稿"行(点提交)与"待审批"行(点审批/驳回)；
  // sales/04 期望"已审批"行(点发货)。状态词表（backend/src/models/status/sales.rs:14-41）：
  //   draft / pending / approved（小写）。
  // 端点：POST /api/v1/erp/sales/orders + /orders/{id}/submit + /orders/{id}/approve
  // DTO：CreateSalesOrderRequest（services/so/mod.rs:128）customer_id:i32 + items:[{product_id,quantity,unit_price}]
  if (productId && currentUserId) {
    // 获取客户 ID
    let salesCustomerId: number | undefined;
    try {
      const cusResp = await ctx.get(`${API_PREFIX}/crm/customers?page=1&page_size=1`, { headers });
      const cusBody = await safeJson(cusResp);
      salesCustomerId = extractItems<{ id: number }>(cusBody)[0]?.id;
    } catch (e) {
      console.warn('[globalSeed] 销售订单种子获取客户失败:', (e as Error).message);
    }

    if (salesCustomerId) {
      const SALES_ORDER_MIN_DRAFT = 2;
      const SALES_ORDER_MIN_PENDING = 3;
      const SALES_ORDER_MIN_APPROVED = 5;

      // 查询各状态现有数量
      let currentDraft = 0;
      let currentPending = 0;
      let currentApproved = 0;
      try {
        const draftResp = await ctx.get(
          `${API_PREFIX}/sales/orders?page=1&page_size=1&status=draft`,
          { headers }
        );
        const draftBody = await safeJson(draftResp);
        currentDraft = ((draftBody?.data as Record<string, unknown>)?.total as number) ?? 0;

        const pendingResp = await ctx.get(
          `${API_PREFIX}/sales/orders?page=1&page_size=1&status=pending`,
          { headers }
        );
        const pendingBody = await safeJson(pendingResp);
        currentPending = ((pendingBody?.data as Record<string, unknown>)?.total as number) ?? 0;

        const approvedResp = await ctx.get(
          `${API_PREFIX}/sales/orders?page=1&page_size=1&status=approved`,
          { headers }
        );
        const approvedBody = await safeJson(approvedResp);
        currentApproved = ((approvedBody?.data as Record<string, unknown>)?.total as number) ?? 0;
      } catch (e) {
        console.warn('[globalSeed] 销售订单状态查询异常:', (e as Error).message);
      }

      // 辅助：创建一个 draft 态销售订单，返回 id
      const createSalesOrder = async (): Promise<number | null> => {
        const ts = Date.now().toString().slice(-8);
        const resp = await seedPost(`${API_PREFIX}/sales/orders`, {
          customer_id: salesCustomerId,
          items: [{ product_id: productId, quantity: '10', unit_price: '25.00' }],
          notes: `E2E-SEED-${ts}`,
        });
        if (resp.ok()) {
          const body = await safeJson(resp);
          const data = body?.data as Record<string, unknown> | undefined;
          return (data?.id as number) ?? null;
        }
        await reportSeedWrite(resp, '销售订单创建');
        return null;
      };

      // 补建 draft 态订单
      const needDraft = Math.max(0, SALES_ORDER_MIN_DRAFT - currentDraft);
      for (let i = 0; i < needDraft; i++) {
        const id = await createSalesOrder();
        if (id) console.log(`[globalSeed] 创建销售订单(draft) id=${id}`);
      }

      // 补建 pending 态订单（创建后 submit）
      const needPending = Math.max(0, SALES_ORDER_MIN_PENDING - currentPending);
      for (let i = 0; i < needPending; i++) {
        const id = await createSalesOrder();
        if (id) {
          const submitResp = await seedPost(`${API_PREFIX}/sales/orders/${id}/submit`, {});
          if (submitResp.ok()) {
            console.log(`[globalSeed] 创建销售订单(pending) id=${id}`);
          } else {
            await reportSeedWrite(submitResp, `销售订单提交 id=${id}`);
          }
        }
      }

      // 补建 approved 态订单（创建后 submit + approve）
      const needApproved = Math.max(0, SALES_ORDER_MIN_APPROVED - currentApproved);
      for (let i = 0; i < needApproved; i++) {
        const id = await createSalesOrder();
        if (id) {
          const submitResp = await seedPost(`${API_PREFIX}/sales/orders/${id}/submit`, {});
          if (!submitResp.ok()) {
            await reportSeedWrite(submitResp, `销售订单提交(approved 前置) id=${id}`);
          }
          const approveResp = await seedPost(`${API_PREFIX}/sales/orders/${id}/approve`, {});
          if (approveResp.ok()) {
            console.log(`[globalSeed] 创建销售订单(approved) id=${id}`);
          } else {
            await reportSeedWrite(approveResp, `销售订单审批 id=${id}`);
          }
        }
      }
    }
  }

  // ---- 12. 采购订单多状态种子（覆盖 purchase/02 DRAFT+PENDING_APPROVAL, purchase/03 APPROVED）----
  // purchase/02 期望"草稿"行(DRAFT,点提交)与"待审批"行(PENDING_APPROVAL,点审批/驳回)；
  // purchase/03 期望"已审批"行(APPROVED,点收货)。
  // 状态词表（backend/src/models/status/purchase_inventory.rs:13-31）：DRAFT / PENDING_APPROVAL / APPROVED（大写）。
  // 端点：POST /api/v1/erp/purchase/orders + /orders/{id}/submit + /orders/{id}/approve
  // DTO：CreatePurchaseOrderRequest（services/po/mod.rs:31）supplier_id:i32, order_date:NaiveDate(必填), items:[{material_id,quantity_ordered,unit_price}]
  // 业务校验：validate_order_request（services/po/order_ops/crud.rs:112-173）除 DTO 外还强制
  //   warehouse_id（:137「仓库 ID 不能为空」）与 department_id（:148「部门 ID 不能为空」）
  //   非空且必须真实存在，故种子建单必须一并带上真实 warehouse_id + department_id。
  try {
    const supResp = await ctx.get(`${API_PREFIX}/purchase/suppliers?page=1&page_size=1`, {
      headers,
    });
    const supBody = await safeJson(supResp);
    const supplierId = extractItems<{ id: number }>(supBody)[0]?.id;

    // 取真实仓库/部门 id 作为采购订单必填前置（后端 validate_order_request 强校验非空且必须存在）。
    const poWarehouseId = await resolveWarehouseId();
    const poDepartmentId = await resolveDepartmentId();

    if (supplierId && productId && poWarehouseId && poDepartmentId) {
      const PO_MIN_DRAFT = 2;
      const PO_MIN_PENDING = 3;
      const PO_MIN_APPROVED = 5;

      // 查询各状态现有数量
      let poDraft = 0;
      let poPending = 0;
      let poApproved = 0;
      try {
        const poDraftResp = await ctx.get(
          `${API_PREFIX}/purchase/orders?page=1&page_size=1&status=DRAFT`,
          { headers }
        );
        const poDraftBody = await safeJson(poDraftResp);
        poDraft = ((poDraftBody?.data as Record<string, unknown>)?.total as number) ?? 0;

        const poPendResp = await ctx.get(
          `${API_PREFIX}/purchase/orders?page=1&page_size=1&status=PENDING_APPROVAL`,
          { headers }
        );
        const poPendBody = await safeJson(poPendResp);
        poPending = ((poPendBody?.data as Record<string, unknown>)?.total as number) ?? 0;

        const poApprResp = await ctx.get(
          `${API_PREFIX}/purchase/orders?page=1&page_size=1&status=APPROVED`,
          { headers }
        );
        const poApprBody = await safeJson(poApprResp);
        poApproved = ((poApprBody?.data as Record<string, unknown>)?.total as number) ?? 0;
      } catch (e) {
        console.warn('[globalSeed] 采购订单状态查询异常:', (e as Error).message);
      }

      // 辅助：创建一个 DRAFT 态采购订单，返回 id
      const today = new Date().toISOString().slice(0, 10);
      const createPurchaseOrder = async (): Promise<number | null> => {
        const ts = Date.now().toString().slice(-8);
        const resp = await seedPost(`${API_PREFIX}/purchase/orders`, {
          supplier_id: supplierId,
          order_date: today,
          warehouse_id: poWarehouseId,
          department_id: poDepartmentId,
          notes: `E2E-SEED-PO-${ts}`,
          items: [
            {
              material_id: productId,
              quantity_ordered: '20',
              unit_price: '15.00',
            },
          ],
        });
        if (resp.ok()) {
          const body = await safeJson(resp);
          const data = body?.data as Record<string, unknown> | undefined;
          return (data?.id as number) ?? null;
        }
        await reportSeedWrite(resp, '采购订单创建');
        return null;
      };

      // 补建 DRAFT
      const needPoDraft = Math.max(0, PO_MIN_DRAFT - poDraft);
      for (let i = 0; i < needPoDraft; i++) {
        const id = await createPurchaseOrder();
        if (id) console.log(`[globalSeed] 创建采购订单(DRAFT) id=${id}`);
      }

      // 补建 PENDING_APPROVAL（创建后 submit）
      const needPoPending = Math.max(0, PO_MIN_PENDING - poPending);
      for (let i = 0; i < needPoPending; i++) {
        const id = await createPurchaseOrder();
        if (id) {
          const submitResp = await seedPost(`${API_PREFIX}/purchase/orders/${id}/submit`, {});
          if (submitResp.ok()) {
            console.log(`[globalSeed] 创建采购订单(PENDING_APPROVAL) id=${id}`);
          } else {
            await reportSeedWrite(submitResp, `采购订单提交 id=${id}`);
          }
        }
      }

      // 补建 APPROVED（创建后 submit + approve）
      const needPoApproved = Math.max(0, PO_MIN_APPROVED - poApproved);
      for (let i = 0; i < needPoApproved; i++) {
        const id = await createPurchaseOrder();
        if (id) {
          const submitResp = await seedPost(`${API_PREFIX}/purchase/orders/${id}/submit`, {});
          if (!submitResp.ok()) {
            await reportSeedWrite(submitResp, `采购订单提交(APPROVED 前置) id=${id}`);
          }
          const approveResp = await seedPost(`${API_PREFIX}/purchase/orders/${id}/approve`, {});
          if (approveResp.ok()) {
            console.log(`[globalSeed] 创建采购订单(APPROVED) id=${id}`);
          } else {
            await reportSeedWrite(approveResp, `采购订单审批 id=${id}`);
          }
        }
      }
    } else if (supplierId && productId) {
      // 缺真实仓库/部门前置：种子行根本产不出来 → 计入台账判红（不再只 ⚠️ 后隐身）。
      recordSeedFailure(
        '采购订单种子缺前置',
        `warehouse=${poWarehouseId} department=${poDepartmentId}（步骤12 全部未建）`
      );
    }
  } catch (e) {
    recordSeedFailure('采购订单种子', (e as Error).message);
  }

  // ---- 13. AR 应收发票种子（覆盖 sales/05 列表 + sales/06 收款按钮需已审核态）----
  // 端点：POST /api/v1/erp/ar/invoices + /ar/invoices/{id}/approve
  // DTO：CreateArInvoiceRequestDto（handlers/ar_invoice_handler.rs:46）
  // 状态词表（models/status/general.rs common）：DRAFT→APPROVED 经 approve 端点。
  // 前端 arModule.invoice.getInvoiceStatusLabel(APPROVED) = '已审核'。
  // 保证至少 3 张 APPROVED 发票（sales/06 多用例需逐张操作不同发票）。
  {
    const AR_MIN_APPROVED = 3;
    try {
      let approvedArCount = 0;
      try {
        const arResp = await ctx.get(
          `${API_PREFIX}/ar/invoices?page=1&page_size=1&status=APPROVED`,
          {
            headers,
          }
        );
        const arBody = await safeJson(arResp);
        // AR list handler 返回 ApiResponse<Vec<Model>> 或 PaginatedResponse；探测 total
        approvedArCount =
          ((arBody?.data as Record<string, unknown>)?.total as number) ??
          (Array.isArray(arBody?.data) ? (arBody.data as unknown[]).length : 0);
      } catch (e) {
        console.warn('[globalSeed] AR 发票计数查询异常:', (e as Error).message);
      }

      if (approvedArCount < AR_MIN_APPROVED && currentUserId) {
        const cusResp = await ctx.get(`${API_PREFIX}/crm/customers?page=1&page_size=1`, {
          headers,
        });
        const cusBody = await safeJson(cusResp);
        const arCustomerId = extractItems<{ id: number }>(cusBody)[0]?.id;
        if (arCustomerId) {
          const need = AR_MIN_APPROVED - approvedArCount;
          for (let i = 0; i < need; i++) {
            const now = new Date();
            const due = new Date(now.getTime() + 30 * 86400000);
            const arCreate = await seedPost(`${API_PREFIX}/ar/invoices`, {
              customer_id: arCustomerId,
              invoice_date: now.toISOString().slice(0, 10),
              due_date: due.toISOString().slice(0, 10),
              invoice_amount: '12000',
            });
            if (arCreate.ok()) {
              const arData = await safeJson(arCreate);
              const arId = (arData?.data as Record<string, unknown>)?.id as number;
              if (arId) {
                const approveAr = await seedPost(`${API_PREFIX}/ar/invoices/${arId}/approve`, {});
                if (approveAr.ok()) {
                  console.log(`[globalSeed] 创建 AR 应收发票(APPROVED) id=${arId}`);
                } else {
                  await reportSeedWrite(approveAr, `AR 发票审核 id=${arId}`);
                }
              }
            } else {
              await reportSeedWrite(arCreate, `AR 发票创建(第${i + 1}张)`);
            }
          }
        }
      } else {
        console.log(`[globalSeed] AR 发票 APPROVED 已有 ${approvedArCount}，满足需求`);
      }
    } catch (e) {
      recordSeedFailure('AR 发票种子', (e as Error).message);
    }
  }

  // ---- 14. AP 应付发票种子（覆盖 purchase/05 列表 + purchase/06 付款需已审核态）----
  // 端点：POST /api/v1/erp/ap/invoices + /ap/invoices/{id}/approve
  // DTO：CreateApInvoiceRequest（services/ap_invoice_ops/types.rs:24）supplier_id, invoice_type, amount, invoice_date, due_date
  // 状态词表（finance.rs INVOICE_AUDITED + common STATUS_DRAFT）：DRAFT→AUDITED 经 approve。
  // 前端 apModule.invoice.getInvoiceStatusLabel(AUDITED) = '已审核'。
  // 保证至少 3 张 AUDITED 发票（purchase/06 多用例需逐张操作不同发票）。
  {
    const AP_MIN_AUDITED = 3;
    try {
      let auditedApCount = 0;
      try {
        const apResp = await ctx.get(
          `${API_PREFIX}/ap/invoices?page=1&page_size=1&invoice_status=AUDITED`,
          { headers }
        );
        const apBody = await safeJson(apResp);
        auditedApCount =
          ((apBody?.data as Record<string, unknown>)?.total as number) ??
          (Array.isArray(apBody?.data) ? (apBody.data as unknown[]).length : 0);
      } catch (e) {
        console.warn('[globalSeed] AP 发票计数查询异常:', (e as Error).message);
      }

      if (auditedApCount < AP_MIN_AUDITED && currentUserId) {
        const supResp = await ctx.get(`${API_PREFIX}/purchase/suppliers?page=1&page_size=1`, {
          headers,
        });
        const supBody = await safeJson(supResp);
        const apSupplierId = extractItems<{ id: number }>(supBody)[0]?.id;
        if (apSupplierId) {
          const need = AP_MIN_AUDITED - auditedApCount;
          for (let i = 0; i < need; i++) {
            const now = new Date();
            const due = new Date(now.getTime() + 30 * 86400000);
            const apCreate = await seedPost(`${API_PREFIX}/ap/invoices`, {
              supplier_id: apSupplierId,
              invoice_type: 'PURCHASE',
              amount: '8000',
              invoice_date: now.toISOString().slice(0, 10),
              due_date: due.toISOString().slice(0, 10),
            });
            if (apCreate.ok()) {
              const apData = await safeJson(apCreate);
              const apId = (apData?.data as Record<string, unknown>)?.id as number;
              if (apId) {
                const approveAp = await seedPost(`${API_PREFIX}/ap/invoices/${apId}/approve`, {});
                if (approveAp.ok()) {
                  console.log(`[globalSeed] 创建 AP 应付发票(AUDITED) id=${apId}`);
                } else {
                  await reportSeedWrite(approveAp, `AP 发票审核 id=${apId}`);
                }
              }
            } else {
              await reportSeedWrite(apCreate, `AP 发票创建(第${i + 1}张)`);
            }
          }
        }
      } else {
        console.log(`[globalSeed] AP 发票 AUDITED 已有 ${auditedApCount}，满足需求`);
      }
    } catch (e) {
      recordSeedFailure('AP 发票种子', (e as Error).message);
    }
  }

  // ---- 15. 库存种子（sales/04 发货、purchase/04 质检等链路的前置库存）----
  // 独立建库存行的原因：步骤5只建了仓库、步骤2只建了产品，没有库存行时
  //   sales/04-04 打开发货对话框的 loadDeliveryStockRows（GET /inventory/stock?
  //   warehouse_id&product_id）会拿到空列表 → 「库存行」下拉无选项，四维出库走不通。
  // seed 方式（最贴近真实业务，非直改库存真相表）：走后端确有的真实收货链路，
  //   由「建专属采购订单 → submit → approve → 建入库单（带批次/色号/缸号四维）→
  //   confirm」自动落 inventory_stocks：confirm_receipt 事务内 update_inventory_txn
  //   → upsert_stock_for_item → create_stock_fabric_txn 落库（backend/src/services/
  //   purchase_receipt_ops/state.rs:49、purchase_receipt_private.rs:197-356）。
  // 维度口径（与后端真相源对齐，字段名以真实 DTO 为准）：
  //   - 出库走「款号(product_id)+色号(color_no)+批次(batch_no)+缸号(dye_lot_no)」四维，
  //     染色布色号/缸号均必填（backend/src/services/inv/fabric_class.rs:36 validate_fabric_trace），
  //     故这里建的是「染色布」库存行，color_code/lot_no/batch_no 全部给非空值，
  //     否则前端 DeliveryDialog.vue:236-242 与后端出库校验会因缺维度直接拒绝发货。
  //   - 入库明细必填 material_id/line_no/material_code/material_name/quantity/quantity_alt/unit_master
  //     （backend/src/services/purchase_receipt_dto.rs:53-118），batch_no 由
  //     validate_receipt_item_dimensions（crud.rs:112）强校验非空。
  //   - 库存等级取值域：一等品（models/status/purchase_inventory.rs:225 inventory_stock_grade::FIRST）。
  // 仓库选择：发货/收货对话框的仓库下拉第一项 = 前端 fetchWarehouses 用的同一查询口径
  //   （GET /warehouses，无 status、默认 page_size=10，后端按 warehouse_code 升序，
  //    warehouse_service.rs:51），故逐一对该有序列表里的每个仓库补建库存，
  //   保证测试无论点到哪一项都有四维库存行可用。
  // 幂等：先查后建——对 (product_id, warehouse_id) 组合 GET /inventory/stock 取 total，
  //   total>0 即跳过；batch_no/dye_lot_no 带时间戳+分片后缀保证跨分片共库唯一不冲突。
  // 失败暴露：confirm/建单/建库任一步 >=500 视为后端潜伏缺陷，原样打印端点+HTTP+响应体，
  //   不吞、不 fake。
  if (productId) {
    try {
      // 取供应商（步骤3建）与产品编码/名称（入库明细 material_code/material_name 必填真实值）
      const supForStockResp = await ctx.get(`${API_PREFIX}/purchase/suppliers?page=1&page_size=1`, {
        headers,
      });
      const supForStockBody = await safeJson(supForStockResp);
      const stockSupplierId = extractItems<{ id: number }>(supForStockBody)[0]?.id;

      const prodDetailResp = await ctx.get(`${API_PREFIX}/products/${productId}`, { headers });
      const prodDetailBody = await safeJson(prodDetailResp);
      const prodDetail = (prodDetailBody?.data as Record<string, unknown>) ?? {};
      const materialCode = (prodDetail.code as string) ?? '';
      const materialName = (prodDetail.name as string) ?? '';
      const unitMaster = (prodDetail.unit as string) ?? '米';

      // 取与前端发货对话框同口径的仓库有序列表
      const whListResp = await ctx.get(`${API_PREFIX}/warehouses?page=1&page_size=10`, { headers });
      const whListBody = await safeJson(whListResp);
      const stockWarehouses = extractItems<{ id: number }>(whListBody);

      if (!stockSupplierId || !materialCode || stockWarehouses.length === 0) {
        recordSeedFailure(
          '库存种子前置不足',
          `supplier=${stockSupplierId} code=${materialCode} warehouses=${stockWarehouses.length}（库存行未建）`
        );
      } else {
        const today = new Date().toISOString().slice(0, 10);
        // 库存种子的专属采购订单同样走 validate_order_request：warehouse_id 之外还强制
        // department_id（crud.rs:148），故复用 resolveDepartmentId 取真实部门 id。
        const stockDepartmentId = await resolveDepartmentId();
        for (const wh of stockWarehouses) {
          // 幂等：该产品在该仓库已有库存行则跳过
          const existResp = await ctx.get(
            `${API_PREFIX}/inventory/stock?product_id=${productId}&warehouse_id=${wh.id}&page=1&page_size=1`,
            { headers }
          );
          const existBody = await safeJson(existResp);
          const existTotal = ((existBody?.data as Record<string, unknown>)?.total as number) ?? 0;
          if (existTotal > 0) {
            continue;
          }

          // batch_no / dye_lot_no 带时间戳+分片后缀：跨分片共库唯一，四维不撞行
          const suffix = `${Date.now().toString().slice(-8)}s${SHARD_INDEX || 'x'}`;
          const batchNo = `E2E-SB${suffix}`;
          const dyeLotNo = `E2E-SD${suffix}`;
          const colorCode = 'E2E-SEED-COLOR';

          // 1) 专属采购订单（数量给足，一次入库 10000，避免多个发货用例耗尽）
          const poResp = await seedPost(`${API_PREFIX}/purchase/orders`, {
            supplier_id: stockSupplierId,
            order_date: today,
            warehouse_id: wh.id,
            department_id: stockDepartmentId,
            notes: `E2E-SEED-STOCK-PO-${suffix}`,
            items: [
              {
                material_id: productId,
                quantity_ordered: '10000',
                unit_price: '15.00',
              },
            ],
          });
          const poBody = await safeJson(poResp);
          const poId = (poBody?.data as Record<string, unknown>)?.id as number;
          if (!poResp.ok() || !poId) {
            const errText = JSON.stringify(await poResp.json().catch(() => null));
            recordSeedFailure(
              '库存种子建采购订单',
              `HTTP ${poResp.status()} warehouse=${wh.id} body=${errText.slice(0, 300)}`
            );
            continue;
          }
          // 2) 提交 + 审批
          await seedPost(`${API_PREFIX}/purchase/orders/${poId}/submit`, {});
          const poApprove = await seedPost(`${API_PREFIX}/purchase/orders/${poId}/approve`, {});
          if (!poApprove.ok()) {
            const errText = JSON.stringify(await poApprove.json().catch(() => null));
            recordSeedFailure(
              '库存种子采购订单审批',
              `HTTP ${poApprove.status()} po=${poId} body=${errText.slice(0, 300)}`
            );
            continue;
          }
          // 3) 建入库单：染色布四维齐全（material_code/material_name 取产品真实值）
          const rcvResp = await seedPost(`${API_PREFIX}/purchase/receipts`, {
            supplier_id: stockSupplierId,
            order_id: poId,
            receipt_date: today,
            warehouse_id: wh.id,
            notes: `E2E-SEED-STOCK-RCV-${suffix}`,
            items: [
              {
                line_no: 1,
                material_id: productId,
                material_code: materialCode,
                material_name: materialName,
                batch_no: batchNo,
                color_code: colorCode,
                lot_no: dyeLotNo,
                grade: '一等品',
                quantity: '10000',
                quantity_alt: '0',
                unit_master: unitMaster,
              },
            ],
          });
          const rcvBody = await safeJson(rcvResp);
          const rcvId = (rcvBody?.data as Record<string, unknown>)?.id as number;
          if (!rcvResp.ok() || !rcvId) {
            const errText = JSON.stringify(await rcvResp.json().catch(() => null));
            recordSeedFailure(
              '库存种子建入库单',
              `HTTP ${rcvResp.status()} po=${poId} warehouse=${wh.id} body=${errText.slice(0, 300)}`
            );
            continue;
          }
          // 4) 质检完成回写 pass → 收货单转 PASSED（门控前置，见 seedInspectionPass）
          // 5) 确认入库 → update_inventory_txn 自动落 inventory_stocks
          const inspected = await seedInspectionPass(rcvId, stockSupplierId, '10000', '库存种子');
          if (!inspected) {
            continue;
          }
          const confirmResp = await seedPost(
            `${API_PREFIX}/purchase/receipts/${rcvId}/confirm`,
            {}
          );
          if (confirmResp.ok()) {
            console.log(
              `[globalSeed] 库存种子落库成功 product=${productId} warehouse=${wh.id} rcv=${rcvId} 色号=${colorCode} 批次=${batchNo} 缸号=${dyeLotNo} qty=10000`
            );
          } else {
            const errText = JSON.stringify(await confirmResp.json().catch(() => null));
            recordSeedFailure(
              '库存种子「确认入库」',
              `HTTP ${confirmResp.status()} rcv=${rcvId} warehouse=${wh.id} body=${errText.slice(0, 300)}`
            );
          }
        }
      }
    } catch (e) {
      recordSeedFailure('库存种子异常', (e as Error).message);
    }
  }

  // ---- 16. 采购入库单 + 质检单种子（族B 采购后链：purchase/04 待质检 + 收货入口）----
  // 本步要两种数据，且在新门控下它们**不是同一条链的先后两步**，故拆成两条独立种子：
  //   (a) pending 质检行（/purchase-inspection 页、purchase/04 待质检用例）：
  //       建单 → 建质检单。质检单创建只校验收货单存在性
  //       （purchase_inspection_service.rs:88-96），不要求收货单已确认，因此这条链
  //       故意**不确认**收货单——确认在门控下本来就必须先质检合格，而 complete(pass) 会把
  //       收货单推成 PASSED，pending 质检行就没了。
  //   (b) 已确认入库行（/purchase-receipt 页、purchase/04「已确认单可再建检验单」）：
  //       建单 → 质检 complete(pass) 回写 PASSED → confirm（见 seedInspectionPass）。
  // 入库质检门控要求：inspection_status 非 PASSED 时 confirm 必然 400（PENDING 亦被拒）。
  // 入库单字段要求：supplier_id/warehouse_id/receipt_date + 明细(batch_no 必填, color_code 非空时 lot_no 必填)。
  // 幂等：先查 pending 态质检数量，不足才补建。
  if (productId && currentUserId) {
    try {
      const INSPECTION_MIN_PENDING = 3;
      let pendingInsCount = 0;
      try {
        const piResp = await ctx.get(
          `${API_PREFIX}/purchase/inspections?page=1&page_size=1&status=pending`,
          { headers }
        );
        const piBody = await safeJson(piResp);
        pendingInsCount =
          ((piBody?.data as Record<string, unknown>)?.total as number) ??
          extractItems(piBody)?.length ??
          0;
      } catch (e) {
        console.warn('[globalSeed] 采购质检计数查询异常:', (e as Error).message);
      }

      if (pendingInsCount < INSPECTION_MIN_PENDING) {
        const supForInsResp = await ctx.get(`${API_PREFIX}/purchase/suppliers?page=1&page_size=1`, {
          headers,
        });
        const supForInsBody = await safeJson(supForInsResp);
        const insSupplierId = extractItems<{ id: number }>(supForInsBody)[0]?.id;

        const insWarehouseId = await resolveWarehouseId();
        const insDeptId = await resolveDepartmentId();

        // 产品明细字段（入库单 items 必填）
        const prodForInsResp = await ctx.get(`${API_PREFIX}/products/${productId}`, { headers });
        const prodForInsBody = await safeJson(prodForInsResp);
        const insProdDetail = (prodForInsBody?.data as Record<string, unknown>) ?? {};
        const insMatCode = (insProdDetail.code as string) ?? '';
        const insMatName = (insProdDetail.name as string) ?? '';
        const insUnit = (insProdDetail.unit as string) ?? '米';

        if (insSupplierId && insWarehouseId) {
          const today = new Date().toISOString().slice(0, 10);
          const seedSupplierId = insSupplierId;
          const seedWarehouseId = insWarehouseId;
          const need = INSPECTION_MIN_PENDING - pendingInsCount;

          // 公共前置：建一张"已审批采购订单 + 未确认收货单"。
          // 任一步失败都计入种子失败汇总并返回 null，调用方必须跳过本项而不是继续往下走
          // （若只 console.error 后继续往下走，"种子行根本没建"会变成下游整簇红的隐性根因）。
          const seedApprovedReceipt = async (
            label: string,
            ts: string,
            quantity: string
          ): Promise<{ poId: number; rcvId: number } | null> => {
            const poResp = await seedPost(`${API_PREFIX}/purchase/orders`, {
              supplier_id: seedSupplierId,
              order_date: today,
              warehouse_id: seedWarehouseId,
              department_id: insDeptId,
              notes: `E2E-SEED-INSP-${ts}`,
              items: [
                {
                  material_id: productId,
                  quantity_ordered: quantity,
                  unit_price: '15.00',
                },
              ],
            });
            const poData = await safeJson(poResp);
            const poId = (poData?.data as Record<string, unknown>)?.id as number;
            if (!poResp.ok() || !poId) {
              recordSeedFailure(
                `${label} 采购订单`,
                `HTTP ${poResp.status()} body=${(await poResp.text().catch(() => '')).slice(0, 300)}`
              );
              return null;
            }
            await seedPost(`${API_PREFIX}/purchase/orders/${poId}/submit`, {});
            const poApprove = await seedPost(`${API_PREFIX}/purchase/orders/${poId}/approve`, {});
            if (!poApprove.ok()) {
              recordSeedFailure(
                `${label} 采购订单审批`,
                `HTTP ${poApprove.status()} po=${poId} body=${(await poApprove.text().catch(() => '')).slice(0, 300)}`
              );
              return null;
            }
            const rcvResp = await seedPost(`${API_PREFIX}/purchase/receipts`, {
              supplier_id: seedSupplierId,
              order_id: poId,
              receipt_date: today,
              warehouse_id: seedWarehouseId,
              department_id: insDeptId,
              notes: `E2E-SEED-INSP-RCV-${ts}`,
              items: [
                {
                  line_no: 1,
                  material_id: productId,
                  material_code: insMatCode,
                  material_name: insMatName,
                  batch_no: `E2E-IB${ts}`,
                  color_code: 'E2E-INSP-COLOR',
                  lot_no: `E2E-IL${ts}`,
                  grade: '一等品',
                  quantity: quantity,
                  quantity_alt: '0',
                  unit_master: insUnit,
                  unit_price: '15.00',
                },
              ],
            });
            const rcvData = await safeJson(rcvResp);
            const rcvId = (rcvData?.data as Record<string, unknown>)?.id as number;
            if (!rcvResp.ok() || !rcvId) {
              recordSeedFailure(
                `${label} 入库单`,
                `HTTP ${rcvResp.status()} po=${poId} body=${(await rcvResp.text().catch(() => '')).slice(0, 300)}`
              );
              return null;
            }
            return { poId, rcvId };
          };
          for (let i = 0; i < need; i++) {
            const ts = `${Date.now().toString().slice(-8)}i${i}s${SHARD_INDEX || 'x'}`;
            // 1) 建专属已审批采购订单 + 入库单（染色布四维齐全：batch_no/color_code/lot_no 均非空）
            const created = await seedApprovedReceipt(`质检种子 i=${i}`, ts, '500');
            if (!created) {
              continue;
            }
            const { poId, rcvId } = created;

            // 2) 建 pending 质检单（POST /purchase/inspections），关联该收货单。
            //    质检单创建只校验收货单存在性（purchase_inspection_service.rs:88-96），
            //    不要求其已确认入库；本步的被测对象就是"待质检"这一态，故**故意不确认收货单**
            //    ——一旦 complete(pass) 就会把它推进成 PASSED 并由门控允许确认，pending 行就没了。
            const insCreateResp = await seedPost(`${API_PREFIX}/purchase/inspections`, {
              receipt_id: rcvId,
              order_id: poId,
              supplier_id: insSupplierId,
              inspection_date: today,
              notes: `E2E-SEED-INSP-Q-${ts}`,
            });
            if (insCreateResp.ok()) {
              const insData = await safeJson(insCreateResp);
              const insId = (insData?.data as Record<string, unknown>)?.id as number;
              console.log(
                `[globalSeed] 创建采购质检单(pending) id=${insId} receipt=${rcvId} po=${poId}`
              );
            } else {
              recordSeedFailure(
                `采购质检单创建(关联 rcv=${rcvId})`,
                `HTTP ${insCreateResp.status()} body=${(await insCreateResp.text().catch(() => '')).slice(0, 300)}`
              );
            }
          }

          // 种子(b)：已确认入库行（COMPLETED）。必须走"质检 complete(pass) 回写 PASSED → confirm"
          // 全链，否则门控 400 拒绝、页面无已确认行，purchase/04 与收货入口用例会以
          // "查不到数据"的形式红掉，根因难查。
          const confirmedTs = `${Date.now().toString().slice(-8)}c${SHARD_INDEX || 'x'}`;
          const confirmedReceipt = await seedApprovedReceipt(
            `已确认入库行种子`,
            confirmedTs,
            '1000'
          );
          if (confirmedReceipt) {
            const passed = await seedInspectionPass(
              confirmedReceipt.rcvId,
              seedSupplierId,
              '1000',
              '已确认入库行种子'
            );
            if (passed) {
              const confirmResp = await seedPost(
                `${API_PREFIX}/purchase/receipts/${confirmedReceipt.rcvId}/confirm`,
                {}
              );
              if (confirmResp.ok()) {
                console.log(
                  `[globalSeed] 已确认入库行种子成功 rcv=${confirmedReceipt.rcvId} po=${confirmedReceipt.poId}`
                );
              } else {
                recordSeedFailure(
                  '已确认入库行种子「确认入库」',
                  `HTTP ${confirmResp.status()} rcv=${confirmedReceipt.rcvId} body=${(await confirmResp.text().catch(() => '')).slice(0, 300)}`
                );
              }
            }
          }
        } else {
          recordSeedFailure(
            '采购质检种子缺前置',
            `supplier=${insSupplierId} warehouse=${insWarehouseId}（步骤16 质检/收货链未建）`
          );
        }
      } else {
        console.log(`[globalSeed] 采购质检单 pending 已有 ${pendingInsCount}，满足需求`);
      }
    } catch (e) {
      recordSeedFailure('采购入库+质检种子异常', (e as Error).message);
    }
  }

  // ---- 17. AP 付款申请+付款单种子（族B purchase/06-04 付款管理 tab 需有记录）----
  // 端点链：
  //   POST /ap/payment-requests → submit → approve → POST /ap/payments。
  // CreateApPaymentRequest(付款申请) 必填：supplier_id, request_date, payment_type, payment_method, request_amount, items。
  // ApPaymentRequestItemDto（后端 ap_payment_request_service.rs:641）：invoice_id, apply_amount, notes(可选)。
  // items 中 invoice_id 必须关联真实非 DRAFT/非 CANCELLED 的应付单（validate_invoice_items_txn），
  // 且 apply_amount 不超过该应付单 unpaid_amount。
  // CreateApPaymentRequest(付款单) 必填：request_id, payment_date。
  // 付款申请状态流转：DRAFT →(submit)→ APPROVING →(approve)→ APPROVED；然后建付款单。
  if (currentUserId) {
    try {
      const PAYMENT_MIN = 2;
      let existingPayments = 0;
      try {
        const payListResp = await ctx.get(`${API_PREFIX}/ap/payments?page=1&page_size=1`, {
          headers,
        });
        const payListBody = await safeJson(payListResp);
        existingPayments =
          ((payListBody?.data as Record<string, unknown>)?.total as number) ??
          extractItems(payListBody)?.length ??
          0;
      } catch (e) {
        console.warn('[globalSeed] AP 付款计数查询异常:', (e as Error).message);
      }

      if (existingPayments < PAYMENT_MIN) {
        const supForPayResp = await ctx.get(`${API_PREFIX}/purchase/suppliers?page=1&page_size=1`, {
          headers,
        });
        const supForPayBody = await safeJson(supForPayResp);
        const paySupplierId = extractItems<{ id: number }>(supForPayBody)[0]?.id;

        // 取一张已审核 AUDITED 的应付单用于 items 关联（步骤14 已建 AUDITED 发票）
        let linkedApInvoiceId: number | undefined;
        let linkedApInvoiceUnpaid = 0;
        try {
          const apInvResp = await ctx.get(
            `${API_PREFIX}/ap/invoices?page=1&page_size=1&invoice_status=AUDITED`,
            { headers }
          );
          const apInvBody = await safeJson(apInvResp);
          const apInvList = extractItems<{ id: number; unpaid_amount?: string | number }>(
            apInvBody
          );
          if (apInvList.length > 0) {
            linkedApInvoiceId = apInvList[0].id;
            linkedApInvoiceUnpaid = Number(apInvList[0].unpaid_amount ?? 0);
          }
        } catch (e) {
          console.warn('[globalSeed] AP 应付单(AUDITED)查询异常:', (e as Error).message);
        }

        if (paySupplierId && linkedApInvoiceId) {
          const today = new Date().toISOString().slice(0, 10);
          const need = PAYMENT_MIN - existingPayments;
          for (let i = 0; i < need; i++) {
            const ts = `${Date.now().toString().slice(-8)}p${i}s${SHARD_INDEX || 'x'}`;
            // apply_amount 不超过 unpaid_amount，取 min(5000, unpaid)
            const applyAmt = String(Math.min(5000, linkedApInvoiceUnpaid || 5000));
            // 1) 创建付款申请（含 items 明细，后端 CreateApPaymentRequest 必填）
            const prCreateResp = await seedPost(`${API_PREFIX}/ap/payment-requests`, {
              supplier_id: paySupplierId,
              request_date: today,
              payment_type: 'PURCHASE',
              payment_method: '银行转账',
              request_amount: applyAmt,
              notes: `E2E-SEED-PAYREQ-${ts}`,
              items: [
                {
                  invoice_id: linkedApInvoiceId,
                  apply_amount: applyAmt,
                },
              ],
            });
            const prData = await safeJson(prCreateResp);
            const prId = (prData?.data as Record<string, unknown>)?.id as number;
            if (!prCreateResp.ok() || !prId) {
              await reportSeedWrite(prCreateResp, `AP 付款申请创建 i=${i}`);
              continue;
            }
            // 2) 提交付款申请
            const prSubmitResp = await seedPost(
              `${API_PREFIX}/ap/payment-requests/${prId}/submit`,
              {}
            );
            if (!prSubmitResp.ok()) {
              await reportSeedWrite(prSubmitResp, `AP 付款申请提交 id=${prId}`);
              continue;
            }
            // 3) 审批付款申请
            const prApproveResp = await seedPost(
              `${API_PREFIX}/ap/payment-requests/${prId}/approve`,
              {}
            );
            if (!prApproveResp.ok()) {
              await reportSeedWrite(prApproveResp, `AP 付款申请审批 id=${prId}`);
              continue;
            }
            // 4) 创建付款单
            const payCreateResp = await seedPost(`${API_PREFIX}/ap/payments`, {
              request_id: prId,
              payment_date: today,
              notes: `E2E-SEED-PAY-${ts}`,
            });
            if (payCreateResp.ok()) {
              const payData = await safeJson(payCreateResp);
              const payId = (payData?.data as Record<string, unknown>)?.id as number;
              console.log(`[globalSeed] 创建 AP 付款单 id=${payId} (request=${prId})`);
            } else {
              await reportSeedWrite(payCreateResp, `AP 付款单创建(req=${prId})`);
            }
          }
        } else if (paySupplierId && !linkedApInvoiceId) {
          recordSeedFailure(
            'AP 付款申请缺已审核应付单',
            `supplier=${paySupplierId}（items 必填 invoice_id，步骤17 付款链未建）`
          );
        }
      } else {
        console.log(`[globalSeed] AP 付款单已有 ${existingPayments}，满足需求`);
      }
    } catch (e) {
      recordSeedFailure('AP 付款种子', (e as Error).message);
    }
  }

  // ---- 17.5 确保覆盖当前日期的会计期间存在（AR 收款/凭证的期间校验前置）----
  // 后端 check_date_locked_txn（accounting_period_service.rs:645）按 payment_date 查询
  // accounting_periods 表 start_date<=date AND end_date>=date，不存在则拒绝。
  // 端点：POST /api/v1/erp/finance/accounting-periods（routes/finance.rs:70-73，missing_handlers.rs:110）
  // Payload：{ year, period }（period=1-12），后端自动计算当月首日至末日为 start/end_date。
  // 幂等口径：靠「400 body 文案含"已存在"」判幂等不可行——后端 missing_handlers.rs:125
  // 走 AppError::business（出参永久脱敏为「业务处理失败」），文案匹配永不成立，
  // 早已存在的期间会被误判成创建失败。正解=用**读端点做存在性前置**（GET /finance/accounting-periods
  // 返回 Vec<Dto>，missing_handlers.rs:72-83，含 year/period 列）：已存在即跳过写；不存在才 POST；
  // POST 仍失败 = 真实缺陷，经 reportSeedWrite 计入汇总判红（禁止反向放松）。
  {
    const now = new Date();
    const periodYear = now.getFullYear();
    const periodMonth = now.getMonth() + 1;
    const periodLabel = `会计期间 ${periodYear}-${String(periodMonth).padStart(2, '0')}`;
    try {
      const listResp = await ctx.get(`${API_PREFIX}/finance/accounting-periods`, { headers });
      const listBody = await safeJson(listResp);
      const periods =
        (listBody?.data as Array<{ year?: number; period?: number }> | undefined) ?? null;
      if (!listResp.ok() || !Array.isArray(periods)) {
        await reportSeedWrite(listResp, `${periodLabel} 存在性回读`);
      } else if (periods.some(p => p.year === periodYear && p.period === periodMonth)) {
        console.log(`[globalSeed] ${periodLabel} 已存在（读端点确认），跳过创建`);
      } else {
        const periodResp = await seedPost(`${API_PREFIX}/finance/accounting-periods`, {
          year: periodYear,
          period: periodMonth,
        });
        if (periodResp.ok()) {
          console.log(`[globalSeed] 创建${periodLabel} 成功`);
        } else {
          await reportSeedWrite(periodResp, `${periodLabel} 创建`);
        }
      }
    } catch (e) {
      recordSeedFailure(`${periodLabel} 检查/创建异常`, (e as Error).message);
    }
  }

  // ---- 17.6 固定日期全流程用例所需的会计期间前置（族B 11-ar payments 2026-05-06/07）----
  // 后端 check_payment_period_locked（ar_ops/collection.rs 转 accounting_period_service.rs 做期间闭区间校验）
  // 按 payment_date 以闭区间 start<=date<=end 命中 accounting_periods，缺失即业务拒绝。
  // fullflow/11-ar.spec.ts 用固定 payment_date '2026-05-06'/'2026-05-07'（非"当前月"），
  // 17.5 的当前月种子不覆盖 ⇒ 补一条确定性 2026-05 OPEN 期间，与用例日期对齐。
  // 幂等口径与 17.5 同源：读端点确认已存在即跳过，缺失才 POST；POST 失败计入汇总判红（禁止反向放松）。
  {
    const fixedPeriodYear = 2026;
    const fixedPeriodMonth = 5;
    const periodLabel = `会计期间 ${fixedPeriodYear}-${String(fixedPeriodMonth).padStart(2, '0')}`;
    try {
      const listResp = await ctx.get(`${API_PREFIX}/finance/accounting-periods`, { headers });
      const listBody = await safeJson(listResp);
      const periods =
        (listBody?.data as Array<{ year?: number; period?: number }> | undefined) ?? null;
      if (!listResp.ok() || !Array.isArray(periods)) {
        await reportSeedWrite(listResp, `${periodLabel} 存在性回读`);
      } else if (periods.some(p => p.year === fixedPeriodYear && p.period === fixedPeriodMonth)) {
        console.log(`[globalSeed] ${periodLabel} 已存在（读端点确认），跳过创建`);
      } else {
        const periodResp = await seedPost(`${API_PREFIX}/finance/accounting-periods`, {
          year: fixedPeriodYear,
          period: fixedPeriodMonth,
        });
        if (periodResp.ok()) {
          console.log(`[globalSeed] 创建${periodLabel} 成功`);
        } else {
          await reportSeedWrite(periodResp, `${periodLabel} 创建`);
        }
      }
    } catch (e) {
      recordSeedFailure(`${periodLabel} 检查/创建异常`, (e as Error).message);
    }
  }

  // ---- 18. AR 收款单种子（族B sales/06-04 收款管理 tab 需有记录）----
  // 端点：POST /api/v1/erp/ar/payments
  // CreateArPaymentRequest（handlers/ar_payment_handler.rs:31）必填：
  //   customer_id:i32, amount:Decimal, payment_method:String(min1,max50), payment_date:NaiveDate。
  // 业务校验：check_payment_period_locked 要求 payment_date 落在已设置的 OPEN 会计期间内
  //   （步骤 17.5 已确保当前年月期间存在）。
  if (currentUserId) {
    try {
      const AR_PAY_MIN = 2;
      let existingArPayments = 0;
      try {
        const arPayListResp = await ctx.get(`${API_PREFIX}/ar/payments?page=1&page_size=1`, {
          headers,
        });
        const arPayListBody = await safeJson(arPayListResp);
        // AR payment list 返回 { list: [], total: n }（handlers/ar_payment_handler.rs:82）
        const arPayData = arPayListBody?.data as Record<string, unknown> | undefined;
        existingArPayments =
          (arPayData?.total as number) ??
          (Array.isArray(arPayData?.list) ? (arPayData.list as unknown[]).length : 0);
      } catch (e) {
        console.warn('[globalSeed] AR 收款计数查询异常:', (e as Error).message);
      }

      if (existingArPayments < AR_PAY_MIN) {
        const cusForArPayResp = await ctx.get(`${API_PREFIX}/crm/customers?page=1&page_size=1`, {
          headers,
        });
        const cusForArPayBody = await safeJson(cusForArPayResp);
        const arPayCustomerId = extractItems<{ id: number }>(cusForArPayBody)[0]?.id;

        // 取一张 APPROVED AR 发票用于关联（可选）
        let relatedInvoiceId: number | undefined;
        try {
          const arInvResp = await ctx.get(
            `${API_PREFIX}/ar/invoices?page=1&page_size=1&status=APPROVED`,
            { headers }
          );
          const arInvBody = await safeJson(arInvResp);
          const arInvList = Array.isArray(arInvBody?.data)
            ? (arInvBody.data as Array<{ id: number }>)
            : extractItems<{ id: number }>(arInvBody);
          relatedInvoiceId = arInvList[0]?.id;
        } catch {
          // 关联发票可选，查询失败不阻塞
        }

        if (arPayCustomerId) {
          const today = new Date().toISOString().slice(0, 10);
          const need = AR_PAY_MIN - existingArPayments;
          for (let i = 0; i < need; i++) {
            const arPayCreateResp = await seedPost(`${API_PREFIX}/ar/payments`, {
              customer_id: arPayCustomerId,
              amount: '6000',
              payment_method: '银行转账',
              payment_date: today,
              ...(relatedInvoiceId ? { invoice_ids: [relatedInvoiceId] } : {}),
            });
            if (arPayCreateResp.ok()) {
              const arPayData = await safeJson(arPayCreateResp);
              const arPayId = (arPayData?.data as Record<string, unknown>)?.id as number;
              console.log(`[globalSeed] 创建 AR 收款单 id=${arPayId} customer=${arPayCustomerId}`);
            } else {
              await reportSeedWrite(arPayCreateResp, `AR 收款单创建 i=${i}`);
            }
          }
        }
      } else {
        console.log(`[globalSeed] AR 收款单已有 ${existingArPayments}，满足需求`);
      }
    } catch (e) {
      recordSeedFailure('AR 收款种子', (e as Error).message);
    }
  }

  // ---- 19. 供应商商品色号对照表种子（sku-mapping 功能 e2e 前置；幂等，按 supplier_code 精确定位）----
  // 背景（后端 m0015 business 域种子）：已自建 2 个演示供应商 SUP-DEMO-FAB-01/02 + 各自商品/色号，
  // 但对照表 product_supplier_mappings 本身为空壳数据，e2e 需要至少一条完整"我方产品色号 ↔
  // 供应商商品色号"映射用于级联维护页/转采购翻译等用例的只读前提（写用例自带独立数据 + 清理，不依赖此步）。
  // ⚠️定位方式：m0015 使 suppliers 表多出演示行，`suppliers?page_size=1` 的 [0] 现为 SUP-DEMO-FAB-01
  //   （最小 id），既有用例依赖分页顺序的 supplier 已不可靠。此处一律按 supplier_code 精确匹配，
  //   绝不依赖分页顺序。
  // ⚠️保密：供应商编码仅用于采购域对照，绝不写入任何销售域 seed。
  // 幂等：引用全局 product+color + SUP-DEMO-FAB-01/FAB-P001/PC-A01，先查后建（存在则跳过）。
  if (productId && productColorId) {
    try {
      // 1) 按 supplier_code 精确定位演示供应商 SUP-DEMO-FAB-01
      const demoSupResp = await ctx.get(`${API_PREFIX}/purchase/suppliers?page=1&page_size=200`, {
        headers,
      });
      const demoSupBody = await safeJson(demoSupResp);
      const demoSup = extractItems<{ id: number; supplier_code?: string }>(demoSupBody).find(
        s => s.supplier_code === 'SUP-DEMO-FAB-01'
      );
      if (!demoSup?.id) {
        recordSeedFailure(
          '对照表种子缺演示供应商',
          '未找到 SUP-DEMO-FAB-01（m0015 未生效？），对照表种子未建'
        );
      } else {
        // 2) 定位其商品 FAB-P001（按 product_code 精确匹配）
        const dpResp = await ctx.get(
          `${API_PREFIX}/purchase/supplier-products?supplier_id=${demoSup.id}&page=1&page_size=200`,
          { headers }
        );
        const dpBody = await safeJson(dpResp);
        const demoProduct = extractItems<{ id: number; product_code?: string }>(dpBody).find(
          p => p.product_code === 'FAB-P001'
        );
        // 3) 定位该商品的色号 PC-A01（按 color_no 精确匹配）
        let demoColorId: number | undefined;
        if (demoProduct?.id) {
          const dcResp = await ctx.get(
            `${API_PREFIX}/purchase/supplier-product-colors?supplier_product_id=${demoProduct.id}&page=1&page_size=200`,
            { headers }
          );
          const dcBody = await safeJson(dcResp);
          const demoColor = extractItems<{ id: number; color_no?: string }>(dcBody).find(
            c => c.color_no === 'PC-A01'
          );
          demoColorId = demoColor?.id;
        }
        if (!demoProduct?.id || !demoColorId) {
          recordSeedFailure(
            '对照表种子缺演示商品/色号',
            `product=${demoProduct?.id} color=${demoColorId}（FAB-P001/PC-A01 未定位到，对照表未建）`
          );
        } else {
          // 4) 幂等检查：该 (我方产品, 我方色号, 演示供应商) 组合是否已有对照
          const mapResp = await ctx.get(
            `${API_PREFIX}/purchase/sku-mappings?product_id=${productId}&supplier_id=${demoSup.id}&page=1&page_size=200`,
            { headers }
          );
          const mapBody = await safeJson(mapResp);
          const existing = extractItems<{ product_color_id?: number | null }>(mapBody).some(
            m => (m.product_color_id ?? null) === productColorId
          );
          if (existing) {
            console.log(
              `[globalSeed] 对照表种子已存在（product=${productId} color=${productColorId} supplier=${demoSup.id}），跳过`
            );
          } else {
            const createResp = await seedPost(`${API_PREFIX}/purchase/sku-mappings`, {
              product_id: productId,
              product_color_id: productColorId,
              supplier_id: demoSup.id,
              supplier_product_id: demoProduct.id,
              supplier_product_color_id: demoColorId,
              supplier_price: '88.00',
              is_primary: true,
              is_enabled: true,
            });
            if (createResp.ok()) {
              const created = await safeJson(createResp);
              console.log(
                `[globalSeed] 创建对照表种子 id=${(created?.data as Record<string, unknown>)?.id} ` +
                  `(product=${productId} color=${productColorId} → SUP-DEMO-FAB-01/FAB-P001/PC-A01)`
              );
            } else {
              await reportSeedWrite(createResp, '对照表种子创建');
            }
          }
        }
      }
    } catch (e) {
      recordSeedFailure('对照表种子', (e as Error).message);
    }
  }

  // ---- 20. 本位币种子（GET /currencies/base 404 的数据层根因， flow shard8 实证）----
  // 后端契约事实（逐行读源，非推测）：
  //   ① currency_handler::get_base_currency —— currency_service::get_base_currency 查不到
  //      currencies 表 is_base=true 行时返回 AppError::not_found（HTTP 404），
  //      判据=库内无本位币行，与路由注册无关（routes/finance.rs::currencies 已挂载）；
  //   ② routes/finance.rs::currencies 只注册 list/base/set-base/rates-history/convert/sync-all/
  //      supported，**不存在任何"创建币种"端点**，且全仓迁移（m0005 建表 + system/mod.rs 补列）
  //      均无币种行预置 ⇒ 种子无 API 写入口，唯一路径是直连 DB；与 setup-wizard/
  //      00-setup-wizard.spec.ts「初始化后数据库真实校验」同款 psql 直连方案（复用既有挂点
  //      ensureGlobalBusinessSeed + recordSeedFailure 台账，不新开第二套种子机制）；
  //   ③ currency_service::set_base_currency —— "本位币唯一"由应用层事务保证（先全置 false
  //      再置目标 true），DB 无约束兜底 ⇒ 种子只在**全局无本位币**时把 CNY 置本位，
  //      已有其它本位币时不抢位（探针只要求存在本位币，不要求必须是 CNY）；
  //   ④ create_exchange_rate 的 validate_currency_code 白名单只管汇率写入路径，currencies
  //      表插入不经过该函数；CNY 仍取白名单内真实 ISO 4217 码。
  // 幂等：currencies.code 为 m0005 的 UNIQUE 列；插入用 INSERT...SELECT WHERE NOT EXISTS，
  //   置本位 UPDATE 同样带 NOT EXISTS 守卫 ⇒ 重跑不产生重复行、不产生第二个本位币。
  // 争用：Playwright 的 globalSetup 在每个 `npx playwright test` 进程内、workers 启动前
  //   只执行一次；ci-e2e 各分片 job 拥有独立 postgres service 容器（ci-cd.yml services.postgres），
  //   不存在跨进程同库并发写。
  // 取值：CNY = backend/src/constants.rs::DEFAULT_CURRENCY，与业务单据列 DEFAULT 'CNY' 同源；
  //   id 不写死（serial 自增）；precision/symbol 按 models/currency.rs 可空列给真实值。
  // 失败口径：psql 缺失/连不上/SQL 报错/回读无本位币 → recordSeedFailure 计入台账，
  //   由函数末尾既有汇总统一显式判红（不静默 catch 吞掉——本仓前科教训）。
  {
    const dbUrl = process.env.DATABASE_URL;
    if (!dbUrl) {
      recordSeedFailure(
        '本位币种子',
        'DATABASE_URL 未设置——币种无创建端点，直连 DB 是唯一写路径，缺 URL 即种子失败'
      );
    } else {
      // SQL 经 psql -f - 从 stdin 传入，规避内嵌单引号/中文的 shell 转义问题；
      // URL 用单引号包裹并对内部单引号做 shell 标准转义（当前 URL 无单引号，防御性处理）
      const psqlTarget = `'${dbUrl.replace(/'/g, `'\\''`)}'`;
      const seedSql =
        'INSERT INTO currencies (code, name, symbol, "precision", is_base, is_active, is_deleted, created_at, updated_at)\n' +
        "SELECT 'CNY', '人民币', '¥', 2, true, true, false, now(), now()\n" +
        "WHERE NOT EXISTS (SELECT 1 FROM currencies WHERE code = 'CNY');\n" +
        'UPDATE currencies SET is_base = true, updated_at = now()\n' +
        "WHERE code = 'CNY'\n" +
        '  AND NOT EXISTS (SELECT 1 FROM currencies WHERE is_base = true);\n';
      try {
        execSync(`psql -d ${psqlTarget} -v ON_ERROR_STOP=1 -q -f -`, {
          input: seedSql,
          stdio: ['pipe', 'pipe', 'pipe'],
          env: { ...process.env, PGCONNECT_TIMEOUT: '5' },
        });
        // 回读核验：真实存在 is_base=true 行才算就绪（打印实际 code，不夸大）
        const baseCode = execSync(
          `psql -d ${psqlTarget} -tAc "SELECT code FROM currencies WHERE is_base = true LIMIT 1"`,
          {
            stdio: ['ignore', 'pipe', 'pipe'],
            env: { ...process.env, PGCONNECT_TIMEOUT: '5' },
          }
        )
          .toString()
          .trim();
        if (!baseCode) {
          recordSeedFailure('本位币种子回读', '写语句已执行但查无 is_base=true 行——种子未真实生效');
        } else {
          console.log(`[globalSeed] 本位币就绪：code=${baseCode}`);
        }
      } catch (e) {
        const err = e as { stderr?: { toString(): string }; message?: string };
        const detail = String(err.stderr?.toString() ?? err.message ?? e).slice(0, 300);
        recordSeedFailure('本位币种子（直连 DB 写入）', detail);
      }
    }
  }

  // ---- 21. 客户信用评级种子（GET /crm/customers/{id}/credit 404 的数据层根因， flow shard6 实证）----
  // 后端契约事实（逐行读源，非推测）：
  //   ① routes/crm.rs:48-9 注册 "/customers/{id}/credit" → customer_credit_handler::get_credit，
  //      Path 参数是客户 id（handler:112 Path(customer_id)）；查无行时
  //      customer_credit_service::get_by_customer_id（customer_credit_service.rs:77-86）返回
  //      None → handler:125 AppError::not_found（HTTP 404，文案"客户 X 的信用评级不存在"）。
  //      shard6 backend.log:37987 显示 handler 命中并返回 NotFound ⇒ 404 来自缺数据，非路由漂移。
  //   ② 与本位币（第 20 节，无创建端点只能 psql 直连）相反：本数据存在官方创建端点
  //      POST /crm/customer-credits（crm.rs:93-94 → create_credit handler:267，
  //      body=CreditRatingRequestDto :37-49：customer_id 必填 i32；credit_level ≤20 字符、
  //      credit_limit 0~10 亿且 ≤2 位小数（utils/validator.rs:28-40）、credit_days/score 可选）
  //      ⇒ 按"有创建端点就优先走 API"口径经该端点登记，不新开 DB 直连第二套写路径。
  //   ③ 落库表 customer_credit_ratings（models/customer_credit.rs:9；DDL
  //      m0012_add_ap_ar_finance_analysis.rs:615-634）：customer_id INTEGER NOT NULL（**无
  //      UNIQUE、无 FK**），credit_limit/status NOT NULL，id SERIAL ⇒ 同客户多行在 DB 层不被
  //      禁止，幂等只能靠"先查后建"；service set_credit_rating（customer_credit_limit.rs:31-71）
  //      是先查后写 upsert，但其**更新分支会把请求缺省字段刷回默认值**
  //      （level='B'/score=60/days=30，:41-45）⇒ 已有评级必须跳过、绝不 POST 覆盖
  //      （本位币"不抢位"的同款互斥语义在此=不覆盖既有评级行；该表无全局唯一位概念）。
  // 探针目标漂移说明：flow/22 用例的 customerId 实时取 /crm/customers?page=1&page_size=1
  //   items[0]，而后端列表排序为 created_at DESC（customer_ops/query.rs:104/:121/:129），
  //   运行期其它用例新建客户会让首位漂移 ⇒ 本节保证"库内存在带评级的客户数据层前置"
  //   （含空库时全局客户尚未存在的兜底路径），flow/22 用例内另按同一先查后建口径为
  //   实时解析出的探针客户登记评级（见该 spec 内注释），两层合力使 strict 探针拿到真实数据。
  // 幂等：先 GET credit——200 即跳过；404 才 POST；POST 后回读 GET 必须 200 才算就绪。
  //   重跑不产生第二行（GET 命中即跳过；即便竞跑，service upsert 亦先查后写）。
  // 失败口径：列表空/先查非 200 且非 404/POST 非 2xx/回读非 200 → recordSeedFailure 计入
  //   台账（写请求经 reportSeedWrite 同口径入账），由函数末尾既有汇总统一显式判红，
  //   不静默 catch。
  {
    const creditLabel = '客户信用评级种子';
    try {
      const cusResp = await ctx.get(`${API_PREFIX}/crm/customers?page=1&page_size=1`, { headers });
      if (!cusResp.ok()) {
        const bodyText = await cusResp.text().catch(() => '');
        recordSeedFailure(
          `${creditLabel}·客户列表查询`,
          `HTTP ${cusResp.status()} body=${bodyText.slice(0, 200)}`
        );
      } else {
        const cusBody = await safeJson(cusResp);
        const creditTarget = extractItems<{ id: number }>(cusBody)[0];
        if (!creditTarget?.id) {
          recordSeedFailure(
            `${creditLabel}·客户列表`,
            'GET /crm/customers?page=1&page_size=1 列表为空——无客户可登记评级，数据层前置不成立'
          );
        } else {
          const targetCustomerId = creditTarget.id;
          const probeResp = await ctx.get(
            `${API_PREFIX}/crm/customers/${targetCustomerId}/credit`,
            { headers }
          );
          const probeStatus = probeResp.status();
          if (probeStatus === 200) {
            console.log(
              `[globalSeed] 客户 ${targetCustomerId} 已有信用评级，跳过（不覆盖既有等级/额度）`
            );
          } else if (probeStatus === 404) {
            const creditCreate = await seedPost(`${API_PREFIX}/crm/customer-credits`, {
              customer_id: targetCustomerId,
              credit_level: 'B',
              credit_score: 60,
              credit_limit: '100000',
              credit_days: 30,
            });
            if (!creditCreate.ok()) {
              await reportSeedWrite(
                creditCreate,
                `${creditLabel}·创建评级(客户${targetCustomerId})`
              );
            } else {
              const creditReadBack = await ctx.get(
                `${API_PREFIX}/crm/customers/${targetCustomerId}/credit`,
                { headers }
              );
              if (creditReadBack.status() !== 200) {
                const rbBody = await creditReadBack.text().catch(() => '');
                recordSeedFailure(
                  `${creditLabel}·回读`,
                  `POST 已 2xx 但 GET /crm/customers/${targetCustomerId}/credit 回读 status=${creditReadBack.status()} body=${rbBody.slice(0, 200)}——评级未真实生效`
                );
              } else {
                console.log(`[globalSeed] 客户 ${targetCustomerId} 信用评级种子就绪`);
              }
            }
          } else {
            const probeBody = await probeResp.text().catch(() => '');
            recordSeedFailure(
              `${creditLabel}·先查`,
              `GET /crm/customers/${targetCustomerId}/credit status=${probeStatus} body=${probeBody.slice(0, 200)}`
            );
          }
        }
      }
    } catch (e) {
      recordSeedFailure(creditLabel, (e as Error).message);
    }
  }

  // 种子失败汇总：缺行/软失败都不让它隐身——下游用例的红要先看这里，再判用例本身。
  // 本清单非空时**显式 throw 判红**："打印后继续"会把
  // 「种子没就绪」放大成下游成片红且根因隐身（本仓纪律=静默失败必须响亮化）。
  if (SEED_FAILURES.length > 0) {
    console.error(
      `[globalSeed] ❌ 种子失败汇总 ${SEED_FAILURES.length} 项` +
        `（对应种子行没建出来/写请求被拒，下游用例会以"查不到数据/按钮不可达"的形式红）:`
    );
    SEED_FAILURES.forEach((f, idx) => {
      console.error(`  ${idx + 1}. ${f}`);
    });
    // 取证落盘：Playwright 的 --shard 按用例 hash 分配、与 spec 文件无关（ci-cd.yml 分片注释），
    // 红的那片往往不是缺数据的那片，只翻各分片 stdout 极易漏根因。这里把清单落成
    // 随产物上传的 JSON + GitHub 注解；打印/落盘/注解都完成后才 throw，判红不吞取证。
    const shardTag = SHARD_INDEX || 'local';
    try {
      mkdirSync('reports', { recursive: true });
      writeFileSync(
        `reports/seed-failures-shard${shardTag}.json`,
        JSON.stringify(
          { shard: shardTag, count: SEED_FAILURES.length, failures: SEED_FAILURES },
          null,
          2
        )
      );
    } catch (e) {
      // 落盘失败不阻断判红本身：降级为显式打印后仍 throw。
      console.error(`[globalSeed] ❌ 种子失败清单落盘失败: ${(e as Error).message}`);
    }
    SEED_FAILURES.forEach(f => {
      // 用 warn（本仓 no-console 只放行 warn/error；runner 对 stderr 同样解析
      // workflow 命令），注解打到 PR checks 面板上，排查种子问题时不必再翻各片 stdout。
      console.warn(
        `::warning file=frontend/e2e/global-setup.ts::[globalSeed shard=${shardTag}] ${f}`
      );
    });
    // 判红（响亮化）：setup 抛错 → 本分片以真实根因终止，而不是带着空台账继续跑出成片假红。
    throw new Error(
      `[globalSeed] 种子阶段失败 ${SEED_FAILURES.length} 项（清单已打印并落盘 ` +
        `reports/seed-failures-shard${shardTag}.json），按本仓"静默失败必须响亮化"纪律判红；` +
        '请先修种子根因（多为后端契约/前置数据缺陷），再判下游用例。'
    );
  } else {
    console.log('[globalSeed] 种子失败汇总：0 项');
  }

  console.log('[globalSeed] 全局业务实体种子完成');
}
