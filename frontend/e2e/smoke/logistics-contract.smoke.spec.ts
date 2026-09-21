// 物流运单状态契约冒烟测试
//
// 覆盖后端运单接口的三条真实契约（此前的实现里它们全部不成立）：
//   1. 列表返回分页结构 {items,total,page,page_size}，且每条带关联销售订单号
//   2. 状态/关键字筛选真正下推到 SQL（用「三个状态分片之和 = 不分片的总数」这一
//      与数据量无关的恒等式验证，筛选被忽略时该式必然不成立）
//   3. 状态取值只认后端状态机 IN_TRANSIT/DELIVERED/SIGNED，非法取值直接 4xx
// 另加一条 UI 断言：状态筛选下拉只出现三个真实状态，防止前端词表再度漂移。
import { test, expect } from '../diagnose-fixture';
import type { APIRequestContext } from '@playwright/test';
import { gotoWithRetry } from './_goto';

// 与前端 src/api/request.ts 同源的相对路径：请求经 vite/preview 的 /api/ 代理转发到后端，
// 与产品真实链路一致，也避免在测试里重复配置后端地址
const API_PREFIX = '/api/v1/erp';

/** 后端 logistics_waybill 状态机的全部合法值 */
const WAYBILL_STATUSES = ['IN_TRANSIT', 'DELIVERED', 'SIGNED'];

interface WaybillListData {
  items: Array<{ status: string; order_id: number; order_no?: string }>;
  total: number;
  page: number;
  page_size: number;
}

async function listWaybills(
  request: APIRequestContext,
  query: string
): Promise<{ status: number; body: { data?: WaybillListData } }> {
  const res = await request.get(`${API_PREFIX}/inventory/logistics?${query}`);
  const body = (await res.json()) as { data?: WaybillListData };
  return { status: res.status(), body };
}

test.describe('物流运单状态与列表契约', () => {
  test('列表返回分页结构，状态取值全部在状态机内', async ({ request }) => {
    const { status, body } = await listWaybills(request, 'page=1&page_size=5');
    expect(status).toBe(200);
    expect(body.data, '列表响应缺少 data 对象').toBeTruthy();
    const data = body.data as WaybillListData;
    // 空列表也必须返回 items 数组与分页元数据，不能整体缺键
    expect(Array.isArray(data.items), 'items 必须是数组').toBe(true);
    expect(typeof data.total).toBe('number');
    expect(data.page).toBe(1);
    expect(data.page_size).toBe(5);
    for (const item of data.items) {
      expect(WAYBILL_STATUSES, `运单出现状态机外的取值：${item.status}`).toContain(item.status);
      // 关联订单号由后端回查 sales_orders 补齐，前端不再自造运单号字段
      expect(typeof item.order_no, '运单缺少关联销售订单号').toBe('string');
    }
  });

  test('状态筛选真正生效：三个状态分片之和等于不分片总数', async ({ request }) => {
    const all = await listWaybills(request, 'page=1&page_size=100');
    expect(all.status).toBe(200);
    const totalAll = (all.body.data as WaybillListData).total;

    let sumOfParts = 0;
    for (const value of WAYBILL_STATUSES) {
      const part = await listWaybills(request, `page=1&page_size=100&status=${value}`);
      expect(part.status, `按状态 ${value} 筛选应被接受`).toBe(200);
      const data = part.body.data as WaybillListData;
      for (const item of data.items) {
        expect(item.status, `筛选 ${value} 却返回了 ${item.status}`).toBe(value);
      }
      sumOfParts += data.total;
    }
    expect(sumOfParts, '分状态计数之和与总数不一致，说明状态筛选未下推').toBe(totalAll);
  });

  test('非法状态取值被拒绝', async ({ request }) => {
    const res = await request.get(
      `${API_PREFIX}/inventory/logistics?page=1&page_size=5&status=shipped`
    );
    expect(res.status(), '前端旧词表的小写状态不应被后端接受').toBeGreaterThanOrEqual(400);
    expect(res.status()).toBeLessThan(500);
  });

  test('页面状态筛选下拉只列出三个真实状态', async ({ page }) => {
    await gotoWithRetry(page, '/logistics');
    const statusSelect = page.locator('.el-form-item', { hasText: '状态' }).first();
    await statusSelect.locator('.el-select').click();
    // 同一时刻只有一个下拉展开，可见项即状态选项
    const dropdown = page.locator('.el-select-dropdown:visible');
    await expect(dropdown).toBeVisible({ timeout: 30_000 });
    const texts = await dropdown.locator('.el-select-dropdown__item:visible').allTextContents();
    expect(texts.map(text => text.trim())).toEqual(['运输中', '已送达', '已签收']);
  });
});
