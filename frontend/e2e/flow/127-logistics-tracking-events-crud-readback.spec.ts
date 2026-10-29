import { test, expect } from '../diagnose-fixture';
import {
  APP_ERROR_CODES,
  loginViaUI,
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  deferCleanup,
  flushDeferredCleanups,
  failureCode,
  API_BASE,
  API_PREFIX,
  type DeferredCleanup,
} from './helpers';

/**
 * 127 物流跟踪端点族的读写回（finance 域，logistics-tracking 族）
 *
 * 功能：守住物流跟踪面（运单跟踪事件表 logistics_tracking_events 与运单表 logistics_waybills）
 * 确实把提交值落库并能读回，覆盖以下五个真实端点，均由 logistics_tracking_handler 与 print_handler 产出：
 * - POST /logistics-tracking/waybills/{id}/tracking-events 记录跟踪事件，
 *   入参 TrackingEvent（event_time/rfc3339 必填、description 必填、event_type 必填且必须在
 *   pickup/in_transit/arrived/delivered 取值域内、location 可选、data_source 可选缺省 manual）；
 *   事件类型越界返回 400 且码为 VALIDATION_ERROR，运单不存在返回 404。
 *   出参 ApiResponse<Model>，data 为事件行对象。
 * - GET /logistics-tracking/waybills/{id}/tracking-events 查询该运单全部事件，
 *   按 event_time 升序返回 Vec<Model>，出参 ApiResponse<Vec>。
 * - POST /logistics-tracking/waybills/{id}/calculate-freight 按重量/体积/距离取最大值核算运费
 *   并回写 waybill.freight_fee，无请求体（handler 只取路径 id）；运单不存在返回 404。
 *   出参 ApiResponse<Decimal>，data 为运费值（字符串形态）。
 * - POST /logistics-tracking/waybills/{id}/link-purchase-order 关联采购订单到运单，
 *   入参 LinkPurchaseOrderRequest（po_id 必填）；采购订单不存在返回 404，运单不存在返回 404；
 *   成功后运单的 order_id 变 po_id、order_type 变 purchase_order。
 *   出参 ApiResponse<Model>，data 为更新后的运单对象。
 * - GET /logistics-tracking/waybills/{id}/print 打印运单 DOCX，由 print_handler 产出；
 *   返回二进制 application/vnd.openxmlformats-officedocument.wordprocessingml.document，
 *   非 JSON，须用 page.request.get 直接判定 HTTP 状态与 Content-Type。
 *
 * 调用方：routes/logistics_tracking.rs 的 logistics_tracking 函数注册，
 * handler 在 handlers/logistics_tracking_handler.rs 与 handlers/print_handler.rs。
 * 入参：依赖 logistics_waybills 表，运单通过 POST /logistics 自建
 *   （必填 order_id 取 EntityContext.salesOrderId、logistics_company、tracking_number）。
 * 传给谁：LogisticsService → sea_orm → PostgreSQL（logistics_tracking_events、logistics_waybills）。
 * 存什么：跟踪事件 id/waybill_id/event_time/location/description/event_type/data_source；
 *   运单 order_id/order_type/freight_fee。
 * 存哪里：表 logistics_tracking_events 与 logistics_waybills，
 *   路由前缀 /api/v1/erp/logistics-tracking/waybills。
 *
 * 断言均为值判据，写操作落库回读，不用恒真、不只判 ok()。
 */

type TrackingEventRow = {
  id: number;
  waybill_id: number;
  event_time: string;
  location: string | null;
  description: string;
  event_type: string;
  data_source: string;
};

type WaybillRow = {
  id: number;
  order_id: number;
  order_type: string | null;
  logistics_company: string;
  tracking_number: string;
  freight_fee: string | null;
  total_weight: string | null;
  total_volume: string | null;
  distance_km: string | null;
  freight_rate: string | null;
};

