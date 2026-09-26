import { test, expect } from '../diagnose-fixture';
import type { Locator, Page } from '@playwright/test';
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
import { safeGoto } from '../flow/ui-helpers';

/**
 * 供应商商品色号对照表（sku-mapping / 面料二批调货）e2e
 *
 * 打真实 server + 真实 PostgreSQL（globalSetup migrate + 建角色/账号 + 业务种子 m0015 演示供应商
 * SUP-DEMO-FAB-01/02 + FAB-P001/P002/P101/P102 + 色号 + 对照表种子）。
 *
 * 本文件覆盖：
 *  A. 对照表 API 契约：keyword 过滤分页形状、validate_refs 真实链路、resolve 无映射中性拒 +
 *     出参脱敏、供应商商品/色号目录 create/duplicate/supplier-not-exist 的真实错误词表。
 *  B. 保密矩阵：sales_rep/sales_manager 对 sku-mappings、supplier-products、supplier-product-colors
 *     全 403；销售订单列表/详情响应体递归扫描无任何 supplier_ 前缀键。
 *  C. 角色可达：admin 对照表可达；purchase_clerk 维护页「新增对照」按钮可见；sales_rep 直航被路由守卫拦截。
 *  D. 级联 UI：purchase_clerk 供应商侧三级级联（供应商→供应商商品→供应商色号）的「未选上级则下级
 *     disabled」+「切换供应商清空下级」+ 远程搜索出 FAB-P001/PC-A01；purchase_clerk 完整 UI happy-path
 *     端到端选我方产品+色号→级联→保存→列表 keyword 命中（目标角色自证）；admin 同路径亦保留（全权验证）。
 *  E. API 全链路 + 转采购翻译 hook：purchase_clerk 建供应商商品/色号→对照→list→resolve→重复负例（自证）；
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

function escRe(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/** 锚到含指定 label 的 el-form-item（对话框/页面作用域内）。 */
function formItem(root: Locator, labelText: string): Locator {
  return root
    .locator('.el-form-item')
    .filter({ has: root.locator('.el-form-item__label', { hasText: labelText }) })
    .first();
}

/**
 * 通用「打开下拉 → 可选输入过滤 → 在可见 dropdown 里点目标项」。
 * 同时兼容 el-select（.el-select__wrapper）与 el-select-v2 虚拟滚动（.el-select-v2__wrapper）：
 * 远程搜索型（filterable+remote）须传 query 触发 remote-method，再在 .el-select-dropdown__item 点目标；
 * 绝不去 click 只读 input。
 */
async function chooseOption(
  page: Page,
  root: Locator,
  labelText: string,
  opts: { query?: string; target: string | RegExp }
): Promise<void> {
  const item = formItem(root, labelText);
  const trigger = item.locator('.el-select__wrapper, .el-select-v2__wrapper').first();
  await trigger.waitFor({ state: 'visible', timeout: 15_000 });
  await trigger.click({ timeout: 10_000 });
  if (opts.query) {
    await page.keyboard.type(opts.query);
    await page.waitForTimeout(400);
  }
  const dropdown = page.locator('.el-select-dropdown:visible').last();
  await dropdown.waitFor({ state: 'visible', timeout: 15_000 });
  const target = typeof opts.target === 'string' ? new RegExp(escRe(opts.target)) : opts.target;
  const option = dropdown.locator('.el-select-dropdown__item').filter({ hasText: target }).first();
  await option.waitFor({ state: 'visible', timeout: 15_000 });
  await option.click({ timeout: 10_000 });
  await dropdown.waitFor({ state: 'hidden', timeout: 5_000 }).catch(() => {});
}

/** 读取某 el-select-v2 表单项当前是否处于禁用态（级联「未选上级则下级 disabled」断言）。 */
async function v2IsDisabled(root: Locator, labelText: string): Promise<boolean> {
  const cls = await formItem(root, labelText)
    .locator('.el-select-v2')
    .first()
    .getAttribute('class')
    .catch(() => null);
  return !!cls && cls.includes('is-disabled');
}

/** 读取禁用/只读展示 el-input 的当前值（如「供应商品编码」只读回显）。 */
async function fieldInputValue(root: Locator, labelText: string): Promise<string> {
  return (
    (await formItem(root, labelText)
      .locator('input')
      .first()
      .inputValue()
      .catch(() => '')) ?? ''
  );
}

/** 按 label 填充可编辑 el-input（如「协议价」）。 */
async function fillFieldByLabel(
  page: Page,
  root: Locator,
  labelText: string,
  value: string
): Promise<void> {
  const inp = formItem(root, labelText).locator('input').first();
  await inp.waitFor({ state: 'visible', timeout: 10_000 });
  await inp.click({ clickCount: 3 });
  await inp.fill(value);
  await page.waitForTimeout(100);
}

