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
 * 116 色卡价格族季节规则 阶梯价 客户专属价 首个真契约覆盖
 *
 * 功能。把 POST 与 GET 与 PUT 与 DELETE 四条季节规则口（/api/v1/erp/color-prices/seasonal-rules
 * 与其 id 段）、阶梯价的 POST /color-prices/tiers 冒号 price_id 与 DELETE /color-prices/tiers/item
 * 冒号 tier_id 两口、客户专属价的 POST /color-prices/customer-special 一口，钉成真实落库回读链，
 * 并把服务层校验门、归属双源形状、唯一约束通道如实钉成当前真实形状。与 115 号分工：115 覆盖
 * 主价格表五个写口，本片覆盖主价格表之上的三张附属表写口，两片共用同一套母体造数范式。
 *
 * 调用方。Playwright 收集器按 flow 目录发现本 spec，worker 会话由 e2e/diagnose-fixture 的
 * worker 作用域 storageState 注入，全程 apiCall 不导航页面，不重复登录，只在 beforeEach
 * 用 /auth/me 探活证明会话可用。
 *
 * 入参。季节规则提交体按 CreateSeasonalRuleDto 与 UpdateSeasonalRuleDto 的字段与可空性对齐；
 * 阶梯价提交体按 CreatePriceTierDto 对齐且必须显式携带 product_color_price_id；客户专属价提交体
 * 按 CreateCustomerColorPriceDto 对齐。金额与比例用字符串承载 rust_decimal，出参用 Number 归一对表。
 *
 * 传给谁。请求经 apiCall 到 handlers/color_price_handler.rs 的 create_seasonal_rule 与
 * get_seasonal_rule 与 update_seasonal_rule 与 delete_seasonal_rule 与 list_seasonal_rules 与
 * create_tier 与 list_tiers 与 delete_tier 与 create_customer_special_price 与
 * list_customer_special_prices，服务层落到 services/color_price_seasonal_service.rs 与
 * services/color_price_tier_service.rs 与 handler 内联的专属价插入段。
 *
 * 存什么。季节规则落 seasonal_price_rules 一行，is_active 由服务端写真，created_at 与 updated_at
 * 由服务端写当前时刻，product_category_id 原样存提交的外键值；阶梯价落 color_price_tiers 一行，
 * sequence 缺省零、customer_level 可空、区间值原样存；客户专属价落 customer_color_prices 一行，
 * approved_by 与 approved_at 由 handler 写 null（专属价没有审批口），currency 原样存提交值。
 *
 * 存哪里。seasonal_price_rules 与 color_price_tiers 与 customer_color_prices 三张表；
 * 阶梯删除与季节删除经 services/audit_log_service.rs 的 delete_with_audit_i64 同事务写 audit_logs
 * 一条删除前快照。母体宿主产品与色号与主价格与分类与客户各自落自己的域主表，全部带本用例 tag。
 *
 * 契约真相。逐段读后端源与真实 DDL，非推测。
 * 季节规则（services/color_price_seasonal_service.rs 与 handlers/color_price_handler.rs 与
 * migration m0044 内联 DDL）：create 服务层只跑三枚门，季节只允许 SS 与 AW 与 HOLIDAY，
 * 调整方式只允许 percentage 与 fixed，valid_until 早于 valid_from 拒绝；三枚门全部走
 * SeasonalError::Validation 映射 AppError::validation_displayable，即 400 加 VALIDATION_ERROR
 * 且外显真实文案。注意 handler 的 create 未调用 dto.validate 抽取器层注解门（rule_name 长度注解
 * 是惰性配置），所以本层唯一的拒绝通道就是上述三枚服务层门。出参为 seasonal_price_rule::Model
 * 全列 json 直出。is_active 由服务端写 true，提交体里的同名列不参与（Create DTO 没有该字段）。
 * 列表口出参四键 items 与 total 与 page 与 page_size，可按 season 与 is_active 与
 * product_category_id 过滤，按 created_at 倒序。对象口按 id 命中，未命中走 SeasonalError::NotFound
 * 映射 AppError::not_found 脱敏族 404。PUT 与 POST 同三枚门，日期门是合并语义：只提交
 * valid_until 时用现存 valid_from 参与比较。DELETE 是物理删除且同事务写审计快照（handler 注入
 * 真实操作人 user_id），
 * 未命中主键返回 AppError::not_found 即 404；删后对象口必须 404，不接受"删了还能读到"。
 * DB 层 chk_seasonal_type 与 chk_seasonal_adjustment_type 与 chk_seasonal_valid_range 三枚 CHECK
 * 与服务层词表同源，服务层先拦，DB 层只是兜底第二道。
 * 阶梯价（services/color_price_tier_service.rs 与 routes/color_price.rs）：路由段
 * /tiers 冒号 price_id 的 POST handler 是 create_tier，其签名只抽 Json 不抽 Path，
 * 即路径段 price_id 不参与归属判定，母体归属完全由请求体 product_color_price_id 决定。
 * 这是当前真实形状，本用例用双母体错位提交把它钉死；后端若日后收敛为单点来源（读路径段或
 * 强制两段一致），本用例即红以提醒同步。服务层唯一门是 min_quantity 不小于 max_quantity 拒绝，
 * 走 TierError::Validation 映射 AppError::validation 脱敏族，即 400 加 VALIDATION_ERROR
 * 但不外显文案，故只钉码不钉文案。列表口 GET /tiers 冒号 price_id 先查母体存在性（缺失 404），
 * 再按 data scope 判父记录归属（admin 恒 All 直过），出参两键 items 与 total，items 按 sequence
 * 升序；total 等于 items 长度而非全表计数。删除口走 delete_with_audit_i64 物理删加审计快照，
 * 未命中映射 TierError::NotFound 即 404 NOT_FOUND。区间语义按 DDL 注释 min 含 max 不含，
 * max_quantity 为 null 表示无限上界；出参只回显列值，包含关系由读取方 price_calculator 侧解释，
 * 本用例钉的是列值原样落库与列表顺序。DB 层 uq_tier_price 唯一键（product_color_price_id 与
 * min_quantity 与 customer_level）与 chk_tier_customer_level CHECK 存在，但服务层没有前置查询，
 * 撞键会裸抛 DbErr 走 500 族。本用例不主动撞该键（撞了等于把缺陷固化成期望），只如实记录该
 * 通道在下方"e2e 验不到的面"段。
 * 客户专属价（handler 内联插入段）：create_customer_special_price 显式调用 dto.validate 抽取
 * 注解门，但该 DTO 没有任何字段级 validate 注解，所以字段级注解门是恒通的空门；真正的必填
 * 约束来自 serde（缺 currency 等必填字段即 axum Json 提取失败，经 middleware/trace_context.rs
 * 归一为 400 加 VALIDATION_ERROR 脱敏族）。币种没有走 currency_service 的 ISO 4217 门（与主
 * 价格表不同源，属当前真实形状，本用例钉"任意三字码原样落库"并在下方标为漂移点）。数据范围门
 * 仅对非 All 范围判父客户可见性，admin 恒 All 不可达。同键重复提交撞 DB 唯一键
 * uq_customer_color_price（customer_id 与 product_id 与 color_id 与 valid_from 四列复合），
 * 服务层没有前置查重，DbErr 经 map_err 走 AppError::database 即 500 加 DATABASE_ERROR。
 * 这是当前真实形状（缺陷通道，正确做法应映射业务码），本用例如实钉它并留红提醒后端收敛。
 * 出参为 customer_color_price::Model 全列 json 直出，approved_by 与 approved_at 由 handler
 * 显式写 null。列表口走 PaginatedResponse 四键信封。
 *
 * 假绿防线。每次写操作紧跟对象口或列表口逐列回读，未回读列不作为通过依据；门拒一律钉 HTTP
 * 与机器码，可外显族加钉文案原文，被拒后一律回读断零推进；阶梯删除后断剩余 id 集合与 sequence
 * 集合与区间值集合三重精确等值，不接受"条数减一"；同键重复提交钉 500 族加列表 total 仍一，
 * 双母体错位钉归属跟随请求体两个方向各查一次；全程不用 ?? 与 或 兜底，必出键缺失即形漂判红。
 *
 * e2e 验不到的面（如实交接，不假装覆盖）。一 阶梯撞 uq_tier_price 与撞 chk_tier_customer_level
 * 与母体外键缺失三个通道当前都是 DbErr 裸抛 500，把 500 写成期望等于把缺陷固化，故交后端集成
 * 测试覆盖（后端修复为业务码后本域 e2e 再补钉）。二 专属价与阶梯价的数据范围 403 分支需要
 * 非 All 范围账号，worker 会话是 admin 恒 All，该分支只能靠带角色夹具的集成测试。三 季节规则
 * 删除的审计快照内容在 audit_logs 表，本域没有读口，删除链只能证明"行没了"不能证明"快照对"。
 */

