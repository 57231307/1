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
 * 113 色卡主体与色号维护链首个真契约覆盖（改卡口 归档口 标记遗失口 批量建行口 色号删口 三张打印口）
 *
 * 与既有覆盖的分工。既有 23 号与色卡页面只把本域做到列表与详情能打开的浅覆盖，
 * 判定器口径下 PUT 改卡、DELETE 归档、标记遗失、批量建行、色号删行、
 * 发放单与打样单与大货批色单三张打印口均为零覆盖。本 spec 把这些写口钉成真实落库回读链，
 * 并把状态门与校验门拒绝钉到机器码与零漂移两个口径。
 *
 * 契约真相（逐段读后端源，非推测）：
 * 色卡主体（handlers/color_card/crud.rs 与 services/color_card_crud_service.rs）：
 * 建卡初态落 draft，stock_quantity 与 issued_quantity 由服务端写 0，色卡类型只收
 * PANTONE 与 CNCS 与 CUSTOM，越界走 validation_displayable 外显含允许集的原文。
 * 改卡门为卡必须 draft，非 draft 落 CrudError::InvalidState，映射为脱敏 business，
 * 即 HTTP 400 加 BUSINESS_ERROR 且出参文案不含内部细节。校验在 ActiveModel 赋值链中途，
 * 但整笔在事务内且未走到 update，同请求里另一枚合法列必须零提交（半提交即缺陷）。
 * DELETE 是归档语义不是物理删，落库写 archived，请求体是 ArchiveColorCardDto，
 * 缺体会被 axum Json 提取器拒为 400 加 VALIDATION_ERROR 加固定格式原文，故必须带体。
 * 归档与标记遗失同走 validate_color_card_status_transition，终态集为 expired 与 lost 与
 * archived，从终态再流转一律 InvalidState；到 lost 与 archived 对非终态起点一律放行。
 * 色号（handlers/color_card/items.rs 与 services/color_card_item_service.rs）：
 * 单条建行与批量导入的门是可编辑集，取值只有 draft，拒绝映射为 business_displayable，
 * 出参文案固定为"只有草稿态色卡可以维护色号"，两个入口共用同一集合常量。
 * 建行与批量导入与删行都在同事务内维护色卡 total_colors。批量导入出参四键
 * success_count 与 failed_count 与 errors 与 total_colors，errors 条目含 color_code 与 reason，
 * 逐行失败不进事务回滚，同批的成功行照样落库。
 * 色号校验只认服务层，HEX 必须带井号且可解析，RGB 必须 0 到 255，越界走 validation_displayable。
 * CMYK 与 Lab 由服务端从 RGB 派生，提交缺省也要有值。
 * 删口按父卡与行主键双条件定位，父卡不匹配或行已删一律 ItemNotFound 映射 404。
 * 打印（handlers/print_handler.rs 与 services/print_service.rs）：
 * 三张口的路径段是各自单据主键而非色卡主键，bulk-approval 取大货批色行，
 * issue 取色卡发放行，lab-dip 取打样通知单行。装配顺序先取打印数据后取模板布局，
 * 故单据不存在先行 404（脱敏 NOT_FOUND），单据存在而该单据类型无启用且默认模板同样 404，
 * 两因同码，只有"同一单加模板前后由 404 翻成 200"能把它们分辨开。
 * 出参为 docx 流，content-type 为 wordprocessingml，下载名含单据类型与本单主键。
 * 打样建单（services/lab_dip_ops）必填 light_source 与 required_date，交期不得早于今天，
 * 单号由服务端事务内取号，出参整行含 request_no 与 status。
 * 模板建口（handlers/print_handler.rs::create_print_template）doc_type 必须属受支持集，
 * 越界 fail-visible 400 外显所提交值；is_default 为真时同事务内先清同 module 其余行的默认位，
 * 故本用例用私有 module 名，只影响自己，不动任何既有模块的默认位。
 *
 * 假绿防线。每条写操作都紧跟一次对象读回逐列对表，色卡主体列与色号行集一起看；
 * 门拒一律钉 HTTP 400 加机器码族，可外显族再钉文案原文，被拒后一律回读断零漂移；
 * 批量导入的失败行既断 errors 点名又断列表里真查不到它，成功行断真查得到；
 * 三张打印口的不存在分支先用各自对象口证明该主键无行，再断打印口 404 加 NOT_FOUND 码，
 * 不接受只判"非 2xx"；正向分支以无模板 404 到有模板 200 且体首两字节为 zip 魔数双证。
 * 全程不用 ?? 或 || 兜底，后端必出的键缺失即形漂判红。
 */

type Row = Record<string, unknown>;