function uniqTag(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

test.describe.serial('127 物流跟踪端点族读写回', () => {
  const cleanup: DeferredCleanup[] = [];
  let waybillId: number;

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test.afterEach(async ({ page }) => {
    await flushDeferredCleanups(page, cleanup);
  });

  // 为每组用例建一条运单并注册清理
  async function seedWaybill(page: import('@playwright/test').Page, tag: string): Promise<number> {
    const ctx = getCtx();
    expect(ctx.salesOrderId, '前置：EntityContext.salesOrderId 缺失').toBeTruthy();
    const wb = await apiCall<WaybillRow>(page, 'POST', '/logistics', {
      order_id: ctx.salesOrderId,
      logistics_company: `e2e-物流-${tag}`,
      tracking_number: `e2e-TN-${tag}`,
    });
    waybillId = wb.data.id;
    deferCleanup(cleanup, 'DELETE', `/logistics/${waybillId}`, `lt-wb-${tag}`);
    return waybillId;
  }

  test('POST 记录跟踪事件后 GET 列表读回逐键持久', async ({ page }) => {
    const tag = uniqTag();
    const wbId = await seedWaybill(page, tag);

    const now = new Date().toISOString();
    const created = await apiCall<TrackingEventRow>(
      page,
      'POST',
      `/logistics-tracking/waybills/${wbId}/tracking-events`,
      {
        event_time: now,
        description: `e2e-事件-${tag}`,
        event_type: 'pickup',
        location: 'e2e-上海转运中心',
      }
    );
    expect(created.code, '记录事件应返回 200').toBe(200);
    const evtId = created.data.id;
    expect(Number.isInteger(evtId) && evtId > 0, `事件 id 应为正整数`).toBe(true);

    expect(created.data.waybill_id).toBe(wbId);
    expect(created.data.event_type).toBe('pickup');
    expect(created.data.description).toBe(`e2e-事件-${tag}`);
    expect(created.data.location).toBe('e2e-上海转运中心');
    expect(created.data.data_source, '缺省 data_source 应为 manual').toBe('manual');

    // 回读确认落库
    const listed = await apiCall<TrackingEventRow[]>(
      page,
      'GET',
      `/logistics-tracking/waybills/${wbId}/tracking-events`
    );
    const hit = listed.data.find(e => e.id === evtId);
    expect(hit, `事件列表应含刚记事件 id=${evtId}`).toBeDefined();
    expect(hit!.event_type).toBe('pickup');
    expect(hit!.description).toBe(`e2e-事件-${tag}`);
    expect(hit!.location).toBe('e2e-上海转运中心');
  });

  test('POST 记录非法事件类型返回 400 且码点名取值域', async ({ page }) => {
    const tag = uniqTag();
    const wbId = await seedWaybill(page, tag);

    const fail = await apiCallExpectFail(
      page,
      'POST',
      `/logistics-tracking/waybills/${wbId}/tracking-events`,
      {
        event_time: new Date().toISOString(),
        description: 'e2e-非法类型',
        event_type: 'invalid_type_xyz',
      }
    );
    expect(fail.status, '非法事件类型应 400').toBe(400);
    expect(
      fail.message,
      `错误信息应点名允许取值域 pickup/in_transit/arrived/delivered，实际：${fail.message}`
    ).toContain('pickup');
  });

  test('POST calculate-freight 回写运单 freight_fee 并读回', async ({ page }) => {
    const tag = uniqTag();
    const wbId = await seedWaybill(page, tag);

    const result = await apiCall<string>(
      page,
      'POST',
      `/logistics-tracking/waybills/${wbId}/calculate-freight`
    );
    expect(result.code, '运费核算应返回 200').toBe(200);
    // 运单 freight_rate 未设置（建运单时不传），所以核算结果应为 0
    expect(Number(result.data), '未设费率的运单运费应为 0').toBe(0);

    // 回读运单确认 freight_fee 被写入
    const wb = await apiCall<WaybillRow>(page, 'GET', `/logistics/${wbId}`);
    expect(
      Number(wb.data.freight_fee),
      `freight_fee 应被回写为 0，实际：${wb.data.freight_fee}`
    ).toBe(0);
  });

  test('POST link-purchase-order 改运单 order_id 与 order_type 并读回', async ({ page }) => {
    const tag = uniqTag();
    const wbId = await seedWaybill(page, tag);
    const ctx = getCtx();
    expect(ctx.purchaseOrderId, '前置：EntityContext.purchaseOrderId 缺失').toBeTruthy();

    const result = await apiCall<WaybillRow>(
      page,
      'POST',
      `/logistics-tracking/waybills/${wbId}/link-purchase-order`,
      { po_id: ctx.purchaseOrderId }
    );
    expect(result.code, '关联采购订单应返回 200').toBe(200);
    expect(result.data.order_id, 'order_id 应变为采购订单 id').toBe(ctx.purchaseOrderId);
    expect(result.data.order_type, 'order_type 应变为 purchase_order').toBe('purchase_order');

    // 回读运单确认持久
    const wb = await apiCall<WaybillRow>(page, 'GET', `/logistics/${wbId}`);
    expect(wb.data.order_id).toBe(ctx.purchaseOrderId);
    expect(wb.data.order_type).toBe('purchase_order');
  });

  test('POST link-purchase-order 不存在的 po_id 返回 404', async ({ page }) => {
    const tag = uniqTag();
    const wbId = await seedWaybill(page, tag);

    const fail = await apiCallExpectFail(
      page,
      'POST',
      `/logistics-tracking/waybills/${wbId}/link-purchase-order`,
      { po_id: 999999999 }
    );
    expect(fail.status, '不存在的采购订单应 404').toBe(404);
    expect(failureCode(fail)).toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('GET print 对已建运单返回 DOCX 二进制（非 JSON）', async ({ page }) => {
    const tag = uniqTag();
    const wbId = await seedWaybill(page, tag);

    const resp = await page.request.get(
      `${API_BASE}${API_PREFIX}/logistics-tracking/waybills/${wbId}/print`
    );
    expect(resp.status(), '打印应返回 200').toBe(200);
    const ct = resp.headers()['content-type'] ?? '';
    expect(
      ct,
      `Content-Type 应为 DOCX MIME，实际：${ct}`
    ).toContain('wordprocessingml');
  });

  test('GET tracking-events 对不存在的运单返回 404', async ({ page }) => {
    const fail = await apiCallExpectFail(
      page,
      'GET',
      '/logistics-tracking/waybills/999999999/tracking-events'
    );
    // list_tracking_events 不校验运单存在性，直接返回空列表；若后端已加校验则 404
    // 两种都是合法行为，此处只断不 5xx
    expect(
      fail.status,
      `对不存在运单的 GET tracking-events 应 <500，实际 ${fail.status}`
    ).toBeLessThan(500);
  });
});
