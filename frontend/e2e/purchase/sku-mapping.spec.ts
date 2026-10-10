import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import * as fs from 'fs';
import {
  loginAsRole,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  expectDenied,
  API_BASE,
  API_PREFIX,
  tryCleanup,
} from '../flow/helpers';
import {
  safeGoto,
  formItemByExactLabel,
  pickSelectIn,
  isFieldDisabled,
  getFieldValue,
  fillFieldByLabel,
} from '../flow/ui-helpers';

/**
 * 供应商商品色号对照表（sku-mapping / 面料二批调货）e2e
 *
 * 打真实 server + 真实 PostgreSQL（globalSetup migrate + 建角色/账号 + 业务种子 m0015 演示供应商
 * SUP-DEMO-FAB-01/02 + FAB-P001/P002/P101/P102 + 色号 + 对照表种子）。
 *
 * 本文件覆盖：
 *  A. 对照表 API 契约：keyword 过滤分页形状、validate_refs 真实链路、resolve 无映射中性拒 +
 *     出参脱敏、供应商商品/色号目录 create/duplicate/supplier-not-exist 的真实错误词表。
 *  B. 保密矩阵：salesperson/sales_manager 对 sku-mappings、supplier-products、supplier-product-colors
 *     全 403；销售订单列表/详情响应体递归扫描无任何 supplier_ 前缀键。
 *  C. 角色可达：admin 对照表可达；purchaser 维护页「新增对照」按钮可见；salesperson 直航被路由守卫拦截。
 *  D. 级联 UI：purchaser 供应商侧三级级联（供应商→供应商商品→供应商色号）的「未选上级则下级
 *     disabled」+「切换供应商清空下级」+ 远程搜索出 FAB-P001/PC-A01；purchaser 完整 UI happy-path
 *     端到端选我方产品+色号→级联→保存→列表 keyword 命中（目标角色自证）；admin 同路径亦保留（全权验证）。
 *  E. API 全链路 + 转采购翻译 hook：purchaser 建供应商商品/色号→对照→list→resolve→重复负例（自证）；
 *     转采购有映射→回填生效（未给单价时 unit_price 被 supplier_price 默认，作为可观测代理）；
 *     无映射→中性业务拒「无该色号」且不含保密词；色号在产品下不存在→中性拒（脱敏 business）。
 *
 * 已知不可端到端覆盖（见交付报告「缺口/降级」，均因后端契约客观事实，非本 spec 迁就）：
 *  1. PurchaseOrderItemDto 不回显 supplier_product_code / supplier_color_no 快照列（全仓无任何 GET 端点回读），
 *     故 hook「回填」不能直接断言该两列，改以「未给单价 → item.unit_price 被对照表 supplier_price 默认」
 *     这一真实可观测副作用证明翻译确已发生，并另用 resolve 复现映射内容。快照列不可读缺口已如实上报。
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

/** 保密负向：外显文案不得出现的调货/供应商内部实现词。 */
const FORBIDDEN_SECRETS = /供应商|对照|supplier|调货|自制/i;

// 下拉/字段交互统一委托 flow/ui-helpers 唯一事实源（本文件旧内联 formItem/chooseOption/
// v2IsDisabled/fieldInputValue/fillFieldByLabel 已删除，改调共享 API）：
//  - 我方产品 / 供应商（el-select，本地 filterable）→ pickSelectIn（传 query 触发过滤 + optionText 选目标）
//  - 我方色号 / 供应商商品 / 供应商色号（el-select-v2 remote，高基数）→ 亦用 pickSelectIn：
//      Element Plus 2.14.4 下 el-select-v2 与 el-select 共用 `useNamespace('select')`，
//      触发器真实类名同为 `.el-select__wrapper`（不存在 `.el-select-v2__wrapper`），
//      option 同为 `.el-select-dropdown__item`；pickSelectIn 已按真实渲染类名定位，
//      并以 query 走 keyboard.type 触发 remote-method + 等 loading 收，等价于远程搜索选择。
//      （共享 helper pickV2Remote/pickV2In 现按 `.el-select-v2__wrapper` 定位，本 EP 版本永不命中，
//       属 ui-helpers 侧缺陷，已单独上报编排者，不在本 spec 内私改。）
//  - 级联禁用态 → isFieldDisabled；只读回显值 → getFieldValue；文本填写 → fillFieldByLabel
//  - 需要 form-item 本身下钻（如读 el-select-v2 根块 .el-select 的 innerText）→ formItemByExactLabel
// 共享 helper 均以 root 作用域 + `^label$` 精确锚定、无静默 catch，与旧内联实现语义等价但更健壮。

// ---------------------------------------------------------------------------
// 真实数据发现（以 admin 会话执行——需跨产品/色号/供应商/商品/色号目录做全量取数）
// 注：products:read 已授采购三角色，purchaser 亦可自行调用；此处沿用 admin 会话以简化复用。
// ---------------------------------------------------------------------------

interface DemoFixture {
  productId: number;
  productCode: string;
  productColorId: number;
  colorNo: string;
  sup1Id: number; // SUP-DEMO-FAB-01
  sup1Name: string;
  sup2Id: number; // SUP-DEMO-FAB-02
  sp1Id: number; // FAB-P001
  sp1Code: string;
  sc1Id: number; // PC-A01（FAB-P001 下）
  sc1No: string;
}

interface ProductColor {
  id: number;
  color_no: string;
}

/**
 * admin 会话下发现：一个带色号的真实我方产品 + 演示供应商 SUP-DEMO-FAB-01/02 及其 FAB-P001/PC-A01。
 * 找不到即抛错——暴露 m0015 种子缺失的真实环境问题，绝不静默兜底成假 ID。
 */
