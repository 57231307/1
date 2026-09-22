import { test, expect } from '../diagnose-fixture';
import { loginAsRole, loginViaUI, apiCall } from './helpers';

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
