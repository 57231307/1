import { test, expect } from '../diagnose-fixture';
import type { Page, Response } from '@playwright/test';
import {
  loginAsRole,
  apiCallRaw,
  apiCallExpectFail,
  expectDenied,
  genCode,
  BASE_URL,
} from './helpers';
import { safeGoto } from './ui-helpers';

/**
 * 55 角色 × 页面/列表端点 正/反双向"真断言"（补权限假绿与覆盖缺口）
 *
 * 本 spec 的存在意义（针对审计发现的权限"假绿/缺口"）：
 * - 旧遍历类用例只做 `assertPageHealthy`（0 pageerror / 0 5xx / 主容器非白屏）。当后端对某岗
 *   403 而前端把列表渲成"暂无数据"空态时，主容器仍有 >10 字符文本 → 该用例照绿。这就是
 *   "登录成功 = 过 / 403 渲空也绿"的假绿面。
 * - 因此本 spec 的**正向**必须同时钉：① 该岗有权的**列表主端点**返回 HTTP 200 且**响应体里
 *   真有数据行**（beforeAll/前置用真实 API 预置 ≥1 条，断 rows>=1）；② 对应**功能页不被路由守卫
 *   弹到 /403** 且**表格 DOM 真渲染出可见行**。二者缺一不算正向。
 * - **反向**（越权）必须钉到"看不到且被拒"：① 路由守卫把导航挡到 /403，和/或 ② 直接打该列表端点
 *   返回 403（用 helpers.expectDenied 归因到 FORBIDDEN 权限门机器码，杜绝 CSRF/404/5xx 冒名）。
 *   理想成对钉（守卫 + 端点同时钉）。
 *
 * 事实来源（file:line，非推测）：
 * - 角色有效权限：CI 库业务角色由 backend init 全矩阵播种（见 permission.rs 各 *_role_resources），
 *   再叠加 e2e `SEED_ROLE_EXTRA_PERMISSIONS`（global-setup.ts:450-542）。本 spec 的正/负样本选取
 *   以**两个口径都成立**为目标（见下方矩阵表注释），拿不准的软断言已在报告中标注。
 * - 列表是否按 data_scope 过滤（决定"能否看到种子行"）：
 *   · `/products`（product_handler.rs:255 → product_ops/crud.rs:68）**不做行级范围过滤** →
 *     持 products:read 的岗即可见全局种子产品（global-setup.ts:1350-1389 保证 ≥1）。
 *   · `/currencies`（currency_handler.rs:44）**无 auth/无范围过滤** → 全局可见；本位币 CNY 由
 *     global-setup.ts:2805-2850 直连 DB 幂等预置（缺则 globalSetup 判红），故 rows>=1 有据。
 *   · `/crm/customers`（customer_ops/crud.rs:107）**按行级数据范围过滤**（owner/dept/公海），
 *     非 owner 岗对全局种子不保证可见 → 本 spec 以 sales_manager 自建自见（owner=本人）来钉 rows。
 *   · `/inventory/pieces`（inventory_piece_handler.rs:87 → utils/data_scope.rs:83）按 CreatedBy
 *     过滤且无 created_by IS NULL 放行分支；warehouse_manager/quality_inspector 只有 pieces:read
 *     无 pieces:create，无法自建，故其正向面只能做到"授权真实生效（200 + 合法分页信封，非 403）"
 *     这一能力级证据，无法保证 rows>=1——此为已知较弱正向，详见交付报告与下方用例内 console.error。
 * - 路由守卫：src/router/index.ts:1712-1768，`hasRoutePermission` 失败 → next({path:'/403'})；
 *   COARSE_DOMAIN_RESOURCES(:1540+) 把粗域键(finance:read 等)解释为"持该域任一细资源读即放行"。
 * - admin 专属 GET /users 越权 403：既有 33-vertical-privilege.spec.ts 已证非 admin 岗 403。
 */

// ---------------------------------------------------------------------------
// 测试常量与可复用原语
// ---------------------------------------------------------------------------

const LIST_UI_WAIT_MS = 30_000;
const DOM_ROW_WAIT_MS = 12_000;

/** 列表响应行的两种真实信封形态（单一事实来源，不猜第三种）：
 *  - 分页：data = { items:[...], total, page, page_size }（utils/response.rs:34）
 *  - 裸数组：data = [...]（list_currencies/list_vouchers 等 ApiResponse<Vec>）
 *  形态不符 → 抛错点名，不兜底（兜底会把契约漂移伪装成 0 行绿灯）。 */
