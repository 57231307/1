// 大货批色审批（bulk-color-approvals）由销售订单明细行发起的端到端落库回读 — flow/81
//
// 覆盖目标（对着后端真实契约编写，断言全部走 GET 回读真实落库键，不只看 toast/2xx）：
//   1. 带出持久：POST 建单带 sales_order_item_id 后，反范式 product_id/color_no 由后端以该明细行覆盖
//      回填（不信任前端手填值），GET 回读逐字符相等。
//   2. 列表富化：GET 列表每条恒带 order_no/customer_name/approver_name 三键，且取到关联真实值
//      （证明关联名称来自单次 LEFT JOIN，而非裸 Model 的恒空列）。
//   3. 越权拒绝：sales_order_id 为本单、sales_order_item_id 取他单或不存在的明细行时，建单被业务
//      校验拒绝（HTTP 400 + 校验机器码，非 5xx 裸崩），且 GET 列表不新增行（fail-closed 零落库）。
//   4. 按行发货门禁作用域：批色门禁在销售发货前置校验里按本次发货明细行的 (product_id,color_no)
//      归一匹配到订单行，命中未通过审批的行才拦截，未命中的行不被该门禁拦（证明按行而非整单）。
//
// 后端契约事实（据此断言，非臆测）：
//   - 建单请求体新增可选 sales_order_item_id：非空则后端要求该 sales_order_items 行存在且其所属
//     订单等于入参 sales_order_id，否则以校验错误拒绝；并以该行的 product_id/color_no 覆盖回填反范式列。
//     必填 sales_order_id/dye_batch_id/customer_id（三者均为指向真实父行的外键，故必须引用真实实体，
//     不能凭空造 id，否则外键约束会让落库以数据库错误失败而非静默成功）。
//   - 建单成功响应与列表条目均带 sales_order_item_id、product_id、color_no；列表条目用真实状态键
//     approval_status，且带 order_no/customer_name/approver_name 三个关联名称键（无值时为 null）。
//   - 列表响应信封为 data.items（分页对象 {items,total,page,page_size}），不存在 approval_no/applicant_name 键。
//   - 越权/不存在明细行的拒绝走校验通道：HTTP 400、机器码 VALIDATION_ERROR、message 为真实拒绝原因。
//   - 销售发货的批色门禁发生在“缸号一致性 + 批色门禁”前置校验阶段，早于订单状态门与库存扣减门，
//     因此即使订单尚未审批、尚无可用库存，命中行的批色拒绝原因仍会先于其它门出现；未命中行的拒绝
//     原因则不含批色字样（它要么放行、要么卡在后续与本门禁无关的订单状态/库存门）。
//
// 真实落库前提：批色记录的外键 sales_order_id→sales_orders、dye_batch_id→dye_batch、
// customer_id→customers 均 NOT NULL 且 ON DELETE RESTRICT；sales_order_item_id→sales_order_items 可空
// 且 ON DELETE SET NULL。故夹具必须先用 API 造真实客户与销售订单（含带色号的明细行），并复用
// ensureTestEntities 产出的真实染色批次 id（ctx.dyeBatchId），不能伪造主键。
//
// 尚未覆盖并交回编排方的风险点（非本 spec 放宽，而是夹具前置链过重，见交付报告）：
//   子项 4 的“approved 后整单放行”成功路径需要跑完剪样/发送客户/客户确认的完整状态机，而剪样门
//   要求关联染色批次已完成、存在质检合格且数量充足的大货库存、关联生产订单已完成，夹具过重，
//   本 spec 不伪造该成功断言，仅按行作用域真实覆盖“命中行点名批色拦截 / 未命中行不被该门禁拦”两个方向。
import { test, expect } from '@playwright/test';
import type { Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  APP_ERROR_CODES,
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
  ensureTestEntities,
  expectBusinessRejection,
  failureCode,
  genCode,
  getCtx,
  tryCleanup,
  verifyEndpointHealthy,
} from './helpers';

