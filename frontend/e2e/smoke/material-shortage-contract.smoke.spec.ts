// 缺料预警取值与列表契约冒烟测试
//
// 此前的前端词表（severity: critical/high/medium/low、status: pending/notified/resolved）
// 与后端真实取值完全不同，且列表根本不返回状态字段，页面筛选与状态操作全是假控件。
// 本用例钉住三条真实契约：
//   1. 列表行的 level 只可能是 Critical/Severe/Warning/Normal，status 只可能是
//      identified/purchase_request/purchase_order/received/resolved（或 null=预警未落库）
//   2. 级别筛选真正生效（四个级别分片之和 = 不分片总数）
//   3. 越界取值（旧前端词表里换了词根的写法）直接 4xx，而不是被静默当成「无数据」；
//      仅大小写与规范码不同的写法（critical vs Critical）按后端 eq_ignore_ascii_case 约定
//      归一接受，但归一后必须与规范码返回同一结果集，避免出现第二套筛选口径
// 另加一条 UI 断言：筛选下拉只列出后端真实取值。
import { test, expect } from '../diagnose-fixture';
import type { APIRequestContext } from '@playwright/test';
import { gotoWithRetry } from './_goto';

// 与前端 src/api/request.ts 同源：经 vite/preview 的 /api/ 代理转发到后端
const API_PREFIX = '/api/v1/erp';

/** 后端 ShortageLevel::ALL 的取值 */
const SHORTAGE_LEVELS = ['Critical', 'Severe', 'Warning', 'Normal'];

/** 后端 shortage_alert_status::ALL 的取值 */
const SHORTAGE_STATUSES = [
  'identified',
  'purchase_request',
  'purchase_order',
  'received',
  'resolved',
];

/** 后端 ReplenishmentSuggestion::priority 的取值（由缺料级别映射，大写值） */
const REPLENISHMENT_PRIORITIES = ['URGENT', 'HIGH', 'MEDIUM', 'LOW'];

interface ReplenishmentSuggestion {
  material_id: number;
  shortage_quantity: string | number;
  suggested_quantity: string | number;
  priority: string;
}

interface ShortageRow {
  material_id: number;
  alert_no: string | null;
  level: string;
  status: string | null;
  required_quantity: number;
  shortage_quantity: number;
}

interface ShortageListData {
  items: ShortageRow[];
  total: number;
  page: number;
  page_size: number;
}

async function listShortages(
  request: APIRequestContext,
  query: string
): Promise<{ status: number; data: ShortageListData | undefined }> {
  const res = await request.get(`${API_PREFIX}/material-shortage/list?${query}`);
  const body = (await res.json()) as { data?: ShortageListData };
  return { status: res.status(), data: body.data };
}