async function discoverDemoFixture(page: Page): Promise<DemoFixture> {
  const prods = await apiCallRaw<{ items: Array<{ id: number; code?: string; name?: string }> }>(
    page,
    'GET',
    '/products?page=1&page_size=50'
  );
  let productId = 0;
  let productCode = '';
  let productColorId = 0;
  let colorNo = '';
  for (const p of prods?.items ?? []) {
    const colors = await apiCallRaw<ProductColor[]>(page, 'GET', `/products/${p.id}/colors`);
    const c = colors?.[0];
    if (c?.id) {
      productId = p.id;
      productCode = String(p.code ?? p.id);
      productColorId = c.id;
      colorNo = c.color_no;
      break;
    }
  }
  if (!productId || !productColorId) {
    throw new Error('[discoverDemoFixture] 未找到任何带色号的真实产品（globalSeed 产品/色号缺失）');
  }

  const sups = await apiCallRaw<{
    items: Array<{ id: number; supplier_code?: string; supplier_name?: string }>;
  }>(page, 'GET', '/purchase/suppliers?page=1&page_size=200');
  const sup1 = sups?.items?.find(s => s.supplier_code === 'SUP-DEMO-FAB-01');
  const sup2 = sups?.items?.find(s => s.supplier_code === 'SUP-DEMO-FAB-02');
  if (!sup1?.id || !sup2?.id) {
    throw new Error(
      `[discoverDemoFixture] 未按 supplier_code 找到演示供应商（m0015 未生效）: sup1=${sup1?.id} sup2=${sup2?.id}`
    );
  }

  const products = await apiCallRaw<{
    items: Array<{ id: number; product_code?: string }>;
  }>(page, 'GET', `/purchase/supplier-products?supplier_id=${sup1.id}&page=1&page_size=200`);
  const sp1 = products?.items?.find(p => p.product_code === 'FAB-P001');
  if (!sp1?.id) {
    throw new Error('[discoverDemoFixture] 演示供应商 SUP-DEMO-FAB-01 下无 FAB-P001');
  }
  const colors = await apiCallRaw<{ items: Array<{ id: number; color_no?: string }> }>(
    page,
    'GET',
    `/purchase/supplier-product-colors?supplier_product_id=${sp1.id}&page=1&page_size=200`
  );
  const sc1 = colors?.items?.find(c => c.color_no === 'PC-A01');
  if (!sc1?.id) {
    throw new Error('[discoverDemoFixture] FAB-P001 下无色号 PC-A01');
  }

  return {
    productId,
    productCode,
    productColorId,
    colorNo,
    sup1Id: sup1.id,
    sup1Name: String(sup1.supplier_name ?? ''),
    sup2Id: sup2.id,
    sp1Id: sp1.id,
    sp1Code: 'FAB-P001',
    sc1Id: sc1.id,
    sc1No: 'PC-A01',
  };
}

// ===========================================================================
// A. 对照表 API 契约 + validate_refs + resolve 脱敏
// ===========================================================================

test.describe('SKU 对照表 - 采购维护契约与保密矩阵', () => {
  test('采购角色列表查询支持 keyword 过滤且返回分页形状', async ({ page }) => {
    await loginAsRole(page, 'purchaser');
    const data = await apiCallRaw<{ items: unknown[]; total: number; page: number }>(
      page,
      'GET',
      // apiCall/apiCallRaw 内部已拼 `${API_BASE}${API_PREFIX}`，此处只传相对路径——
      // 旧写法多拼 ${API_PREFIX} 造成双前缀 → permission.rs:65 判"未知的资源路径" FORBIDDEN（假红）。
      '/purchase/sku-mappings?page=1&page_size=20&keyword=E2E-GC'
    );
    expect(Array.isArray(data.items), 'data.items 必须是数组（分页契约）').toBe(true);
    expect(typeof data.total, 'data.total 必须是数字').toBe('number');
    expect(data.page, 'page 回显应为 1').toBe(1);
  });

  test('validate_refs：真实 product+supplier 但供应商商品不存在 → 400 VALIDATION_ERROR', async ({
    page,
  }) => {
    // 先 admin 发现真实 product/supplier（全量取数复用 admin 会话），再切回 clerk 发写请求。
    await loginAsRole(page, 'admin');
    const fx = await discoverDemoFixture(page);
    await loginAsRole(page, 'purchaser');

    // supplier_product_id 指向不存在记录 → validate_refs「供应商商品 ID 不存在」→ validation
    const res = await apiCallExpectFail(page, 'POST', '/purchase/sku-mappings', {
      product_id: fx.productId,
      product_color_id: null,
      supplier_id: fx.sup1Id,
      supplier_product_id: 999_999,
      supplier_product_color_id: null,
      priority: 1,
    });
    expect(
      res.status,
      `validate_refs 失败应为 4xx，实际 status=${res.status}`
    ).toBeGreaterThanOrEqual(400);
    expect(res.status).toBeLessThan(500);
    expect(
      res.code,
      `validate_refs 应返回 VALIDATION_ERROR（本仓映射 HTTP 400），实际 code=${res.code}`
    ).toBe('VALIDATION_ERROR');
  });

  test('resolve 无映射 → 400 BUSINESS_ERROR（中性拒绝，非 500）', async ({ page }) => {
    await loginAsRole(page, 'admin');
    const fx = await discoverDemoFixture(page);
    // 选一个对 (productId, color IS NULL) 无映射的供应商：用 SUP-DEMO-FAB-02（种子只给 01 建了映射，
    // 且 resolve 未带 color_id → 查 product_color_id IS NULL 的行，演示库无此类映射）。
    await loginAsRole(page, 'purchaser');
    const res = await apiCallExpectFail(
      page,
      'GET',
      `/purchase/sku-mappings/resolve?product_id=${fx.productId}&supplier_id=${fx.sup2Id}`
    );
    expect(res.status, `resolve 无映射应 400，实际 ${res.status}`).toBe(400);
    expect(res.code).toBe('BUSINESS_ERROR');
    // 出参脱敏（AppError::business → "业务处理失败"），不泄露对照细节/供应商字样
    expect(res.message ?? '', `resolve 出参 message 应被脱敏，实际="${res.message}"`).not.toMatch(
      FORBIDDEN_SECRETS
    );
  });
});

// ===========================================================================
// B/C. 保密矩阵（含供应商目录 403）+ admin 可达 + 路由守卫
// ===========================================================================

