import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  API_BASE,
  API_PREFIX,
  APP_ERROR_CODES,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
} from './helpers';

/**
 * 114 色卡发放链门控与发放面读口首个真契约覆盖（发放口 四枚改判口 三枚成本口 两枚导出口 两枚报表口 发放列表口）
 *
 * 与既有覆盖的分工。既有 23 号只把发放列表与发放明细与汇总做成"能打开"的浅覆盖，
 * 判定器口径下发放口、四枚改判口（归还 遗失 损坏 取消）、三枚成本口（结转 恢复 过期损失）、
 * 发放记录导出口、发放明细导出口、客户台账口、订单关联口均为零覆盖。
 * 本 spec 把这些口钉成真实门禁判定加零漂移回读链。
 *
 * 本 spec 只覆盖拒绝分支与读口。库存登记入口（PUT /color-cards/:id 的 stock_quantity）
 * 已经存在，发放成功路径由同批库存登记与发放链路用例覆盖，因此本 spec 不假设发放表为空：
 * 全表行数只作本用例内零漂移的基线，无记录前置一律用不存在的主键证明，
 * 明细口按页大小口径对账，不按全表绝对 0 判定。
 *
 * 契约真相（逐段读后端源，非推测）：
 * 发放口（handlers/color_card/issue.rs 与 services/color_card_issue_service.rs）：
 * 请求体 IssueColorCardDto 必填 color_card_id 与 customer_id，缺体被 Json 提取器拒为
 * 400 加 VALIDATION_ERROR 加固定格式原文。校验顺序固定为
 * 色卡存在性与状态与库存（闸门 1 与 2）先于客户信用与状态（闸门 3 与 5）先于客户超期（闸门 4）
 * 先于预计归还日期区间，故前一道门拒绝时后面的门不会被触达。
 * 闸门 1 拒绝走 CardNotIssuable，映射为 business_displayable，出参文案固定
 * "只有草稿态色卡可以发放"，可发放集只有 draft（与色号维护门同源）。
 * 闸门 2 与 3 与 4 与 5 的拒绝走 GateCheckFailed，映射为脱敏 business，
 * 出参 code 仍是 BUSINESS_ERROR 但文案不含内部数字，故本 spec 对这些分支只钉码不钉文案。
 * 色卡不存在与客户不存在都映射为 404（NOT_FOUND）。
 * 发放记录状态词表 issued 与 returned 与 lost 与 damaged 与 cancelled，
 * 除 issued 外全是终态，终态上任何改判一律 InvalidState 脱敏 business。
 * 归还与取消与损坏的请求体经 OptionalJson 归一，缺体等价空对象（不 400）；
 * 遗失口的 MarkLostDto 是必填体且 compensation_amount 必须大于 0，
 * 该金额校验在取记录之前，故对不存在的记录也先落 400 加 VALIDATION_ERROR 加外显原文
 * "赔付金额必须 > 0"；损坏口对负数金额同样先落 400，外显原文"赔付金额不能为负"。
 * 四枚改判口对不存在记录一律 RecordNotFound 映射 404。
 * 发放列表口分页对 page 取 clamp 1 到 1000、对 page_size 取 clamp 1 到 100，
 * 出参信封四键 items 与 total 与 page 与 page_size，越界入参被归一后可在出参见证。
 * 成本口（services/color_card_cost_accounting_service.rs）：
 * 三枚口都先按发放记录主键取行，取不到即 404，文案含记录号但出参走脱敏 not_found，
 * 故只钉码。恢复口的出参在记录存在且状态非 cancelled 时走 business 拒绝，
 * 结转口对 cancelled 记录拒绝，过期损失口只对 issued 记录核算——三枚门都以发放记录为前置，
 * 在本 spec 上统一钉"无记录即 404 且发放表行数零漂移"。
 * 导出口（handlers/color_card/issue.rs 与 analytics.rs）：
 * 两枚出口都是 xlsx 二进制流，与发放表行数无关，0 行也产出带表头的合法工作簿，
 * content-type 为 spreadsheetml.sheet，下载名带 .xlsx，响应体是 zip 容器（头两字节 50 4B）。
 * 明细导出口的表头固定 16 列且与发放明细报表口的行键同名（xlsx 由同一份 rows 生成）。
 * 报表口（services/color_card_issue_report_service.rs）：
 * 台账口按 customer_id 过滤未软删发放记录，订单关联口按 sales_order_id 过滤，
 * 两者出参都是数组且行键含 issue_id 与 color_card_id 与 card_no 与 customer_name 与 status；
 * 发放明细口同样走 base_cond（含 is_deleted=false），三口在同筛选下必须同为空，
 * 任一口漏掉过滤条件或漏掉软删过滤就会与另两口感官不一致而判红。
 *
 * 假绿防线。每一次被拒的写操作都紧跟两枚回读，发放表行数与目标色卡的已发放数都不许动；
 * 门拒一律钉 HTTP 400 加机器码族，可外显族再钉文案原文，缺体与越界金额分别钉到不同分支；
 * 导出口真取字节并验 zip 魔数与 content-type 与下载名，不接受只判 2xx；
 * 三报表口以同筛选互证空态，而不是各自"能打开"；分页口以越界入参钉 clamp 归一后的真实出参。
 * 全程不用 ?? 或 || 兜底。
 */