test.describe('缺料预警取值与列表契约', () => {
  test('列表返回分页结构，级别与状态都在后端取值域内', async ({ request }) => {
    const { status, data } = await listShortages(request, 'page=1&page_size=5');
    expect(status).toBe(200);
    expect(data, '列表响应缺少 data 对象').toBeTruthy();
    const payload = data as ShortageListData;
    expect(Array.isArray(payload.items), 'items 必须是数组').toBe(true);
    expect(typeof payload.total).toBe('number');
    expect(payload.page).toBe(1);
    expect(payload.page_size).toBe(5);
    for (const row of payload.items) {
      expect(typeof row.material_id).toBe('number');
      expect(SHORTAGE_LEVELS, `出现取值域外的缺料级别：${row.level}`).toContain(row.level);
      if (row.status !== null) {
        expect(SHORTAGE_STATUSES, `出现状态机外的缺料状态：${row.status}`).toContain(row.status);
      }
      if (row.alert_no !== null) {
        // 缺料单号由后端 persist_alerts 生成，前端不再自造
        expect(row.alert_no.startsWith('MS-'), `缺料单号格式异常：${row.alert_no}`).toBe(true);
      }
    }
  });

  test('级别筛选真正生效：四个级别分片之和等于不分片总数', async ({ request }) => {
    const all = await listShortages(request, 'page=1&page_size=200');
    expect(all.status).toBe(200);
    const totalAll = (all.data as ShortageListData).total;

    let sumOfParts = 0;
    for (const level of SHORTAGE_LEVELS) {
      const part = await listShortages(request, `page=1&page_size=200&level=${level}`);
      expect(part.status, `按级别 ${level} 筛选应被接受`).toBe(200);
      const payload = part.data as ShortageListData;
      for (const row of payload.items) {
        expect(row.level, `筛选 ${level} 却返回了 ${row.level}`).toBe(level);
      }
      sumOfParts += payload.total;
    }
    expect(sumOfParts, '分级别计数之和与总数不一致，说明级别筛选未生效').toBe(totalAll);
  });

  test('状态筛选只返回该状态的行', async ({ request }) => {
    for (const value of SHORTAGE_STATUSES) {
      const part = await listShortages(request, `page=1&page_size=200&status=${value}`);
      expect(part.status, `按状态 ${value} 筛选应被接受`).toBe(200);
      for (const row of (part.data as ShortageListData).items) {
        expect(row.status, `筛选 ${value} 却返回了 ${row.status}`).toBe(value);
      }
    }
  });

  test('越界取值被拒绝；仅大小写不同的写法按后端约定归一且结果集一致', async ({ request }) => {
    // critical 与规范码 Critical 只差大小写：后端 validate_enum_param 用
    // eq_ignore_ascii_case 匹配并回传常量本身，这是仓库既有的入参约定（不是漏判越界）。
    // 真正越界的是换了词根的旧前端词表（severity 那套 critical/high/medium/low 里，
    // high/medium/low 库里从不存在）与库里不存在的状态（pending/notified）。
    for (const query of [
      'level=high',
      'level=medium',
      'level=low',
      'level=not_a_level',
      'status=pending',
      'status=notified',
    ]) {
      const res = await request.get(`${API_PREFIX}/material-shortage/list?${query}&page_size=5`);
      expect(res.status(), `${query} 不应被后端接受`).toBeGreaterThanOrEqual(400);
      expect(res.status(), `${query} 属入参问题，不应是 5xx`).toBeLessThan(500);
    }

    // 归一必须落到同一个结果集，否则"接受小写"就成了第二套筛选口径
    const canonical = await listShortages(request, 'page=1&page_size=200&level=Critical');
    const lowercase = await listShortages(request, 'page=1&page_size=200&level=critical');
    expect(canonical.status, '规范码 Critical 筛选应被接受').toBe(200);
    expect(lowercase.status, '小写 critical 应归一为 Critical 后接受').toBe(200);
    const keys = (part: { data?: ShortageListData }) =>
      (part.data as ShortageListData).items.map(row => `${row.material_id}:${row.level}`).join('|');
    expect(keys(lowercase), '两种大小写写法返回不同结果集，筛选出现了两套口径').toBe(
      keys(canonical)
    );
    for (const row of (lowercase.data as ShortageListData).items) {
      expect(row.level, `归一后仍返回了非规范码的级别：${row.level}`).toBe('Critical');
    }
  });

  test('补货建议端点返回可渲染结构（页面已接线，不再是从零调用）', async ({ request }) => {
    const res = await request.get(`${API_PREFIX}/material-shortage/replenishment`);
    expect(res.status()).toBe(200);
    const body = (await res.json()) as {
      data?: { suggestions: ReplenishmentSuggestion[]; total: number };
    };
    const payload = body.data;
    expect(payload, '补货建议响应缺少 data 对象').toBeTruthy();
    const suggestions = (payload as { suggestions: ReplenishmentSuggestion[] }).suggestions;
    expect(Array.isArray(suggestions), 'suggestions 必须是数组').toBe(true);
    expect((payload as { total: number }).total).toBe(suggestions.length);
    for (const row of suggestions) {
      expect(REPLENISHMENT_PRIORITIES, `出现取值域外的优先级：${row.priority}`).toContain(
        row.priority
      );
      // 建议量 = 缺口量 × 1.2，二者必须同为正数
      expect(Number(row.shortage_quantity)).toBeGreaterThan(0);
      expect(Number(row.suggested_quantity)).toBeGreaterThan(0);
    }
  });

  test('页面筛选下拉只列出后端真实取值', async ({ page }) => {
    await gotoWithRetry(page, '/material-shortage');
    const selects = page.locator('.filter-bar .el-select');
    await expect(selects.first()).toBeVisible({ timeout: 30_000 });
    // 收起时必须等浮层真的消失：Element Plus 的下拉面板是 teleport 到 body 的浮层，
    // 上一轮 run 里 Escape 后面板未收起就点第二个 select，点击落在浮层上，
    // 读到的仍是级别面板的 4 项，被误判成"状态下拉列错了取值"。
    const collapsed = page.locator('.el-select-dropdown:visible');

    await selects.first().click();
    const levelDropdown = collapsed;
    await expect(levelDropdown).toBeVisible({ timeout: 30_000 });
    const levelTexts = await levelDropdown
      .locator('.el-select-dropdown__item:visible')
      .allTextContents();
    expect(levelTexts.map(text => text.trim())).toEqual(['紧急', '严重', '一般', '正常']);
    await page.keyboard.press('Escape');
    await expect(collapsed).toHaveCount(0);

    await selects.nth(1).click();
    // 先确认打开的是状态面板（5 项），再逐项比对文案，避免读到未收起的上一个面板
    await expect(collapsed.locator('.el-select-dropdown__item:visible')).toHaveCount(5);
    const statusTexts = await collapsed
      .locator('.el-select-dropdown__item:visible')
      .allTextContents();
    expect(statusTexts.map(text => text.trim())).toEqual([
      '已识别',
      '已发起采购申请',
      '已转采购订单',
      '采购已入库',
      '已解除',
    ]);
    await page.keyboard.press('Escape');
    await expect(collapsed).toHaveCount(0);
  });
});