test.describe('SKU 对照表 - 销售域保密与角色矩阵', () => {
  for (const salesRole of ['salesperson', 'sales_manager']) {
    test(`${salesRole} 访问对照表/供应商目录端点全部 403`, async ({ page }) => {
      await loginAsRole(page, salesRole);

      const mappingList = await apiCallExpectFail(
        page,
        'GET',
        '/purchase/sku-mappings?page=1&page_size=5'
      );
      expectDenied(mappingList, `${salesRole} GET 对照表应 403`);

      const mappingCreate = await apiCallExpectFail(page, 'POST', '/purchase/sku-mappings', {
        product_id: 1,
        supplier_id: 1,
        supplier_product_id: 1,
      });
      expectDenied(mappingCreate, `${salesRole} POST 对照表应 403`);

      const resolve = await apiCallExpectFail(
        page,
        'GET',
        '/purchase/sku-mappings/resolve?product_id=1&supplier_id=1'
      );
      expectDenied(resolve, `${salesRole} resolve 应 403`);

      // 保密强化：供应商商品目录 / 供应商色号目录亦不得对销售暴露
      const products = await apiCallExpectFail(
        page,
        'GET',
        '/purchase/supplier-products?supplier_id=1&page=1&page_size=5'
      );
      expectDenied(products, `${salesRole} GET supplier-products 应 403`);

      const colors = await apiCallExpectFail(
        page,
        'GET',
        '/purchase/supplier-product-colors?supplier_product_id=1&page=1&page_size=5'
      );
      expectDenied(colors, `${salesRole} GET supplier-product-colors 应 403`);
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
      expect(leaked, `销售域响应体出现 supplier_ 键（调货模型泄露）: ${leaked.join(', ')}`).toEqual(
        []
      );
    });
  }

  test('admin 对照：同端点全部 200 可达（区分端点缺失与权限拒绝）', async ({ page }) => {
    await loginAsRole(page, 'admin');
    // 强化到严格 200（非 <400）——与 salesperson/sales_manager 侧 403 断言形成"权限矩阵"
    // 真值表：admin 全权 → 200；销售角色无 purchase 权限 → 403。避免 204/3xx 蒙混。
    const mappingList = await page.request.get(
      `${API_BASE}${API_PREFIX}/purchase/sku-mappings?page=1&page_size=5`
    );
    expect(mappingList.status(), 'admin GET 对照表应 200').toBe(200);

    const products = await page.request.get(
      `${API_BASE}${API_PREFIX}/purchase/supplier-products?supplier_id=1&page=1&page_size=5`
    );
    expect(products.status(), 'admin GET 供应商商品目录应 200').toBe(200);

    const colors = await page.request.get(
      `${API_BASE}${API_PREFIX}/purchase/supplier-product-colors?supplier_product_id=1&page=1&page_size=5`
    );
    expect(colors.status(), 'admin GET 供应商色号目录应 200').toBe(200);

    // resolve：无映射时后端返 400 BUSINESS_ERROR（见 A 段 resolve 用例）。
    // 这里断 admin 有权限（≠403），端点存在性由 4xx 侧证；403 与 400/404 的语义
    // 区分：403=权限拒，400=业务拒（无映射），404=路径不存在。admin 只应 403/404 二选一被排除。
    const resolve = await page.request.get(
      `${API_BASE}${API_PREFIX}/purchase/sku-mappings/resolve?product_id=1&supplier_id=1`
    );
    expect(
      resolve.status(),
      `admin resolve 应不为 403（权限拒）/非 404（路径缺失），实际 ${resolve.status()}`
    ).not.toBe(403);
    expect(resolve.status()).not.toBe(404);
  });
});

test.describe('SKU 对照表 - 前端路由与菜单矩阵', () => {
  test('purchaser 可达维护页且「新增对照」按钮可见（授予 create）', async ({ page }) => {
    await loginAsRole(page, 'purchaser');
    await safeGoto(page, '/purchase/sku-mapping');
    await page.waitForTimeout(500);
    const createBtn = page.getByRole('button', { name: /新增对照/ }).first();
    await expect(createBtn).toBeVisible({ timeout: 15000 });
  });

  test('salesperson 无对照表入口：直接导航被路由守卫拦截（不渲染维护页表格）', async ({ page }) => {
    await loginAsRole(page, 'salesperson');
    await safeGoto(page, '/purchase/sku-mapping');
    await page.waitForTimeout(800);
    const createBtn = page.getByRole('button', { name: /新增对照|新增|新建/ });
    await expect(createBtn).toHaveCount(0);
    const heading = page.locator('h2', { hasText: /SKU 对照|SKU对照|对照表/ });
    await expect(heading).toHaveCount(0);
  });
});

// ===========================================================================
// D1. purchaser 供应商侧级联：未选上级禁用 + 切换供应商清空下级 + 远程搜索
// ===========================================================================

test.describe('SKU 对照表 - 级联交互（purchaser 供应商侧）', () => {
  test('供应商商品/色号级联：未选供应商则下级 disabled；选供应商后出 FAB-P001、选商品后出 PC-A01；切换供应商清空下级', async ({
    page,
  }) => {
    await loginAsRole(page, 'admin');
    const fx = await discoverDemoFixture(page);
    await loginAsRole(page, 'purchaser');

    await safeGoto(page, '/purchase/sku-mapping');
    await page
      .getByRole('button', { name: /新增对照/ })
      .first()
      .click();
    const dialog = page.locator('.el-dialog:visible').last();
    await dialog.waitFor({ state: 'visible', timeout: 15_000 });
    await page.waitForTimeout(300);

    // (1) 未选供应商：供应商商品、供应商色号两级 el-select-v2 均 disabled（可用性/保密门禁）
    expect(
      await isFieldDisabled(dialog, '供应商商品'),
      '未选供应商时「供应商商品」应 disabled'
    ).toBe(true);
    expect(await isFieldDisabled(dialog, '供应商色号'), '未选商品时「供应商色号」应 disabled').toBe(
      true
    );

    // (2) 选供应商=演示甲 → 供应商商品变可用；远程搜索出 FAB-P001
    await pickSelectIn(dialog, page, '供应商', { query: fx.sup1Name, optionText: fx.sup1Name });
    expect(await isFieldDisabled(dialog, '供应商商品'), '选供应商后「供应商商品」应解禁').toBe(
      false
    );
    await pickSelectIn(dialog, page, '供应商商品', { query: 'FAB-P001', optionText: 'FAB-P001' });

    // 供应商商品编码只读回显 FAB-P001
    expect(await getFieldValue(dialog, '供应商品编码')).toContain('FAB-P001');
    // 选商品后供应商色号解禁
    expect(await isFieldDisabled(dialog, '供应商色号'), '选商品后「供应商色号」应解禁').toBe(false);
    await pickSelectIn(dialog, page, '供应商色号', { query: 'PC-A01', optionText: 'PC-A01' });
    expect(await getFieldValue(dialog, '供应商色号编号')).toContain('PC-A01');

    // (3) 切换供应商 → 清空下级：改选演示乙后，供应商商品/编码/色号回显全部清空
    await pickSelectIn(dialog, page, '供应商', { query: '演示乙', optionText: '演示乙' });
    expect(await getFieldValue(dialog, '供应商品编码'), '切换供应商后供应商品编码应被清空').toBe(
      ''
    );
    expect(await getFieldValue(dialog, '供应商色号编号'), '切换供应商后供应商色号应被清空').toBe(
      ''
    );
    // 清空下级后「供应商商品」仍可用（供应商已选），但其值应为空（下拉无选中项文本回显）
    // 用共享 formItemByExactLabel（精确锚定，无静默）取 el-select-v2 的根块类文本
    // （EP 2.14.x 下 el-select-v2 根 class 为 .el-select，不存在 .el-select-v2 类块）。
    const spText = await formItemByExactLabel(dialog, '供应商商品')
      .locator('.el-select')
      .first()
      .innerText();
    expect(spText.includes('FAB-P001'), '切换供应商后不应仍残留旧商品').toBe(false);

    // 本用例只验证级联交互行为（禁用/解禁/清空），不提交（完整保存见下方 purchaser 完整级联用例）
    await dialog.getByRole('button', { name: /取消/ }).first().click();
  });
});

// ===========================================================================
// D2. admin 完整 UI happy-path 建对照 → 列表 keyword 按我方色号命中
//     （admin 全权路径有效；purchaser 自证路径见 D2b）
// ===========================================================================