// ---------------------------------------------------------------------------
// 真实数据发现（以 admin 会话执行——需跨产品/色号/供应商/商品/色号目录做全量取数）
// 注：products:read 已授采购三角色，purchase_clerk 亦可自行调用；此处沿用 admin 会话以简化复用。
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
    await loginAsRole(page, 'purchase_clerk');
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
    // 先 admin 发现真实 product/supplier（全量取数复用 admin 会话），再切回 clerk 发写请求。
    await loginAsRole(page, 'admin');
    const fx = await discoverDemoFixture(page);
    await loginAsRole(page, 'purchase_clerk');

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
    await loginAsRole(page, 'purchase_clerk');
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
  for (const salesRole of ['sales_rep', 'sales_manager']) {
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

  test('admin 对照：同端点可达（区分端点缺失与权限拒绝）', async ({ page }) => {
    await loginAsRole(page, 'admin');
    const res = await page.request.get(
      `${API_BASE}${API_PREFIX}/purchase/sku-mappings?page=1&page_size=5`
    );
    expect(res.status(), 'admin 应可访问对照表（<400）').toBeLessThan(400);
  });
});

test.describe('SKU 对照表 - 前端路由与菜单矩阵', () => {
  test('purchase_clerk 可达维护页且「新增对照」按钮可见（授予 create）', async ({ page }) => {
    await loginAsRole(page, 'purchase_clerk');
    await safeGoto(page, '/purchase/sku-mapping');
    await page.waitForTimeout(500);
    const createBtn = page.getByRole('button', { name: /新增对照/ }).first();
    await expect(createBtn).toBeVisible({ timeout: 15000 });
  });

  test('sales_rep 无对照表入口：直接导航被路由守卫拦截（不渲染维护页表格）', async ({ page }) => {
    await loginAsRole(page, 'sales_rep');
    await safeGoto(page, '/purchase/sku-mapping');
    await page.waitForTimeout(800);
    const createBtn = page.getByRole('button', { name: /新增对照|新增|新建/ });
    await expect(createBtn).toHaveCount(0);
    const heading = page.locator('h2', { hasText: /SKU 对照|SKU对照|对照表/ });
    await expect(heading).toHaveCount(0);
  });
});

// ===========================================================================
// D1. purchase_clerk 供应商侧级联：未选上级禁用 + 切换供应商清空下级 + 远程搜索
// ===========================================================================

test.describe('SKU 对照表 - 级联交互（purchase_clerk 供应商侧）', () => {
  test('供应商商品/色号级联：未选供应商则下级 disabled；选供应商后出 FAB-P001、选商品后出 PC-A01；切换供应商清空下级', async ({
    page,
  }) => {
    await loginAsRole(page, 'admin');
    const fx = await discoverDemoFixture(page);
    await loginAsRole(page, 'purchase_clerk');

    await safeGoto(page, '/purchase/sku-mapping');
    await page
      .getByRole('button', { name: /新增对照/ })
      .first()
      .click();
    const dialog = page.locator('.el-dialog:visible').last();
    await dialog.waitFor({ state: 'visible', timeout: 15_000 });
    await page.waitForTimeout(300);

    // (1) 未选供应商：供应商商品、供应商色号两级 el-select-v2 均 disabled（可用性/保密门禁）
    expect(await v2IsDisabled(dialog, '供应商商品'), '未选供应商时「供应商商品」应 disabled').toBe(
      true
    );
    expect(await v2IsDisabled(dialog, '供应商色号'), '未选商品时「供应商色号」应 disabled').toBe(
      true
    );

    // (2) 选供应商=演示甲 → 供应商商品变可用；远程搜索出 FAB-P001
    await chooseOption(page, dialog, '供应商', {
      query: fx.sup1Name,
      target: fx.sup1Name,
    });
    expect(await v2IsDisabled(dialog, '供应商商品'), '选供应商后「供应商商品」应解禁').toBe(false);
    await chooseOption(page, dialog, '供应商商品', { query: 'FAB-P001', target: 'FAB-P001' });

    // 供应商商品编码只读回显 FAB-P001
    expect(await fieldInputValue(dialog, '供应商品编码')).toContain('FAB-P001');
    // 选商品后供应商色号解禁
    expect(await v2IsDisabled(dialog, '供应商色号'), '选商品后「供应商色号」应解禁').toBe(false);
    await chooseOption(page, dialog, '供应商色号', { query: 'PC-A01', target: 'PC-A01' });
    expect(await fieldInputValue(dialog, '供应商色号编号')).toContain('PC-A01');

    // (3) 切换供应商 → 清空下级：改选演示乙后，供应商商品/编码/色号回显全部清空
    await chooseOption(page, dialog, '供应商', { query: '演示乙', target: '演示乙' });
    expect(await fieldInputValue(dialog, '供应商品编码'), '切换供应商后供应商品编码应被清空').toBe(
      ''
    );
    expect(await fieldInputValue(dialog, '供应商色号编号'), '切换供应商后供应商色号应被清空').toBe(
      ''
    );
    // 清空下级后「供应商商品」仍可用（供应商已选），但其值应为空（下拉无选中项文本回显）
    const spText = await formItem(dialog, '供应商商品')
      .locator('.el-select-v2')
      .first()
      .innerText()
      .catch(() => '');
    expect(spText.includes('FAB-P001'), '切换供应商后不应仍残留旧商品').toBe(false);

    // 本用例只验证级联交互行为（禁用/解禁/清空），不提交（完整保存见下方 purchase_clerk 完整级联用例）
    const cancel = dialog.getByRole('button', { name: /取消/ }).first();
    await cancel.click().catch(() => {});
  });
});

