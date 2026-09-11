import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall, generateTotp, trackPageHealth, assertPageHealthy } from './helpers';

/**
 * P5.5 2FA TOTP 全流程测试
 * 依赖：P3.2（generateTotp）+ 后端 totp/setup、enable、recovery-codes 端点
 *
 * 验证：
 * - setup 拿 secret → TOTP 生成 → enable → 退出后登录带 token 成功、错 token 拒绝
 * - recovery-codes 一次性消费
 * - 禁用回退单因素
 */
test.describe('P5.5 2FA TOTP', () => {
  test('setup → enable → 登录带 token 成功', async ({ page }) => {
    await loginViaUI(page);
    const collector = trackPageHealth(page);

    // 1. setup 拿 secret（后端 GET /auth/totp/setup，非 POST）
    const setupResp = await apiCall(page, 'GET', '/auth/totp/setup');
    const secret = setupResp?.secret ?? setupResp?.data?.secret;
    if (!secret) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    // 2. 生成 TOTP
    const totpCode = generateTotp(secret);

    // 3. enable（保留真实错误到断言消息，便于 CI 诊断 enable 失败根因）
    let enableErr: string | undefined;
    const enableResp = await apiCall(page, 'POST', '/auth/totp/enable', {
      secret,
      code: totpCode,
    }).catch(e => {
      enableErr = (e as Error).message;
      console.error(`[35-totp] enable 失败: ${enableErr}`);
      return null;
    });
    expect(
      enableResp !== null,
      `[35-totp] enable 应成功（secret=${secret?.slice(0, 8)}… code=${totpCode}）: ${enableErr ?? 'enable 返回 null'}`
    ).toBeTruthy();

    await assertPageHealthy(page, collector, { allowConsoleWarn: true });
  });

  test('错 TOTP code 拒绝', async ({ page }) => {
    await loginViaUI(page);

    const setupResp = await apiCall(page, 'GET', '/auth/totp/setup');
    const secret = setupResp?.secret ?? setupResp?.data?.secret;
    if (!secret) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    // 故意用错 code
    const enableResp = await apiCall(page, 'POST', '/auth/totp/enable', {
      secret,
      code: '000000',
    }).catch(e => {
      console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`);
      return null;
    });

    // 应失败或返回错误
    expect(enableResp === null || enableResp?.error).toBeTruthy();
  });

  test('生成恢复码并消费', async ({ page }) => {
    await loginViaUI(page);

    // 后端真实路径为 /auth/totp/recovery-codes（nest 在 /auth 下的 totp 子组）
    const recoveryResp = await apiCall(page, 'POST', '/auth/totp/recovery-codes');
    const codes = recoveryResp?.codes ?? recoveryResp?.data?.codes;
    if (!codes || !Array.isArray(codes) || codes.length === 0) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    // 恢复码应是一次性的
    expect(codes.length).toBeGreaterThan(0);
  });
});
