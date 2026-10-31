// 报价 E2E 套件 — 04 明细选产品自动带出主档资料
//
// 钉住的新契约（QuotationItemEditor 产品选择改造）：
//  (1) 产品下拉为服务端搜索（filterable remote，走 GET /products?search= 名称/编码模糊匹配）。
//      后端列表接口 page_size 上限为 100（handler clamp），任何「本地全量下拉」都会静默截断，
//      本例用新建产品的编码搜索命中并选中，钉住「首页之外也能选到」这条防截断链路。
//  (2) 选中产品后：单位（只读锁定）与规格（明细实列 sales_quotation_items.specification，可再编辑）
//      由主档带出；产品编码、成分、幅宽、克重为只读主档展示列（明细表无对应列，不落库；
//      报价转销售订单的米/公斤换算链实时回查 products 主档，不读明细快照，见 convert 链断言
//      套件 13-quotation-translation-readback）。
//  (3) 断言基准全部取自 GET /products/{id} 回读的主档真值（非用例造值字面量、非写死 DOM 文本），
//      提交体与落库回读（POST /quotations 请求体 + GET /quotations/{id}）与主档真值逐字相等。
//  (4) 切换产品带出值整体替换、无上一个产品残留；清空产品时带出的单位/规格/只读列一并清空。
import { test, expect, type Page, type Locator } from '../diagnose-fixture';
import {
  loginViaUI,
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  genCode,
  tryCleanup,
} from '../flow/helpers';
import { pickSelectIn } from '../flow/ui-helpers';
import { cellInputByColumn } from '../flow/table-column';
// 产品类型单源常量（与后端 product_type 词表同源）：建品夹具取值必须经成员引用。
import { PRODUCT_TYPE } from '../../src/constants/product-type';

/** GET /products/{id} 回读的产品主档切片（编码为前端契约别名键，别名由后端 append_frontend_aliases 补齐） */
interface ProductMaster {
  id: number;
  product_code: string;
  specification: string | null;
  fabric_composition: string | null;
  /** rust_decimal 序列化为字符串（如 "160.00"），断言按回读原样拼接，不预设数值形态 */
  width: string | number | null;
  gram_weight: string | number | null;
  unit: string;
}

/** 报价明细 GET /quotations/{id} 回读的明细行切片（后端 QuotationItemResponseDto） */
interface QuotationItemRowRead {
  product_id: number;
  specification: string | null;
  unit: string;
}

const CREATED_QUOTATION_IDS: number[] = [];

/** 自建一个资料可辨识的面料产品，返回「GET /products/{id} 回读的主档真值」作为后续断言基准 */
async function createIdentifiableProduct(
  page: Page,
  tag: string,
  specification: string,
  composition: string,
  gramWeight: number,
  width: number
): Promise<ProductMaster> {
  const code = genCode(`E2E-AF${tag}`);
  const created = await apiCall<{ id?: number }>(page, 'POST', '/products', {
    code,
    name: `E2E带出产品${code}`,
    unit: '米',
    category_id: getCtx().productCategoryIds[0],
    // 报价带出主档的面料产品：带克重与幅宽做米↔公斤换算，取面料相容类型（成品布），
    // 三键（幅宽/克重/面料成分）由入参与本键同全，取值经单源常量引用。
    specification,
    product_type: PRODUCT_TYPE.finished,
    fabric_composition: composition,
    gram_weight: gramWeight,
    width,
  });
  const id = created.data?.id;
  if (!id) throw new Error(`前置失败：带出产品(${tag})创建未返回 id: ${JSON.stringify(created)}`);
  // 断言基准 = 回读真值（含 Decimal 序列化为字符串后的原样），不引用上面提交的字面量
  return await apiCallRaw<ProductMaster>(page, 'GET', `/products/${id}`);
}

/** 在明细表第一行的产品下拉里输入编码做服务端搜索并选中；搜索请求命中即钉住防截断链路 */
async function selectProductByCode(page: Page, itemsTable: Locator, code: string) {
  const productSelect = itemsTable.locator('.el-select').first();
  await productSelect.click();
  await productSelect.locator('input').first().fill(code);
  const dropdown = page.locator('.el-select-dropdown:visible').last();
  const option = dropdown.locator('.el-select-dropdown__item').filter({ hasText: code }).first();
  await expect(option, `服务端搜索应命中新建产品编码 ${code}（防分页截断链路）`).toBeVisible({
    timeout: 15000,
  });
  await option.click();
  await dropdown.waitFor({ state: 'hidden', timeout: 15000 }).catch(() => {});
}