/** 色卡主体列（handlers/color_card 的 ColorCardListItem 与 ColorCardDetail 共有面） */
interface CardRow {
  id: number;
  card_no: string;
  card_name: string;
  card_type: string;
  season: string | null;
  brand: string | null;
  total_colors: number;
  status: string;
  stock_quantity: number;
  issued_quantity: number;
}

/** 色卡详情（比列表多 description 与色号行集） */
interface CardDetail extends CardRow {
  description: string | null;
  items: ColorItemRow[];
}

/** 色号行出参（handlers/color_card/helpers.rs 的 ColorItemInfo） */
interface ColorItemRow {
  id: number;
  color_code: string;
  color_name: string;
  rgb_r: number;
  rgb_g: number;
  rgb_b: number;
  cmyk_c: string | number | null;
  lab_l: string | number | null;
  hex_value: string;
  pantone_code: string | null;
  sequence: number;
}

/** 批量导入出参四键 */
interface BatchImportResp {
  success_count: number;
  failed_count: number;
  errors: Array<{ color_code: string; reason: string }>;
  total_colors: number;
}

/** 色号列表口与发放列表口共用的分页信封 */
type Paged<T> = { items: T[]; total: number; page: number; page_size: number };

/** 打印模板建行出参 */
interface PrintTemplateRow {
  id: number;
  template_code: string;
  doc_type: string;
  module: string;
  status: string;
  is_default: boolean;
}

/** 打样通知单建行出参（整行 Model 的可见列） */
interface LabDipRequestRow {
  id: number;
  request_no: string;
  light_source: string;
  required_date: string;
  status: string;
}

/**
 * 必然不存在于任何单据表的探测主键。
 * 入参：无。传给谁：三张打印口与其对应对象口的存在性证明。
 * 存什么：一个落在 i32 正向内且远高于 CI 库真实自增值的常量（打印 handler 会把 i64 路径段
 * 下转型为 i32 再查库，取超出 i32 的值会让下转型绕回成一个真实存在的 id，探测即失真）。
 * 存哪里：不落盘，只在断言前置证明里使用，且每次都先用对象口复验它确实无行。
 */
const ABSENT_PROBE_ID = 2_000_000_000;

const ymd = (daysFromToday: number): string =>
  new Date(Date.now() + daysFromToday * 86_400_000).toISOString().slice(0, 10);
const toNum = (v: unknown): number => Number(String(v));
const str = (v: unknown): string => String(v);
const printUrl = (path: string): string => `${API_BASE}${API_PREFIX}${path}`;

/**
 * 造一张草稿色卡并断初态与库存两列的真实落库值。
 * 入参：page 与本用例唯一码。传给谁：各用例自建自持的前置卡，不与并发车道共用行。
 * 存什么：提交 card_no 与 card_name 与 card_type，回读 status 与 stock_quantity 两枚服务端列。
 * 存哪里：color_cards 一行，status 由服务端写 draft，库存两列由服务端写 0。
 */
async function seedDraftCard(page: Page, tag: string): Promise<CardRow> {
  const created = await apiCallRaw<CardRow>(page, 'POST', '/color-cards', {
    card_no: `CC113${tag}`,
    card_name: `113色卡${tag}`,
    card_type: 'PANTONE',
  });
  expect(
    Number.isInteger(Number(created.id)) && Number(created.id) > 0,
    `建卡应返回真实主键，实际 ${JSON.stringify(created)}`
  ).toBe(true);
  expect(str(created.card_no), '建卡回显 card_no 必须等于提交值').toBe(`CC113${tag}`);
  expect(str(created.status), `建卡初态必须落 draft，实际 ${str(created.status)}`).toBe('draft');
  expect(
    toNum(created.stock_quantity),
    `建卡库存必须由服务端写 0，实际 ${str(created.stock_quantity)}`
  ).toBe(0);
  expect(
    toNum(created.issued_quantity),
    `建卡已发放数必须由服务端写 0，实际 ${str(created.issued_quantity)}`
  ).toBe(0);
  return created;
}

/**
 * 读色号列表口并取一行的完整原始列表。
 * 入参：page 与父卡主键。传给谁：删行与批量导入后的行集真读。
 * 存什么：不落盘，返回后端 items 数组。
 * 存哪里：不存。信封结构漂移（items 非数组）当场判红，不做 ?? [] 兜底。
 */
async function readItems(page: Page, cardId: number): Promise<ColorItemRow[]> {
  const list = await apiCallRaw<Paged<ColorItemRow>>(
    page,
    'GET',
    `/color-cards/${cardId}/items?page=1&page_size=100`
  );
  expect(Array.isArray(list.items), `色号列表口必须回 items 数组，实际 ${JSON.stringify(list)}`).toBe(
    true
  );
  return list.items;
}

