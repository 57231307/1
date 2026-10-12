import { test, expect } from '../diagnose-fixture';
import type { Locator, Page, Response } from '@playwright/test';
import { loginAsRole, apiCall, apiCallRaw, genCode, tryCleanup } from './helpers';
import {
  safeGoto,
  waitForDialog,
  submitDialog,
  fillFieldByLabel,
  findTableRow,
} from './ui-helpers';

/**
 * 56 读回可见性（read-after-write）：保存显示成功 → 刷新 → 目标列单元格真的显示该值
 *
 * 本族直击「保存提示成功、刷新后数据看不到」这类现有 e2e 测不出的缺陷：
 * - 30-persistence 只走 API 后端键回读，测不出**前端列表列 prop 漂移**（后端键存在、
 *   出参也含该键，但列绑成不存在的 prop ⇒ 刷新后该列恒空，用户看不到刚存的值）；
 * - 65/68 用 API 回读或抓请求体，同样绕过了「刷新后 UI 列渲染」这一段。
 *
 * 每条用例都真做（缺「刷新后断列值」即假绿）：
 *   loginAsRole('admin') → safeGoto 页面 → 新建/编辑 → 填**唯一 tag**（genCode/Date.now）
 *   → 保存（UI 提交用 captureUiSubmit 抓前端真实请求体 + 确认真 2xx；API 建单用 apiCall
 *     ——helpers.ts apiCall 在 envelope code≠200 时抛错，故 apiCall 返回即已确认 2xx）
 *   → `await page.reload()` 强制重新拉取后端（模拟用户刷新，清空 SPA 内存态）
 *   → 在含该唯一 tag 的表格里**按列头定位目标列单元格** → expect(cell).toContainText(期望值)。
 *
 * 本波修复锚定的两条已修缺陷（回归护栏）：
 * - D1 染色配方**编辑改名**：后端 UpdateDyeRecipeRequest 补 recipe_name 并在 update() Set
 *   （services/dye_recipe_service.rs），前端 /dye-recipe 编辑提交体携带 recipe_name
 *   （views/dye-recipe/index.vue handleSubmitForm updatePayload）。用例 1 同时钉「请求体带
 *   recipe_name」与「刷新后配方名称列=新名」两向。
 *   （注：views/fabric/tabs/RecipeFormDialogTab.vue 是另一入口、其 updatePayload 仍不带
 *   recipe_name，非本波修复面，本用例走已修的 /dye-recipe 独立页。）
 * - D2 库存预留**备注列 prop**：后端出参键 notes，前端列表列 prop 已从 remark 改为 notes
 *   （views/inventory/tabs/InventoryReservationTab.vue el-table-column prop="notes"）。
 *   用例 2 按 ID 列定位行、断「备注」列=提交文本；若 prop 回退成不存在的键，该列刷新后恒空即红。
 * - 用例 3/4 为本波未覆盖的核心单据 create→reload→列值可见（生产配方 / 采购入库）。
 *
 * 读列判据（columnCell）刻意不靠整行 textContent：唯一 tag 只写进目标列，但断言必须落在
 * 与表头列名对应的那一格，才能真正抓「列 prop 绑错字段」这类漂移（值落到了别的列或没落，
 * 目标列单元格都会不等于期望值）。
 *
 * 页面/端点（相对 baseURL=/api/v1/erp）：
 * - /dye-recipe 列表 GET /production/dye-recipes（后端按 recipe_no contains 过滤）、
 *   编辑 PUT /production/dye-recipes/{id}（列：配方编号 recipe_no / 配方名称 recipe_name）。
 * - /inventory 库存预留 tab（lazy 挂载）列表 GET /inventory/reservations（返回 {list,total}，
 *   本页无分页器，前端取 page_size=100；列：ID / … / 备注 notes）。
 *   建单 POST /inventory/reservations 后端预检 order_id/product_id/warehouse_id 三外键存在性，
 *   故夹具用真实销售订单/产品/仓库（UI 新建表单硬编码 order_id:0 必 404，属另一缺陷，本族不以 UI 建）。
 * - /production-recipes 新建配方 POST /production/production-recipes（列：配方编号 recipe_no /
 *   面料名称 fabric_name）。
 * - /purchase-receipt 列表 GET /purchase/receipts（后端支持 supplier_id 过滤；建单
 *   POST /purchase/receipts，列：入库单号 receipt_no / 供应商 supplier_name JOIN 富化）。
 */

