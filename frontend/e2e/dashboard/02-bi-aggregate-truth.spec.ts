// BI/销售聚合报表 **数值真值** E2E — dashboard/02
// 目标：把"报表聚合结果等于用例内自建 seed 的真实汇总"钉成硬断言，
//       替代只断 200 / 只断行数>0 / 只断 toast 的假绿（对齐覆盖审计结论）。
// 逐端点对后端源码核实（非编造）：
//   GET /crm/customers/{id}/summary  （crm.rs customers + crm_customer_enhancement_routes）
//     service get_customer_relation_summary（cust.rs:88-142）用**数据库 SUM/COUNT 聚合**：
//       total_orders = COUNT(sales_order WHERE customer_id=?)
//       total_order_amount = SUM(total_amount)
//       total_opportunities = COUNT(crm_opportunity WHERE customer_id=?)
//       follow_up_count = COUNT(customer_followup WHERE customer_id=?)
//   → 用全新客户 id 隔离，只写入用例自建 seed，聚合真值可精确预期。
//   GET /sales/orders?customer_id=? 列表回读，逐行 total_amount 相加应等于 summary 的 SUM（交叉校验）。
//   GET /crm/leads/funnel-report 销售漏斗聚合（handlers/crm_handler.rs:1251 →
//     services/crm/lead.rs:943 lead_funnel_report）——跨全库计数，仅严格健康 + 结构健全性断言
//     （不做 seed 真值：全局计数受并行分片共享库影响，钉死等值会造成假红/假绿，故如实标注覆盖边界）。
//   已注册端点统一走 verifyEndpointHealthy(strict)；xlsx 导出走 verifyDownloadEndpointHealthy（JSON helper 会对 200 xlsx 误抛假红）。
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  ensureTestEntities,
  getCtx,
  genCode,
  tryCleanup,
  verifyEndpointHealthy,
  verifyDownloadEndpointHealthy,
} from '../flow/helpers';

/** rust_decimal 序列化为字符串（"1000.0000"）或 number，统一 String→Number；
 * 缺键/异常形状时 Number(String(undefined))=NaN，聚合断言必然判红，不做 ?? 0 兜底 */
const toNum = (v: unknown): number => Number(String(v));

interface Summary {
  customer_id: number;
  total_leads: number;
  total_opportunities: number;
  total_orders: number;
  total_order_amount: unknown;
  follow_up_count: number;
}

