import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  getCtx,
  genCode,
  failureCode,
  tryCleanup,
  APP_ERROR_CODES,
} from './helpers';

/**
 * 71 BI 销售聚合数值真伪（既有覆盖深断言=0 的补位；26 号仅端点健康扫描）
 *
 * 端点与响应结构（逐条对过当前源码，非臆造）：
 * - GET  /bi/sales/by-time?start_date=&end_date=&granularity=      routes/analytics.rs:491-494（analytics::routes
 *   内 .nest("/bi", bi()) :619，mod.rs:420 merge 到 /api/v1/erp）；
 *   入参 ByTimeQuery（bi_handler.rs:33-38：start_date/end_date 为必填 NaiveDate、granularity 必填 String）；
 *   出参**双层信封** ApiResponse{code,message,data=BiResponse{code:0,message:'success',data:[TimeSeriesPoint]} }
 *   （bi_handler.rs:44-56 + bi_analysis_ops/types.rs:17-30；TimeSeriesPoint{period,total_amount,
 *   order_count,quantity,profit_amount}，f64 序列化为 JSON number，types.rs:39-50）。
 * - GET  /bi/sales/drilldown/year-to-month?year=                    routes/analytics.rs:524-527；
 *   12 个月完整序列缺失补 0（bi_analysis_ops/drilldown.rs:77-110）。
 * - GET  /bi/sales/drilldown/customer-to-order/{customer_id}        routes/analytics.rs:532-535；
 *   出参 data={customer_id, orders:[{order_id, amount, date}]}（drilldown.rs:259-274）。
 * - GET  /sales/orders（分页读回明细）                                routes/sales.rs:23（mod.rs:462 nest /sales）；
 *   SalesOrderQuery 支持 customer_id/start_date/end_date/page/page_size（sales_order_handler.rs:21-33），
 *   出参 PaginatedResponse{items,total,page,page_size}（order_query.rs:103-118），行内金额=SalesOrderDetail
 *   rust_decimal 字符串（services/so/mod.rs:43-72）。
 * - POST /sales/orders                                                sales_order_crud 链：
 *   CreateSalesOrderRequest（services/so/mod.rs:128-165）status/order_date 透传落库
 *   （order_crud.rs:265-277 build_order_active_model——status 缺省 draft，显式提交按提交值）；
 *   行金额=round_dp(2)(qty×price)（order_crud.rs:342-364），订单 total_amount=Σ行（:419-444）。
 * - DELETE /sales/orders/{id}                                         非 shipped/completed 可硬删（order_crud.rs:706-711）。
 *
 * 聚合口径契约（判据 3 的钉桩对象）：bi_analysis_ops/sales.rs:57-95 by-time SQL——
 * 数据源 sales_orders.total_amount / sales_order_items.quantity，业务规则注释（sales.rs:10-13）
 * 明言"排除 CANCELLED 和 DRAFT 状态的订单"。
 *
 * ⚠️ 预置判红点（71-03，断言按**契约**写死不迁就现状）：写入方词表 so_status 为小写
 * （models/status/sales.rs:16/28 DRAFT='draft'、CANCELLED='cancelled'），而 BI/钻取全部 SQL 用
 * 大写比较（sales.rs:74/165/188/251/306/355、drilldown.rs:64/156/243/296）——Postgres 字符串
 * 比较大小写敏感，排除门恒不命中 → draft/cancelled 订单会被计入聚合。该缺陷属"源码缺陷"，
 * 已按纪律进交付报告"待主编排立案"（不自行修改 backend）；71-03 在修复前判红是**功能**，
 * 禁止通过放宽断言/改期望值洗绿。
 *
 * 防缓存假绿/假红设计（bi_analysis_service.rs:32-48 5min TTL 缓存，键=scope+参数）：
 * - 专属年份 2096（全仓 e2e grep 无占用；61 号占 2098/2099、70 号占 2097），窗口参数天然唯一；
 * - 同一用例内对 by-time 的每次查询使用**互不相同的参数键**（seed 后首查、删除后无痕复查
 *   用 end_date=11-29 变体），避免命中缓存拿到陈旧值造成"无痕假绿"或"首查假红"；
 *   drilldown 系端点当前源码无 set_cache 路径（drilldown.rs 全文），可安全复查询。
 * - 金额全部取二进制精确小数（.00/.25/.50/.75），聚合值与明细和用 === 精确相等，零容差。
 *
 * 残留纪律：每例订单/客户在正常路径显式删除并断"删后聚合无痕"；断言中途失败时 finally
 * 走 tryCleanup 兜底。若 worker 被硬杀留下残留，后续运行 71-01/02 的精确求和会显红——
 * 那是清理未执行的环境信号（判责=残留），不是本用例放宽断言的理由。
 */