/** 构造一枚合法色号提交体（服务层校验只认 HEX 可解析与 RGB 在界内） */
const mkItem = (
  code: string,
  seq: number,
  hex = '#1E90FF',
  r = 30,
  g = 144,
  b = 255
): Record<string, unknown> => ({
  color_code: code,
  color_name: `113色号${code}`,
  rgb_r: r,
  rgb_g: g,
  rgb_b: b,
  hex_value: hex,
  sequence: seq,
});

test.describe.serial('113 色卡主体与色号维护链', () => {
  test.beforeEach(async ({ page }) => {
    // worker 会话由 diagnose-fixture 的 worker 作用域 storageState 注入，本链全程走 apiCall
    // 不导航页面，故不重复登录；只证明会话真实可用（写口依赖它的 CSRF 与凭据前置）
    const me = await apiCallRaw<Row>(page, 'GET', '/auth/me');
    expect(
      Number.isInteger(Number(me.id)) && Number(me.id) > 0,
      `/auth/me 必须回真实用户 id，实际 ${JSON.stringify(me)}`
    ).toBe(true);
  });

  test('T1 改卡回读 非法类型被拒且同请求合法列零提交 批量建行含单行失败 删行总数回退 重复删被拒', async ({
    page,
  }) => {
    const tag = genCode('CC113A');
    const card = await seedDraftCard(page, tag);
    const cardId = Number(card.id);
    expect(toNum(card.total_colors), '新卡色号数必须为 0').toBe(0);

    // 1) 改卡口（A 档）改名与季节与品牌与描述，对象口逐列回读
    const newName = `113色卡改${tag}`;
    await apiCall(page, 'PUT', `/color-cards/${cardId}`, {
      card_name: newName,
      season: 'SS',
      brand: `BRAND${tag}`,
      description: `113描述${tag}`,
    });
    const afterPut = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${cardId}`);
    expect(str(afterPut.card_name), '改卡后 card_name 必须等于提交原文').toBe(newName);
    expect(str(afterPut.season), '改卡后 season 必须等于提交原文').toBe('SS');
    expect(str(afterPut.brand), '改卡后 brand 必须等于提交原文').toBe(`BRAND${tag}`);
    expect(str(afterPut.description), '改卡后 description 必须等于提交原文').toBe(`113描述${tag}`);
    expect(str(afterPut.status), '只改文本列不许动状态').toBe('draft');
    expect(toNum(afterPut.total_colors), '只改文本列不许动色号数').toBe(0);

    // 2) 非法色卡类型必须 400 加校验机器码与外显原文，且同请求里的合法列也不许半提交
    const badType = await apiCallExpectFail(page, 'PUT', `/color-cards/${cardId}`, {
      card_type: 'PANTONE_X',
      season: 'AW',
    });
    expect(badType.status, `非法色卡类型必须 400，实际 ${badType.status}`).toBe(400);
    expect(
      failureCode(badType),
      `非法色卡类型必须落校验机器码，实际 ${JSON.stringify(badType.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      str(badType.message).includes('PANTONE / CNCS / CUSTOM'),
      `校验拒绝须外显允许集原文（validation_displayable 通道），实际 ${str(badType.message)}`
    ).toBe(true);
    const afterBadType = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${cardId}`);
    expect(str(afterBadType.card_type), '被拒的改卡不许动色卡类型').toBe('PANTONE');
    expect(str(afterBadType.season), '被拒请求内的合法列同样不许落库').toBe('SS');
    expect(str(afterBadType.card_name), '被拒的改卡不许动名称').toBe(newName);

    // 3) 批量导入三成功一失败（A 档批量建行口），出参四键对表并真读行集
    const okCodes = [`OKA${tag}`, `OKB${tag}`, `OKC${tag}`];
    const badCode = `BAD${tag}`;
    const batch = await apiCallRaw<BatchImportResp>(
      page,
      'POST',
      `/color-cards/${cardId}/items/batch`,
      {
        items: [
          mkItem(okCodes[0], 1),
          mkItem(okCodes[1], 2, '#FF0000', 255, 0, 0),
          // HEX 不可解析的一行进 errors，且不许回滚同批其余行
          mkItem(badCode, 3, 'notahex', 10, 10, 10),
          mkItem(okCodes[2], 4, '#00FF00', 0, 255, 0),
        ],
      }
    );
    expect(
      toNum(batch.success_count),
      `批量导入成功数必须为 3，实际 ${JSON.stringify(batch)}`
    ).toBe(3);
    expect(toNum(batch.failed_count), '批量导入失败数必须为 1').toBe(1);
    expect(Array.isArray(batch.errors), '批量导入 errors 必须回数组').toBe(true);
    expect(toNum(batch.errors.length), 'errors 长度必须等于 failed_count').toBe(1);
    expect(str(batch.errors[0].color_code), 'errors 必须点名失败行的色号编码').toBe(badCode);
    expect(
      str(batch.errors[0].reason).length > 0,
      `失败原因必须非空，实际 ${JSON.stringify(batch.errors[0])}`
    ).toBe(true);
    expect(toNum(batch.total_colors), '批量导入后 total_colors 必须等于成功数 3').toBe(3);

    // 行集真读：成功三行必须在列，失败一行必须不在列（防"计数对但没落库"的半截实现）
    const itemsAfterBatch = await readItems(page, cardId);
    for (const code of okCodes) {
      expect(
        itemsAfterBatch.find(i => str(i.color_code) === code),
        `批量导入后列表应含 ${code}，实际 ${JSON.stringify(itemsAfterBatch)}`
      ).toBeDefined();
    }
    expect(
      itemsAfterBatch.find(i => str(i.color_code) === badCode),
      `失败行 ${badCode} 不许落库`
    ).toBeUndefined();
    expect(itemsAfterBatch.length, '列表行数必须等于成功数').toBe(3);

    // 服务端派生列必须落值（提交体里没有 CMYK 与 Lab）
    const first = itemsAfterBatch.find(i => str(i.color_code) === okCodes[0]);
    expect(first, '回读首行失败').toBeDefined();
    expect(
      first!.cmyk_c !== null && first!.cmyk_c !== undefined,
      `CMYK 必须由服务端从 RGB 派生非空，实际 ${String(first!.cmyk_c)}`
    ).toBe(true);
    expect(
      first!.lab_l !== null && first!.lab_l !== undefined,
      `Lab 明度必须由服务端派生非空，实际 ${String(first!.lab_l)}`
    ).toBe(true);
    expect(str(first!.hex_value), '色号 hex 必须回读提交原文').toBe('#1E90FF');
    expect(toNum(first!.rgb_r), '色号 rgb_r 必须回读提交原文').toBe(30);
    expect(toNum(first!.sequence), '色号 sequence 必须回读提交原文').toBe(1);

    // 卡口 total_colors 与列表口 total 与行数组长度三口互证
    const cardAfterBatch = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${cardId}`);
    expect(toNum(cardAfterBatch.total_colors), '卡口 total_colors 必须等于批量成功数').toBe(3);
    expect(
      cardAfterBatch.items.length,
      `详情口的 items 必须与列表口同数，实际 ${cardAfterBatch.items.length}`
    ).toBe(3);

    // 4) 色号删口（A 档）删中间一行，行数与总数两口径回读
    const victim = cardAfterBatch.items.find(i => str(i.color_code) === okCodes[1]);
    expect(victim, '删除前回读目标行失败').toBeDefined();
    const survivorCode = okCodes[2];
    await apiCall(page, 'DELETE', `/color-cards/${cardId}/items/${Number(victim!.id)}`);

    const itemsAfterDelete = await readItems(page, cardId);
    expect(itemsAfterDelete.length, '删行后列表必须剩 2 行').toBe(2);
    expect(
      itemsAfterDelete.find(i => str(i.color_code) === okCodes[1]),
      `被删行 ${okCodes[1]} 不许仍在列`
    ).toBeUndefined();
    expect(
      itemsAfterDelete.find(i => str(i.color_code) === survivorCode),
      `幸存者行 ${survivorCode} 必须仍在列（防删错行）`
    ).toBeDefined();
    const cardAfterDelete = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${cardId}`);
    expect(
      toNum(cardAfterDelete.total_colors),
      `删行后卡口 total_colors 必须由 3 退到 2，实际 ${str(cardAfterDelete.total_colors)}`
    ).toBe(2);
    expect(str(cardAfterDelete.status), '删色号不许动色卡状态').toBe('draft');

    // 5) 重复删同一行必须 404（ItemNotFound 族），行数与总数零漂移
    const reDelete = await apiCallExpectFail(
      page,
      'DELETE',
      `/color-cards/${cardId}/items/${Number(victim!.id)}`
    );
    expect(reDelete.status, `重复删色号必须 404，实际 ${reDelete.status}`).toBe(404);
    expect(
      failureCode(reDelete),
      `重复删色号必须落未找到机器码，实际 ${JSON.stringify(reDelete.code)}`
    ).toBe(APP_ERROR_CODES.NOT_FOUND);
    const itemsAfterReDelete = await readItems(page, cardId);
    expect(itemsAfterReDelete.length, '被拒的重复删不许再少行').toBe(2);
    const cardAfterReDelete = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${cardId}`);
    expect(toNum(cardAfterReDelete.total_colors), '被拒的重复删不许再动总数').toBe(2);

    // 6) RGB 越界的单行建行必须 400 加外显原文，且行数零增长
    const rgbOutCode = `RGBOVR${tag}`;
    const rgbOut = await apiCallExpectFail(page, 'POST', `/color-cards/${cardId}/items`, {
      ...mkItem(rgbOutCode, 9, '#FF0000', 300, 0, 0),
    });
    expect(rgbOut.status, `RGB 越界必须 400，实际 ${rgbOut.status}`).toBe(400);
    expect(failureCode(rgbOut)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      str(rgbOut.message).includes('RGB'),
      `RGB 越界须外显服务层原文，实际 ${str(rgbOut.message)}`
    ).toBe(true);
    const itemsAfterRgbOut = await readItems(page, cardId);
    expect(
      itemsAfterRgbOut.find(i => str(i.color_code) === rgbOutCode),
      '被拒的建行不许落库'
    ).toBeUndefined();
    expect(itemsAfterRgbOut.length, '被拒的建行不许让行数增长').toBe(2);
  });

  test('T2 标记遗失 终态再流转被拒 非草稿维护色号两入口同拒 缺体改卡被拒 归档与重复归档', async ({
    page,
  }) => {
    const tag = genCode('CC113B');
    const card = await seedDraftCard(page, tag);
    const cardId = Number(card.id);

    // 前置一行色号，供"非草稿态维护色号"负例证明总数与行集都真的没动
    const preCode = `PRE${tag}`;
    await apiCall(page, 'POST', `/color-cards/${cardId}/items`, { ...mkItem(preCode, 1) });
    const cardWithItem = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${cardId}`);
    expect(toNum(cardWithItem.total_colors), '建行后 total_colors 必须为 1').toBe(1);

    // 1) 标记遗失（A 档）draft 到 lost 为合法流转，对象口回读
    await apiCall(page, 'POST', `/color-cards/${cardId}/mark-lost`);
    const afterLost = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${cardId}`);
    expect(str(afterLost.status), `标记遗失后必须落 lost，实际 ${str(afterLost.status)}`).toBe(
      'lost'
    );
    expect(toNum(afterLost.total_colors), '标记遗失不许动色号数').toBe(1);

    // 2) 已是终态再标记遗失必须 400 加 BUSINESS_ERROR（脱敏族不读文案），状态零漂移
    const reLost = await apiCallExpectFail(page, 'POST', `/color-cards/${cardId}/mark-lost`);
    expect(reLost.status, `终态色卡重复标记遗失必须 400，实际 ${reLost.status}`).toBe(400);
    expect(
      failureCode(reLost),
      `终态重复标记遗失必须落业务机器码，实际 ${JSON.stringify(reLost.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const afterReLost = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${cardId}`);
    expect(str(afterReLost.status), '被拒的重复标记不许改状态').toBe('lost');

    // 3) 非草稿态改卡必须被主体门拒（同一门在色号门之前，两族文案不同不许混）
    const putOnLost = await apiCallExpectFail(page, 'PUT', `/color-cards/${cardId}`, {
      card_name: `113遗失后改名${tag}`,
    });
    expect(putOnLost.status, `lost 态改卡必须 400，实际 ${putOnLost.status}`).toBe(400);
    expect(failureCode(putOnLost)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const cardAfterPutReject = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${cardId}`);
    expect(
      str(cardAfterPutReject.card_name),
      `被拒的改卡不许动名称，实际 ${str(cardAfterPutReject.card_name)}`
    ).toBe(`113色卡${tag}`);

    // 4) 非草稿态单条建行必须被色号门拒且外显公开规则原文，行集与总数零漂移
    const lateCode = `LATE${tag}`;
    const maintainOnLost = await apiCallExpectFail(page, 'POST', `/color-cards/${cardId}/items`, {
      ...mkItem(lateCode, 2, '#0A0B0C', 10, 11, 12),
    });
    expect(maintainOnLost.status, `非草稿态建行必须 400，实际 ${maintainOnLost.status}`).toBe(400);
    expect(failureCode(maintainOnLost)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      str(maintainOnLost.message),
      `色号维护门必须外显公开规则文案（business_displayable 族），实际 ${str(maintainOnLost.message)}`
    ).toBe('只有草稿态色卡可以维护色号');
    const itemsOnLost = await readItems(page, cardId);
    expect(
      itemsOnLost.find(i => str(i.color_code) === lateCode),
      '被拒的建行不许落库'
    ).toBeUndefined();
    expect(itemsOnLost.length, '被拒的维护后色号行数必须仍是 1').toBe(1);

    // 5) 批量导入同受该门约束（两个入口共用同一集合常量，只堵一个即漏）
    const batchCode = `BATCH${tag}`;
    const batchOnLost = await apiCallExpectFail(
      page,
      'POST',
      `/color-cards/${cardId}/items/batch`,
      { items: [mkItem(batchCode, 3, '#0D0E0F', 13, 14, 15)] }
    );
    expect(batchOnLost.status, `非草稿态批量导入必须 400，实际 ${batchOnLost.status}`).toBe(400);
    expect(failureCode(batchOnLost)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(str(batchOnLost.message)).toBe('只有草稿态色卡可以维护色号');
    const cardAfterBatchReject = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${cardId}`);
    expect(toNum(cardAfterBatchReject.total_colors), '被拒的批量导入不许动总数').toBe(1);
    const itemsAfterBatchReject = await readItems(page, cardId);
    expect(
      itemsAfterBatchReject.find(i => str(i.color_code) === batchCode),
      '被拒的批量导入不许落任何行'
    ).toBeUndefined();

    // 6) 终态归档必须被拒（终态集 expired 与 lost 与 archived 不再流转）
    const archiveOnLost = await apiCallExpectFail(page, 'DELETE', `/color-cards/${cardId}`, {
      reason: `113终态归档探针${tag}`,
    });
    expect(archiveOnLost.status, `lost 态归档必须 400，实际 ${archiveOnLost.status}`).toBe(400);
    expect(failureCode(archiveOnLost)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const cardAfterArchiveReject = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${cardId}`);
    expect(str(cardAfterArchiveReject.status), '被拒的归档不许改状态').toBe('lost');

    // 7) 缺体归档必须被请求解码门拒（400 加校验码加固定格式原文），状态零漂移
    const tagNoBody = genCode('CC113N');
    const cardNoBody = await seedDraftCard(page, tagNoBody);
    const noBodyArchive = await apiCallExpectFail(
      page,
      'DELETE',
      `/color-cards/${Number(cardNoBody.id)}`
    );
    expect(
      noBodyArchive.status,
      `归档缺请求体必须 400（handler 收 Json<ArchiveColorCardDto>），实际 ${noBodyArchive.status}`
    ).toBe(400);
    expect(failureCode(noBodyArchive)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const cardNoBodyAfter = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${Number(cardNoBody.id)}`);
    expect(str(cardNoBodyAfter.status), '被拒的缺体归档不许把卡改成 archived').toBe('draft');

    // 8) 另一张草稿卡走正常归档（A 档 DELETE 口），带体提交，对象口回读 archived
    const tag2 = genCode('CC113C');
    const card2 = await seedDraftCard(page, tag2);
    const card2Id = Number(card2.id);
    await apiCall(page, 'DELETE', `/color-cards/${card2Id}`, { reason: `113归档原因${tag2}` });
    const afterArchive = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${card2Id}`);
    expect(str(afterArchive.status), `归档后必须落 archived，实际 ${str(afterArchive.status)}`).toBe(
      'archived'
    );

    // 9) 归档后改卡必须被主体状态门拒，且提交列零漂移
    const putOnArchived = await apiCallExpectFail(page, 'PUT', `/color-cards/${card2Id}`, {
      card_name: `113归档后改名${tag2}`,
    });
    expect(putOnArchived.status, `archived 态改卡必须 400，实际 ${putOnArchived.status}`).toBe(400);
    expect(failureCode(putOnArchived)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const card2AfterPutReject = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${card2Id}`);
    expect(str(card2AfterPutReject.card_name), '被拒的改卡不许动名称').toBe(`113色卡${tag2}`);
    expect(str(card2AfterPutReject.status), '被拒的改卡不许动状态').toBe('archived');

    // 10) 重复归档必须被终态门拒
    const reArchive = await apiCallExpectFail(page, 'DELETE', `/color-cards/${card2Id}`, {
      reason: `113重复归档${tag2}`,
    });
    expect(reArchive.status, `重复归档必须 400，实际 ${reArchive.status}`).toBe(400);
    expect(failureCode(reArchive)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const card2AfterReArchive = await apiCallRaw<CardDetail>(page, 'GET', `/color-cards/${card2Id}`);
    expect(str(card2AfterReArchive.status), '被拒的重复归档不许改状态').toBe('archived');

    // 11) 不存在主键的三个口一律 404（不是 5xx，也不是把缺行当空对象回）
    const missingGet = await apiCallExpectFail(page, 'GET', `/color-cards/${ABSENT_PROBE_ID}`);
    expect(missingGet.status, `不存在色卡详情必须 404，实际 ${missingGet.status}`).toBe(404);
    expect(failureCode(missingGet)).toBe(APP_ERROR_CODES.NOT_FOUND);
    const missingItems = await apiCallExpectFail(
      page,
      'GET',
      `/color-cards/${ABSENT_PROBE_ID}/items?page=1&page_size=10`
    );
    expect(missingItems.status, `不存在色卡的色号列表必须 404，实际 ${missingItems.status}`).toBe(
      404
    );
    expect(failureCode(missingItems)).toBe(APP_ERROR_CODES.NOT_FOUND);
    const missingMarkLost = await apiCallExpectFail(
      page,
      'POST',
      `/color-cards/${ABSENT_PROBE_ID}/mark-lost`
    );
    expect(missingMarkLost.status, `不存在色卡标记遗失必须 404，实际 ${missingMarkLost.status}`).toBe(
      404
    );
    expect(failureCode(missingMarkLost)).toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('T3 三张打印口无单据时同码 404 非法单据类型模板被拒 打样单到默认模板翻成 docx', async ({
    page,
  }) => {
    const tag = genCode('CC113P');

    // 1) 大货批色打印口。先经批色对象口证明该主键无行，再断打印口 404 加未找到机器码。
    const bulkProbe = await apiCallExpectFail(page, 'GET', `/bulk-color-approvals/${ABSENT_PROBE_ID}`);
    expect(
      bulkProbe.status,
      `探测主键在批色表必须无行（有行说明探测值失效），实际 ${bulkProbe.status}`
    ).toBe(404);
    const bulkPrint = await page.request.get(
      printUrl(`/color-cards/${ABSENT_PROBE_ID}/bulk-approval/print`)
    );
    expect(
      bulkPrint.status(),
      `批色单打印口对无行单据必须 404，实际 ${bulkPrint.status()}`
    ).toBe(404);
    const bulkPrintBody = await bulkPrint.json();
    expect(
      failureCode(bulkPrintBody),
      `打印拒绝必须走统一信封带字符串机器码，实际 ${JSON.stringify(bulkPrintBody)}`
    ).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 2) 发放单打印口。库存登记入口（PUT /color-cards/:id 的 stock_quantity）已经存在，
    //    发放表可能有同分片用例写入的历史行，故不按全表行数断言，
    //    只对不存在的主键断 404，不假装跑通正向。
    const issueProbe = await apiCallExpectFail(page, 'GET', `/color-cards/issues/${ABSENT_PROBE_ID}`);
    expect(issueProbe.status, `探测主键在发放表必须无行，实际 ${issueProbe.status}`).toBe(404);
    const issuePrint = await page.request.get(printUrl(`/color-cards/${ABSENT_PROBE_ID}/issue/print`));
    expect(
      issuePrint.status(),
      `发放单打印口在无发放记录载体上必须 404，实际 ${issuePrint.status()}`
    ).toBe(404);
    expect(failureCode(await issuePrint.json())).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 3) 模板建口对非法单据类型必须 400 加外显所提交值（fail-visible，不落库）
    const badDocType = await apiCallExpectFail(page, 'POST', '/print-templates', {
      template_name: `113非法单据模板${tag}`,
      template_code: genCode('TPL113BAD'),
      doc_type: 'not_a_doc_type',
      module: `cc113_${tag}`,
      status: 'active',
      is_default: false,
    });
    expect(badDocType.status, `非法单据类型建模板必须 400，实际 ${badDocType.status}`).toBe(400);
    expect(failureCode(badDocType)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      str(badDocType.message).includes('not_a_doc_type'),
      `非法单据类型须外显所提交值便于定位，实际 ${str(badDocType.message)}`
    ).toBe(true);

    // 4) 打样打印口的两因同码分辨。先建真实打样通知单（交期不早于今天）
    const labReq = await apiCallRaw<LabDipRequestRow>(page, 'POST', '/production/lab-dip/requests', {
      light_source: 'D65',
      required_date: ymd(7),
      customer_color_no: `LDN${tag}`,
      sample_versions: 4,
      remarks: `113打印链打样${tag}`,
    });
    const reqId = Number(labReq.id);
    expect(
      Number.isInteger(reqId) && reqId > 0,
      `打样单建单必须回真实主键，实际 ${JSON.stringify(labReq)}`
    ).toBe(true);
    expect(
      str(labReq.request_no).length > 0,
      `打样单号必须由服务端取号非空，实际 ${str(labReq.request_no)}`
    ).toBe(true);
    expect(str(labReq.light_source), '打样单光源必须回读提交原文').toBe('D65');
    expect(str(labReq.required_date), '打样单交期必须回读提交原文').toBe(ymd(7));
    expect(
      str(labReq.status),
      `打样单初态必须由服务端落 pending，实际 ${str(labReq.status)}`
    ).toBe('pending');
    const reqReadback = await apiCallRaw<LabDipRequestRow>(page, 'GET', `/production/lab-dip/requests/${reqId}`);
    expect(Number(reqReadback.id), '打样单对象口必须命中本单').toBe(reqId);

    // 单据存在但该单据类型此刻没有启用且默认的模板，装配先取数据后取模板，故仍 404
    const beforeTpl = await page.request.get(printUrl(`/color-cards/${reqId}/lab-dip/print`));
    expect(
      beforeTpl.status(),
      `单据存在而模板缺失时打印必须 404，实际 ${beforeTpl.status()}`
    ).toBe(404);

    // 建本用例私有 module 的启用默认模板（is_default 只清同 module 的默认位，不碰既有模块）
    const tplCode = genCode('TPL113LD');
    const tpl = await apiCallRaw<PrintTemplateRow>(page, 'POST', '/print-templates', {
      template_name: `113打样打印模板${tag}`,
      template_code: tplCode,
      doc_type: 'lab_dip_request',
      module: `cc113_${tag}`,
      type: 'report',
      paper_size: 'A4',
      orientation: 'portrait',
      copies_default: 1,
      is_default: true,
      status: 'active',
    });
    const tplId = Number(tpl.id);
    expect(
      Number.isInteger(tplId) && tplId > 0,
      `模板建行必须回主键，实际 ${JSON.stringify(tpl)}`
    ).toBe(true);
    expect(str(tpl.doc_type), '模板回显 doc_type 必须等于提交值').toBe('lab_dip_request');
    expect(str(tpl.status), '模板回显 status 必须为 active').toBe('active');
    expect(tpl.is_default === true, '模板回显默认位必须为真').toBe(true);

    // 同一单据同一路径，加了模板由 404 翻成 200，证明上一跳的 404 来自模板装配分支
    const afterTpl = await page.request.get(printUrl(`/color-cards/${reqId}/lab-dip/print`));
    expect(afterTpl.status(), `带默认模板后打印必须 200，实际 ${afterTpl.status()}`).toBe(200);
    const ctype = afterTpl.headers()['content-type'] ?? '';
    expect(
      ctype.includes('wordprocessingml'),
      `打印出参 content-type 必须是 docx 流，实际 ${ctype}`
    ).toBe(true);
    const bytes = await afterTpl.body();
    expect(bytes.byteLength, 'docx 体必须非空').toBeGreaterThan(0);
    // docx 是 zip 容器，头两字节必须是 50 4B（PK），空文件或 JSON 错误体都到不了这里
    expect(bytes[0], `docx 首字节必须是 0x50，实际 ${bytes[0]}`).toBe(0x50);
    expect(bytes[1], `docx 次字节必须是 0x4b，实际 ${bytes[1]}`).toBe(0x4b);
    const disposition = afterTpl.headers()['content-disposition'] ?? '';
    expect(
      disposition.includes(`lab_dip_request_${reqId}.docx`),
      `下载名必须含单据类型与本单主键，实际 ${disposition}`
    ).toBe(true);

    // 5) 收尾。先停模板再删打样单，避免留下启用默认行影响后续打印判定
    await apiCall(page, 'PUT', `/print-templates/${tplId}`, { status: 'inactive' });
    const tplOff = await apiCallRaw<PrintTemplateRow>(page, 'GET', `/print-templates/${tplId}`);
    expect(str(tplOff.status), '模板停用后必须回读 inactive').toBe('inactive');

    // 模板停用后同一路径必须退回 404（启用位真实参与解析，不是建了就一直可用）
    const afterOff = await page.request.get(printUrl(`/color-cards/${reqId}/lab-dip/print`));
    expect(
      afterOff.status(),
      `模板停用后打印必须退回 404（启用位真实生效），实际 ${afterOff.status()}`
    ).toBe(404);

    await apiCall(page, 'DELETE', `/production/lab-dip/requests/${reqId}`);
    const gone = await apiCallExpectFail(page, 'GET', `/production/lab-dip/requests/${reqId}`);
    expect(gone.status, `打样单删除后按 id 回读必须 404，实际 ${gone.status}`).toBe(404);
    await apiCall(page, 'DELETE', `/print-templates/${tplId}`);
    const tplGone = await apiCallExpectFail(page, 'GET', `/print-templates/${tplId}`);
    expect(tplGone.status, `模板删除后按 id 回读必须 404，实际 ${tplGone.status}`).toBe(404);
  });
});
