// P2-3 V2Table 冒烟测试公共工具
// 批次 190 规则 6 修复：mock 数据已抽取到 e2e/fixtures/auth.ts，本文件仅做再导出
// 保持 smoke spec 的 import 路径不变（向后兼容）

import type { BrowserContext } from '@playwright/test';
import { applyAuthMocks as applyAuthMocksShared } from '../fixtures/auth';
import { workerUsernameFromEnv } from '../worker-auth';

export {
  generateFakeJwt,
  injectAuthToken,
  mockAuthMe,
  mockInitStatus,
  mockBusinessApi,
  getAiTestUsername,
  waitForPageReady,
} from '../fixtures/auth';

/**
 * applyAuthMocks 的角色账号登录修正（CI E 族成片 401 的根因修复点）。
 *
 * 根因链（制品实证，非推断）
 * - 角色测试账号由 global-setup ensureRoleUsers 创建，初始密码是
 *   DEFAULT_ROLE_PASSWORD，与建号主体密码（会话池里那份 worker 密码）不同；
 * - 共享实现只接收 opts.username，密码若恒取建号主体密码 ⇒ 传角色账号用户名登录必为
 *   密码错误 401。backend.log 里同一角色账号 LOGIN_FAILURE 与带 cred.password 的
 *   LOGIN_SUCCESS 并存 ⇒ 账号与密码本体均正确，纯 helper 契约缺陷
 *   （有 username 入口却无 password 入口）。
 *
 * 本包装的纪律：
 * - 显式传 opts.password（角色凭证来自 role-credentials.json）→ 用该密码真实登录；
 * - 不传 password → 原样委托共享实现，由共享实现取本 worker 的会话账号与密码；
 * - 登录失败仍抛与共享实现同式的原样错误（HTTP 状态 + 响应体），不吞、不降级断言。
 */
export async function applyAuthMocks(
  context: BrowserContext,
  opts?: { username?: string; password?: string }
): Promise<void> {
  if (!opts?.password) {
    return applyAuthMocksShared(context, opts);
  }

  const { request } = await import('@playwright/test');
  const apiBase = process.env.API_BASE || 'http://localhost:8082';
  const apiPrefix = '/api/v1/erp';
  // 显式传了 password 却没传 username：身份仍必须是本 worker 的会话账号，
  // 不能回落到整片共用账号（共用账号 + 多并发 worker 即 CSRF 凭据互踢的根因）。
  const username = opts.username ?? workerUsernameFromEnv();

  const ctx = await request.newContext({
    baseURL: apiBase,
    extraHTTPHeaders: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
    },
  });

  const resp = await ctx.post(`${apiPrefix}/auth/login`, {
    data: { username, password: opts.password },
  });

  if (!resp.ok()) {
    const body = await resp.text();
    await ctx.dispose();
    throw new Error(`applyAuthMocks 登录失败 (user=${username}): HTTP ${resp.status()} ${body}`);
  }

  const cookies = await ctx.storageState();
  const accessCookie = cookies.cookies.find(c => c.name === 'access_token');
  if (!accessCookie) {
    await ctx.dispose();
    throw new Error('applyAuthMocks 登录后未获得 access_token cookie');
  }

  await context.addCookies(cookies.cookies);
  await ctx.dispose();
}