// ===========================================================================
// D2. admin 完整 UI happy-path 建对照 → 列表 keyword 按我方色号命中
//     （admin 全权路径有效；purchase_clerk 自证路径见 D2b）
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

    // 我方产品（el-select filterable）+ 我方色号（el-select-v2 remote）
    await chooseOption(page, dialog, '我方产品', { query: fx.productCode, target: fx.productCode });
    await chooseOption(page, dialog, '我方色号', { query: fx.colorNo, target: fx.colorNo });
    // 只读展示我方色号编号 == 选中的 colorNo
    expect(await fieldInputValue(dialog, '我方色号编号')).toContain(fx.colorNo);

    // 供应商三级级联
    await chooseOption(page, dialog, '供应商', { query: fx.sup1Name, target: fx.sup1Name });
    await chooseOption(page, dialog, '供应商商品', { query: 'FAB-P001', target: 'FAB-P001' });
    await chooseOption(page, dialog, '供应商色号', { query: 'PC-A01', target: 'PC-A01' });

    // 协议价
    await fillFieldByLabel(page, dialog, '协议价', '66.60');

    // 提交并捕获创建响应 id（跳过 CSRF 竞败的 403 中间态）
    const respPromise = page.waitForResponse(
      r =>
        r.url().includes('/purchase/sku-mappings') &&
        r.request().method() === 'POST' &&
        r.status() !== 403,
      { timeout: 30_000 }
    );
    await dialog
      .getByRole('button', { name: /确定|保存|提交/ })
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
// D2b. purchase_clerk 完整 UI happy-path 建对照 → 列表 keyword 按我方色号命中
//     目标角色自证：products:read 已授采购三角色（permission.rs:395），
//     purchase_clerk 可端到端在维护页选我方产品+色号→供应商三级级联→保存。
// ===========================================================================