function parseListRows(data: unknown, context: string): unknown[] {
  if (Array.isArray(data)) return data;
  if (
    data != null &&
    typeof data === 'object' &&
    Array.isArray((data as { items?: unknown }).items)
  ) {
    return (data as { items: unknown[] }).items;
  }
  const shape = data == null ? String(data) : `object{${Object.keys(data as object).join(',')}}`;
  throw new Error(`[${context}] 列表契约失配：期望 data 为数组或含 data.items 数组，实际 ${shape}`);
}

/** 从分页/裸数组信封取 total（分页有；裸数组用长度）。仅用于失败诊断信息，不作硬门。 */
function listTotal(data: unknown): number {
  if (Array.isArray(data)) return data.length;
  if (
    data != null &&
    typeof data === 'object' &&
    typeof (data as { total?: unknown }).total === 'number'
  ) {
    return (data as { total: number }).total;
  }
  return -1;
}

/** 端点正向：以该岗真实会话直接打列表端点，判 HTTP 200 + 解析出行数。
 *  apiCallRaw 对失败信封（code!==200，含 403 FORBIDDEN）会抛错，因此这里"没抛"即已排除 403/4xx。 */
async function assertEndpointRows(
  page: Page,
  path: string,
  context: string,
  minRows = 1
): Promise<unknown[]> {
  const data = await apiCallRaw<unknown>(page, 'GET', path);
  const rows = parseListRows(data, context);
  expect(
    rows.length,
    `${context}：列表端点 ${path} 应返回 ≥${minRows} 行（数据层非空），实际 ${rows.length} 行`
  ).toBeGreaterThanOrEqual(minRows);
  return rows;
}