test.describe('BI/销售聚合报表 - 02 数值真值', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('客户聚合 summary 的订单数/金额/商机数/跟进数 = 用例内 seed 真实汇总', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    expect(productId, '前置：需至少一个产品用于订单明细').toBeTruthy();

    // 全新客户：所有聚合只覆盖本用例 seed，无历史/并行数据干扰
    const cust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `E2E BI聚合客户 ${genCode('BI')}`,
    });
    const customerId = cust.data?.id;
    expect(customerId, `建客户失败：${JSON.stringify(cust)}`).toBeTruthy();

    const orderIds: number[] = [];
    const oppIds: number[] = [];

    try {
      await verifyEndpointHealthy(page, `/crm/customers/${customerId}/summary`);

      // ---- seed 三笔销售订单，金额已知（行金额=qty*price，无折扣/税，round 2）----
      const seedSpecs: Array<[number, number]> = [
        [10, 100], // 1000
        [5, 200], // 1000
        [3, 150], // 450
      ];
      let expectedAmountSum = 0;
      for (const [qty, price] of seedSpecs) {
        const o = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
          customer_id: customerId,
          items: [{ product_id: productId, quantity: qty, unit_price: price }],
        });
        expect(o.data?.id, `建销售订单失败：${JSON.stringify(o)}`).toBeTruthy();
        orderIds.push(o.data!.id!);
        expectedAmountSum += qty * price;
      }

      // ---- seed 两个商机 ----
      for (let i = 0; i < 2; i++) {
        const oppNo = `${genCode('E2E-BIOPP')}-${i}`;
        const opp = await apiCall<{ id?: number }>(page, 'POST', '/crm/opportunities', {
          opportunity_no: oppNo,
          opportunity_name: oppNo,
          customer_id: customerId,
          opportunity_stage: 'QUALIFICATION',
          estimated_amount: 8000,
          win_probability: 50,
          expected_close_date: '2026-12-31',
        });
        expect(opp.data?.id, `建商机失败：${JSON.stringify(opp)}`).toBeTruthy();
        oppIds.push(opp.data!.id!);
      }

      // ---- seed 一条跟进 ----
      await apiCall(page, 'POST', `/crm/customers/${customerId}/follow-ups`, {
        type: 'phone',
        content: 'E2E BI 聚合跟进',
        next_follow_date: '2026-12-01',
      });

      // ==== 聚合真值断言（GET summary 服务端 SUM/COUNT）====
      const summary = await apiCallRaw<Summary>(
        page,
        'GET',
        `/crm/customers/${customerId}/summary`
      );
      expect(
        summary.total_orders,
        `聚合订单数应等于 seed 条数 3，实际 ${summary.total_orders}`
      ).toBe(orderIds.length);
      expect(
        toNum(summary.total_order_amount),
        `聚合金额 SUM 应等于 seed 汇总 ${expectedAmountSum}，实际 ${summary.total_order_amount}`
      ).toBeCloseTo(expectedAmountSum, 2);
      expect(
        summary.total_opportunities,
        `聚合商机数应等于 seed 条数 2，实际 ${summary.total_opportunities}`
      ).toBe(oppIds.length);
      expect(
        summary.follow_up_count,
        `聚合跟进数应等于 seed 条数 1，实际 ${summary.follow_up_count}`
      ).toBe(1);

      // ==== 交叉校验：列表逐行金额相加 == summary 的 SUM（同一 DB 两条读路径口径一致）====
      // GET /sales/orders 列表返回 PaginatedResponse{items,total,page,page_size}（utils/response.rs:34），
      // data 必含 items 数组——不做 `?? []` 兜底。
      const list = await apiCallRaw<{ items: Array<{ id: number; total_amount: unknown }> }>(
        page,
        'GET',
        `/sales/orders?customer_id=${customerId}&page=1&page_size=100`
      );
      expect(Array.isArray(list.items), '销售订单列表响应缺 data.items 数组').toBe(true);
      const rows = list.items.filter(o => orderIds.includes(o.id));
      // 三条 seed 订单都应出现在客户过滤列表里（回读真实落库，非只看聚合数）
      expect(rows.length, `列表回读 seed 订单数应为 ${orderIds.length}，实际 ${rows.length}`).toBe(
        orderIds.length
      );
      const listAmountSum = rows.reduce((acc, o) => acc + toNum(o.total_amount), 0);
      expect(
        listAmountSum,
        `列表逐行 total_amount 汇总应等于 summary SUM=${expectedAmountSum}，实际 ${listAmountSum}`
      ).toBeCloseTo(expectedAmountSum, 2);
    } finally {
      for (const id of orderIds)
        await tryCleanup(page, 'DELETE', `/sales/orders/${id}`, 'sales_order');
      for (const id of oppIds)
        await tryCleanup(page, 'DELETE', `/crm/opportunities/${id}`, 'opportunity');
      await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, 'customer');
    }
  });

  test('再补一笔订单后聚合金额随之增长（证明聚合非缓存回声、随落库变化）', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    expect(productId, '前置：需至少一个产品').toBeTruthy();

    const cust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `E2E BI增量客户 ${genCode('BI2')}`,
    });
    const customerId = cust.data?.id;
    expect(customerId, '建客户失败').toBeTruthy();

    const orderIds: number[] = [];
    try {
      // 基线：一笔 600
      const o1 = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
        customer_id: customerId,
        items: [{ product_id: productId, quantity: 6, unit_price: 100 }],
      });
      orderIds.push(o1.data!.id!);
      const base = await apiCallRaw<Summary>(page, 'GET', `/crm/customers/${customerId}/summary`);
      expect(toNum(base.total_order_amount), '基线聚合金额应为 600').toBeCloseTo(600, 2);
      expect(base.total_orders, '基线聚合订单数应为 1').toBe(1);

      // 追加一笔 400 → 聚合应变 1000、2 单
      const o2 = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
        customer_id: customerId,
        items: [{ product_id: productId, quantity: 4, unit_price: 100 }],
      });
      orderIds.push(o2.data!.id!);
      const after = await apiCallRaw<Summary>(page, 'GET', `/crm/customers/${customerId}/summary`);
      expect(after.total_orders, '追加后聚合订单数应为 2').toBe(2);
      expect(
        toNum(after.total_order_amount),
        `追加后聚合金额应为 1000（600+400），实际 ${after.total_order_amount}`
      ).toBeCloseTo(1000, 2);
    } finally {
      for (const id of orderIds)
        await tryCleanup(page, 'DELETE', `/sales/orders/${id}`, 'sales_order');
      await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, 'customer');
    }
  });

  test('销售漏斗聚合报表严格健康 + 结构健全（跨全库计数，钉数值区间非只断200）', async ({
    page,
  }) => {
    await ensureTestEntities(page);
    const today = new Date().toISOString().slice(0, 10);

    await verifyEndpointHealthy(
      page,
      `/crm/leads/funnel-report?start_date=${today}&end_date=${today}`
    );

    const funnel = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/leads/funnel-report?start_date=${today}&end_date=${today}`
    );
    // 所有计数字段必须是非负整数（不是缺键/NaN），比率字段是 [0,100] 的有限数
    const intKeys = [
      'total_leads',
      'converted_leads',
      'total_opportunities',
      'won_opportunities',
      'total_customers',
      'total_orders',
    ];
    for (const k of intKeys) {
      const v = funnel?.[k];
      expect(
        typeof v === 'number' && Number.isInteger(v as number) && (v as number) >= 0,
        `漏斗字段 ${k} 应为非负整数，实际 ${JSON.stringify(v)}`
      ).toBe(true);
    }
    const rateKeys = [
      'lead_to_opp_rate',
      'opp_to_customer_rate',
      'opp_to_order_rate',
      'overall_conversion_rate',
    ];
    for (const k of rateKeys) {
      const v = funnel?.[k];
      expect(
        typeof v === 'number' &&
          Number.isFinite(v as number) &&
          (v as number) >= 0 &&
          (v as number) <= 100,
        `漏斗比率 ${k} 应为 [0,100] 有限数，实际 ${JSON.stringify(v)}`
      ).toBe(true);
    }
  });

  test('销售分析报表二进制导出严格健康（xlsx 走 download helper，非 JSON 解析假红）', async ({
    page,
  }) => {
    await ensureTestEntities(page);
    // routes/crm.rs sales_analysis export → xlsx_response（二进制），JSON helper 会误抛，必须用 download 校验
    await verifyDownloadEndpointHealthy(page, '/crm/sales-analysis/export');
  });
});