test.describe('SKU 对照表 - 完整 UI happy-path（purchase_clerk 自证）', () => {
  test('purchase_clerk 新增对照：选我方产品+色号→级联供应商/商品/色号→协议价→保存→列表命中', async ({
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

    // ── 切换到 purchase_clerk，以目标角色身份执行全流程 ──
    await loginAsRole(page, 'purchase_clerk');

    await safeGoto(page, '/purchase/sku-mapping');
    await page
      .getByRole('button', { name: /新增对照/ })
      .first()
      .click();
    const dialog = page.locator('.el-dialog:visible').last();
    await dialog.waitFor({ state: 'visible', timeout: 15_000 });
    await page.waitForTimeout(300);

    // 我方产品（el-select filterable，经 GET /products 取数——purchase_clerk 已授 products:read）
    await chooseOption(page, dialog, '我方产品', { query: fx.productCode, target: fx.productCode });
    // 我方色号（el-select-v2 remote，经 GET /products/:id/colors）
    await chooseOption(page, dialog, '我方色号', { query: fx.colorNo, target: fx.colorNo });
    // 断言只读回显 == 选中的色号编号
    expect(await fieldInputValue(dialog, '我方色号编号')).toContain(fx.colorNo);

    // 供应商三级级联（purchase_clerk 有 suppliers:read、supplier-products:read、supplier-product-colors:read）
    await chooseOption(page, dialog, '供应商', { query: fx.sup1Name, target: fx.sup1Name });
    await chooseOption(page, dialog, '供应商商品', { query: 'FAB-P001', target: 'FAB-P001' });
    await chooseOption(page, dialog, '供应商色号', { query: 'PC-A01', target: 'PC-A01' });

    // 协议价
    await fillFieldByLabel(page, dialog, '协议价', '55.55');

    // 提交并捕获创建响应（purchase_clerk 已授 sku-mappings:create）
    const respPromise = page.waitForResponse(
      r =>
        r.url().includes('/purchase/sku-mappings') &&
        r.request().method() === 'POST' &&
        r.status() !== 403,
      { timeout: 30_000 }
    );
    await dialog
      .getByRole('button', { name: /确定|保存|提交/ })
      .last()
      .click();
    const resp = await respPromise;
    const created = (await resp.json().catch(() => null)) as {
      code?: number;
      data?: { id?: number };
    } | null;
    expect(resp.status(), `purchase_clerk 新建对照应成功，实际 HTTP ${resp.status()}`).toBeLessThan(
      400
    );
    const createdId = created?.data?.id;
    expect(typeof createdId, '创建响应应回 data.id').toBe('number');

    // 列表按我方色号 keyword 命中该新建对照（purchase_clerk 已授 sku-mappings:read）
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
    expect(
      hit,
      `keyword=${fx.colorNo} 未命中 purchase_clerk 新建的对照 id=${createdId}`
    ).toBeTruthy();
    expect(hit?.supplier_product_code).toBe('FAB-P001');
    expect(hit?.supplier_color_no).toBe('PC-A01');

    // 清理：purchase_clerk 无 sku-mappings:delete，切 admin 仅做运维清理
    await loginAsRole(page, 'admin');
    await tryCleanup(page, 'DELETE', `/purchase/sku-mappings/${createdId}`, 'D2b用例后清理对照');
  });
});

// ===========================================================================
// E1. API 级 happy-path 全链路（purchase_clerk 自证：建供应商商品→色号→对照→list→resolve→重复拒）
//     purchase_clerk 已授 products:read / supplier-products:read+create+update /
//     supplier-product-colors:read+create+update / sku-mappings:read+create+update，
//     全部步骤（除最终 DELETE 清理外）均可由目标角色完成。
// ===========================================================================

test.describe('SKU 对照表 - API 全链路 happy-path（purchase_clerk 自证）', () => {
  test('purchase_clerk: create supplier-product→color→mapping→list→resolve + 重复/非法负例', async ({
    page,
  }) => {
    // 前置数据发现（跨产品/供应商/目录全量取数，沿用 admin 会话简化复用）
    await loginAsRole(page, 'admin');
    const fx = await discoverDemoFixture(page);

    // ── 切换到 purchase_clerk，以目标角色执行全部业务操作 ──
    await loginAsRole(page, 'purchase_clerk');
    const ts = Date.now().toString().slice(-8);

    let spId = 0;
    let scId = 0;
    let mapId = 0;
    try {
      // 1) 新建供应商商品（purchase_clerk 有 supplier-products:create）
      const sp = await apiCall<{ id?: number }>(page, 'POST', '/purchase/supplier-products', {
        supplier_id: fx.sup1Id,
        product_code: `E2E-SP${ts}`,
        product_name: `E2E供应商品${ts}`,
        unit: '米',
      });
      spId = Number(sp.data?.id);
      expect(spId, 'purchase_clerk 创建供应商商品应回 id').toBeGreaterThan(0);

      // 2) 新建供应商色号（purchase_clerk 有 supplier-product-colors:create）
      const sc = await apiCall<{ id?: number }>(page, 'POST', '/purchase/supplier-product-colors', {
        supplier_product_id: spId,
        color_no: `E2E-SC${ts}`,
        color_name: `E2E供应色号${ts}`,
        extra_cost: '2.50',
      });
      scId = Number(sc.data?.id);
      expect(scId, 'purchase_clerk 创建供应商色号应回 id').toBeGreaterThan(0);
      expect(
        String((sc.data as Record<string, unknown> | undefined)?.extra_cost ?? ''),
        'extra_cost 应回显为字符串承载的 Decimal'
      ).toMatch(/2\.5/);

      // 3) 新建对照（purchase_clerk 有 sku-mappings:create + products:read 前置校验通过）
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
      expect(mapId, 'purchase_clerk 创建对照应回 id').toBeGreaterThan(0);

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
      // 清理：purchase_clerk 有 supplier-products/colors 的 update（PUT 停用），但无 sku-mappings:delete。
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
      // 对照 DELETE 需 admin（purchase_clerk 无此权限），仅做运维清理
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
