// 大货批色审批契约面 — enhanced/bulk-color-approval-contract
// 功能（业务规则面，逐端点对后端源码核实后编写）：
//   1) 写族状态门：cut-sample、send-to-customer、approve、reject、rework、downgrade、scrap
//      在非允许前态必须被拒绝且业务列零漂移（判据链在
//      backend/src/services/bulk_color_approval_service.rs::transition_to 与各专用方法内，
//      拒绝出参走脱敏常量「业务处理失败」，机器码 BUSINESS_ERROR）。
//   2) 未知记录 id 的写族与 history 读族统一 404（机器码 NOT_FOUND，脱敏「资源未找到」）。
//   3) 报废是唯一可直达的合法跃迁（pending → scrapped，允许前态集合含 pending）：
//      断言状态列、reject_reason 留痕列、delivery_blocking 与 history 轨迹同源。
//   4) 客户确认色差门：ΔE>2.5 在状态门之前拒绝，出参可外显逐字文案
//      （backend/src/services/bulk_color_approval_service.rs::customer_approve 色差判定段）。
//   5) 提醒读族与发送族的阈值参数传播：GET reminders/pending 在 threshold_hours=0 必含
//      本轮新建 pending 行、=72 必不含；reminders/pending-configurable 与 followups 只认
//      sent_to_customer 态，本轮 pending 行任何阈值下都不得出现在其中。
//      POST reminders/send-pending 与 send-followups 用 threshold_hours=100000（约 11.4 年，
//      任何运行期数据都不可能超过）断言 sent_count 恒 0，反证阈值真实生效。
//   6) POST reminders/check-timeouts 用 reject_days=99999 断言零命中零改写（自动拒绝成功路径
//      需要超过 7 天的 sent_to_customer 历史数据，HTTP 面无法造旧时间戳，属残量登记项）。
//   7) GET report 按自建唯一 customer_id 聚合，各状态计数与本轮真实落库状态逐一相等；
//      from_date 下推到未来窗时必须整体排除本轮行，证明日期参数未被吞。
//   8) GET statistics 用响应自身字段反算三个率（approved*100 与 approval_rate*total 等式），
//      并用未来 from_date 窗断言 total 与各率全为 0 的边界形态。
// 调用方：CI e2e 分片 36（extras enhanced+production，.github/workflows/ci-cd.yml 组 36）。
// 入参：全部经 apiCall、apiCallRaw、apiCallExpectFail 走真实后端与真实库；
//   本轮自建销售订单、自建产品、自建客户、引用本 worker ensureTestEntities 的染色批次。
// 传给谁：断言只针对本轮自建行按主键或唯一 customer_id 精确回读，不触碰他人行。
// 存什么：bulk_color_approvals 的状态列、reject_reason、approver_id、approval_date、
//   sent_to_customer_at、delta_e_value、delivery_blocking；bulk_color_approval_history 的
//   from_status、to_status、reason、snapshot。
// 存哪里：表 bulk_color_approvals 与 bulk_color_approval_history，迁移定义在
//   migration crate 的 v15 批色域段。
// 残量登记（不许算已覆盖）：
//   cut-sample 成功路径、send-to-customer 成功路径、approve 与 reject 与 rework 的成功路径
//   需要染色批次到 STORED 生命周期态且存在同缸号同批号的合格大货库存行；缸号生命周期
//   pending_schedule 到 STORED 非法（流转表见 backend/src/services/dye_batch_state_machine_validation.rs），
//   HTTP 面无法在本轮闭环内造出该前置，三态跃迁只覆盖了拒绝侧与零漂移侧。
//   check-timeouts 与 send-to-customer 的外发面：提醒实现为
//   NotificationType::Internal 站内落库（backend/src/services/bulk_color_approval_service.rs::
//   send_pending_reminders 请求构造段），无出站通道，外发风险面为空。
import { test, expect } from '../diagnose-fixture';
import { PRODUCT_TYPE } from '../../src/constants/product-type';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  getCtx,
  genCode,
  tryCleanup,
  failureCode,
  APP_ERROR_CODES,
  type ApiFailureResult,
} from '../flow/helpers';

interface ApprovalDetail {
  id: number;
  sales_order_id: number;
  customer_id: number;
  product_id: number | null;
  approval_status: string;
  approver_id: number | null;
  approval_date: string | null;
  sent_to_customer_at: string | null;
  delta_e_value: string | number | null;
  reject_reason: string | null;
  delivery_blocking: boolean;
  sample_piece_id: number | null;
  remark: string | null;
}