/** 悬停产品下拉并点击清空图标（element-plus clearable 的清除按钮类名为 el-select__clear） */
async function clearProductSelection(itemsTable: Locator) {
  const productSelect = itemsTable.locator('.el-select').first();
  await productSelect.hover();
  const clearBtn = productSelect.locator('.el-select__clear');
  await expect(clearBtn, '已选产品时清除图标应可见').toBeVisible({ timeout: 10000 });
  await clearBtn.click();
  // 清空后下拉应回到占位符态（选中值为空）
  await expect(
    productSelect.locator('.el-select__placeholder').filter({ hasText: '选择产品' }).first()
  ).toBeVisible({ timeout: 10000 });
}

test.describe('04 明细选产品自动带出主档资料', () => {
  test.afterEach(async ({ page }) => {
    while (CREATED_QUOTATION_IDS.length) {
      const id = CREATED_QUOTATION_IDS.pop();
      if (id != null) await tryCleanup(page, 'DELETE', `/quotations/${id}`, `quotation#${id}`);
    }
    // 带出产品用后不删。ensureTestEntities 从全局产品池（GET /products 按 code 升序）取
    // productIds 首位并可能对其补写色号，本用例的 E2E-AF 编码恰好排序最前、且为无色的新建产品，
    // 正是被借用写色号的头号对象。用例结束即硬删产品时，同分片并发 worker 稍后向该 id
    // POST /products/{id}/colors 会踩 product_colors 对 products 的外键约束，
    // 后端原样映射为 DATABASE_ERROR，把无关用例（02-approval 夹具阶段）拖红。
    // 通用夹具产品（E2E-P，E2E-QP 系）本仓约定即建而不删，CI 库每次重建，留存不影响断言正确性。
  });

  test('04-01 选中产品带出主档资料：只读列回读主档真值、切换清空不残留、提交与落库与主档一致', async ({
    page,
  }) => {
    test.setTimeout(240_000);
    await loginViaUI(page);
    await ensureTestEntities(page);

    const masterA = await createIdentifiableProduct(
      page,
      'A',
      `AF带出规格A${Date.now()}`,
      '65%棉 35%涤A样',
      200,
      160
    );
    const masterB = await createIdentifiableProduct(
      page,
      'B',
      `AF带出规格B${Date.now()}`,
      '100%锦纶B样',
      120,
      90
    );
    expect(masterA.id, '前置失败：A 产品回读缺 id').toBeTruthy();
    // 断言形态修正  Playwright 的 toBe 走 Object.is 严格相等，不识别 expect.any 非对称匹配器，
    // 对合法字符串也恒红；类型判定必须用 toEqual（深比较支持非对称匹配器），断言强度不变，仍要求该键为字符串
    expect(masterA.product_code, '前置失败：A 产品回读缺 product_code 别名').toEqual(
      expect.any(String)
    );

    await page.goto('/quotations/new');
    await expect(page.locator('form')).toBeVisible({ timeout: 30000 });
    await pickSelectIn(page, page, '客户');
    await page.getByLabel('报价日期').fill('2026-08-19');
    await page.keyboard.press('Enter');
    await page.getByLabel('有效期至').fill('2026-09-19');
    await page.keyboard.press('Enter');
    await pickSelectIn(page, page, '价格条款');
    await pickSelectIn(page, page, '币种');
    await page.getByLabel('汇率').fill('1');
    // 税率必录（sales_quotations.tax_rate NOT NULL，系统不设默认档位）
    await page.getByPlaceholder('按百分数填写，如 13 表示 13%').fill('13');

    await page.getByRole('button', { name: '添加产品' }).click();
    const itemsTable = page.locator('[aria-label="报价明细编辑表"]');
    // 列序与 QuotationItemEditor 表头一一对应：
    // 0 # / 1 产品 / 2 产品编码 / 3 成分 / 4 幅宽 / 5 克重 / 6 色号 / 7 规格 / 8 单位
    const cells = () => itemsTable.locator('.el-table__row').first().locator('td');

    // —— 选中 A：只读带出列与带出快照全部等于 A 主档回读真值 ——
    await selectProductByCode(page, itemsTable, masterA.product_code);
    await expect(cells().nth(2)).toHaveText(masterA.product_code, { timeout: 15000 });
    await expect(cells().nth(3)).toHaveText(String(masterA.fabric_composition));
    await expect(cells().nth(4)).toHaveText(`${masterA.width} cm`);
    await expect(cells().nth(5)).toHaveText(`${masterA.gram_weight} g/m²`);
    // 规格为明细实列快照：带出值应等于主档回读值，且仍可编辑（输入框存在）
    await expect(cells().nth(7).locator('input')).toHaveValue(String(masterA.specification));
    // 单位只读锁定跟随主档交易单位
    await expect(cells().nth(8).locator('input')).toHaveValue(masterA.unit);

    // —— 切换到 B：带出值整体替换为 B 主档真值，A 的值不残留 ——
    await selectProductByCode(page, itemsTable, masterB.product_code);
    await expect(cells().nth(2)).toHaveText(masterB.product_code, { timeout: 15000 });
    await expect(cells().nth(2)).not.toContainText(masterA.product_code);
    await expect(cells().nth(3)).toHaveText(String(masterB.fabric_composition));
    await expect(cells().nth(4)).toHaveText(`${masterB.width} cm`);
    await expect(cells().nth(5)).toHaveText(`${masterB.gram_weight} g/m²`);
    await expect(cells().nth(7).locator('input')).toHaveValue(String(masterB.specification));

    // —— 清空产品：带出的单位/规格与只读列一并清空，不留上一产品数据 ——
    await clearProductSelection(itemsTable);
    await expect(cells().nth(2)).toHaveText('—');
    await expect(cells().nth(3)).toHaveText('—');
    await expect(cells().nth(4)).toHaveText('—');
    await expect(cells().nth(5)).toHaveText('—');
    await expect(cells().nth(7).locator('input')).toHaveValue('');
    await expect(cells().nth(8).locator('input')).toHaveValue('');

    // —— 重新选中 A 并录入数量/单价后保存草稿：
    //    提交体与 GET 回读落库的 specification/unit 必须等于主档真值（界面带出不止步于显示）——
    await selectProductByCode(page, itemsTable, masterA.product_code);
    await expect(cells().nth(7).locator('input')).toHaveValue(String(masterA.specification), {
      timeout: 15000,
    });
    // 数量/单价 纯手写数字录入列 按列头锚定（无步进加减按钮 不依赖换型后消失的 spinbutton 角色）
    const qRow04 = itemsTable.locator('.el-table__row').first();
    const qQty04 = await cellInputByColumn(itemsTable, qRow04, '数量');
    await qQty04.fill('10');
    const qPrice04 = await cellInputByColumn(itemsTable, qRow04, '单价');
    await qPrice04.fill('5');

    const requestPromise = page.waitForRequest(
      req => req.method() === 'POST' && new URL(req.url()).pathname.endsWith('/quotations'),
      { timeout: 30000 }
    );
    await page.getByRole('button', { name: '保存草稿' }).click();
    const createReq = await requestPromise;
    const createBody = createReq.postDataJSON() as {
      items: Array<{ product_id: number; unit: string; specification?: string }>;
    };
    expect(
      createBody.items[0]?.specification,
      '提交体的规格应为带出的主档规格（能落库的落库，不是假象）'
    ).toBe(masterA.specification);
    expect(createBody.items[0]?.unit, '提交体单位应等于主档交易单位').toBe(masterA.unit);

    const createResp = await page
      .waitForResponse(
        resp =>
          resp.request().method() === 'POST' &&
          new URL(resp.url()).pathname.endsWith('/quotations'),
        { timeout: 30000 }
      )
      .then(r => r.json());
    const quotationId = createResp?.data?.id as number | undefined;
    expect(quotationId, '草稿保存应返回报价单 id').toBeTruthy();
    if (quotationId) CREATED_QUOTATION_IDS.push(quotationId);

    const detail = await apiCallRaw<{ items: QuotationItemRowRead[] }>(
      page,
      'GET',
      `/quotations/${quotationId}`
    );
    const line = detail.items?.[0];
    expect(line, '前置失败：回读报价单无明细行').toBeTruthy();
    expect(line.product_id, '落库行产品应为 A').toBe(masterA.id);
    expect(line.specification, '落库规格应逐字等于 A 主档回读值').toBe(masterA.specification);
    expect(line.unit, '落库单位应逐字等于 A 主档回读单位').toBe(masterA.unit);
  });
});
