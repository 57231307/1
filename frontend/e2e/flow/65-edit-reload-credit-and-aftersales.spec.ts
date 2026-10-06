import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall, ensureTestEntities, genCode } from './helpers';

/**
 * 65 定制订单售后工单「编辑-重开回读」闭环（用户点名缺陷族：创建时填写了完整内容，
 * 保存后再次编辑，则前面保存的内容不完整）
 *
 * 端点契约（backend 只读核实，route-snapshot 旁证：424/1202/1204/1651 行）：
 * - POST /api/v1/erp/custom-orders                                        routes/mod.rs:466 nest；handler:197
 * - POST /api/v1/erp/custom-orders/{id}/after-sales                       routes/custom_order.rs:72-76；handler:687
 * - GET  /api/v1/erp/custom-orders/{id}/after-sales                       data = PagedResponse{items,total,page,page_size}
 *                                                                          （custom_order_response_dto.rs:152，非裸数组）
 * - PUT  /api/v1/erp/custom-orders/after-sales/{id}                       routes/custom_order.rs:77-80；handler:747
 * - GET  /api/v1/erp/custom-orders/{id}                                   详情内嵌 after_sales: AfterSalesInfo[]（handler:281）
 *
 * 创建入参 CreateAfterSalesDto（services/custom_order_dto.rs:34-46）：
 *   customer_id / issue_type ∈ complaint|repair|exchange|return_goods|refund（service:105-112）
 *   / description / refund_amount（refund 类型必填，service:115-119）
 *   / quality_issue_id / reason_category ∈ quality|logistics|customer_preference|other
 *   / reason_detail
 * PUT 入参 UpdateAfterSalesDto（custom_order_dto.rs:50-54）仅 status/resolution/refund_amount；
 *   service.update 基于既有整行 ActiveModel 逐 Some 覆盖（service:166-180）→ 未提交列应保留原值。
 * 状态机（service:406-418）：opened→accepted/rejected/closed（小写 token）。
 *
 * 售后记录无附件/备注类可断言字段之外的落库真值读回通道：本域唯一读回端点是
 * AfterSalesInfo（models/custom_order_response_dto.rs:105-126，13 键，含 JOIN 富化 customer_name）。
 *
 * 读回判据（2026-10 对源码重核，处置 a——此前本节声称 customer_id/reason_* "DTO 无键→读回恒缺键
 * 必红"，该前提已过时）：当前源码 AfterSalesInfo（backend/src/models/custom_order_response_dto.rs:105-126）
 * **含** customer_id / reason_category / reason_detail 全部三键；列表与详情内嵌均经
 * into_model::<AfterSalesInfo> 的 LEFT JOIN 富化单次查询产出（handlers/custom_order_handler.rs:259、
 * 289-291、726，map_after_sales 已不存在）。因此下方对三键的断言是**真实回读判据**（缺键/被吞即红），
 * 不再是已知红；若 CI 仍在这三处判红，即为读侧富化链回归的源码缺陷，保持红交后端，禁止放宽。
 *   （注：售后域**不存在**附件字段——after_sales::Model（models/after_sales.rs:9-37）无
 *   attachment* 列，链路各层 grep attachment 仅命中 process log（handler:547/781），
 *   故本 spec 不含附件断言；此前「售后附件」调查描述在本域无载体，属用例编写错误，已剔除。）
 */

/** 出参行类型：读端点 JSON 一律显式建模（snake_case，禁 ?? [] 与双形状探测） */
type AfterSalesRow = {
  id?: number;
  issue_type?: string;
  description?: string;
  status?: string;
  resolution?: string | null;
  refund_amount?: string | null;
  quality_issue_id?: number | null;
  opened_at?: string;
  closed_at?: string | null;
  // customer_id/reason_category/reason_detail：DTO 现含三键（见文件头重核说明），
  // 按真实回读判据断言，读回缺键/值不符即红
  customer_id?: unknown;
  reason_category?: unknown;
  reason_detail?: unknown;
};

type PagedAfterSales = {
  items?: AfterSalesRow[];
  total?: number;
  page?: number;
  page_size?: number;
};

type CustomOrderDetail = {
  id?: number;
  order_no?: string;
  after_sales?: AfterSalesRow[];
};

/** 信封显式钉桩：data.items 必须为数组（PagedResponse 唯一形状），缺键即抛，不兜底 */
function requireItems(data: PagedAfterSales | undefined | null, endpoint: string): AfterSalesRow[] {
  const items = data?.items;
  expect(
    Array.isArray(items),
    `${endpoint} 响应 data.items 必须为数组（PagedResponse{items,total,page,page_size}），实际 ${JSON.stringify(data)}`
  ).toBe(true);
  return items as AfterSalesRow[];
}

/** 按主键从列表定位自建记录；找不到即红（禁静默过滤） */
function findRow(items: AfterSalesRow[], id: number, endpoint: string): AfterSalesRow {
  const row = items.find(it => Number(it.id) === id);
  expect(
    row,
    `${endpoint} 应回读到自建售后工单 id=${id}，实际 items=${JSON.stringify(items)}`
  ).toBeTruthy();
  return row as AfterSalesRow;
}