type Row = Record<string, unknown>;

/** BI 双层信封钉桩：ApiResponse.data 必须是 {code:0,message,data:...}，内层 data 直取，禁 ?? 兜底 */
function unwrapBi<T>(raw: unknown, endpoint: string): T {
  const env = raw as { code?: unknown; message?: unknown; data?: unknown } | null;
  expect(
    env && typeof env === 'object' && !Array.isArray(env),
    `${endpoint}：应返回 BiResponse 对象 {code,message,data}，实际 ${JSON.stringify(raw)}`
  ).toBe(true);
  if (!env) {
    // 显式抛错而非继续解引用：类型收窄 + 失败信息带端点名（不制造 TypeError 噪音）
    throw new Error(`${endpoint}：BiResponse 信封缺失，实际 ${JSON.stringify(raw)}`);
  }
  expect(
    env.code,
    `${endpoint}：BiResponse.code 应=0（types.rs:26 success 构造），实际 ${JSON.stringify(raw)}`
  ).toBe(0);
  expect(
    'data' in env,
    `${endpoint}：BiResponse.data 键必须存在（禁缺失即当空），实际 ${JSON.stringify(raw)}`
  ).toBe(true);
  return env.data as T;
}

function requireArray(data: unknown, endpoint: string): Row[] {
  expect(Array.isArray(data), `${endpoint}：出参必须为数组，实际 ${JSON.stringify(data)}`).toBe(
    true
  );
  return data as Row[];
}

function requireItemsEnvelope(data: unknown, endpoint: string): { items: Row[]; total: number } {
  const env = data as { items?: unknown; total?: unknown; page?: unknown; page_size?: unknown };
  expect(
    Array.isArray(env?.items),
    `${endpoint}：分页唯一形状 {items,total,page,page_size}，items 必须为数组，实际 ${JSON.stringify(data)}`
  ).toBe(true);
  expect(typeof env?.total, `${endpoint}：total 必须为数字（禁 ?? 0 兜底）`).toBe('number');
  expect(typeof env?.page, `${endpoint}：page 键必须存在`).toBe('number');
  expect(typeof env?.page_size, `${endpoint}：page_size 键必须存在`).toBe('number');
  return { items: env.items as Row[], total: Number(env.total) };
}

interface SeedOrder {
  id: number;
  orderNo: string;
  day: string;
  total: number;
  status: string;
}

/** 本批共享的三张正向订单规格（金额与期望和全部为二进制精确小数：.00/.25/.50/.75；
 *  qty 合计 23，总额 7309.25。`total` 是**权威期望值**（后端 Decimal round_dp(2) 链的精确和），
 *  禁止用 JS Number 乘加现算期望——非二进制精确的中间值会引入伪误差） */
interface OrderSpec {
  day: string;
  status: string;
  items: Array<{ qty: string; price: string }>;
  total: number;
}
const ORDERS_SPEC: OrderSpec[] = [
  {
    day: '2096-11-05',
    status: 'pending',
    items: [
      { qty: '10', price: '500.00' },
      { qty: '2', price: '2.25' },
    ],
    total: 5004.5,
  },
  {
    day: '2096-11-06',
    status: 'pending',
    items: [{ qty: '8', price: '250.50' }],
    total: 2004,
  },
  {
    day: '2096-11-19',
    status: 'pending',
    items: [{ qty: '3', price: '100.25' }],
    total: 300.75,
  },
];
const EXPECT_TOTAL_SUM = 7309.25;
const EXPECT_QTY_SUM = 23;
const EXPECT_ORDER_COUNT = 3;

async function seedCustomer(page: Page, tag: string): Promise<number> {
  const code = genCode('E2E71C');
  const res = await apiCall<Row>(page, 'POST', '/crm/customers', {
    customer_code: code,
    customer_name: `71号${tag}客户_${code}`,
    contact_phone: '13800007101',
  });
  const id = Number(res.data?.id);
  expect(id, `[71] 客户创建应回 id：${JSON.stringify(res)}`).toBeGreaterThan(0);
  return id;
}