test.describe('SKU 对照表 - 完整 UI happy-path（admin 全权路径）', () => {
  test('新增对照：选我方产品+色号→级联选演示供应商/商品/色号→填协议价→保存→列表命中', async ({
    page,
  }) => {
    await loginAsRole(page, 'admin');
    const fx = await discoverDemoFixture(page);

    // 规避与 globalSeed 对照种子（product_seed×SUP-DEMO-FAB-01）的 (product,color,supplier) 唯一键冲突：
    // 先删掉该我方产品+色号+SUP1 已存在的对照（admin 有 delete 权），保证 UI 新建不撞唯一约束。
    const preList = await apiCallRaw<{ items: Array<{ id: number; product_color_id?: number }> }>(
      page,
      'GET',
      `/purchase/sku-mappings?product_id=${fx.productId}&supplier_id=${fx.sup1Id}&page=1&page_size=200`
    );
    for (const m of preList?.items ?? []) {
      if (m.product_color_id === fx.productColorId) {
        await tryCleanup(page, 'DELETE', `/purchase/sku-mappings/${m.id}`, 'UI前置清理对照');
      }
    }

    await safeGoto(page, '/purchase/sku-mapping');
    await page
      .getByRole('button', { name: /新增对照/ })
      .first()
      .click();
    const dialog = page.locator('.el-dialog:visible').last();
    await dialog.waitFor({ state: 'visible', timeout: 15_000 });
    await page.waitForTimeout(300);

    // 我方产品（el-select 本地 filterable）→ pickSelectIn（query 触发过滤 + optionText 选目标）；
    // 我方色号（el-select-v2 remote，EP 触发器类名同为 .el-select__wrapper）→ pickSelectIn + query 触发远程
    await pickSelectIn(dialog, page, '我方产品', {
      query: fx.productCode,
      optionText: fx.productCode,
    });
    await pickSelectIn(dialog, page, '我方色号', { query: fx.colorNo, optionText: fx.colorNo });
    // 只读展示我方色号编号 == 选中的 colorNo
    expect(await getFieldValue(dialog, '我方色号编号')).toContain(fx.colorNo);

    // 供应商三级级联（均 el-select-v2 remote）
    await pickSelectIn(dialog, page, '供应商', { query: fx.sup1Name, optionText: fx.sup1Name });
    await pickSelectIn(dialog, page, '供应商商品', { query: 'FAB-P001', optionText: 'FAB-P001' });
    await pickSelectIn(dialog, page, '供应商色号', { query: 'PC-A01', optionText: 'PC-A01' });

    // 协议价
    await fillFieldByLabel(dialog, page, '协议价', '66.60');

    // 提交并捕获创建响应 id（跳过 CSRF 竞败的 403 中间态）。
    // 对话框主按钮走 t('common.confirm')，真实文案为「确认」（非「确定」）——
    // 本仓 common.confirm=「确认」被 23 个视图与 ui-helpers.submitDialog 默认词表共用；
    // 旧正则 /确定|保存|提交/ 漏掉「确认」→ .last() 零命中 → 点击 30s 超时、POST 从不发出（假红）。
    // 锚定到对话框作用域后仅「确认」命中，无「取消」子串歧义（取消不含上述任一词）。
    const respPromise = page.waitForResponse(
      r =>
        r.url().includes('/purchase/sku-mappings') &&
        r.request().method() === 'POST' &&
        r.status() !== 403,
      { timeout: 30_000 }
    );
    await dialog
      .getByRole('button', { name: /确定|确认|保存|提交/ })
      .last()
      .click();
    const resp = await respPromise;
    const created = (await resp.json().catch(() => null)) as {
      code?: number;
      data?: { id?: number };
    } | null;
    expect(resp.status(), `UI 新建对照应成功，实际 HTTP ${resp.status()}`).toBeLessThan(400);
    const createdId = created?.data?.id;
    expect(typeof createdId, '创建响应应回 data.id').toBe('number');

    // 列表按我方色号 keyword（API）命中该新建对照，供应商侧编码一致
    const listed = await apiCallRaw<{
      items: Array<{
        id: number;
        color_no?: string;
        supplier_product_code?: string;
        supplier_color_no?: string;
      }>;
    }>(
      page,
      'GET',
      `/purchase/sku-mappings?keyword=${encodeURIComponent(fx.colorNo)}&page=1&page_size=200`
    );
    const hit = (listed?.items ?? []).find(x => x.id === createdId);
    expect(hit, `keyword=${fx.colorNo} 未命中新建对照 id=${createdId}`).toBeTruthy();
    expect(hit?.supplier_product_code).toBe('FAB-P001');
    expect(hit?.supplier_color_no).toBe('PC-A01');

    // 清理，避免残留与后续运行唯一键冲突
    await tryCleanup(page, 'DELETE', `/purchase/sku-mappings/${createdId}`, 'UI用例后清理对照');
  });
});

// ===========================================================================
// D2b. purchaser 完整 UI happy-path 建对照 → 列表 keyword 按我方色号命中
//     目标角色自证：products:read 已授采购三角色（permission.rs:395），
//     purchaser 可端到端在维护页选我方产品+色号→供应商三级级联→保存。
// ===========================================================================

