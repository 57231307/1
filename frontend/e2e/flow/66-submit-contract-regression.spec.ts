import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  ensureBudgetPlan,
  getCtx,
  genCode,
  tryCleanup,
  verifyEndpointHealthy,
  failureCode,
  APP_ERROR_CODES,
} from './helpers';

/**
 * 66 提交契约族正向回归——本批修掉的"编辑和填写时一切正常，但提交和保存时报错参数错误"路径
 *
 * 每条用例的判定结构（族级锚点）：
 *   ① 提交必须被后端接受（HTTP 非 4xx——apiCall 对 code!==200 抛错并携带 status，
 *      submitMustNotBeRejected 把"提交必失败"族缺陷直接显红，禁止只看"没抛错"）；
 *   ② 提交后用**详情/列表端点回读**断言落库真值 === 提交值（只断 200 属假绿：
 *      键名错/字段被 service 吞掉时 200 也成立，但库里是默认值/NULL）；
 *   ③ rust_decimal 出参按字符串序列化，所有数值断言先 Number() 归一。
 *
 * 端点与 DTO 真实性核实（backend 只读 grep，逐条对过源码，非臆造）：
 * - POST /api/v1/erp/crm/customer-credits                    routes/crm.rs:93-94 → customer_credit_handler::create_credit
 *   入参 CreditRatingRequestDto（handler:37-49：customer_id/credit_level/credit_score/credit_limit/credit_days/remark）
 * - POST /api/v1/erp/crm/customer-credits/{id}/occupy|release routes/crm.rs:113-118；
 *   入参 CreditAmountRequest（handler:82-85）**只有一个 `amount` 键**；路径 {id} 即 customer_id（handler:167/193）
 * - GET  /api/v1/erp/crm/customer-credits/{customer_id}      routes/crm.rs:97；出参 customer_credit::Model 单对象
 * - POST /api/v1/erp/custom-orders/{id}/after-sales          routes/custom_order.rs:72-76；
 *   CreateAfterSalesDto（services/custom_order_aftersales_service.rs:34-46）已无 custom_order_id（任务 #148：
 *   归属由 path 权威提供，body 携带未知键被 serde 忽略）；issue_type ∈ complaint/repair/exchange/return_goods/refund（service:105-112）
 * - GET  /api/v1/erp/custom-orders/{id}/after-sales          出参 PagedResponse{items,total,page,page_size}（handler:738）
 * - POST /api/v1/erp/sales/sales-contracts                   routes/sales.rs:136-139；
 *   CreateSalesContractRequestDto（sales_contract_handler.rs:43-60：delivery_date 等可空列已改 Option、
 *   补齐 signed_date/effective_date/expiry_date/payment_method/delivery_location 真实列；items 明细随行提交）
 * - GET  /api/v1/erp/sales/sales-contracts/{id}/items        routes/sales.rs:170；出参 sales_contract_item::Model **裸数组**
 *   （handler:230-242，本端点无分页信封；quantity_tolerance_pct 为 DECIMAL(5,2) 可空列，models/sales_contract_item.rs:30）
 * - POST /api/v1/erp/purchase/purchase-contracts             routes/purchase.rs:231；CreateContractRequestDto
 *   （purchase_contract_handler.rs:40-54）**该域无明细 items 字段**——"采购侧明细"真实载体是采购订单行
 *   POST /api/v1/erp/purchase/orders（services/po/mod.rs:33-79 CreatePurchaseOrderRequest，
 *   行级 quantity_tolerance_pct Some 时 [0,100]：po/mod.rs:123/196-204）与 GET /purchase/orders/{id}/items
 *   （purchase_order_handler.rs:378-387 → services/po/order_ops/query.rs:21-46 出参
 *   **Vec<PurchaseOrderItemDto>**（services/po/order.rs:63-90），读键以 DTO rename 为准：
 *   material_id=实体列 product_id（order.rs:65）、quantity_ordered=实体列 quantity（order.rs:71），
 *   unit_price/quantity_tolerance_pct 无改名——#4671 判责 66-04 读实体原名 quantity/product_id 得 NaN 属用例读键错）
 * - PUT  /api/v1/erp/departments/{id}                        routes/iam.rs:68；UpdateDepartmentRequest 已含
 *   code: Option<String>（department_handler.rs:47，P0 契约修复：原缺字段⇒前端编辑编码被静默丢弃），
 *   service 侧非空才覆盖并查重排除自身（services/department_service.rs:247-263）
 * - POST /api/v1/erp/purchase/suppliers                      routes/purchase.rs:313；CreateSupplierRequest
 *   supplier_short_name: Option + length(2..100)、credit_code: Option + length(equal=18)
 *   （services/supplier_service.rs:1064/1067——validator 对 Option 解包：缺省=None 跳过，Some(非法值) 拒绝）
 *
 * 假绿防线：所有读回端点先 verifyEndpointHealthy strict 探针（禁吞 404/403）；信封显式钉桩
 * （数组端点断 Array.isArray、分页端点断 items 数组），禁 `?? []` 与双形状探测；
 * 全部自建自流转数据（客户/产品/定制订单/合同/部门/供应商逐例自造），不依赖其它用例残留。
 */

