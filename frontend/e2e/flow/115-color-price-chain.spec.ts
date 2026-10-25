import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  APP_ERROR_CODES,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
} from './helpers';

/**
 * 115 色卡价格族建价 改价 批量调价 审批 删除 首个真契约覆盖
 *
 * 功能。把 POST /api/v1/erp/color-prices 建价、PUT /api/v1/erp/color-prices 冒号 id 改价、
 * POST /api/v1/erp/color-prices 冒号 id 冒号 approve 审批、
 * POST /api/v1/erp/color-prices/batch-adjust 批量调价、
 * DELETE /api/v1/erp/color-prices 冒号 id 软删 五个写口钉成真实落库回读链，
 * 并把校验门、阈值分派、状态门与重复动作拒绝钉到机器码与零漂移两个口径。
 *
 * 调用方。Playwright 收集器按 flow 目录发现本 spec，worker 会话由 e2e/diagnose-fixture 的
 * worker 作用域 storageState 注入，本链全程走 apiCall 不导航页面，因此不重复登录，
 * 只在 beforeEach 用 /auth/me 探活证明会话可用（写口依赖它的 CSRF 与凭据前置）。
 *
 * 入参。宿主产品与色号自建自持，色号价格提交体按 CreateColorPriceDto 与 UpdateColorPriceDto
 * 与 BatchAdjustPriceDto 与 ApproveColorPriceDto 的字段与可空性对齐，金额与百分比用字符串
 * 承载 rust_decimal，出参 base_price 用 Number 归一后对表。
 *
 * 传给谁。请求经 apiCall 走 page.request.fetch 到后端 Axum 路由，落 handlers/color_price_handler.rs
 * 的 create_color_price 与 update_color_price 与 delete_color_price 与 batch_adjust_color_prices
 * 与 approve_color_price，服务层落到 services/color_price_crud_service.rs 与
 * services/color_price_batch_service.rs。
 *
 * 存什么。产品落 products 一行；色号落 product_colors 一行；价格落 product_color_prices 一行，
 * 服务端建价初态写 approval_status 等 APPROVED 与 is_active 真与 priority 零与 created_by 当前用户，
 * 且 approved_by 与 approved_at 保持 null；批量调价对未越阈值的项写新 base_price 与 APPROVED 与
 * 审批时间戳，对越阈值的项保持 base_price 原值仅将 approval_status 改为 PENDING 并在
 * color_price_history 追加一条 change_type 为 batch 的历史；审批 APPROVED 从最近一条历史取 new_price
 * 落到 base_price 并置 APPROVED；审批 REJECTED 只改 approval_status 到 REJECTED 且不改 base_price；
 * DELETE 只把 is_active 置 false 属软删语义。
 *
 * 存哪里。products 与 product_colors 由 catalogue 域主表承载；product_color_prices 与
 * color_price_history 由本域主表与历史表承载；审批审计经 audit_log_service 的 update_with_audit
 * 落到 audit_logs。宿主数据与价格记录只在本用例内可见，tag 前缀保证并发车道互不冲突。
 *
 * 契约真相。逐段读后端源，非推测。
 * 建价初态与列归属（services/color_price_crud_service.rs 的 create）：
 * approval_status 恒写 APPROVED 常量、is_active 真、priority 缺省零、created_by 由 handler
 * 注入 auth.user_id、approved_by 与 approved_at 保持 None；这些列前端提交无效，只以服务端出参为准。
 * 校验门（validate_currency 与 validate_season 与 validate_customer_level 与 validate_quantity_range
 * 与 validate_date_range）全部走 CrudError::Validation 映射到 AppError::validation_displayable，
 * 即 HTTP 400 加 VALIDATION_ERROR 且外显真实拒绝文案；币种门代理到 services/currency_service 的
 * ISO 4217 白名单，本域不写第二份清单；季节只允许 SS 与 AW 与 HOLIDAY；客户等级只允许
 * constants/customer_tier::ALLOWED 集合 VIP 与 GOLD 与 SILVER 与 NORMAL。
 * 改价门（update）：先对 currency 与 season 与 customer_level 逐枚校验再走 ActiveModel 赋值链，
 * 校验失败即整笔返回，同请求里其他合法列不写；NotFound 映射 404。
 * 删除门（delete）：NotFound 映射 404，已 is_active 为假返回 CrudError::InvalidState，
 * handler 走 AppError::business 脱敏族即 400 BUSINESS_ERROR 且不外显内部文案。
 * 批量调价（services/color_price_batch_service.rs 的 batch_adjust）：
 * 先 fetch_prices_map 一次性取全部 price_id，缺失即 PriceNotFound(id) 映射 AppError::not_found
 * 脱敏 404 加 NOT_FOUND；随后逐条 calculate_new_price，adjustment_type 只允许 percentage 与 fixed，
 * 越界走 BatchError::Validation 映射 AppError::validation 脱敏 400 加 VALIDATION_ERROR；
 * 未越阈走 auto 分支写新 base_price 与 APPROVED 与审批人时间戳，越阈走 pending 分支
 * 保持 base_price 原值仅改 approval_status 到 PENDING；阈值取自 APPROVAL_THRESHOLD 常量 0.10；
 * 出参三键 auto_approved 与 pending_approval 与 total，数组顺序与请求 items 顺序同源；
 * 任何一步早退都在 insert_history_records 与 apply_price_updates 之前抛出，同批的合法项与
 * 同批的历史行均不落库。
 * 审批（approve）：事务内 lock_exclusive 取价，NotFound 走 PriceNotFound 映射 404；
 * approval_status 非 PENDING 返回 BatchError::InvalidState 走 AppError::business_displayable
 * 即 400 BUSINESS_ERROR 且外显固定文案 "价格不处于待审批状态，无法审批"；
 * decision 只允许 APPROVED 与 REJECTED 两个大写常量，越界走 BatchError::Validation
 * 映射 AppError::validation 脱敏 400 加 VALIDATION_ERROR；
 * APPROVED 分支从最近一条历史取 new_price 落到 base_price 并置 APPROVED 与审批人时间戳，
 * REJECTED 分支只改 approval_status 到 REJECTED 且不改 base_price 与不覆盖历史审批位。
 *
 * 假绿防线。每一条写操作都紧跟对象口逐列回读，未回读列不作为通过依据；
 * 门拒一律钉 HTTP 400 或 404 与机器码，可外显族再钉文案原文，被拒后一律回读断零漂移；
 * 阈值分派断两条独立口径，pending 分支既断 approval_status 又断 base_price 未改；
 * auto 分支既断 base_price 新值又断 approval_status 与审批人时间戳非空；
 * 批量里非法 adjust_type 用同批一合法加一非法证明整笔不落；
 * 重复审批与重复删除都用回读钉状态与列零变化；
 * 全程不用 ?? 或 或 兜底，后端必出的键缺失即形漂判红。
 */