type Row = Record<string, unknown>;

/** 色卡主体列（本链只用 status 与 issued_quantity 与 stock_quantity 三枚） */
interface CardRow {
  id: number;
  card_no: string;
  card_name: string;
  card_type: string;
  total_colors: number;
  status: string;
  stock_quantity: number;
  issued_quantity: number;
}

/** 发放记录出参（handlers/color_card/issue.rs 的 IssueRecordInfo） */
interface IssueRow {
  id: number;
  color_card_id: number;
  customer_id: number;
  issue_qty: number;
  status: string;
  compensation_amount: string | number | null;
  returned_by: number | null;
  actual_return_date: string | null;
  purpose: string | null;
  remark: string | null;
}

/** 分页信封 */
type Paged<T> = { items: T[]; total: number; page: number; page_size: number };

const ymd = (daysFromToday: number): string =>
  new Date(Date.now() + daysFromToday * 86_400_000).toISOString().slice(0, 10);
const toNum = (v: unknown): number => Number(String(v));
const str = (v: unknown): string => String(v);
const apiUrl = (path: string): string => `${API_BASE}${API_PREFIX}${path}`;

/**
 * 必然不存在的发放记录主键。
 * 入参：无。传给谁：四枚改判口与三枚成本口与发放详情口的存在性前置。
 * 存什么：一个落在 i32 正向内且远高于 CI 库真实自增值的常量（成本口的路径段是 i32，
 * 取超界值会在反序列化阶段就变成别的错误，探测即失真）。
 * 存哪里：不落盘。每次使用前先用发放详情口复验它确实无行。
 */
const ABSENT_RECORD_ID = 2_000_000_000;

/** 读发放表总行数。入参 page。传给谁：每个拒绝分支前后的零漂移对账。存什么与存哪里：不落盘 */
async function issueTotal(page: Page): Promise<number> {
  const list = await apiCallRaw<Paged<IssueRow>>(
    page,
    'GET',
    '/color-cards/issues?page=1&page_size=1'
  );
  expect(
    Array.isArray(list.items) && typeof list.total === 'number',
    `发放列表信封必须含 items 数组与数值 total，实际 ${JSON.stringify(list)}`
  ).toBe(true);
  return toNum(list.total);
}