type Row = Record<string, unknown>;

/** ① 提交锚点：断言响应不是 4xx/5xx。apiCall 对非 2xx/非 code=200 抛错并携带 HTTP status，
 *  这里统一转成带"族级回归锚点"说明的显式失败，禁止把抛错当环境噪音吞掉。 */
async function submitMustNotBeRejected<T = Row>(
  page: Page,
  method: 'POST' | 'PUT',
  path: string,
  body: Record<string, unknown>,
  what: string
): Promise<{ code: number; message?: string; data: T }> {
  try {
    const res = await apiCall<T>(page, method, path, body);
    expect(
      res.code,
      `${what}：提交应被后端接受（code=200）。本缺陷族症状即"填写正常、提交报参数错误"，此处判红`
    ).toBe(200);
    return res;
  } catch (e) {
    const err = e as Error & { status?: number };
    throw new Error(
      `${what}：提交得到 HTTP status=${err.status ?? '未知(非JSON/网络异常)'}，` +
        `族级回归锚点要求"非 4xx/5xx"。原始错误：${err.message}`
    );
  }
}

/** 数组信封钉桩：data 本身必须是数组（裸数组出参端点），缺键/变形即红，不兜底 */
function requireArray(data: unknown, endpoint: string): Row[] {
  expect(
    Array.isArray(data),
    `${endpoint}：出参 data 必须为数组（handler 直接 to_value(Vec<Model>)），实际 ${JSON.stringify(data)}`
  ).toBe(true);
  return data as Row[];
}

/** 分页信封钉桩：唯一形状 PaginatedResponse{items,total,page,page_size} */
function requireItemsEnvelope(data: unknown, endpoint: string): { items: Row[]; total: number } {
  const env = data as { items?: unknown; total?: unknown; page?: unknown; page_size?: unknown };
  expect(
    Array.isArray(env?.items),
    `${endpoint}：分页唯一形状为 {items,total,page,page_size}，items 必须为数组，实际 ${JSON.stringify(data)}`
  ).toBe(true);
  expect(
    typeof env?.total,
    `${endpoint}：total 必须存在且为数字（禁 ?? 0 兜底），实际 ${JSON.stringify(data)}`
  ).toBe('number');
  return { items: env.items as Row[], total: Number(env.total) };
}

/** 按主键在回读结果里定位自建记录，找不到即红（禁静默过滤成"当它不存在"） */
function findRowById(items: Row[], id: number, endpoint: string): Row {
  const row = items.find(it => Number(it.id) === id);
  expect(
    row,
    `${endpoint}：应回读到自建记录 id=${id}，实际 items=${JSON.stringify(items)}`
  ).toBeTruthy();
  return row as Row;
}

/** 自建客户（POST /crm/customers，65 号 spec 同款真实契约：customer_code/customer_name） */
async function seedCustomer(page: Page, tag: string): Promise<number> {
  const code = genCode(`E2E66C`);
  const res = await submitMustNotBeRejected<Row>(
    page,
    'POST',
    '/crm/customers',
    {
      customer_code: code,
      customer_name: `66号${tag}客户_${code}`,
      contact_phone: '13800006601',
    },
    `[66] 自建客户(${tag})`
  );
  const id = Number(res.data?.id);
  expect(id, `[66] 客户创建应回 id：${JSON.stringify(res)}`).toBeGreaterThan(0);
  return id;
}

/** 自建产品（POST /products，与 65 号 spec 同款：code/name/unit） */
async function seedProduct(page: Page, tag: string): Promise<number> {
  const code = genCode(`E2E66P`);
  const res = await submitMustNotBeRejected<Row>(
    page,
    'POST',
    '/products',
    { code, name: `66号${tag}布_${code}`, unit: 'm' },
    `[66] 自建产品(${tag})`
  );
  const id = Number(res.data?.id);
  expect(id, `[66] 产品创建应回 id：${JSON.stringify(res)}`).toBeGreaterThan(0);
  return id;
}

