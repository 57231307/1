// 报价 E2E 套件 — 13 报价翻译回读（转订单字段真值 + 复制为新单真实预填）
//
// 本 spec 补齐审计（见交付报告「转采翻译不回读」「复制预填锚定共享行 nth(1) 不稳」）在 CI 侧可测的两条交易链缺口：
//
//  (A) 报价→销售订单 转采翻译回读（原 02-03 仅断 converted_sales_order_id 有值，未回读订单/明细的
//      金额、数量、单位、米↔公斤换算是否被正确翻译落库）。
//      依据后端 services/quotation_convert_service.rs::copy_quotation_items_to_order：
//        order_item.quantity        ← quotation_item.quantity
//        order_item.unit_price      ← quotation_item.unit_price
//        order_item.subtotal        ← quotation_item.amount          (= round(qty*unit_price,2)，见 quotation_ops/crud.rs:173)
//        order_item.total_amount    ← quotation_item.amount_with_tax (= round(qty*unit_price_with_tax,2)，crud.rs:174)
//        order_item.quantity_meters / quantity_kg ← DualUnitConverter 按报价行单位 + 产品克重/幅宽换算
//        order_item.product_id      ← quotation_item.product_id
//      报价专用产品（ctx.quotationProductId）单位=米、gram_weight=180、width=150，
//      故 qty 米 → quantity_meters=qty；quantity_kg = qty * 180 * 150 / 100000 = qty*0.27。
//      以固定 qty=40 / unit_price=12.5 / 税 0（tax_inclusive=false, tax_rate=0, unit_price_with_tax=12.5）使
//      subtotal==total_amount==500、quantity_meters==40、quantity_kg==10.8，逐项硬断言（非仅产出 id）。
//
//  (B) 复制为新单字段真实预填回读（原 sales/01-05 用 `tr:nth(1)` 抓共享行、且仅断「客户非空」）。
//      本例改为：先 API 自建一张「值可辨识」的源报价单（qty=40 / 单价=12.5 / 含税=12.5 / 单位=米 /
//      product=ctx.quotationProductId），经列表行「复制为新单」进入新建页（copyFrom 路由，list.vue:333 →
//      create.vue loadForCopy），回读新建页明细行预填的数值（inputValue 数值化解析，避免 EP precision=2 文本格式脆断），
//      再保存为新草稿 → GET /quotations/{新单} 回读落库明细 quantity/unit_price/unit/product_id 与源单逐字相等，
//      且源单未被覆盖（新单 id ≠ 源单 id、源单明细不变）。全程锚定本用例自建单，不依赖列表行序。
//
// 会话：默认 admin（loginViaUI/applyAuthMocks 走 TEST_USERNAME=admin，ensureTestEntities 亦 admin）。
// 销售订单明细经 GET /sales/orders/{id} 回读：admin(role_id=1) 不触发 sales_order_handler 的非管理员金额字段脱敏分支，
// unit_price/subtotal/total_amount 原值可取（否则会是真实契约问题，不予放宽掩盖）。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  loginViaUI,
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  tryCleanup,
  BASE_URL,
} from '../flow/helpers';
import { pickListArray } from '../flow/ui-helpers';

/** 报价明细行响应键（models/quotation_response_dto.rs::QuotationItemResponseDto，DECIMAL 经 JSON 为字符串） */
interface QuotationItemLite {
  id: number;
  product_id: number;
  unit: string;
  quantity: number | string;
  unit_price: number | string;
  unit_price_with_tax: number | string;
  amount: number | string;
  amount_with_tax: number | string;
}

/** GET /sales/orders/{id} 明细行（services/so/order_query.rs::SalesOrderItemDetail，admin 会话不脱敏金额） */
interface SalesOrderItemLite {
  id: number;
  product_id: number;
  quantity: number | string;
  unit_price: number | string;
  subtotal: number | string;
  tax_amount: number | string;
  total_amount: number | string;
  quantity_meters: number | string | null;
  quantity_kg: number | string | null;
}