test.describe('SKU 对照表 - 完整 UI happy-path（purchaser 自证）', () => {
  test('purchaser 新增对照：选我方产品+色号→级联供应商/商品/色号→协议价→保存→列表命中', async ({
    page,
  }) => {
    // 前置数据发现（需跨产品/色号/供应商/商品/色号目录做全量取数，沿用 admin 会话）
    await loginAsRole(page, 'admin');
    const fx = await discoverDemoFixture(page);

    // 规避唯一键冲突：先清理该 (product, color, supplier) 可能已存在的对照
    const preList = await apiCallRaw<{ items: Array<{ id: number; product_color_id?: number }> }>(
      page,
      'GET',
      `/purchase/sku-mappings?product_id=${fx.productId}&supplier_id=${fx.sup1Id}&page=1&page_size=200`
    );
    for (const m of preList?.items ?? []) {
      if (m.product_color_id === fx.productColorId) {
        await tryCleanup(page, 'DELETE', `/purchase/sku-mappings/${m.id}`, 'D2b前置清理对照');
      }
    }

    // ── 切换到 purchaser，以目标角色身份执行全流程 ──
    await loginAsRole(page, 'purchaser');

    await safeGoto(page, '/purchase/sku-mapping');
    await page
      .getByRole('button', { name: /新增对照/ })
      .first()
      .click();
    const dialog = page.locator('.el-dialog:visible').last();
    await dialog.waitFor({ state: 'visible', timeout: 15_000 });
    await page.waitForTimeout(300);

    // 我方产品（el-select 本地 filterable，经 GET /products 取数——purchaser 已授 products:read）
    await pickSelectIn(dialog, page, '我方产品', {
      query: fx.productCode,
      optionText: fx.productCode,
    });
    // 我方色号（el-select-v2 remote，经 GET /products/:id/colors）
    await pickSelectIn(dialog, page, '我方色号', { query: fx.colorNo, optionText: fx.colorNo });
    // 断言只读回显 == 选中的色号编号
    expect(await getFieldValue(dialog, '我方色号编号')).toContain(fx.colorNo);

    // 供应商三级级联（均 el-select-v2 remote；purchaser 有 suppliers:read、supplier-products:read、
    // supplier-product-colors:read）
    await pickSelectIn(dialog, page, '供应商', { query: fx.sup1Name, optionText: fx.sup1Name });
    await pickSelectIn(dialog, page, '供应商商品', { query: 'FAB-P001', optionText: 'FAB-P001' });
    await pickSelectIn(dialog, page, '供应商色号', { query: 'PC-A01', optionText: 'PC-A01' });

    // 协议价
    await fillFieldByLabel(dialog, page, '协议价', '55.55');

    // 提交并捕获创建响应（purchaser 已授 sku-mappings:create）。
    // 主按钮真实文案「确认」= t('common.confirm')；正则须含「确认」，否则点击零命中超时、POST 不发出。
    const respPromise = page.waitForResponse(
      r =>
        r.url().includes('/purchase/sku-mappings') &&
        r.request().method() === 'POST' &&
        r.status() !== 403,
      { timeout: 30_000 }
    );
    await dialog
      .getByRole('button', { name: /确定|确认|保存|提交/ })
      .last()
      .click();
    const resp = await respPromise;
    const created = (await resp.json().catch(() => null)) as {
      code?: number;
      data?: { id?: number };
    } | null;
    expect(resp.status(), `purchaser 新建对照应成功，实际 HTTP ${resp.status()}`).toBeLessThan(400);
    const createdId = created?.data?.id;
    expect(typeof createdId, '创建响应应回 data.id').toBe('number');

    // 列表按我方色号 keyword 命中该新建对照（purchaser 已授 sku-mappings:read）
    const listed = await apiCallRaw<{
      items: Array<{
        id: number;
        color_no?: string;
        supplier_product_code?: string;
        supplier_color_no?: string;
      }>;
    }>(
      page,
      'GET',
      `/purchase/sku-mappings?keyword=${encodeURIComponent(fx.colorNo)}&page=1&page_size=200`
    );
    const hit = (listed?.items ?? []).find(x => x.id === createdId);
    expect(hit, `keyword=${fx.colorNo} 未命中 purchaser 新建的对照 id=${createdId}`).toBeTruthy();
    expect(hit?.supplier_product_code).toBe('FAB-P001');
    expect(hit?.supplier_color_no).toBe('PC-A01');

    // 清理：purchaser 无 sku-mappings:delete，切 admin 仅做运维清理
    await loginAsRole(page, 'admin');
    await tryCleanup(page, 'DELETE', `/purchase/sku-mappings/${createdId}`, 'D2b用例后清理对照');
  });
});

// ===========================================================================
// E1. API 级 happy-path 全链路（purchaser 自证：建供应商商品→色号→对照→list→resolve→重复拒）
//     purchaser 已授 products:read / supplier-products:read+create+update /
//     supplier-product-colors:read+create+update / sku-mappings:read+create+update，
//     全部步骤（除最终 DELETE 清理外）均可由目标角色完成。
// ===========================================================================