/** 列表条目（data.items 单元）：仅声明本 spec 据后端契约要断言的键，键名与后端序列化逐字符一致 */
interface BulkColorListRow {
  id: number;
  sales_order_id: number;
  sales_order_item_id: number | null;
  product_id: number | null;
  color_no: string | null;
  approval_status: string;
  order_no: string | null;
  customer_name: string | null;
  approver_name: string | null;
}

/** 建单响应（成功信封 data）：带 sales_order_item_id/product_id/color_no/approval_status */
interface BulkColorCreateResp {
  id: number;
  sales_order_id: number;
  sales_order_item_id: number | null;
  product_id: number | null;
  color_no: string | null;
  approval_status: string;
}

/** 销售订单明细行投影：用于取 id/product_id/color_no 作为批色发起与断言基准 */
interface SalesOrderItemRow {
  id: number;
  product_id: number;
  color_no: string;
}

/** 销售订单详情（建单返回与 GET 详情共用形状的子集） */
interface SalesOrderDetailResp {
  id?: number;
  order_no?: string;
  items?: SalesOrderItemRow[];
}

/**
 * 取某销售订单下的批色列表行（GET 回读）。
 * 断言 data.items 必为数组：缺键或形状漂移即判红，禁止用 `?? []` 吞掉缺键。
 */
async function readBulkColorRowsByOrder(
  page: import('@playwright/test').Page,
  salesOrderId: number
): Promise<BulkColorListRow[]> {
  const res = await apiCallRaw<{ items?: unknown; total?: number }>(
    page,
    'GET',
    `/bulk-color-approvals?sales_order_id=${salesOrderId}&page=1&page_size=100`
  );
  expect(
    Array.isArray(res?.items),
    `GET /bulk-color-approvals 响应 data.items 必须为数组（分页对象 {items,total,page,page_size}），实际 ${JSON.stringify(res)}`
  ).toBe(true);
  return res.items as BulkColorListRow[];
}

/** 按主键在列表行里精确定位单条批色记录 */
function findRowById(rows: BulkColorListRow[], id: number): BulkColorListRow | undefined {
  return rows.find(r => r.id === id);
}

/**
 * 创建一个带两行明细（各含不同 product + 不同 color_no）的销售订单，并回读真实落库的明细行 id/色号。
 * 返回建单人可见的销售订单 id、订单号与两行明细（第一行为 X、第二行为 Y）。
 */
async function createTwoLineSalesOrder(
  page: import('@playwright/test').Page,
  customerId: number,
  productX: number,
  colorX: string,
  productY: number,
  colorY: string
): Promise<SalesOrderDetailResp> {
  const resp = await apiCall<SalesOrderDetailResp>(page, 'POST', '/sales/orders', {
    customer_id: customerId,
    order_date: new Date().toISOString(),
    // quantity/unit_price 走字符串（rust_decimal 反序列化既有口径），数量取 1 与建单锁库存既有约定一致
    items: [
      { product_id: productX, quantity: '1', unit_price: '20', color_no: colorX },
      { product_id: productY, quantity: '1', unit_price: '20', color_no: colorY },
    ],
  });
  expect(resp.data?.id, `建销售订单未返回 id：${JSON.stringify(resp.data)}`).toBeTruthy();
  expect(
    Array.isArray(resp.data?.items),
    `建订单响应应含明细数组：${JSON.stringify(resp.data)}`
  ).toBe(true);
  expect(resp.data!.items!.length, '订单明细应为两行').toBeGreaterThanOrEqual(2);
  // 回读真实落库：明细行 id 存在且色号逐字符等于提交值（否则后续“带出/按行门禁”基准不成立）
  const readBack = await apiCall<SalesOrderDetailResp>(
    page,
    'GET',
    `/sales/orders/${resp.data!.id}`
  );
  expect(
    Array.isArray(readBack.data?.items),
    `GET 订单详情应能回读明细数组：${JSON.stringify(readBack.data)}`
  ).toBe(true);
  return readBack.data;
}