type Row = Record<string, unknown>;

/** 季节规则出参（seasonal_price_rule::Model 全列 json 直出） */
interface SeasonalRuleRow {
  id: number;
  rule_name: string;
  season: string;
  product_category_id: number | null;
  adjustment_type: string;
  adjustment_value: string | number;
  valid_from: string;
  valid_until: string | null;
  is_active: boolean;
  description: string | null;
  created_at: string;
  updated_at: string;
}

/** 阶梯价出参（color_price_tier::Model 全列 json 直出） */
interface TierRow {
  id: number;
  product_color_price_id: number;
  min_quantity: string | number;
  max_quantity: string | number | null;
  tier_price: string | number;
  customer_level: string | null;
  sequence: number;
  created_at: string;
  updated_at: string;
}

/** 客户专属价出参（customer_color_price::Model 全列 json 直出） */
interface CustomerSpecialRow {
  id: number;
  customer_id: number;
  product_id: number;
  color_id: number;
  special_price: string | number;
  discount_percent: string | number | null;
  currency: string;
  valid_from: string;
  valid_until: string | null;
  notes: string | null;
  approved_by: number | null;
  approved_at: string | null;
  created_at: string;
  updated_at: string;
}

/** 列表信封（季节列表与专属价列表四键；阶梯列表只有 items 与 total 两键，单列类型） */
type PagedEnvelope<T> = { items: T[]; total: number; page: number; page_size: number };
type TierListEnvelope = { items: TierRow[]; total: number };

/** 主价格出参的可见子集（母体，列全谱见 115 号） */
type PriceRow = { id: number; approval_status: string; is_active: boolean };

const ABSENT_PROBE_ID = 2_000_000_000;

const toNum = (v: unknown): number => Number(String(v));
const str = (v: unknown): string => String(v);
const ymd = (daysFromToday: number): string =>
  new Date(Date.now() + daysFromToday * 86_400_000).toISOString().slice(0, 10);

