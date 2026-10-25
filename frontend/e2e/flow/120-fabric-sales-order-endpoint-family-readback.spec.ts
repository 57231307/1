import { test, expect, type Page } from '../diagnose-fixture';
import {
  APP_ERROR_CODES,
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
  ensureTestEntities,
  failureCode,
  genCode,
  getCtx,
  loginViaUI,
  tryCleanup,
} from './helpers';

/**
 * 120 面料行业版销售订单端点族真值链（建单逐列回读 明细跨口互证 四道校验门 审批与拒绝状态门 主表更新列映射 双口分页互校 删除门）
 *
 * 与既有覆盖的分工。面料订单七枚端点（列表 详情 建单 改单 删单 审核 拒绝）
 * 在路由快照在册而 e2e 字面量零命中：既有面料用例走标准建单入口
 * 提交完整面料字段，面料专用入口的建单口径（主表面料列一律落空串）、
 * 审核无来源状态门、拒绝理由专列写入、改单列映射与明细单价口径均无人钉。
 * 本 spec 把这七枚钉成写后读回链，并用唯一订单号把面料口与标准口的
 * 同筛选同口径互校做实。
 *
 * 契约真相（逐段读后端源，非推测）。
 * 路由装配在 backend/src/routes/sales.rs 的 sales_fabric_order_routes，
 * 前缀 /sales/fabric-orders；业务实现下沉在 backend/src/services/so/fabric_order.rs
 * 的 SalesService 面料族方法，handler 只做参数提取与会话身份注入。
 * 面料行与客户域销售订单是同一张表同一批列，标准详情口
 * GET sales/orders 对象带出明细数组，是面料行唯一的明细读回通道。
 *
 * 建单 POST 入参必填四键 customer_id order_date required_date items，
 * 每行明细必填 product_id quantity_meters quantity_kg unit_price_meters color_no，
 * 选填 final_price base_price 等。主表初值：状态词表 pending、订单号由服务端
 * 生成器出 SO 加八位日期加三位流水、subtotal 与 total_amount 与 balance_amount
 * 三列同值且口径为 行米数乘行米价（不是成交价）、tax discount shipping paid
 * 四列零、created_by 取会话用户、审批两列与理由两列全空。主表的面料扩展列
 * batch_no color_no dye_lot_no grade packaging_requirement quality_standard
 * 一律落空串——建单口不采集它们，提交值被丢弃是事实，本 spec 如实钉出。
 * delivery_address 提交值写入的是 billing_address 列，shipping_address 原列同名。
 * remarks 提交值写入 notes 列。
 * 明细落库列映射：quantity 等于米数、unit_price 等于成交价（缺省回落米价）、
 * 行 subtotal 等于米数乘成交价、base_price 缺省回落米价、fulfillment_source
 * 建单口不采集落 NULL。因此当成交价不等于米价时，主表 total 与行 subtotal 之和
 * 出现口径差，这是两列族各按各乘数的当前实现，本 spec 两边分别按公式钉死。
 *
 * 校验门：明细数量与单价为负 或 单价精度超两位小数 走可外显校验，
 * 400 加 VALIDATION_ERROR 且出参文案按行号与字段名点名；
 * 缺必填键走解码门 400 加 VALIDATION_ERROR；客户外键不存在穿过业务门撞数据库
 * 外键收口 500 加 DATABASE_ERROR。
 *
 * 状态门。审核入口 POST 对象段 approve 当前不校验来源状态（无门），
 * 对 pending 与 rejected 都会改写为 approved 并写 approved_by 会话用户与
 * approved_at 服务端时间；拒绝入口 POST 对象段 reject 有硬门：来源必须 pending
 * 否则 400 加 BUSINESS_ERROR（脱敏），理由 trim 后为空 400 加 VALIDATION_ERROR
 * 且文案原文可钉；拒绝成功写 status 为 rejected、rejected_reason 为 trim 值、
 * approval_reason 保持 NULL（两动作两列），并同样写 approved_by 与 approved_at。
 * 主表更新 PUT 的 status 键直写列：词表内取值当前无状态机门即可写入（含 shipped），
 * 词表外取值撞数据库 CHECK 收口 400 加 BAD_REQUEST；remarks 与 items 键在
 * 改单口不消费。
 * 其中"审核对来源状态无门"与"改单词表内无状态机门"两处为现状登记，
 * 不代表契约认可，对应缺陷由调度登记、判责在后端实现；本 spec 只钉当前真实
 * 可达的行为，不反向锁死（绝不外显"未来也应允许"的含义），后端补门后
 * 这两步转红即预期内的返工信号。
 *
 * 删除门。面料入口 DELETE 不清明细，带明细的行删主表撞外键收口 500 加
 * DATABASE_ERROR，这是面料口与标准口的真实差异；标准入口 DELETE 同事务先清
 * 预留与明细再删主表，但 shipped 与 completed 状态有业务门拒删（脱敏 business，
 * 只钉 HTTP 与机器码），改回 pending 后方可删——本 spec 用它收口全部自建行。
 * 两口对不存在主键都是 404 加 NOT_FOUND。
 *
 * 假绿防线。每次写都紧跟按主键独立读回逐列对表；负例一律钉 HTTP 与机器码，
 * 可外显族加钉文案原文；被拒建单的零推进用唯一备注标记在客户与状态过滤列表里
 * 反查不出现；两通道互校用服务端生成的唯一订单号过滤，主键与 total 双向对齐，
 * 不拿并发易漂移的全表 total 凑数。全程不用 ?? 或 || 兜底，后端必出的键缺失
 * 即形漂判红；造数失败当场红并点名步骤。
 */