type Row = Record<string, unknown>;

interface CapturedSubmit {
  resp: Response;
  status: number;
  body: Row;
  bodyRaw: string;
}

/**
 * 触发一次 UI 提交并抓取前端真实请求体 + 响应（waitForResponse：请求与结果一体）。
 * 跳过 CSRF 首发瞬态 403（admin 被权限中间件短路，该路由唯一 403 源是 CSRF 中间件、
 * axios 拦截器刷新令牌后静默重放成 200）；若为持续真 403（非 CSRF）则永无 2xx、超时判红，不被掩盖。
 */
async function captureUiSubmit(
  page: Page,
  urlFragment: string,
  method: 'POST' | 'PUT',
  action: () => Promise<void>,
  what: string
): Promise<CapturedSubmit> {
  const respPromise = page.waitForResponse(
    res =>
      res.url().includes(urlFragment) && res.request().method() === method && res.status() !== 403,
    { timeout: 30_000 }
  );
  await action();
  const resp = await respPromise;
  const raw = resp.request().postData();
  expect(
    raw,
    `${what}：未捕获到前端提交请求体（url 含 "${urlFragment}" 的 ${method} 请求 postData 为空）`
  ).not.toBeNull();
  const body = JSON.parse(String(raw)) as Row;
  return { resp, status: resp.status(), body, bodyRaw: String(raw) };
}

/**
 * 按**列头文案**定位目标行的那一格单元格（不是整行文本），用于真读列值。
 * 标准 el-table（本族四页均非 el-table-v2）：Element Plus 2.14 固定列用 sticky 渲染在
 * 同一张 <table> 内（无 el-table__fixed 克隆表），故可见 columnheader 顺序 == 行内 cell 顺序。
 * 表头未出现该列名 / 行列数不足到该列，都按真实缺陷判红（不放宽、不降级为整行 hasText）。
 */
async function columnCell(page: Page, row: Locator, columnLabel: string): Promise<Locator> {
  const headers = page.getByRole('columnheader');
  const headerCount = await headers.count();
  let idx = -1;
  for (let i = 0; i < headerCount; i++) {
    const label = (await headers.nth(i).innerText()).trim();
    if (label === columnLabel) {
      idx = i;
      break;
    }
  }
  expect(
    idx,
    `表格中未渲染出表头列「${columnLabel}」（当前可见表头 ${headerCount} 个，URL=${page.url()}）` +
      `——列缺失或表头文案漂移属真实缺陷，非用例问题，不放宽为整行 hasText`
  ).toBeGreaterThanOrEqual(0);

  const cells = row.getByRole('cell');
  const cellCount = await cells.count();
  expect(
    cellCount,
    `目标行单元格数 ${cellCount} 未覆盖到「${columnLabel}」所在的第 ${idx} 列——行/列渲染不齐属真实缺陷`
  ).toBeGreaterThan(idx);
  return cells.nth(idx);
}

/** 带重试地按值定位目标行（reload 后数据到达有延迟，重试到超时；找不到即红并点名）。 */
async function locateRow(
  page: Page,
  value: string,
  opts: Parameters<typeof findTableRow>[4] = undefined,
  timeoutMs = 15_000
): Promise<Locator> {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const row = await findTableRow(page, value, 1, undefined, opts);
    if (row) return row;
    if (Date.now() >= deadline) break;
    await page.waitForTimeout(500);
  }
  const last = await findTableRow(page, value, 1, undefined, opts);
  expect(
    last,
    `刷新后表格中未定位到含「${value}」的行（URL=${page.url()}）——` +
      `保存未生效或前端列/行渲染缺失，属真实缺陷，非用例问题`
  ).toBeTruthy();
  return last as Locator;
}