/**
 * 造本用例专属的宿主分类 产品 色号。
 * 入参 page 与唯一码 tag。传给谁 主价格母体与专属价的三枚外键前置。
 * 存什么 分类与产品与色号的名称与编码都带 tag。存哪里 product_categories 与 products 与
 * product_colors 各一行。
 */
async function seedHost(
  page: Page,
  tag: string
): Promise<{ categoryId: number; productId: number; colorId: number }> {
  const category = await apiCallRaw<Row>(page, 'POST', '/categories', {
    name: `116类${tag}`,
    code: `116CAT${tag}`,
  });
  const categoryId = Number(category.id);
  expect(
    Number.isInteger(categoryId) && categoryId > 0,
    `分类创建未返回真实主键，实际 ${JSON.stringify(category)}`
  ).toBe(true);

  const product = await apiCallRaw<Row>(page, 'POST', '/products', {
    name: `116产品${tag}`,
    code: `116P${tag}`,
    category_id: categoryId,
    unit: 'm',
  });
  const productId = Number(product.id);
  expect(
    Number.isInteger(productId) && productId > 0,
    `产品创建未返回真实主键，实际 ${JSON.stringify(product)}`
  ).toBe(true);

  const color = await apiCallRaw<Row>(page, 'POST', `/products/${productId}/colors`, {
    color_no: `116CN${tag}`,
    color_name: `116色号${tag}`,
    color_type: 'STANDARD',
    extra_cost: 0,
  });
  const colorId = Number(color.id);
  expect(
    Number.isInteger(colorId) && colorId > 0,
    `色号创建未返回真实主键，实际 ${JSON.stringify(color)}`
  ).toBe(true);

  return { categoryId, productId, colorId };
}

/**
 * 造一条已审批的主价格母体（阶梯价的父记录）。
 * 入参 page 与宿主两枚外键与 tag 与后缀。传给谁 阶梯价 POST 与 GET 的 product_color_price_id。
 * 存什么 币种 CNY 与 base_price 与 effective_from；其余列缺省。
 * 存哪里 product_color_prices 一行，初态 APPROVED 与 is_active 真由服务端写。
 */
async function seedPrice(
  page: Page,
  host: { productId: number; colorId: number },
  tag: string,
  suffix: string,
  basePrice = '100.00'
): Promise<PriceRow> {
  const created = await apiCallRaw<PriceRow>(page, 'POST', '/color-prices', {
    product_id: host.productId,
    color_id: host.colorId,
    currency: 'CNY',
    base_price: basePrice,
    effective_from: ymd(0),
    notes: `116母体${tag}${suffix}`,
  });
  const id = Number(created.id);
  expect(
    Number.isInteger(id) && id > 0,
    `主价格母体创建未返回真实主键，实际 ${JSON.stringify(created)}`
  ).toBe(true);
  expect(str(created.approval_status), '主价格母体初态必须 APPROVED').toBe('APPROVED');
  return created;
}

