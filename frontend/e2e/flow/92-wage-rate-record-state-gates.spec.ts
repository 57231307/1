import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  deferCleanup,
  flushDeferredCleanups,
  expectStateGateRejection,
  type DeferredCleanup,
} from './helpers';

/**
 * 92 产量工资契约缺口补测（工序工价状态机 计件单价数值落库 工资记录建单与状态门）
 *
 * 与既有覆盖的分工。24 号对本域只发过 verifyEndpointHealthy 与 safePostAction
 * 纯状态探针（判定器 b1 b3 档，返回值不落断言），31b 号只探删除守卫。本 spec 是
 * 工价与工资记录全部写端点的首个真契约覆盖，钉落库字段、状态流转与门禁拒绝。
 *
 * 端点与信封真相（backend/src/routes/production.rs 的 wage 路由组，挂载前缀
 * /api/v1/erp/production）：
 * POST 工价创建（wage_handler::create_wage_rate）、GET 详情（get_wage_rate）、
 * GET 按单号（get_wage_rate_by_no）、GET 生效查询（get_effective_wage_rate）、
 * PUT 更新（update_wage_rate）、POST activate disable 两条状态流转、
 * DELETE 软删（delete_wage_rate）。
 * POST 工资记录创建（create_wage_record）、GET 详情（get_wage_record）、
 * GET 按单号（get_wage_record_by_no）、PUT 更新（update_wage_record）、
 * POST calculate confirm pay cancel 四条状态流转。
 * 单对象端点出参 data 为模型本体，分页端点出参 PaginatedResponse 四键。
 *
 * 状态与词表真相（backend/src/models/status/wage_energy_chemical_business.rs）：
 * 工价状态 draft active disabled（wage_rate_status）；工资记录状态
 * draft confirmed paid cancelled（wage_record_status）；工价类型
 * piece time mixed（wage_type）。
 * 业务门真相（backend/src/services/wage_ops 与 models/dto/wage_dto.rs）：
 * 建单校验链 rate.rs::create 依次校验 工序路线存在性、工价类型词表、单价非负、
 * 类型与单价匹配（piece 必须 piece_price 大于 0）、等级系数区间 0 到 1、
 * 失效日期晚于生效日期。工资记录 record.rs::create 校验周期结束不早于周期开始。
 * 状态门 rate.rs::transition_status 要求当前态匹配前态，record.rs::calculate
 * 与 confirm 与 pay 与 cancel 分别钉 draft 有明细 confirmed 与可取消态。
 * 单号由统一生成器取号，工价 PWR 前缀，工资记录 WR 前缀，均为日期八位加流水三位。
 * 生效查询 get_effective_by_route 只回 active 且日期区间覆盖的工价，
 * 停用后该查询必须回空。
 *
 * 假绿防线。负例一律 apiCallExpectFail 加 expectStateGateRejection 钉 HTTP 400
 * 与状态门机器码族，不读文案（AppError 业务文案在出参被掩码为常量）。每条被拒
 * 写操作都紧跟 GET 回读断状态与原值零漂移。rust_decimal 出参为字符串，比较前
 * Number 归一。可空未提交列断 null，不接受 0 占位。用例数据自建自流转，
 * 唯一 workshop 码隔离计数探针，清理走延迟队列（工价与工资记录软删只放行 draft，
 * 已流转记录由状态门拒绝删除，tryCleanup 告警不阻断，如实留痕）。
 *
 * 待裁项说明。calculate 正向产明细依赖周期内 completed 工序流转记录
 * （wage_ops/calculation.rs::load_step_records），该链要整条工序扫码流转夹具，
 * 本 spec 只钉门禁负例与空周期拒绝，confirm pay 正向到账链待工序流转夹具批次补齐。
 */

type Row = Record<string, unknown>;

/** 延迟清理队列（范式同 70 号，全部断言之后统一 flush，队列本文件私有） */
const CLEANUP: DeferredCleanup[] = [];
test.afterEach(async ({ page }) => {
  await flushDeferredCleanups(page, CLEANUP);
});

