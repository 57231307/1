import { request } from '@playwright/test';
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
const STORAGE_STATE_PATH = 'e2e/.auth/storage-state.json';

/**
 * requestWithCsrfRecovery / assignPermissionList 的响应契约（单一声明点）。
 *
 * 对齐 Playwright 真实类型 APIResponse：
 * node_modules/playwright-core/types/types.d.ts:12137 `export interface APIResponse<T = any>`
 *   :12178 `ok(): boolean;`     —— ok 是**方法**不是属性
 *   :12143 `json(): Promise<T>;`
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
}

/** 仅声明 setup 用到的写方法（实参是 Playwright APIRequestContext，方法签名双变可赋值） */
interface CsrfCapableRequestContext {
  post(url: string, options: object): Promise<SetupApiResponse>;
  put(url: string, options: object): Promise<SetupApiResponse>;
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
  writeFileSync(STORAGE_STATE_PATH, JSON.stringify(cookies, null, 2));

  // ---- 2.5 全局业务实体种子（供不依赖 ensureTestEntities 的 extras specs 使用）----
  // extras 目录（sales/purchase/crm/color-card/price/production/quality 等）的
  // spec 不调用 ensureTestEntities，直接 navigate 后断言"状态行存在/下拉有选项"。
  // 本步骤用纯 API 建最小前置集，使 GET 列表非空。幂等：先查后建。
  await ensureGlobalBusinessSeed(ctx);

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
    // 幂等：400/409 或文案含"已存在"都视为账号已建（watchdog 重跑同一分片时
    // 第 1 轮已创建账号，重复 POST 返回 400 BusinessError"用户名已存在"，
    // run 34076635269 十二分片全部因 400 未被幂等识别而瞬间失败）
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
];

// 应用外壳权限码：与后端 init_service_ops/permission.rs 的 SHELL_PERMISSIONS 一致——
// 登录后落地页 /dashboard，主框架铃铛拉取自身未读数。缺码的角色登录后停在 /403。
const SHELL_PERMISSIONS = ['dashboard:read', 'notifications:read'];

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
 * CSRF 一次性消费的恢复包装：写请求 403（CSRF_TOKEN_INVALID）时读取
 * 响应头 X-New-CSRF-Token 更新 headers 并重试一次（每分片独立登录，
 * 并发分片同库时旧 token 必被竞争消费，无恢复则后续全部写请求 403）
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
  return resp;
}

/**
 * 为角色分配权限码（POST /roles/{id}/permissions 单条模式，幂等）
 * 权限码格式 'product:print' → { resource_type: 'product', action: 'print', allowed: true }
 * 单条失败仅告警不中断（黑名单断言对无权限码场景仍成立，只是失去"持码仍拒"精度）
 */