// ---- 确定性夹具（apiCall 在 envelope code≠200 时抛错，返回即已确认真 2xx） ----

async function seedCustomer(page: Page, tag: string): Promise<number> {
  const code = genCode('E56C');
  const res = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
    customer_code: code,
    customer_name: `56号${tag}客户_${code}`,
    contact_phone: '13800005600',
  });
  const id = Number(res.data?.id);
  expect(id, `[56] 客户建单应回 id：${JSON.stringify(res)}`).toBeGreaterThan(0);
  return id;
}

async function seedActiveProduct(
  page: Page,
  tag: string
): Promise<{ id: number; code: string; name: string }> {
  const code = genCode('E56P');
  const name = `56号${tag}布_${code}`;
  // status 显式 active：库存预留建单预检 products.status==master_data::ACTIVE（=小写 active），
  // 缺省 None 会被判 已停用/不存在 而 404。
  const res = await apiCall<{ id?: number }>(page, 'POST', '/products', {
    code,
    name,
    unit: 'm',
    status: 'active',
  });
  const id = Number(res.data?.id);
  expect(id, `[56] 产品建单应回 id：${JSON.stringify(res)}`).toBeGreaterThan(0);
  return { id, code, name };
}

async function seedWarehouse(page: Page, tag: string): Promise<number> {
  const code = genCode('E56W');
  const res = await apiCall<{ id?: number }>(page, 'POST', '/warehouses', {
    name: `56号${tag}仓库_${code}`,
    code,
  });
  const id = Number(res.data?.id);
  expect(id, `[56] 仓库建单应回 id：${JSON.stringify(res)}`).toBeGreaterThan(0);
  return id;
}

async function seedSalesOrder(page: Page, customerId: number, productId: number): Promise<number> {
  const res = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
    customer_id: customerId,
    order_date: new Date().toISOString(),
    items: [{ product_id: productId, quantity: 5, unit_price: '20.00' }],
  });
  const id = Number(res.data?.id);
  expect(
    id,
    `[56] 销售订单建单应回 id（预留 order_id FK）：${JSON.stringify(res).slice(0, 200)}`
  ).toBeGreaterThan(0);
  return id;
}

async function seedSupplier(page: Page, tag: string): Promise<{ id: number; name: string }> {
  const name = `E56入库供应商_${tag}`;
  const res = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
    supplier_name: name,
    contact_phone: '13900005601',
  });
  const id = Number(res.data?.id);
  expect(id, `[56] 供应商建单应回 id：${JSON.stringify(res)}`).toBeGreaterThan(0);
  return { id, name };
}