/**
 * 自建唯一命名产品并返回 id。
 * 调用方：本文件各用例的夹具准备。入参：page、唯一后缀 tag、来自 ensureTestEntities 的产品分类 id。
 * 传给谁：id 进销售订单明细行的 product_id。
 * 为什么不复用共享快照：ctx.productIds 是现读的全库产品列表，并发分片的收尾清理会删掉这些行，
 * 于是 POST /sales/orders 以 `产品 ID N 不存在` 400 被业务校验拒绝（用例被别人的清理打断）。
 * 用例自身依赖的数据一律自建自清。
 */
async function createOwnProduct(page: Page, tag: string, categoryId: number): Promise<number> {
  const resp = await apiCall<{ id?: number }>(page, 'POST', '/products', {
    name: `E2E81产品${tag}`,
    code: `E2E81${tag}`,
    category_id: categoryId,
    unit: 'm',
  });
  const id = Number(resp.data?.id);
  expect(id, `自建产品失败：${JSON.stringify(resp.data)}`).toBeGreaterThan(0);
  return id;
}

/** 取本用例可用的产品分类 id（ensureTestEntities 已保证"面料"分类存在并写入 ctx） */
function ownCategoryId(): number {
  const categoryId = getCtx().productCategoryIds[0];
  expect(categoryId, '前置：批色用例需产品分类 id 以自建产品').toBeTruthy();
  return categoryId!;
}