/** 自建一张「值可辨识」草稿报价单（qty=40 / 单价 12.5 / 含税 12.5 / 税 0），引用报价专用产品；返回 id+单号。 */
async function seedRecognizableDraft(page: Page, noteTag: string): Promise<number> {
  const ctx = getCtx();
  if (!ctx.quotationProductId) throw new Error('前置缺失：ctx.quotationProductId 未就绪');
  if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
  if (!ctx.userIds[0]) throw new Error('前置缺失：ctx.userIds[0] 未就绪');
  const res = await apiCall<{ id?: number }>(page, 'POST', '/quotations', {
    customer_id: ctx.customerId,
    sales_user_id: ctx.userIds[0],
    quotation_date: new Date().toISOString().slice(0, 10),
    valid_until: new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10),
    currency: 'CNY',
    exchange_rate: '1',
    base_currency: 'CNY',
    price_terms: 'FOB',
    tax_inclusive: false,
    tax_rate: '0',
    items: [
      {
        product_id: ctx.quotationProductId,
        unit: ctx.quotationProductUnit,
        quantity: '40',
        unit_price: '12.5',
        unit_price_with_tax: '12.5',
      },
    ],
    notes: `E2E-13-${noteTag}`,
  });
  const id = res.data?.id;
  expect(id, `前置失败：报价单创建未返回 id（${noteTag}）`).toBeTruthy();
  const created = await apiCallRaw<{ status: string; total_amount: number | string }>(
    page,
    'GET',
    `/quotations/${id}`
  );
  expect(created.status, '自建报价单应为 draft').toBe('draft');
  // 源单金额先行回读锁定（qty*price=500），确保「预填回读」比对的基准本身正确
  expect(Number(created.total_amount), '源报价单总额应为 40*12.5=500').toBe(500);
  return id as number;
}

const CREATED_QUOTATION_IDS: number[] = [];
test.afterEach(async ({ page }) => {
  while (CREATED_QUOTATION_IDS.length) {
    const id = CREATED_QUOTATION_IDS.pop();
    if (id != null) await tryCleanup(page, 'DELETE', `/quotations/${id}`, `quotation#${id}`);
  }
});