/** rust_decimal 经 JSON 序列化为字符串，比较前归一 */
const toNum = (v: unknown): number => Number(String(v));

const ymd = (daysFromToday: number): string =>
  new Date(Date.now() + daysFromToday * 86_400_000).toISOString().slice(0, 10);

/** 自建工序路线（工价建单要求路线真实存在，wage_ops/rate.rs::create 首道校验） */
async function createProcessRoute(page: Page, tag: string): Promise<{ id: number; code: string }> {
  const code = genCode(tag);
  const res = await apiCall<Row>(page, 'POST', '/production/process-routes', {
    route_code: code,
    route_name: `92号工序${code}`,
    seq: 1,
    process_type: 'dye',
  });
  const id = Number(res.data?.id);
  expect(id, `工序路线创建应返回 id，实际 ${JSON.stringify(res)}`).toBeGreaterThan(0);
  deferCleanup(CLEANUP, 'DELETE', `/production/process-routes/${id}`, `[92] route ${code}`);
  return { id, code };
}

/** 自建工价（默认 piece 类型正单价，返回体即落库回显，后续仍以 GET 详情为准） */
async function createWageRate(
  page: Page,
  routeId: number,
  workshop: string,
  overrides: Record<string, unknown> = {}
): Promise<{ id: number; rateNo: string }> {
  const res = await apiCall<Row>(page, 'POST', '/production/wage-rates', {
    process_route_id: routeId,
    wage_type: 'piece',
    piece_price: '2.50',
    effective_date: ymd(0),
    workshop,
    remarks: `92号工价${workshop}`,
    ...overrides,
  });
  const id = Number(res.data?.id);
  expect(id, `工价创建应返回 id，实际 ${JSON.stringify(res)}`).toBeGreaterThan(0);
  const rateNo = String(res.data?.rate_no);
  deferCleanup(CLEANUP, 'DELETE', `/production/wage-rates/${id}`, `[92] rate ${rateNo}`);
  return { id, rateNo };
}

/** 自建工资记录（周期由调用方给定，未来窗口用于钉空周期负例） */
async function createWageRecord(
  page: Page,
  workshop: string,
  periodStart: string,
  periodEnd: string
): Promise<{ id: number; recordNo: string }> {
  const res = await apiCall<Row>(page, 'POST', '/production/wage-records', {
    period_start: periodStart,
    period_end: periodEnd,
    workshop,
    remarks: `92号工资单${workshop}`,
  });
  const id = Number(res.data?.id);
  expect(id, `工资记录创建应返回 id，实际 ${JSON.stringify(res)}`).toBeGreaterThan(0);
  const recordNo = String(res.data?.record_no);
  deferCleanup(CLEANUP, 'DELETE', `/production/wage-records/${id}`, `[92] record ${recordNo}`);
  return { id, recordNo };
}

/** 按 workshop 精确过滤的工价分页探针（负例零写入判据，四键全断） */
async function countRatesByWorkshop(page: Page, workshop: string): Promise<number> {
  const data = await apiCallRaw<{
    items?: unknown;
    total?: unknown;
    page?: unknown;
    page_size?: unknown;
  }>(page, 'GET', `/production/wage-rates?workshop=${workshop}&page=1&page_size=5`);
  expect(
    Array.isArray(data?.items),
    `工价分页 items 必须为数组，实际 ${JSON.stringify(data)}`
  ).toBe(true);
  expect(typeof data.total, '工价分页 total 必须为数字（禁兜底默认值）').toBe('number');
  expect(typeof data.page, '工价分页 page 必须回显').toBe('number');
  expect(typeof data.page_size, '工价分页 page_size 必须回显').toBe('number');
  return Number(data.total);
}