async function seedOrder(page: Page, customerId: number, spec: OrderSpec): Promise<SeedOrder> {
  const ctx = getCtx();
  const productId = ctx.productIds[0];
  if (!productId) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
  const res = await apiCall<Row>(page, 'POST', '/sales/orders', {
    customer_id: customerId,
    // 日期取正午 UTC：任何 |时区偏移|≤12 的会话时区下 to_char 日桶不落邻日
    order_date: `${spec.day}T12:00:00Z`,
    status: spec.status,
    items: spec.items.map((it, i) => ({
      product_id: productId,
      quantity: it.qty,
      unit_price: it.price,
      notes: `E2E71-${spec.day}-${i}`,
    })),
  });
  const id = Number(res.data?.id);
  expect(id, `[71] 订单创建应回 id：${JSON.stringify(res)}`).toBeGreaterThan(0);
  const orderNo = String(res.data?.order_no);
  expect(orderNo, `订单应回 order_no：${JSON.stringify(res.data)}`).toBeTruthy();
  // total 取 spec 权威值（后端 Decimal 精确和），不在 JS 里现算乘加，杜绝伪误差
  return { id, orderNo, day: spec.day, total: spec.total, status: spec.status };
}

/** 显式硬删 + 200 确认；调用方收集成功删除的 id，finally 只兜底未删成的 */
async function deleteOrder(page: Page, id: number): Promise<void> {
  const res = await apiCall(page, 'DELETE', `/sales/orders/${id}`);
  expect(res.code, `清理：删除销售订单 ${id} 应成功`).toBe(200);
}

/** 用例级三段式：seed → 断言 → 显式清理（正常路径）；finally 兜底清理（失败路径） */
async function withSeededOrders(
  page: Page,
  tag: string,
  specs: OrderSpec[],
  body: (customerId: number, orders: SeedOrder[]) => Promise<void>
): Promise<void> {
  const customerId = await seedCustomer(page, tag);
  const orders: SeedOrder[] = [];
  const deleted = new Set<number>();
  try {
    for (const s of specs) {
      orders.push(await seedOrder(page, customerId, s));
    }
    await body(customerId, orders);
    for (const o of orders) {
      await deleteOrder(page, o.id);
      deleted.add(o.id);
    }
    for (const o of orders) {
      const gone = await apiCallExpectFail(page, 'GET', `/sales/orders/${o.id}`);
      expect(
        gone.status,
        `删后回读无痕：GET /sales/orders/${o.id} 应 404，实际 ${JSON.stringify(gone)}`
      ).toBe(404);
    }
  } finally {
    for (const o of orders) {
      if (!deleted.has(o.id)) {
        await tryCleanup(page, 'DELETE', `/sales/orders/${o.id}`, `[71] 兜底清理订单 ${o.id}`);
      }
    }
    await tryCleanup(
      page,
      'DELETE',
      `/crm/customers/${customerId}`,
      `[71] 兜底清理客户 ${customerId}`
    );
  }
}

