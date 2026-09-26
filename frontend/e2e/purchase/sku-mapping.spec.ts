import { test, expect } from '../diagnose-fixture';
import {
  loginAsRole,
  apiCallExpectFail,
  apiCallRaw,
  expectDenied,
  API_PREFIX,
} from '../flow/helpers';
import { safeGoto } from '../flow/ui-helpers';

/**
 * 供应商商品色号对照表（sku-mapping / 面料二批调货）e2e
 *
 * 打真实 server + 真实 PostgreSQL（globalSetup 已 migrate + 建角色/账号 + 业务种子）。
 *
 * 覆盖（对应编排任务 B/A6/A7）：
 * - B-列表契约：采购角色 GET /purchase/sku-mappings（含 keyword 过滤参数）→ 200 且分页形状。
 * - validate_refs 真实 HTTP：用真实 product/supplier + 不存在的 supplier_product_id 创建 →
 *   400 VALIDATION_ERROR（证明归属校验在真实链路上生效）。
 * - resolve 无映射：GET /purchase/sku-mappings/resolve → 400 BUSINESS_ERROR（中性拒绝，非 500）。
 * - A6 保密：sales_rep 访问对照表/resolve 端点 → 403；销售订单列表/详情响应体 key 递归扫描
 *   不含任何 supplier_ 前缀键（供应商编号/色号绝不对销售暴露）。
 * - A7/B4 角色矩阵：admin 可达对照表（区分「权限拒绝」与「端点缺失」）；purchase_clerk 走
 *   维护页时「新建」按钮可见（授予 create），sales_rep 无该页/入口。
 *
 * 已知不可端到端覆盖（见交付报告「缺口」）：
 *   本仓无 supplier_products / supplier_product_colors 的创建端点，e2e 亦无 DB 直写通道，
 *   故「维护一条完整对照 → 采购新建 PO 预览带出供应商编号/色号 → 提交成功并回填明细快照」
 *   这条 happy-path 无法在 e2e 造出前置数据；该链路由后端源码结构契约
 *   (tests/sku_mapping_contract_test.rs) + resolve/validate 错误路径 e2e 联合保证。
 */

/** 递归收集对象/数组中出现的所有 key（用于销售响应体 supplier_ 泄露扫描）。 */
function collectKeys(node: unknown, out: Set<string>): void {
  if (Array.isArray(node)) {
    for (const el of node) collectKeys(el, out);
  } else if (node && typeof node === 'object') {
    for (const [k, v] of Object.entries(node as Record<string, unknown>)) {
      out.add(k);
      collectKeys(v, out);
    }
  }
}

/** 取一个真实产品 id（globalSeed 保证至少存在一个产品）。 */
async function firstProductId(page: import('@playwright/test').Page): Promise<number | undefined> {
  const data = await apiCallRaw<{ items?: Array<{ id: number }> }>(
    page,
    'GET',
    `${API_PREFIX}/products?page=1&page_size=1`
  );
  return data?.items?.[0]?.id;
}

/** 取一个真实供应商 id。 */
async function firstSupplierId(page: import('@playwright/test').Page): Promise<number | undefined> {
  const data = await apiCallRaw<{ items?: Array<{ id: number }> }>(
    page,
    'GET',
    `${API_PREFIX}/purchase/suppliers?page=1&page_size=1`
  );
  return data?.items?.[0]?.id;
}

test.describe('SKU 对照表 - 采购维护契约与保密矩阵', () => {
  test('采购角色列表查询支持 keyword 过滤且返回分页形状', async ({ page }) => {
    await loginAsRole(page, 'purchase_clerk');
    // keyword 为我方色号模糊搜索入口（高基数场景）；空结果亦应 200 且带 items 数组。
    const data = await apiCallRaw<{ items: unknown[]; total: number; page: number }>(
      page,
      'GET',
      `${API_PREFIX}/purchase/sku-mappings?page=1&page_size=20&keyword=E2E-GC`
    );
    expect(Array.isArray(data.items), 'data.items 必须是数组（分页契约）').toBe(true);
    expect(typeof data.total, 'data.total 必须是数字').toBe('number');
    expect(data.page, 'page 回显应为 1').toBe(1);
  });

  test('validate_refs：真实 product+supplier 但供应商商品不存在 → 400 VALIDATION_ERROR', async ({
    page,
  }) => {
    await loginAsRole(page, 'purchase_clerk');
    const productId = await firstProductId(page);
    const supplierId = await firstSupplierId(page);
    test.skip(productId === undefined || supplierId === undefined, '前置产品/供应商种子缺失');

    // supplier_product_id 指向不存在记录 → validate_refs 命中「供应商商品 ID 不存在」→ validation
    const res = await apiCallExpectFail(page, 'POST', '/purchase/sku-mappings', {
      product_id: productId,
      product_color_id: null,
      supplier_id: supplierId,
      supplier_product_id: 999_999,
      supplier_product_color_id: null,
      priority: 1,
    });
    expect(res.status, 'validate_refs 失败应为 4xx，实际非 4xx').toBeGreaterThanOrEqual(400);
    expect(res.status).toBeLessThan(500);
    expect(
      res.code,
      `validate_refs 应返回 VALIDATION_ERROR（本仓映射 HTTP 400），实际 code=${res.code}`
    ).toBe('VALIDATION_ERROR');
  });

  test('resolve 无映射 → 400 BUSINESS_ERROR（中性拒绝，非 500）', async ({ page }) => {
    await loginAsRole(page, 'purchase_clerk');
    const productId = await firstProductId(page);
    const supplierId = await firstSupplierId(page);
    test.skip(productId === undefined || supplierId === undefined, '前置产品/供应商种子缺失');

    const res = await apiCallExpectFail(
      page,
      'GET',
      `/purchase/sku-mappings/resolve?product_id=${productId}&supplier_id=${supplierId}`
    );
    // 无映射时 handler 走 AppError::business → 400 BUSINESS_ERROR（而非 500 / 而非 200）
    expect(res.status, 'resolve 无映射应 400').toBe(400);
    expect(res.code).toBe('BUSINESS_ERROR');
    // 出参脱敏：真实指路文案不外显（保密边界，销售可见面同理）
    expect(
      res.message ?? '',
      'resolve 出参 message 应被脱敏，不泄露对照细节'
    ).not.toMatch(/供应商|对照|supplier/);
  });
});

