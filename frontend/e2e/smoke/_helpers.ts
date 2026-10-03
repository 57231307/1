// P2-3 V2Table 冒烟测试公共工具
// 批次 190 规则 6 修复：mock 数据已抽取到 e2e/fixtures/auth.ts，本文件仅做再导出
// 保持 smoke spec 的 import 路径不变（向后兼容）

import type { BrowserContext } from '@playwright/test';
import { applyAuthMocks as applyAuthMocksShared } from '../fixtures/auth';

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
 * applyAuthMocks 的角色账号登录修正（CI #4669 E 族成片 401 的根因修复点）。
 *
 * 根因链（#4669 制品实证，非推断）：
 * - 角色测试账号由 global-setup ensureRoleUsers 创建，初始密码是
 *   DEFAULT_ROLE_PASSWORD='E2eRole#2026'（global-setup.ts:484、:746-751），
 *   与分片主账号密码（env TEST_PASSWORD）不同；
 * - fixtures/auth.ts:173-175 的 applyAuthMocks 只接收 opts.username，密码恒取
 *   TEST_PASSWORD ⇒ 传角色账号用户名登录必为密码错误 401。
 *   backend.log（rs31 分片）17482 行 security_audit_detail：
 *   LOGIN_FAILURE user=e2e_salesperson reason="未授权：无效的密码: 密码错误"；
 *   同片 17602 行 22:58:16 loginInIsolatedContext（带 cred.password）同账号 LOGIN_SUCCESS
 *   ⇒ 账号与密码本体均正确，纯 helper 契约缺陷（有 username 入口却无 password 入口）。
 *
 * 本包装的纪律：
 * - 显式传 opts.password（角色凭证来自 role-credentials.json）→ 用该密码真实登录；
 * - 不传 password → 原样委托共享实现，55+ spec 既有的 e2e_admin_s{n}+TEST_PASSWORD
 *   路径行为逐字节不变（零回归面）；
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
  const shardIndex = process.env.E2E_SHARD_INDEX ?? '';
  const baseUsername = process.env.E2E_BASE_USERNAME || 'e2e_admin';
  const username = opts.username ?? (shardIndex !== '' ? `e2e_admin_s${shardIndex}` : baseUsername);

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