/** UI 正向：goto 功能页（断未被守卫弹到 /403）+ 捕获该页列表 GET 响应判 200 + DOM 渲染出可见行。 */
async function assertPageRendersRows(
  page: Page,
  routePath: string,
  listUrlSubstring: string,
  context: string
): Promise<void> {
  await safeGoto(page, routePath);

  // 守卫未拦截：URL 不应是 /403（若被弹走，说明该岗对页面 meta.permission 无权 → 正向前提不成立）
  const url = page.url();
  expect(
    url,
    `${context}：进入功能页 ${routePath} 被路由守卫弹到 /403，说明该岗无权，正向前提不成立`
  ).not.toMatch(/\/403(\?|#|$)/);

  // 捕获页面自身发起的列表 GET（kills 403-渲空假绿：403 会给出非 200 状态或错误信封）
  let listStatus = -1;
  try {
    const resp: Response = await page.waitForResponse(
      r => r.url().includes(listUrlSubstring) && r.request().method() === 'GET',
      { timeout: LIST_UI_WAIT_MS }
    );
    listStatus = resp.status();
  } catch (e) {
    console.error(
      `[55-role] 未在 ${LIST_UI_WAIT_MS}ms 内捕获页面 GET ${listUrlSubstring}（UI 可能未自动加载列表或路径漂移）: ${(e as Error).message}`
    );
  }
  if (listStatus !== -1) {
    expect(
      listStatus,
      `${context}：页面列表请求 ${listUrlSubstring} 应 200（403/4xx 即假绿来源），实际 ${listStatus}`
    ).toBe(200);
  }

  // DOM 表格真渲染出可见行（仅取 body 数据行：Element Plus 标准表 .el-table__body .el-table__row；
  // 虚拟/v2 表 .v2-table-body-row。刻意不写 [role="row"]——它也会命中表头行，会让"零数据行"假绿）。
  const dataRows = page.locator('.el-table__body .el-table__row, .el-table-v2 .v2-table-body-row');
  const firstRow = dataRows.first();
  let domCount = 0;
  try {
    await firstRow.waitFor({ state: 'visible', timeout: DOM_ROW_WAIT_MS });
    domCount = await dataRows.count();
  } catch (e) {
    console.error(`[55-role] DOM 数据行等待失败(${listUrlSubstring}): ${(e as Error).message}`);
  }
  expect(
    domCount,
    `${context}：${routePath} 表格应渲染 ≥1 可见行（数据真在 UI 出现），实际 DOM 行数 ${domCount}；页面列表响应状态=${listStatus}`
  ).toBeGreaterThanOrEqual(1);
}

/** 反向：直接打越权列表端点，用 expectDenied 归因到 403 + FORBIDDEN 权限门机器码。 */
async function assertEndpointDenied(page: Page, path: string, context: string): Promise<void> {
  const result = await apiCallExpectFail(page, 'GET', path);
  expectDenied(result, context);
}

/** 反向：goto 越权页，断被路由守卫挡到 /403（waitForURL 拿真导航结果，非仅初始 URL）。 */
async function assertPageGuardDenied(
  page: Page,
  routePath: string,
  context: string
): Promise<void> {
  await page.goto(`${BASE_URL}${routePath}`, { waitUntil: 'domcontentloaded' }).catch(e => {
    console.error(`[55-role] goto ${routePath} 异常: ${(e as Error).message}`);
  });
  let reached403 = true;
  try {
    await page.waitForURL(/\/403(\?|#|$)/, { timeout: 15_000 });
  } catch (e) {
    reached403 = false;
    console.error(`[55-role] 等待被挡到 /403 超时(${routePath}): ${(e as Error).message}`);
  }
  expect(
    reached403,
    `${context}：越权页 ${routePath} 应被路由守卫挡到 /403，实际停留在 ${page.url()}`
  ).toBe(true);
  // 附带钉：内容不应渲染出该页的真实列表区（被挡后 403 页不含业务表格）
  const leakedTable = await page
    .locator('.el-table__body .el-table__row')
    .count()
    .catch(() => 0);
  expect(
    leakedTable,
    `${context}：越权页 ${routePath} 不应渲染出业务数据行（越权可见），实际 ${leakedTable}`
  ).toBe(0);
}

// ===========================================================================
// 正向矩阵（角色应有权 → 列表端点 200 + 真数据行 + 功能页渲染出行）
// ===========================================================================

test.describe('正向：有权岗位看得到真实数据（非 assertPageHealthy 式假绿）', () => {
  test('purchaser GET /products → 200 且真行 ≥1 + /product 页渲染出表格行', async ({ page }) => {
    // 依据：product_handler.rs:255 list_products 无行级范围过滤；purchaser 的 products:read 在
    // 后端矩阵(purchase_clerk) 与 e2e extras(global-setup.ts:470) 两个口径下都存在；
    // 全局产品种子由 global-setup.ts:1350-1389 保证 ≥1（缺则 globalSetup 已判红）。
    await loginAsRole(page, 'purchaser');
    await assertEndpointRows(page, '/products?page=1&page_size=20', 'purchaser→/products');
    await assertPageRendersRows(page, '/product', '/erp/products', 'purchaser→/product 页');
  });

  test('accountant GET /currencies → 200 且真行 ≥1 + /currency 页渲染出表格行', async ({
    page,
  }) => {
    // 依据：currency_handler.rs:44 list_currencies 无 auth/无范围过滤（全局可见）；accountant 的
    // currencies:read 见后端矩阵(permission.rs:1081)。**口径风险**：accountant 不在
    // SEED_ROLE_EXTRA_PERMISSIONS，若 CI 库该岗非 init 全矩阵而是最小 extras(SHELL only)，
    // 会 403——本用例即会把该模型分歧如实判红（不掩盖），详见交付报告。
    // 数据前置：本位币 CNY 由 global-setup.ts:2805-2850 直连 DB 幂等预置，rows>=1 有据。
    await loginAsRole(page, 'accountant');
    await assertEndpointRows(page, '/currencies', 'accountant→/currencies');
    await assertPageRendersRows(page, '/currency', '/erp/currencies', 'accountant→/currency 页');
  });

  test('sales_manager 自建客户后 GET /crm/customers 见该行 + /customer 页渲染该行', async ({
    page,
  }) => {
    // 依据：sales_manager 持 customers:"*"(permission.rs:604)。crm/customers 按行级范围过滤，
    // 故不依赖全局种子，而以"本人自建 → owner=本人 → 必可见"来钉 rows（确定性、自包含）。
    await loginAsRole(page, 'sales_manager');
    const uniqueName = genCode('E2E-RPA-CUS');
    const created = await apiCallRaw<{ id?: number; customer_name?: string }>(
      page,
      'POST',
      '/crm/customers',
      { customer_name: uniqueName, contact_phone: '13800000000' }
    );
    expect(
      created?.id != null && created.id > 0,
      `sales_manager 自建客户未返回有效 id（前置数据未落，实际 id=${JSON.stringify(created?.id)}）`
    ).toBe(true);

    // 端点：列表响应含该唯一行（kills 403-渲空 + 行级隔离假绿）
    const data = await apiCallRaw<unknown>(page, 'GET', '/crm/customers?page=1&page_size=100');
    const rows = parseListRows(data, 'sales_manager→/crm/customers');
    const matched = rows.filter(
      r => (r as { customer_name?: string }).customer_name === uniqueName
    );
    expect(
      matched.length,
      `sales_manager 自建客户应出现在其客户列表中，unique=${uniqueName}，实际命中 ${matched.length}/${rows.length}`
    ).toBeGreaterThanOrEqual(1);

    // UI：功能页渲染 + 该唯一客户行以文本形式真实可见（稳定可见标识）
    await safeGoto(page, '/customer');
    const url = page.url();
    expect(url, `sales_manager 进 /customer 被守卫弹到 /403（无 customers:read？）`).not.toMatch(
      /\/403(\?|#|$)/
    );
    let listStatus = -1;
    try {
      const resp = await page.waitForResponse(
        r => r.url().includes('/erp/customers') && r.request().method() === 'GET',
        { timeout: LIST_UI_WAIT_MS }
      );
      listStatus = resp.status();
    } catch (e) {
      console.error(`[55-role] 未捕获 /customer 页列表 GET: ${(e as Error).message}`);
    }
    if (listStatus !== -1) {
      expect(listStatus, `sales_manager /customer 页列表请求应 200，实际 ${listStatus}`).toBe(200);
    }
    const namedRow = page.locator('.el-table__body .el-table__row', { hasText: uniqueName });
    let namedVisible = true;
    try {
      await namedRow.first().waitFor({ state: 'visible', timeout: DOM_ROW_WAIT_MS });
    } catch (e) {
      namedVisible = false;
      console.error(
        `[55-role] /customer 页未渲染出自建唯一客户行(${uniqueName}): ${(e as Error).message}`
      );
    }
    expect(
      namedVisible,
      `sales_manager /customer 页应在表格中真实渲染出自建客户行(unique=${uniqueName})`
    ).toBe(true);
  });

  // 已知较弱正向（能力级，非"看得到数据"级）：见文件头 data_scope 说明——warehouse_manager /
  // quality_inspector 仅有 pieces:read（extras 与矩阵皆在册），pieces 列表按 CreatedBy 过滤、
  // 无 NULL 放行、且该二岗无 pieces:create 无法自建 → rows>=1 不成立，故此处只钉"授权真实生效"
  // （200 + 合法分页信封），显式区别于上面三例的"真行≥1+UI 渲行"。这不是 403-渲空假绿：403 会给出
  // FORBIDDEN 错误信封，本用例判 data.items 为数组 + total 为数字，二者不可能出现在 403 直出体上。
  for (const role of ['warehouse_manager', 'quality_inspector']) {
    test(`${role} GET /inventory/pieces → 200 且分页信封合法（能力级正向，rows>=1 因 scope/无 create 不保证）`, async ({
      page,
    }) => {
      await loginAsRole(page, role);
      const data = await apiCallRaw<unknown>(page, 'GET', '/inventory/pieces?page=1&page_size=20');
      const rows = parseListRows(data, `${role}→/inventory/pieces`);
      const total = listTotal(data);
      expect(
        total,
        `${role}→/inventory/pieces：分页信封应含数值 total（证明已过权限门、非 403 直出体）`
      ).toBeGreaterThanOrEqual(0);
      console.error(
        `[55-role][已知较弱正向] ${role} /inventory/pieces rows=${rows.length} total=${total} ——` +
          `该岗仅 pieces:read 且无 pieces:create，受 CreatedBy 行级过滤限制，rows>=1 不作硬断言。`
      );
    });
  }
});

// ===========================================================================
// 反向矩阵（越权：路由守卫挡 /403 + 直连端点 403，成对钉）
// ===========================================================================

test.describe('反向：越权页面/端点看不到且被拒', () => {
  test('purchaser：越权读客户/销售订单/凭证 → 守卫 /403 + 端点 403；本域产品仍 200（区分端点缺失）', async ({
    page,
  }) => {
    await loginAsRole(page, 'purchaser');
    // purchaser(purchase_clerk 集 + extras) 不含 customers / orders(销售) / vouchers
    await assertPageGuardDenied(page, '/customer', 'purchaser 越权 /customer');
    await assertEndpointDenied(page, '/crm/customers?page=1&page_size=5', 'purchaser 越权读客户');
    await assertEndpointDenied(
      page,
      '/sales/orders?page=1&page_size=5',
      'purchaser 越权读销售订单'
    );
    await assertEndpointDenied(page, '/vouchers?page=1&page_size=5', 'purchaser 越权读凭证');
    await assertEndpointDenied(page, '/users?page=1&page_size=5', 'purchaser 越权读用户(admin面)');
    // 对照：purchaser 有权的 /products 可达 200 → 证明上一条 403 是权限拒而非端点整体缺失
    await assertEndpointRows(page, '/products?page=1&page_size=5', 'purchaser→/products 对照');
  });

  test('sales_manager：越权读采购单/凭证/用户 → 端点 403 + 守卫 /403', async ({ page }) => {
    await loginAsRole(page, 'sales_manager');
    await assertPageGuardDenied(page, '/purchase', 'sales_manager 越权 /purchase');
    await assertEndpointDenied(
      page,
      '/purchase/orders?page=1&page_size=5',
      'sales_manager 越权读采购单'
    );
    await assertEndpointDenied(page, '/vouchers?page=1&page_size=5', 'sales_manager 越权读凭证');
    await assertEndpointDenied(page, '/users?page=1&page_size=5', 'sales_manager 越权读用户');
    // 对照：sales_manager 有权读订单（extras orders:read + 矩阵）——只断 200（scope 空不苛求 rows）
    const data = await apiCallRaw<unknown>(page, 'GET', '/sales/orders?page=1&page_size=5');
    const rows = parseListRows(data, 'sales_manager→/sales/orders 对照');
    console.error(
      `[55-role][对照] sales_manager /sales/orders rows=${rows.length}（dept/self scope，仅作权限可达对照）`
    );
  });

  test('warehouse_manager：越权读采购单/客户/凭证/用户 + 库存/财务/采购页被挡 /403', async ({
    page,
  }) => {
    await loginAsRole(page, 'warehouse_manager');
    await assertEndpointDenied(
      page,
      '/purchase/orders?page=1&page_size=5',
      'warehouse_manager 越权读采购单'
    );
    await assertEndpointDenied(
      page,
      '/crm/customers?page=1&page_size=5',
      'warehouse_manager 越权读客户'
    );
    await assertEndpointDenied(
      page,
      '/vouchers?page=1&page_size=5',
      'warehouse_manager 越权读凭证'
    );
    await assertEndpointDenied(page, '/users?page=1&page_size=5', 'warehouse_manager 越权读用户');
    await assertPageGuardDenied(page, '/finance', 'warehouse_manager 越权 /finance');
    await assertPageGuardDenied(page, '/purchase', 'warehouse_manager 越权 /purchase');
    // 已知缺口暴露：warehouse_manager 仅 pieces:read（无 inventory:read），而 /inventory 页守卫键是
    // inventory:read → 该岗能直接打 /inventory/pieces（上面正向已 200）却被页面守卫挡在门外。
    // 若本条红（未被挡），说明守卫键与后端资源键不一致，属真实缺口，交回编排方，不在此放宽。
    await assertPageGuardDenied(
      page,
      '/inventory',
      'warehouse_manager 仅 pieces:read，/inventory 页应被挡'
    );
  });

  test('accountant：越权读客户/采购单/销售订单/用户 + /customer 页被挡 /403', async ({ page }) => {
    await loginAsRole(page, 'accountant');
    // accountant 权限集不含 customers / purchase-orders / orders / users
    await assertPageGuardDenied(page, '/customer', 'accountant 越权 /customer');
    await assertEndpointDenied(page, '/crm/customers?page=1&page_size=5', 'accountant 越权读客户');
    await assertEndpointDenied(
      page,
      '/purchase/orders?page=1&page_size=5',
      'accountant 越权读采购单'
    );
    await assertEndpointDenied(
      page,
      '/sales/orders?page=1&page_size=5',
      'accountant 越权读销售订单'
    );
    await assertEndpointDenied(page, '/users?page=1&page_size=5', 'accountant 越权读用户');
  });

  test('quality_inspector：越权读凭证/客户/采购单/用户 + /customer、/product 页被挡 /403', async ({
    page,
  }) => {
    await loginAsRole(page, 'quality_inspector');
    await assertPageGuardDenied(page, '/customer', 'quality_inspector 越权 /customer');
    await assertPageGuardDenied(page, '/product', 'quality_inspector 越权 /product');
    await assertEndpointDenied(
      page,
      '/vouchers?page=1&page_size=5',
      'quality_inspector 越权读凭证'
    );
    await assertEndpointDenied(
      page,
      '/crm/customers?page=1&page_size=5',
      'quality_inspector 越权读客户'
    );
    await assertEndpointDenied(
      page,
      '/purchase/orders?page=1&page_size=5',
      'quality_inspector 越权读采购单'
    );
    await assertEndpointDenied(page, '/users?page=1&page_size=5', 'quality_inspector 越权读用户');
  });
});