test.describe.serial('56 读回可见性（保存显示成功→刷新→目标列单元格真的显示该值）', () => {
  test.beforeEach(async ({ page }) => {
    await loginAsRole(page, 'admin');
  });

  test('56-01 染色配方编辑改名（D1）：请求体带 recipe_name + PUT 真 2xx → 刷新后「配方名称」列=新名', async ({
    page,
  }) => {
    // 夹具：API 建一条 draft 配方（recipe_no 唯一、可编辑；必填列全部非空，避免编辑弹窗校验把
    // content/color_* 判空而阻止 PUT——那会把「改名未生效」误判成用例前置问题）。
    const recipeNo = genCode('DR56');
    const originalName = `原始名_${genCode('ORIG')}`;
    const created = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-recipes', {
      recipe_no: recipeNo,
      recipe_name: originalName,
      color_no: genCode('CN'),
      color_code: genCode('CC'),
      color_name: `原始色名_${recipeNo}`,
      fabric_type: '棉',
      chemical_formula: `原始正文_${recipeNo}`,
      version: 1,
      status: 'draft',
    });
    const recipeId = Number(created.data?.id);
    expect(recipeId, `[56-01] 配方建单应回 id：${JSON.stringify(created)}`).toBeGreaterThan(0);

    await safeGoto(page, '/dye-recipe');

    // 服务端按配方编号过滤（后端 list 对 recipe_no contains），把目标行收敛到第 1 页，规避分页。
    const listEpFrag = '/api/v1/erp/production/dye-recipes';
    const filterGet = page.waitForResponse(
      r => r.url().includes(listEpFrag) && r.url().includes(`recipe_no=`) && r.status() === 200,
      { timeout: 20_000 }
    );
    await fillFieldByLabel(page, page, '配方编号', recipeNo);
    await page.getByRole('button', { name: '查询' }).first().click();
    await filterGet;

    const row = await locateRow(page, recipeNo);
    await row.getByRole('button', { name: '编辑' }).first().click();
    const dlg = await waitForDialog(page);

    const newName = `改后名_${genCode('NEW')}`;
    await fillFieldByLabel(dlg, page, '配方名称', newName);

    const cap = await captureUiSubmit(
      page,
      `${listEpFrag}/${recipeId}`,
      'PUT',
      () => submitDialog(dlg, /确认/),
      '[56-01] 编辑改名提交'
    );

    // D1 前端面：改名必须真把 recipe_name 放进提交体（fabric tab 旧省略该键→serde 静默丢弃的复发形态）
    expect(
      cap.body.recipe_name,
      `[56-01] 编辑提交体必须携带 recipe_name=${JSON.stringify(newName)}（省略该键=后端保持原名，刷新看不到改动即 D1 缺陷），实际 body=${cap.bodyRaw}`
    ).toBe(newName);
    expect(
      cap.status,
      `[56-01] 改名 PUT 必须真 2xx（update handler Ok(Json(ApiResponse)) 无状态改写；3xx/4xx 皆回归），实际 status=${cap.status} body=${cap.bodyRaw}`
    ).toBe(200);

    // 刷新后真读列值（本用例核心：模拟用户 F5，绕开 SPA 内存态）
    await page.reload();
    const filterGet2 = page.waitForResponse(
      r => r.url().includes(listEpFrag) && r.url().includes(`recipe_no=`) && r.status() === 200,
      { timeout: 20_000 }
    );
    await fillFieldByLabel(page, page, '配方编号', recipeNo);
    await page.getByRole('button', { name: '查询' }).first().click();
    await filterGet2;

    const row2 = await locateRow(page, recipeNo);
    const nameCell = await columnCell(page, row2, '配方名称');
    await expect(
      nameCell,
      `[56-01] 刷新后「配方名称」列应显示新名=${newName}（仍为旧名或空=后端 update 未落 recipe_name 或前端列 prop 漂移，属真实缺陷）`
    ).toContainText(newName);
    await expect(
      nameCell,
      `[56-01] 刷新后「配方名称」列不得仍是旧名=${originalName}`
    ).not.toContainText(originalName);

    await tryCleanup(page, 'DELETE', `/production/dye-recipes/${recipeId}`, '[56-01] 染色配方');
  });

  test('56-02 库存预留备注列（D2）：API 建带唯一备注的预留（确认真 2xx）→ 刷新→「备注」列=该文本', async ({
    page,
  }) => {
    const tag = genCode('E56N');
    const customerId = await seedCustomer(page, '预留');
    const { id: productId } = await seedActiveProduct(page, '预留');
    const warehouseId = await seedWarehouse(page, '预留');
    const orderId = await seedSalesOrder(page, customerId, productId);

    const notesTag = `E2E56预留备注_${tag}`;
    const created = await apiCall<{ id?: number }>(page, 'POST', '/inventory/reservations', {
      order_id: orderId,
      product_id: productId,
      warehouse_id: warehouseId,
      quantity: '25',
      notes: notesTag,
    });
    const reservationId = Number(created.data?.id);
    expect(
      reservationId,
      `[56-02] 预留建单应回 id（apiCall 非 2xx 会抛错，返回即确认已落库）：${JSON.stringify(created)}`
    ).toBeGreaterThan(0);

    // 后端落库侧先钉一次：读列表端点确认 notes 已持久（把「DB 有值」与「前端列渲染」分开定位）。
    const raw = await apiCallRaw<{ list?: Row[]; items?: Row[] }>(
      page,
      'GET',
      `/inventory/reservations?product_id=${productId}&warehouse_id=${warehouseId}&page=1&page_size=100`
    );
    const listArr = raw.list ?? raw.items ?? [];
    expect(
      Array.isArray(listArr),
      `[56-02] 预留列表出参必须含 list/items 数组，实际 ${JSON.stringify(raw).slice(0, 200)}`
    ).toBe(true);
    const dbRow = listArr.find(it => Number(it.id) === reservationId);
    expect(dbRow, `[56-02] 后端列表应回读自建预留 id=${reservationId}`).toBeTruthy();
    expect(
      dbRow!.notes,
      `[56-02] 后端出参 notes 键应等于提交文本（缺键/值不符=后端未持久化，判红交后端）：${JSON.stringify(dbRow)}`
    ).toBe(notesTag);

    const listEpFrag = '/api/v1/erp/inventory/reservations';
    await safeGoto(page, '/inventory');
    // 预留 tab 是 lazy 挂载：点开才触发本页 onMounted 拉取 GET /inventory/reservations。
    const tabGet1 = page.waitForResponse(
      r => r.url().includes(listEpFrag) && r.request().method() === 'GET' && r.status() === 200,
      { timeout: 20_000 }
    );
    await page.getByRole('tab', { name: '库存预留' }).click();
    await tabGet1;

    // 刷新：SPA 内存态清空，重新走后端拉取（模拟用户看不到已存数据的场景）
    await page.reload();
    const tabGet2 = page.waitForResponse(
      r => r.url().includes(listEpFrag) && r.request().method() === 'GET' && r.status() === 200,
      { timeout: 20_000 }
    );
    await page.getByRole('tab', { name: '库存预留' }).click();
    await tabGet2;

    // 按 ID 列（首列 prop="id"）精确定位自建行，再断「备注」列——解耦「行存在」与「列渲染」。
    const row = await locateRow(page, String(reservationId), { match: 'first-cell-equals' });
    const notesCell = await columnCell(page, row, '备注');
    await expect(
      notesCell,
      `[56-02] 刷新后「备注」列应显示提交文本=${notesTag}（D2 列 prop=notes；` +
        `若该列恒空而上方 API 回读为绿，即前端列表列绑成不存在的 prop，属真实缺陷非用例问题）`
    ).toContainText(notesTag);

    await tryCleanup(
      page,
      'DELETE',
      `/inventory/reservations/${reservationId}`,
      '[56-02] 库存预留'
    );
    await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, '[56-02] 销售订单');
    await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, '[56-02] 客户');
  });

  test('56-03 生产配方新建→刷新：UI 提交真 2xx →「面料名称」列显示唯一面料名', async ({ page }) => {
    await safeGoto(page, '/production-recipes');
    await page.getByRole('button', { name: '新建配方' }).first().click();
    const dlg = await waitForDialog(page);

    const fabricName = `E2E56生产配方_${genCode('PF')}`;
    await fillFieldByLabel(dlg, page, '面料名称', fabricName);

    const cap = await captureUiSubmit(
      page,
      '/api/v1/erp/production/production-recipes',
      'POST',
      () => submitDialog(dlg, /确定/),
      '[56-03] 新建生产配方提交'
    );
    expect(
      cap.status,
      `[56-03] 新建生产配方 POST 必须真 2xx，实际 status=${cap.status} body=${cap.bodyRaw}`
    ).toBe(200);
    const json = (await cap.resp.json()) as { code?: number; data?: Row };
    expect(json.code, `[56-03] 成功信封 code=200，实际 ${cap.bodyRaw}`).toBe(200);
    const recipeNo = String(json.data?.recipe_no ?? '');
    expect(recipeNo, `[56-03] 建单应回 recipe_no（后端取号）：${cap.bodyRaw}`).not.toBe('');

    // 刷新后真读列值：唯一面料名应出现在「面料名称」列
    await page.reload();
    const row = await locateRow(page, recipeNo, { match: 'first-cell-equals' });
    const fabricCell = await columnCell(page, row, '面料名称');
    await expect(
      fabricCell,
      `[56-03] 刷新后「面料名称」列应显示=${fabricName}（仍空/仍为占位=后端未落 fabric_name 或前端列 prop 漂移，属真实缺陷）`
    ).toContainText(fabricName);

    const recipeId = Number(json.data?.id);
    if (recipeId > 0) {
      await tryCleanup(
        page,
        'DELETE',
        `/production/production-recipes/${recipeId}`,
        '[56-03] 生产配方'
      );
    }
  });

  test('56-04 采购入库新建→刷新：API 建单确认真 2xx →「供应商」列显示自建供应商名（JOIN 富化读回）', async ({
    page,
  }) => {
    const tag = genCode('E56R');
    const { id: supplierId, name: supplierName } = await seedSupplier(page, tag);
    const warehouseId = await seedWarehouse(page, '入库');
    const {
      id: productId,
      code: productCode,
      name: productName,
    } = await seedActiveProduct(page, '入库');

    const receiptDate = new Date().toISOString().slice(0, 10); // 后端 receipt_date 为 NaiveDate(YYYY-MM-DD)
    const created = await apiCall<{ id?: number; receipt_no?: string }>(
      page,
      'POST',
      '/purchase/receipts',
      {
        supplier_id: supplierId,
        receipt_date: receiptDate,
        warehouse_id: warehouseId,
        items: [
          {
            line_no: 1,
            material_id: productId,
            material_code: productCode,
            material_name: productName,
            batch_no: `E56B${tag}`,
            quantity: 5,
            quantity_alt: 0,
            unit_master: 'm',
          },
        ],
      }
    );
    const receiptId = Number(created.data?.id);
    const receiptNo = String(created.data?.receipt_no ?? '');
    expect(
      receiptId,
      `[56-04] 入库建单应回 id（apiCall 非 2xx 会抛错）：${JSON.stringify(created).slice(0, 200)}`
    ).toBeGreaterThan(0);
    expect(
      receiptNo,
      `[56-04] 入库建单应回 receipt_no：${JSON.stringify(created).slice(0, 200)}`
    ).not.toBe('');

    // 刷新一次以模拟用户「保存后看不到」的真实回读路径（入库单已由 API 建好，
    // reload 强制重新拉后端、清空 SPA 内存态）。列表按 created_at 倒序，自建单必在第 1 页，
    // 故同 56-03 直接按入库单号定位自建行，不用供应商下拉绕行（下拉过滤异步填充易超时，属脆弱实现）。
    await safeGoto(page, '/purchase-receipt');
    await page.reload();

    const row = await locateRow(page, receiptNo);
    const supplierCell = await columnCell(page, row, '供应商');
    await expect(
      supplierCell,
      `[56-04] 刷新后「供应商」列应显示自建供应商名=${supplierName}（列 prop=supplier_name 来自后端 JOIN 富化；` +
        `恒空/显示 #id=JOIN 缺失或前端列 prop 漂移，属真实缺陷非用例问题）`
    ).toContainText(supplierName);

    await tryCleanup(page, 'DELETE', `/purchase/receipts/${receiptId}`, '[56-04] 采购入库单');
    await tryCleanup(page, 'DELETE', `/purchase/suppliers/${supplierId}`, '[56-04] 供应商');
  });
});