async function assignPermissionList(
  ctx: CsrfCapableRequestContext,
  roleId: number,
  permissionCodes: string[],
  headers: Record<string, string>
): Promise<void> {
  for (const code of permissionCodes) {
    const [resourceType, action] = code.split(':');
    if (!resourceType || !action) {
      console.warn(`[globalSetup] 权限码格式非法（应为 resource:action）: ${code}`);
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
      if (!resp.ok() && resp.status() !== 400 && resp.status() !== 409) {
        console.warn(`[globalSetup] 权限 ${code} 分配失败 HTTP ${resp.status()}`);
      }
    } catch (e) {
      console.warn(`[globalSetup] 权限 ${code} 分配异常:`, (e as Error).message);
    }
  }
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
  // 补建角色只给应用外壳权限码（落地页 + 自己的收件箱）：
  // 32-roles 断言"登录 + Dashboard 可达"，业务域权限按各专项 spec 自行分配
  const allRolesToEnsure = [
    ...SEED_ROLES.filter(code => !existingCodes.has(code)).map(code => ({
      code,
      name: code,
      permissions: SHELL_PERMISSIONS,
    })),
    ...BOUNDARY_ROLES.filter(r => !existingCodes.has(r.code)),
    ...BLACKLIST_TEST_ROLES.filter(r => !existingCodes.has(r.code)),
  ];

  // 原写法 new Map<string, number>(existingRoles.map(r => [r.code, r.id]).filter(([code]) => code))
  // 因 r.code: string | undefined 把元组放宽成 (string|number)[]，Map 构造报 TS2769；
  // 收紧为：仅纳入 code 存在的行（与既有 existingCodes 口径一致）。
  const roleCodeToId = new Map<string, number>();
  for (const r of existingRoles) {
    if (r.code) roleCodeToId.set(r.code, r.id);
  }

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
        // 分配权限（POST /roles/{id}/permissions 单条模式：resource_type+action）
        if (role.permissions.length > 0) {
          await assignPermissionList(loginCtx, created.data.id, role.permissions, headers);
        }
        console.log(`[globalSetup] 角色 ${role.code} 创建成功 (id=${created.data.id})`);
      }
    } else if (createRoleResp.status() === 400 || createRoleResp.status() === 409) {
      console.log(`[globalSetup] 角色 ${role.code} 已存在，跳过创建`);
      // 已存在角色也补齐权限码（幂等；33b 黑名单断言依赖"持码仍拒"）
      if (role.permissions.length > 0) {
        const roleId = roleCodeToId.get(role.code);
        if (roleId) {
          await assignPermissionList(loginCtx, roleId, role.permissions, headers);
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
      console.warn(`[globalSetup] 角色账号 ${username} 创建失败 HTTP ${createResp.status()}`);
    }
    credentials[code] = { username, password };
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
 * 背景（CI #4646 簇 A 铁证）：extras 分片的 sales/purchase/crm/color-card/price/
 * production/quality 等 spec 不调用 ensureTestEntities，直接导航后断言"状态行存在/
 * 下拉有选项"。globalSetup 过去只建角色/账号，不建业务实体 → 40+ 例成片空红。
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
  // 获取 csrf_token（写操作需要）
  const cookies = (await ctx.storageState()).cookies;
  const csrfCookie = cookies.find(c => c.name === 'csrf_token');
  if (!csrfCookie) {
    console.warn('[globalSeed] 无 csrf_token cookie，跳过业务种子（后续写请求将 403）');
    return;
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

  // 辅助：取当前用户 ID（用于 sales_user_id）
  let currentUserId = 0;
  try {
    const meResp = await ctx.get(`${API_PREFIX}/auth/me`, { headers });
    const meBody = await safeJson(meResp);
    currentUserId = ((meBody?.data as Record<string, unknown>)?.id as number) ?? 0;
    if (!currentUserId) {
      console.error('[globalSeed] /auth/me 未返回 id，后续需要 user_id 的实体将跳过');
    }
  } catch (e) {
    console.error('[globalSeed] /auth/me 查询异常:', (e as Error).message);
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
    console.warn('[globalSeed] 产品分类检查异常:', (e as Error).message);
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
    console.warn('[globalSeed] 产品检查/创建异常:', (e as Error).message);
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
      await seedPost(`${API_PREFIX}/purchase/suppliers`, {
        supplier_name: `E2E全局供应商${ts}`,
        supplier_short_name: 'E2EG',
        contact_phone: '13800000099',
      });
      console.log('[globalSeed] 创建全局供应商');
    }
  } catch (e) {
    console.warn('[globalSeed] 供应商检查异常:', (e as Error).message);
  }

  // ---- 4. 客户 ----
  try {
    const cusResp = await ctx.get(`${API_PREFIX}/crm/customers?page=1&page_size=1`, { headers });
    const cusBody = await safeJson(cusResp);
    const cuss = extractItems<{ id: number }>(cusBody);
    if (cuss.length === 0) {
      const ts = Date.now().toString().slice(-6);
      await seedPost(`${API_PREFIX}/crm/customers`, {
        customer_name: `E2E全局客户${ts}`,
        contact_phone: '13900000099',
      });
      console.log('[globalSeed] 创建全局客户');
    }
  } catch (e) {
    console.warn('[globalSeed] 客户检查异常:', (e as Error).message);
  }

  // ---- 5. 仓库 ----
  try {
    const whResp = await ctx.get(`${API_PREFIX}/warehouses?page=1&page_size=2`, { headers });
    const whBody = await safeJson(whResp);
    const whs = extractItems<{ id: number }>(whBody);
    if (whs.length < 2) {
      for (let i = whs.length; i < 2; i++) {
        const ts = Date.now().toString().slice(-6);
        await seedPost(`${API_PREFIX}/warehouses`, {
          name: `E2E全局仓库${ts}${i}`,
          code: `E2E-GW${ts}${i}`,
        });
      }
      console.log('[globalSeed] 创建全局仓库');
    }
  } catch (e) {
    console.warn('[globalSeed] 仓库检查异常:', (e as Error).message);
  }

  // ---- 6. 色卡 ----
  try {
    const ccResp = await ctx.get(`${API_PREFIX}/color-cards?page=1&page_size=1`, { headers });
    const ccBody = await safeJson(ccResp);
    const ccs = extractItems<{ id: number }>(ccBody);
    if (ccs.length === 0) {
      const ts = Date.now().toString().slice(-6);
      await seedPost(`${API_PREFIX}/color-cards`, {
        card_no: `E2E-GCC${ts}`,
        card_name: `E2E全局色卡${ts}`,
        card_type: 'PANTONE',
      });
      console.log('[globalSeed] 创建全局色卡');
    }
  } catch (e) {
    console.warn('[globalSeed] 色卡检查异常:', (e as Error).message);
  }

  // ---- 7. 色号定价 ----
  if (productId && productColorId) {
    try {
      const cpResp = await ctx.get(`${API_PREFIX}/color-prices?page=1&page_size=1`, { headers });
      const cpBody = await safeJson(cpResp);
      const cps = extractItems<{ id: number }>(cpBody);
      if (cps.length === 0) {
        await seedPost(`${API_PREFIX}/color-prices`, {
          product_id: productId,
          color_id: productColorId,
          currency: 'CNY',
          base_price: '15.00',
          effective_from: new Date().toISOString().slice(0, 10),
        });
        console.log('[globalSeed] 创建全局色号定价');
      }
    } catch (e) {
      console.warn('[globalSeed] 色号定价检查异常:', (e as Error).message);
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
              // submit + approve（小额自批后端直接通过或需 approve）
              await seedPost(`${API_PREFIX}/quotations/${qId}/submit`, {});
              const approveResp = await seedPost(`${API_PREFIX}/quotations/${qId}/approve`, {});
              console.log(
                `[globalSeed] 创建并审批全局报价单 id=${qId} (approve ${approveResp.status()})`
              );
            }
          } else {
            console.warn(`[globalSeed] 报价单创建失败 HTTP ${createQ.status()}`);
          }
        }
      }
    } catch (e) {
      console.warn('[globalSeed] 报价单检查异常:', (e as Error).message);
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
      await seedPost(`${API_PREFIX}/production/fabric-inspections`, {
        inspection_date: new Date().toISOString().slice(0, 10),
        product_id: productId,
        color_no: 'E2E-GC',
        scoring_system: 'four_point',
        fabric_width_inches: 60,
        inspector_name: 'E2E全局验布员',
      });
      console.log('[globalSeed] 创建全局验布记录（含 fabric_width_inches=60）');
    }
  } catch (e) {
    console.warn('[globalSeed] 验布记录检查异常:', (e as Error).message);
  }

  // ---- 10. BPM 流程定义（幂等：先查后建）----
  try {
    const bpmResp = await ctx.get(`${API_PREFIX}/bpm/definitions?page=1&page_size=50`, { headers });
    const bpmBody = await safeJson(bpmResp);
    const bpms = extractItems<{ code?: string }>(bpmBody);
    if (!bpms.some(d => d.code === 'sales_order_approval')) {
      const approverId = currentUserId;
      await seedPost(`${API_PREFIX}/bpm/definitions`, {
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
      console.log('[globalSeed] 创建 BPM sales_order_approval');
    }
  } catch (e) {
    console.warn('[globalSeed] BPM 定义检查异常:', (e as Error).message);
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
        if (resp.status() >= 500) {
          console.error(
            `[globalSeed] ⚠️ 销售订单创建返回 HTTP ${resp.status()}（疑似后端潜伏缺陷，不吞掉）`
          );
        }
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
            console.warn(`[globalSeed] 销售订单 submit 失败 id=${id} HTTP ${submitResp.status()}`);
          }
        }
      }

      // 补建 approved 态订单（创建后 submit + approve）
      const needApproved = Math.max(0, SALES_ORDER_MIN_APPROVED - currentApproved);
      for (let i = 0; i < needApproved; i++) {
        const id = await createSalesOrder();
        if (id) {
          await seedPost(`${API_PREFIX}/sales/orders/${id}/submit`, {});
          const approveResp = await seedPost(`${API_PREFIX}/sales/orders/${id}/approve`, {});
          if (approveResp.ok()) {
            console.log(`[globalSeed] 创建销售订单(approved) id=${id}`);
          } else {
            console.warn(
              `[globalSeed] 销售订单 approve 失败 id=${id} HTTP ${approveResp.status()}`
            );
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
  try {
    const supResp = await ctx.get(`${API_PREFIX}/purchase/suppliers?page=1&page_size=1`, {
      headers,
    });
    const supBody = await safeJson(supResp);
    const supplierId = extractItems<{ id: number }>(supBody)[0]?.id;

    if (supplierId && productId) {
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
        if (resp.status() >= 500) {
          console.error(
            `[globalSeed] ⚠️ 采购订单创建返回 HTTP ${resp.status()}（疑似后端潜伏缺陷，不吞掉）`
          );
        }
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
            console.warn(`[globalSeed] 采购订单 submit 失败 id=${id} HTTP ${submitResp.status()}`);
          }
        }
      }

      // 补建 APPROVED（创建后 submit + approve）
      const needPoApproved = Math.max(0, PO_MIN_APPROVED - poApproved);
      for (let i = 0; i < needPoApproved; i++) {
        const id = await createPurchaseOrder();
        if (id) {
          await seedPost(`${API_PREFIX}/purchase/orders/${id}/submit`, {});
          const approveResp = await seedPost(`${API_PREFIX}/purchase/orders/${id}/approve`, {});
          if (approveResp.ok()) {
            console.log(`[globalSeed] 创建采购订单(APPROVED) id=${id}`);
          } else {
            console.warn(
              `[globalSeed] 采购订单 approve 失败 id=${id} HTTP ${approveResp.status()}`
            );
          }
        }
      }
    }
  } catch (e) {
    console.warn('[globalSeed] 采购订单种子异常:', (e as Error).message);
  }

  // ---- 13. AR 应收发票种子（覆盖 sales/05-01 列表列头可见性验证）----
  // 端点：POST /api/v1/erp/ar/invoices
  // DTO：CreateArInvoiceRequestDto（handlers/ar_invoice_handler.rs:46）
  // 状态：DRAFT（创建默认态），需 APPROVED 态用于后续收款流程测试
  try {
    const arResp = await ctx.get(`${API_PREFIX}/ar/invoices?page=1&page_size=1&status=DRAFT`, {
      headers,
    });
    const arBody = await safeJson(arResp);
    const arInvoices = Array.isArray(arBody?.data) ? arBody?.data : extractItems(arBody);
    if ((arInvoices as unknown[]).length === 0 && currentUserId) {
      // 获取一个客户 id
      const cusResp = await ctx.get(`${API_PREFIX}/crm/customers?page=1&page_size=1`, { headers });
      const cusBody = await safeJson(cusResp);
      const arCustomerId = extractItems<{ id: number }>(cusBody)[0]?.id;
      if (arCustomerId) {
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
          console.log(`[globalSeed] 创建 AR 应收发票(DRAFT) id=${arId}`);
          // 审核通过使其变为 APPROVED（对应前端标签"已审核"）
          if (arId) {
            const approveAr = await seedPost(`${API_PREFIX}/ar/invoices/${arId}/approve`, {});
            console.log(`[globalSeed] AR 发票审核 id=${arId} HTTP ${approveAr.status()}`);
          }
        } else if (arCreate.status() >= 500) {
          console.error(
            `[globalSeed] ⚠️ AR 发票创建返回 HTTP ${arCreate.status()}（疑似后端潜伏缺陷，不吞掉）`
          );
        } else {
          const errBody = (await arCreate.json().catch(() => null)) as Record<
            string,
            unknown
          > | null;
          console.warn(
            `[globalSeed] AR 发票创建失败 HTTP ${arCreate.status()} body=${JSON.stringify(errBody).slice(0, 200)}`
          );
        }
      }
    }
  } catch (e) {
    console.warn('[globalSeed] AR 发票种子异常:', (e as Error).message);
  }

  // ---- 14. AP 应付发票种子（覆盖 purchase/05-01 列表列头可见性验证）----
  // 端点：POST /api/v1/erp/ap/invoices
  // DTO：CreateApInvoiceRequest（services/ap_invoice_ops/types.rs:24）supplier_id, invoice_type, amount, invoice_date, due_date
  // 状态：DRAFT→approve→AUDITED
  try {
    const apResp = await ctx.get(`${API_PREFIX}/ap/invoices?page=1&page_size=1`, { headers });
    const apBody = await safeJson(apResp);
    const apInvoices = Array.isArray(apBody?.data) ? apBody?.data : extractItems(apBody);
    if ((apInvoices as unknown[]).length === 0 && currentUserId) {
      const supResp = await ctx.get(`${API_PREFIX}/purchase/suppliers?page=1&page_size=1`, {
        headers,
      });
      const supBody = await safeJson(supResp);
      const apSupplierId = extractItems<{ id: number }>(supBody)[0]?.id;
      if (apSupplierId) {
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
          console.log(`[globalSeed] 创建 AP 应付发票(DRAFT) id=${apId}`);
          // 审核通过 → AUDITED（前端标签"已审核"）
          if (apId) {
            const approveAp = await seedPost(`${API_PREFIX}/ap/invoices/${apId}/approve`, {});
            console.log(`[globalSeed] AP 发票审核 id=${apId} HTTP ${approveAp.status()}`);
          }
        } else if (apCreate.status() >= 500) {
          console.error(
            `[globalSeed] ⚠️ AP 发票创建返回 HTTP ${apCreate.status()}（疑似后端潜伏缺陷，不吞掉）`
          );
        } else {
          const errBody = (await apCreate.json().catch(() => null)) as Record<
            string,
            unknown
          > | null;
          console.warn(
            `[globalSeed] AP 发票创建失败 HTTP ${apCreate.status()} body=${JSON.stringify(errBody).slice(0, 200)}`
          );
        }
      }
    }
  } catch (e) {
    console.warn('[globalSeed] AP 发票种子异常:', (e as Error).message);
  }

  // ---- 15. 库存种子（sales/04 发货、purchase/04 质检等链路的前置库存）----
  // 根因（取证 #4647 簇A）：步骤5只建了仓库、步骤2只建了产品，但从没建库存行，
  //   导致 sales/04-04 打开发货对话框时 loadDeliveryStockRows（GET /inventory/stock?
  //   warehouse_id&product_id）拿到空列表 → 「库存行」下拉无选项，四维出库走不通。
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
  //   不吞、不 fake（与 color_price INT8/i32 潜伏 bug 同类风险点）。
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
        console.warn(
          `[globalSeed] 库存种子前置不足（supplier=${stockSupplierId} code=${materialCode} warehouses=${stockWarehouses.length}），跳过库存种子`
        );
      } else {
        const today = new Date().toISOString().slice(0, 10);
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
            const errText = await poResp.text().catch(() => '');
            console.error(
              `[globalSeed] ⚠️ 库存种子建采购订单失败 HTTP ${poResp.status()} warehouse=${wh.id} body=${errText.slice(0, 300)}${poResp.status() >= 500 ? '（疑似后端潜伏缺陷，不吞掉）' : ''}`
            );
            continue;
          }
          // 2) 提交 + 审批
          await seedPost(`${API_PREFIX}/purchase/orders/${poId}/submit`, {});
          const poApprove = await seedPost(`${API_PREFIX}/purchase/orders/${poId}/approve`, {});
          if (!poApprove.ok()) {
            const errText = await poApprove.text().catch(() => '');
            console.error(
              `[globalSeed] ⚠️ 库存种子采购订单审批失败 HTTP ${poApprove.status()} po=${poId} body=${errText.slice(0, 300)}${poApprove.status() >= 500 ? '（疑似后端潜伏缺陷，不吞掉）' : ''}`
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
            const errText = await rcvResp.text().catch(() => '');
            console.error(
              `[globalSeed] ⚠️ 库存种子建入库单失败 HTTP ${rcvResp.status()} po=${poId} warehouse=${wh.id} body=${errText.slice(0, 300)}${rcvResp.status() >= 500 ? '（疑似后端潜伏缺陷，不吞掉）' : ''}`
            );
            continue;
          }
          // 4) 确认入库 → update_inventory_txn 自动落 inventory_stocks
          const confirmResp = await seedPost(
            `${API_PREFIX}/purchase/receipts/${rcvId}/confirm`,
            {}
          );
          if (confirmResp.ok()) {
            console.log(
              `[globalSeed] 库存种子落库成功 product=${productId} warehouse=${wh.id} rcv=${rcvId} 色号=${colorCode} 批次=${batchNo} 缸号=${dyeLotNo} qty=10000`
            );
          } else {
            const errText = await confirmResp.text().catch(() => '');
            console.error(
              `[globalSeed] ⚠️ 库存种子「确认入库」失败 HTTP ${confirmResp.status()} rcv=${rcvId} warehouse=${wh.id} body=${errText.slice(0, 300)}${confirmResp.status() >= 500 ? '（疑似后端潜伏缺陷，不吞掉）' : ''}`
            );
          }
        }
      }
    } catch (e) {
      console.error('[globalSeed] 库存种子异常:', (e as Error).message);
    }
  }

  console.log('[globalSeed] 全局业务实体种子完成');
}