test.describe('SKU 对照表 - API 全链路 happy-path（purchaser 自证）', () => {
  test('purchaser: create supplier-product→color→mapping→list→resolve + 重复/非法负例', async ({
    page,
  }) => {
    // 前置数据发现（跨产品/供应商/目录全量取数，沿用 admin 会话简化复用）
    await loginAsRole(page, 'admin');
    const fx = await discoverDemoFixture(page);

    // ── 切换到 purchaser，以目标角色执行全部业务操作 ──
    await loginAsRole(page, 'purchaser');
    const ts = Date.now().toString().slice(-8);

    let spId = 0;
    let scId = 0;
    let mapId = 0;
    try {
      // 1) 新建供应商商品（purchaser 有 supplier-products:create）
      const sp = await apiCall<{ id?: number }>(page, 'POST', '/purchase/supplier-products', {
        supplier_id: fx.sup1Id,
        product_code: `E2E-SP${ts}`,
        product_name: `E2E供应商品${ts}`,
        unit: '米',
      });
      spId = Number(sp.data?.id);
      expect(spId, 'purchaser 创建供应商商品应回 id').toBeGreaterThan(0);

      // 2) 新建供应商色号（purchaser 有 supplier-product-colors:create）
      const sc = await apiCall<{ id?: number }>(page, 'POST', '/purchase/supplier-product-colors', {
        supplier_product_id: spId,
        color_no: `E2E-SC${ts}`,
        color_name: `E2E供应色号${ts}`,
        extra_cost: '2.50',
      });
      scId = Number(sc.data?.id);
      expect(scId, 'purchaser 创建供应商色号应回 id').toBeGreaterThan(0);
      expect(
        String((sc.data as Record<string, unknown> | undefined)?.extra_cost ?? ''),
        'extra_cost 应回显为字符串承载的 Decimal'
      ).toMatch(/2\.5/);

      // 3) 新建对照（purchaser 有 sku-mappings:create + products:read 前置校验通过）
      const map = await apiCall<{ id?: number; supplier_price?: string }>(
        page,
        'POST',
        '/purchase/sku-mappings',
        {
          product_id: fx.productId,
          product_color_id: fx.productColorId,
          supplier_id: fx.sup1Id,
          supplier_product_id: spId,
          supplier_product_color_id: scId,
          supplier_price: '50.00',
          is_primary: true,
          is_enabled: true,
        }
      );
      mapId = Number(map.data?.id);
      expect(mapId, 'purchaser 创建对照应回 id').toBeGreaterThan(0);

      // 4) list keyword 按我方色号命中，供应方编码回显正确
      const listed = await apiCallRaw<{
        items: Array<{ id: number; supplier_product_code?: string; supplier_color_no?: string }>;
      }>(
        page,
        'GET',
        `/purchase/sku-mappings?keyword=${encodeURIComponent(fx.colorNo)}&page=1&page_size=200`
      );
      const hit = (listed?.items ?? []).find(x => x.id === mapId);
      expect(hit, `keyword 未命中对照 id=${mapId}`).toBeTruthy();
      expect(hit?.supplier_product_code, 'list 富化供应商品编码应为自建编码').toBe(`E2E-SP${ts}`);
      expect(hit?.supplier_color_no, 'list 富化供应色号应为自建色号').toBe(`E2E-SC${ts}`);

      // 5) resolve 命中，返回供应方编码/色号
      const resolved = await apiCallRaw<{
        supplier_product_code?: string;
        supplier_color_no?: string;
        supplier_price?: string | number;
      }>(
        page,
        'GET',
        `/purchase/sku-mappings/resolve?product_id=${fx.productId}&color_id=${fx.productColorId}&supplier_id=${fx.sup1Id}`
      );
      expect(resolved?.supplier_product_code).toBe(`E2E-SP${ts}`);
      expect(resolved?.supplier_color_no).toBe(`E2E-SC${ts}`);
      expect(Number(resolved?.supplier_price), 'resolve 应回协议价').toBeCloseTo(50, 1);

      // 6) 重复 create（同 product+color+supplier 唯一键）→ 400 BUSINESS_ERROR（脱敏）
      const dupMap = await apiCallExpectFail(page, 'POST', '/purchase/sku-mappings', {
        product_id: fx.productId,
        product_color_id: fx.productColorId,
        supplier_id: fx.sup1Id,
        supplier_product_id: spId,
        supplier_product_color_id: scId,
        is_enabled: true,
      });
      expect(dupMap.status, '重复对照应 4xx').toBeGreaterThanOrEqual(400);
      expect(dupMap.status).toBeLessThan(500);
      expect(dupMap.code, `重复对照应 BUSINESS_ERROR，实际 ${dupMap.code}`).toBe('BUSINESS_ERROR');

      // 7) 同供应商重复 product_code → BUSINESS_ERROR（business_displayable，真实文案外显）
      const dupSp = await apiCallExpectFail(page, 'POST', '/purchase/supplier-products', {
        supplier_id: fx.sup1Id,
        product_code: `E2E-SP${ts}`,
        product_name: `重复${ts}`,
        unit: '米',
      });
      expect(dupSp.status).toBeGreaterThanOrEqual(400);
      expect(dupSp.status).toBeLessThan(500);
      expect(dupSp.code, `重复编码应 BUSINESS_ERROR，实际 ${dupSp.code}`).toBe('BUSINESS_ERROR');
      expect(dupSp.message ?? '', '编码重复为可外显文案').toMatch(/已存在|编码/);

      // 8) 供应商不存在 → 400 VALIDATION_ERROR
      const badSup = await apiCallExpectFail(page, 'POST', '/purchase/supplier-products', {
        supplier_id: 999_999,
        product_code: `E2E-BAD${ts}`,
        product_name: '非法供应商',
        unit: '米',
      });
      expect(badSup.status, '供应商不存在应 400').toBe(400);
      expect(badSup.code).toBe('VALIDATION_ERROR');

      // 9) 同商品重复色号 → BUSINESS_ERROR（displayable）
      const dupSc = await apiCallExpectFail(page, 'POST', '/purchase/supplier-product-colors', {
        supplier_product_id: spId,
        color_no: `E2E-SC${ts}`,
        color_name: '重复色号',
        extra_cost: '0',
      });
      expect(dupSc.code, `重复色号应 BUSINESS_ERROR，实际 ${dupSc.code}`).toBe('BUSINESS_ERROR');
      expect(dupSc.message ?? '').toMatch(/已存在|色号/);
    } finally {
      // 清理：purchaser 有 supplier-products/colors 的 update（PUT 停用），但无 sku-mappings:delete。
      // 供应商商品/色号无 DELETE 端点，置停用软清理。
      if (scId)
        await apiCall(page, 'PUT', `/purchase/supplier-product-colors/${scId}`, {
          supplier_product_id: spId,
          color_no: `E2E-SC${ts}`,
          color_name: '停用',
          is_enabled: false,
        }).catch(e => console.warn('[清理] 停用供应色号失败:', (e as Error).message));
      if (spId)
        await apiCall(page, 'PUT', `/purchase/supplier-products/${spId}`, {
          supplier_id: fx.sup1Id,
          product_code: `E2E-SP${ts}`,
          product_name: '停用',
          unit: '米',
          is_enabled: false,
        }).catch(e => console.warn('[清理] 停用供应商品失败:', (e as Error).message));
      // 对照 DELETE 需 admin（purchaser 无此权限），仅做运维清理
      if (mapId) {
        await loginAsRole(page, 'admin');
        await tryCleanup(page, 'DELETE', `/purchase/sku-mappings/${mapId}`, 'E1清理对照(admin)');
      }
    }
  });
});

// ===========================================================================
// E2. 转采购翻译 hook（API 两态：有映射回填 / 无映射中性拒）
//     ——见文件头缺口 1：快照列不可经 API 读回，以 unit_price 默认 + resolve 复现映射为可观测代理。
// ===========================================================================