type Row = Record<string, unknown>;

/** 失败机器码：DATABASE_ERROR 是约束违例统一信封的数据库族通道，
 * helpers 常量集未收录，按后端出参原文局部声明，不改共享夹具 */
const DATABASE_ERROR_CODE = 'DATABASE_ERROR';

const str = (v: unknown): string => String(v);
const num = (v: unknown): number => Number(String(v));

/** 时间列断非空可解析，返回 epoch 毫秒供等值比对 */
function assertParseableTime(value: unknown, label: string): number {
  expect(
    typeof value === 'string' && value.length > 0,
    `${label} 必须是非空字符串，实际 ${JSON.stringify(value)}`
  ).toBe(true);
  const parsed = Date.parse(String(value));
  expect(Number.isNaN(parsed), `${label} 必须能被解析为时间，实际 ${JSON.stringify(value)}`).toBe(
    false
  );
  return parsed;
}

interface FabricItem {
  product_id: number;
  quantity_meters: string;
  quantity_kg: string;
  unit_price_meters: string;
  color_no: string;
  final_price?: string;
  base_price?: string;
  remarks?: string;
}

/** 本 worker 专属面料建单，返回主表整行；造数失败当场红并点名是建单这一步 */
async function createFabricOrder(page: Page, items: FabricItem[], tag: string): Promise<Row> {
  const ctx = getCtx();
  expect(
    ctx.customerId !== undefined && ctx.productIds.length > 0,
    `120 夹具客户或产品缺失，customerId=${String(ctx.customerId)} productIds=${JSON.stringify(
      ctx.productIds
    )}`
  ).toBe(true);
  const created = await apiCallRaw<Row>(page, 'POST', '/sales/fabric-orders', {
    customer_id: ctx.customerId,
    order_date: new Date().toISOString(),
    required_date: new Date(Date.now() + 30 * 86_400_000).toISOString(),
    shipping_address: `120收货${tag}`,
    delivery_address: `120开票${tag}`,
    remarks: `120备注${tag}`,
    batch_no: `BN${tag}`,
    color_no: `CN${tag}`,
    items,
  });
  const orderId = num(created.id);
  expect(
    Number.isInteger(orderId) && orderId > 0,
    `120 面料建单失败，实际 ${JSON.stringify(created)}`
  ).toBe(true);
  return created;
}

/** 一行的合法明细入参：米数与米价固定，成交价可选覆盖 */
function item(productIndex: number, over: Partial<FabricItem> = {}): FabricItem {
  return {
    product_id: getCtx().productIds[productIndex],
    quantity_meters: '10.00',
    quantity_kg: '3.00',
    unit_price_meters: '25.50',
    color_no: `E2E-120-${productIndex}`,
    ...over,
  };
}

/** 标准详情口读回（面料行唯一的明细通道），明细与主表同回 */
async function readStandardDetail(page: Page, orderId: number): Promise<Row> {
  return await apiCallRaw<Row>(page, 'GET', `/sales/orders/${orderId}`);
}

/** 按订单号在两通道各查一次，钉唯一命中与主键一致（并发安全的两口径互校） */
async function crossCheckOrderNo(page: Page, orderNo: string, orderId: number): Promise<void> {
  const fabricList = await apiCallRaw<Row>(
    page,
    'GET',
    `/sales/fabric-orders?order_no=${encodeURIComponent(orderNo)}&page=1&page_size=20`
  );
  const fabricRows = fabricList.items as Row[];
  expect(Array.isArray(fabricRows), `面料口按订单号列表必须出 items 数组`).toBe(true);
  expect(
    num(fabricList.total),
    `面料口按唯一订单号必须恰命中一行，实际 ${JSON.stringify(fabricRows)}`
  ).toBe(1);
  expect(num(fabricRows[0].id), '面料口命中行主键必须等于自建订单').toBe(orderId);

  const stdList = await apiCallRaw<Row>(
    page,
    'GET',
    `/sales/orders?order_no=${encodeURIComponent(orderNo)}&page=1&page_size=20`
  );
  const stdRows = stdList.items as Row[];
  expect(Array.isArray(stdRows), '标准口按订单号列表必须出 items 数组').toBe(true);
  expect(
    num(stdList.total),
    `同一张表同一过滤条件下标准口 total 必须与面料口一致为 1，实际 ${JSON.stringify(stdRows)}`
  ).toBe(num(fabricList.total));
  expect(num(stdRows[0].id), '标准口命中行主键必须等于自建订单').toBe(orderId);
}