interface HistoryRow {
  id: number;
  bulk_color_approval_id: number;
  from_status: string | null;
  to_status: string;
  reason: string | null;
}

interface PagedInfo<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
}

/**
 * 功能：创建一张本轮专属的 pending 批色记录（父销售订单、客户、产品均自建带唯一后缀）。
 * 调用方：本文件各用例。
 * 入参：tag 为本轮唯一后缀；dye_batch_id 取本 worker ensureTestEntities 的真实缸号（FK NOT NULL）。
 * 传给谁：返回的 approvalId 供写族状态门与 history 回读使用。
 * 存什么：bulk_color_approvals 一行（初始 pending、delivery_blocking=true）及其
 *   bulk_color_approval_history 首行（from_status 为 null、to_status 为 pending）。
 * 存哪里：表 bulk_color_approvals、bulk_color_approval_history；父单在 sales_orders。
 */
async function seedPendingApproval(
  page: import('@playwright/test').Page,
  tag: string
): Promise<{
  approvalId: number;
  customerId: number;
  salesOrderId: number;
  productId: number;
}> {
  const ctx = getCtx();
  const dyeBatchId = ctx.dyeBatchId;
  expect(dyeBatchId, '前置：批色建单需本 worker 真实染色批次 id（外键 NOT NULL）').toBeTruthy();
  const categoryId = ctx.productCategoryIds[0];
  expect(categoryId, '前置：批色用例需产品分类 id 以自建产品').toBeTruthy();

  const prod = await apiCall<{ id?: number }>(page, 'POST', '/products', {
    name: `批色契约产品${tag}`,
    code: `E2CBC${tag}`,
    category_id: categoryId,
    unit: 'm',
    product_type: PRODUCT_TYPE.auxiliary,
  });
  const productId = Number(prod.data?.id);
  expect(productId, `自建产品失败：${JSON.stringify(prod.data)}`).toBeGreaterThan(0);

  const cust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
    customer_name: `批色契约客户${tag}`,
  });
  const customerId = Number(cust.data?.id);
  expect(customerId, `自建客户失败：${JSON.stringify(cust.data)}`).toBeGreaterThan(0);

  const order = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
    customer_id: customerId,
    order_date: new Date().toISOString(),
    items: [{ product_id: productId, quantity: '1', unit_price: '20' }],
  });
  const salesOrderId = Number(order.data?.id);
  expect(salesOrderId, `自建销售订单失败：${JSON.stringify(order.data)}`).toBeGreaterThan(0);

  const created = await apiCall<{ id?: number; approval_status?: string }>(
    page,
    'POST',
    '/bulk-color-approvals',
    {
      sales_order_id: salesOrderId,
      dye_batch_id: dyeBatchId,
      customer_id: customerId,
      product_id: productId,
      remark: `批色契约${tag}`,
    }
  );
  const approvalId = Number(created.data?.id);
  expect(approvalId, `建批色单失败：${JSON.stringify(created.data)}`).toBeGreaterThan(0);
  expect(
    created.data!.approval_status,
    `新建批色单初始态必须为 pending，实际 ${created.data!.approval_status}`
  ).toBe('pending');

  return { approvalId, customerId, salesOrderId, productId };
}

/** 按主键回读批色详情（GET 详情为既有 C 档通道，这里只作回读载体） */
async function readApproval(page: import('@playwright/test').Page, approvalId: number) {
  return await apiCallRaw<ApprovalDetail>(page, 'GET', `/bulk-color-approvals/${approvalId}`);
}

/** 断言业务状态门拒绝族：HTTP 400、机器码 BUSINESS_ERROR、脱敏常量逐字「业务处理失败」 */
function assertStateGateDenied(fail: ApiFailureResult) {
  expect(fail.status, `状态门必须返回 400，实际 ${fail.status}：${JSON.stringify(fail)}`).toBe(400);
  expect(failureCode(fail), `状态门机器码必须为 BUSINESS_ERROR，实际 ${fail.code}`).toBe(
    APP_ERROR_CODES.BUSINESS_ERROR
  );
  expect(fail.message, `状态门外显文案必须为脱敏常量，实际 ${fail.message}`).toBe('业务处理失败');
}