test.describe.serial('65 售后工单编辑-重开回读闭环', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('65-01 售后全字段创建→重开回显→部分修改+补 reason→再重开：改过=新值、没改过=原值、落库交叉校验', async ({
    page,
  }) => {
    // ========== 步骤 0：自建自流转的前置链路（客户→产品→定制订单），绝不依赖 seed 数据 ==========
    const custCode = genCode('E2E65C');
    const custName = `65号售后闭环客户_${custCode}`;
    const createdCust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_code: custCode,
      customer_name: custName,
      contact_person: '65号联系人',
      contact_phone: '13900000065',
    });
    const customerId = Number(createdCust.data?.id);
    expect(customerId, `自建客户应返回 id：${JSON.stringify(createdCust)}`).toBeGreaterThan(0);

    const prodCode = genCode('E2E65P');
    const createdProd = await apiCall<{ id?: number }>(page, 'POST', '/products', {
      code: prodCode,
      name: `65号售后闭环布_${prodCode}`,
      unit: 'm',
    });
    const productId = Number(createdProd.data?.id);
    expect(productId, `自建产品应返回 id：${JSON.stringify(createdProd)}`).toBeGreaterThan(0);

    const createdOrder = await apiCall<{ id?: number; order_no?: string }>(
      page,
      'POST',
      '/custom-orders',
      {
        customer_id: customerId,
        product_id: productId,
        spec: `E2E65/SPEC/${prodCode}`,
        quantity: '120',
        unit: 'm',
        total_amount: '18000.50',
        currency: 'CNY',
        notes: `65号闭环定制单_${prodCode}`,
      }
    );
    const orderId = Number(createdOrder.data?.id);
    expect(orderId, `自建定制订单应返回 id：${JSON.stringify(createdOrder)}`).toBeGreaterThan(0);

    // ========== 步骤 1：创建售后工单，填写全部可填字段（含易丢的 refund_amount/reason_*）==========
    const issueType = 'refund'; // refund 是唯一强制带金额的词表值，覆盖金额链路
    const description = `E2E65_售后描述_${custCode}_批次42`;
    const reasonCategory = 'quality';
    const reasonDetail = `E2E65_原因明细_${custCode}_染色牢度不达标`;
    const refundAmountOriginal = '1234.56';

    const created = await apiCall<AfterSalesRow>(
      page,
      'POST',
      `/custom-orders/${orderId}/after-sales`,
      {
        customer_id: customerId,
        issue_type: issueType,
        description,
        refund_amount: refundAmountOriginal,
        reason_category: reasonCategory,
        reason_detail: reasonDetail,
      }
    );
    const afterSalesId = Number(created.data?.id);
    expect(afterSalesId, `售后创建应返回 id：${JSON.stringify(created)}`).toBeGreaterThan(0);
    expect(created.data?.status, '新建售后初始状态应为 opened（小写词表）').toBe('opened');

    const listEp = `/custom-orders/${orderId}/after-sales?page=1&page_size=50`;

    // ========== 步骤 2：重新打开（列表端点回读），断言字段真回显为刚提交的值 ==========
    const reopened = await apiCall<PagedAfterSales>(page, 'GET', listEp);
    expect(Number(reopened.data?.total), '分页信封 total 应为数字且 >=1').toBeGreaterThanOrEqual(1);
    const row1 = findRow(requireItems(reopened.data, listEp), afterSalesId, listEp);

    // —— 绿断言（当前契约可回显的键）——
    expect(row1.issue_type, '重开回显：售后类型应等于提交值').toBe(issueType);
    expect(row1.description, '重开回显：描述应等于提交值（不是空/默认）').toBe(description);
    expect(row1.status, '重开回显：状态应为 opened').toBe('opened');
    expect(
      Number(row1.refund_amount),
      `重开回显：退款金额应回显 1234.56（rust_decimal 出参字符串，Number 归一），实际 ${JSON.stringify(row1.refund_amount)}`
    ).toBe(1234.56);

    // —— 真实回读判据（DTO 现含三键；缺键/值不符即红，读侧富化链回归交后端，勿放宽）——
    expect(
      row1.customer_id,
      `重开回显：customer_id 已提交且 service 落库（aftersales_service.rs:126 Set），读侧经 into_model::<AfterSalesInfo> 富化应回显。实际值：${JSON.stringify(row1.customer_id)}`
    ).toBe(customerId);
    expect(
      row1.reason_category,
      `重开回显：reason_category 已提交且落库（aftersales_service.rs:134），DTO/富化均含该键，读回应回显。实际值：${JSON.stringify(row1.reason_category)}`
    ).toBe(reasonCategory);
    expect(
      row1.reason_detail,
      `重开回显：reason_detail 已提交且落库（aftersales_service.rs:135），DTO/富化均含该键，读回应回显。实际值：${JSON.stringify(row1.reason_detail)}`
    ).toBe(reasonDetail);

    // ========== 步骤 3：修改若干字段（PUT 仅提交 status/resolution/refund_amount，
    //          刻意不提交 issue_type/description/reason_*——正是「整行重存洗列」缺陷的触发形态）==========
    const resolutionNew = `E2E65_处理结论_${custCode}_已受理`;
    const refundAmountNew = '987.65';
    const updated = await apiCall<AfterSalesRow>(
      page,
      'PUT',
      `/custom-orders/after-sales/${afterSalesId}`,
      {
        status: 'accepted',
        resolution: resolutionNew,
        refund_amount: refundAmountNew,
      }
    );
    expect(Number(updated.data?.id), 'PUT 应回同一工单 id').toBe(afterSalesId);

    // ========== 步骤 4：再次重开：改过的=新值，没改过的仍=原值 ==========
    const reopened2 = await apiCall<PagedAfterSales>(page, 'GET', listEp);
    const row2 = findRow(requireItems(reopened2.data, listEp), afterSalesId, listEp);

    // 改过的字段 = 新值
    expect(row2.status, '再重开：status 已提交 accepted → 应为新值').toBe('accepted');
    expect(row2.resolution, '再重开：resolution 已提交 → 应为新值').toBe(resolutionNew);
    expect(Number(row2.refund_amount), '再重开：refund_amount 已提交 → 应为新值 987.65').toBe(
      987.65
    );
    expect(
      row2.closed_at,
      `accepted 不是终态（service:169-171 仅 closed/resolved/rejected 置 closed_at），应保持 null，实际 ${JSON.stringify(row2.closed_at)}`
    ).toBeNull();

    // 没改过的字段 = 原值（抓「整行重存把未提交列洗 NULL」）
    expect(row2.issue_type, '再重开：issue_type 未提交 → 应保留原值（被洗 NULL/重置即缺陷）').toBe(
      issueType
    );
    expect(
      row2.description,
      '再重开：description 未提交 → 应保留原值（被洗 NULL/置空即缺陷）'
    ).toBe(description);
    expect(row2.quality_issue_id, '再重开：quality_issue_id 创建未填 → 应仍为 null').toBeNull();

    // 再重开的 reason_*/customer_id 回显：与步骤 2 同判据（富化回读，缺键即红）
    expect(
      row2.customer_id,
      `再重开：customer_id 应仍等于原提交值（未提交列被洗即缺陷）实际 ${JSON.stringify(row2.customer_id)}`
    ).toBe(customerId);
    expect(
      row2.reason_category,
      `再重开：reason_category 应仍等于原提交值（PUT 未含该列，被洗 NULL 即整行重存洗列缺陷）实际 ${JSON.stringify(row2.reason_category)}`
    ).toBe(reasonCategory);
    expect(
      row2.reason_detail,
      `再重开：reason_detail 应仍等于原提交值（被洗 NULL 即洗列缺陷）实际 ${JSON.stringify(row2.reason_detail)}`
    ).toBe(reasonDetail);

    // ========== 步骤 5：第二个读回端点交叉校验落库真值（不能只看一个列表端点）==========
    const detail = await apiCall<CustomOrderDetail>(page, 'GET', `/custom-orders/${orderId}`);
    expect(Number(detail.data?.id), '定制订单详情应回读同一订单').toBe(orderId);
    expect(
      Array.isArray(detail.data?.after_sales),
      `详情 after_sales 必须为数组（handler:281 map_after_sales），实际 ${JSON.stringify(detail.data?.after_sales)}`
    ).toBe(true);
    const detailRow = findRow(
      detail.data?.after_sales as AfterSalesRow[],
      afterSalesId,
      `GET /custom-orders/${orderId} (after_sales)`
    );
    // 交叉校验：与列表端点逐键一致（同源于 DB 行）
    expect(detailRow.status, '详情交叉校验：status 与列表一致').toBe(row2.status);
    expect(detailRow.resolution, '详情交叉校验：resolution 与列表一致').toBe(row2.resolution);
    expect(Number(detailRow.refund_amount), '详情交叉校验：refund_amount 与列表一致').toBe(987.65);
    expect(detailRow.issue_type, '详情交叉校验：issue_type 保留原值').toBe(issueType);
    expect(detailRow.description, '详情交叉校验：description 保留原值').toBe(description);
    expect(
      detailRow.reason_category,
      `详情交叉校验：reason_category 应等于原提交值（与列表同源富化）实际 ${JSON.stringify(detailRow.reason_category)}`
    ).toBe(reasonCategory);
    expect(
      detailRow.reason_detail,
      `详情交叉校验：reason_detail 应等于原提交值（与列表同源富化）实际 ${JSON.stringify(detailRow.reason_detail)}`
    ).toBe(reasonDetail);
    expect(
      detailRow.customer_id,
      `详情交叉校验：customer_id 应等于原提交值（与列表同源富化）实际 ${JSON.stringify(detailRow.customer_id)}`
    ).toBe(customerId);
  });
});