type Row = Record<string, unknown>;

/** 色号价格出参的可见列（handlers/color_price_handler.rs 的 ColorPriceDetail） */
interface ColorPriceDetailRow {
  id: number;
  product_id: number;
  color_id: number;
  currency: string;
  base_price: string | number;
  effective_from: string;
  effective_to: string | null;
  customer_level: string | null;
  min_quantity: string | number | null;
  max_quantity: string | number | null;
  customer_id: number | null;
  season: string | null;
  is_active: boolean;
  priority: number;
  notes: string | null;
  created_by: number | null;
  approved_by: number | null;
  approved_at: string | null;
  approval_status: string;
  created_at: string;
  updated_at: string;
}

/** 批量调价出参三键（handlers/color_price_handler.rs 的 batch_adjust_color_prices） */
interface BatchAdjustResp {
  auto_approved: number[];
  pending_approval: number[];
  total: number;
}

/** 价格历史出参分页信封（handlers/color_price_handler.rs 的 get_color_price_history） */
interface HistoryResp {
  items: Array<{
    id: number;
    product_color_price_id: number;
    old_price: string | number;
    new_price: string | number;
    change_type: string;
    change_percent: string | number | null;
    approved_by: number | null;
  }>;
  total: number;
  page: number;
  page_size: number;
}

/** 分页信封固定四键 */
type Paged<T> = { items: T[]; total: number; page: number; page_size: number };

/**
 * 必然不存在于 product_color_prices 的探测主键。
 * 入参 无。传给谁 三个写口的不存在分支，先经对象口证明主键无行。
 * 存什么 一个落在 i64 正向内且远高于 CI 库真实自增值的常量。
 * 存哪里 不落盘，只在前置证明与断言里使用。
 */
const ABSENT_PROBE_ID = 2_000_000_000;

const toNum = (v: unknown): number => Number(String(v));
const str = (v: unknown): string => String(v);
const ymd = (daysFromToday: number): string =>
  new Date(Date.now() + daysFromToday * 86_400_000).toISOString().slice(0, 10);

/**
 * 为本用例创建专属的宿主产品与色号，返回 product_id 与 color_id。
 * 入参 page 与本用例唯一码 tag。传给谁 每条价格建价的两个外键前置。
 * 存什么 分类 code 与名称带 tag，产品 code 与名称带 tag，色号 color_no 带 tag。
 * 存哪里 products 一行、product_colors 一行；宿主数据保留于库中，本用例只清价格不留悬空引用。
 */
async function seedHostProductColor(
  page: Page,
  tag: string
): Promise<{ productId: number; colorId: number }> {
  const category = await apiCallRaw<Row>(page, 'POST', '/categories', {
    name: `115类${tag}`,
    code: `115CAT${tag}`,
  });
  const categoryId = Number(category.id);
  expect(
    Number.isInteger(categoryId) && categoryId > 0,
    `分类创建未返回真实主键，实际 ${JSON.stringify(category)}`
  ).toBe(true);

  const product = await apiCallRaw<Row>(page, 'POST', '/products', {
    name: `115产品${tag}`,
    code: `115P${tag}`,
    category_id: categoryId,
    unit: 'm',
  });
  const productId = Number(product.id);
  expect(
    Number.isInteger(productId) && productId > 0,
    `产品创建未返回真实主键，实际 ${JSON.stringify(product)}`
  ).toBe(true);

  const color = await apiCallRaw<Row>(page, 'POST', `/products/${productId}/colors`, {
    color_no: `115CN${tag}`,
    color_name: `115色号${tag}`,
    color_type: 'STANDARD',
    extra_cost: 0,
  });
  const colorId = Number(color.id);
  expect(
    Number.isInteger(colorId) && colorId > 0,
    `色号创建未返回真实主键，实际 ${JSON.stringify(color)}`
  ).toBe(true);

  return { productId, colorId };
}

