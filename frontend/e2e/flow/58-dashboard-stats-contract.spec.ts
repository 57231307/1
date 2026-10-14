// 仪表盘概览统计契约锁（真实后端）：overview 必须成功返回并产出前端卡片所需的聚合。
// 回归自"统计全 0"：亚秒级假超时把合法聚合判成 5xx 致前端回落全零；以及前端绑定了后端
// 不产出的键致相应卡恒 0。此处断言状态码与键存在/类型，不依赖具体计数，避免脆断。
import { test, expect } from '../diagnose-fixture';
import { loginViaUI, API_BASE, API_PREFIX } from './helpers';

test.describe('仪表盘概览统计契约', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('overview 返回 200 且含前端所需聚合', async ({ page }) => {
    const resp = await page.request.get(`${API_BASE}${API_PREFIX}/dashboard/overview`);
    // 回归点1：真实聚合应在超时预算内 200，不得被降级为 5xx（那会让前端回落全零）。
    expect(resp.status(), 'overview 应成功返回，不得因超时降级为 5xx').toBe(200);

    const body = await resp.json();
    const data = body.data ?? {};
    // 回归点2：前端卡片绑定的键后端必须真实产出，否则该卡恒 0。
    expect(data, '缺 total_products').toHaveProperty('total_products');
    expect(data, '缺 customer_count').toHaveProperty('customer_count');
    expect(data, '缺 inventory_total').toHaveProperty('inventory_total');
    // 库存主口径改为公斤合计，卡片绑 inventory_weight_kg，须由 overview 产出。
    expect(data, '缺 inventory_weight_kg').toHaveProperty('inventory_weight_kg');
    expect(data, '缺 today_orders').toHaveProperty('today_orders');
    // 面料总数卡改绑 fabric_count（坯布+成品布，排除辅料），须由 overview 产出。
    expect(data, '缺 fabric_count').toHaveProperty('fabric_count');

    expect(typeof data.customer_count, 'customer_count 应为计数(数值)').toBe('number');
    expect(typeof data.today_orders, 'today_orders 应为计数(数值)').toBe('number');
    expect(typeof data.fabric_count, 'fabric_count 应为计数(数值)').toBe('number');
    // 库存总量以米数合计承载（Decimal→String），数值亦可，二者皆接受。
    expect(['number', 'string'], 'inventory_total 应为数值或数值字符串').toContain(
      typeof data.inventory_total
    );
  });
});