test.describe('SKU 对照表 - 转采购翻译 hook', () => {
  let warehouseId = 0;
  let departmentId = 0;
  let salesOrderId = 0;

  test.beforeEach(async ({ page }) => {
    await loginAsRole(page, 'admin');
    warehouseId =
      (
        await apiCallRaw<{ items: Array<{ id: number }> }>(
          page,
          'GET',
          '/warehouses?page=1&page_size=1'
        )
      )?.items?.[0]?.id ?? 0;
    departmentId =
      (
        await apiCallRaw<{ items: Array<{ id: number }> }>(
          page,
          'GET',
          '/departments?page=1&page_size=1'
        )
      )?.items?.[0]?.id ?? 0;
    salesOrderId =
      (
        await apiCallRaw<{ items: Array<{ id: number }> }>(
          page,
          'GET',
          '/sales/orders?page=1&page_size=1'
        )
      )?.items?.[0]?.id ?? 0;
  });

  test('有对照：转采购生成 PO 时未给单价→按对照表 supplier_price 默认（回填生效可观测证明）', async ({
    page,
  }) => {
    expect(warehouseId, '前置仓库缺失').toBeGreaterThan(0);
    expect(departmentId, '前置部门缺失').toBeGreaterThan(0);

    const fx = await discoverDemoFixture(page);
    const supPrice = '77.77';
    let mapId = 0;
    let poId = 0;
    try {
      // 清冲突 + 建对照（我方产品色号 → FAB-P001/PC-A01，协议价 supPrice）
      const preList = await apiCallRaw<{ items: Array<{ id: number; product_color_id?: number }> }>(
        page,
        'GET',
        `/purchase/sku-mappings?product_id=${fx.productId}&supplier_id=${fx.sup1Id}&page=1&page_size=200`
      );
      for (const m of preList?.items ?? []) {
        if (m.product_color_id === fx.productColorId)
          await tryCleanup(page, 'DELETE', `/purchase/sku-mappings/${m.id}`, 'hook前置清理');
      }
      const map = await apiCall<{ id?: number }>(page, 'POST', '/purchase/sku-mappings', {
        product_id: fx.productId,
        product_color_id: fx.productColorId,
        supplier_id: fx.sup1Id,
        supplier_product_id: fx.sp1Id,
        supplier_product_color_id: fx.sc1Id,
        supplier_price: supPrice,
        is_primary: true,
        is_enabled: true,
      });
      mapId = Number(map.data?.id);
      expect(mapId).toBeGreaterThan(0);

      // 转采购：带 source_sales_order_id + item.color_no，且不给 unit_price
      const po = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
        supplier_id: fx.sup1Id,
        order_date: new Date().toISOString().slice(0, 10),
        warehouse_id: warehouseId,
        department_id: departmentId,
        source_sales_order_id: salesOrderId || 1,
        items: [{ material_id: fx.productId, quantity_ordered: '10', color_no: fx.colorNo }],
      });
      poId = Number(po.data?.id);
      expect(poId, '有对照时转采购应成功建单').toBeGreaterThan(0);

      // 可观测回填代理：明细单价被对照表 supplier_price 默认（后端仅在缺省单价时覆盖）
      const items = await apiCallRaw<Array<{ unit_price?: string | number }>>(
        page,
        'GET',
        `/purchase/orders/${poId}/items`
      );
      expect(Array.isArray(items) && items.length, 'PO 明细应非空').toBeTruthy();
      expect(
        Number(items[0].unit_price),
        `转采购明细 unit_price 应回填为对照协议价 ${supPrice}，实际 ${items[0].unit_price}`
      ).toBeCloseTo(77.77, 1);

      // 并用 resolve 复现映射内容（供应方编码/色号 = FAB-P001/PC-A01），佐证翻译目标正确
      const resolved = await apiCallRaw<{
        supplier_product_code?: string;
        supplier_color_no?: string;
      }>(
        page,
        'GET',
        `/purchase/sku-mappings/resolve?product_id=${fx.productId}&color_id=${fx.productColorId}&supplier_id=${fx.sup1Id}`
      );
      expect(resolved?.supplier_product_code).toBe('FAB-P001');
      expect(resolved?.supplier_color_no).toBe('PC-A01');
    } finally {
      if (poId) await tryCleanup(page, 'DELETE', `/purchase/orders/${poId}`, '清理转采购PO');
      if (mapId) await tryCleanup(page, 'DELETE', `/purchase/sku-mappings/${mapId}`, '清理对照');
    }
  });

  test('无对照：转采购被中性拒绝（message 含"无该色号"，不含供应商/对照/supplier/调货/自制）', async ({
    page,
  }) => {
    expect(warehouseId, '前置仓库缺失').toBeGreaterThan(0);
    expect(departmentId, '前置部门缺失').toBeGreaterThan(0);

    const fx = await discoverDemoFixture(page);
    // 用 SUP-DEMO-FAB-02（对 (我方产品, 我方色号) 无对照）触发无映射；先软清理任何潜在冲突映射。
    const preList = await apiCallRaw<{ items: Array<{ id: number; product_color_id?: number }> }>(
      page,
      'GET',
      `/purchase/sku-mappings?product_id=${fx.productId}&supplier_id=${fx.sup2Id}&page=1&page_size=200`
    );
    for (const m of preList?.items ?? []) {
      if (m.product_color_id === fx.productColorId)
        await tryCleanup(page, 'DELETE', `/purchase/sku-mappings/${m.id}`, '负例前置清理');
    }

    const res = await apiCallExpectFail(page, 'POST', '/purchase/orders', {
      supplier_id: fx.sup2Id,
      order_date: new Date().toISOString().slice(0, 10),
      warehouse_id: warehouseId,
      department_id: departmentId,
      source_sales_order_id: salesOrderId || 1,
      items: [{ material_id: fx.productId, quantity_ordered: '10', color_no: fx.colorNo }],
    });
    expect(res.status, `无对照转采购应 4xx，实际 ${res.status}`).toBeGreaterThanOrEqual(400);
    expect(res.status).toBeLessThan(500);
    expect(res.code, `应 BUSINESS_ERROR，实际 ${res.code}`).toBe('BUSINESS_ERROR');
    // 关键保密负向：displayable 中性文案含「无该色号」，且绝不泄露供应商/对照/supplier/调货/自制
    expect(res.message ?? '', `无映射文案应含"无该色号"，实际="${res.message}"`).toContain(
      '无该色号'
    );
    expect(res.message ?? '', `无映射文案不得泄露保密词，实际="${res.message}"`).not.toMatch(
      FORBIDDEN_SECRETS
    );
  });

  test('色号在该产品下不存在：转采购被中性拒绝（脱敏 business，不泄露实现词）', async ({
    page,
  }) => {
    expect(warehouseId, '前置仓库缺失').toBeGreaterThan(0);
    expect(departmentId, '前置部门缺失').toBeGreaterThan(0);

    const fx = await discoverDemoFixture(page);
    const res = await apiCallExpectFail(page, 'POST', '/purchase/orders', {
      supplier_id: fx.sup1Id,
      order_date: new Date().toISOString().slice(0, 10),
      warehouse_id: warehouseId,
      department_id: departmentId,
      source_sales_order_id: salesOrderId || 1,
      // 该产品下不存在的色号 → hook 走「色号不存在」分支（AppError::business，出参脱敏）
      items: [
        {
          material_id: fx.productId,
          quantity_ordered: '10',
          color_no: `__E2E_NO_SUCH_${Date.now()}__`,
        },
      ],
    });
    expect(res.status, `色号不存在应 4xx，实际 ${res.status}`).toBeGreaterThanOrEqual(400);
    expect(res.status).toBeLessThan(500);
    expect(res.code).toBe('BUSINESS_ERROR');
    // 该分支用 AppError::business（脱敏为"业务处理失败"），真实指路文案只进日志、不外显——
    // 出参仍满足保密：不出现任何供应商/对照/supplier/调货/自制字样。
    expect(
      res.message ?? '',
      `色号不存在出参应脱敏、不含保密词，实际="${res.message}"`
    ).not.toMatch(FORBIDDEN_SECRETS);
  });
});

// ===========================================================================
// E3. 批量导入（真实文件上传链路）
//     经 UI 上传 csv 走 POST /purchase/sku-mappings/import：服务端按扩展名/magic 解析
//     文件，列名→字段的权威表在 backend/src/handlers/sku_mapping_handler.rs:88-122，
//     逐行引用校验与落库复用 service.import_batch。断三件事：合法行落库可读回、
//     引用不存在编码的行按行拒并点名列、表头缺必需列整文件 400 零落库。
// ===========================================================================

