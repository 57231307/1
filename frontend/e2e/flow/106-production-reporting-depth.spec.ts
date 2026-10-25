// 生产报工与工序流转链深度用例
//
// 本文件只覆盖经源码核实真实存在且可跑通的两条链，全部打真实后端真实建单，
//   不用 page.route 一类造响应，不硬编码业务 ID，不用 try catch 吞断言。
//
// 链路一 生产订单报工。写入方是 handlers 层 production_order_handler，
//   推进走 PUT 状态端点，报工走 POST progress 端点。单号纪律是缺陷3修复，
//   建单 DTO 上不存在 order_no 字段，手输 order_no 经 flatten 残差识别被忽略并 warn，
//   真实单号由服务端 DocumentNumberGenerator 在 PO 前缀下事务取号，故断建单响应
//   order_no 以 PO 开头且不等于携带值。状态门是缺陷1修复，progress 端点仅
//   IN_PROGRESS 可上报，非该状态被拒走 business_displayable，出参 400 与
//   BUSINESS_ERROR 码，且不改写任何列。进度三态沿用 RFC 7386 merge patch 语义，
//   键缺席保持、显式 null 清空、有值覆盖，actual_quantity 与 remarks 逐字段读回真值。
//   状态白名单与状态机转换分两族负例，白名单外的串在 handler 校验阶段被拒 400 与
//   VALIDATION_ERROR，白名单内但状态机不允许的跳转在服务层被拒 400 与 BUSINESS_ERROR。
//
// 链路二 流转卡与车间工序流转。写入方是 flow_card_service 及其 ops 子模块，
//   卡号与条码在事务内经统一生成器取号且条码与卡号同体，故断 barcode 逐字符等于
//   card_no 且都以 FC 开头。卡状态机为小写词表 pending scheduled preparing dyeing
//   dyed inspecting completed shipped terminated，工序记录状态机为小写词表
//   pending in_progress completed abnormal rework。开工门要求卡处于 scheduled
//   preparing dyeing dyed inspecting 之一，完工门要求工序处于 in_progress，
//   合格量大于实际量被拒，有异常描述则落 abnormal。回修复制来源工序序号与编码并
//   以 回修 前缀命名，rework_source_id 指向来源记录。开工记录的计划产量快照取自
//   流转卡计划配布数量。完成备布回填实际配布数量但状态保持 preparing 等待进缸，
//   这是写入方有意设计而非缺陷，用例据此断状态不变而实际配布量已写入。
//
// 证据补充 生产订单 COMPLETED 完成路径含事务内库存联动，取默认成品仓用
//   WarehouseEntity 对 is_active 等值查询后接 one，若环境存在多个启用仓该 one 会报错
//   使 COMPLETED 失败，且生产数量为零被拒。该路径依赖环境仓库数据不确定，本文件不纳入
//   断言，只把可稳定复现的报工主链与工序流转主链钉在用例里。
//
// 造数纪律 每支用例先 seed 本 worker 专属的生产订单与流转卡，引用 ensureTestEntities
//   真实产品，列表读回用 order_no 模糊或 production_order_id 等值把范围收敛到本用例数据，
//   避免共享库其它历史行干扰断言基线。
import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  ensureTestEntities,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  getCtx,
  genCode,
  failureCode,
  APP_ERROR_CODES,
} from './helpers';
import { pickListArray } from './ui-helpers';

const ORDERS = '/production/production-orders/orders';
const FLOW_CARDS = '/production/flow-cards';

// 读回响应里的一个键并转成数字，键缺席或非有限值直接抛错，不用默认值掩盖缺键。
function readNumber(obj: Record<string, unknown>, key: string, label: string): number {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label} 响应缺少后端真实键 ${key}，实际键=${Object.keys(obj).join(',')}`);
  }
  const value = Number(obj[key]);
  if (!Number.isFinite(value)) {
    throw new Error(`${label} 键 ${key} 不可解析为有限数字，raw=${String(obj[key])}`);
  }
  return value;
}

// 读回响应里的一个键并要求为非空字符串，缺席或非串直接抛错。
function readString(obj: Record<string, unknown>, key: string, label: string): string {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label} 响应缺少后端真实键 ${key}，实际键=${Object.keys(obj).join(',')}`);
  }
  const value = obj[key];
  if (typeof value !== 'string' || value.length === 0) {
    throw new Error(`${label} 键 ${key} 不是非空字符串，raw=${String(value)}`);
  }
  return value;
}

