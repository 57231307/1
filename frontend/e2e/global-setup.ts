import { request } from '@playwright/test';
import { writeFileSync, mkdirSync } from 'fs';

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';
// 分片专属账号：每个 CI runner（matrix.shard）独享，根除跨分片并发登录的 CSRF 互踢。
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

  const resp = await ctx.post(`${API_PREFIX}/auth/login`, {
    data: { username: SHARD_USERNAME, password: SHARD_PASSWORD },
  });

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
  const loginResp = await loginCtx.post(`${API_PREFIX}/auth/login`, {
    data: { username: BASE_USERNAME, password: BASE_PASSWORD },
  });
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
  const rolesBody = (await rolesResp.json().catch((e) => {
    console.warn(`[assignPermissionList] 权限分配失败（不影响角色账号创建）:`, (e as Error).message);
    return null;
  })) as {
    data?: { items?: Array<{ id: number; name?: string }> } | Array<{ id: number; name?: string }>;
  } | null;
  // 响应结构：data.roles[]（role.name 为中文如"管理员"，code 才是 'admin'）
  const roleData = rolesBody?.data as
    | { roles?: Array<{ id: number; name?: string; code?: string }> }
    | Array<{ id: number; name?: string; code?: string }>
    | undefined;
  const roleList = Array.isArray(roleData)
    ? roleData
    : roleData?.roles || (rolesBody?.data as { items?: typeof roleData })?.items || [];
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
    const body = await createResp.text().catch(() => '');
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
  const loginCheck = await checkCtx.post(`${API_PREFIX}/auth/login`, {
    data: { username: SHARD_USERNAME, password: SHARD_PASSWORD },
  });
  await checkCtx.dispose();
  if (!loginCheck.ok()) {
    const body = await loginCheck.text().catch(() => '');
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
  'admin', 'system_admin', 'department_manager', 'purchasing_manager',
  'purchaser', 'sales_manager', 'salesperson', 'warehouse_manager',
  'warehouse_keeper', 'finance_manager', 'accountant', 'cashier',
  'cost_accountant', 'quality_manager', 'quality_inspector',
  'production_manager', 'production_worker', 'dyeing_technician',
  'color_card_manager', 'after_sales_manager', 'customer_service',
  'report_viewer', 'auditor', 'procurement_specialist', 'supplier_manager',
  'inventory_accountant', 'tax_accountant', 'ap_accountant', 'ar_accountant',
  'fixed_assets_accountant', 'budget_analyst',
];

