// P9-3 销售 E2E 套件 — 14 销售价目 · GET /sales/sales-prices/strategies 活体覆盖
//
// ## 本 spec 要补的洞（写它之前 e2e 对 /strategies 是 0 数据断言）
// 全仓 grep 只能找到 `e2e/flow/27-other-modules-full.spec.ts:31` 对 `/sales/sales-prices` 的健康探测
// （`verifyEndpointHealthy` 会把 404/403 也当"健康"，不校验内容），以及 traversal 端点登记表；
// **没有任何一条用例断言过 `/sales-prices/strategies` 真的返回行、返回的是哪些行、字段线格式是什么**。
// 于是后端把 `list_strategies` 从 legacy "价格策略"（price_strategies 表，含 name/description/rules）
// 改成"当前生效价目"（`sales_price_service.rs:270-280` 三判据过滤 + 返回 `sales_price::Model`）这件事，
// e2e 层完全没有防线：判据被删、词表外状态混进结果、Decimal 退化成 JSON 浮点数，都不会被 e2e 抓住。
//
// ## 判据口径（与已落地定案一致，不重开决策）
// - 生效判据三条（backend/src/services/sales_price_service.rs:270-280）：
//   `status=approved` AND `effective_date<=今天` AND `(expiry_date IS NULL OR expiry_date>今天)`；
//   日期一律按后端取源 `chrono::Utc::now().date_naive()`（UTC 日历日），本 spec 的日期串也按 UTC 生成，
//   避免"本地已过零点、后端还是昨天"造成的边界假红。
// - 状态词表：权威 `backend/src/models/status/sales.rs:173-185 price_approval`，销售侧写入方全集
//   只有 pending（建单 :119）/ approved（审批 :162）；`inactive` 无销售侧写入方，
//   DB CHECK `chk_sales_price_status` 只钉 pending/approved（迁移 price_vocab_check/mod.rs:101），
//   契约锁见 `backend/tests/contract_wave8_price_status_parity_test.rs`。
// - Decimal 线格式：`rust_decimal` 未启 `serde-float` ⇒ JSON 里是**字符串**
//   （`src/api/sales-price.ts:4-8` 已按此声明 DecimalString；DB 列 DECIMAL(18,6)，m0011:180）。
// - 响应信封：`ApiResponse<PaginatedResponse<Model>>` ⇒ `data` 里是 `{items,total,page,page_size}`
//   （`backend/src/utils/response.rs:40-45`、handler `sales_price_handler.rs:160-162`），键名以此为准。
//
// ## 反假绿约定
// - 正向断言"我们亲手建并审批的那一行 id 真的出现在结果里"，绝不把"列表恒空/查不到"写成期望；
// - 负向断言配**同页锚点**：先证明本例的正行出现在同一页（结果集确实覆盖到我们的 id 区段），
//   再断言被过滤的行不在其中，排除"因为分页把它挤出去所以看起来像被过滤"的漏检假绿；
//   并对返回页做全称判定（每一行都必须 status=approved），单行漏检也会红；
// - 每条"行不在结果里"之前都先用 `GET /sales/sales-prices/{id}` 回读该行**确实存在且状态/日期如我们所愿**，
//   排除"根本没建成功"造成的假通过；
// - 匿名 401 用例显式 `storageState: { cookies: [], origins: [] }`（照
//   `e2e/enhanced/data-scope-isolation.spec.ts:207-235` 的取证：不指定就会继承主账号 cookie，"匿名"是假前提），
//   并钉 status + 信封机器码 `UNAUTHORIZED` 双判据，**不读错误文案**（本仓鉴权拒绝文案永久脱敏）；
// - 建单/审批的入参按后端 DTO 原样给（`CreateSalesPriceInput` `sales_price_service.rs:24-38`），
//   Decimal 用字符串字面量，不塞浮点（浮点会被 serde 拒成 400）。

import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  API_BASE,
  API_PREFIX,
  BASE_URL,
  apiCall,
  apiCallRaw,
  failureCode,
  genCode,
  tryCleanup,
} from '../flow/helpers';

/** `/sales/sales-prices/strategies` 的 data 形状（PaginatedResponse<sales_price::Model>） */
interface StrategiesData {
  items: SalesPriceRow[];
  total: number;
  page: number;
  page_size: number;
}

/** 后端 `sales_price::Model` 真实列（backend/src/models/sales_price.rs:9-30；无 name/description/rules） */
interface SalesPriceRow {
  id: number;
  product_id: number;
  price: unknown;
  min_order_qty: unknown;
  currency: string;
  unit: string;
  price_type: string;
  effective_date: string;
  expiry_date: string | null;
  status: string;
}