test.describe('大货批色审批 - 81 由销售订单明细行发起', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('带出持久：建单带明细行后反范式列由后端覆盖回填，GET 回读逐字符相等', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const categoryId = ownCategoryId();
    const productX = await createOwnProduct(page, genCode('81X'), categoryId);
    const productY = await createOwnProduct(page, genCode('81Y'), categoryId);
    const dyeBatchId = ctx.dyeBatchId;

    expect(dyeBatchId, '前置：批色建单需真实染色批次 id（外键 NOT NULL）').toBeTruthy();

    const custName = `E2E批色带出客户 ${genCode('BCA-A')}`;
    const cust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: custName,
    });
    const customerId = cust.data?.id;
    expect(customerId, `创建客户失败：${JSON.stringify(cust)}`).toBeTruthy();

    const colorX = `E2E-CX-${genCode('BCAC')}`;
    const colorY = `E2E-CY-${genCode('BCAC')}`;
    const order = await createTwoLineSalesOrder(
      page,
      customerId!,
      productX,
      colorX,
      productY,
      colorY
    );
    const orderId = order.id!;
    const itemX = order.items![0];
    const itemY = order.items![1];
    expect(itemX.color_no, `回读明细行 X 色号应等于提交值，实际 ${itemX.color_no}`).toBe(colorX);

    let bulkId = 0;
    try {
      await verifyEndpointHealthy(page, `/bulk-color-approvals?sales_order_id=${orderId}`);

      // 故意传入与明细行 X 不一致的 product/color 诱饵，证明后端以 sales_order_item_id 行覆盖回填、
      // 不信任前端手填值（若后端未覆盖，则回读会得到诱饵值而非行值，本断言即判红）。
      const created = await apiCall<BulkColorCreateResp>(page, 'POST', '/bulk-color-approvals', {
        sales_order_id: orderId,
        sales_order_item_id: itemX.id,
        dye_batch_id: dyeBatchId,
        customer_id: customerId,
        product_id: itemY.product_id,
        color_no: 'DECOY-COLOR-SHOULD-BE-OVERRIDDEN',
      });
      bulkId = Number(created.data?.id);
      expect(bulkId, `建批色记录未返回 id：${JSON.stringify(created.data)}`).toBeGreaterThan(0);

      // GET 回读：sales_order_item_id 持久正确，product_id/color_no 等于明细行 X 的真实值（非诱饵）
      const row = findRowById(await readBulkColorRowsByOrder(page, orderId), bulkId);
      expect(row, `GET 列表应能回读新建批色 ${bulkId}`).toBeDefined();
      expect(
        row!.sales_order_item_id,
        `回读 sales_order_item_id 应等于明细行 id ${itemX.id}，实际 ${row!.sales_order_item_id}`
      ).toBe(itemX.id);
      expect(
        row!.product_id,
        `回读 product_id 应被覆盖为明细行 X 的 ${itemX.product_id}（诱饵应被丢弃），实际 ${row!.product_id}`
      ).toBe(itemX.product_id);
      expect(
        row!.color_no,
        `回读 color_no 应被覆盖为明细行 X 的 ${itemX.color_no}（诱饵应被丢弃），实际 ${row!.color_no}`
      ).toBe(itemX.color_no);
    } finally {
      await tryCleanup(page, 'DELETE', `/bulk-color-approvals/${bulkId}`, 'bulk_color_approval');
      await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, 'sales_order');
      await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, 'customer');
      await tryCleanup(page, 'DELETE', `/products/${productX}`, 'product');
      await tryCleanup(page, 'DELETE', `/products/${productY}`, 'product');
    }
  });

  test('列表富化：GET 回读 order_no/customer_name 为关联真实值（非恒空列）', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const categoryId = ownCategoryId();
    const productX = await createOwnProduct(page, genCode('81X'), categoryId);
    const productY = await createOwnProduct(page, genCode('81Y'), categoryId);
    const dyeBatchId = ctx.dyeBatchId;

    expect(dyeBatchId, '前置：批色建单需真实染色批次 id（外键 NOT NULL）').toBeTruthy();

    const custName = `E2E批色富化客户 ${genCode('BCA-B')}`;
    const cust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: custName,
    });
    const customerId = cust.data?.id;
    expect(customerId, `创建客户失败：${JSON.stringify(cust)}`).toBeTruthy();

    const colorX = `E2E-CX-${genCode('BCBC')}`;
    const order = await createTwoLineSalesOrder(
      page,
      customerId!,
      productX,
      colorX,
      productY,
      `E2E-CY-${genCode('BCBC')}`
    );
    const orderId = order.id!;
    const realOrderNo = order.order_no!;
    expect(realOrderNo, '回读订单号非空）。').toBeTruthy();
    const itemX = order.items![0];

    let bulkId = 0;
    try {
      const created = await apiCall<BulkColorCreateResp>(page, 'POST', '/bulk-color-approvals', {
        sales_order_id: orderId,
        sales_order_item_id: itemX.id,
        dye_batch_id: dyeBatchId,
        customer_id: customerId,
      });
      bulkId = Number(created.data?.id);
      expect(bulkId, `建批色记录未返回 id：${JSON.stringify(created.data)}`).toBeGreaterThan(0);

      const row = findRowById(await readBulkColorRowsByOrder(page, orderId), bulkId);
      expect(row, `GET 列表应能回读新建批色 ${bulkId}`).toBeDefined();
      // order_no：来自父销售订单关联（FK 必存在），应等于建单时的真实订单号，证明关联名称真出值
      expect(
        row!.order_no,
        `回读 order_no 应等于真实订单号 ${realOrderNo}，实际 ${row!.order_no}`
      ).toBe(realOrderNo);
      // customer_name：来自客户关联（FK 必存在），应等于本用例新建客户的真实名称
      expect(
        row!.customer_name,
        `回读 customer_name 应等于新建客户名 ${custName}，实际 ${row!.customer_name}`
      ).toBe(custName);
      // 新建为 pending、未审批，审批人名称键恒存在且为 null（区分“无关联”与“字段缺失”）
      expect(row!.sales_order_item_id, '新建带明细行，回读该键存在').toBe(itemX.id);
      expect(row!.approval_status, `新建初始状态应为 pending，实际 ${row!.approval_status}`).toBe(
        'pending'
      );
    } finally {
      await tryCleanup(page, 'DELETE', `/bulk-color-approvals/${bulkId}`, 'bulk_color_approval');
      await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, 'sales_order');
      await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, 'customer');
      await tryCleanup(page, 'DELETE', `/products/${productX}`, 'product');
      await tryCleanup(page, 'DELETE', `/products/${productY}`, 'product');
    }
  });

  test('越权拒绝：明细行取他单或不存在均被校验拒绝，且列表不新增行', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const categoryId = ownCategoryId();
    const productX = await createOwnProduct(page, genCode('81X'), categoryId);
    const productY = await createOwnProduct(page, genCode('81Y'), categoryId);
    const dyeBatchId = ctx.dyeBatchId;

    expect(dyeBatchId, '前置：批色建单需真实染色批次 id（外键 NOT NULL）').toBeTruthy();

    const cust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `E2E批色越权客户 ${genCode('BCA-C')}`,
    });
    const customerId = cust.data?.id;
    expect(customerId, `创建客户失败：${JSON.stringify(cust)}`).toBeTruthy();

    // 本单 A（sales_order_id 取自此）
    const orderA = await createTwoLineSalesOrder(
      page,
      customerId!,
      productX,
      `E2E-CX-${genCode('BCX')}`,
      productY,
      `E2E-CY-${genCode('BCX')}`
    );
    const orderAId = orderA.id!;

    // 他单 B 的明细行（用于“行不属于所选订单”越权）
    const orderB = await createTwoLineSalesOrder(
      page,
      customerId!,
      productX,
      `E2E-CX-${genCode('BCB')}`,
      productY,
      `E2E-CY-${genCode('BCB')}`
    );
    const orderBId = orderB.id!;
    const foreignItemId = orderB.items![0].id;

    try {
      // 基线：本单 A 当前批色行数（应为 0，本单尚未建批色）
      const baselineRows = await readBulkColorRowsByOrder(page, orderAId);
      expect(baselineRows.length, '本单基线批色行数应为 0').toBe(0);

      // 越权 A1：明细行属于他单 B（行存在但 order_id 不等于所选 sales_order_id）
      const rejectForeign = await apiCallExpectFail(page, 'POST', '/bulk-color-approvals', {
        sales_order_id: orderAId,
        sales_order_item_id: foreignItemId,
        dye_batch_id: dyeBatchId,
        customer_id: customerId,
      });
      expectBusinessRejection(rejectForeign, '明细行不属于所选订单应被业务校验拒绝（400，非 5xx）');
      expect(
        failureCode(rejectForeign),
        `越权建单拒绝码应为校验族 VALIDATION_ERROR，实际 code=${rejectForeign.code}`
      ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
      expect(
        String(rejectForeign.message ?? ''),
        `越权建单应外显真实拒绝原因，实际 message=${rejectForeign.message}`
      ).toContain('明细行');

      // 越权 A2：明细行不存在（应被存在性校验拒绝，而非落库时撞外键的裸 500）
      const rejectMissing = await apiCallExpectFail(page, 'POST', '/bulk-color-approvals', {
        sales_order_id: orderAId,
        sales_order_item_id: 999_999_999,
        dye_batch_id: dyeBatchId,
        customer_id: customerId,
      });
      expectBusinessRejection(rejectMissing, '明细行不存在应被业务校验拒绝（400，非 5xx 裸崩）');
      expect(
        failureCode(rejectMissing),
        `不存在明细行拒绝码应为校验族 VALIDATION_ERROR，实际 code=${rejectMissing.code}`
      ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
      expect(
        String(rejectMissing.message ?? ''),
        `不存在明细行应外显存在性拒绝原因，实际 message=${rejectMissing.message}`
      ).toContain('不存在');

      // 拒绝后本单仍无新行（fail-closed：校验先于落库，零脏行）
      const afterRows = await readBulkColorRowsByOrder(page, orderAId);
      expect(afterRows.length, '越权拒绝后本单不应新增批色行').toBe(0);
      expect(
        afterRows.some(r => r.sales_order_item_id === foreignItemId),
        '列表不得出现挂在他单明细行上的越权批色记录'
      ).toBe(false);
    } finally {
      await tryCleanup(page, 'DELETE', `/sales/orders/${orderAId}`, 'sales_order');
      await tryCleanup(page, 'DELETE', `/sales/orders/${orderBId}`, 'sales_order');
      await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, 'customer');
      await tryCleanup(page, 'DELETE', `/products/${productX}`, 'product');
      await tryCleanup(page, 'DELETE', `/products/${productY}`, 'product');
    }
  });

  test('按行发货门禁作用域：命中未批行点名批色拦截，未命中行不被该门禁拦', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const categoryId = ownCategoryId();
    const productX = await createOwnProduct(page, genCode('81X'), categoryId);
    const productY = await createOwnProduct(page, genCode('81Y'), categoryId);
    const dyeBatchId = ctx.dyeBatchId;
    expect(dyeBatchId, '前置：批色建单需真实染色批次 id（外键 NOT NULL）').toBeTruthy();

    const cust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `E2E批色门禁客户 ${genCode('BCA-D')}`,
    });
    const customerId = cust.data?.id;
    expect(customerId, `创建客户失败：${JSON.stringify(cust)}`).toBeTruthy();

    const colorX = `E2E-CX-${genCode('BCD')}`;
    const colorY = `E2E-CY-${genCode('BCD')}`;
    const order = await createTwoLineSalesOrder(
      page,
      customerId!,
      productX,
      colorX,
      productY,
      colorY
    );
    const orderId = order.id!;
    const itemX = order.items![0];
    const itemY = order.items![1];

    // 仅对明细行 X 建一条 pending 批色（带 sales_order_item_id，反范式列由后端覆盖为行 X 值）
    let bulkId = 0;
    try {
      const created = await apiCall<BulkColorCreateResp>(page, 'POST', '/bulk-color-approvals', {
        sales_order_id: orderId,
        sales_order_item_id: itemX.id,
        dye_batch_id: dyeBatchId,
        customer_id: customerId,
      });
      bulkId = Number(created.data?.id);
      expect(bulkId, `为行 X 建批色失败：${JSON.stringify(created.data)}`).toBeGreaterThan(0);

      // 发货行 X：命中该 pending 批色行，门禁应在订单状态/库存门之前先点名批色拒绝
      const shipX = await apiCallExpectFail(page, 'POST', `/sales/orders/${orderId}/ship`, {
        order_id: orderId,
        warehouse_code: 'E2E-SHIP-WH',
        items: [{ product_id: itemX.product_id, quantity: '1', color_no: itemX.color_no }],
      });
      expectBusinessRejection(shipX, '发货命中未批行 X 应被业务拒绝（400，非 5xx）');
      const shipXCode = failureCode(shipX);
      expect(
        shipXCode === APP_ERROR_CODES.BUSINESS_ERROR,
        `命中行的发货拒绝应为业务族（批色门禁），实际 code=${shipX.code}`
      ).toBe(true);
      expect(
        String(shipX.message ?? ''),
        `命中行的发货拒绝原因应点名批色未通过审批，实际 message=${shipX.message}`
      ).toContain('批色');

      // 发货行 Y（无批色）：不被行 X 的未批记录外溢拦截。
      // 该发货仍会因后续与本门禁无关的订单状态/库存门而失败，但拒绝原因不应含“批色”字样，
      // 以此证明批色门禁按 (product_id,color_no) 命中行生效、未命中行放行于本门禁。
      const shipY = await apiCallExpectFail(page, 'POST', `/sales/orders/${orderId}/ship`, {
        order_id: orderId,
        warehouse_code: 'E2E-SHIP-WH',
        items: [{ product_id: itemY.product_id, quantity: '1', color_no: itemY.color_no }],
      });
      const shipYCode = failureCode(shipY);
      expect(
        shipY.status < 500,
        `未命中行 Y 的发货结果不应是 5xx 裸崩，实际 status=${shipY.status} code=${shipY.code}`
      ).toBe(true);
      // 放行或卡在后续与本门禁无关的门（订单状态/库存）皆可，但绝不应由批色门禁产生
      const yNotBlockByBulkColor =
        shipYCode === undefined || !String(shipY.message ?? '').includes('批色');
      expect(
        yNotBlockByBulkColor,
        `未命中行 Y 不应被批色门禁拦截（拒绝原因不得含“批色”字样），实际 status=${shipY.status} code=${shipY.code} message=${shipY.message}`
      ).toBe(true);
    } finally {
      await tryCleanup(page, 'DELETE', `/bulk-color-approvals/${bulkId}`, 'bulk_color_approval');
      await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, 'sales_order');
      await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, 'customer');
      await tryCleanup(page, 'DELETE', `/products/${productX}`, 'product');
      await tryCleanup(page, 'DELETE', `/products/${productY}`, 'product');
    }
  });
});