test.describe.serial('120 面料行业版销售订单端点族真值链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('T1 建单服务端初值逐列回显 明细跨口对表 主表与行金额口径差如实钉', async ({ page }) => {
    const tag = genCode('T1');
    const self = await apiCallRaw<Row>(page, 'GET', '/auth/me');
    // 行一：成交价缺省（回落米价）；行二：成交价 9.00 低于米价 10.00，制造口径差
    const items: FabricItem[] = [
      item(0),
      item(1, { quantity_meters: '2.00', unit_price_meters: '10.00', final_price: '9.00' }),
    ];
    const created = await createFabricOrder(page, items, tag);
    const orderId = num(created.id);
    const orderNo = str(created.order_no);

    // 主表初值：订单号格式 状态 金额四族列 身份列 面料列 审批理由列
    expect(
      /^SO\d{11}$/.test(orderNo),
      `订单号必须是 SO 加八位日期加三位流水形态，实际 ${orderNo}`
    ).toBe(true);
    expect(str(created.status), '建单初值状态必须为 pending').toBe('pending');
    // 主表金额口径：Σ(米数乘米价) = 10.00×25.50 + 2.00×10.00 = 275.00
    expect(num(created.subtotal), 'subtotal 必须等于米数乘米价之和').toBe(275);
    expect(num(created.total_amount), 'total_amount 必须与 subtotal 同值').toBe(275);
    expect(num(created.balance_amount), 'balance_amount 必须与 total_amount 同值').toBe(275);
    expect(num(created.tax_amount), '税额初值必须为 0').toBe(0);
    expect(num(created.discount_amount), '折扣初值必须为 0').toBe(0);
    expect(num(created.shipping_cost), '运费初值必须为 0').toBe(0);
    expect(num(created.paid_amount), '已付初值必须为 0').toBe(0);
    expect(num(created.created_by), 'created_by 必须是会话用户').toBe(num(self.id));
    expect(created.approved_by, '建单不得写审批人').toBeNull();
    expect(created.approved_at, '建单不得写审批时间').toBeNull();
    expect(created.approval_reason, '建单不得写审批理由').toBeNull();
    expect(created.rejected_reason, '建单不得写拒绝理由').toBeNull();
    expect(created.contact_person, '联系人占位列建单必须为 NULL').toBeNull();
    expect(created.contact_phone, '联系电话占位列建单必须为 NULL').toBeNull();
    // 面料主表扩展列：建单口不采集，提交值丢弃、列落空串（当前实现的事实）
    for (const col of [
      'batch_no',
      'color_no',
      'dye_lot_no',
      'grade',
      'packaging_requirement',
      'quality_standard',
    ]) {
      expect(str(created[col]), `面料扩展列 ${col} 建单必须落空串而不是提交值`).toBe('');
    }
    expect(str(created.notes), 'remarks 提交值必须写入 notes 列').toBe(`120备注${tag}`);
    expect(str(created.shipping_address), '收货地址必须落同名列').toBe(`120收货${tag}`);
    expect(str(created.billing_address), 'delivery_address 提交值落 billing_address 列').toBe(
      `120开票${tag}`
    );
    expect(str(created.customer_id), '客户外键必须回显夹具客户').toBe(String(getCtx().customerId));
    expect(
      assertParseableTime(created.order_date, '订单日期'),
      '订单日期 epoch 必须可解析'
    ).toBeGreaterThan(0);

    // 面料对象读口互证
    const fabricDetail = await apiCallRaw<Row>(page, 'GET', `/sales/fabric-orders/${orderId}`);
    expect(num(fabricDetail.id), '面料详情主键必须等于建行主键').toBe(orderId);
    expect(str(fabricDetail.order_no), '面料详情订单号必须等于建行回显').toBe(orderNo);
    expect(str(fabricDetail.status), '面料详情状态必须为 pending').toBe('pending');

    // 明细只能经标准详情口回读：两行逐列对表，行口径为 米数乘成交价
    const detail = await readStandardDetail(page, orderId);
    expect(num(detail.id), '标准详情主键必须等于自建订单').toBe(orderId);
    expect(Array.isArray(detail.items), '标准详情必须带 items 明细数组').toBe(true);
    const rows = detail.items as Row[];
    expect(rows.length, `明细行数必须恰为提交的 2 行，实际 ${JSON.stringify(rows)}`).toBe(2);
    const line1 = rows.find(r => num(r.product_id) === getCtx().productIds[0]);
    expect(line1, `明细找不到引用产品 0 的行，实际 ${JSON.stringify(rows)}`).toBeDefined();
    expect(num(line1!.quantity), '行 quantity 列必须等于米数').toBe(10);
    expect(num(line1!.quantity_meters), '行米数列必须回显').toBe(10);
    expect(num(line1!.unit_price), '成交价缺省时行 unit_price 必须回落米价').toBe(25.5);
    expect(num(line1!.final_price), '成交价缺省时 final_price 必须回落米价').toBe(25.5);
    expect(num(line1!.base_price), 'base_price 缺省必须回落米价').toBe(25.5);
    expect(num(line1!.subtotal), '行 subtotal 必须等于米数乘成交价').toBe(255);
    expect(str(line1!.color_no), '行色号必须回显提交值').toBe('E2E-120-0');
    expect(num(line1!.quantity_kg), '行公斤数列必须回显').toBe(3);
    expect(num(line1!.shipped_quantity), '未发货行发货数量必须为 0').toBe(0);
    expect(num(line1!.shipped_quantity_meters), '未发货行发货米数必须为 0').toBe(0);
    expect(
      line1!.fulfillment_source,
      `建单口不采集流转来源，必须落 NULL，实际 ${JSON.stringify(line1!.fulfillment_source)}`
    ).toBeNull();
    const line2 = rows.find(r => num(r.product_id) === getCtx().productIds[1]);
    expect(line2, `明细找不到引用产品 1 的行，实际 ${JSON.stringify(rows)}`).toBeDefined();
    expect(num(line2!.unit_price), '行 unit_price 必须取成交价而不是米价').toBe(9);
    expect(num(line2!.final_price), '行 final_price 必须回显成交价').toBe(9);
    expect(num(line2!.subtotal), '行 subtotal 必须等于米数乘成交价 2×9').toBe(18);

    // 口径差如实钉：主表按米价求和 275，行按成交价求和 255+18=273，差 2.00 为当前实现
    const sumLine = rows.reduce((acc, r) => acc + num(r.subtotal), 0);
    expect(sumLine, '行 subtotal 之和必须按成交价口径等于 273').toBe(273);
    expect(num(detail.total_amount), '主表金额必须按米价口径保持 275').toBe(275);

    // 两通道按唯一订单号互校命中
    await crossCheckOrderNo(page, orderNo, orderId);

    await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, `[120] T1 ${orderId}`);
  });

  test('T2 明细四道校验门与解码门外键门 被拒提交零建行', async ({ page }) => {
    const tag = genCode('T2');
    const ctx = getCtx();

    // 一 米数为负：可外显校验，文案点名行号与字段
    const negQty = await apiCallExpectFail(page, 'POST', '/sales/fabric-orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      required_date: new Date(Date.now() + 86_400_000).toISOString(),
      remarks: `120被拒负数量${tag}`,
      items: [item(0, { quantity_meters: '-1.00' })],
    });
    expect(negQty.status, `负米数必须 400，实际 ${negQty.status}`).toBe(400);
    expect(
      failureCode(negQty),
      `负米数必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(negQty.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(str(negQty.message), '负米数文案必须点名行号与字段原文').toBe(
      '第 1 项 quantity_meters 不能为负数'
    );

    // 二 米价三位小数越精度
    const badPrecision = await apiCallExpectFail(page, 'POST', '/sales/fabric-orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      required_date: new Date(Date.now() + 86_400_000).toISOString(),
      remarks: `120被拒精度越界${tag}`,
      items: [item(0, { unit_price_meters: '25.501' })],
    });
    expect(badPrecision.status, `越精度米价必须 400，实际 ${badPrecision.status}`).toBe(400);
    expect(
      failureCode(badPrecision),
      `越精度必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(badPrecision.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(str(badPrecision.message), '越精度文案必须点名行号与字段原文').toBe(
      '第 1 项 unit_price_meters 精度不能超过 2 位小数'
    );

    // 三 成交价越精度：可选价格字段同走精度门
    const badFinal = await apiCallExpectFail(page, 'POST', '/sales/fabric-orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      required_date: new Date(Date.now() + 86_400_000).toISOString(),
      remarks: `120被拒成交价越精度${tag}`,
      items: [item(0, { final_price: '9.001' })],
    });
    expect(badFinal.status, `越精度成交价必须 400，实际 ${badFinal.status}`).toBe(400);
    expect(str(badFinal.message), '成交价越精度文案必须点名字段名').toBe(
      '第 1 项 final_price 精度不能超过 2 位小数'
    );

    // 四 缺 items 必填键：解码门
    const missingItems = await apiCallExpectFail(page, 'POST', '/sales/fabric-orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      required_date: new Date(Date.now() + 86_400_000).toISOString(),
      remarks: `120被拒缺明细${tag}`,
    });
    expect(missingItems.status, `缺 items 必须 400，实际 ${missingItems.status}`).toBe(400);
    expect(
      failureCode(missingItems),
      `缺必填键必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(missingItems.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 五 日期不可解析：解码门同族
    const badDate = await apiCallExpectFail(page, 'POST', '/sales/fabric-orders', {
      customer_id: ctx.customerId,
      order_date: '2025-13-45 not-a-time',
      required_date: new Date(Date.now() + 86_400_000).toISOString(),
      remarks: `120被拒坏日期${tag}`,
      items: [item(0)],
    });
    expect(badDate.status, `坏日期必须 400，实际 ${badDate.status}`).toBe(400);
    expect(
      failureCode(badDate),
      `坏日期必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(badDate.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 六 客户外键缺行：穿过业务门撞数据库外键，收口 500 加 DATABASE_ERROR
    const fkMissing = await apiCallExpectFail(page, 'POST', '/sales/fabric-orders', {
      customer_id: 999999999,
      order_date: new Date().toISOString(),
      required_date: new Date(Date.now() + 86_400_000).toISOString(),
      remarks: `120被拒坏客户${tag}`,
      items: [item(0)],
    });
    expect(
      fkMissing.status,
      `客户不存在的建单必须走外键通道 500，实际 ${fkMissing.status} code=${JSON.stringify(
        fkMissing.code
      )}`
    ).toBe(500);
    expect(
      failureCode(fkMissing),
      `外键违例必须归因 DATABASE_ERROR，实际 ${JSON.stringify(fkMissing.code)}`
    ).toBe(DATABASE_ERROR_CODE);

    // 零推进总证：六次被拒的专属备注标记都不许出现在本客户待审列表里
    const list = await apiCallRaw<Row>(
      page,
      'GET',
      `/sales/fabric-orders?customer_id=${ctx.customerId}&status=pending&page=1&page_size=100`
    );
    const rows = list.items as Row[];
    expect(Array.isArray(rows), '客户与状态过滤列表必须出 items 数组').toBe(true);
    const leaked = rows.filter(r => str(r.notes).includes(tag));
    expect(leaked, `被拒的六次提交都不许留下带本用例标记的行`).toHaveLength(0);
  });

  test('T3 审核无来源状态门 拒绝硬门与理由专列 被拒零漂移', async ({ page }) => {
    const self = await apiCallRaw<Row>(page, 'GET', '/auth/me');

    // 一 pending 审核：写审批身份两列并回读
    const o1 = await createFabricOrder(page, [item(0)], genCode('T3A'));
    const o1Id = num(o1.id);
    const approved = await apiCallRaw<Row>(
      page,
      'POST',
      `/sales/fabric-orders/${o1Id}/approve`,
      {}
    );
    expect(str(approved.status), '审核后状态必须为 approved').toBe('approved');
    expect(num(approved.approved_by), '审批人必须是会话用户而不是请求体携带').toBe(num(self.id));
    assertParseableTime(approved.approved_at, '审批时间');
    const o1After = await apiCallRaw<Row>(page, 'GET', `/sales/fabric-orders/${o1Id}`);
    expect(str(o1After.status), '审核必须真落库').toBe('approved');
    expect(o1After.approval_reason, '审核入口不采集审批理由，列必须保持 NULL').toBeNull();

    // 二 对 approved 行拒绝：撞 pending 硬门（脱敏 business）
    const rejectApproved = await apiCallExpectFail(
      page,
      'POST',
      `/sales/fabric-orders/${o1Id}/reject`,
      { reason: '已审批订单试拒绝' }
    );
    expect(rejectApproved.status, `approved 行拒绝必须 400，实际 ${rejectApproved.status}`).toBe(
      400
    );
    expect(
      failureCode(rejectApproved),
      `拒绝状态门必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(rejectApproved.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const o1Unchanged = await apiCallRaw<Row>(page, 'GET', `/sales/fabric-orders/${o1Id}`);
    expect(str(o1Unchanged.status), '被拒的拒绝不许改动状态').toBe('approved');
    expect(o1Unchanged.rejected_reason, '被拒的拒绝不许写拒绝理由列').toBeNull();

    // 三 空白理由：可外显校验门原文
    const o2 = await createFabricOrder(page, [item(1)], genCode('T3B'));
    const o2Id = num(o2.id);
    const blankReason = await apiCallExpectFail(
      page,
      'POST',
      `/sales/fabric-orders/${o2Id}/reject`,
      { reason: '   ' }
    );
    expect(blankReason.status, `空白理由拒绝必须 400，实际 ${blankReason.status}`).toBe(400);
    expect(
      failureCode(blankReason),
      `空白理由必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(blankReason.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(str(blankReason.message), '空白理由文案必须是后端原文').toBe('审批拒绝理由不能为空');
    const o2StillPending = await apiCallRaw<Row>(page, 'GET', `/sales/fabric-orders/${o2Id}`);
    expect(str(o2StillPending.status), '被拒的拒绝不许把 pending 改成别的状态').toBe('pending');

    // 四 拒绝成功：trim 后写拒绝专列，两动作两列，审批理由列不挪用
    const reasonRaw = `  120拒稿${genCode('R')}  `;
    const reasonTrimmed = reasonRaw.trim();
    const rejected = await apiCallRaw<Row>(page, 'POST', `/sales/fabric-orders/${o2Id}/reject`, {
      reason: reasonRaw,
    });
    expect(str(rejected.status), '拒绝后状态必须为 rejected').toBe('rejected');
    expect(
      str(rejected.rejected_reason),
      `拒绝理由列必须等于 trim 后的提交值，实际 ${JSON.stringify(rejected.rejected_reason)}`
    ).toBe(reasonTrimmed);
    expect(rejected.approval_reason, '拒绝动作不许写审批理由列').toBeNull();
    expect(num(rejected.approved_by), '拒绝动作同样记录操作人会话身份').toBe(num(self.id));
    assertParseableTime(rejected.approved_at, '拒绝记录的审批时间列');
    const o2After = await apiCallRaw<Row>(page, 'GET', `/sales/fabric-orders/${o2Id}`);
    expect(str(o2After.status), '拒绝必须真落库').toBe('rejected');
    expect(str(o2After.rejected_reason), '回读的拒绝理由必须与写响应一致').toBe(reasonTrimmed);

    // 五 重复拒绝：rejected 不在 pending 白名单，撞同一状态门
    const rejectAgain = await apiCallExpectFail(
      page,
      'POST',
      `/sales/fabric-orders/${o2Id}/reject`,
      { reason: '二次拒绝' }
    );
    expect(rejectAgain.status, `rejected 行二次拒绝必须 400，实际 ${rejectAgain.status}`).toBe(400);
    expect(failureCode(rejectAgain), '二次拒绝必须归因 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );

    // 六 审核入口对 rejected 行当前无来源状态门，会直接改写回 approved。
    // 本步是现状登记，不代表契约认可：审核缺状态门的缺陷由调度另立车道、判责在
    // 后端实现；这里只钉"当前实现允许了这一步"，不反向锁死，后端补门后本步转红
    // 即预期内的返工信号。
    const approveRejected = await apiCallRaw<Row>(
      page,
      'POST',
      `/sales/fabric-orders/${o2Id}/approve`,
      {}
    );
    expect(
      str(approveRejected.status),
      `现状登记（非契约认可）：审核入口对 rejected 行当前放行并改写 approved，实际 ${str(
        approveRejected.status
      )}`
    ).toBe('approved');
    const o2Final = await apiCallRaw<Row>(page, 'GET', `/sales/fabric-orders/${o2Id}`);
    expect(str(o2Final.status), '现状登记：该无门改写必须真落库为 approved').toBe('approved');
    expect(
      str(o2Final.rejected_reason),
      '审核改写状态后，此前拒绝留下的理由列按当前实现保留原值'
    ).toBe(reasonTrimmed);

    // 七 两动作对不存在主键都先行 404
    const approveMissing = await apiCallExpectFail(
      page,
      'POST',
      '/sales/fabric-orders/99999999/approve',
      {}
    );
    expect(approveMissing.status, `不存在主键的审核必须 404，实际 ${approveMissing.status}`).toBe(
      404
    );
    expect(failureCode(approveMissing), '不存在主键的审核必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
    const rejectMissing = await apiCallExpectFail(
      page,
      'POST',
      '/sales/fabric-orders/99999999/reject',
      { reason: '不存在主键拒绝' }
    );
    expect(rejectMissing.status, `不存在主键的拒绝必须 404，实际 ${rejectMissing.status}`).toBe(
      404
    );
    expect(failureCode(rejectMissing), '不存在主键的拒绝必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );

    await tryCleanup(page, 'DELETE', `/sales/orders/${o1Id}`, `[120] T3 o1 ${o1Id}`);
    await tryCleanup(page, 'DELETE', `/sales/orders/${o2Id}`, `[120] T3 o2 ${o2Id}`);
  });

  test('T4 主表更新列映射 remarks不消费 状态词表内无门词表外撞CHECK 删除状态门联动', async ({
    page,
  }) => {
    const tag = genCode('T4');
    const created = await createFabricOrder(page, [item(0)], tag);
    const orderId = num(created.id);
    const originalNotes = str(created.notes);
    const newRequiredMs = Date.now() + 45 * 86_400_000;

    // 一 列映射：delivery_address 写 billing_address，remarks 在改单口不消费
    const updated = await apiCallRaw<Row>(page, 'PUT', `/sales/fabric-orders/${orderId}`, {
      shipping_address: `120改收货${tag}`,
      delivery_address: `120改开票${tag}`,
      remarks: `120改备注${tag}-不该落库`,
      required_date: new Date(newRequiredMs).toISOString(),
    });
    expect(str(updated.shipping_address), '改单收货地址必须落同名列').toBe(`120改收货${tag}`);
    expect(str(updated.billing_address), '改单 delivery_address 提交值落 billing_address 列').toBe(
      `120改开票${tag}`
    );
    expect(str(updated.notes), '改单口不消费 remarks，notes 必须保持建单值').toBe(originalNotes);
    expect(
      assertParseableTime(updated.required_date, '改单后交期'),
      '交期 epoch 必须等于新提交值'
    ).toBe(newRequiredMs);
    const t1 = await apiCallRaw<Row>(page, 'GET', `/sales/fabric-orders/${orderId}`);
    expect(str(t1.notes), '对象读回也必须证明 remarks 的改动没落库').toBe(originalNotes);

    // 二 状态词表外取值：改单口直写列，撞数据库 CHECK 收口 400 加 BAD_REQUEST
    const badStatus = await apiCallExpectFail(page, 'PUT', `/sales/fabric-orders/${orderId}`, {
      status: 'e2e_词表外状态',
    });
    expect(badStatus.status, `词表外状态必须 400，实际 ${badStatus.status}`).toBe(400);
    expect(
      failureCode(badStatus),
      `词表外状态必须归因 BAD_REQUEST（CHECK 违例经改单口收口），实际 ${JSON.stringify(
        badStatus.code
      )}`
    ).toBe(APP_ERROR_CODES.BAD_REQUEST);
    const afterBadStatus = await apiCallRaw<Row>(page, 'GET', `/sales/fabric-orders/${orderId}`);
    expect(str(afterBadStatus.status), '被拒的改状态不许改动状态列').toBe('pending');

    // 三 词表内取值 shipped：改单口当前无状态机门即写成功，并联动标准删除口的状态门。
    // 本步同样是现状登记，不代表契约认可：改单缺状态机门的缺陷由调度另立车道、
    // 判责在后端实现；补门后本步转红即预期内的返工信号。
    const shipped = await apiCallRaw<Row>(page, 'PUT', `/sales/fabric-orders/${orderId}`, {
      status: 'shipped',
    });
    expect(
      str(shipped.status),
      `现状登记（非契约认可）：改单口对词表内 shipped 当前放行直写，实际 ${str(shipped.status)}`
    ).toBe('shipped');
    const deleteBlocked = await apiCallExpectFail(page, 'DELETE', `/sales/orders/${orderId}`);
    expect(
      deleteBlocked.status,
      `shipped 行的标准删除必须撞状态门 400，实际 ${deleteBlocked.status}`
    ).toBe(400);
    expect(
      failureCode(deleteBlocked),
      `删除状态门必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(deleteBlocked.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const stillThere = await apiCallRaw<Row>(page, 'GET', `/sales/fabric-orders/${orderId}`);
    expect(str(stillThere.status), '被拒的删除不许动行').toBe('shipped');

    // 四 改回 pending 后方可删（与三互为正反手的收口通道，同样属现状登记：
    //    改单补门后若 shipped 不再可写，本步连同三一起转红为预期返工信号）
    const back = await apiCallRaw<Row>(page, 'PUT', `/sales/fabric-orders/${orderId}`, {
      status: 'pending',
    });
    expect(
      str(back.status),
      `现状登记（非契约认可）：词表内 pending 改回当前放行，实际 ${str(back.status)}`
    ).toBe('pending');

    // 五 改单与删除对不存在主键都先行 404
    const putMissing = await apiCallExpectFail(page, 'PUT', '/sales/fabric-orders/99999999', {
      remarks: '不存在主键改单',
    });
    expect(putMissing.status, `不存在主键的改单必须 404，实际 ${putMissing.status}`).toBe(404);
    expect(failureCode(putMissing), '不存在主键的改单必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );

    await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, `[120] T4 ${orderId}`);
  });

  test('T5 分页回显与 clamp 状态过滤语义 面料口删带明细行撞外键 标准口清明细删除成功', async ({
    page,
  }) => {
    const ctx = getCtx();
    const created = await createFabricOrder(page, [item(0), item(1)], genCode('T5'));
    const orderId = num(created.id);
    const orderNo = str(created.order_no);

    // 一 分页四键回显与生效页长
    const list = await apiCallRaw<Row>(
      page,
      'GET',
      `/sales/fabric-orders?customer_id=${ctx.customerId}&order_no=${encodeURIComponent(
        orderNo
      )}&page=1&page_size=20`
    );
    expect(num(list.page), '列表 page 必须回显生效页码 1').toBe(1);
    expect(num(list.page_size), '列表 page_size 必须回显生效页长 20').toBe(20);
    expect(num(list.total), `按唯一订单号过滤 total 必须为 1，实际 ${JSON.stringify(list)}`).toBe(
      1
    );
    expect(num((list.items as Row[])[0].id), '命中行主键必须等于自建订单').toBe(orderId);

    // 二 页长与页码 clamp：500 收到 100，越界页码收到 1000 且出空页不报错；
    //    同筛选紧邻两次读取比 total，钉"越界页只空页不改计数"的口径
    const clamped = await apiCallRaw<Row>(
      page,
      'GET',
      `/sales/fabric-orders?customer_id=${ctx.customerId}&page=1&page_size=500`
    );
    expect(num(clamped.page_size), 'page_size=500 必须被 clamp 到 100 回显').toBe(100);
    const nearPage = await apiCallRaw<Row>(
      page,
      'GET',
      `/sales/fabric-orders?customer_id=${ctx.customerId}&page=1&page_size=100`
    );
    const farPage = await apiCallRaw<Row>(
      page,
      'GET',
      `/sales/fabric-orders?customer_id=${ctx.customerId}&page=99999&page_size=100`
    );
    expect(num(farPage.page), 'page=99999 必须被 clamp 到 1000 回显').toBe(1000);
    expect((farPage.items as Row[]).length, '越界空页 items 必须为空数组').toBe(0);
    expect(
      num(farPage.total),
      `越界页的 total 必须与同筛选首页一致为 ${String(num(nearPage.total))}，实际 ${String(
        num(farPage.total)
      )}`
    ).toBe(num(nearPage.total));
    expect(nearPage.items, `首页结果必须包含自建订单 ${String(orderId)}`).toEqual(
      expect.arrayContaining([expect.objectContaining({ id: orderId })])
    );

    // 三 状态过滤语义：pending 过滤必须命中，shipped 过滤必须不命中同一单号
    const pendingHit = await apiCallRaw<Row>(
      page,
      'GET',
      `/sales/fabric-orders?order_no=${encodeURIComponent(orderNo)}&status=pending&page=1&page_size=20`
    );
    expect(num(pendingHit.total), 'pending 过滤按唯一单号必须命中 1').toBe(1);
    const shippedMiss = await apiCallRaw<Row>(
      page,
      'GET',
      `/sales/fabric-orders?order_no=${encodeURIComponent(orderNo)}&status=shipped&page=1&page_size=20`
    );
    expect(num(shippedMiss.total), 'shipped 过滤按唯一单号必须为 0').toBe(0);

    // 四 面料删除口不清明细：带明细行删主表撞外键收口 500 加 DATABASE_ERROR，
    //    行与明细都在；随后标准口同事务清明细删主表收口本用例
    const fabricDelete = await apiCallExpectFail(page, 'DELETE', `/sales/fabric-orders/${orderId}`);
    expect(
      fabricDelete.status,
      `面料口删带明细的行必须走外键通道 500，实际 ${fabricDelete.status} code=${JSON.stringify(
        fabricDelete.code
      )}`
    ).toBe(500);
    expect(
      failureCode(fabricDelete),
      `明细外键违例必须归因 DATABASE_ERROR，实际 ${JSON.stringify(fabricDelete.code)}`
    ).toBe(DATABASE_ERROR_CODE);
    const stillAlive = await apiCallRaw<Row>(page, 'GET', `/sales/fabric-orders/${orderId}`);
    expect(num(stillAlive.id), '被面料口拒删的行必须原样在库').toBe(orderId);
    const detailWithItems = await readStandardDetail(page, orderId);
    expect((detailWithItems.items as Row[]).length, '被拒的删除不许连带清掉明细行').toBe(2);

    const deleted = await apiCall<unknown>(page, 'DELETE', `/sales/orders/${orderId}`);
    expect(
      num(deleted.code),
      `标准删除的成功信封状态码必须是 200，实际 ${JSON.stringify(deleted.code)}`
    ).toBe(200);

    const goneFabric = await apiCallExpectFail(page, 'GET', `/sales/fabric-orders/${orderId}`);
    expect(goneFabric.status, '删除后面料详情必须 404').toBe(404);
    expect(failureCode(goneFabric), '删除后面料详情必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
    const goneStd = await apiCallExpectFail(page, 'GET', `/sales/orders/${orderId}`);
    expect(goneStd.status, '删除后标准详情必须 404').toBe(404);
    expect(failureCode(goneStd), '删除后标准详情必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );

    // 五 面料删除口对不存在主键先行 404（与 500 外键通道可分辨）
    const deleteMissing = await apiCallExpectFail(page, 'DELETE', '/sales/fabric-orders/99999999');
    expect(deleteMissing.status, `不存在主键的面料删除必须 404，实际 ${deleteMissing.status}`).toBe(
      404
    );
    expect(failureCode(deleteMissing), '不存在主键的面料删除必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
  });
});