/**
 * 建一条色号价格并证明服务端把初态与归属列真实落库。
 * 入参 page 与宿主两个主键与 tag 与 base_price 与额外字段。
 * 传给谁 建价口 POST /color-prices，出参直接返回 ColorPriceDetail。
 * 存什么 product_id 与 color_id 与 currency 与 base_price 与 effective_from 由提交体决定，
 * 其余可空列走缺省；服务端写初态 approval_status APPROVED 与 is_active 真 与 priority 零 与 created_by 当前用户。
 * 存哪里 product_color_prices 一行。
 */
async function seedPrice(
  page: Page,
  host: { productId: number; colorId: number },
  tag: string,
  basePrice: string,
  extra: Record<string, unknown> = {}
): Promise<ColorPriceDetailRow> {
  const created = await apiCallRaw<ColorPriceDetailRow>(page, 'POST', '/color-prices', {
    product_id: host.productId,
    color_id: host.colorId,
    currency: 'CNY',
    base_price: basePrice,
    effective_from: ymd(0),
    notes: `115初建${tag}`,
    ...extra,
  });
  const id = Number(created.id);
  expect(
    Number.isInteger(id) && id > 0,
    `建价必须返回真实主键，实际 ${JSON.stringify(created)}`
  ).toBe(true);
  return created;
}

/**
 * 通过对象口 GET /color-prices 冒号 id 逐列回读一条价格。
 * 入参 page 与价格主键。传给谁 每次写操作的落地回读，作为断言的唯一真相来源。
 * 存什么 不落盘，直接返回后端出参。
 * 存哪里 不存。id 缺失或 approval_status 缺失即判红，不做 ?? 兜底。
 */
async function readPrice(page: Page, id: number): Promise<ColorPriceDetailRow> {
  const row = await apiCallRaw<ColorPriceDetailRow>(page, 'GET', `/color-prices/${id}`);
  expect(
    Number(row.id) === id,
    `对象口回读主键必须等于查询值，实际 ${JSON.stringify(row)}`
  ).toBe(true);
  expect(
    typeof row.approval_status === 'string',
    `approval_status 必须存在且为字符串，实际 ${JSON.stringify(row)}`
  ).toBe(true);
  return row;
}

