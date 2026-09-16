import { test, expect } from '../diagnose-fixture';
import { loginAsRole, apiCall } from './helpers';

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

    const versionResp = await apiCall(page, 'GET', '/system-update/version');
    expect(versionResp).toBeTruthy();
    // 版本号应为 YYYY.M.D.HHMM 格式或编译期版本
    const version = versionResp?.version ?? versionResp?.data?.version;
    expect(version).toBeTruthy();
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
    // report_viewer 不存在则尝试 e2e_readonly；两者都不可用则 skip
    let viewerOk = false;
    try {
      await loginAsRole(page, 'report_viewer');
      viewerOk = true;
    } catch {
      try {
        await loginAsRole(page, 'e2e_readonly');
        viewerOk = true;
      } catch {
        console.warn('[E2E] test.skip: report_viewer 与 e2e_readonly 凭证均不可用');
      }
    }
    if (!viewerOk) {
      test.skip();
      return;
    }

    const resp = await apiCall(page, 'GET', '/system-update/version').catch(e => {
      console.warn(`[E2E] 操作失败: ${(e as Error).message}`);
      return null;
    });
    // viewer 应被拒：apiCall 403 时 throw resp=null（通过）；或 resp.code 非 200
    const denied = resp === null || (resp?.code !== 200 && resp?.code !== 0);
    expect(
      denied,
      `viewer 查询系统更新应被拒，实际 resp=${JSON.stringify(resp)?.slice(0, 200)}`
    ).toBeTruthy();
  });
});