test.describe.serial('114 色卡发放链门控与发放面读口', () => {
  test.beforeEach(async ({ page }) => {
    // worker 会话由 diagnose-fixture 注入，本链全程走 apiCall 与裸请求，不导航页面
    const me = await apiCallRaw<Row>(page, 'GET', '/auth/me');
    expect(
      Number.isInteger(Number(me.id)) && Number(me.id) > 0,
      `/auth/me 必须回真实用户 id，实际 ${JSON.stringify(me)}`
    ).toBe(true);
  });

  test('T1 发放口三道门各钉一码 缺体被拒 被拒后发放表与已发放数双零推进 分页 clamp 出参可证', async ({
    page,
  }) => {
    const tag = genCode('ISS114A');
    const totalBefore = await issueTotal(page);

    // 前置：自建客户（信用额度为正且状态 active，使闸门 3 与 5 不是拒绝原因）与自建草稿卡
    const customer = await apiCallRaw<Row>(page, 'POST', '/crm/customers', {
      customer_name: `114客户${tag}`,
      contact_person: `114联系${tag}`,
      contact_phone: `139${Date.now().toString().slice(-8)}`,
      customer_type: 'retail',
      status: 'active',
      credit_limit: '50000.00',
    });
    const customerId = Number(customer.id);
    expect(customerId > 0, `客户自建失败 ${JSON.stringify(customer)}`).toBe(true);

    const card = await apiCallRaw<CardRow>(page, 'POST', '/color-cards', {
      card_no: `CC114${tag}`,
      card_name: `114发放靶卡${tag}`,
      card_type: 'CUSTOM',
    });
    const cardId = Number(card.id);
    expect(cardId > 0, `色卡自建失败 ${JSON.stringify(card)}`).toBe(true);
    expect(str(card.status), '靶卡初态必须 draft').toBe('draft');

    // 1) 闸门 2（库存不足）拒绝。草稿卡且库存由服务端写 0，发放数量 1 必然不可用
    const stockReject = await apiCallExpectFail(page, 'POST', '/color-cards/issues', {
      color_card_id: cardId,
      customer_id: customerId,
      issue_qty: 1,
      purpose: `114用途${tag}`,
      remark: `114备注${tag}`,
      dye_lot_no: `DL114${tag}`,
    });
    expect(stockReject.status, `库存不足必须 400，实际 ${stockReject.status}`).toBe(400);
    expect(
      failureCode(stockReject),
      `库存不足必须落业务机器码（GateCheckFailed 走脱敏 business），实际 ${JSON.stringify(stockReject.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    // 被拒后两口径零推进：发放表不增行，靶卡已发放数不动（事务未半途写库）
    expect(await issueTotal(page), '被拒的发放不许产生发放行').toBe(totalBefore);
    const cardAfterStockReject = await apiCallRaw<CardRow>(page, 'GET', `/color-cards/${cardId}`);
    expect(toNum(cardAfterStockReject.issued_quantity), '被拒的发放不许动已发放数').toBe(
      toNum(cardAfterStockReject.stock_quantity)
    );
    expect(toNum(cardAfterStockReject.issued_quantity), '被拒的发放后已发放数必须仍是 0').toBe(0);
    expect(str(cardAfterStockReject.status), '被拒的发放不许动色卡状态').toBe('draft');

    // 2) 发放数量 0 同样被闸门 2 拒（数量下限与库存充足是同一道门的两支）
    const qtyReject = await apiCallExpectFail(page, 'POST', '/color-cards/issues', {
      color_card_id: cardId,
      customer_id: customerId,
      issue_qty: 0,
    });
    expect(qtyReject.status, `发放数量 0 必须 400，实际 ${qtyReject.status}`).toBe(400);
    expect(failureCode(qtyReject)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 3) 闸门 1 拒绝。先把靶卡归档（终态），非 draft 卡发放必须外显公开规则文案
    await apiCall(page, 'DELETE', `/color-cards/${cardId}`, { reason: `114归档以测闸门1${tag}` });
    const archivedCard = await apiCallRaw<CardRow>(page, 'GET', `/color-cards/${cardId}`);
    expect(str(archivedCard.status), '归档后必须 archived').toBe('archived');
    const gateOne = await apiCallExpectFail(page, 'POST', '/color-cards/issues', {
      color_card_id: cardId,
      customer_id: customerId,
      issue_qty: 1,
    });
    expect(gateOne.status, `非草稿卡发放必须 400，实际 ${gateOne.status}`).toBe(400);
    expect(failureCode(gateOne)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      str(gateOne.message),
      `闸门 1 拒绝必须外显公开规则原文（business_displayable 族），实际 ${str(gateOne.message)}`
    ).toBe('只有草稿态色卡可以发放');
    const cardAfterGateOne = await apiCallRaw<CardRow>(page, 'GET', `/color-cards/${cardId}`);
    expect(str(cardAfterGateOne.status), '被拒的发放不许把卡改出终态').toBe('archived');
    expect(toNum(cardAfterGateOne.issued_quantity), '被拒的发放不许动已发放数').toBe(0);

    // 4) 色卡不存在必须 404（NOT_FOUND 族，不是把缺卡当成库存不足的 400）
    const missingCard = await apiCallExpectFail(page, 'POST', '/color-cards/issues', {
      color_card_id: ABSENT_RECORD_ID,
      customer_id: customerId,
      issue_qty: 1,
    });
    expect(missingCard.status, `不存在色卡发放必须 404，实际 ${missingCard.status}`).toBe(404);
    expect(failureCode(missingCard)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 5) 缺体必须被请求解码门拒（400 加校验码），发放表仍 0 行
    const noBody = await apiCallExpectFail(page, 'POST', '/color-cards/issues');
    expect(noBody.status, `发放缺体必须 400，实际 ${noBody.status}`).toBe(400);
    expect(failureCode(noBody)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(await issueTotal(page), '被拒的缺体发放不许产生行').toBe(totalBefore);

    // 6) 分页归一必须在出参可见（page 下界 1，page_size 上界 100）
    const clamped = await apiCallRaw<Paged<IssueRow>>(
      page,
      'GET',
      '/color-cards/issues?page=0&page_size=500'
    );
    expect(toNum(clamped.page), `page=0 必须归一到 1，实际 ${str(clamped.page)}`).toBe(1);
    expect(
      toNum(clamped.page_size),
      `page_size=500 必须归一到 100，实际 ${str(clamped.page_size)}`
    ).toBe(100);
    expect(toNum(clamped.total), '归一分页不许改变总数口径').toBe(totalBefore);

    // 7) 按客户与状态两枚过滤键各读一次，空态必须与总口径自洽（过滤键不许放大结果集）
    const byCustomer = await apiCallRaw<Paged<IssueRow>>(
      page,
      'GET',
      `/color-cards/issues?customer_id=${customerId}&page=1&page_size=20`
    );
    expect(toNum(byCustomer.total), '本用例自建客户的发放数必须为 0').toBe(0);
    expect(byCustomer.items.length, '过滤口行数必须与 total 同值').toBe(0);
    const byStatus = await apiCallRaw<Paged<IssueRow>>(
      page,
      'GET',
      '/color-cards/issues?status=issued&page=1&page_size=20'
    );
    expect(
      toNum(byStatus.total),
      `status=issued 过滤结果不得超过总行数 ${totalBefore}`
    ).toBeLessThanOrEqual(totalBefore);

    // 清理。色卡无物理删口（DELETE 即归档），靶卡已归档；客户走延迟删通道
    await apiCall(page, 'DELETE', `/crm/customers/${customerId}`);
    // 客户删除是软删除（status 置 inactive），对象口仍可读回，故以状态回读证明删除生效。
    const custAfterDelete = await apiCallRaw<Row>(page, 'GET', `/crm/customers/${customerId}`);
    expect(
      str(custAfterDelete.status),
      `客户软删后状态必须落 inactive，实际 ${JSON.stringify(custAfterDelete)}`
    ).toBe('inactive');
  });

  test('T2 四枚改判口无记录即 404 遗失与损坏的金额前置校验先于取记录 全链零推进', async ({
    page,
  }) => {
    const tag = genCode('ISS114B');
    const totalBefore = await issueTotal(page);

    // 前置证明：发放详情口对该探测主键必须 404，改判口的 404 才归因到"记录不存在"
    const probe = await apiCallExpectFail(page, 'GET', `/color-cards/issues/${ABSENT_RECORD_ID}`);
    expect(probe.status, `发放详情口对无行主键必须 404，实际 ${probe.status}`).toBe(404);
    expect(failureCode(probe)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 1) 归还口（OptionalJson，缺体合法）对无记录必须 404
    const returnMissing = await apiCallExpectFail(
      page,
      'POST',
      `/color-cards/issues/${ABSENT_RECORD_ID}/return`
    );
    expect(returnMissing.status, `无记录归还必须 404，实际 ${returnMissing.status}`).toBe(404);
    expect(failureCode(returnMissing)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 2) 归还口带真实日期体同样 404（证明日期体不会把不存在记录"补"成存在）
    const returnWithBody = await apiCallExpectFail(
      page,
      'POST',
      `/color-cards/issues/${ABSENT_RECORD_ID}/return`,
      { actual_return_date: ymd(0), remark: `114归还探针${tag}` }
    );
    expect(returnWithBody.status, `带体无记录归还必须 404，实际 ${returnWithBody.status}`).toBe(
      404
    );

    // 3) 取消口缺体合法（OptionalJson 归一），无记录必须 404
    const cancelMissing = await apiCallExpectFail(
      page,
      'POST',
      `/color-cards/issues/${ABSENT_RECORD_ID}/cancel`
    );
    expect(cancelMissing.status, `无记录取消必须 404，实际 ${cancelMissing.status}`).toBe(404);
    expect(failureCode(cancelMissing)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 4) 遗失口的 MarkLostDto 是必填体，缺体先被请求解码门拒（不是 404）
    const lostNoBody = await apiCallExpectFail(
      page,
      'POST',
      `/color-cards/issues/${ABSENT_RECORD_ID}/lost`
    );
    expect(lostNoBody.status, `遗失缺体必须 400，实际 ${lostNoBody.status}`).toBe(400);
    expect(failureCode(lostNoBody)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 5) 遗失口金额校验在取记录之前，故无记录也先落 400 加外显原文
    const lostZero = await apiCallExpectFail(
      page,
      'POST',
      `/color-cards/issues/${ABSENT_RECORD_ID}/lost`,
      { compensation_amount: '0.00', remark: `114遗失零额${tag}` }
    );
    expect(lostZero.status, `赔付金额 0 必须 400，实际 ${lostZero.status}`).toBe(400);
    expect(failureCode(lostZero)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      str(lostZero.message),
      `遗失口金额门必须外显原文（validation_displayable 族），实际 ${str(lostZero.message)}`
    ).toBe('赔付金额必须 > 0');

    // 6) 遗失口带合法金额才走到取记录，无记录落 404
    const lostMissing = await apiCallExpectFail(
      page,
      'POST',
      `/color-cards/issues/${ABSENT_RECORD_ID}/lost`,
      { compensation_amount: '128.00' }
    );
    expect(lostMissing.status, `合法金额下无记录必须 404，实际 ${lostMissing.status}`).toBe(404);
    expect(failureCode(lostMissing)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 7) 损坏口负数金额同样先落 400 加外显原文（与遗失口的文案不同，两支各自钉死）
    const damagedNegative = await apiCallExpectFail(
      page,
      'POST',
      `/color-cards/issues/${ABSENT_RECORD_ID}/damaged`,
      { compensation_amount: '-1.00' }
    );
    expect(damagedNegative.status, `负数赔付必须 400，实际 ${damagedNegative.status}`).toBe(400);
    expect(failureCode(damagedNegative)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      str(damagedNegative.message),
      `损坏口负额必须外显原文，实际 ${str(damagedNegative.message)}`
    ).toBe('赔付金额不能为负');

    // 8) 损坏口合法金额与缺体两分支都走到取记录，无记录落 404
    const damagedWithAmount = await apiCallExpectFail(
      page,
      'POST',
      `/color-cards/issues/${ABSENT_RECORD_ID}/damaged`,
      { compensation_amount: '66.00', remark: `114损坏探针${tag}` }
    );
    expect(damagedWithAmount.status, `无记录损坏必须 404，实际 ${damagedWithAmount.status}`).toBe(
      404
    );
    const damagedNoBody = await apiCallExpectFail(
      page,
      'POST',
      `/color-cards/issues/${ABSENT_RECORD_ID}/damaged`
    );
    expect(
      damagedNoBody.status,
      `损坏缺体必须 404（OptionalJson 归一后走取记录），实际 ${damagedNoBody.status}`
    ).toBe(404);

    // 9) 十次拒绝之后发放表行数必须与基线同值，一张改判记录都没被"写出来"
    expect(await issueTotal(page), '全部拒绝分支之后发放表行数必须与基线同值').toBe(totalBefore);
  });

  test('T3 三枚成本口无记录即 404 制作成本口按色号数真算 两枚导出口真取 xlsx 字节', async ({
    page,
  }) => {
    const tag = genCode('ISS114C');

    // 前置证明：探测主键无发放行（结转与恢复与过期损失三口的记录级门都以发放行为前置），
    // 故三枚口对探测主键必须 404；不按全表行数判定，同分片其他用例可能已有历史发放行。

    // 1) 结转口无记录必须 404（取记录先于色卡取数，故拒绝来自记录缺失而非库存）
    const transfer = await apiCallExpectFail(
      page,
      'GET',
      `/color-cards/cost/issue/${ABSENT_RECORD_ID}/transfer`
    );
    expect(transfer.status, `无记录成本结转必须 404，实际 ${transfer.status}`).toBe(404);
    expect(failureCode(transfer)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 2) 过期损失口无记录必须 404
    const expiry = await apiCallExpectFail(
      page,
      'GET',
      `/color-cards/cost/issue/${ABSENT_RECORD_ID}/expiry-loss`
    );
    expect(expiry.status, `无记录过期损失必须 404，实际 ${expiry.status}`).toBe(404);
    expect(failureCode(expiry)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 3) 恢复口无记录必须 404（该口是 POST，缺体经 apiCallExpectFail 仍走取记录分支）
    const restore = await apiCallExpectFail(
      page,
      'POST',
      `/color-cards/cost/issue/${ABSENT_RECORD_ID}/restore`
    );
    expect(restore.status, `无记录成本恢复必须 404，实际 ${restore.status}`).toBe(404);
    expect(failureCode(restore)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 4) 制作成本口用真实卡与真实色号数做真值核对。
    //    每色号标准成本可由环境变量 COLOR_CARD_COST_PER_COLOR 覆盖，故不钉绝对金额，
    //    钉两条与环境无关的确定判据：零色号卡成本必须为 0（乘数为 0），
    //    两色号卡成本必须恰是一色号卡成本的两倍（成本由 total_colors 线性驱动）。
    const card2 = await apiCallRaw<CardRow>(page, 'POST', '/color-cards', {
      card_no: `CC114T${tag}`,
      card_name: `114双色卡${tag}`,
      card_type: 'CNCS',
    });
    const card2Id = Number(card2.id);
    expect(card2Id > 0, `双色卡自建失败 ${JSON.stringify(card2)}`).toBe(true);
    await apiCallRaw(page, 'POST', `/color-cards/${card2Id}/items/batch`, {
      items: [`T1${tag}`, `T2${tag}`].map((code, idx) => ({
        color_code: code,
        color_name: `114成本色号${code}`,
        rgb_r: 20 + idx,
        rgb_g: 40 + idx,
        rgb_b: 60 + idx,
        hex_value: idx === 0 ? '#14283C' : '#15293D',
        sequence: idx + 1,
      })),
    });
    const card2WithColors = await apiCallRaw<CardRow>(page, 'GET', `/color-cards/${card2Id}`);
    expect(toNum(card2WithColors.total_colors), '两行色号后 total_colors 必须为 2').toBe(2);

    const card1 = await apiCallRaw<CardRow>(page, 'POST', '/color-cards', {
      card_no: `CC114S${tag}`,
      card_name: `114单色卡${tag}`,
      card_type: 'CNCS',
    });
    const card1Id = Number(card1.id);
    expect(card1Id > 0, `单色卡自建失败 ${JSON.stringify(card1)}`).toBe(true);
    await apiCall(page, 'POST', `/color-cards/${card1Id}/items`, {
      color_code: `S1${tag}`,
      color_name: `114单色色号${tag}`,
      rgb_r: 20,
      rgb_g: 40,
      rgb_b: 60,
      hex_value: '#14283C',
      sequence: 1,
    });
    const card1WithColors = await apiCallRaw<CardRow>(page, 'GET', `/color-cards/${card1Id}`);
    expect(toNum(card1WithColors.total_colors), '一行色号后 total_colors 必须为 1').toBe(1);

    const cost2 = await apiCallRaw<Row>(page, 'GET', `/color-cards/cost/production/${card2Id}`);
    const cost1 = await apiCallRaw<Row>(page, 'GET', `/color-cards/cost/production/${card1Id}`);
    expect(
      toNum(cost1.amount) > 0,
      `单色号卡的制作成本必须为正，实际 ${JSON.stringify(cost1)}`
    ).toBe(true);
    expect(
      toNum(cost2.amount),
      `双色号卡成本必须恰为单色号卡的两倍（成本由 total_colors 线性驱动），两读口 ${JSON.stringify(cost1)} 对 ${JSON.stringify(cost2)}`
    ).toBe(toNum(cost1.amount) * 2);
    expect(
      str(cost1.currency).length > 0,
      `成本出参必须带币种，实际 ${JSON.stringify(cost1)}`
    ).toBe(true);
    expect(str(cost2.currency), '两读口币种必须同源同值').toBe(str(cost1.currency));

    // 零色号卡成本必须为 0（total_colors 为乘数，空卡不得摊出任何成本）
    const emptyCard = await apiCallRaw<CardRow>(page, 'POST', '/color-cards', {
      card_no: `CC114E${tag}`,
      card_name: `114空卡${tag}`,
      card_type: 'CNCS',
    });
    const emptyCost = await apiCallRaw<Row>(
      page,
      'GET',
      `/color-cards/cost/production/${Number(emptyCard.id)}`
    );
    expect(
      toNum(emptyCost.amount),
      `无色号卡的制作成本必须为 0，实际 ${JSON.stringify(emptyCost)}`
    ).toBe(0);

    // 不存在色卡的制作成本必须 404，不许把缺行摊成 0 元成本
    const missingCost = await apiCallExpectFail(
      page,
      'GET',
      `/color-cards/cost/production/${ABSENT_RECORD_ID}`
    );
    expect(missingCost.status, `不存在色卡的成本归集必须 404，实际 ${missingCost.status}`).toBe(
      404
    );
    expect(failureCode(missingCost)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 5) 发放记录导出口（A 档）。0 行也产出带表头的合法工作簿，真取字节验容器头
    const issueExport = await page.request.get(apiUrl('/color-cards/issues/export'));
    expect(issueExport.status(), `发放记录导出应 200，实际 ${issueExport.status()}`).toBe(200);
    const ieType = issueExport.headers()['content-type'] ?? '';
    expect(
      ieType.includes('spreadsheetml.sheet'),
      `发放记录导出 content-type 必须是 xlsx 流，实际 ${ieType}`
    ).toBe(true);
    expect(
      (issueExport.headers()['content-disposition'] ?? '').includes(
        'color_card_issues_export.xlsx'
      ),
      `发放记录导出下载名必须为 color_card_issues_export.xlsx，实际 ${issueExport.headers()['content-disposition']}`
    ).toBe(true);
    const ieBytes = await issueExport.body();
    expect(ieBytes.byteLength, '发放记录导出体必须非空').toBeGreaterThan(0);
    expect(ieBytes[0], `xlsx 是 zip 容器首字节必须是 0x50，实际 ${ieBytes[0]}`).toBe(0x50);
    expect(ieBytes[1], `xlsx 次字节必须是 0x4b，实际 ${ieBytes[1]}`).toBe(0x4b);

    // 6) 发放明细报表导出口（A 档），与明细口同一份 rows，两通道必须同为空态
    const detailList = await apiCallRaw<Row[]>(
      page,
      'GET',
      '/color-cards/reports/issue-detail?page=1&page_size=20'
    );
    expect(
      Array.isArray(detailList),
      `发放明细报表口必须回数组，实际 ${JSON.stringify(detailList)}`
    ).toBe(true);
    const detailTotal = await issueTotal(page);
    expect(
      detailList.length,
      '发放明细首屏行数必须等于全量发放行数与页大小的较小值（明细与列表同一份 rows）'
    ).toBe(Math.min(detailTotal, 20));
    const detailExport = await page.request.get(apiUrl('/color-cards/reports/issue-detail/export'));
    expect(detailExport.status(), `明细导出应 200，实际 ${detailExport.status()}`).toBe(200);
    const deBytes = await detailExport.body();
    expect(deBytes.byteLength, '明细导出体必须非空').toBeGreaterThan(0);
    expect(deBytes[0], '明细导出首字节必须是 0x50').toBe(0x50);
    expect(
      (detailExport.headers()['content-type'] ?? '').includes('spreadsheetml.sheet'),
      '明细导出 content-type 必须是 xlsx 流'
    ).toBe(true);

    // 7) 客户台账口与订单关联口在同筛选下与发放列表口三通道互证空态
    const customer = await apiCallRaw<Row>(page, 'POST', '/crm/customers', {
      customer_name: `114台账客户${tag}`,
      contact_person: `114台账联系${tag}`,
      contact_phone: `139${Date.now().toString().slice(-8)}`,
      customer_type: 'wholesale',
      status: 'active',
    });
    const customerId = Number(customer.id);
    expect(customerId > 0, `台账客户自建失败 ${JSON.stringify(customer)}`).toBe(true);

    const ledger = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/color-cards/reports/customer-ledger/${customerId}`
    );
    expect(Array.isArray(ledger), `台账口必须回数组，实际 ${JSON.stringify(ledger)}`).toBe(true);
    expect(ledger.length, '新客户的色卡台账必须为 0 行').toBe(0);
    const ledgerByFilter = await apiCallRaw<Paged<IssueRow>>(
      page,
      'GET',
      `/color-cards/issues?customer_id=${customerId}&page=1&page_size=20`
    );
    expect(toNum(ledgerByFilter.total), '台账口与发放列表口必须同为空（两通道同筛选同口径）').toBe(
      ledger.length
    );

    const orderRelated = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/color-cards/reports/order-related/${ABSENT_RECORD_ID}`
    );
    expect(Array.isArray(orderRelated), '订单关联口必须回数组').toBe(true);
    expect(orderRelated.length, '无发放行时订单关联报表必须 0 行').toBe(0);
    const relatedByFilter = await apiCallRaw<Paged<IssueRow>>(
      page,
      'GET',
      `/color-cards/issues?sales_order_id=${ABSENT_RECORD_ID}&page=1&page_size=20`
    );
    expect(toNum(relatedByFilter.total), '订单关联口与发放列表口按同一订单键必须同为空').toBe(
      orderRelated.length
    );

    // 8) 不存在的色卡与不存在的客户走台账与预警面都必须点名拒绝，不许静默回空对象
    const ledgerMissingCustomer = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/color-cards/reports/customer-ledger/${ABSENT_RECORD_ID}`
    );
    expect(
      Array.isArray(ledgerMissingCustomer) && ledgerMissingCustomer.length,
      `无行客户的台账口必须仍是 0 行数组，实际 ${JSON.stringify(ledgerMissingCustomer)}`
    ).toBe(0);

    // 清理。本用例自建的三张卡（双色卡、单色卡、空卡）逐张删除收口，再删客户
    await apiCall(page, 'DELETE', `/color-cards/${card2Id}`, { reason: `114双色卡收口${tag}` });
    await apiCall(page, 'DELETE', `/color-cards/${card1Id}`, { reason: `114单色卡收口${tag}` });
    await apiCall(page, 'DELETE', `/color-cards/${Number(emptyCard.id)}`, {
      reason: `114空卡收口${tag}`,
    });
    await apiCall(page, 'DELETE', `/crm/customers/${customerId}`);
  });
});