test.describe.serial('115 色卡价格族链', () => {
  test.beforeEach(async ({ page }) => {
    // worker 会话由 diagnose-fixture 的 worker 作用域 storageState 注入，本链全程走 apiCall
    // 不导航页面，故不重复登录；只证明会话真实可用（写口依赖它的 CSRF 与凭据前置）
    const me = await apiCallRaw<Row>(page, 'GET', '/auth/me');
    expect(
      Number.isInteger(Number(me.id)) && Number(me.id) > 0,
      `/auth/me 必须回真实用户 id，实际 ${JSON.stringify(me)}`
    ).toBe(true);
  });

  test('T1 建价初态与列归属 五枚校验门逐个外显 改价半提交被拒 软删与重复删被拒 不存在主键三口 404', async ({
    page,
  }) => {
    const tag = genCode('CP115A');
    const host = await seedHostProductColor(page, tag);
    const priceIds: number[] = [];

    // 1) 建价主链（A 档 POST /color-prices）
    const created = await seedPrice(page, host, tag, '100.00');
    priceIds.push(Number(created.id));
    expect(str(created.currency), '建价必须回显提交币种').toBe('CNY');
    expect(toNum(created.base_price), '建价 base_price 必须回读提交值').toBe(100);
    expect(str(created.effective_from), '建价 effective_from 必须回读提交值').toBe(ymd(0));
    expect(str(created.approval_status), '建价初态必须落 APPROVED').toBe('APPROVED');
    expect(created.is_active === true, '建价 is_active 必须为真').toBe(true);
    expect(toNum(created.priority), '建价 priority 缺省必须落 0').toBe(0);
    expect(
      created.created_by !== null && created.created_by !== undefined,
      `建价 created_by 必须由服务端注入当前用户，实际 ${String(created.created_by)}`
    ).toBe(true);
    expect(
      created.approved_by === null,
      `建价 approved_by 初态必须为 null，实际 ${String(created.approved_by)}`
    ).toBe(true);
    expect(
      created.approved_at === null,
      `建价 approved_at 初态必须为 null，实际 ${String(created.approved_at)}`
    ).toBe(true);
    const createdReadback = await readPrice(page, Number(created.id));
    expect(str(createdReadback.notes), '建价 notes 必须回读提交原文').toBe(`115初建${tag}`);

    // 2) 非法币种（validate_currency 走 ISO 4217 白名单，validation_displayable 外显）
    const badCurrency = await apiCallExpectFail(page, 'POST', '/color-prices', {
      product_id: host.productId,
      color_id: host.colorId,
      currency: 'XYZ1',
      base_price: '50.00',
      effective_from: ymd(0),
    });
    expect(badCurrency.status, `非法币种必须 400，实际 ${badCurrency.status}`).toBe(400);
    expect(failureCode(badCurrency)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      str(badCurrency.message).includes('XYZ1'),
      `非法币种拒绝必须外显所提交值，实际 ${str(badCurrency.message)}`
    ).toBe(true);

    // 3) 非法季节（SS 与 AW 与 HOLIDAY 之外一律拒）
    const badSeason = await apiCallExpectFail(page, 'POST', '/color-prices', {
      product_id: host.productId,
      color_id: host.colorId,
      currency: 'CNY',
      base_price: '50.00',
      effective_from: ymd(0),
      season: 'SUMMER',
    });
    expect(badSeason.status, `非法季节必须 400，实际 ${badSeason.status}`).toBe(400);
    expect(failureCode(badSeason)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      str(badSeason.message).includes('SUMMER'),
      `非法季节拒绝必须外显所提交值，实际 ${str(badSeason.message)}`
    ).toBe(true);

    // 4) 非法客户等级（constants/customer_tier::ALLOWED 之外一律拒）
    const badLevel = await apiCallExpectFail(page, 'POST', '/color-prices', {
      product_id: host.productId,
      color_id: host.colorId,
      currency: 'CNY',
      base_price: '50.00',
      effective_from: ymd(0),
      customer_level: 'BRONZE',
    });
    expect(badLevel.status, `非法客户等级必须 400，实际 ${badLevel.status}`).toBe(400);
    expect(failureCode(badLevel)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      str(badLevel.message).includes('BRONZE'),
      `非法客户等级必须外显所提交值，实际 ${str(badLevel.message)}`
    ).toBe(true);

    // 5) min_quantity 不小于 max_quantity（validate_quantity_range）
    const badRange = await apiCallExpectFail(page, 'POST', '/color-prices', {
      product_id: host.productId,
      color_id: host.colorId,
      currency: 'CNY',
      base_price: '50.00',
      effective_from: ymd(0),
      min_quantity: '10.00',
      max_quantity: '5.00',
    });
    expect(badRange.status, `min 不小于 max 必须 400，实际 ${badRange.status}`).toBe(400);
    expect(failureCode(badRange)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      str(badRange.message).includes('min_quantity'),
      `数量区间拒绝必须外显被校验列，实际 ${str(badRange.message)}`
    ).toBe(true);

    // 6) effective_from 晚于 effective_to（validate_date_range）
    const badDate = await apiCallExpectFail(page, 'POST', '/color-prices', {
      product_id: host.productId,
      color_id: host.colorId,
      currency: 'CNY',
      base_price: '50.00',
      effective_from: ymd(10),
      effective_to: ymd(0),
    });
    expect(badDate.status, `起晚于止必须 400，实际 ${badDate.status}`).toBe(400);
    expect(failureCode(badDate)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      str(badDate.message).includes('effective_from'),
      `日期区间拒绝必须外显被校验列，实际 ${str(badDate.message)}`
    ).toBe(true);

    // 上述五条拒绝建价后，宿主下只能看到本次成功的那一条
    const listAfterRejects = await apiCallRaw<Paged<ColorPriceDetailRow>>(
      page,
      'GET',
      `/color-prices?product_id=${host.productId}&page=1&page_size=50`
    );
    expect(
      listAfterRejects.items.length,
      `被拒的建价不许落库，宿主产品下应仅一条价格，实际 ${JSON.stringify(listAfterRejects)}`
    ).toBe(1);
    expect(toNum(listAfterRejects.total), '列表口 total 必须与 items 长度同源').toBe(1);

    // 7) 改价主链（A 档 PUT /color-prices 冒号 id）
    await apiCall(page, 'PUT', `/color-prices/${Number(created.id)}`, {
      base_price: '150.00',
      notes: `115改价${tag}`,
      priority: 5,
    });
    const afterPut = await readPrice(page, Number(created.id));
    expect(toNum(afterPut.base_price), '改价 base_price 必须回读新值').toBe(150);
    expect(str(afterPut.notes), '改价 notes 必须回读原文').toBe(`115改价${tag}`);
    expect(toNum(afterPut.priority), '改价 priority 必须回读新值').toBe(5);
    expect(str(afterPut.approval_status), '改价不许动审批态（保持初态 APPROVED）').toBe('APPROVED');

    // 8) 改价非法季节同请求携带合法 base_price 必须整笔拒 且同请求里的合法列零提交（半提交即缺陷）
    const putMixed = await apiCallExpectFail(page, 'PUT', `/color-prices/${Number(created.id)}`, {
      base_price: '999.00',
      season: 'SPRING',
    });
    expect(putMixed.status, `改价非法季节必须 400，实际 ${putMixed.status}`).toBe(400);
    expect(failureCode(putMixed)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      str(putMixed.message).includes('SPRING'),
      `改价非法季节必须外显所提交值，实际 ${str(putMixed.message)}`
    ).toBe(true);
    const afterPutReject = await readPrice(page, Number(created.id));
    expect(
      toNum(afterPutReject.base_price),
      `被拒请求内的合法列不许半提交，base_price 必须仍是 150，实际 ${str(afterPutReject.base_price)}`
    ).toBe(150);
    expect(
      afterPutReject.season === null,
      `被拒请求内的合法列不许半提交，season 必须仍是 null，实际 ${String(afterPutReject.season)}`
    ).toBe(true);

    // 9) 改价不存在主键必须 404
    const putMissing = await apiCallExpectFail(page, 'PUT', `/color-prices/${ABSENT_PROBE_ID}`, {
      base_price: '10.00',
    });
    expect(putMissing.status, `改不存在价格必须 404，实际 ${putMissing.status}`).toBe(404);
    expect(failureCode(putMissing)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 10) 软删（A 档 DELETE /color-prices 冒号 id） 只改 is_active 不改 approval_status
    await apiCall(page, 'DELETE', `/color-prices/${Number(created.id)}`);
    const afterDelete = await readPrice(page, Number(created.id));
    expect(
      afterDelete.is_active === false,
      `软删后 is_active 必须为假，实际 ${String(afterDelete.is_active)}`
    ).toBe(true);
    expect(
      str(afterDelete.approval_status),
      '软删不许动审批态（保持 APPROVED）'
    ).toBe('APPROVED');

    // 11) 重复删除必须 400 BUSINESS_ERROR 且回读零漂移（CrudError::InvalidState 走脱敏族不钉文案）
    const reDelete = await apiCallExpectFail(
      page,
      'DELETE',
      `/color-prices/${Number(created.id)}`
    );
    expect(reDelete.status, `重复软删必须 400，实际 ${reDelete.status}`).toBe(400);
    expect(failureCode(reDelete)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const afterReDelete = await readPrice(page, Number(created.id));
    expect(
      afterReDelete.is_active === false,
      `被拒的重复删除不许把 is_active 翻回真，实际 ${String(afterReDelete.is_active)}`
    ).toBe(true);

    // 12) 不存在主键的三个读口与两个写口一律 404
    const getMissing = await apiCallExpectFail(page, 'GET', `/color-prices/${ABSENT_PROBE_ID}`);
    expect(getMissing.status, `不存在价格详情必须 404，实际 ${getMissing.status}`).toBe(404);
    expect(failureCode(getMissing)).toBe(APP_ERROR_CODES.NOT_FOUND);
    const delMissing = await apiCallExpectFail(page, 'DELETE', `/color-prices/${ABSENT_PROBE_ID}`);
    expect(delMissing.status, `不存在价格软删必须 404，实际 ${delMissing.status}`).toBe(404);
    expect(failureCode(delMissing)).toBe(APP_ERROR_CODES.NOT_FOUND);
    const approveMissing = await apiCallExpectFail(
      page,
      'POST',
      `/color-prices/${ABSENT_PROBE_ID}/approve`,
      { decision: 'APPROVED' }
    );
    expect(
      approveMissing.status,
      `不存在价格审批必须 404，实际 ${approveMissing.status}`
    ).toBe(404);
    expect(failureCode(approveMissing)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 清理段：本用例内 priceIds 声明可见，逐条软删已删的走 400 也接受
    for (const id of priceIds) {
      await apiCall(page, 'DELETE', `/color-prices/${id}`).catch(() => {
        // 主链已软删本条，清理段重删被 InvalidState 拒属预期，不掩盖真实错误以外的分支
      });
    }
  });

  test('T2 批量调价阈值分派 未越阈落新价 越阈保原值 非法 adjust_type 整笔不落 不存在 id 与空 items 与零漂', async ({
    page,
  }) => {
    const tag = genCode('CP115B');
    const host = await seedHostProductColor(page, tag);
    const priceIds: number[] = [];

    // 前置 四条价格 基准价 100 200 300 400 用于阈值判定与 out 参数顺序校验
    const p1 = await seedPrice(page, host, `${tag}P1`, '100.00');
    const p2 = await seedPrice(page, host, `${tag}P2`, '200.00');
    const p3 = await seedPrice(page, host, `${tag}P3`, '300.00');
    const p4 = await seedPrice(page, host, `${tag}P4`, '400.00');
    for (const p of [p1, p2, p3, p4]) {
      priceIds.push(Number(p.id));
    }

    // 1) 批量调价（A 档 POST /color-prices/batch-adjust） 四条按 items 顺序混合 auto 与 pending
    //    P1 +5 percentage 未越阈 走 auto 分支 落新价 105 APPROVED 审批人与时间戳非空
    //    P2 +20 percentage 越阈 走 pending 分支 base_price 保持 200 只改 approval_status 到 PENDING
    //    P3 fixed -50 越阈 走 pending 分支 base_price 保持 300 approval_status 到 PENDING
    //    P4 fixed +10 未越阈 走 auto 分支 落新价 410 APPROVED
    const batch = await apiCallRaw<BatchAdjustResp>(page, 'POST', '/color-prices/batch-adjust', {
      items: [
        { price_id: Number(p1.id), adjustment_type: 'percentage', adjustment_value: '0.05' },
        { price_id: Number(p2.id), adjustment_type: 'percentage', adjustment_value: '0.20' },
        { price_id: Number(p3.id), adjustment_type: 'fixed', adjustment_value: '-50.00' },
        { price_id: Number(p4.id), adjustment_type: 'fixed', adjustment_value: '10.00' },
      ],
      change_reason: `115批量调价${tag}`,
    });
    expect(Array.isArray(batch.auto_approved), 'auto_approved 必须回数组').toBe(true);
    expect(Array.isArray(batch.pending_approval), 'pending_approval 必须回数组').toBe(true);
    expect(
      batch.auto_approved.map(Number).join(','),
      `auto_approved 顺序必须与 items 一致，实际 ${JSON.stringify(batch.auto_approved)}`
    ).toBe(`${Number(p1.id)},${Number(p4.id)}`);
    expect(
      batch.pending_approval.map(Number).join(','),
      `pending_approval 顺序必须与 items 一致，实际 ${JSON.stringify(batch.pending_approval)}`
    ).toBe(`${Number(p2.id)},${Number(p3.id)}`);
    expect(toNum(batch.total), 'total 必须等于 items 长度').toBe(4);

    // 2) 逐条对象口回读 阈值两分支的列漂移独立验证
    const p1After = await readPrice(page, Number(p1.id));
    expect(
      toNum(p1After.base_price),
      `P1 auto 分支 base_price 必须由 100 落到 105，实际 ${str(p1After.base_price)}`
    ).toBe(105);
    expect(str(p1After.approval_status), 'P1 auto 分支必须落 APPROVED').toBe('APPROVED');
    expect(
      p1After.approved_by !== null && p1After.approved_by !== undefined,
      `P1 auto 分支必须写审批人，实际 ${String(p1After.approved_by)}`
    ).toBe(true);
    expect(
      p1After.approved_at !== null && p1After.approved_at !== undefined,
      `P1 auto 分支必须写审批时间戳，实际 ${String(p1After.approved_at)}`
    ).toBe(true);

    const p2After = await readPrice(page, Number(p2.id));
    expect(
      toNum(p2After.base_price),
      `P2 pending 分支 base_price 必须保持原值 200，实际 ${str(p2After.base_price)}`
    ).toBe(200);
    expect(str(p2After.approval_status), 'P2 pending 分支必须落 PENDING').toBe('PENDING');
    expect(
      p2After.approved_by === null,
      `P2 pending 分支不许写审批人，实际 ${String(p2After.approved_by)}`
    ).toBe(true);
    expect(
      p2After.approved_at === null,
      `P2 pending 分支不许写审批时间戳，实际 ${String(p2After.approved_at)}`
    ).toBe(true);

    const p3After = await readPrice(page, Number(p3.id));
    expect(
      toNum(p3After.base_price),
      `P3 pending 分支 base_price 必须保持原值 300，实际 ${str(p3After.base_price)}`
    ).toBe(300);
    expect(str(p3After.approval_status), 'P3 pending 分支必须落 PENDING').toBe('PENDING');

    const p4After = await readPrice(page, Number(p4.id));
    expect(
      toNum(p4After.base_price),
      `P4 auto 分支 base_price 必须由 400 落到 410，实际 ${str(p4After.base_price)}`
    ).toBe(410);
    expect(str(p4After.approval_status), 'P4 auto 分支必须落 APPROVED').toBe('APPROVED');

    // 3) 历史口证明 pending 分支也落了 batch 历史 new_price 是 240 与 250
    const p2History = await apiCallRaw<HistoryResp>(page, 'GET', `/color-prices/${Number(p2.id)}/history`);
    expect(
      p2History.items.length > 0,
      `P2 调价后必须有历史行，实际 ${JSON.stringify(p2History)}`
    ).toBe(true);
    const p2Last = p2History.items[0];
    expect(toNum(p2Last.old_price), 'P2 历史 old_price 必须是原 200').toBe(200);
    expect(
      toNum(p2Last.new_price),
      `P2 历史 new_price 必须是算出的 240，实际 ${str(p2Last.new_price)}`
    ).toBe(240);
    expect(str(p2Last.change_type), 'P2 历史 change_type 必须为 batch').toBe('batch');
    expect(
      p2Last.approved_by === null,
      `P2 pending 历史行的审批位必须为 null，实际 ${String(p2Last.approved_by)}`
    ).toBe(true);

    // 4) 非法 adjust_type 走同批一合法加一非法证明整笔不落
    //    新建 P5 用于承载合法项，非法项引用同一条 P5（先建 让 fetch_prices_map 通过）
    //    这样 calculate_new_price 抛错 早退在 insert_history_records 与 apply_price_updates 之前
    const p5 = await seedPrice(page, host, `${tag}P5`, '500.00');
    priceIds.push(Number(p5.id));
    const p6 = await seedPrice(page, host, `${tag}P6`, '600.00');
    priceIds.push(Number(p6.id));
    const badType = await apiCallExpectFail(page, 'POST', '/color-prices/batch-adjust', {
      items: [
        { price_id: Number(p5.id), adjustment_type: 'fixed', adjustment_value: '10.00' },
        { price_id: Number(p6.id), adjustment_type: 'percent', adjustment_value: '0.10' },
      ],
      change_reason: `115非法adjust_type${tag}`,
    });
    expect(
      badType.status,
      `非法 adjust_type 必须 400，实际 ${badType.status}`
    ).toBe(400);
    expect(failureCode(badType)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const p5AfterBad = await readPrice(page, Number(p5.id));
    expect(
      toNum(p5AfterBad.base_price),
      `同批被拒 P5 base_price 不许半提交，必须仍是 500，实际 ${str(p5AfterBad.base_price)}`
    ).toBe(500);
    expect(str(p5AfterBad.approval_status), '同批被拒 P5 approval_status 必须仍是 APPROVED').toBe(
      'APPROVED'
    );
    const p6AfterBad = await readPrice(page, Number(p6.id));
    expect(toNum(p6AfterBad.base_price), '同批被拒 P6 base_price 必须仍是 600').toBe(600);

    // 5) 不存在的 price_id 走 fetch_prices_map 拒绝 404 且同批合法项也不落
    const p7 = await seedPrice(page, host, `${tag}P7`, '700.00');
    priceIds.push(Number(p7.id));
    const batchMissing = await apiCallExpectFail(page, 'POST', '/color-prices/batch-adjust', {
      items: [
        { price_id: Number(p7.id), adjustment_type: 'fixed', adjustment_value: '5.00' },
        { price_id: ABSENT_PROBE_ID, adjustment_type: 'fixed', adjustment_value: '5.00' },
      ],
    });
    expect(
      batchMissing.status,
      `批量含不存在 id 必须 404，实际 ${batchMissing.status}`
    ).toBe(404);
    expect(failureCode(batchMissing)).toBe(APP_ERROR_CODES.NOT_FOUND);
    const p7AfterMissing = await readPrice(page, Number(p7.id));
    expect(
      toNum(p7AfterMissing.base_price),
      `同批被拒 P7 base_price 不许半提交，必须仍是 700，实际 ${str(p7AfterMissing.base_price)}`
    ).toBe(700);

    // 6) items 空数组 后端未做非空阻断 handler 未显式调用 dto.validate 契约即返回三键零
    //    这里不假装拒绝 只钉当前真实形状 若日后加了非空门 本用例改为期望 400 即会红 提醒同步
    const emptyItems = await apiCallRaw<BatchAdjustResp>(page, 'POST', '/color-prices/batch-adjust', {
      items: [],
    });
    expect(toNum(emptyItems.total), '空 items 出参 total 必须为 0').toBe(0);
    expect(emptyItems.auto_approved.length, '空 items 出参 auto_approved 必须为空数组').toBe(0);
    expect(
      emptyItems.pending_approval.length,
      '空 items 出参 pending_approval 必须为空数组'
    ).toBe(0);

    // 清理段
    for (const id of priceIds) {
      await apiCall(page, 'DELETE', `/color-prices/${id}`).catch(() => {
        // 单条重复软删被 InvalidState 拒属预期 不掩盖真实错误以外的分支
      });
    }
  });

  test('T3 审批 APPROVED 从历史落新价 审批 REJECTED 保原值 状态门重复审批外显文案 非法决定被拒 审批后列零漂', async ({
    page,
  }) => {
    const tag = genCode('CP115C');
    const host = await seedHostProductColor(page, tag);
    const priceIds: number[] = [];

    // 前置 建两条价 各 100 与 200 走 batch_adjust 越阈使其进 PENDING
    const p1 = await seedPrice(page, host, `${tag}A1`, '100.00');
    const p2 = await seedPrice(page, host, `${tag}A2`, '200.00');
    const p3 = await seedPrice(page, host, `${tag}A3`, '300.00');
    for (const p of [p1, p2, p3]) {
      priceIds.push(Number(p.id));
    }
    const toPending = await apiCallRaw<BatchAdjustResp>(page, 'POST', '/color-prices/batch-adjust', {
      items: [
        { price_id: Number(p1.id), adjustment_type: 'percentage', adjustment_value: '0.20' },
        { price_id: Number(p2.id), adjustment_type: 'percentage', adjustment_value: '-0.20' },
        { price_id: Number(p3.id), adjustment_type: 'percentage', adjustment_value: '0.30' },
      ],
      change_reason: `115审批前置${tag}`,
    });
    expect(toPending.pending_approval.length, '三条价格必须全部越阈进 pending').toBe(3);
    expect(
      str((await readPrice(page, Number(p1.id))).approval_status),
      'P1 前置审批后必须 PENDING'
    ).toBe('PENDING');

    // 1) 审批 APPROVED（A 档 POST /color-prices 冒号 id 冒号 approve）从最近一条历史取 new_price 落到 base_price
    const approved = await apiCallRaw<ColorPriceDetailRow>(
      page,
      'POST',
      `/color-prices/${Number(p1.id)}/approve`,
      { decision: 'APPROVED', comments: `115通过${tag}` }
    );
    expect(
      Number(approved.id),
      `审批出参主键必须等于路径 id，实际 ${JSON.stringify(approved)}`
    ).toBe(Number(p1.id));
    expect(str(approved.approval_status), '审批 APPROVED 后 approval_status 必须翻回 APPROVED').toBe(
      'APPROVED'
    );
    expect(
      toNum(approved.base_price),
      `审批 APPROVED 后 base_price 必须从历史取 120 落到主表，实际 ${str(approved.base_price)}`
    ).toBe(120);
    expect(
      approved.approved_by !== null && approved.approved_by !== undefined,
      `审批必须写审批人，实际 ${String(approved.approved_by)}`
    ).toBe(true);
    expect(
      approved.approved_at !== null && approved.approved_at !== undefined,
      `审批必须写审批时间戳，实际 ${String(approved.approved_at)}`
    ).toBe(true);
    const p1Final = await readPrice(page, Number(p1.id));
    expect(toNum(p1Final.base_price), '对象口回读 base_price 与出参必须同值').toBe(120);

    // 审批后历史行的 approved_by 必须被回填
    const p1History = await apiCallRaw<HistoryResp>(
      page,
      'GET',
      `/color-prices/${Number(p1.id)}/history`
    );
    expect(p1History.items.length, '审批后必须有历史行').toBeGreaterThan(0);
    const p1LastHistory = p1History.items[0];
    expect(
      p1LastHistory.approved_by !== null && p1LastHistory.approved_by !== undefined,
      `审批 APPROVED 必须回填历史行的审批位，实际 ${String(p1LastHistory.approved_by)}`
    ).toBe(true);

    // 2) 审批 REJECTED 只改 approval_status 不改 base_price 保留原值 100
    //    P2 原 200 越阈 -20 percent 期望新价 160 但走 REJECTED 主表 base_price 保持 200
    const rejected = await apiCallRaw<ColorPriceDetailRow>(
      page,
      'POST',
      `/color-prices/${Number(p2.id)}/approve`,
      { decision: 'REJECTED', comments: `115拒绝${tag}` }
    );
    expect(str(rejected.approval_status), '审批 REJECTED 后必须落 REJECTED').toBe('REJECTED');
    expect(
      toNum(rejected.base_price),
      `审批 REJECTED 后主表 base_price 必须保持原值 200，实际 ${str(rejected.base_price)}`
    ).toBe(200);
    expect(
      rejected.approved_by !== null && rejected.approved_by !== undefined,
      `审批 REJECTED 也必须写审批人，实际 ${String(rejected.approved_by)}`
    ).toBe(true);
    const p2Final = await readPrice(page, Number(p2.id));
    expect(str(p2Final.approval_status), '对象口回读 REJECTED').toBe('REJECTED');
    expect(toNum(p2Final.base_price), '对象口回读 base_price 保持 200').toBe(200);

    // 3) 状态门 已 APPROVED 的价格重复审批必须 400 BUSINESS_ERROR 且外显固定文案
    const reApprove = await apiCallExpectFail(
      page,
      'POST',
      `/color-prices/${Number(p1.id)}/approve`,
      { decision: 'APPROVED' }
    );
    expect(reApprove.status, `重复审批必须 400，实际 ${reApprove.status}`).toBe(400);
    expect(failureCode(reApprove)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      str(reApprove.message),
      `状态门拒绝必须外显固定文案 business_displayable，实际 ${str(reApprove.message)}`
    ).toBe('价格不处于待审批状态，无法审批');
    const p1AfterReApprove = await readPrice(page, Number(p1.id));
    expect(str(p1AfterReApprove.approval_status), '被拒的重复审批不许动状态').toBe('APPROVED');
    expect(toNum(p1AfterReApprove.base_price), '被拒的重复审批不许动价格').toBe(120);

    // 4) REJECTED 也非 PENDING 走同门 且列零漂移
    const rejectOnRejected = await apiCallExpectFail(
      page,
      'POST',
      `/color-prices/${Number(p2.id)}/approve`,
      { decision: 'APPROVED' }
    );
    expect(
      rejectOnRejected.status,
      `REJECTED 上再审批必须 400，实际 ${rejectOnRejected.status}`
    ).toBe(400);
    expect(failureCode(rejectOnRejected)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const p2AfterRejectAgain = await readPrice(page, Number(p2.id));
    expect(str(p2AfterRejectAgain.approval_status), '被拒的二次审批不许把 REJECTED 改回').toBe(
      'REJECTED'
    );

    // 5) 非法 decision 走 BatchError::Validation 映射脱敏 400 加 VALIDATION_ERROR
    //    P3 仍在 PENDING 里 提交非法决定后必须整笔不落 回读三列零漂移
    const badDecision = await apiCallExpectFail(
      page,
      'POST',
      `/color-prices/${Number(p3.id)}/approve`,
      { decision: 'MAYBE' }
    );
    expect(badDecision.status, `非法决定必须 400，实际 ${badDecision.status}`).toBe(400);
    expect(failureCode(badDecision)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const p3AfterBadDecision = await readPrice(page, Number(p3.id));
    expect(
      str(p3AfterBadDecision.approval_status),
      '非法决定被拒后 approval_status 必须仍是 PENDING'
    ).toBe('PENDING');
    expect(
      toNum(p3AfterBadDecision.base_price),
      '非法决定被拒后 base_price 必须仍是原值 300'
    ).toBe(300);
    expect(
      p3AfterBadDecision.approved_by === null,
      `非法决定被拒后不许写审批人，实际 ${String(p3AfterBadDecision.approved_by)}`
    ).toBe(true);

    // 6) P3 走正常 REJECTED 拒绝 保留 base_price 300 与 approval_status REJECTED
    const rejectP3 = await apiCallRaw<ColorPriceDetailRow>(
      page,
      'POST',
      `/color-prices/${Number(p3.id)}/approve`,
      { decision: 'REJECTED' }
    );
    expect(str(rejectP3.approval_status), 'P3 REJECTED 后必须落 REJECTED').toBe('REJECTED');
    expect(toNum(rejectP3.base_price), 'P3 REJECTED 后主表 base_price 必须保持原值 300').toBe(300);

    // 清理段
    for (const id of priceIds) {
      await apiCall(page, 'DELETE', `/color-prices/${id}`).catch(() => {
        // 单条重复软删被 InvalidState 拒属预期 不掩盖真实错误以外的分支
      });
    }
  });
});