test.describe.serial('116 色卡价格附属表链', () => {
  test.beforeEach(async ({ page }) => {
    const me = await apiCallRaw<Row>(page, 'GET', '/auth/me');
    expect(
      Number.isInteger(Number(me.id)) && Number(me.id) > 0,
      `/auth/me 必须回真实用户 id，实际 ${JSON.stringify(me)}`
    ).toBe(true);
  });

  test('T1 季节规则四口全链 服务层三门逐枚外显 非法列同请求合法列零提交 删除后对象口 404 重复删 404', async ({
    page,
  }) => {
    const tag = genCode('SR116A');
    const host = await seedHost(page, tag);
    const categoryRef = { product_category_id: host.categoryId };
    const ruleIds: number[] = [];

    // 1) 建规则（A 档 POST /seasonal-rules）钉服务端代写列与提交列各自归位
    const created = await apiCallRaw<SeasonalRuleRow>(
      page,
      'POST',
      '/color-prices/seasonal-rules',
      {
        rule_name: `116夏涨${tag}`,
        season: 'SS',
        adjustment_type: 'percentage',
        adjustment_value: '0.08',
        valid_from: ymd(0),
        valid_until: ymd(90),
        description: `116季节描述${tag}`,
        ...categoryRef,
      }
    );
    const ruleId = Number(created.id);
    expect(
      Number.isInteger(ruleId) && ruleId > 0,
      `建季节规则必须回真实主键，实际 ${JSON.stringify(created)}`
    ).toBe(true);
    ruleIds.push(ruleId);
    expect(str(created.rule_name), '出参 rule_name 必须回读提交原文').toBe(`116夏涨${tag}`);
    expect(str(created.season), '出参 season 必须回读提交原文').toBe('SS');
    expect(str(created.adjustment_type), '出参 adjustment_type 必须回读提交原文').toBe(
      'percentage'
    );
    expect(
      toNum(created.adjustment_value),
      `出参 adjustment_value 必须回读提交值，实际 ${str(created.adjustment_value)}`
    ).toBe(0.08);
    expect(str(created.valid_from), '出参 valid_from 必须回读提交原文').toBe(ymd(0));
    expect(str(created.valid_until), '出参 valid_until 必须回读提交原文').toBe(ymd(90));
    expect(
      created.is_active === true,
      `is_active 必须由服务端写 true，实际 ${String(created.is_active)}`
    ).toBe(true);
    expect(toNum(created.product_category_id), 'product_category_id 必须回读提交的外键值').toBe(
      host.categoryId
    );
    expect(
      str(created.created_at).length > 0 && str(created.updated_at).length > 0,
      `created_at 与 updated_at 必须由服务端写非空，实际 ${created.created_at} 与 ${created.updated_at}`
    ).toBe(true);

    // 对象口逐列复验（出参自回显不等于落库，必须再查一次）
    const objRead = await apiCallRaw<SeasonalRuleRow>(
      page,
      'GET',
      `/color-prices/seasonal-rules/${ruleId}`
    );
    expect(Number(objRead.id), '对象口必须命中本规则').toBe(ruleId);
    expect(str(objRead.description), '对象口 description 必须回读提交原文').toBe(
      `116季节描述${tag}`
    );
    expect(str(objRead.season), '对象口 season 必须落 SS').toBe('SS');
    expect(toNum(objRead.adjustment_value), '对象口 adjustment_value 必须落 0.08').toBe(0.08);

    // 2) 三枚服务层门逐枚拒绝且外显真实文案（validation_displayable 族）
    const badSeason = await apiCallExpectFail(page, 'POST', '/color-prices/seasonal-rules', {
      rule_name: `116非法季节${tag}`,
      season: 'SUMMER',
      adjustment_type: 'percentage',
      adjustment_value: '0.05',
      valid_from: ymd(0),
    });
    expect(badSeason.status, `非法季节必须 400，实际 ${badSeason.status}`).toBe(400);
    expect(failureCode(badSeason)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      str(badSeason.message).includes('SUMMER'),
      `非法季节拒绝必须外显所提交值，实际 ${str(badSeason.message)}`
    ).toBe(true);

    const badType = await apiCallExpectFail(page, 'POST', '/color-prices/seasonal-rules', {
      rule_name: `116非法方式${tag}`,
      season: 'AW',
      adjustment_type: 'percent',
      adjustment_value: '0.05',
      valid_from: ymd(0),
    });
    expect(badType.status, `非法调整方式必须 400，实际 ${badType.status}`).toBe(400);
    expect(failureCode(badType)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      str(badType.message).includes('percent'),
      `非法调整方式拒绝必须外显所提交值，实际 ${str(badType.message)}`
    ).toBe(true);

    const badDate = await apiCallExpectFail(page, 'POST', '/color-prices/seasonal-rules', {
      rule_name: `116非法区间${tag}`,
      season: 'HOLIDAY',
      adjustment_type: 'fixed',
      adjustment_value: '1.5',
      valid_from: ymd(100),
      valid_until: ymd(10),
    });
    expect(badDate.status, `止早于起必须 400，实际 ${badDate.status}`).toBe(400);
    expect(failureCode(badDate)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      str(badDate.message).includes('valid_from'),
      `日期区间拒绝必须外显被校验列，实际 ${str(badDate.message)}`
    ).toBe(true);

    // 三次被拒的 rule_name 都不许出现在列表里（零推进回读，按名称逐枚点名）
    const listAfterRejects = await apiCallRaw<PagedEnvelope<SeasonalRuleRow>>(
      page,
      'GET',
      '/color-prices/seasonal-rules?page=1&page_size=100'
    );
    expect(Array.isArray(listAfterRejects.items), '列表口必须回 items 数组').toBe(true);
    for (const ghost of [`116非法季节${tag}`, `116非法方式${tag}`, `116非法区间${tag}`]) {
      expect(
        listAfterRejects.items.find(r => str(r.rule_name) === ghost),
        `被拒的规则 ${ghost} 不许落库`
      ).toBeUndefined();
    }

    // 3) 列表口过滤与四键信封（本用例第一条 SS 规则必须在 season=SS 过滤里）
    const ssList = await apiCallRaw<PagedEnvelope<SeasonalRuleRow>>(
      page,
      'GET',
      '/color-prices/seasonal-rules?season=SS&page=1&page_size=50'
    );
    expect(
      ssList.items.find(r => Number(r.id) === ruleId),
      `season=SS 过滤必须命中本规则，实际 ${JSON.stringify(ssList.items)}`
    ).toBeDefined();
    expect(toNum(ssList.page), '列表口 page 必须回显查询值').toBe(1);
    expect(toNum(ssList.page_size), '列表口 page_size 必须回显查询值').toBe(50);
    expect(
      toNum(ssList.total) >= ssList.items.length,
      `列表口 total 不得小于 items 长度，实际 ${ssList.total} 与 ${ssList.items.length}`
    ).toBe(true);
    for (const r of ssList.items) {
      expect(str(r.season), 'season=SS 过滤结果里不许混入其他季节').toBe('SS');
    }

    // 4) 改规则（A 档 PUT）改名 改值 停用 三列同提交
    await apiCall(page, 'PUT', `/color-prices/seasonal-rules/${ruleId}`, {
      rule_name: `116夏涨改${tag}`,
      adjustment_value: '0.15',
      is_active: false,
    });
    const afterPut = await apiCallRaw<SeasonalRuleRow>(
      page,
      'GET',
      `/color-prices/seasonal-rules/${ruleId}`
    );
    expect(str(afterPut.rule_name), '改后 rule_name 必须等于提交原文').toBe(`116夏涨改${tag}`);
    expect(toNum(afterPut.adjustment_value), '改后 adjustment_value 必须落 0.15').toBe(0.15);
    expect(afterPut.is_active === false, `改后 is_active 必须翻假（PUT 是唯一停用口）`).toBe(true);
    expect(str(afterPut.season), '只改三列不许动 season').toBe('SS');
    expect(str(afterPut.adjustment_type), '只改三列不许动 adjustment_type').toBe('percentage');

    // 5) 改规则的日期门是合并语义 只提交早于现存 valid_from 的 valid_until 也必须拒
    const badPutDate = await apiCallExpectFail(
      page,
      'PUT',
      `/color-prices/seasonal-rules/${ruleId}`,
      { valid_until: ymd(-30) }
    );
    expect(badPutDate.status, `合并日期门必须 400，实际 ${badPutDate.status}`).toBe(400);
    expect(failureCode(badPutDate)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const afterBadPutDate = await apiCallRaw<SeasonalRuleRow>(
      page,
      'GET',
      `/color-prices/seasonal-rules/${ruleId}`
    );
    expect(
      str(afterBadPutDate.valid_until),
      `被拒的合并日期提交不许落 valid_until，实际 ${str(afterBadPutDate.valid_until)}`
    ).toBe(ymd(90));

    // 6) 改规则非法季节同请求携带合法列 整笔拒 合法列零提交（半提交即缺陷）
    const badPutSeason = await apiCallExpectFail(
      page,
      'PUT',
      `/color-prices/seasonal-rules/${ruleId}`,
      { season: 'WINTER', adjustment_value: '0.99' }
    );
    expect(badPutSeason.status, `改季节非法值必须 400，实际 ${badPutSeason.status}`).toBe(400);
    expect(failureCode(badPutSeason)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const afterBadPutSeason = await apiCallRaw<SeasonalRuleRow>(
      page,
      'GET',
      `/color-prices/seasonal-rules/${ruleId}`
    );
    expect(str(afterBadPutSeason.season), '被拒的季节不许半提交').toBe('SS');
    expect(
      toNum(afterBadPutSeason.adjustment_value),
      `被拒请求内的合法列 adjustment_value 同样不许落库，实际 ${str(afterBadPutSeason.adjustment_value)}`
    ).toBe(0.15);

    // 7) 不存在主键 三个口一律 404
    const getMissing = await apiCallExpectFail(
      page,
      'GET',
      `/color-prices/seasonal-rules/${ABSENT_PROBE_ID}`
    );
    expect(getMissing.status, `不存在季节规则详情必须 404，实际 ${getMissing.status}`).toBe(404);
    expect(failureCode(getMissing)).toBe(APP_ERROR_CODES.NOT_FOUND);
    const putMissing = await apiCallExpectFail(
      page,
      'PUT',
      `/color-prices/seasonal-rules/${ABSENT_PROBE_ID}`,
      { adjustment_value: '0.01' }
    );
    expect(putMissing.status, `不存在季节规则改值必须 404，实际 ${putMissing.status}`).toBe(404);
    expect(failureCode(putMissing)).toBe(APP_ERROR_CODES.NOT_FOUND);
    const delMissing = await apiCallExpectFail(
      page,
      'DELETE',
      `/color-prices/seasonal-rules/${ABSENT_PROBE_ID}`
    );
    expect(delMissing.status, `不存在季节规则删除必须 404，实际 ${delMissing.status}`).toBe(404);
    expect(failureCode(delMissing)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 8) 物理删（A 档 DELETE）删后对象口必须 404 不接受软删位口径
    await apiCall(page, 'DELETE', `/color-prices/seasonal-rules/${ruleId}`);
    const gone = await apiCallExpectFail(page, 'GET', `/color-prices/seasonal-rules/${ruleId}`);
    expect(gone.status, `物理删后对象口必须 404，实际 ${gone.status}`).toBe(404);
    expect(failureCode(gone)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 9) 重复删同一规则必须再次 404（delete_with_audit 先查后删）
    const reDelete = await apiCallExpectFail(
      page,
      'DELETE',
      `/color-prices/seasonal-rules/${ruleId}`
    );
    expect(reDelete.status, `重复删季节规则必须 404，实际 ${reDelete.status}`).toBe(404);
    expect(failureCode(reDelete)).toBe(APP_ERROR_CODES.NOT_FOUND);
    const stillGone = await apiCallExpectFail(
      page,
      'GET',
      `/color-prices/seasonal-rules/${ruleId}`
    );
    expect(stillGone.status, '被拒的重复删不许让行复活').toBe(404);

    // 清理段（ruleId 已删；本作用域内 ruleIds 只含已删行，无残留）
    for (const id of ruleIds) {
      expect(id > 0, `清理段主键必须为正，实际 ${id}`).toBe(true);
    }
  });

  test('T2 阶梯价两行建与序列升序回读 路径段不参与的当前形状 min 不小于 max 被拒 删中间行后集合三重精确', async ({
    page,
  }) => {
    const tag = genCode('TR116B');
    const host = await seedHost(page, tag);
    // 两条独立母体 用于钉 POST 归属只跟随请求体不跟随路径段
    const priceA = await seedPrice(page, host, tag, 'A');
    const priceB = await seedPrice(page, host, tag, 'B', '200.00');
    const priceAId = Number(priceA.id);
    const priceBId = Number(priceB.id);
    const tierIds: number[] = [];

    // 1) 建三档阶梯 提交 sequence 乱序（3 1 2）钉列表口按 sequence 升序而非插入顺序
    const mkTier = (
      priceId: number,
      min: string,
      max: string | null,
      price: string,
      seq: number
    ) =>
      max === null
        ? {
            product_color_price_id: priceId,
            min_quantity: min,
            tier_price: price,
            sequence: seq,
          }
        : {
            product_color_price_id: priceId,
            min_quantity: min,
            max_quantity: max,
            tier_price: price,
            sequence: seq,
          };

    const t3 = await apiCallRaw<TierRow>(
      page,
      'POST',
      `/color-prices/tiers/${priceAId}`,
      mkTier(priceAId, '500.00', null, '55.00', 3)
    );
    expect(
      Number.isInteger(Number(t3.id)) && Number(t3.id) > 0,
      `阶梯建 3 档必须回真实主键，实际 ${JSON.stringify(t3)}`
    ).toBe(true);
    tierIds.push(Number(t3.id));
    expect(
      Number(t3.product_color_price_id),
      '阶梯出参的母体键必须等于请求体里的 product_color_price_id'
    ).toBe(priceAId);
    expect(
      t3.max_quantity === null,
      `max_quantity 缺省必须回 null（无限上界），实际 ${String(t3.max_quantity)}`
    ).toBe(true);
    expect(toNum(t3.tier_price), 'tier_price 必须回读提交值').toBe(55);

    const t1 = await apiCallRaw<TierRow>(
      page,
      'POST',
      `/color-prices/tiers/${priceAId}`,
      mkTier(priceAId, '1.00', '100.00', '60.00', 1)
    );
    const t2 = await apiCallRaw<TierRow>(
      page,
      'POST',
      `/color-prices/tiers/${priceAId}`,
      mkTier(priceAId, '100.00', '500.00', '58.00', 2)
    );
    tierIds.push(Number(t1.id), Number(t2.id));

    // 2) 列表口两键信封 items 按 sequence 升序
    const list = await apiCallRaw<TierListEnvelope>(page, 'GET', `/color-prices/tiers/${priceAId}`);
    expect(Array.isArray(list.items), '列表口必须回 items 数组').toBe(true);
    expect(list.items.length, '母体 A 下必须三条阶梯').toBe(3);
    expect(toNum(list.total), '列表口 total 必须等于 items 长度').toBe(3);
    expect(
      list.items.map(t => toNum(t.sequence)).join(','),
      `列表必须按 sequence 升序（3 1 2 提交乱序），实际 ${list.items.map(t => t.sequence).join(',')}`
    ).toBe('1,2,3');
    expect(
      list.items.map(t => Number(t.id)).join(','),
      '升序后的 id 序列必须精确等于 t1 t2 t3'
    ).toBe(`${Number(t1.id)},${Number(t2.id)},${Number(t3.id)}`);
    // 区间值原样落库（min 含 max 不含是读取方语义，本口钉列值归位）
    expect(toNum(list.items[0].min_quantity), '一档 min 必须 1').toBe(1);
    expect(toNum(list.items[0].max_quantity), '一档 max 必须 100').toBe(100);
    expect(toNum(list.items[1].min_quantity), '二档 min 必须 100').toBe(100);
    expect(list.items[2].max_quantity === null, '三档 max 列值必须为 null 不是缺键').toBe(true);

    // 3) POST 归属跟随请求体而非路径段（当前真实形状 create_tier 不抽 Path）
    //    路径段写母体 A 请求体写母体 B 行必须落在 B 的列表里且不在 A 的列表里
    const misplaced = await apiCallRaw<TierRow>(
      page,
      'POST',
      `/color-prices/tiers/${priceAId}`,
      mkTier(priceBId, '10.00', '20.00', '10.00', 1)
    );
    tierIds.push(Number(misplaced.id));
    expect(
      Number(misplaced.product_color_price_id),
      '归属必须等于请求体的母体键 不是路径段的'
    ).toBe(priceBId);
    const listB = await apiCallRaw<TierListEnvelope>(
      page,
      'GET',
      `/color-prices/tiers/${priceBId}`
    );
    expect(listB.items.length, '母体 B 下必须恰有这一条错挂阶梯').toBe(1);
    expect(Number(listB.items[0].id), 'B 列表里那一行必须是本次建的行').toBe(Number(misplaced.id));
    const listAAfter = await apiCallRaw<TierListEnvelope>(
      page,
      'GET',
      `/color-prices/tiers/${priceAId}`
    );
    expect(listAAfter.items.length, 'A 列表不许因错挂请求增长').toBe(3);

    // 4) min 不小于 max 被服务层拒（TierError::Validation 走脱敏族 只钉码不钉文案）
    const badRange = await apiCallExpectFail(
      page,
      'POST',
      `/color-prices/tiers/${priceAId}`,
      mkTier(priceAId, '900.00', '100.00', '99.00', 4)
    );
    expect(badRange.status, `min 不小于 max 必须 400，实际 ${badRange.status}`).toBe(400);
    expect(failureCode(badRange)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const listAfterBad = await apiCallRaw<TierListEnvelope>(
      page,
      'GET',
      `/color-prices/tiers/${priceAId}`
    );
    expect(listAfterBad.items.length, '被拒的建行不许让 A 列表增长').toBe(3);
    expect(
      listAfterBad.items.find(t => toNum(t.sequence) === 4),
      '被拒的 4 档不许落库'
    ).toBeUndefined();

    // 5) 母体不存在 列表口 404（先判父再判归属）
    const listMissing = await apiCallExpectFail(
      page,
      'GET',
      `/color-prices/tiers/${ABSENT_PROBE_ID}`
    );
    expect(listMissing.status, `不存在母体的阶梯列表必须 404，实际 ${listMissing.status}`).toBe(
      404
    );
    expect(failureCode(listMissing)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 6) 删中间行（A 档 DELETE /tiers/item 冒号 tier_id）删后剩余集合三重精确等值
    await apiCall(page, 'DELETE', `/color-prices/tiers/item/${Number(t2.id)}`);
    const listAfterDelete = await apiCallRaw<TierListEnvelope>(
      page,
      'GET',
      `/color-prices/tiers/${priceAId}`
    );
    expect(
      listAfterDelete.items
        .map(t => Number(t.id))
        .sort((a, b) => a - b)
        .join(','),
      '剩余 id 集合必须精确等于 t1 与 t3'
    ).toBe([Number(t1.id), Number(t3.id)].sort((a, b) => a - b).join(','));
    expect(
      listAfterDelete.items.map(t => toNum(t.sequence)).join(','),
      '剩余 sequence 序列必须精确等于 1 3'
    ).toBe('1,3');
    expect(
      listAfterDelete.items
        .map(t => `${toNum(t.min_quantity)}:${String(t.max_quantity)}:${toNum(t.tier_price)}`)
        .join(','),
      '剩余区间与价格集合必须精确等于一档与三档'
    ).toBe('1:100:60,500:null:55');
    expect(toNum(listAfterDelete.total), '剩余 total 必须等于 2').toBe(2);

    // 7) 重复删同一阶梯必须 404 且集合零漂移
    const reDeleteTier = await apiCallExpectFail(
      page,
      'DELETE',
      `/color-prices/tiers/item/${Number(t2.id)}`
    );
    expect(reDeleteTier.status, `重复删阶梯必须 404，实际 ${reDeleteTier.status}`).toBe(404);
    expect(failureCode(reDeleteTier)).toBe(APP_ERROR_CODES.NOT_FOUND);
    const listAfterReDelete = await apiCallRaw<TierListEnvelope>(
      page,
      'GET',
      `/color-prices/tiers/${priceAId}`
    );
    expect(listAfterReDelete.items.length, '被拒的重复删不许再少行').toBe(2);

    // 清理段（本作用域内 tierIds 声明可见 剩余两行逐枚物理删）
    for (const id of tierIds) {
      await apiCall(page, 'DELETE', `/color-prices/tiers/item/${id}`).catch(() => {
        // 主链已删 t2 与清理重复触达被 404 拒属预期 不掩盖真实错误以外的分支
      });
    }
    for (const priceId of [priceAId, priceBId]) {
      await apiCall(page, 'DELETE', `/color-prices/${priceId}`).catch(() => {
        // 母体软删二次触达被 InvalidState 拒属预期
      });
    }
  });

  test('T3 专属价关系键与审批位初态 必填缺失 400 脱敏 任意三字码原样落库 同键重复撞唯一键 500 族零推进', async ({
    page,
  }) => {
    const tag = genCode('CS116C');
    const host = await seedHost(page, tag);
    const customer = await apiCallRaw<Row>(page, 'POST', '/crm/customers', {
      customer_name: `116客户${tag}`,
      customer_type: 'retail',
    });
    const customerId = Number(customer.id);
    expect(
      Number.isInteger(customerId) && customerId > 0,
      `客户自建失败，实际 ${JSON.stringify(customer)}`
    ).toBe(true);

    const specialIds: number[] = [];
    const validFrom = ymd(0);

    // 1) 建专属价（A 档 POST /customer-special）钉关系键四列与审批位初态
    const created = await apiCallRaw<CustomerSpecialRow>(
      page,
      'POST',
      '/color-prices/customer-special',
      {
        customer_id: customerId,
        product_id: host.productId,
        color_id: host.colorId,
        special_price: '88.50',
        discount_percent: '0.95',
        currency: 'CNY',
        valid_from: validFrom,
        valid_until: ymd(180),
        notes: `116专属${tag}`,
      }
    );
    const id = Number(created.id);
    expect(
      Number.isInteger(id) && id > 0,
      `专属价建行必须回真实主键，实际 ${JSON.stringify(created)}`
    ).toBe(true);
    specialIds.push(id);
    expect(Number(created.customer_id), '关系键 customer_id 必须回读提交值').toBe(customerId);
    expect(Number(created.product_id), '关系键 product_id 必须回读提交值').toBe(host.productId);
    expect(Number(created.color_id), '关系键 color_id 必须回读提交值').toBe(host.colorId);
    expect(str(created.valid_from), '关系键 valid_from 必须回读提交原文').toBe(validFrom);
    expect(toNum(created.special_price), 'special_price 必须回读提交值').toBe(88.5);
    expect(toNum(created.discount_percent), 'discount_percent 必须回读提交值').toBe(0.95);
    expect(
      created.approved_by === null,
      `approved_by 初态必须由 handler 写 null，实际 ${String(created.approved_by)}`
    ).toBe(true);
    expect(
      created.approved_at === null,
      `approved_at 初态必须由 handler 写 null，实际 ${String(created.approved_at)}`
    ).toBe(true);

    // 列表口回读（专属价没有对象口 按四元组里的 customer 维度过滤后逐列对表）
    const list = await apiCallRaw<PagedEnvelope<CustomerSpecialRow>>(
      page,
      'GET',
      `/color-prices/customer-special?customer_id=${customerId}&page=1&page_size=20`
    );
    const mine = list.items.find(r => Number(r.id) === id);
    expect(mine, `列表口必须命中本行，实际 ${JSON.stringify(list.items)}`).toBeDefined();
    expect(toNum(mine!.special_price), '列表口 special_price 必须与建值同数').toBe(88.5);
    expect(str(mine!.currency), '列表口 currency 必须回读原文').toBe('CNY');
    expect(str(mine!.notes), '列表口 notes 必须回读原文').toBe(`116专属${tag}`);
    expect(toNum(list.page), '列表口 page 必须回显查询值').toBe(1);
    expect(toNum(list.page_size), '列表口 page_size 必须回显查询值').toBe(20);
    expect(toNum(list.total), '列表口 total 必须至少含本行').toBeGreaterThanOrEqual(1);

    // active_only 过滤 本行 valid_until 在未来 必须在活跃集里
    const activeList = await apiCallRaw<PagedEnvelope<CustomerSpecialRow>>(
      page,
      'GET',
      `/color-prices/customer-special?customer_id=${customerId}&active_only=true&page=1&page_size=20`
    );
    expect(
      activeList.items.find(r => Number(r.id) === id),
      '未过期的专属价必须出现在 active_only 集里'
    ).toBeDefined();

    // 2) 缺必填币种 走 serde 必填缺失加提取器归一 400 VALIDATION_ERROR 脱敏族（不钉文案）
    const noCurrency = await apiCallExpectFail(page, 'POST', '/color-prices/customer-special', {
      customer_id: customerId,
      product_id: host.productId,
      color_id: host.colorId,
      special_price: '77.00',
      valid_from: ymd(1),
    });
    expect(noCurrency.status, `缺必填 currency 必须 400，实际 ${noCurrency.status}`).toBe(400);
    expect(failureCode(noCurrency)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const listAfterNoCurrency = await apiCallRaw<PagedEnvelope<CustomerSpecialRow>>(
      page,
      'GET',
      `/color-prices/customer-special?customer_id=${customerId}&page=1&page_size=20`
    );
    expect(listAfterNoCurrency.items.length, '被拒的缺币建行不许落任何行').toBe(1);

    // 3) 任意三字码原样落库 当前形状 专属价没有走 currency_service 的 ISO 4217 门
    //    与主价格表 validate_currency 不同源 属漂移点 待后端收敛 若日后加门本断言即红提醒同步
    const oddCurrency = await apiCallRaw<CustomerSpecialRow>(
      page,
      'POST',
      '/color-prices/customer-special',
      {
        customer_id: customerId,
        product_id: host.productId,
        color_id: host.colorId,
        special_price: '66.00',
        currency: 'ZZZ',
        valid_from: ymd(2),
      }
    );
    expect(
      str(oddCurrency.currency),
      `非法 ISO 三字码当前必须原样落库（无币种门），实际 ${str(oddCurrency.currency)}`
    ).toBe('ZZZ');
    specialIds.push(Number(oddCurrency.id));

    // 4) 同键重复提交（customer_id 与 product_id 与 color_id 与 valid_from 四列复合键）
    //    当前真实形状 服务层无前置查重 撞 DB 唯一键 uq_customer_color_price 走 DbErr 到
    //    AppError::database 即 500 加 DATABASE_ERROR 正确做法应映射业务码 属缺陷通道
    //    把 500 写成期望不等于认可它 列表口 total 必须证明第二次没有落库（零推进）
    const dup = await apiCallExpectFail(page, 'POST', '/color-prices/customer-special', {
      customer_id: customerId,
      product_id: host.productId,
      color_id: host.colorId,
      special_price: '99.00',
      currency: 'CNY',
      valid_from: validFrom,
    });
    expect(
      dup.status,
      `同键重复提交当前必须撞唯一键走 500 族（若后端改映射业务码 本断言翻红即提醒同步收窄此处），实际 ${dup.status}`
    ).toBe(500);
    expect(failureCode(dup)).toBe('DATABASE_ERROR');
    const listAfterDup = await apiCallRaw<PagedEnvelope<CustomerSpecialRow>>(
      page,
      'GET',
      `/color-prices/customer-special?customer_id=${customerId}&page=1&page_size=20`
    );
    expect(listAfterDup.items.length, '被拒的同键重复不许让行数增长').toBe(2);
    const firstRow = listAfterDup.items.find(r => Number(r.id) === id);
    expect(firstRow, '首行必须仍在列').toBeDefined();
    expect(
      toNum(firstRow!.special_price),
      `被拒的重复提交不许覆盖首行价格，实际 ${str(firstRow!.special_price)}`
    ).toBe(88.5);

    // 5) 异 valid_from 同三键是合法新行 证明唯一键第四列真实参与复合判定
    const newDateRow = await apiCallRaw<CustomerSpecialRow>(
      page,
      'POST',
      '/color-prices/customer-special',
      {
        customer_id: customerId,
        product_id: host.productId,
        color_id: host.colorId,
        special_price: '70.00',
        currency: 'CNY',
        valid_from: ymd(3),
      }
    );
    expect(
      Number.isInteger(Number(newDateRow.id)) && Number(newDateRow.id) > id,
      `异 valid_from 的同三键新行必须落库，实际 ${JSON.stringify(newDateRow)}`
    ).toBe(true);
    specialIds.push(Number(newDateRow.id));
    const listFinal = await apiCallRaw<PagedEnvelope<CustomerSpecialRow>>(
      page,
      'GET',
      `/color-prices/customer-special?customer_id=${customerId}&page=1&page_size=20`
    );
    expect(listFinal.items.length, '最终列表必须三行').toBe(3);
    expect(
      listFinal.items
        .map(r => str(r.valid_from))
        .sort()
        .join(','),
      '三行的 valid_from 集合必须精确等于三个日期'
    ).toBe([validFrom, ymd(2), ymd(3)].sort().join(','));
  });
});
