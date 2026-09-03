import { request } from '@playwright/test';
import { writeFileSync, mkdirSync } from 'fs';

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';
// 分片专属账号：每个 CI runner（matrix.shard）独享，根除跨分片并发登录的 CSRF 互踢。
// 分片账号通过真实 UI（用户管理页面）创建，属于测试前置数据准备（ensureTestEntities 同级，
// 不属于测试验证手段），UI 测试本身仍全部走真实用户操作。
const BASE_USERNAME = process.env.TEST_USERNAME || 'e2e_admin';
const BASE_PASSWORD = process.env.TEST_PASSWORD || 'E2e@TestPassword2026!';
const SHARD_INDEX = process.env.E2E_SHARD_INDEX ?? '';
const SHARD_USERNAME = SHARD_INDEX !== '' ? `e2e_admin_s${SHARD_INDEX}` : BASE_USERNAME;
const SHARD_PASSWORD = BASE_PASSWORD;
const STORAGE_STATE_PATH = 'e2e/.auth/storage-state.json';

export default async function globalSetup() {
  // ---- 1. 分片账号不存在时，通过真实 UI 创建（e2e_admin 登录 → 用户管理页 → 新建用户）----
  if (SHARD_USERNAME !== BASE_USERNAME) {
    await ensureShardUserViaUI();
  }

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
  const rolesBody = (await rolesResp.json().catch(() => null)) as {
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
    if (body.includes('已存在') || createResp.status() === 409) {
      console.log(`[globalSetup] 分片账号 ${SHARD_USERNAME} 已存在，跳过创建`);
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