test.describe.serial('71 BI 销售聚合数值真伪（聚合==明细求和，零容差）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('71-01 by-time 逐日聚合 == 明细逐单求和；分页列表 items/total/page_size 一致；删后聚合无痕', async ({
    page,
  }) => {
    await withSeededOrders(page, '聚合', ORDERS_SPEC, async (customerId, orders) => {
      // ① 写后回读明细真值：逐单 detail 的 total_amount == 行求和（后端 round_dp(2) 链）
      for (const o of orders) {
        const detail = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${o.id}`);
        expect(Number(detail.id), `详情回读 id=${o.id}`).toBe(o.id);
        expect(detail.status, `详情回读 status=${o.status}`).toBe(o.status);
        expect(String(detail.order_date), `详情回读日期应含 ${o.day}`).toContain(o.day);
        expect(Number(detail.total_amount), `详情回读 total_amount=${o.total}`).toBe(o.total);
        const items = requireArray(detail.items, `/sales/orders/${o.id}.items`);
        expect(
          items.reduce((s, it) => s + Number(it.total_amount), 0),
          `订单 ${o.id} 行 total_amount 求和应=订单头 ${o.total}`
        ).toBe(o.total);
      }

      // ② 分页明细读回：PaginatedResponse 形状 + total/page_size 与 items.length 一致性
      const listEp = `/sales/orders?customer_id=${customerId}&page=1&page_size=50&start_date=2096-11-01&end_date=2096-11-30`;
      const listed = requireItemsEnvelope(await apiCallRaw(page, 'GET', listEp), listEp);
      expect(listed.total, '客户+窗口过滤后 total 应恰=自建 3 单').toBe(EXPECT_ORDER_COUNT);
      expect(listed.items.length, '单页内 items.length 应=total').toBe(listed.total);
      expect(listed.items.length <= 50, 'items.length 不得超过请求 page_size').toBe(true);
      expect(
        listed.items.reduce((s, it) => s + Number(it.total_amount), 0),
        '分页明细求和应=7309.25'
      ).toBe(EXPECT_TOTAL_SUM);

      // ③ 聚合端点 by-time（日粒度）：每个返回桶必须逐日精确等于当日明细和
      const byTimeEp =
        '/bi/sales/by-time?start_date=2096-11-01&end_date=2096-11-30&granularity=day';
      const points = requireArray(
        unwrapBi<unknown>(await apiCallRaw(page, 'GET', byTimeEp), byTimeEp),
        byTimeEp
      );
      const expectedByDay: Record<string, number> = {};
      for (const o of orders) expectedByDay[o.day] = o.total;
      expect(points.length, `by-time 应恰返回 3 个日桶，实际 ${JSON.stringify(points)}`).toBe(
        EXPECT_ORDER_COUNT
      );
      for (const p of points) {
        const period = String(p.period);
        expect(
          expectedByDay[period],
          `by-time 返回了非本用例种下的日桶 ${period}（窗口 2096 应纯净）`
        ).toBeTruthy();
        expect(Number(p.total_amount), `日桶 ${period} 聚合金额应精确=明细和`).toBe(
          expectedByDay[period]
        );
        expect(Number(p.order_count), `日桶 ${period} 订单数应=1`).toBe(1);
      }
      expect(
        points.reduce((s, p) => s + Number(p.total_amount), 0),
        '聚合求和 == 明细求和（根因锚点：数字是真的，不是端点活着）'
      ).toBe(EXPECT_TOTAL_SUM);
      expect(
        points.reduce((s, p) => s + Number(p.quantity), 0),
        '数量聚合 == 行数量总和 23'
      ).toBe(EXPECT_QTY_SUM);
      expect(
        points.reduce((s, p) => s + Number(p.order_count), 0),
        '订单数聚合 == 3'
      ).toBe(EXPECT_ORDER_COUNT);
    });

    // ④ 删后聚合无痕：换一个缓存键（end 提前一天，仍覆盖全部种单日）复查应为空序列，
    //    证明 2096-11 窗口未残留任何计入聚合的行
    const cleanEp = '/bi/sales/by-time?start_date=2096-11-01&end_date=2096-11-29&granularity=day';
    const cleanPoints = requireArray(
      unwrapBi<unknown>(await apiCallRaw(page, 'GET', cleanEp), cleanEp),
      cleanEp
    );
    expect(cleanPoints.length, `删后窗口应无任何聚合桶，实际 ${JSON.stringify(cleanPoints)}`).toBe(
      0
    );
  });

  test('71-02 跨聚合源一致：year-to-month 月度桶 == customer-to-order 明细和 == 订单头求和', async ({
    page,
  }) => {
    await withSeededOrders(page, '钻取', ORDERS_SPEC, async (customerId, orders) => {
      // 年→月：12 个月完整序列，仅 11 月承载本用例数据，其余月必须为 0
      const ytmEp = '/bi/sales/drilldown/year-to-month?year=2096';
      const months = requireArray(
        unwrapBi<unknown>(await apiCallRaw(page, 'GET', ytmEp), ytmEp),
        ytmEp
      );
      expect(months.length, `year-to-month 应返回完整 12 月序列（drilldown.rs:77-110）`).toBe(12);
      const nov = months.find(m => m.period === '2096-11');
      expect(nov, `应含 2096-11 桶，实际 ${JSON.stringify(months)}`).toBeTruthy();
      expect(Number(nov?.total_amount), '月度桶 == 明细和 7309.25').toBe(EXPECT_TOTAL_SUM);
      expect(Number(nov?.order_count), '月度桶订单数==3').toBe(EXPECT_ORDER_COUNT);
      expect(
        months.reduce((s, m) => s + Number(m.total_amount), 0),
        '全年求和 == 明细和（其余 11 桶必须为 0，含负值/串月即红）'
      ).toBe(EXPECT_TOTAL_SUM);

      // 客户→订单：返回的必须是且仅是本次自建 3 单的逐单明细
      const c2oEp = `/bi/sales/drilldown/customer-to-order/${customerId}`;
      const drill = unwrapBi<{ customer_id: number; orders: Row[] }>(
        await apiCallRaw(page, 'GET', c2oEp),
        c2oEp
      );
      expect(Number(drill.customer_id), '钻取应回显请求的 customer_id').toBe(customerId);
      const rows = requireArray(drill.orders, `${c2oEp}.orders`);
      expect(rows.length, `钻取明细应恰 3 行，实际 ${JSON.stringify(rows)}`).toBe(
        EXPECT_ORDER_COUNT
      );
      const seededIds = new Set(orders.map(o => o.id));
      for (const r of rows) {
        expect(
          seededIds.has(Number(r.order_id)),
          `钻取返回了非本用例订单 id=${r.order_id}：${JSON.stringify(r)}`
        ).toBe(true);
      }
      for (const o of orders) {
        const r = rows.find(x => Number(x.order_id) === o.id);
        expect(r, `钻取应命中自建订单 ${o.id}（${o.orderNo}）`).toBeTruthy();
        expect(Number(r?.amount), `钻取行金额==订单头 ${o.total}`).toBe(o.total);
        expect(String(r?.date), `钻取行日期==种下日 ${o.day}`).toBe(o.day);
      }
      expect(
        rows.reduce((s, r) => s + Number(r.amount), 0),
        '钻取明细求和 == 月度聚合桶（聚合==明细，两路 SQL 交叉验证）'
      ).toBe(EXPECT_TOTAL_SUM);
    });
  });

  test('71-03 状态排除契约：draft/cancelled 不得进聚合与钻取（预置判红——大小写门失守缺陷钉桩）', async ({
    page,
  }) => {
    // 契约来源：sales.rs:10-11 业务规则注释 + SQL 意图 NOT IN ('CANCELLED','DRAFT')。
    // 现状缺陷：写入方 so_status 是小写（models/status/sales.rs:16/28），大写比较恒不命中
    // → 若本例判红且窗口聚合值=6556.25（三桶全进），即为该缺陷实证；禁止把期望改成"含脏状态"来洗绿。
    const specs: OrderSpec[] = [
      {
        day: '2096-12-05',
        status: 'pending',
        items: [{ qty: '10', price: '100.00' }],
        total: 1000,
      },
      {
        day: '2096-12-06',
        status: 'draft',
        items: [{ qty: '10', price: '222.25' }],
        total: 2222.5,
      },
      {
        day: '2096-12-07',
        status: 'cancelled',
        items: [{ qty: '5', price: '666.75' }],
        total: 3333.75,
      },
    ];
    await withSeededOrders(page, '状态门', specs, async (customerId, orders) => {
      const ep = '/bi/sales/by-time?start_date=2096-12-01&end_date=2096-12-31&granularity=day';
      const points = requireArray(unwrapBi<unknown>(await apiCallRaw(page, 'GET', ep), ep), ep);
      expect(
        points.length,
        `仅 pending 订单应成桶（draft/cancelled 进桶=排除门失守），实际 ${JSON.stringify(points)}`
      ).toBe(1);
      expect(String(points[0]?.period), '唯一桶应为 2096-12-05（pending 日）').toBe('2096-12-05');
      expect(
        points.reduce((s, p) => s + Number(p.total_amount), 0),
        `窗口聚合必须=1000（若=6556.25 即大写比较缺陷实证）`
      ).toBe(1000);

      const c2oEp = `/bi/sales/drilldown/customer-to-order/${customerId}`;
      const drill = unwrapBi<{ orders: Row[] }>(await apiCallRaw(page, 'GET', c2oEp), c2oEp);
      const rows = requireArray(drill.orders, `${c2oEp}.orders`);
      const dirtyIds = orders.filter(o => o.status !== 'pending').map(o => o.id);
      for (const r of rows) {
        expect(
          dirtyIds.includes(Number(r.order_id)),
          `钻取混入脏状态订单 id=${r.order_id}（drilldown.rs:243 同一大小写门缺陷面）`
        ).toBe(false);
      }
      expect(rows.length, '钻取应只含 pending 的 1 单').toBe(1);
      expect(Number(rows[0]?.amount), '钻取金额=1000').toBe(1000);
    });
  });

  test('71-04 BI 参数非法值与未知粒度回退契约', async ({ page }) => {
    // 日期倒挂：sales.rs:34-36 validation_displayable → 400 + VALIDATION_ERROR + 可读原因
    const reversed = await apiCallExpectFail(
      page,
      'GET',
      '/bi/sales/by-time?start_date=2096-11-30&end_date=2096-11-01&granularity=day'
    );
    expect(reversed.status, `日期倒挂应 400，实际 ${JSON.stringify(reversed)}`).toBe(400);
    expect(failureCode(reversed), '日期倒挂机器码=VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    expect(String(reversed.message), '拒绝原因必须外显').toContain('结束日期不能早于开始日期');

    // 必填 start_date 缺失 → 4xx（Query<ByTimeQuery> 强类型必填，不许静默全量）
    const missing = await apiCallExpectFail(
      page,
      'GET',
      '/bi/sales/by-time?end_date=2096-11-30&granularity=day'
    );
    expect(
      missing.status >= 400 && missing.status < 500,
      `缺 start_date 应 4xx，实际 ${JSON.stringify(missing)}`
    ).toBe(true);

    // 非法年份（drilldown.rs:32 边界 1900..=2999 之外）→ 400 VALIDATION_ERROR
    const badYear = await apiCallExpectFail(
      page,
      'GET',
      '/bi/sales/drilldown/year-to-month?year=1899'
    );
    expect(badYear.status, `year=1899 应 400，实际 ${JSON.stringify(badYear)}`).toBe(400);
    expect(failureCode(badYear), '非法年份机器码=VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );

    // 非法月份（month-to-day 守卫 1..=12）
    const badMonth = await apiCallExpectFail(
      page,
      'GET',
      '/bi/sales/drilldown/month-to-day?year=2096&month=13'
    );
    expect(
      badMonth.status >= 400 && badMonth.status < 500,
      `month=13 应 4xx，实际 ${JSON.stringify(badMonth)}`
    ).toBe(true);

    // 未知 granularity 不报错而是回退月桶（sales.rs:98-107 `_ => YYYY-MM`）。
    // 缓存纪律：空窗检查与种单后的检查**必须用不同参数键**（by-time 5min TTL，
    // 同键二次查询会命中空窗缓存造成假红）——空窗走从未种数的 2096-02，种单走 2096-09。
    const empty = requireArray(
      unwrapBi<unknown>(
        await apiCallRaw(
          page,
          'GET',
          '/bi/sales/by-time?start_date=2096-02-01&end_date=2096-02-29&granularity=fortnight'
        ),
        'by-time granularity 回退(空窗)'
      ),
      'by-time granularity 回退(空窗)'
    );
    expect(empty.length, '空窗回退应返回空数组而不是报错').toBe(0);

    await withSeededOrders(
      page,
      '回退',
      [
        {
          day: '2096-09-10',
          status: 'pending',
          items: [{ qty: '5', price: '300.25' }],
          total: 1501.25,
        },
      ],
      async (_customerId, orders) => {
        const fbEp =
          '/bi/sales/by-time?start_date=2096-09-01&end_date=2096-09-30&granularity=fortnight';
        const buckets = requireArray(
          unwrapBi<unknown>(await apiCallRaw(page, 'GET', fbEp), fbEp),
          fbEp
        );
        expect(buckets.length, `回退月桶应恰 1 个，实际 ${JSON.stringify(buckets)}`).toBe(1);
        expect(String(buckets[0]?.period), '回退桶形态=YYYY-MM').toBe('2096-09');
        expect(Number(buckets[0]?.total_amount), '回退桶金额==明细和 1501.25').toBe(
          orders[0].total
        );
      }
    );
  });
});