test.describe('SKU 对照表 - 销售域保密与角色矩阵', () => {
  for (const salesRole of ['sales_rep', 'sales_manager']) {
    test(`${salesRole} 访问对照表端点全部 403`, async ({ page }) => {
      await loginAsRole(page, salesRole);

      const list = await apiCallExpectFail(
        page,
        'GET',
        '/purchase/sku-mappings?page=1&page_size=5'
      );
      expectDenied(list, `${salesRole} GET 对照表应 403`);

      const create = await apiCallExpectFail(page, 'POST', '/purchase/sku-mappings', {
        product_id: 1,
        supplier_id: 1,
        supplier_product_id: 1,
      });
      expectDenied(create, `${salesRole} POST 对照表应 403`);

      const resolve = await apiCallExpectFail(
        page,
        'GET',
        '/purchase/sku-mappings/resolve?product_id=1&supplier_id=1'
      );
      expectDenied(resolve, `${salesRole} resolve 应 403`);
    });

    test(`${salesRole} 销售订单响应体不含任何 supplier_ 前缀键（模型保密）`, async ({ page }) => {
      await loginAsRole(page, salesRole);
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
        page,
        'GET',
        '/sales/orders?page=1&page_size=5'
      );
      const keys = new Set<string>();
      collectKeys(list, keys);

      const first = list?.items?.[0];
      if (first?.id) {
        const detail = await apiCallRaw<unknown>(page, 'GET', `/sales/orders/${first.id}`);
        collectKeys(detail, keys);
      }

      const leaked = [...keys].filter(k => k.toLowerCase().includes('supplier'));
      expect(
        leaked,
        `销售域响应体出现 supplier_ 键（调货模型泄露）: ${leaked.join(', ')}`
      ).toEqual([]);
    });
  }

  test('admin 对照：同端点可达（区分端点缺失与权限拒绝）', async ({ page }) => {
    await loginAsRole(page, 'admin');
    const res = await page.request.get(
      `${process.env.API_BASE || 'http://localhost:8082'}${API_PREFIX}/purchase/sku-mappings?page=1&page_size=5`
    );
    expect(res.status(), 'admin 应可访问对照表（<400）').toBeLessThan(400);
  });
});

test.describe('SKU 对照表 - 前端路由与菜单矩阵', () => {
  test('purchase_clerk 可达维护页且「新建」按钮可见（授予 create）', async ({ page }) => {
    await loginAsRole(page, 'purchase_clerk');
    await safeGoto(page, '/purchase/sku-mapping');
    await page.waitForTimeout(500);
    // 维护页头部新建按钮（SKU_MAPPING_CREATE 权限门控）
    const createBtn = page.getByRole('button', { name: /新建|新增/ }).first();
    await expect(createBtn).toBeVisible({ timeout: 15000 });
  });

  test('sales_rep 无对照表入口：直接导航被路由守卫拦截（不渲染维护页表格）', async ({ page }) => {
    await loginAsRole(page, 'sales_rep');
    await safeGoto(page, '/purchase/sku-mapping');
    await page.waitForTimeout(800);
    // 无 sku-mappings:read → 路由守卫不放行：页面不应出现对照表专属的新建按钮/维护表格容器
    const createBtn = page.getByRole('button', { name: /新建对照|新增对照/ });
    await expect(createBtn).toHaveCount(0);
    // 且未停留在对照表标题页
    const heading = page.locator('h2', { hasText: /SKU 对照|对照表/ });
    await expect(heading).toHaveCount(0);
  });
});