/** 按 workshop 精确过滤的工资记录分页探针 */
async function countRecordsByWorkshop(page: Page, workshop: string): Promise<number> {
  const data = await apiCallRaw<{
    items?: unknown;
    total?: unknown;
    page?: unknown;
    page_size?: unknown;
  }>(page, 'GET', `/production/wage-records?workshop=${workshop}&page=1&page_size=5`);
  expect(
    Array.isArray(data?.items),
    `工资记录分页 items 必须为数组，实际 ${JSON.stringify(data)}`
  ).toBe(true);
  expect(typeof data.total, '工资记录分页 total 必须为数字').toBe('number');
  return Number(data.total);
}

test.describe.serial('92 产量工资契约：工价状态机与工资记录建单门禁', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('92-01 工价建单落真值，详情与单号双读路径一致，可空列保持 null', async ({ page }) => {
    const route = await createProcessRoute(page, 'E92RTA');
    const workshop = genCode('E92WSA');

    const created = await createWageRate(page, route.id, workshop, {
      grade_a_ratio: '1.0',
      grade_b_ratio: '0.8',
      grade_c_ratio: '0.5',
      expiry_date: ymd(30),
    });
    // 建单回显即钉状态与单号形态（wage_ops/rate.rs 建单固定 draft，统一生成器 PWR 前缀）
    expect(created.rateNo, `工价单号应为 PWR 加日期八位加流水三位，实际 ${created.rateNo}`).toMatch(
      /^PWR\d{11}$/
    );
    expect(String(created.rateNo).slice(3, 11), '单号日期段应为今日 UTC 日期').toBe(
      new Date().toISOString().slice(0, 10).replace(/-/g, '')
    );

    // 详情回读逐字段比对库里真值，不只信 POST 回显
    const got = await apiCallRaw<Row>(page, 'GET', `/production/wage-rates/${created.id}`);
    expect(Number(got.id), '详情 id 应等于建单 id').toBe(created.id);
    expect(got.status, '详情状态应为 draft').toBe('draft');
    expect(got.wage_type, '工价类型应逐字落库 piece').toBe('piece');
    expect(toNum(got.piece_price), '计件单价应落为 2.50 真值').toBe(2.5);
    expect(toNum(got.time_price), '计时单价未提交应落 0（rate.rs 缺省语义）').toBe(0);
    expect(toNum(got.grade_a_ratio), 'A 级系数应落 1.0 真值').toBe(1);
    expect(toNum(got.grade_b_ratio), 'B 级系数应落 0.8 真值').toBe(0.8);
    expect(toNum(got.grade_c_ratio), 'C 级系数应落 0.5 真值').toBe(0.5);
    expect(String(got.route_code), '路线编码应冗余自工序路线').toBe(route.code);
    expect(Number(got.process_route_id), '路线 id 引用应落库').toBe(route.id);
    expect(String(got.effective_date), '生效日期应逐字回读').toBe(ymd(0));
    expect(String(got.expiry_date), '失效日期应逐字回读').toBe(ymd(30));
    expect(String(got.workshop), '车间应逐字落库').toBe(workshop);
    // is_deleted 建表 NOT NULL，出参缺键即行形漂移，必须判红不得兜底吞掉
    expect(got.is_deleted, '未删除记录 is_deleted 必须为假且键必须在册').toBe(false);

    // 单号读路径与 id 读路径必须指向同一行
    const byNo = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/wage-rates/by-no/${created.rateNo}`
    );
    expect(Number(byNo.id), 'by-no 与详情应回同一行').toBe(created.id);
    expect(toNum(byNo.piece_price), 'by-no 单价应与详情一致').toBe(2.5);

    // 分页探针应至少含自建行（workshop 唯一码隔离）
    expect(
      await countRatesByWorkshop(page, workshop),
      `按车间过滤应命中自建工价 1 条，workshop=${workshop}`
    ).toBe(1);
  });

  test('92-02 工价建单五道校验门全被拒且零写入', async ({ page }) => {
    const route = await createProcessRoute(page, 'E92RTB');
    const workshop = genCode('E92WSB');

    // 工价类型非法（wage_type 词表 piece time mixed 之外）
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', '/production/wage-rates', {
        process_route_id: route.id,
        wage_type: 'hourly',
        piece_price: '1.00',
        effective_date: ymd(0),
        workshop,
      }),
      '非法工价类型应被业务门拒绝'
    );

    // piece 类型缺正计件单价（validate_wage_type_prices）
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', '/production/wage-rates', {
        process_route_id: route.id,
        wage_type: 'piece',
        piece_price: '0',
        effective_date: ymd(0),
        workshop,
      }),
      '计件类型零单价应被业务门拒绝'
    );

    // 计件单价为负（validate_prices_non_negative）
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', '/production/wage-rates', {
        process_route_id: route.id,
        wage_type: 'piece',
        piece_price: '-1.00',
        effective_date: ymd(0),
        workshop,
      }),
      '负计件单价应被业务门拒绝'
    );

    // 失效日期不晚于生效日期（validate_expiry_date）
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', '/production/wage-rates', {
        process_route_id: route.id,
        wage_type: 'piece',
        piece_price: '2.00',
        effective_date: ymd(10),
        expiry_date: ymd(5),
        workshop,
      }),
      '失效早于生效应被业务门拒绝'
    );

    // 幽灵工序路线（create 首道校验路线存在性）
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', '/production/wage-rates', {
        process_route_id: 999999999,
        wage_type: 'piece',
        piece_price: '2.00',
        effective_date: ymd(0),
        workshop,
      }),
      '不存在的工序路线应被业务门拒绝'
    );

    // 五道拒绝之后该车间零落库（被拒写操作不留痕，掩码拒绝不等于部分写入）
    expect(
      await countRatesByWorkshop(page, workshop),
      `五道校验门拒绝后不应有任何落库行，workshop=${workshop}`
    ).toBe(0);
  });

  test('92-03 工价 draft active disabled 全程流转与状态门零漂移', async ({ page }) => {
    const route = await createProcessRoute(page, 'E92RTC');
    const workshop = genCode('E92WSC');
    const created = await createWageRate(page, route.id, workshop, {
      // 失效日期不提交，可空列必须保持 null（生效查询用 expiry is null 分支）
      grade_c_ratio: '0',
    });

    const draftView = await apiCallRaw<Row>(page, 'GET', `/production/wage-rates/${created.id}`);
    expect(draftView.expiry_date, '未提交的失效日期应存 NULL 而非占位值').toBeNull();

    // 生效查询在 draft 期应为空（get_effective_by_route 只认 active）
    const onDate = encodeURIComponent(new Date().toISOString());
    const effDraft = await apiCallRaw<unknown>(
      page,
      'GET',
      `/production/wage-rates/effective/${route.id}?on_date=${onDate}`
    );
    expect(effDraft, 'draft 工价不应出现在生效查询，实际 ' + JSON.stringify(effDraft)).toBeNull();

    // draft 期更新合法（update 门只放行 draft），改后回读真值
    await apiCall(page, 'PUT', `/production/wage-rates/${created.id}`, {
      piece_price: '3.75',
      remarks: '92-03草稿期更新',
    });
    const edited = await apiCallRaw<Row>(page, 'GET', `/production/wage-rates/${created.id}`);
    expect(toNum(edited.piece_price), 'draft 期更新单价应落库 3.75').toBe(3.75);
    expect(String(edited.remarks), 'draft 期更新备注应落库').toBe('92-03草稿期更新');

    // activate draft 到 active，生效查询命中同一行
    const activated = await apiCall<Row>(
      page,
      'POST',
      `/production/wage-rates/${created.id}/activate`
    );
    expect(String(activated.data?.status), 'activate 应返回 active 态').toBe('active');
    const effActive = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/wage-rates/effective/${route.id}?on_date=${onDate}`
    );
    expect(Number(effActive.id), 'active 且日期覆盖的工价应被生效查询命中').toBe(created.id);

    // 重复 activate 被拒（transition_status 前态匹配 draft，当前 active）且状态零漂移
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', `/production/wage-rates/${created.id}/activate`),
      'active 工价重复 activate 应被状态门拒绝'
    );
    const afterDupActivate = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/wage-rates/${created.id}`
    );
    expect(String(afterDupActivate.status), '被拒的 activate 不应改变状态').toBe('active');

    // active 期更新被拒且原值未变（update 门只放行 draft）
    expectStateGateRejection(
      await apiCallExpectFail(page, 'PUT', `/production/wage-rates/${created.id}`, {
        piece_price: '9.99',
      }),
      'active 工价更新应被状态门拒绝'
    );
    const afterDupUpdate = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/wage-rates/${created.id}`
    );
    expect(toNum(afterDupUpdate.piece_price), '被拒的更新不应写入 9.99，原值须保留').toBe(3.75);

    // disable active 到 disabled，生效查询回空
    const disabled = await apiCall<Row>(
      page,
      'POST',
      `/production/wage-rates/${created.id}/disable`
    );
    expect(String(disabled.data?.status), 'disable 应返回 disabled 态').toBe('disabled');
    const effDisabled = await apiCallRaw<unknown>(
      page,
      'GET',
      `/production/wage-rates/effective/${route.id}?on_date=${onDate}`
    );
    expect(
      effDisabled,
      'disabled 工价必须退出生效查询，实际 ' + JSON.stringify(effDisabled)
    ).toBeNull();

    // disabled 上 activate 被拒（前态匹配 draft）且状态零漂移
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', `/production/wage-rates/${created.id}/activate`),
      'disabled 工价 activate 应被状态门拒绝'
    );
    const afterBadActivate = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/wage-rates/${created.id}`
    );
    expect(String(afterBadActivate.status), '被拒流转后状态应保持 disabled').toBe('disabled');
  });

  test('92-04 工资记录建单落真值，周期校验门拒绝且零写入', async ({ page }) => {
    const workshop = genCode('E92WSD');
    const created = await createWageRecord(page, workshop, ymd(0), ymd(0));

    expect(
      created.recordNo,
      `工资单号应为 WR 加日期八位加流水三位，实际 ${created.recordNo}`
    ).toMatch(/^WR\d{11}$/);

    const got = await apiCallRaw<Row>(page, 'GET', `/production/wage-records/${created.id}`);
    expect(got.status, '新建工资记录应为 draft').toBe('draft');
    expect(String(got.period_start), '周期开始应逐字落库').toBe(ymd(0));
    expect(String(got.period_end), '周期结束应逐字落库').toBe(ymd(0));
    expect(String(got.workshop), '车间应逐字落库').toBe(workshop);
    expect(toNum(got.total_amount), '建单总额应为 0（计算前零值语义）').toBe(0);
    expect(Number(got.total_workers), '建单总人数应为 0').toBe(0);
    expect(Number(got.total_step_records), '建单工序记录数应为 0').toBe(0);
    // 未发生的确认与发放审计列必须保持 NULL，不得被 0 或时间占位
    expect(got.confirmed_by, '未确认行 confirmed_by 应存 NULL').toBeNull();
    expect(got.confirmed_at, '未确认行 confirmed_at 应存 NULL').toBeNull();
    expect(got.paid_by, '未发放行 paid_by 应存 NULL').toBeNull();
    expect(got.paid_at, '未发放行 paid_at 应存 NULL').toBeNull();

    const byNo = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/wage-records/by-no/${created.recordNo}`
    );
    expect(Number(byNo.id), 'by-no 与详情应回同一行').toBe(created.id);

    // 周期倒挂负例（record.rs::create 校验周期结束不早于开始）后该车间仍只有 1 行
    const failWorkshop = genCode('E92WSE');
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', '/production/wage-records', {
        period_start: ymd(10),
        period_end: ymd(5),
        workshop: failWorkshop,
      }),
      '周期结束早于开始应被业务门拒绝'
    );
    expect(
      await countRecordsByWorkshop(page, failWorkshop),
      `周期倒挂拒绝后不应落库，workshop=${failWorkshop}`
    ).toBe(0);
  });

  test('92-05 工资记录 calculate confirm pay 门禁拒绝零漂移，cancel 全程', async ({ page }) => {
    const workshop = genCode('E92WSF');
    // 未来周期保证周期内无任何 completed 工序记录（calculation.rs 空周期分支确定性成立）
    const created = await createWageRecord(page, workshop, ymd(365), ymd(366));

    // 空周期 calculate 被拒，且记录汇总列保持建单零值（拒绝不落任何明细与半成品）
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', `/production/wage-records/${created.id}/calculate`, {
        recalculate: false,
      }),
      '未来空周期 calculate 应被业务门拒绝'
    );
    const afterCalc = await apiCallRaw<Row>(page, 'GET', `/production/wage-records/${created.id}`);
    expect(String(afterCalc.status), '被拒的 calculate 不应改变状态').toBe('draft');
    expect(toNum(afterCalc.total_amount), '被拒的 calculate 后总额应保持 0').toBe(0);
    // total_workers 建表 NOT NULL 且必出参，先断在册与类型再断值，不得用 0 顶漂移
    expect(
      typeof afterCalc.total_workers,
      '被拒的 calculate 后 total_workers 键必须在册且为数字'
    ).toBe('number');
    expect(Number(afterCalc.total_workers), '被拒的 calculate 后总人数应保持 0').toBe(0);

    // 无明细 confirm 被拒（confirm 门 detail 计数为 0），确认审计列保持 NULL
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', `/production/wage-records/${created.id}/confirm`),
      '无明细 confirm 应被业务门拒绝'
    );
    const afterConfirm = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/wage-records/${created.id}`
    );
    expect(String(afterConfirm.status), '被拒的 confirm 不应改变状态').toBe('draft');
    expect(afterConfirm.confirmed_by, '被拒的 confirm 不应写入确认人').toBeNull();

    // draft 直接 pay 被拒（pay 门只放行 confirmed）
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', `/production/wage-records/${created.id}/pay`),
      'draft 直接 pay 应被状态门拒绝'
    );
    const afterPay = await apiCallRaw<Row>(page, 'GET', `/production/wage-records/${created.id}`);
    expect(String(afterPay.status), '被拒的 pay 不应改变状态').toBe('draft');
    expect(afterPay.paid_by, '被拒的 pay 不应写入发放人').toBeNull();

    // cancel draft 到 cancelled 是合法流转（record.rs::cancel 放行 draft 与 confirmed）
    const cancelled = await apiCall<Row>(
      page,
      'POST',
      `/production/wage-records/${created.id}/cancel`
    );
    expect(String(cancelled.data?.status), 'cancel 应返回 cancelled 态').toBe('cancelled');
    const byNo = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/wage-records/by-no/${created.recordNo}`
    );
    expect(String(byNo.status), '单号读路径应回读到 cancelled').toBe('cancelled');

    // cancelled 终态上的 confirm pay cancel 更新全部被拒且状态零漂移
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', `/production/wage-records/${created.id}/confirm`),
      'cancelled 上 confirm 应被状态门拒绝'
    );
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', `/production/wage-records/${created.id}/pay`),
      'cancelled 上 pay 应被状态门拒绝'
    );
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', `/production/wage-records/${created.id}/cancel`),
      'cancelled 上重复 cancel 应被状态门拒绝'
    );
    expectStateGateRejection(
      await apiCallExpectFail(page, 'PUT', `/production/wage-records/${created.id}`, {
        remarks: '终态后更新应被拒',
      }),
      'cancelled 上更新应被状态门拒绝'
    );
    const finalView = await apiCallRaw<Row>(page, 'GET', `/production/wage-records/${created.id}`);
    expect(String(finalView.status), '终态上四条被拒操作后状态应保持 cancelled').toBe('cancelled');
    expect(String(finalView.remarks), '被拒更新不得改写备注原值').toBe(`92号工资单${workshop}`);
  });
});