test.describe('SKU 对照表 - 批量导入（真实文件上传）', () => {
  test('purchaser 上传 csv：合法行入库可读回、非法产品编码行逐行拒、表头缺列整文件拒', async ({
    page,
  }) => {
    await loginAsRole(page, 'admin');
    const fx = await discoverDemoFixture(page);

    await loginAsRole(page, 'purchaser');
    const ts = Date.now().toString().slice(-8);
    const spCode = `E2E-IMP-SP${ts}`;
    const scNo = `E2E-IMP-SC${ts}`;
    const badProductCode = `E2E-IMP-NOPE${ts}`;

    // 前置：导入按「编码」引用供应商商品/色号，故先自建唯一编码（可回读、可清理）
    const sp = await apiCall<{ id?: number }>(page, 'POST', '/purchase/supplier-products', {
      supplier_id: fx.sup1Id,
      product_code: spCode,
      product_name: `E2E导入供应商品${ts}`,
      unit: '米',
    });
    const spId = Number(sp.data?.id);
    expect(spId, '前置：创建供应商商品应回 id').toBeGreaterThan(0);
    const sc = await apiCall<{ id?: number }>(page, 'POST', '/purchase/supplier-product-colors', {
      supplier_product_id: spId,
      color_no: scNo,
      color_name: `E2E导入供应色号${ts}`,
      extra_cost: '1.00',
    });
    const scId = Number(sc.data?.id);
    expect(scId, '前置：创建供应商色号应回 id').toBeGreaterThan(0);

    let mappingId = 0;
    try {
      // 表头逐字取自后端权威映射表；缺列/未知列后端整文件 400，不静默降级成 0 行
      const COLS = [
        'product_code',
        'color_no',
        'supplier_code',
        'supplier_product_code',
        'supplier_color_no',
        'supplier_price',
        'min_order_quantity',
        'lead_time',
        'is_primary',
        'priority',
        'is_enabled',
        'remarks',
      ];
      const goodRow = [
        fx.productCode,
        fx.colorNo,
        'SUP-DEMO-FAB-01',
        spCode,
        scNo,
        '61.25',
        '100',
        '7',
        'true',
        '1',
        'true',
        `E2E导入${ts}`,
      ].join(',');
      const badRow = [
        badProductCode,
        '',
        'SUP-DEMO-FAB-01',
        spCode,
        scNo,
        '',
        '',
        '',
        '',
        '',
        '',
        '',
      ].join(',');
      const importUrl = '/purchase/sku-mappings/import';
      const waitImport = () =>
        page.waitForResponse(r => r.url().includes(importUrl) && r.request().method() === 'POST', {
          timeout: 30_000,
        });

      await safeGoto(page, '/purchase/sku-mapping');
      await page
        .getByRole('button', { name: /批量导入/ })
        .first()
        .click();
      const dialog = page.locator('.el-dialog:visible').last();
      await dialog.waitFor({ state: 'visible', timeout: 15_000 });
      const fileInput = dialog.locator('input[type=file]');

      // 1) 两行混合文件：1 合法 + 1 引用不存在产品编码
      const csvPath = `/tmp/sku-import-${ts}.csv`;
      fs.writeFileSync(csvPath, [COLS.join(','), goodRow, badRow].join('\r\n'), 'utf-8');
      const p1 = waitImport();
      await fileInput.setInputFiles(csvPath);
      const resp1 = await p1;
      expect(resp1.status(), `上传 csv 应 200，实际=${resp1.status()}`).toBe(200);
      const b1 = (await resp1.json()) as {
        data?: {
          total_count?: number;
          success_count?: number;
          error_count?: number;
          errors?: Array<{ row?: number; column?: string; message?: string; value?: string }>;
        };
      };
      const d1 = b1.data ?? {};
      expect(d1.total_count, 'total_count 应为文件数据行数 2').toBe(2);
      expect(d1.success_count, '合法行应成功 1 条').toBe(1);
      expect(d1.error_count, '引用不存在产品编码的行应失败 1 条').toBe(1);
      expect(d1.errors?.length, '失败明细须逐行返回').toBe(1);
      expect(String(d1.errors?.[0]?.column), '失败须点名涉事列').toBe('product_code');
      expect(
        Number(d1.errors?.[0]?.row),
        `失败须归属第 2 数据行，实际=${d1.errors?.[0]?.row}`
      ).toBe(2);
      expect(
        String(d1.errors?.[0]?.value),
        '失败明细须回显被拒的编码值（行级归因，不许含糊成"导入失败"）'
      ).toBe(badProductCode);

      // 对话框侧同步可见失败明细 1 行（用户看得到，不只有接口返回）
      await expect(
        dialog.locator('.import-errors .el-table__body tbody tr'),
        '导入对话框须列出 1 行失败明细'
      ).toHaveCount(1);

      // 2) 落库回读：合法行以唯一编码精确命中，且协议价按导入值入库
      const listed = await apiCallRaw<{
        items: Array<{ id: number; supplier_product_code?: string; supplier_price?: string }>;
      }>(
        page,
        'GET',
        `/purchase/sku-mappings?supplier_id=${fx.sup1Id}&product_id=${fx.productId}&page=1&page_size=200`
      );
      const hit = (listed?.items ?? []).filter(m => m.supplier_product_code === spCode);
      expect(
        hit.length,
        '导入成功的行必须能经列表读回，且唯一编码只命中一条（非法行不得落库）'
      ).toBe(1);
      mappingId = Number(hit[0]?.id);
      expect(mappingId, '读回行须有主键').toBeGreaterThan(0);
      expect(
        String(hit[0]?.supplier_price),
        `协议价须按导入值 61.25 入库（Decimal 以字符串承载），实际=${hit[0]?.supplier_price}`
      ).toContain('61.25');

      // 3) 表头缺必需列：整文件 400，族别 VALIDATION_ERROR，且点名缺失列（不静默当 0 行）
      const missingHeader = COLS.filter(c => c !== 'product_code').join(',');
      const missingPath = `/tmp/sku-import-missing-${ts}.csv`;
      fs.writeFileSync(missingPath, [missingHeader, `x,${fx.colorNo}`].join('\r\n'), 'utf-8');
      const p2 = waitImport();
      await fileInput.setInputFiles(missingPath);
      const resp2 = await p2;
      expect(resp2.status(), `表头缺必需列应 400，实际=${resp2.status()}`).toBe(400);
      const b2 = (await resp2.json()) as { code?: string; message?: string };
      expect(b2.code, `文件形态拒绝须归 VALIDATION_ERROR 族，实际=${b2.code}`).toBe(
        'VALIDATION_ERROR'
      );
      expect(
        String(b2.message ?? ''),
        '参数校验类拒绝须点名缺失列名（fail-visible），不得含糊成"导入失败"'
      ).toContain('product_code');
    } finally {
      // 清理：对照需 admin 删除；供应商商品/色号无 DELETE 端点，PUT 停用软清理
      if (scId)
        await apiCall(page, 'PUT', `/purchase/supplier-product-colors/${scId}`, {
          supplier_product_id: spId,
          color_no: scNo,
          color_name: '停用',
          is_enabled: false,
        }).catch(e => console.warn('[清理] 停用导入供应色号失败:', (e as Error).message));
      if (spId)
        await apiCall(page, 'PUT', `/purchase/supplier-products/${spId}`, {
          supplier_id: fx.sup1Id,
          product_code: spCode,
          product_name: '停用',
          unit: '米',
          is_enabled: false,
        }).catch(e => console.warn('[清理] 停用导入供应商品失败:', (e as Error).message));
      if (mappingId) {
        await loginAsRole(page, 'admin');
        await tryCleanup(
          page,
          'DELETE',
          `/purchase/sku-mappings/${mappingId}`,
          'E3清理导入生成的对照(admin)'
        );
      }
    }
  });
});