// 判断某键存在且取值恰为 null，用于三态语义里的显式清空与初值缺席核对。
function readNull(obj: Record<string, unknown>, key: string, label: string): boolean {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label} 响应缺少后端真实键 ${key}，实际键=${Object.keys(obj).join(',')}`);
  }
  return obj[key] === null;
}

// 金额与数量按货币精度比较，容差 0.01，失败信息带期望与实际。
function expectQuantity(actual: number, expected: number, label: string): void {
  expect(Math.abs(actual - expected), `${label}，期望=${expected} 实际=${actual}`).toBeLessThan(
    0.01
  );
}

// 状态逐字符比写入方词表，不做大小写归一，也不放行词表外取值。
function expectStatus(actual: unknown, expected: string, label: string): void {
  expect(String(actual), `${label}，期望逐字符 ${expected} 实际 ${String(actual)}`).toBe(expected);
}

// 字符串逐字符相等断言，失败信息带期望与实际。
function expectStringEq(actual: string, expected: string, label: string): void {
  expect(actual, `${label}，期望 ${expected} 实际 ${actual}`).toBe(expected);
}

// 校验分页信封四键齐备并回显请求页码与每页量，data 为 PaginatedResponse 形态。
function expectEnvelope(data: Record<string, unknown>, listKey: string, page: number, pageSize: number): void {
  for (const key of [listKey, 'total', 'page', 'page_size']) {
    expect(
      Object.prototype.hasOwnProperty.call(data, key),
      `分页响应应含信封键 ${key}，实际键=${Object.keys(data).join(',')}`
    ).toBe(true);
  }
  expect(readNumber(data, 'page', '分页页码'), '回显页码应为请求提交的值').toBe(page);
  expect(readNumber(data, 'page_size', '分页每页量'), '回显每页量应为请求提交的值').toBe(pageSize);
}

// 建一张真实生产订单并返回其读回数据，供流转卡外键锚定与其它用例复用。
async function seedOrder(
  page: import('@playwright/test').Page,
  productId: number,
  remarks: string
): Promise<Record<string, unknown>> {
  const res = await apiCall<Record<string, unknown>>(page, 'POST', ORDERS, {
    product_id: productId,
    planned_quantity: '500',
    remarks,
  });
  expect(res.code, '创建生产订单应返回 200 成功信封').toBe(200);
  return res.data;
}

// 建一张真实流转卡并返回其读回数据，计划配布数量取正以通过写入方正数校验。
async function seedCard(
  page: import('@playwright/test').Page,
  orderId: number,
  orderNo: string,
  plannedWeight: string
): Promise<Record<string, unknown>> {
  const res = await apiCall<Record<string, unknown>>(page, 'POST', FLOW_CARDS, {
    production_order_id: orderId,
    order_no: orderNo,
    planned_fabric_weight: plannedWeight,
  });
  expect(res.code, '创建流转卡应返回 200 成功信封').toBe(200);
  return res.data;
}

// GET 读回生产订单详情，写操作后一律走这一步证明真值已落库。
async function orderDetail(
  page: import('@playwright/test').Page,
  orderId: number
): Promise<Record<string, unknown>> {
  return apiCallRaw<Record<string, unknown>>(page, 'GET', `${ORDERS}/${orderId}`);
}

// GET 读回流转卡详情。
async function cardDetail(
  page: import('@playwright/test').Page,
  cardId: number
): Promise<Record<string, unknown>> {
  return apiCallRaw<Record<string, unknown>>(page, 'GET', `${FLOW_CARDS}/${cardId}`);
}

// GET 读回单条工序记录详情。
async function stepDetail(
  page: import('@playwright/test').Page,
  stepId: number
): Promise<Record<string, unknown>> {
  return apiCallRaw<Record<string, unknown>>(page, 'GET', `${FLOW_CARDS}/steps/${stepId}`);
}

// GET 读回某张流转卡的全部工序记录，出参 data 为裸数组。
async function cardSteps(
  page: import('@playwright/test').Page,
  cardId: number
): Promise<Record<string, unknown>[]> {
  const data = await apiCallRaw<unknown>(page, 'GET', `${FLOW_CARDS}/${cardId}/steps`);
  return pickListArray<Record<string, unknown>>(data, 'bare', '流转卡工序记录列表');
}

test.describe('生产报工与工序流转链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('生产订单报工主链，单号服务端取号且进度三态逐字段读回真值', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    expect(productId, '前置产品缺失').toBeTruthy();

    // 携带一个伪造 order_no，用于证明服务端取号会忽略手输值
    const hackedNo = `HACK-${genCode('PO')}`;
    const tag = genCode('PR');
    const created = await apiCall<Record<string, unknown>>(page, 'POST', ORDERS, {
      product_id: productId,
      planned_quantity: '500',
      order_no: hackedNo,
      remarks: `E2E 报工 ${tag}`,
    });
    expect(created.code, '建单应返回 200 成功信封').toBe(200);
    const orderId = readNumber(created.data, 'id', '生产订单建单响应');
    const orderNo = readString(created.data, 'order_no', '生产订单单号');

    // 建单真值，状态逐字符落词表大写 DRAFT，单号服务端取 PO 前缀且不等于携带值
    expectStatus(created.data.status, 'DRAFT', '建单后订单状态');
    expect(orderNo.startsWith('PO'), `单号应服务端取 PO 前缀，实际 ${orderNo}`).toBe(true);
    expect(orderNo === hackedNo, `手输单号 ${hackedNo} 不得入库，实际 ${orderNo}`).toBe(false);
    expect(readNumber(created.data, 'product_id', '订单产品 ID')).toBe(productId);
    expectQuantity(
      readNumber(created.data, 'planned_quantity', '订单计划数量'),
      500,
      '计划数量应等于提交值'
    );
    // 未提交 priority，读回应为 DB 默认 5，而非塌成 0
    expect(readNumber(created.data, 'priority', '订单优先级默认值'), 'priority 应为 DB 默认 5').toBe(
      5
    );
    expect(
      readNull(created.data, 'actual_quantity', '建单实际数量'),
      '建单后 actual_quantity 应为 null'
    ).toBe(true);
    expectStringEq(readString(created.data, 'remarks', '建单备注'), `E2E 报工 ${tag}`, '建单备注');

    // 推进到排产再到生产中，状态逐字符落大写词表
    const toScheduled = await apiCall<Record<string, unknown>>(page, 'PUT', `${ORDERS}/${orderId}/status`, {
      status: 'SCHEDULED',
    });
    expect(toScheduled.code, '推进到 SCHEDULED 应返回 200').toBe(200);
    expectStatus(toScheduled.data.status, 'SCHEDULED', '排产后订单状态');
    const toInProgress = await apiCall<Record<string, unknown>>(page, 'PUT', `${ORDERS}/${orderId}/status`, {
      status: 'IN_PROGRESS',
    });
    expect(toInProgress.code, '推进到 IN_PROGRESS 应返回 200').toBe(200);
    expectStatus(toInProgress.data.status, 'IN_PROGRESS', '生产中订单状态');
    const inProgDetail = await orderDetail(page, orderId);
    expectStatus(inProgDetail.status, 'IN_PROGRESS', '读回生产中状态');
    expect(
      typeof inProgDetail.actual_start_date === 'string' &&
        String(inProgDetail.actual_start_date).length > 0,
      `进入生产中应写入实际开始日期，实际 ${String(inProgDetail.actual_start_date)}`
    ).toBe(true);

    // 三态之一 有值覆盖，actual_quantity 与 remarks 同时给值
    await apiCall(page, 'POST', `${ORDERS}/${orderId}/progress`, {
      actual_quantity: '250.25',
      remarks: '首批报工',
    });
    const afterWrite = await orderDetail(page, orderId);
    expectQuantity(
      readNumber(afterWrite, 'actual_quantity', '覆盖后实际数量'),
      250.25,
      '进度写入后 actual_quantity 应等于提交值'
    );
    expectStringEq(readString(afterWrite, 'remarks', '覆盖后备注'), '首批报工', '进度写入后备注');
    expectStatus(afterWrite.status, 'IN_PROGRESS', '报工不应改变订单状态');

    // 三态之二 键缺席保持，空体两键都不进请求
    await apiCall(page, 'POST', `${ORDERS}/${orderId}/progress`, {});
    const afterKeep = await orderDetail(page, orderId);
    expectQuantity(
      readNumber(afterKeep, 'actual_quantity', '缺席后实际数量'),
      250.25,
      '键缺席不得清空，actual_quantity 应保持 250.25'
    );
    expectStringEq(readString(afterKeep, 'remarks', '缺席后备注'), '首批报工', '键缺席不得清空备注');

    // 三态之三之一 有值覆盖 actual_quantity，remarks 键缺席保持
    await apiCall(page, 'POST', `${ORDERS}/${orderId}/progress`, { actual_quantity: '300' });
    const afterOver = await orderDetail(page, orderId);
    expectQuantity(
      readNumber(afterOver, 'actual_quantity', '再次覆盖后实际数量'),
      300,
      'actual_quantity 应被覆盖为 300'
    );
    expectStringEq(readString(afterOver, 'remarks', '再次覆盖后备注'), '首批报工', 'remarks 键缺席应保持');

    // 三态之三之二 显式 null 清空 actual_quantity，remarks 键缺席保持
    await apiCall(page, 'POST', `${ORDERS}/${orderId}/progress`, { actual_quantity: null });
    const afterClear = await orderDetail(page, orderId);
    expect(
      readNull(afterClear, 'actual_quantity', '显式 null 后实际数量'),
      '显式 null 应把 actual_quantity 清空为 null'
    ).toBe(true);
    expectStringEq(readString(afterClear, 'remarks', '显式 null 后备注'), '首批报工', 'remarks 键缺席应保持');

    // 显式 null 清空 remarks，读回应为 null
    await apiCall(page, 'POST', `${ORDERS}/${orderId}/progress`, { remarks: null });
    const afterRemarkClear = await orderDetail(page, orderId);
    expect(
      readNull(afterRemarkClear, 'remarks', '清空备注后'),
      '显式 null 应把 remarks 清空为 null'
    ).toBe(true);

    // 详情 LEFT JOIN 富化应带出真实产品名称
    expect(
      typeof afterRemarkClear.product_name === 'string' &&
        String(afterRemarkClear.product_name).length > 0,
      `详情应富化出非空 product_name，实际 ${String(afterRemarkClear.product_name)}`
    ).toBe(true);

    // 列表按本用例真实单号收敛，分页信封四键齐备且能查回该单
    const listData = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `${ORDERS}?order_no=${encodeURIComponent(orderNo)}&page=1&page_size=5`
    );
    expectEnvelope(listData, 'items', 1, 5);
    const rows = pickListArray<Record<string, unknown>>(listData, 'items', '生产订单列表');
    const found = rows.find(r => Number(r.id) === orderId);
    expect(found, `按单号 ${orderNo} 过滤应能查回订单 ${orderId}`).toBeTruthy();
    expectStatus(found!.status, 'IN_PROGRESS', '列表读回订单状态');
  });

  test('报工状态门与状态机两族非法值，被拒后不改写任何列', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    expect(productId, '前置产品缺失').toBeTruthy();

    const tag = genCode('PG');
    const created = await seedOrder(page, productId!, `E2E 报工门 ${tag}`);
    const orderId = readNumber(created, 'id', '生产订单建单响应');

    // DRAFT 状态上报进度应被拒，走 business_displayable，出参 400 与 BUSINESS_ERROR
    const draftProgress = await apiCallExpectFail(page, 'POST', `${ORDERS}/${orderId}/progress`, {
      actual_quantity: '10',
    });
    expect(
      draftProgress.status,
      `非 IN_PROGRESS 上报进度应恰为 400，实际 status=${draftProgress.status} code=${failureCode(draftProgress)}`
    ).toBe(400);
    expect(
      failureCode(draftProgress),
      `进度状态门属业务拒绝族，实际 code=${failureCode(draftProgress)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    // 被拒不得改写，状态仍 DRAFT，实际数量仍 null
    const afterDraftReject = await orderDetail(page, orderId);
    expectStatus(afterDraftReject.status, 'DRAFT', '进度被拒后状态不应改变');
    expect(
      readNull(afterDraftReject, 'actual_quantity', '进度被拒后实际数量'),
      '进度被拒不得写入 actual_quantity'
    ).toBe(true);

    // 白名单外串属 handler 校验族，400 与 VALIDATION_ERROR
    const bogusStatus = await apiCallExpectFail(page, 'PUT', `${ORDERS}/${orderId}/status`, {
      status: 'NOT_A_STATE',
    });
    expect(
      bogusStatus.status,
      `白名单外状态串应恰为 400，实际 status=${bogusStatus.status} code=${failureCode(bogusStatus)}`
    ).toBe(400);
    expect(
      failureCode(bogusStatus),
      `白名单外状态串属请求校验拒绝族，实际 code=${failureCode(bogusStatus)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 白名单内但状态机不允许的跳转属服务层业务族，400 与 BUSINESS_ERROR，DRAFT 到 IN_PROGRESS 非法
    const illegalJump = await apiCallExpectFail(page, 'PUT', `${ORDERS}/${orderId}/status`, {
      status: 'IN_PROGRESS',
    });
    expect(
      illegalJump.status,
      `非法状态跳转应恰为 400，实际 status=${illegalJump.status} code=${failureCode(illegalJump)}`
    ).toBe(400);
    expect(
      failureCode(illegalJump),
      `非法状态跳转属状态机业务拒绝族，实际 code=${failureCode(illegalJump)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expectStatus((await orderDetail(page, orderId)).status, 'DRAFT', '非法跳转后状态不应改变');

    // 合法排产后仍非 IN_PROGRESS，进度门继续拦截
    await apiCall(page, 'PUT', `${ORDERS}/${orderId}/status`, { status: 'SCHEDULED' });
    const schedProgress = await apiCallExpectFail(page, 'POST', `${ORDERS}/${orderId}/progress`, {
      actual_quantity: '10',
    });
    expect(
      failureCode(schedProgress),
      `SCHEDULED 上报进度属业务拒绝族，实际 code=${failureCode(schedProgress)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const afterSchedReject = await orderDetail(page, orderId);
    expectStatus(afterSchedReject.status, 'SCHEDULED', 'SCHEDULED 报工被拒后状态不应改变');
    expect(
      readNull(afterSchedReject, 'actual_quantity', 'SCHEDULED 报工被拒后实际数量'),
      'SCHEDULED 报工被拒不得写入 actual_quantity'
    ).toBe(true);
  });

  test('流转卡工序流转主链，卡号条码同体，工序开始完成回修异常逐字段读回', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    expect(productId, '前置产品缺失').toBeTruthy();

    const tag = genCode('FC');
    const order = await seedOrder(page, productId!, `E2E 流转 ${tag}`);
    const orderId = readNumber(order, 'id', '生产订单建单响应');
    const orderNo = readString(order, 'order_no', '生产订单单号');

    const card = await seedCard(page, orderId, orderNo, '120.5');
    const cardId = readNumber(card, 'id', '流转卡建卡响应');
    const cardNo = readString(card, 'card_no', '流转卡卡号');
    const barcode = readString(card, 'barcode', '流转卡条码');

    // 建卡真值，条码与卡号同体且都以 FC 前缀取号，初态 pending 小写，序号 1，实际配布未回填
    expectStatus(card.status, 'pending', '建卡后卡状态');
    expect(cardNo.startsWith('FC'), `卡号应服务端取 FC 前缀，实际 ${cardNo}`).toBe(true);
    expectStringEq(barcode, cardNo, '条码应与卡号同体');
    expect(readNumber(card, 'current_step_seq', '建卡当前工序序号'), '建卡当前工序序号应为 1').toBe(1);
    expectQuantity(
      readNumber(card, 'planned_fabric_weight', '计划配布数量'),
      120.5,
      '计划配布数量应等于提交值'
    );
    expect(
      readNull(card, 'actual_fabric_weight', '建卡实际配布数量'),
      '建卡后 actual_fabric_weight 应为 null'
    ).toBe(true);
    expect(readNumber(card, 'production_order_id', '流转卡外键'), '卡应锚定真实订单').toBe(orderId);

    // 排缸到 scheduled，写入排缸时间
    const scheduled = await apiCall<Record<string, unknown>>(page, 'POST', `${FLOW_CARDS}/${cardId}/schedule`, {
      dye_lot_no: `E2E-LOT-${tag}`,
    });
    expectStatus(scheduled.data.status, 'scheduled', '排缸后卡状态');
    expect(
      typeof scheduled.data.scheduled_at === 'string' && String(scheduled.data.scheduled_at).length > 0,
      `排缸应写入 scheduled_at，实际 ${String(scheduled.data.scheduled_at)}`
    ).toBe(true);

    // 备布到 preparing
    const preparing = await apiCall<Record<string, unknown>>(page, 'POST', `${FLOW_CARDS}/${cardId}/start-preparing`);
    expectStatus(preparing.data.status, 'preparing', '备布后卡状态');

    // 完成备布回填实际配布量但状态保持 preparing，这是写入方有意设计而非缺陷
    const prepared = await apiCall<Record<string, unknown>>(page, 'POST', `${FLOW_CARDS}/${cardId}/complete-preparing`, {
      actual_fabric_weight: '118',
    });
    expectStatus(prepared.data.status, 'preparing', '完成备布后状态应保持 preparing 等待进缸');
    const cardAfterPrepare = await cardDetail(page, cardId);
    expectQuantity(
      readNumber(cardAfterPrepare, 'actual_fabric_weight', '备布回填后实际配布量'),
      118,
      '完成备布应把实际配布量写入 118'
    );
    expectQuantity(
      readNumber(cardAfterPrepare, 'planned_fabric_weight', '备布后计划配布量'),
      120.5,
      '计划配布量应保持建卡值'
    );

    // 扫码开工序，无路线走 CUSTOM 默认，计划产量快照取卡的计划配布量
    const started = await apiCall<Record<string, unknown>>(page, 'POST', `${FLOW_CARDS}/steps/start`, {
      flow_card_id: cardId,
    });
    const stepId = readNumber(started.data, 'id', '开工工序记录响应');
    expectStatus(started.data.status, 'in_progress', '开工后工序状态');
    expectStringEq(readString(started.data, 'route_code', '默认工序编码'), 'CUSTOM', '无路线开工编码应为 CUSTOM');
    expectStringEq(readString(started.data, 'route_name', '默认工序名称'), '自定义工序', '无路线开工名称应为自定义工序');
    expectStringEq(readString(started.data, 'process_type', '默认工序类型'), 'other', '无路线开工类型应为 other');
    expect(readNumber(started.data, 'step_seq', '开工工序序号'), '开工序号应取卡当前序号 1').toBe(1);
    expectQuantity(
      readNumber(started.data, 'planned_quantity', '开工计划产量'),
      120.5,
      '开工计划产量应快照卡计划配布量'
    );
    expect(readNull(started.data, 'actual_quantity', '开工实际产量'), '开工后 actual_quantity 应为 null').toBe(true);
    expect(readNull(started.data, 'end_at', '开工结束时间'), '开工后 end_at 应为 null').toBe(true);
    expect(readNull(started.data, 'rework_source_id', '开工回修来源'), '普通开工 rework_source_id 应为 null').toBe(
      true
    );
    // 开工把卡的当前工序序号同步为 1
    const cardAfterStart = await cardDetail(page, cardId);
    expect(readNumber(cardAfterStart, 'current_step_seq', '开工后卡当前序号'), '开工应同步卡当前序号为 1').toBe(1);

    // 扫码完工序，合格量不超实际量走正常完成
    const completed = await apiCall<Record<string, unknown>>(page, 'POST', `${FLOW_CARDS}/steps/${stepId}/complete`, {
      actual_quantity: '100',
      qualified_quantity: '95',
    });
    expectStatus(completed.data.status, 'completed', '正常完工后工序状态');
    const stepDone = await stepDetail(page, stepId);
    expectQuantity(readNumber(stepDone, 'actual_quantity', '完工实际产量'), 100, '完工实际产量应等于提交值');
    expectQuantity(readNumber(stepDone, 'qualified_quantity', '完工合格产量'), 95, '完工合格产量应等于提交值');
    expect(
      typeof stepDone.end_at === 'string' && String(stepDone.end_at).length > 0,
      `完工应写入 end_at，实际 ${String(stepDone.end_at)}`
    ).toBe(true);
    const duration = readNumber(stepDone, 'duration_minutes', '完工工时');
    expect(duration >= 0, `完工工时应为非负数，实际 ${duration}`).toBe(true);

    // 从已完成工序发回修，复制来源序号与编码并加回修前缀，来源指向被回修记录
    const rework = await apiCall<Record<string, unknown>>(page, 'POST', `${FLOW_CARDS}/steps/${stepId}/rework`, {
      flow_card_id: cardId,
    });
    const reworkId = readNumber(rework.data, 'id', '回修工序记录响应');
    expectStatus(rework.data.status, 'rework', '回修工序状态');
    expectStringEq(
      readString(rework.data, 'route_name', '回修工序名称'),
      '回修-自定义工序',
      '回修名称应带来源工序前缀'
    );
    expect(readNumber(rework.data, 'step_seq', '回修工序序号'), '回修应复制来源工序序号').toBe(1);
    expectStringEq(readString(rework.data, 'route_code', '回修工序编码'), 'CUSTOM', '回修应复制来源工序编码');
    expect(
      readNumber(rework.data, 'rework_source_id', '回修来源 ID'),
      '回修来源应指向被回修工序'
    ).toBe(stepId);
    expectQuantity(
      readNumber(rework.data, 'planned_quantity', '回修计划产量'),
      120.5,
      '回修计划产量应快照来源工序'
    );
    // 回修记录独立成行，不等于来源记录
    expect(reworkId === stepId, '回修应新建记录而非覆盖来源').toBe(false);

    // 再开一道工序并以异常描述收尾，落 abnormal
    const started2 = await apiCall<Record<string, unknown>>(page, 'POST', `${FLOW_CARDS}/steps/start`, {
      flow_card_id: cardId,
    });
    const step2Id = readNumber(started2.data, 'id', '第二道开工工序响应');
    const abnormal = await apiCall<Record<string, unknown>>(page, 'POST', `${FLOW_CARDS}/steps/${step2Id}/complete`, {
      actual_quantity: '60',
      abnormal_description: '色差异常',
    });
    expectStatus(abnormal.data.status, 'abnormal', '有异常描述完工后应落 abnormal');
    const stepAbn = await stepDetail(page, step2Id);
    expectStringEq(readString(stepAbn, 'abnormal_description', '异常描述'), '色差异常', '异常描述应落库');

    // 该卡工序记录应含完工一道加回修一道加异常一道共三条，且都属本卡
    const steps = await cardSteps(page, cardId);
    expect(steps.length, `流转卡 ${cardId} 工序记录应为三条`).toBe(3);
    for (const s of steps) {
      expect(readNumber(s, 'flow_card_id', '工序归属卡 ID'), '每条工序都应归属本卡').toBe(cardId);
    }
    const reworkRow = steps.find(r => Number(r.id) === reworkId);
    expect(reworkRow, `工序列表应含回修记录 ${reworkId}`).toBeTruthy();
    expectStatus(reworkRow!.status, 'rework', '列表读回回修状态');
  });

  test('工序与备布非法值及词表分页信封纪律，非法请求均无副作用', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    expect(productId, '前置产品缺失').toBeTruthy();

    // 建卡缺必填 production_order_id，属请求解码拒绝族 400 与 VALIDATION_ERROR
    const missingFk = await apiCallExpectFail(page, 'POST', FLOW_CARDS, {
      planned_fabric_weight: '10',
    });
    expect(
      missingFk.status,
      `建卡缺 production_order_id 应恰为 400，实际 status=${missingFk.status} code=${failureCode(missingFk)}`
    ).toBe(400);
    expect(
      failureCode(missingFk),
      `建卡缺必填键属请求解码拒绝族，实际 code=${failureCode(missingFk)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 计划配布为负属服务层业务族 400 与 BUSINESS_ERROR，且该单下不得留下卡行
    const tagA = genCode('FCNEG');
    const orderA = await seedOrder(page, productId!, `E2E 负量 ${tagA}`);
    const orderAId = readNumber(orderA, 'id', '生产订单建单响应');
    const negCard = await apiCallExpectFail(page, 'POST', FLOW_CARDS, {
      production_order_id: orderAId,
      planned_fabric_weight: '-5',
    });
    expect(
      negCard.status,
      `负计划配布应恰为 400，实际 status=${negCard.status} code=${failureCode(negCard)}`
    ).toBe(400);
    expect(
      failureCode(negCard),
      `负计划配布属服务层业务拒绝族，实际 code=${failureCode(negCard)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const cardListAfterNeg = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `${FLOW_CARDS}?production_order_id=${orderAId}&page=1&page_size=10`
    );
    expect(
      pickListArray<Record<string, unknown>>(cardListAfterNeg, 'items', '负量拒绝后按单查卡').length,
      '被拒的建卡请求不应在该订单下留下卡行'
    ).toBe(0);

    // 完成备布实际配布为 0 被拒且不改写卡，实际配布仍 null
    const tagB = genCode('FCZ0');
    const orderB = await seedOrder(page, productId!, `E2E 备布零 ${tagB}`);
    const cardB = await seedCard(page, readNumber(orderB, 'id', '订单 B'), readString(orderB, 'order_no', '单号 B'), '90');
    const cardBId = readNumber(cardB, 'id', '流转卡 B');
    await apiCall(page, 'POST', `${FLOW_CARDS}/${cardBId}/schedule`);
    await apiCall(page, 'POST', `${FLOW_CARDS}/${cardBId}/start-preparing`);
    const prepZero = await apiCallExpectFail(page, 'POST', `${FLOW_CARDS}/${cardBId}/complete-preparing`, {
      actual_fabric_weight: '0',
    });
    expect(
      failureCode(prepZero),
      `实际配布零属正数校验拒绝族，实际 code=${failureCode(prepZero)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const cardBAfterZero = await cardDetail(page, cardBId);
    expectStatus(cardBAfterZero.status, 'preparing', '备布零拒绝后卡状态应保持 preparing');
    expect(
      readNull(cardBAfterZero, 'actual_fabric_weight', '备布零拒绝后实际配布'),
      '被拒的备布回填不得写入实际配布量'
    ).toBe(true);

    // 非 preparing 状态完成备布被拒，卡停在 scheduled，实际配布仍 null
    const tagC = genCode('FCST');
    const orderC = await seedOrder(page, productId!, `E2E 备布门 ${tagC}`);
    const cardC = await seedCard(page, readNumber(orderC, 'id', '订单 C'), readString(orderC, 'order_no', '单号 C'), '80');
    const cardCId = readNumber(cardC, 'id', '流转卡 C');
    await apiCall(page, 'POST', `${FLOW_CARDS}/${cardCId}/schedule`);
    const prepGate = await apiCallExpectFail(page, 'POST', `${FLOW_CARDS}/${cardCId}/complete-preparing`, {
      actual_fabric_weight: '50',
    });
    expect(
      failureCode(prepGate),
      `非 preparing 完成备布属业务拒绝族，实际 code=${failureCode(prepGate)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const cardCAfterGate = await cardDetail(page, cardCId);
    expectStatus(cardCAfterGate.status, 'scheduled', '被拒的备布完成不应改变卡状态');
    expect(
      readNull(cardCAfterGate, 'actual_fabric_weight', '被拒备布完成后的实际配布'),
      '被拒的备布完成不得写入实际配布量'
    ).toBe(true);

    // pending 卡开工序被拒且不落工序记录，卡下工序仍 0 条
    const tagD = genCode('FCPND');
    const orderD = await seedOrder(page, productId!, `E2E 开工门 ${tagD}`);
    const cardD = await seedCard(page, readNumber(orderD, 'id', '订单 D'), readString(orderD, 'order_no', '单号 D'), '70');
    const cardDId = readNumber(cardD, 'id', '流转卡 D');
    const startGate = await apiCallExpectFail(page, 'POST', `${FLOW_CARDS}/steps/start`, {
      flow_card_id: cardDId,
    });
    expect(
      failureCode(startGate),
      `pending 卡开工序属业务拒绝族，实际 code=${failureCode(startGate)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      (await cardSteps(page, cardDId)).length,
      '被拒的开工请求不得留下工序记录'
    ).toBe(0);

    // 完工合格量大于实际量被拒且工序停在 in_progress，实际与合格量未写入
    const tagE = genCode('FQQ');
    const orderE = await seedOrder(page, productId!, `E2E 合格超量 ${tagE}`);
    const cardE = await seedCard(page, readNumber(orderE, 'id', '订单 E'), readString(orderE, 'order_no', '单号 E'), '60');
    const cardEId = readNumber(cardE, 'id', '流转卡 E');
    await apiCall(page, 'POST', `${FLOW_CARDS}/${cardEId}/schedule`);
    await apiCall(page, 'POST', `${FLOW_CARDS}/${cardEId}/start-preparing`);
    const startedE = await apiCall<Record<string, unknown>>(page, 'POST', `${FLOW_CARDS}/steps/start`, {
      flow_card_id: cardEId,
    });
    const stepEId = readNumber(startedE.data, 'id', '流转卡 E 工序');
    const overQualified = await apiCallExpectFail(page, 'POST', `${FLOW_CARDS}/steps/${stepEId}/complete`, {
      actual_quantity: '50',
      qualified_quantity: '60',
    });
    expect(
      failureCode(overQualified),
      `合格量超实际量属业务拒绝族，实际 code=${failureCode(overQualified)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const stepEAfter = await stepDetail(page, stepEId);
    expectStatus(stepEAfter.status, 'in_progress', '被拒的完工请求不应改变工序状态');
    expect(readNull(stepEAfter, 'actual_quantity', '被拒完工后的实际量'), '被拒的完工不得写入实际量').toBe(true);
    expect(readNull(stepEAfter, 'qualified_quantity', '被拒完工后的合格量'), '被拒的完工不得写入合格量').toBe(true);

    // 词表纪律，流转卡状态词表为小写，按大写 PENDING 过滤等值条件命中零行
    const upperFilter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `${FLOW_CARDS}?status=PENDING&page=1&page_size=10`
    );
    expectEnvelope(upperFilter, 'items', 1, 10);
    expect(
      pickListArray<Record<string, unknown>>(upperFilter, 'items', '按大写 PENDING 过滤流转卡').length,
      'PENDING 不是流转卡小写词表取值，命中行数应为 0'
    ).toBe(0);
    expect(readNumber(upperFilter, 'total', '按大写 PENDING 过滤总数'), '大写取值过滤 total 应为 0').toBe(0);

    // 生产订单词表为大写，按词表外串过滤恒零行且分页信封齐备
    const orderFilter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `${ORDERS}?status=not_a_state&page=1&page_size=8`
    );
    expectEnvelope(orderFilter, 'items', 1, 8);
    expect(
      pickListArray<Record<string, unknown>>(orderFilter, 'items', '按词表外串过滤订单').length,
      'not_a_state 不是生产订单词表取值，命中行数应为 0'
    ).toBe(0);
  });
});