/** 本 spec 创建的行 id，afterEach 逐条尽力清理（DELETE /sales/sales-prices/{id}） */
const CREATED_PRICE_IDS: number[] = [];
test.afterEach(async ({ page }) => {
  while (CREATED_PRICE_IDS.length) {
    const id = CREATED_PRICE_IDS.pop();
    if (id != null) {
      await tryCleanup(page, 'DELETE', `/sales/sales-prices/${id}`, `sales_price#${id}`);
    }
  }
});

/**
 * 取一个真实存在的活跃产品 id 供价目行引用。
 * `sales_prices.product_id` 没有 FOREIGN KEY（m0011:175-194 列清单可查），但本 spec 的 UI 用例要
 * 页面正常渲染、且引用真实实体才是真数据，故仍取真实 id；取不到就抛错（不兜底假 id）。
 */
async function pickProductId(page: Page): Promise<number> {
  const data = await apiCallRaw<{ items: { id: number }[]; total: number }>(
    page,
    'GET',
    '/products?page=1&page_size=1&status=active'
  );
  const id = data.items?.[0]?.id;
  if (typeof id !== 'number') {
    throw new Error(
      `前置缺失：GET /products?status=active 没有返回任何产品（items=${JSON.stringify(data.items)}）` +
        `——价目行需要真实 product_id，禁止兜底假 id`
    );
  }
  return id;
}

/** 按后端取源（Utc::now().date_naive()）生成 UTC 日历日字符串，offsetDays 支持前后偏移 */
function utcDate(offsetDays = 0): string {
  const d = new Date();
  d.setUTCDate(d.getUTCDate() + offsetDays);
  return d.toISOString().slice(0, 10);
}

interface SeedOpts {
  productId: number;
  /** 是否推进到 approved（走真实审批端点，不是直接改库） */
  approved: boolean;
  effectiveDate: string;
  /** undefined = 不传该字段 ⇒ 落库 NULL（判据③的 NULL 分支） */
  expiryDate?: string;
  price?: string;
  marker: string;
}

/**
 * 用真实端点建一条销售价目（建单必落 pending，`sales_price_service.rs:119`），
 * 需要 approved 时再走 `POST /sales/sales-prices/{id}/approve`（:162），
 * 两步各自回读校验：建单后状态必须是 pending，审批后必须是 approved。
 * 任一步不如预期立即抛错——"种子没建成"绝不允许被后面的断言掩盖成"看起来通过"。
 */
async function seedSalesPrice(page: Page, opts: SeedOpts): Promise<number> {
  const body: Record<string, unknown> = {
    product_id: opts.productId,
    price: opts.price ?? '123.456789',
    currency: 'CNY',
    unit: '米',
    price_type: 'STANDARD',
    min_order_qty: '1.00',
    effective_date: opts.effectiveDate,
    customer_type: 'wholesale',
  };
  if (opts.expiryDate !== undefined) body.expiry_date = opts.expiryDate;

  const created = await apiCall<SalesPriceRow>(page, 'POST', '/sales/sales-prices', body);
  const id = created.data.id;
  if (typeof id !== 'number') {
    throw new Error(
      `前置失败：POST /sales/sales-prices 未返回 id，响应=${JSON.stringify(created)}`
    );
  }
  CREATED_PRICE_IDS.push(id);

  expect(
    created.data.status,
    `建单必须落权威词表的 pending（sales_price_service.rs:119），实际 id=${id} status=${created.data.status}`
  ).toBe('pending');
  // Decimal 入参按字符串提交，回读也必须原样是字符串线格式（不被静默转成数字/浮点）
  expect(
    typeof created.data.price,
    `创建响应的 price 必须是 JSON 字符串（rust_decimal 未启 serde-float），实际=${JSON.stringify(created.data.price)}`
  ).toBe('string');

  if (opts.approved) {
    await apiCall(page, 'POST', `/sales/sales-prices/${id}/approve`, {
      approved: true,
      remark: opts.marker,
    });
    const after = await apiCallRaw<SalesPriceRow>(page, 'GET', `/sales/sales-prices/${id}`);
    expect(
      after.status,
      `审批后状态必须是 approved（sales_price_service.rs:162），实际 id=${id} status=${after.status}`
    ).toBe('approved');
  }
  return id;
}