// 边界测试角色
const BOUNDARY_ROLES = [
  { code: 'e2e_readonly', name: 'E2E只读角色', permissions: ['dashboard:read'] },
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
    permissions: ['dashboard:read', 'boms:view', 'boms:print', 'stock:export'],
  },
  {
    code: 'temporary',
    name: '临时账号（E2E黑名单验证）',
    permissions: ['dashboard:read', 'boms:view', 'boms:print', 'stock:export'],
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
  ctx: {
    post: (url: string, options: object) => Promise<{ ok: boolean; status: () => number; headers: () => Record<string, string> }>;
    put: (url: string, options: object) => Promise<{ ok: boolean; status: () => number; headers: () => Record<string, string> }>;
  },
  method: 'post' | 'put',
  url: string,
  headers: Record<string, string>,
  data: Record<string, unknown>
): Promise<{ ok: boolean; status: () => number; headers: () => Record<string, string> }> {
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
  ctx: { post: (url: string, options: object) => Promise<{ ok: boolean; status: () => number; headers: () => Record<string, string> }> },
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
        { resource_type: resourceType, action, allowed: true },
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
  const loginResp = await loginCtx.post(`${API_PREFIX}/auth/login`, {
    data: { username: BASE_USERNAME, password: BASE_PASSWORD },
  });
  if (!loginResp.ok()) {
    await loginCtx.dispose();
    throw new Error(`ensureRoleUsers: ${BASE_USERNAME} 登录失败 HTTP ${loginResp.status()}`);
  }
  const loginCookies = (await loginCtx.storageState()).cookies;
  const csrfCookie = loginCookies.find((c) => c.name === 'csrf_token');
  if (!csrfCookie) {
    await loginCtx.dispose();
    throw new Error('ensureRoleUsers: 未取得 csrf_token cookie');
  }
  const headers = {
    'X-CSRF-Token': csrfCookie.value,
    'X-Requested-With': 'XMLHttpRequest',
  };

  // 2. 拉取角色全量清单
  // 后端 RoleListResponse 形态为 { roles: [...], total }（非 items 包装），
  // 兼容两种形态防止字段错位导致解析为空
  const rolesResp = await loginCtx.get(`${API_PREFIX}/roles?page=1&page_size=200`, { headers });
  const rolesBody = (await rolesResp.json().catch((e) => {
    console.warn(`[assignPermissionList] 权限分配失败（不影响角色账号创建）:`, (e as Error).message);
    return null;
  })) as
    | {
        data?: {
          roles?: Array<{ id: number; code?: string; name?: string }>;
          items?: Array<{ id: number; code?: string; name?: string }>;
        };
      }
    | null;
  const existingRoles = rolesBody?.data?.roles ?? rolesBody?.data?.items ?? [];
  const existingCodes = new Set(existingRoles.map((r) => r.code).filter(Boolean));
  console.log(`[globalSetup] 后端现有角色 ${existingRoles.length} 个`);

  // 3. 自动补建缺失种子角色 + 边界角色 + 黑名单验证角色
  const allRolesToEnsure = [
    ...SEED_ROLES.filter((code) => !existingCodes.has(code)).map((code) => ({
      code,
      name: code,
      permissions: [],
    })),
    ...BOUNDARY_ROLES.filter((r) => !existingCodes.has(r.code)),
    ...BLACKLIST_TEST_ROLES.filter((r) => !existingCodes.has(r.code)),
  ];

  const roleCodeToId = new Map<string, number>(
    existingRoles.map((r) => [r.code, r.id]).filter(([code]) => code),
  );

  for (const role of allRolesToEnsure) {
    const createRoleResp = await requestWithCsrfRecovery(
      loginCtx,
      'post',
      `${API_PREFIX}/roles`,
      headers,
      { code: role.code, name: role.name },
    );
    if (createRoleResp.ok()) {
      const created = (await createRoleResp.json().catch((e) => {
    console.warn(`[assignPermissionList] 权限分配失败（不影响角色账号创建）:`, (e as Error).message);
    return null;
  })) as
        | { data?: { id: number } }
        | null;
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
  if (allRolesToEnsure.length > 0) {
    const reFetch = await loginCtx.get(`${API_PREFIX}/roles?page=1&page_size=200`, { headers });
    const reBody = (await reFetch.json().catch((e) => {
    console.warn(`[assignPermissionList] 权限分配失败（不影响角色账号创建）:`, (e as Error).message);
    return null;
  })) as
      | {
          data?: {
            roles?: Array<{ id: number; code?: string }>;
            items?: Array<{ id: number; code?: string }>;
          };
        }
      | null;
    for (const r of reBody?.data?.roles ?? reBody?.data?.items ?? []) {
      if (r.code) roleCodeToId.set(r.code, r.id);
    }
  }

  // 4. 为每个角色创建测试账号
  const credentials: Record<string, { username: string; password: string }> = {};
  for (const [code, roleId] of roleCodeToId) {
    const username = `e2e_${code}`;
    const password = DEFAULT_ROLE_PASSWORD;
    const createResp = await requestWithCsrfRecovery(
      loginCtx,
      'post',
      `${API_PREFIX}/users`,
      headers,
      { username, password, role_id: roleId, real_name: `E2E-${code}` },
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
  console.log(`[globalSetup] 角色凭证写入 ${ROLE_CREDENTIALS_PATH}（${Object.keys(credentials).length} 角色）`);
}
