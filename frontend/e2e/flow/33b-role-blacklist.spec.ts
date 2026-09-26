import { test, expect } from '../diagnose-fixture';
import { loginAsRole, apiCall, apiCallRaw, API_BASE, API_PREFIX } from './helpers';
import { pickListArray } from './ui-helpers';

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

/** 黑名单角色：持 print/export 权限码仍必须被拒 */
const BLACKLIST_ROLES = ['customer', 'temporary'];

test.describe('33b 角色黑名单（print/export/dye-recipe）', () => {
  /**
   * 打印黑名单断言的目标 BOM：由 admin 在本组用例前真实创建。
   * 此前打死 boms/1，种子库无该 BOM 时返回 404 → test.skip 静默跳过，
   * 黑名单是否生效从未被验证（假绿）。
   */
  let printBomId = 0;
  let printBomDiag = '';

  test.beforeAll(async ({ browser }) => {
    const page = await browser.newPage();
    try {
      await loginAsRole(page, 'admin');
      // /products：product_handler.rs:247 list_products → ApiResponse<PaginatedResponse> → data={items}。
      // 单一形状直读；原 `Array.isArray(products)?products:(products?.items??[])` 双形状探测会把
      // 端点形状漂移（裸数组/items/list 任一）静默吸收成"0 条"，掩盖契约变更。
      const products = await apiCallRaw<unknown>(page, 'GET', '/products?page=1&page_size=1');
      const productList = pickListArray<{ id: number }>(products, 'items', '33b 前置产品列表');
      // productList[0]?.id 在空列表时运行期为 undefined（noUncheckedIndexedAccess
      // 关闭使索引结果被误判为 number），显式标注可选，兜底创建后由下方 throw 判空
      let productId: number | undefined = productList[0]?.id;
      if (!productId) {
        const created = await apiCall<{ id?: number }>(page, 'POST', '/products', {
          name: `33b黑名单产品${Date.now().toString().slice(-6)}`,
          code: `33B-P${Date.now().toString().slice(-6)}`,
          unit: '米',
          status: 'active',
        });
        productId = created?.data?.id;
      }
      if (!productId) {
        throw new Error(`BOM 前置产品不可用（列表 0 条且创建未返回 id）`);
      }
      const bom = await apiCall<{ bom?: { id?: number }; id?: number }>(page, 'POST', '/boms', {
        product_id: productId,
        version: 1,
        is_default: false,
        status: 'ACTIVE',
        items: [{ material_id: productId, quantity: 1, unit: '米' }],
      });
      printBomId = bom?.data?.bom?.id ?? bom?.data?.id ?? 0;
      if (!printBomId) {
        throw new Error(`POST /boms 未返回 id：${JSON.stringify(bom).slice(0, 200)}`);
      }
      console.log(`[33b] 打印黑名单目标 BOM 已创建 id=${printBomId}`);
    } catch (e) {
      printBomDiag = (e as Error).message;
      console.error(`[33b] ❌ 打印黑名单目标 BOM 创建失败: ${printBomDiag}`);
    } finally {
      await page.close().catch(e => {
        console.warn(`[33b] beforeAll 临时 page 关闭失败: ${(e as Error).message}`);
      });
    }
  });

  for (const role of BLACKLIST_ROLES) {
    test(`${role} 持 boms:print 权限码调用 /boms/{id}/print → 403`, async ({ page }) => {
      await loginAsRole(page, role);
      console.log(`[33b] ${role} 登录成功（凭证据 ensureRoleUsers 补建）`);
      expect(
        printBomId,
        `beforeAll 未建出打印目标 BOM（${printBomDiag || '无诊断信息'}），黑名单断言无法执行`
      ).toBeTruthy();

      const resp = await page.request.get(`${API_BASE}${API_PREFIX}/boms/${printBomId}/print`);

      const status = resp.status();
      expect(
        status,
        `${role} 在 PRINT_DENIED 黑名单，对真实存在的 BOM ${printBomId} 持权限码也应 403，实际 ${status}` +
          (status === 200 ? '（黑名单失效——真缺陷）' : '')
      ).toBe(403);
      console.log(`[33b] ✅ ${role} 打印黑名单生效 → 403`);
    });

    test(`${role} 持 stock:export 权限码调用（/inventory/stock/export） /inventory/stock/export → 403`, async ({
      page,
    }) => {
      await loginAsRole(page, role);

      const resp = await page.request.get(`${API_BASE}${API_PREFIX}/inventory/stock/export`);

      // /inventory/stock/export 是非敏感导出：黑名单是唯一防线——403 即黑名单直接生效，
      // 若 200 放行则 EXPORT_DENIED 黑名单失效（真缺陷）
      const status = resp.status();
      expect(
        status,
        `${role} 在 EXPORT_DENIED 黑名单，应 403，实际 ${status}` +
          (status === 200 ? '（黑名单失效——真缺陷）' : '')
      ).toBe(403);
      console.log(`[33b] ✅ ${role} 导出黑名单生效 → 403`);
    });
  }

  test('manager 调染料配方导出 → 403（DYE_RECIPE 导出禁单）', async ({ page }) => {
    await loginAsRole(page, 'manager');

    const resp = await page.request.get(`${API_BASE}${API_PREFIX}/production/dye-recipes/export`);

    // GET /dye-recipes/export 由 routes/production.rs:135 注册（挂在 /api/v1/erp/production
    // 域下），是列表级导出端点，与种子数据无关——404 只会是路由/前缀缺陷，必须硬失败暴露。
    const status = resp.status();
    expect(status, `manager 在 DYE_RECIPE_EXPORT_DENIED 清单，导出应 403，实际 ${status}`).toBe(
      403
    );
    console.log('[33b] ✅ manager 染料配方导出禁单生效 → 403');
  });

  test('admin 对照：同打印端点可达（区分端点缺失与黑名单拒绝）', async ({ page }) => {
    await loginAsRole(page, 'admin');
    expect(
      printBomId,
      `beforeAll 未建出打印目标 BOM（${printBomDiag || '无诊断信息'}），admin 对照无法执行`
    ).toBeTruthy();

    const resp = await page.request.get(`${API_BASE}${API_PREFIX}/boms/${printBomId}/print`);

    const status = resp.status();
    // admin 不在黑名单：不应 403；BOM 由 beforeAll 真实创建：不应 404；不应 5xx。
    expect(
      status,
      `admin 对真实 BOM ${printBomId} 打印不应被黑名单拒绝（403=admin 也被误伤）`
    ).not.toBe(403);
    expect(status, `admin 打印真实 BOM ${printBomId} 返回 404——BOM 前置失效或路由缺失`).not.toBe(
      404
    );
    expect(status, `admin 打印不应 5xx，实际 ${status}`).toBeLessThan(500);
    console.log(`[33b] ✅ admin 对照 ${status}（黑名单仅命中指定角色）`);
  });
});