/** 行存在性/状态/日期的独立回读（用于给"不在结果里"补充分性证据，排除"根本没建行"） */
async function assertRowLandedAsExpected(
  page: Page,
  id: number,
  expected: { status: string; effectiveDate: string; expiryDate?: string | null }
): Promise<void> {
  const row = await apiCallRaw<SalesPriceRow>(page, 'GET', `/sales/sales-prices/${id}`);
  expect(row.status, `id=${id} 的 status 应如种子所设`).toBe(expected.status);
  expect(row.effective_date, `id=${id} 的 effective_date 应如种子所设`).toBe(
    expected.effectiveDate
  );
  if (expected.expiryDate === null) {
    expect(
      row.expiry_date,
      `id=${id} 未传 expiry_date 时后端应落 NULL，实际=${JSON.stringify(row.expiry_date)}`
    ).toBeNull();
  } else if (expected.expiryDate !== undefined) {
    expect(row.expiry_date, `id=${id} 的 expiry_date 应如种子所设`).toBe(expected.expiryDate);
  }
}

/** 拉取"当前生效价目"，并先校验信封本身（键名/类型按 PaginatedResponse 原文，不自造键） */
async function fetchStrategies(page: Page, pageSize = 100): Promise<StrategiesData> {
  const res = await apiCall<StrategiesData>(
    page,
    'GET',
    `/sales/sales-prices/strategies?page=1&page_size=${pageSize}`
  );
  expect(res.code, '成功信封 code 必须是 200').toBe(200);
  const data = res.data;
  if (!data) throw new Error(`/strategies 的 data 缺失，响应=${JSON.stringify(res)}`);
  for (const key of ['items', 'total', 'page', 'page_size'] as const) {
    expect(Object.prototype.hasOwnProperty.call(data, key), `PaginatedResponse 缺少键 ${key}`).toBe(
      true
    );
  }
  expect(
    Array.isArray(data.items),
    `data.items 必须是数组，实际=${JSON.stringify(data.items)}`
  ).toBe(true);
  expect(typeof data.total, `data.total 必须是数字，实际=${JSON.stringify(data.total)}`).toBe(
    'number'
  );
  expect(typeof data.page, 'data.page 必须是数字').toBe('number');
  expect(typeof data.page_size, 'data.page_size 必须是数字').toBe('number');
  expect(data.page, '请求 page=1 应原样回显').toBe(1);
  expect(data.page_size, `请求 page_size=${pageSize} 应原样回显`).toBe(pageSize);
  return data;
}

/** 在结果里按 id 取行；取不到时抛带 total/首屏 id 的诊断错误（而非静默 undefined 让后续断言空转） */
function rowById(data: StrategiesData, id: number, context: string): SalesPriceRow {
  const row = data.items.find(r => r.id === id);
  if (!row) {
    throw new Error(
      `${context}：应在"当前生效价目"结果里看到本例自建行 id=${id}，但没有。\n` +
        `total=${data.total} 本页条数=${data.items.length} 本页 id=${JSON.stringify(data.items.map(r => r.id))}\n` +
        `判据：后端按 id 倒序（sales_price_service.rs:279），本例行是刚建的（id 最大），正常情况下必在第 1 页。`
    );
  }
  return row;
}

