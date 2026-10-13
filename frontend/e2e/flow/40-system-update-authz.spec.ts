import { test, expect } from '../diagnose-fixture';
import {
  API_BASE,
  API_PREFIX,
  CSRF_ERROR_CODES,
  expectDenied,
  loginAsRole,
  loginViaUI,
  apiCall,
  apiCallExpectFail,
} from './helpers';

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
    // 后端 get_version（system_update_handler.rs:159-166）返回 ApiResponse<VersionResponse>，
    // VersionResponse { version: String }，载荷在 data，故读 versionResp.data.version
    // （原 `?.version ?? ?.data?.version` 是双重包装）。
    const versionResp = await apiCall<{ version?: string }>(
      page,
      'GET',
      '/system-update/current-version'
    );
    const version = versionResp.data?.version;
    expect(
      version,
      `admin 查询版本应拿到非空 version，实际响应=${JSON.stringify(versionResp)}`
    ).toBeTruthy();
  });

  test('admin 可查询更新状态', async ({ page }) => {
    await loginViaUI(page);

    // 后端 get_update_status（system_update_handler.rs:170-179）返回 ApiResponse<UpdateStatusResponse>，
    // UpdateStatusResponse { is_updating: bool, ... }，载荷在 data，读 statusResp.data.is_updating
    // （原 `?.is_updating ?? ?.data?.is_updating` 是双重包装）。
    const statusResp = await apiCall<{ is_updating: boolean }>(
      page,
      'GET',
      '/system-update/update-status'
    );
    expect(statusResp).toBeTruthy();
    const isUpdating = statusResp.data.is_updating;
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

// ============================================================================
// 系统运维岗 ops 授权 + 写端点可达 + 非授权角色写拒绝 + CSRF MISSING 恢复（真实登录）
//
// 背景：后端本轮为系统更新面拆出 9 读 + 7 写共 16 个细粒度权限键，并新增专责角色
// code=ops（data_scope=all，播种这 16 键）；写端点在 RBAC 键之外还有一层 admin/ops
// 深度防御（require_admin_role → is_system_update_authorized_code）。本组用例用真实账号
// 登录，把「ops 通过两道门、非 admin/ops 被 RBAC 拦」钉成行为级断言，而非静态读源码。
// ============================================================================
test.describe('P5.10 系统运维岗 ops 授权（真实登录）', () => {
  test('ops 可读系统更新端点（RBAC read 键放行，返回真实载荷）', async ({ page }) => {
    // ops 凭证由 global-setup ensureRoleUsers 依后端在册角色（含本轮新增 ops）自动补建账号并落盘；
    // 登不上属基建缺失，必须显式失败而非 skip（否则 ops 放行断言静默消失=假绿）。
    await loginAsRole(page, 'ops');

    const version = await apiCallRawVersion(page);
    expect(
      typeof version === 'string' && version.length > 0,
      `ops 读 current-version 应拿到非空 version，实际=${JSON.stringify(version)}`
    ).toBe(true);

    // update-status 新增的真实生效字段：ops 有 update-status:read，应放行且载荷含新键
    const status = await apiCall<{
      is_updating: boolean;
      applied_version: string | null;
      restart_pending: boolean;
    }>(page, 'GET', '/system-update/update-status');
    expect(typeof status.data.is_updating, 'is_updating 应为布尔').toBe('boolean');
    expect(
      status.data,
      `update-status 载荷应含本轮新增 applied_version 键（契约 system_update_handler.rs）：` +
        `实际=${JSON.stringify(status.data)}`
    ).toHaveProperty('applied_version');
    expect(
      status.data,
      `update-status 载荷应含本轮新增 restart_pending 键：实际=${JSON.stringify(status.data)}`
    ).toHaveProperty('restart_pending');
    expect(
      typeof status.data.restart_pending,
      'restart_pending 应为布尔（后端据实判定，非前端臆测）'
    ).toBe('boolean');
  });

  test('ops 可达写端点：创建备份真实成功（RBAC create 键 + admin/ops 深度防御双放行）', async ({
    page,
  }) => {
    // POST /system-update/backups 仅落一条备份记录（非破坏性），返回 200 即证明
    // ops 同时通过 RBAC（system-update-backups:create）与 require_admin_role 两道门。
    await loginAsRole(page, 'ops');
    const created = await apiCall<{ backup_code?: string; status?: string }>(
      page,
      'POST',
      '/system-update/backups',
      { backup_type: 'full', description: 'e2e-ops-authz' }
    );
    expect(
      created.data?.backup_code,
      `ops 创建备份应成功返回 backup_code，实际=${JSON.stringify(created.data)}`
    ).toBeTruthy();
  });

  test('ops 可达写端点：回滚键放行到 handler（非 403，版本不存在的业务失败可接受）', async ({
    page,
  }) => {
    // 用明显不存在的版本触发回滚，避免真实回滚副作用。判据是「不是 403」——
    // 403 意味着 RBAC（system-update-rollback:create）或角色深度防御错误拒绝了 ops；
    // 只要放行到 handler，版本查无会返回业务错误（400/500），这同样证明写权限键已生效。
    await loginAsRole(page, 'ops');
    const res = await apiCallExpectFail(page, 'POST', '/system-update/rollback', {
      version: '__e2e_nonexistent_version__',
    });
    expect(
      res.status,
      `ops 回滚请求应越过 RBAC/角色门到达 handler（不得 403）；` +
        `实际 status=${res.status} code=${res.code}——403 说明 ops 未真正持有该写键`
    ).not.toBe(403);
  });

  test('非 admin 非 ops 角色对 system-update 写端点被 RBAC 拒为 403', async ({ page }) => {
    let deniedRole: string | null = null;
    const errors: string[] = [];
    for (const role of ['report_viewer', 'e2e_readonly']) {
      try {
        await loginAsRole(page, role);
        deniedRole = role;
        break;
      } catch (e) {
        errors.push(`${role}: ${(e as Error).message}`);
      }
    }
    expect(
      deniedRole,
      `[40-authz] 非授权角色未就绪，无法验证写端点 403（基建缺失：见 ensureRoleUsers/role-credentials）——` +
        errors.join(' | ')
    ).not.toBeNull();

    // 创建备份：只读角色无 system-update-backups:create，RBAC 必拒（且必须是权限门 FORBIDDEN，
    // 不是 CSRF 竞败——helpers.expectDenied 已内置二者区分）。
    const backupDenied = await apiCallExpectFail(page, 'POST', '/system-update/backups', {
      backup_type: 'full',
    });
    expectDenied(backupDenied, `${deniedRole} 创建系统更新备份应被权限中间件拒为 403`);

    // 回滚：同理无 system-update-rollback:create → 403 FORBIDDEN。
    const rollbackDenied = await apiCallExpectFail(page, 'POST', '/system-update/rollback', {
      version: '__e2e_nonexistent_version__',
    });
    expectDenied(rollbackDenied, `${deniedRole} 回滚系统更新应被权限中间件拒为 403`);
  });
});

// ============================================================================
// CSRF MISSING 恢复通道：写请求不带 X-CSRF-Token 时，中间件仍下发恢复头，且整体维持 403。
// 后端 csrf 中间件对 MISSING 与 NotFound(INVALID) 同模式签发恢复 token
// （x-new-csrf-token 响应头 + Set-Cookie csrf_token），但不放宽 403 策略。这里用真实登录会话
// 直接裸发（不经 apiCall 的自动恢复重放），验证恢复头确实可观测且状态仍为 403。
// ============================================================================
test.describe('P5.10 CSRF MISSING 恢复头（真实登录、不自动重放）', () => {
  test('ops 会话裸发无 CSRF 头的写请求 → 403 且携带 x-new-csrf-token + Set-Cookie', async ({
    page,
  }) => {
    await loginAsRole(page, 'ops');
    // 绕过 apiCall（其会注入 CSRF 头并自动重放），手发一个缺 X-CSRF-Token 的 POST，
    // 让 CSRF 中间件命中 MISSING 分支，观察恢复通道。
    const resp = await page.request.fetch(`${API_BASE}${API_PREFIX}/system-update/backups`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
        // 故意不发 X-CSRF-Token，触发 CSRF_TOKEN_MISSING
      },
      data: JSON.stringify({ backup_type: 'full', description: 'e2e-csrf-missing' }),
    });

    expect(
      resp.status(),
      `缺 CSRF 头的写请求应被 CSRF 中间件拒为 403（不放宽失效即放行），实际 HTTP ${resp.status()}`
    ).toBe(403);

    const body = (await resp.json().catch(() => null)) as { code?: string } | null;
    expect(
      body?.code,
      `403 应归因 CSRF_TOKEN_MISSING（MISSING 分支），实际 body=${JSON.stringify(body)}`
    ).toBe(CSRF_ERROR_CODES.MISSING);

    const headers = resp.headers();
    const recoveryToken = headers['x-new-csrf-token'];
    expect(
      typeof recoveryToken === 'string' && recoveryToken.length > 0,
      `MISSING 恢复应下发非空 x-new-csrf-token 头，实际=${JSON.stringify(recoveryToken)}`
    ).toBe(true);
    expect(
      headers['set-cookie'],
      `MISSING 恢复应同时 Set-Cookie 下发 csrf_token（供前端读取重放），实际 set-cookie=${JSON.stringify(
        headers['set-cookie']
      )}`
    ).toContain('csrf_token=');
  });
});

/** 取 current-version 的 version 字符串（ops 读放行断言用，失败即抛由用例判红，不兜底掩盖） */
async function apiCallRawVersion(page: import('@playwright/test').Page): Promise<string> {
  const res = await apiCall<{ version?: string }>(page, 'GET', '/system-update/current-version');
  return res.data?.version as string;
}