const today = (): string => new Date().toISOString().slice(0, 10);

test.describe.serial('66 提交契约族正向回归（提交必失败路径 → 提交成功且落库真值一致）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('66-01 信用占用/释放：body 只有一个 amount 键 → used/available 落库真值回读', async ({
    page,
  }) => {
    // 修复前形态：前端把整个 Partial<CreditAdjustment>（含 credit_amount/adjustment_type 等）当占用载荷，
    // 而后端 CreditAmountRequest 只有 `amount`（customer_credit_handler.rs:82-85）→ 提交必报参数错误。
    const customerId = await seedCustomer(page, '信用');
    const limit = '5000.00';
    const created = await submitMustNotBeRejected<Row>(
      page,
      'POST',
      '/crm/customer-credits',
      { customer_id: customerId, credit_level: 'A', credit_limit: limit, credit_days: 30 },
      `[66-01] 创建客户信用(customer=${customerId})`
    );
    expect(
      Number(created.data?.id),
      `[66-01] 信用创建应回 id：${JSON.stringify(created)}`
    ).toBeGreaterThan(0);

    const detailEp = `/crm/customer-credits/${customerId}`;
    await verifyEndpointHealthy(page, detailEp);

    // 占用 1200.50：载荷只有 amount 一个键
    await submitMustNotBeRejected(
      page,
      'POST',
      `/crm/customer-credits/${customerId}/occupy`,
      { amount: '1200.50' },
      '[66-01] 信用占用提交'
    );
    let credit = await apiCallRaw<Row>(page, 'GET', detailEp);
    expect(Number(credit.credit_limit), '占用回读：总额度落库=提交值 5000').toBe(5000);
    expect(Number(credit.used_credit), '占用回读：已用额度落库=占用额 1200.50').toBe(1200.5);
    expect(Number(credit.available_credit), '占用回读：可用额度=5000-1200.50=3799.50').toBe(3799.5);

    // 释放 200.50：部分释放，验证释放真实参与算术（而非"释放恒置 0"或忽略）
    await submitMustNotBeRejected(
      page,
      'POST',
      `/crm/customer-credits/${customerId}/release`,
      { amount: '200.50' },
      '[66-01] 信用释放提交'
    );
    credit = await apiCallRaw<Row>(page, 'GET', detailEp);
    expect(Number(credit.used_credit), '释放回读：已用=1200.50-200.50=1000.00').toBe(1000);
    expect(Number(credit.available_credit), '释放回读：可用=5000-1000=4000.00').toBe(4000);

    await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, '[66-01] 客户');
  });

  test('66-02 客诉售后工单创建：归属由 path 提供、body 不带 custom_order_id → 列表回读真值', async ({
    page,
  }) => {
    // 修复前形态：CreateAfterSalesDto.custom_order_id 非 Option 必填，前端从不携带
    // → 反序列化 missing field 报"参数错误"，创建必失败（任务 #148，见 service:23-33 注释）。
    const customerId = await seedCustomer(page, '售后');
    const productId = await seedProduct(page, '售后');
    const orderBody = (spec: string) => ({
      customer_id: customerId,
      product_id: productId,
      spec,
      quantity: '120',
      unit: 'm',
      total_amount: '18000.50',
      currency: 'CNY',
    });
    const order1 = await submitMustNotBeRejected<Row>(
      page,
      'POST',
      '/custom-orders',
      orderBody('E2E66/SPEC/ONE'),
      '[66-02] 定制订单1创建'
    );
    const orderId1 = Number(order1.data?.id);
    const order2 = await submitMustNotBeRejected<Row>(
      page,
      'POST',
      '/custom-orders',
      orderBody('E2E66/SPEC/TWO'),
      '[66-02] 定制订单2创建'
    );
    const orderId2 = Number(order2.data?.id);
    expect(orderId1).toBeGreaterThan(0);
    expect(orderId2).toBeGreaterThan(0);

    const description = `E2E66_客诉描述_${genCode('DSC')}`;
    // body 只按新契约提交归属之外的字段；键名与 CreateAfterSalesDto（service:34-46）逐一对过
    const created = await submitMustNotBeRejected<Row>(
      page,
      'POST',
      `/custom-orders/${orderId1}/after-sales`,
      {
        customer_id: customerId,
        issue_type: 'complaint',
        description,
        reason_category: 'quality',
        reason_detail: 'E2E66 染色牢度不达标客诉',
      },
      '[66-02] 客诉售后工单创建（body 无 custom_order_id）'
    );
    const afterSalesId = Number(created.data?.id);
    expect(afterSalesId, `[66-02] 工单创建应回 id：${JSON.stringify(created)}`).toBeGreaterThan(0);

    // 归属伪造锚点：body 带 custom_order_id=订单2 也必须被忽略（serde 忽略未知字段，path 唯一权威）
    await submitMustNotBeRejected<Row>(
      page,
      'POST',
      `/custom-orders/${orderId1}/after-sales`,
      {
        customer_id: customerId,
        issue_type: 'repair',
        description: `E2E66_维修伪造归属_${genCode('FRG')}`,
        custom_order_id: orderId2,
      },
      '[66-02] 第二工单创建（body 伪造 custom_order_id 应被忽略）'
    );

    const listEp1 = `/custom-orders/${orderId1}/after-sales?page=1&page_size=50`;
    await verifyEndpointHealthy(page, listEp1);
    const listed = await apiCallRaw(page, 'GET', listEp1);
    const { items, total } = requireItemsEnvelope(listed, listEp1);
    expect(total, '订单1 的售后列表 total 应=2（两条工单均归属 path 指定的订单1）').toBe(2);
    const row = findRowById(items, afterSalesId, listEp1);
    expect(row.issue_type, '回读：客诉类型落库=complaint').toBe('complaint');
    expect(row.description, '回读：描述落库=提交值（非空/非默认）').toBe(description);
    expect(row.status, '回读：初始状态=opened（service 写入方词表，小写）').toBe('opened');

    // 归属隔离回读：伪造 body 未改变归属——订单2 仍 0 条
    const listEp2 = `/custom-orders/${orderId2}/after-sales?page=1&page_size=50`;
    await verifyEndpointHealthy(page, listEp2);
    const listed2 = await apiCallRaw(page, 'GET', listEp2);
    const env2 = requireItemsEnvelope(listed2, listEp2);
    expect(
      env2.total,
      `归属必须由 path 权威提供：订单2 列表应 total=0（body 伪造 custom_order_id 被忽略），实际 ${JSON.stringify(listed2)}`
    ).toBe(0);

    // 定制订单 DELETE 走 cancel_custom_order（handler:379-387），Json<CancelCustomOrderDto> 必填 reason，
    // tryCleanup 不携带 body 会 400——清理必须按真实契约带体。
    const cancelOrder = (id: number, label: string) =>
      apiCall(page, 'DELETE', `/custom-orders/${id}`, { reason: 'E2E66 用例清理' }).catch(e =>
        console.warn(`[cleanup] ${label} 失败: ${(e as Error).message}`)
      );
    await cancelOrder(orderId1, '[66-02] 定制订单1');
    await cancelOrder(orderId2, '[66-02] 定制订单2');
    await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, '[66-02] 客户');
  });

  test('66-03 销售合同含明细提交：表头补齐列 + 明细行 quantity_tolerance_pct 落库回读', async ({
    page,
  }) => {
    // 修复前形态：delivery_date 非 Option（真实列可空）⇒ 未填即反序列化失败 400"参数错误"；
    // 表头 signed_date/effective_date/expiry_date/payment_method/delivery_location 被 service 吞掉；
    // 明细行 quantity_tolerance_pct 键名/范围契约同批对齐（sales_contract_handler.rs:43-93）。
    const customerId = await seedCustomer(page, '销合');
    const productId = await seedProduct(page, '销合');
    const contractNo = genCode('E2E66SC');
    const items = [
      {
        product_id: productId,
        product_name: 'E2E66 检验坯布',
        product_spec: 'E2E66/160g/150cm',
        unit: 'm',
        quantity: '500.00',
        quantity_tolerance_pct: '10.00',
        unit_price: '12.34',
        delivery_date: '2026-10-05',
        remarks: 'E2E66 行备注',
      },
    ];
    const created = await submitMustNotBeRejected<Row>(
      page,
      'POST',
      '/sales/sales-contracts',
      {
        contract_no: contractNo,
        contract_name: `66号销售合同_${contractNo}`,
        customer_id: customerId,
        total_amount: '12345.60',
        contract_type: 'sale',
        payment_terms: '月结30天',
        delivery_date: '2026-10-15',
        signed_date: '2026-09-01',
        effective_date: '2026-09-05',
        expiry_date: '2026-12-31',
        payment_method: 'BANK_TRANSFER',
        delivery_location: 'E2E66 仓库门口',
        remark: 'E2E66 表头备注',
        items,
      },
      '[66-03] 销售合同（含明细）提交'
    );
    const contractId = Number(created.data?.id);
    expect(contractId, `[66-03] 合同创建应回 id：${JSON.stringify(created)}`).toBeGreaterThan(0);

    const detailEp = `/sales/sales-contracts/${contractId}`;
    await verifyEndpointHealthy(page, detailEp);
    const detail = await apiCallRaw<Row>(page, 'GET', detailEp);
    expect(detail.contract_no, '表头回读：contract_no').toBe(contractNo);
    expect(
      Number(detail.total_amount),
      '表头回读：total_amount 落库=12345.60（Decimal 字符串需 Number 归一）'
    ).toBe(12345.6);
    expect(detail.delivery_date, '表头回读：可空日期列 delivery_date 真实落库').toBe('2026-10-15');
    expect(detail.signed_date, '表头回读：补齐列 signed_date 不再被吞').toBe('2026-09-01');
    expect(detail.effective_date, '表头回读：补齐列 effective_date').toBe('2026-09-05');
    expect(detail.expiry_date, '表头回读：补齐列 expiry_date').toBe('2026-12-31');
    expect(detail.payment_method, '表头回读：补齐列 payment_method').toBe('BANK_TRANSFER');
    expect(detail.delivery_location, '表头回读：补齐列 delivery_location').toBe('E2E66 仓库门口');
    expect(detail.remark, '表头回读：remark（真实列名，非前端自造 remarks）').toBe(
      'E2E66 表头备注'
    );

    const itemsEp = `/sales/sales-contracts/${contractId}/items`;
    await verifyEndpointHealthy(page, itemsEp);
    const itemsRes = await apiCallRaw(page, 'GET', itemsEp);
    const itemRows = requireArray(itemsRes, itemsEp);
    expect(itemRows.length, '明细回读应恰有 1 行').toBe(1);
    const it0 = itemRows[0];
    expect(it0.product_name, '明细回读：product_name').toBe('E2E66 检验坯布');
    expect(Number(it0.product_id), '明细回读：product_id 落库=提交产品').toBe(productId);
    expect(it0.product_spec, '明细回读：product_spec').toBe('E2E66/160g/150cm');
    expect(it0.unit, '明细回读：unit').toBe('m');
    expect(Number(it0.quantity), '明细回读：quantity=500.00').toBe(500);
    expect(Number(it0.unit_price), '明细回读：unit_price=12.34').toBe(12.34);
    expect(
      Number(it0.quantity_tolerance_pct),
      `明细回读：quantity_tolerance_pct 落库=10.00（NULL/被吞即本族缺陷复发），实际 ${JSON.stringify(it0.quantity_tolerance_pct)}`
    ).toBe(10);
    expect(Number(it0.amount), '明细回读：amount 派生列=数量×单价=6170.00').toBe(6170);
    expect(it0.delivery_date, '明细回读：行交货日期').toBe('2026-10-05');
    expect(it0.remarks, '明细回读：行备注').toBe('E2E66 行备注');

    await tryCleanup(page, 'DELETE', `/sales/sales-contracts/${contractId}`, '[66-03] 销售合同');
    await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, '[66-03] 客户');
  });

  test('66-04 采购合同表头提交 + 采购订单明细行 quantity_tolerance_pct 落库回读', async ({
    page,
  }) => {
    // 采购合同 CreateContractRequestDto（purchase_contract_handler.rs:40-54）**没有明细 items 字段**，
    // 采购侧"明细提交"的真实契约载体是采购订单行（CreateOrderItemRequest.quantity_tolerance_pct，
    // services/po/mod.rs:123 + 行级 [0,100] 校验 po/mod.rs:196-204），本用例分别按真实载体钉桩。
    const ctx = getCtx();
    const supplierCode = genCode('E2E66SUP');
    const supplier = await submitMustNotBeRejected<Row>(
      page,
      'POST',
      '/purchase/suppliers',
      {
        supplier_name: `66号采购供应商_${supplierCode}`,
        supplier_short_name: '66供',
        contact_phone: '13800006604',
      },
      '[66-04] 自建供应商'
    );
    const supplierId = Number(supplier.data?.id);
    expect(supplierId, `[66-04] 供应商创建应回 id：${JSON.stringify(supplier)}`).toBeGreaterThan(0);

    // ---- 采购合同表头（含本批补齐的可空日期列与 Option 列）----
    const pcNo = genCode('E2E66PC');
    const pc = await submitMustNotBeRejected<Row>(
      page,
      'POST',
      '/purchase/purchase-contracts',
      {
        contract_no: pcNo,
        contract_name: `66号采购合同_${pcNo}`,
        supplier_id: supplierId,
        total_amount: '8888.00',
        contract_type: 'purchase',
        payment_terms: '货到付款',
        delivery_date: '2026-11-01',
        signed_date: '2026-09-10',
        effective_date: '2026-09-10',
        expiry_date: '2026-12-10',
        payment_method: 'CHECK',
        delivery_location: 'E2E66 厂区',
        remark: 'E2E66 采购备注',
      },
      '[66-04] 采购合同提交'
    );
    const pcId = Number(pc.data?.id);
    expect(pcId, `[66-04] 采购合同应回 id：${JSON.stringify(pc)}`).toBeGreaterThan(0);
    const pcEp = `/purchase/purchase-contracts/${pcId}`;
    await verifyEndpointHealthy(page, pcEp);
    const pcRow = await apiCallRaw<Row>(page, 'GET', pcEp);
    expect(pcRow.contract_no, '采购合同回读：contract_no').toBe(pcNo);
    expect(Number(pcRow.supplier_id), '采购合同回读：supplier_id').toBe(supplierId);
    expect(Number(pcRow.total_amount), '采购合同回读：total_amount=8888.00').toBe(8888);
    expect(pcRow.signed_date, '采购合同回读：补齐列 signed_date 落库').toBe('2026-09-10');
    expect(pcRow.effective_date, '采购合同回读：补齐列 effective_date').toBe('2026-09-10');
    expect(pcRow.expiry_date, '采购合同回读：补齐列 expiry_date').toBe('2026-12-10');
    expect(pcRow.payment_method, '采购合同回读：补齐列 payment_method').toBe('CHECK');
    expect(pcRow.delivery_location, '采购合同回读：补齐列 delivery_location').toBe('E2E66 厂区');
    expect(pcRow.remark, '采购合同回读：remark').toBe('E2E66 采购备注');

    // ---- 采购订单明细行（quantity_tolerance_pct=8 在 [0,100] 内，提交必须被接受并落库）----
    // 后端订单创建门控（crud.rs）：warehouse_id/department_id 必填、预算方案门控（V15 P0-B06），
    // 沿用仓内既有范式 ensureBudgetPlan + ctx 仓库/部门；物料用自建产品（material_id→product_id 列）。
    expect(ctx.warehouseIds.length, '前置：仓库').toBeGreaterThanOrEqual(1);
    expect(ctx.departmentIds.length, '前置：部门').toBeGreaterThanOrEqual(1);
    await ensureBudgetPlan(page);
    const productId = await seedProduct(page, '采购行');
    const po = await submitMustNotBeRejected<Row>(
      page,
      'POST',
      '/purchase/orders',
      {
        supplier_id: supplierId,
        warehouse_id: ctx.warehouseIds[0],
        department_id: ctx.departmentIds[0],
        order_date: today(),
        items: [
          {
            material_id: productId,
            quantity_ordered: '300.00',
            unit_price: '20.00',
            quantity_tolerance_pct: '8.00',
            notes: 'E2E66 采购行',
          },
        ],
      },
      '[66-04] 采购订单（含明细行）提交'
    );
    const poId = Number(po.data?.id);
    expect(poId, `[66-04] 采购订单应回 id：${JSON.stringify(po)}`).toBeGreaterThan(0);

    const poItemsEp = `/purchase/orders/${poId}/items`;
    await verifyEndpointHealthy(page, poItemsEp);
    const poItems = requireArray(await apiCallRaw(page, 'GET', poItemsEp), poItemsEp);
    expect(poItems.length, '采购明细回读应恰有 1 行').toBe(1);
    const row0 = poItems[0];
    expect(
      Number(row0.quantity_tolerance_pct),
      `采购明细回读：quantity_tolerance_pct 落库=8.00（被吞/键名错即本族缺陷复发），实际 ${JSON.stringify(row0.quantity_tolerance_pct)}`
    ).toBe(8);
    // 读键=出参 DTO rename 后真键（services/po/order.rs:71 #[serde(rename =
    // "quantity_ordered")] pub quantity、:65 #[serde(rename = "material_id")] pub product_id）。
    // 读实体原名 quantity/product_id 恒 undefined→NaN，属本用例读键笔误（同 purchase/13-:182
    // 族，#4671 判责 ②），后端出参契约无错——不加 ?? 兜底、不改期望值。
    expect(Number(row0.quantity_ordered), '采购明细回读：quantity_ordered=300.00').toBe(300);
    expect(Number(row0.unit_price), '采购明细回读：unit_price=20.00').toBe(20);
    expect(Number(row0.material_id), '采购明细回读：material_id（DTO 键，落 product_id 列）').toBe(
      productId
    );
    expect(row0.notes, '采购明细回读：notes').toBe('E2E66 采购行');

    await tryCleanup(page, 'DELETE', `/purchase/orders/${poId}`, '[66-04] 采购订单');
    await tryCleanup(page, 'DELETE', `/purchase/purchase-contracts/${pcId}`, '[66-04] 采购合同');
    await tryCleanup(page, 'DELETE', `/purchase/suppliers/${supplierId}`, '[66-04] 供应商');
  });

  test('66-05 部门编码更新：code 必须真的生效（修复前编辑编码被静默丢弃）', async ({ page }) => {
    const codeA = genCode('E2E66DA');
    const deptName = `66号编码回归部门_${codeA}`;
    const created = await submitMustNotBeRejected<Row>(
      page,
      'POST',
      '/departments',
      { name: deptName, code: codeA },
      '[66-05] 部门创建'
    );
    const deptId = Number(created.data?.id);
    expect(deptId, `[66-05] 部门创建应回 id：${JSON.stringify(created)}`).toBeGreaterThan(0);

    const detailEp = `/departments/${deptId}`;
    await verifyEndpointHealthy(page, detailEp);
    let dept = await apiCallRaw<Row>(page, 'GET', detailEp);
    expect(
      dept.code,
      `创建回读：显式 code 应落库为提交值（而非后端自动生成 DEPT_时间戳），实际 ${JSON.stringify(dept.code)}`
    ).toBe(codeA);

    const codeB = genCode('E2E66DB');
    await submitMustNotBeRejected(
      page,
      'PUT',
      `/departments/${deptId}`,
      { code: codeB },
      '[66-05] 部门编码更新'
    );
    dept = await apiCallRaw<Row>(page, 'GET', detailEp);
    expect(
      dept.code,
      `更新回读：code 必须真实改为 ${codeB}——修复前 UpdateDepartmentRequest 无 code 字段，编辑编码被静默丢弃`
    ).toBe(codeB);
    expect(dept.name, '更新回读：未提交 name → 应保持原值（禁止洗 NULL）').toBe(deptName);

    await tryCleanup(page, 'DELETE', `/departments/${deptId}`, '[66-05] 部门');
  });

  test('66-06 供应商简称/统一社会信用代码：Option+length 契约提交成功且落库值一致', async ({
    page,
  }) => {
    // 后端契约（supplier_service.rs:1064/1067）：supplier_short_name=Option+length(2..100)、
    // credit_code=Option+length(equal=18)。修复前形态二选一：前端不采集→提交缺键被必填旧 DTO 拒；
    // 或采集后把空串直发→Some("x") 撞长度校验。本例按修复后契约提交合法值，断言真实落库。
    const code = genCode('E2E66SUP');
    const shortName = 'E2E66简称布行';
    const creditCode = '91310000MA1K3AB0X9'; // 恰 18 位
    const created = await submitMustNotBeRejected<Row>(
      page,
      'POST',
      '/purchase/suppliers',
      {
        supplier_name: `66号契约供应商_${code}`,
        supplier_short_name: shortName,
        credit_code: creditCode,
        contact_phone: '13800006606',
      },
      '[66-06] 供应商提交（简称+18位信用代码）'
    );
    const id = Number(created.data?.id);
    expect(id, `[66-06] 供应商创建应回 id：${JSON.stringify(created)}`).toBeGreaterThan(0);

    const detailEp = `/purchase/suppliers/${id}`;
    await verifyEndpointHealthy(page, detailEp);
    const row = await apiCallRaw<Row>(page, 'GET', detailEp);
    expect(
      row.supplier_short_name,
      `回读：supplier_short_name 落库=提交值，实际 ${JSON.stringify(row.supplier_short_name)}`
    ).toBe(shortName);
    expect(
      row.credit_code,
      `回读：credit_code 落库=18 位提交值，实际 ${JSON.stringify(row.credit_code)}`
    ).toBe(creditCode);

    await tryCleanup(page, 'DELETE', `/purchase/suppliers/${id}`, '[66-06] 供应商');
  });

  test('66-07 销售合同拒绝链：reject 端点理由必填落 rejected_reason + 空理由/缺通过理由/状态门负例', async ({
    page,
  }) => {
    // 销售合同与采购合同同族（contract 小写 draft/active/rejected/cancelled）。此前销售侧仅 66-03
    // 建单落库正向、无拒绝链真断言。本例补 reject 出边与理由列逐字回读（防"只测状态"半级假绿）。
    const customerId = await seedCustomer(page, '销拒');
    const productId = await seedProduct(page, '销拒');
    const createdIds: number[] = [];
    const mkContract = async (): Promise<number> => {
      const contractNo = genCode('E2E66SCR');
      const created = await apiCall<Row>(page, 'POST', '/sales/sales-contracts', {
        contract_no: contractNo,
        contract_name: `66号销售拒绝合同_${contractNo}`,
        customer_id: customerId,
        total_amount: '1000.00',
        contract_type: 'sale',
        payment_terms: '月结30天',
        items: [
          {
            product_id: productId,
            product_name: 'E2E66 拒绝用例产品',
            unit: 'm',
            quantity: '10.00',
            unit_price: '100.00',
          },
        ],
      });
      const cid = Number(created.data?.id);
      expect(cid, `销售合同建单应回 id：${JSON.stringify(created)}`).toBeGreaterThan(0);
      createdIds.push(cid);
      return cid;
    };

    // —— 正例：带理由拒绝 draft → rejected + rejected_reason 逐字回读，approval_reason 不被挪用
    const idReject = await mkContract();
    const rejectReason = `E2E-SC-拒绝-${Date.now()}`;
    await apiCall(page, 'POST', `/sales/sales-contracts/${idReject}/reject`, {
      reason: rejectReason,
    });
    const after = await apiCallRaw<{
      status: string;
      rejected_reason: string | null;
      approval_reason: string | null;
    }>(page, 'GET', `/sales/sales-contracts/${idReject}`);
    expect(after.status, '拒绝后状态应落库为 rejected').toBe('rejected');
    expect(after.rejected_reason, '拒绝理由应逐字落 rejected_reason 专列').toBe(rejectReason);
    expect(after.approval_reason, '拒绝不得写 approval_reason（两动作两列）').toBeNull();

    // —— 状态门：rejected 终态再拒绝 → 400 BUSINESS_ERROR（reject 仅 draft 起拒）
    const repeatReject = await apiCallExpectFail(
      page,
      'POST',
      `/sales/sales-contracts/${idReject}/reject`,
      { reason: 'E2E-SC-重复拒绝' }
    );
    expect(repeatReject.status, '已拒绝合同再次拒绝应被状态门拦为 400').toBe(400);
    expect(failureCode(repeatReject), '重复拒绝机器码应为 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );

    // —— 负例：空/纯空白拒绝理由 → 400 VALIDATION_ERROR（reason 服务端必填）
    const idEmpty = await mkContract();
    const emptyReason = await apiCallExpectFail(
      page,
      'POST',
      `/sales/sales-contracts/${idEmpty}/reject`,
      {
        reason: '   ',
      }
    );
    expect(emptyReason.status, '纯空白拒绝理由应返回 HTTP 400').toBe(400);
    expect(failureCode(emptyReason), '纯空白拒绝理由机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    const stillDraft = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/sales/sales-contracts/${idEmpty}`
    );
    expect(stillDraft.status, '空理由 reject 被拒不得改变状态').toBe('draft');

    // —— 负例：approve 缺通过理由 → 400 VALIDATION_ERROR（必填档）
    const approveNoReason = await apiCallExpectFail(
      page,
      'POST',
      `/sales/sales-contracts/${idEmpty}/approve`
    );
    expect(approveNoReason.status, 'approve 缺 approval_reason 应返回 HTTP 400').toBe(400);
    expect(failureCode(approveNoReason), 'approve 缺理由机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );

    // 尽力清理（rejected 合同删除可能命中状态门告警，属预期，与既有流转型用例清理同则）
    for (const cid of createdIds) {
      await tryCleanup(page, 'DELETE', `/sales/sales-contracts/${cid}`, `[66-07] 销售合同#${cid}`);
    }
    await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, '[66-07] 客户');
  });
});
