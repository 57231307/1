import { test, expect } from '../diagnose-fixture';
import { API_BASE, API_PREFIX, loginAsRole, loginViaUI, apiCall } from './helpers';

/**
 * P5.10 系统更新授权测试
 * 依赖：P3.1（viewer 角色 + admin 角色）
 *
 * 验证：
 * - check/version/status 真实调用 + 数据断言
 * - 非 admin 403（用 viewer）
 * - rollback/local-update 在 CI 空库环境安全路径
 */
test.describe('P5.10 系统更新授权', () => {
  test('admin 可查询版本与状态', async ({ page }) => {
    await loginViaUI(page);

    // 后端真实注册路径是 /system-update/current-version（routes/mod.rs:254-257），
    // 原写法打的 /system-update/version 根本没注册 → 恒 404，本用例此前是靠 apiCall
    // 抛错前的宽容值勉强取版本号，等于没验证任何东西。
    const versionResp = await apiCall(page, 'GET', '/system-update/current-version');
    const version = versionResp?.version ?? versionResp?.data?.version;
    expect(
      version,
      `admin 查询版本应拿到非空 version，实际响应=${JSON.stringify(versionResp)}`
    ).toBeTruthy();
  });

  test('admin 可查询更新状态', async ({ page }) => {
    await loginViaUI(page);

    // 后端真实路径 /system-update/update-status（原 /status 与 init 路由冲突已重命名）
    const statusResp = await apiCall(page, 'GET', '/system-update/update-status');
    expect(statusResp).toBeTruthy();
    // 状态应包含 is_updating 字段
    const isUpdating = statusResp?.is_updating ?? statusResp?.data?.is_updating;
    expect(typeof isUpdating).toBe('boolean');
  });

  test('viewer 无权限查询系统更新（403）', async ({ page }) => {
    // 只读角色账号由 global-setup ensureRoleUsers 幂等补建；两个候选角色都登录不上属
    // 测试基建缺失（凭证文件/角色未建）。原实现 test.skip() 会让 viewer 403 授权断言
    // 静默消失、基建退化无人发现；该能力对当前部署完全适用，故改为显式失败——
    // 基建必须就绪，viewer 403 必须真验证。
    let viewerOk = false;
    const errors: string[] = [];
    for (const role of ['report_viewer', 'e2e_readonly']) {
      try {
        await loginAsRole(page, role);
        viewerOk = true;
        console.log(`[P5.10] 以只读角色 ${role} 登录成功`);
        break;
      } catch (e) {
        errors.push(`${role}: ${(e as Error).message}`);
      }
    }
    expect(
      viewerOk,
      `[P5.10] 只读角色未就绪，无法验证 viewer 403（基建缺失：见 global-setup ensureRoleUsers / role-credentials.json）—— ${errors.join(' | ')}`
    ).toBe(true);

    // 必须拿**真实状态码**判定：原写法 `resp === null || code !== 200` 会把
    // 未注册路径的 404、网络错误、5xx 全算成"权限已拒绝"，而权限中间件其实根本没参与
    // ——这是安全用例里最危险的假绿（端点坏掉 = 授权通过）。
    const resp = await page.request.get(`${API_BASE}${API_PREFIX}/system-update/current-version`);
    expect(
      resp.status(),
      `viewer 读取系统更新版本应被权限中间件拒为 403，实际 HTTP ${resp.status()}` +
        `（404=路径未注册/写错，200=权限未生效，5xx=服务端异常，三者都不能算通过）`
    ).toBe(403);
  });
});