test.describe('大货批色审批 - 状态门与提醒报表契约面', () => {
  test.beforeEach(async ({ page }) => {
    await ensureTestEntities(page);
  });

  test('写族状态门 - 非允许前态一律 400 拒绝且业务列零漂移', async ({ page }) => {
    const tag = genCode('BCG');
    const seed = await seedPendingApproval(page, tag);
    try {
      const before = await readApproval(page, seed.approvalId);
      expect(before.approval_status).toBe('pending');
      const historyBefore = await apiCallRaw<HistoryRow[]>(
        page,
        'GET',
        `/bulk-color-approvals/${seed.approvalId}/history`
      );
      expect(historyBefore.length, '新建后 history 应恰有一行 pending 轨迹').toBe(1);
      expect(historyBefore[0].from_status, '首行 from_status 必须为 null').toBeNull();
      expect(historyBefore[0].to_status).toBe('pending');

      const cut = await apiCallExpectFail(
        page,
        'POST',
        `/bulk-color-approvals/${seed.approvalId}/cut-sample`,
        { sample_length_m: '0.5' }
      );
      assertStateGateDenied(cut);

      const send = await apiCallExpectFail(
        page,
        'POST',
        `/bulk-color-approvals/${seed.approvalId}/send-to-customer`
      );
      assertStateGateDenied(send);

      const approve = await apiCallExpectFail(
        page,
        'POST',
        `/bulk-color-approvals/${seed.approvalId}/approve`,
        { feedback: `诱饵通过${tag}` }
      );
      assertStateGateDenied(approve);

      const reject = await apiCallExpectFail(
        page,
        'POST',
        `/bulk-color-approvals/${seed.approvalId}/reject`,
        { reject_reason: `诱饵拒绝${tag}` }
      );
      assertStateGateDenied(reject);

      const rework = await apiCallExpectFail(
        page,
        'POST',
        `/bulk-color-approvals/${seed.approvalId}/rework`,
        { reject_reason: `诱饵返工${tag}` }
      );
      assertStateGateDenied(rework);

      const downgrade = await apiCallExpectFail(
        page,
        'POST',
        `/bulk-color-approvals/${seed.approvalId}/downgrade`,
        { reject_reason: `诱饵降级${tag}` }
      );
      assertStateGateDenied(downgrade);

      // 零漂移回读：七次被拒后状态、留痕列、样布列全部保持建单时的初值
      const after = await readApproval(page, seed.approvalId);
      expect(after.approval_status, '状态门拒绝后 approval_status 不得漂移').toBe('pending');
      expect(after.reject_reason, '被拒的 reject_reason 不得落库').toBeNull();
      expect(after.approver_id, '被拒的 approver_id 不得落库').toBeNull();
      expect(after.approval_date, '被拒的 approval_date 不得落库').toBeNull();
      expect(after.sent_to_customer_at, '被拒的 sent_to_customer_at 不得落库').toBeNull();
      expect(after.sample_piece_id, '被拒的剪样不得生成样布引用').toBeNull();
      const historyAfter = await apiCallRaw<HistoryRow[]>(
        page,
        'GET',
        `/bulk-color-approvals/${seed.approvalId}/history`
      );
      expect(historyAfter.length, '七次被拒不得追加任何 history 轨迹行').toBe(1);
    } finally {
      await tryCleanup(page, 'DELETE', `/bulk-color-approvals/${seed.approvalId}`, 'approval');
    }
  });

  test('报废合法跃迁 - 状态列留痕列与 history 轨迹同源', async ({ page }) => {
    const tag = genCode('BCS');
    const seed = await seedPendingApproval(page, tag);
    try {
      const reason = `报废原因${tag}`;
      const scrapped = await apiCall<{
        approval_status?: string;
        reject_reason?: string | null;
        delivery_blocking?: boolean;
      }>(page, 'POST', `/bulk-color-approvals/${seed.approvalId}/scrap`, { reject_reason: reason });
      expect(scrapped.data!.approval_status, '报废响应状态必须为 scrapped').toBe('scrapped');
      expect(scrapped.data!.reject_reason, '报废响应留痕必须逐字回显提交原因').toBe(reason);

      const detail = await readApproval(page, seed.approvalId);
      expect(detail.approval_status, '回读状态必须与响应同源').toBe('scrapped');
      expect(detail.reject_reason, '回读留痕必须与响应同源').toBe(reason);
      expect(detail.delivery_blocking, ' scrapped 态必须保持交货门禁为 true').toBe(true);

      const history = await apiCallRaw<HistoryRow[]>(
        page,
        'GET',
        `/bulk-color-approvals/${seed.approvalId}/history`
      );
      expect(history.length, '建单加报废应有两行轨迹').toBe(2);
      expect(history[0].to_status).toBe('pending');
      expect(history[1].from_status, '轨迹链必须首尾相接').toBe(history[0].to_status);
      expect(history[1].to_status, '末行 to_status 必须与当前状态列同源').toBe(
        detail.approval_status
      );
      expect(history[1].reason, '轨迹行 reason 必须留痕报废原因').toBe(reason);

      // 终态再报废必须被状态门拒绝，且状态列零改写
      const again = await apiCallExpectFail(
        page,
        'POST',
        `/bulk-color-approvals/${seed.approvalId}/scrap`,
        { reject_reason: `重复报废${tag}` }
      );
      assertStateGateDenied(again);
      const final = await readApproval(page, seed.approvalId);
      expect(final.reject_reason, '被拒的二次报废不得改写留痕').toBe(reason);
    } finally {
      await tryCleanup(page, 'DELETE', `/bulk-color-approvals/${seed.approvalId}`, 'approval');
    }
  });

  test('色差门 - ΔE 超 2.5 的通过确认被逐字文案拒绝且零漂移', async ({ page }) => {
    const tag = genCode('BCD');
    const seed = await seedPendingApproval(page, tag);
    try {
      const denied = await apiCallExpectFail(
        page,
        'POST',
        `/bulk-color-approvals/${seed.approvalId}/approve`,
        { delta_e_value: 3.5, feedback: `诱饵${tag}` }
      );
      expect(denied.status, `色差门必须 400，实际 ${JSON.stringify(denied)}`).toBe(400);
      expect(failureCode(denied)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
      expect(denied.message, `色差门外显文案必须逐字，实际 ${denied.message}`).toBe(
        '色差 ΔE=3.5 超过 2.5，不允许通过'
      );
      const after = await readApproval(page, seed.approvalId);
      expect(after.approval_status, '色差门拒绝后状态不得漂移').toBe('pending');
      expect(after.delta_e_value, '被拒的 delta_e_value 不得落库').toBeNull();
    } finally {
      await tryCleanup(page, 'DELETE', `/bulk-color-approvals/${seed.approvalId}`, 'approval');
    }
  });

  test('未知记录 id - 写族与 history 读族统一 404', async ({ page }) => {
    const ghostId = 9_100_000_000 + (Date.now() % 1000);
    const writes: Array<{ path: string; body: Record<string, unknown> | undefined }> = [
      { path: `/bulk-color-approvals/${ghostId}/cut-sample`, body: { sample_length_m: '0.5' } },
      { path: `/bulk-color-approvals/${ghostId}/send-to-customer`, body: undefined },
      { path: `/bulk-color-approvals/${ghostId}/approve`, body: undefined },
      { path: `/bulk-color-approvals/${ghostId}/reject`, body: { reject_reason: '幽灵拒绝' } },
      { path: `/bulk-color-approvals/${ghostId}/rework`, body: { reject_reason: '幽灵返工' } },
      { path: `/bulk-color-approvals/${ghostId}/downgrade`, body: { reject_reason: '幽灵降级' } },
      { path: `/bulk-color-approvals/${ghostId}/scrap`, body: { reject_reason: '幽灵报废' } },
    ];
    for (const item of writes) {
      const fail = await apiCallExpectFail(page, 'POST', item.path, item.body);
      expect(fail.status, `幽灵 id 写入必须 404：${item.path} 实际 ${JSON.stringify(fail)}`).toBe(
        404
      );
      expect(failureCode(fail), `幽灵 id 机器码必须 NOT_FOUND：${item.path}`).toBe(
        APP_ERROR_CODES.NOT_FOUND
      );
      expect(fail.message, `幽灵 id 外显必须脱敏常量：${item.path}`).toBe('资源未找到');
    }
    const histFail = await apiCallExpectFail(
      page,
      'GET',
      `/bulk-color-approvals/${ghostId}/history`
    );
    expect(histFail.status, '幽灵 id 的 history 不得降级为空列表 200').toBe(404);
    expect(failureCode(histFail)).toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('提醒读族 - 阈值参数传播与本轮行在场缺席', async ({ page }) => {
    const tag = genCode('BCT');
    const seed = await seedPendingApproval(page, tag);
    try {
      const fresh = await apiCallRaw<PagedInfo<{ id: number; approval_status: string }>>(
        page,
        'GET',
        '/bulk-color-approvals/reminders/pending?threshold_hours=0'
      );
      expect(Array.isArray(fresh.items), 'pending 提醒必须返回分页对象 items 数组').toBe(true);
      expect(
        fresh.items.some(r => r.id === seed.approvalId),
        'threshold_hours=0 必含本轮新建行'
      ).toBe(true);
      expect(
        fresh.items.every(r => r.approval_status === 'pending'),
        'pending 提醒列表混入非 pending 行即为过滤失效'
      ).toBe(true);
      expect(fresh.total, '分页信封 total 必须等于 items 实际行数').toBe(fresh.items.length);
      expect(fresh.page).toBe(1);
      expect(fresh.page_size, 'page_size 必须等于 max(total,1)').toBe(Math.max(fresh.total, 1));

      const strict = await apiCallRaw<PagedInfo<{ id: number }>>(
        page,
        'GET',
        '/bulk-color-approvals/reminders/pending?threshold_hours=72'
      );
      expect(
        strict.items.some(r => r.id === seed.approvalId),
        'threshold_hours=72 必不含本轮刚建行（阈值吞没即红）'
      ).toBe(false);

      const follow = await apiCallRaw<PagedInfo<{ id: number; approval_status: string }>>(
        page,
        'GET',
        '/bulk-color-approvals/reminders/followups?threshold_hours=0'
      );
      expect(
        follow.items.every(r => r.approval_status === 'sent_to_customer'),
        'followups 只允许 sent_to_customer 态进入，混入本轮 pending 行即状态过滤失效'
      ).toBe(true);
      expect(
        follow.items.some(r => r.id === seed.approvalId),
        '本轮 pending 行绝不得出现在 followups'
      ).toBe(false);

      const configurable = await apiCallRaw<Array<{ id: number; approval_status: string }>>(
        page,
        'GET',
        '/bulk-color-approvals/reminders/pending-configurable?reminder_days=0&reject_days=99999'
      );
      expect(
        configurable.every(r => r.approval_status === 'sent_to_customer'),
        'pending-configurable 混入非 sent_to_customer 行即窗口过滤失效'
      ).toBe(true);
      expect(
        configurable.some(r => r.id === seed.approvalId),
        '本轮 pending 行绝不得出现在 pending-configurable'
      ).toBe(false);
    } finally {
      await tryCleanup(page, 'DELETE', `/bulk-color-approvals/${seed.approvalId}`, 'approval');
    }
  });

  test('提醒发送族 - 超远阈值零命中且审批行零改写', async ({ page }) => {
    const tag = genCode('BCR');
    const seed = await seedPendingApproval(page, tag);
    try {
      const sent = await apiCall<{ sent_count: number; threshold_hours: number }>(
        page,
        'POST',
        '/bulk-color-approvals/reminders/send-pending?threshold_hours=100000'
      );
      expect(sent.data!.threshold_hours, '响应必须回显提交阈值').toBe(100000);
      expect(sent.data!.sent_count, '阈值 100000 小时不可能命中任何 pending 行').toBe(0);

      const followed = await apiCall<{ sent_count: number; threshold_hours: number }>(
        page,
        'POST',
        '/bulk-color-approvals/reminders/send-followups?threshold_hours=100000'
      );
      expect(followed.data!.sent_count, '本轮无 sent_to_customer 行，followups 必零命中').toBe(0);

      const checked = await apiCallRaw<Array<{ id: number; approval_status: string }>>(
        page,
        'POST',
        '/bulk-color-approvals/reminders/check-timeouts?reminder_days=99999&reject_days=99999'
      );
      expect(Array.isArray(checked), 'check-timeouts 必须返回数组').toBe(true);
      expect(checked.length, 'reject_days=99999 时超时窗内不可能有记录').toBe(0);

      const after = await readApproval(page, seed.approvalId);
      expect(after.approval_status, '批处理端点不得改写本轮 pending 行状态').toBe('pending');
      const historyAfter = await apiCallRaw<HistoryRow[]>(
        page,
        'GET',
        `/bulk-color-approvals/${seed.approvalId}/history`
      );
      expect(historyAfter.length, '零命中批处理不得追加轨迹行').toBe(1);
    } finally {
      await tryCleanup(page, 'DELETE', `/bulk-color-approvals/${seed.approvalId}`, 'approval');
    }
  });

  test('report 聚合 - customer 维度计数与本轮真实状态逐一相等', async ({ page }) => {
    const tag = genCode('BCP');
    const one = await seedPendingApproval(page, `${tag}A`);
    const two = await seedPendingApproval(page, `${tag}B`);
    try {
      await apiCall(page, 'POST', `/bulk-color-approvals/${two.approvalId}/scrap`, {
        reject_reason: `报表报废${tag}`,
      });
      const rows = await apiCallRaw<
        Array<{
          customer_id: number;
          product_id: number | null;
          total: number;
          approved: number;
          rejected: number;
          rework: number;
          downgraded: number;
          scrapped: number;
          pending: number;
          sampled: number;
          sent_to_customer: number;
        }>
      >(page, 'GET', `/bulk-color-approvals/report?customer_id=${one.customerId}`);
      expect(rows.length, '自建唯一客户下聚合行必须恰为一条').toBe(1);
      const row = rows[0];
      expect(row.customer_id).toBe(one.customerId);
      expect(row.total, '聚合 total 必须等于该客户本轮建行数').toBe(2);
      expect(row.pending, 'pending 计数必须等于本轮滞留行数').toBe(1);
      expect(row.scrapped, 'scrapped 计数必须等于本轮报废行数').toBe(1);
      expect(row.approved, '未发生的终态计数必须为 0').toBe(0);
      expect(row.rejected).toBe(0);
      expect(row.rework).toBe(0);
      expect(row.downgraded).toBe(0);
      expect(row.sampled).toBe(0);
      expect(row.sent_to_customer).toBe(0);

      // 日期参数传播：起窗在未来时本轮行必须整体被排除
      const future = await apiCallRaw<unknown[]>(
        page,
        'GET',
        `/bulk-color-approvals/report?customer_id=${one.customerId}&from_date=2099-01-01T00%3A00%3A00Z`
      );
      expect(Array.isArray(future)).toBe(true);
      expect(future.length, 'from_date 未来窗必须排除本轮全部行（参数被吞即红）').toBe(0);
    } finally {
      await tryCleanup(page, 'DELETE', `/bulk-color-approvals/${one.approvalId}`, 'approval');
      await tryCleanup(page, 'DELETE', `/bulk-color-approvals/${two.approvalId}`, 'approval');
    }
  });

  test('statistics - 响应自身反算三个率且未来窗全零边界', async ({ page }) => {
    const tag = genCode('BCQ');
    const seed = await seedPendingApproval(page, tag);
    try {
      const stats = await apiCallRaw<{
        total: number;
        approved: number;
        rejected: number;
        rework: number;
        downgraded: number;
        scrapped: number;
        approval_rate: string | number;
        rejection_rate: string | number;
        downgrade_rate: string | number;
        avg_delta_e: string | number;
      }>(
        page,
        'GET',
        `/bulk-color-approvals/statistics?from_date=${encodeURIComponent(new Date(Date.now() - 3_600_000).toISOString())}`
      );
      const total = stats.total;
      const approvalRate = Number(stats.approval_rate);
      expect(total, '近一小时内可见行数不可能为 0（本轮至少有一条建行）').toBeGreaterThan(0);
      expect(
        Math.abs(approvalRate - (stats.approved * 100) / total),
        `approval_rate 必须等于 approved*100/total，实际 ${stats.approval_rate}`
      ).toBeLessThan(1e-6);
      expect(
        Math.abs(Number(stats.rejection_rate) - (stats.rejected * 100) / total),
        'rejection_rate 与 rejected/total 不自洽'
      ).toBeLessThan(1e-6);
      expect(
        Math.abs(Number(stats.downgrade_rate) - (stats.downgraded * 100) / total),
        'downgrade_rate 与 downgraded/total 不自洽'
      ).toBeLessThan(1e-6);
      expect(approvalRate, '通过率必须落在 0 到 100').toBeGreaterThanOrEqual(0);
      expect(approvalRate).toBeLessThanOrEqual(100);
      expect(Number(stats.avg_delta_e), '平均 ΔE 不得为负').toBeGreaterThanOrEqual(0);

      const future = await apiCallRaw<{
        total: number;
        approval_rate: string | number;
        rejection_rate: string | number;
        downgrade_rate: string | number;
        avg_delta_e: string | number;
      }>(page, 'GET', '/bulk-color-approvals/statistics?from_date=2099-01-01T00%3A00%3A00Z');
      expect(future.total, '未来起窗必须收敛为 0').toBe(0);
      expect(Number(future.approval_rate), '无数据时比率必须如实 0 而非兜底').toBe(0);
      expect(Number(future.rejection_rate)).toBe(0);
      expect(Number(future.downgrade_rate)).toBe(0);
      expect(Number(future.avg_delta_e)).toBe(0);
    } finally {
      await tryCleanup(page, 'DELETE', `/bulk-color-approvals/${seed.approvalId}`, 'approval');
    }
  });
});