test.describe('13 报价翻译回读', () => {
  test('13-01 报价转销售订单后回读订单明细的金额/数量/单位换算真值', async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
    const ctx = getCtx();
    const sourceId = await seedRecognizableDraft(page, 'CONV');
    CREATED_QUOTATION_IDS.push(sourceId);

    // draft → approved（金额 500 < 10 万，submit 自批）
    await apiCall(page, 'POST', `/quotations/${sourceId}/submit`);
    const afterSubmit = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/quotations/${sourceId}`
    );
    if (afterSubmit.status !== 'approved') {
      // 若走了 BPM 审批态则补 approve，保证 convert 前置 approved（真实流转，不跳过）
      await apiCall(page, 'POST', `/quotations/${sourceId}/approve`);
    }
    expect(
      await apiCallRaw<{ status: string }>(page, 'GET', `/quotations/${sourceId}`).then(
        d => d.status
      ),
      '转订单前置：报价单应为 approved'
    ).toBe('approved');

    // 转订单并取回 converted_sales_order_id（02-03 已证 UI「转销售订单」按钮→converted；此处 API 触发同源）
    await apiCall(page, 'POST', `/quotations/${sourceId}/convert`);
    const converted = await apiCallRaw<{ status: string; converted_sales_order_id?: number }>(
      page,
      'GET',
      `/quotations/${sourceId}`
    );
    expect(converted.status, '转订单后报价单状态应为 converted').toBe('converted');
    const soId = converted.converted_sales_order_id;
    expect(soId, '转订单应回填 converted_sales_order_id').toBeTruthy();

    // 回读订单明细：非仅断产出 id，逐项验证翻译正确性
    const order = await apiCallRaw<{
      items?: unknown;
      total_amount: number | string;
    }>(page, 'GET', `/sales/orders/${soId}`);
    const items = pickListArray<SalesOrderItemLite>(
      { items: order.items },
      'items',
      `销售订单 ${soId} 明细`
    );
    expect(items.length, `转生成的销售订单应含 1 行明细，实际 ${items.length}`).toBe(1);
    const line = items[0];

    expect(Number(line.quantity), '订单行数量应逐值翻译自报价行 40').toBe(40);
    expect(Number(line.unit_price), '订单行单价应翻译自报价行 12.5').toBeCloseTo(12.5, 2);
    // 金额翻译：税 0 → subtotal=total_amount=amount=40*12.5=500
    expect(Number(line.subtotal), '订单行小计应等于报价行 amount=500').toBeCloseTo(500, 2);
    expect(Number(line.total_amount), '订单行总额应等于报价行 amount_with_tax=500').toBeCloseTo(
      500,
      2
    );
    expect(Number(line.tax_amount), '税率为 0 时税额应为 0').toBe(0);
    // 单位→米↔公斤换算（米制：meters=qty，kg=qty*克重*幅宽/100000=40*0.27=10.8）
    expect(Number(line.quantity_meters), '米数应按报价单位「米」直译为 40').toBeCloseTo(40, 2);
    expect(
      Number(line.quantity_kg),
      '公斤数应按产品克重180×幅宽150 换算 40*180*150/100000=10.8'
    ).toBeCloseTo(10.8, 1);
    // 产品外键翻译正确（指向报价所引用的产品）
    expect(line.product_id, '订单行产品应为报价所引用的产品').toBe(ctx.quotationProductId);
    // 订单头总额也应与翻译一致（create_order_from_quotation: total_amount=quotation.total_amount）
    expect(Number(order.total_amount), '订单头总额应翻译自报价总额 500').toBeCloseTo(500, 2);
  });

  test('13-02 复制为新单：新建页预填回读 + 保存后落库字段与源单逐值相等', async ({ page }) => {
    await applyAuthMocks(page.context());
    await ensureTestEntities(page);

    const sourceId = await seedRecognizableDraft(page, 'COPY');
    CREATED_QUOTATION_IDS.push(sourceId);
    const sourceItems = pickListArray<QuotationItemLite>(
      await apiCallRaw<{ items?: unknown }>(page, 'GET', `/quotations/${sourceId}`),
      'items',
      '源报价单明细'
    );
    expect(sourceItems.length, '源报价单应有 1 行明细').toBe(1);

    // 从报价单列表定位「本用例自建单」行（按后端单号收敛，杜绝 tr:nth(1) 抓共享行），点「复制为新单」
    const source = await apiCallRaw<{ quotation_no: string }>(
      page,
      'GET',
      `/quotations/${sourceId}`
    );
    await page.goto(`${BASE_URL}/quotations`);
    const row = page.getByRole('row').filter({ hasText: source.quotation_no }).first();
    await expect(row, `列表应能筛出本例自建单 ${source.quotation_no}`).toBeVisible({
      timeout: 30000,
    });
    await row.getByRole('button', { name: '复制为新单' }).click();

    // 复制态跳新建页（list.vue:333 push /quotations/new?copyFrom=<id>），create.vue loadForCopy 预填
    await expect(page).toHaveURL(/\/quotations\/new\?copyFrom=\d+$/, { timeout: 30000 });

    // 预填回读①：客户确已选中（非占位透明项）——证明 loadForCopy 真正填充了表头
    const customerSelected = page
      .locator('.el-form-item')
      .filter({ has: page.locator('.el-form-item__label', { hasText: /^\s*\*?\s*客户\s*$/ }) })
      .first()
      .locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)');
    await expect(customerSelected.first(), '复制预填后客户应非空').not.toHaveText('', {
      timeout: 15000,
    });

    // 预填回读②：明细行数量/单价/含税单价数值化解析（precision=2 文本不稳，解析为 number 再断言）。
    // QuotationItemEditor 明细行仅 3 个 el-input-number（数量/单价/含税单价），金额列为展示文本非输入。
    const itemsTable = page.locator('[aria-label="报价明细编辑表"]');
    await expect(itemsTable, '复制预填后明细表应渲染').toBeVisible({ timeout: 15000 });
    const spins = itemsTable.getByRole('spinbutton');
    await expect(spins.first(), '预填后明细行应渲染数量输入').toBeVisible({ timeout: 15000 });
    const qtyVal = Number(await spins.nth(0).inputValue());
    const priceVal = Number(await spins.nth(1).inputValue());
    const priceTaxVal = Number(await spins.nth(2).inputValue());
    expect(qtyVal, `新建页数量应预填为源值 40，实际 ${await spins.nth(0).inputValue()}`).toBe(40);
    expect(
      priceVal,
      `新建页单价应预填为源值 12.5，实际 ${await spins.nth(1).inputValue()}`
    ).toBeCloseTo(12.5, 2);
    expect(
      priceTaxVal,
      `新建页含税单价应预填为源值 12.5，实际 ${await spins.nth(2).inputValue()}`
    ).toBeCloseTo(12.5, 2);

    // 保存为「新」草稿：捕获 POST /quotations（跳过 CSRF 竞败 403 中间态）取新单 id
    const createdResp = page
      .waitForResponse(
        r =>
          r.request().method() === 'POST' &&
          /\/quotations(\?|$)/.test(r.url()) &&
          !/\/quotations\/\d+/.test(r.url()) &&
          r.status() !== 403,
        { timeout: 30000 }
      )
      .catch(() => null);
    await page.getByRole('button', { name: '保存草稿' }).click();
    await expect(page.getByText('草稿保存成功')).toBeVisible({ timeout: 30000 });
    const resp = await createdResp;
    expect(resp, '未捕获到复制保存的 POST /quotations 响应').not.toBeNull();
    const createdBody = (await resp!.json()) as { data?: { id?: number } };
    const newId = createdBody.data?.id;
    expect(newId, '复制保存未返回新单 id').toBeTruthy();
    CREATED_QUOTATION_IDS.push(newId as number);
    expect(newId, '复制应生成新单（id 不得等于源单）').not.toBe(sourceId);

    // 落库回读：新单明细 quantity/unit_price/unit/product_id 与源单逐值相等
    const copyDetail = await apiCallRaw<{ status: string; items?: unknown }>(
      page,
      'GET',
      `/quotations/${newId}`
    );
    expect(copyDetail.status, '复制生成的新单应为 draft 态').toBe('draft');
    const copyItems = pickListArray<QuotationItemLite>(
      { items: copyDetail.items },
      'items',
      '复制新单明细'
    );
    expect(copyItems.length, '复制新单应有 1 行明细').toBe(1);
    const src = sourceItems[0];
    const cp = copyItems[0];
    expect(Number(cp.quantity), '复制新单数量应与源一致 40').toBe(Number(src.quantity));
    expect(Number(cp.unit_price), '复制新单单价应与源一致 12.5').toBeCloseTo(
      Number(src.unit_price),
      2
    );
    expect(Number(cp.unit_price_with_tax), '复制新单含税单价应与源一致').toBeCloseTo(
      Number(src.unit_price_with_tax),
      2
    );
    expect(cp.unit, '复制新单单位应逐字继承源单单位').toBe(src.unit);
    expect(cp.product_id, '复制新单产品应与源一致').toBe(src.product_id);

    // 源单未被覆盖：明细 id/数量不变（证明复制走的是新建，不污染源单）
    const srcAfter = pickListArray<QuotationItemLite>(
      await apiCallRaw<{ items?: unknown }>(page, 'GET', `/quotations/${sourceId}`),
      'items',
      '源报价单明细（复制后）'
    );
    expect(Number(srcAfter[0].quantity), '复制不应改源单数量').toBe(40);
    expect(srcAfter[0].id, '复制不应覆盖源明细行').toBe(src.id);
  });
});
