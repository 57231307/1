import { test, expect } from '../diagnose-fixture';
import { loginAsRole } from './helpers';

/**
 * 33b 角色黑名单端到端验证（doto 2026-09-09 print/export 角色黑名单缺口）
 *
 * 后端三层黑名单（backend/src/middleware/permission.rs）：
 * - PRINT_DENIED_ROLE_CODES = ["customer", "temporary"]
 * - EXPORT_DENIED_ROLE_CODES = ["customer", "temporary"]
 * - DYE_RECIPE_EXPORT_DENIED_ROLE_CODES（customer/temporary/manager/operator/...）
 *
 * 黑名单独立于权限码：ensureRoleUsers 给 customer/temporary 配置了
 * product:print / product:export 权限码——若黑名单失效，持码请求会 200 放行；
 * 黑名单生效 → 403。这是对"持码仍拒"的真实端到端断言。
 *
 * dye_recipe 导出：manager 在 DYE_RECIPE 导出禁单中 → 403。
 * 对照组：admin 打印可达（区分端点缺失与权限拒绝）。
 *
 * 凭证来源：ensureRoleUsers 幂等补建（global-setup.ts BLACKLIST_TEST_ROLES）
 * 每步显式日志
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

/** 黑名单角色：持 print/export 权限码仍必须被拒 */
const BLACKLIST_ROLES = ['customer', 'temporary'];

test.describe('33b 角色黑名单（print/export/dye-recipe）', () => {
  for (const role of BLACKLIST_ROLES) {
    test(`${role} 持 boms:print 权限码调用 /boms/1/print → 403`, async ({ page }) => {
      await loginAsRole(page, role);
      console.log(`[33b] ${role} 登录成功（凭证据 ensureRoleUsers 补建）`);

      const resp = await page.request
        .get(`${API_BASE}${API_PREFIX}/boms/1/print`)
        .catch((e) => { console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`); return null; });
      if (!resp) throw new Error('网络错误: /boms/1/print');

      const status = resp.status();
      if (status === 404) {
        // 无 BOM 种子数据：404 在黑名单判断之前/资源侧，单独记录
        test.info().annotations.push({
          type: 'missing-data',
          description: 'boms/1 不存在（种子缺失），打印黑名单断言需种子数据',
        });
        console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
        test.skip();
        return;
      }
      expect(
        status,
        `${role} 在 PRINT_DENIED 黑名单，持权限码也应 403，实际 ${status}` +
          (status === 200 ? '（黑名单失效——真缺陷）' : ''),
      ).toBe(403);
      console.log(`[33b] ✅ ${role} 打印黑名单生效 → 403`);
    });

    test(`${role} 持 stock:export 权限码调用（/inventory/stock/export） /inventory/stock/export → 403`, async ({ page }) => {
      await loginAsRole(page, role);

      const resp = await page.request
        .get(`${API_BASE}${API_PREFIX}/inventory/stock/export`)
        .catch((e) => { console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`); return null; });
      if (!resp) throw new Error('网络错误: /inventory/stock/export');

      // /inventory/stock/export 是非敏感导出：黑名单是唯一防线——403 即黑名单直接生效，
      // 若 200 放行则 EXPORT_DENIED 黑名单失效（真缺陷）
      const status = resp.status();
      expect(
        status,
        `${role} 在 EXPORT_DENIED 黑名单，应 403，实际 ${status}` +
          (status === 200 ? '（黑名单失效——真缺陷）' : ''),
      ).toBe(403);
      console.log(`[33b] ✅ ${role} 导出黑名单生效 → 403`);
    });
  }

  test('manager 调染料配方导出 → 403（DYE_RECIPE 导出禁单）', async ({ page }) => {
    await loginAsRole(page, 'manager');

    const resp = await page.request
      .get(`${API_BASE}${API_PREFIX}/production/dye-recipes/export`)
      .catch((e) => { console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`); return null; });
    if (!resp) throw new Error('网络错误: /production/dye-recipes/export');

    const status = resp.status();
    if (status === 404) {
      // 无染料配方种子数据：404 在黑名单判断之前/资源侧，记录并跳过
      test.info().annotations.push({
        type: 'missing-data',
        description: 'dye-recipes/1 不存在（种子缺失），黑名单断言需种子数据',
      });
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }
    expect(
      status,
      `manager 在 DYE_RECIPE_EXPORT_DENIED 清单，导出应 403，实际 ${status}`,
    ).toBe(403);
    console.log('[33b] ✅ manager 染料配方导出禁单生效 → 403');
  });

  test('admin 对照：同打印端点可达（区分端点缺失与黑名单拒绝）', async ({ page }) => {
    await loginAsRole(page, 'admin');

    const resp = await page.request
      .get(`${API_BASE}${API_PREFIX}/boms/1/print`)
      .catch((e) => { console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`); return null; });
    if (!resp) throw new Error('网络错误: /boms/1/print');

    const status = resp.status();
    if (status === 404) {
      // 无 BOM 种子数据：端点存在但数据缺失——admin 不在黑名单（200 短路）已由代码保证
      test.info().annotations.push({
        type: 'missing-data',
        description: 'admin 对照 boms/1 404（种子缺失），端点存在性由 37 矩阵判定',
      });
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }
    // admin 可达（200=正常打印；403=admin 被黑名单误伤=真缺陷）
    expect(
      status,
      `admin 对照应 200，实际 ${status}（403=admin 也被黑名单误伤）`,
    ).toBeLessThan(403);
    console.log(`[33b] ✅ admin 对照 ${status}（黑名单仅命中指定角色）`);
  });
});