test.describe('14 销售价目 · 当前生效价目（/sales-prices/strategies）', () => {
  test.beforeEach(async ({ context }) => {
    // 真实登录（非 mock 业务接口）：拿 access_token cookie + 存活 CSRF，供 apiCall 复用
    await applyAuthMocks(context);
  });

  test('14-01 建单→审批→当前生效价目真的返回该行，且 price 为十进制字符串线格式', async ({
    page,
  }) => {
    const productId = await pickProductId(page);
    const marker = genCode('SP14A');
    const id = await seedSalesPrice(page, {
      productId,
      approved: true,
      effectiveDate: utcDate(0),
      price: '123.456789',
      marker,
    });

    const data = await fetchStrategies(page);
    const row = rowById(data, id, '14-01');

    expect(row.status, `id=${id} 出现在生效价目里时状态必须是 approved`).toBe('approved');
    expect(row.product_id, `id=${id} 的 product_id 应回显种子引用`).toBe(productId);
    expect(row.effective_date, `id=${id} 的生效日应为今天(UTC)`).toBe(utcDate(0));

    // 线格式契约：Decimal ⇒ JSON 字符串（不是 number）。typeof 判定是对**已解析 JSON**的运行时判定，
    // 类型声明不参与其中，故它真能抓住"后端改启 serde-float / 前端拿到浮点"的回归。
    expect(typeof row.price, `price 必须是字符串线格式，实际类型=${typeof row.price}`).toBe(
      'string'
    );
    expect(row.price, `DB 列 DECIMAL(18,6) ⇒ 6 位定点字符串`).toBe('123.456789');
    expect(String(row.price)).toMatch(/^-?\d+(\.\d{1,6})?$/);
    expect(Number(row.price), '字符串值本身必须等于提交的十进制定点值').toBe(123.456789);
    expect(typeof row.min_order_qty, 'min_order_qty 同样必须是字符串线格式').toBe('string');
    expect(row.min_order_qty).toBe('1.00');

    // 分页元数据与 items 同源（不许 total 与 items 各说一套）
    expect(data.total, 'total 至少覆盖本例这一行').toBeGreaterThanOrEqual(1);
    expect(data.items.length, '本页条数不得超过请求的 page_size').toBeLessThanOrEqual(100);

    // 全称不变式：结果里任何一行都必须是 approved（词表外的行混进来即判据①失效）
    for (const r of data.items) {
      expect(r.status, `id=${r.id} 不是 approved 却出现在"当前生效价目"里`).toBe('approved');
    }
  });

  test('14-02 pending 行必须被过滤掉（同页锚点 + 回读存在性双证，防分页漏检假绿）', async ({
    page,
  }) => {
    const productId = await pickProductId(page);

    const anchorId = await seedSalesPrice(page, {
      productId,
      approved: true,
      effectiveDate: utcDate(0),
      price: '111.111111',
      marker: genCode('SP14B-anchor'),
    });
    const pendingId = await seedSalesPrice(page, {
      productId,
      approved: false,
      effectiveDate: utcDate(0),
      price: '222.222222',
      marker: genCode('SP14B-pending'),
    });
    // 锚点前提：pending 行是后建的（id 更大）⇒ 后端 id 倒序下它排在锚点之前，
    // 若过滤判据被删，它必然出现在第 1 页，负向断言因此不可能因分页而空转。
    expect(
      pendingId,
      'pending 行必须比锚点行更新（id 更大），否则本例的"同页锚点"前提不成立'
    ).toBeGreaterThan(anchorId);

    await assertRowLandedAsExpected(page, pendingId, {
      status: 'pending',
      effectiveDate: utcDate(0),
      expiryDate: null,
    });

    const data = await fetchStrategies(page);
    // ① 锚点：同一页确实覆盖到了我们的行
    rowById(data, anchorId, '14-02 锚点');
    // ② 负向：pending 行不在生效价目里
    const leaked = data.items.find(r => r.id === pendingId);
    expect(
      leaked,
      `pending 行 id=${pendingId} 不得出现在"当前生效价目"里（判据① status=approved）；` +
        `实际把它返回了：${JSON.stringify(leaked)}`
    ).toBeUndefined();
    // ③ 全称：整页不得有任何非 approved 行（哪怕不是本例的行）
    for (const r of data.items) {
      expect(r.status, `id=${r.id} status=${r.status} 不合法：判据①失效`).toBe('approved');
    }
  });

  test('14-03 生效日在未来 / 已到期 / 到期日恰为今天 的 approved 行都不出现', async ({ page }) => {
    const productId = await pickProductId(page);
    const today = utcDate(0);

    const anchorId = await seedSalesPrice(page, {
      productId,
      approved: true,
      effectiveDate: today,
      price: '300.000001',
      marker: genCode('SP14C-anchor'),
    });
    const futureId = await seedSalesPrice(page, {
      productId,
      approved: true,
      effectiveDate: utcDate(3),
      price: '300.000002',
      marker: genCode('SP14C-future'),
    });
    const expiredId = await seedSalesPrice(page, {
      productId,
      approved: true,
      effectiveDate: utcDate(-30),
      expiryDate: utcDate(-1),
      price: '300.000003',
      marker: genCode('SP14C-expired'),
    });
    // 边界：判据③口径是 `expiry_date > today`，故到期日=今天即"已失效"，不得出现
    const expiryTodayId = await seedSalesPrice(page, {
      productId,
      approved: true,
      effectiveDate: utcDate(-5),
      expiryDate: today,
      price: '300.000004',
      marker: genCode('SP14C-edge'),
    });
    // 边界：到期日=明天 ⇒ 仍在有效期内，必须出现（负例的另一侧，防止"整条判据被删宽成永不过期"）
    const expiryTomorrowId = await seedSalesPrice(page, {
      productId,
      approved: true,
      effectiveDate: utcDate(-1),
      expiryDate: utcDate(1),
      price: '300.000005',
      marker: genCode('SP14C-live2'),
    });

    for (const id of [futureId, expiredId, expiryTodayId]) {
      await assertRowLandedAsExpected(page, id, {
        status: 'approved',
        effectiveDate: id === futureId ? utcDate(3) : id === expiredId ? utcDate(-30) : utcDate(-5),
        expiryDate: id === expiredId ? utcDate(-1) : id === expiryTodayId ? today : null,
      });
    }

    const data = await fetchStrategies(page);
    rowById(data, anchorId, '14-03 锚点（判据全集满足的正例）');
    rowById(data, expiryTomorrowId, '14-03 正例（expiry=明天，仍在有效期内）');

    for (const [id, why] of [
      [futureId, `effective_date=${utcDate(3)} 晚于今天 ⇒ 判据②（effective_date<=today）违反`],
      [expiredId, `expiry_date=${utcDate(-1)} 早于今天 ⇒ 判据③违反`],
      [expiryTodayId, `expiry_date=${today} 等于今天 ⇒ 判据③违反（口径是 >today，非 >=today）`],
    ] as [number, string][]) {
      const leaked = data.items.find(r => r.id === id);
      expect(
        leaked,
        `id=${id} 必须不在"当前生效价目"里（${why}）；实际被返回：${JSON.stringify(leaked)}`
      ).toBeUndefined();
    }
  });

  test('14-04 UI 活体：『价格策略』按钮打开『当前生效价目』对话框并真实呈现该行', async ({
    page,
  }) => {
    const productId = await pickProductId(page);
    const id = await seedSalesPrice(page, {
      productId,
      approved: true,
      effectiveDate: utcDate(0),
      price: '234.560000',
      marker: genCode('SP14D'),
    });

    await page.goto('/sales-price');
    // 按钮文案 = salesPrice.index.buttonPriceStrategy（zh '价格策略'，locales 现值，非 t() 兜底字面量）
    await page.getByRole('button', { name: '价格策略', exact: true }).click();

    // 对话框标题 = salesPrice.index.effectiveDialogTitle（zh '当前生效价目'）
    await expect(
      page.getByRole('dialog', { name: '当前生效价目对话框' }),
      '点击价格策略后应打开 aria-label 为『当前生效价目对话框』的对话框'
    ).toBeVisible();
    await expect(page.getByText('当前生效价目', { exact: true })).toBeVisible();

    // 对话框表格列按 sales_price::Model 真实列呈现（index.vue:80-133），
    // 价格走 spFmts.formatCurrency ⇒ '¥' + 6 位定点
    const dialog = page.getByRole('dialog', { name: '当前生效价目对话框' });
    const row = dialog.locator('.el-table__row').filter({ hasText: '¥234.560000' });
    await expect(
      row.first(),
      `对话框内应看到本例自建生效价目行（id=${id} product_id=${productId} price=¥234.560000）`
    ).toBeVisible();
    await expect(row.first()).toContainText(String(productId));
    // 状态标签走 salesPrice.statusLabels.approved（zh '已审批'）
    await expect(row.first()).toContainText('已审批');
  });

  test('14-05 匿名访问 /strategies 必须 401（status + 信封机器码双钉，不读文案）', async ({
    browser,
  }) => {
    // 必须显式空 storageState：不指定会继承 playwright.config.ts:66 的主账号 cookie（假前提）
    const anon = await browser.newContext({
      baseURL: BASE_URL,
      storageState: { cookies: [], origins: [] },
    });
    try {
      const resp = await anon.request.fetch(
        `${API_BASE}${API_PREFIX}/sales/sales-prices/strategies?page=1&page_size=10`,
        { method: 'GET', headers: { 'X-Requested-With': 'XMLHttpRequest' } }
      );
      const body = await resp.json().catch(() => null);
      expect(
        resp.status(),
        `无认证态访问应 401，实际 status=${resp.status()} body=${JSON.stringify(body)}`
      ).toBe(401);
      // 机器码双钉：backend/src/utils/error.rs:707 CODE_UNAUTHORIZED="UNAUTHORIZED"
      // （由 utils/response.rs:140-142 unauthorized_response 走统一 ErrorResponse 产出）。
      // 不读 message：鉴权拒绝文案永久脱敏，读文案只会空转。
      expect(
        failureCode(body),
        `401 必须归因到鉴权机器码 UNAUTHORIZED，实际 body=${JSON.stringify(body)}`
      ).toBe('UNAUTHORIZED');
    } finally {
      await anon.close();
    }
  });
});
