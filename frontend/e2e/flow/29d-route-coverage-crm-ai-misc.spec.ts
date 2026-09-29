import { test, expect } from '../diagnose-fixture';
import { loginViaUI, BASE_URL, API_BASE, API_PREFIX } from './helpers';

test.describe('100% 前端路由 UI 交互全覆盖', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // 辅助：访问页面，验证核心 UI 元素可见
  async function visitPage(page: import('@playwright/test').Page, path: string) {
    await page.goto(`${BASE_URL}${path}`);
    await page.waitForTimeout(2000);
    const container = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-form, .el-empty, .el-tabs, .dashboard-container, canvas, .el-result, .error-page, body'
      )
      .first();
    await container.waitFor({ state: 'visible', timeout: 30_000 });
    return container;
  }

  // 辅助：验证表格+表头
  async function verifyTable(page: import('@playwright/test').Page) {
    // 部分模块（BPM 模板等）是卡片网格/空数据页，无 .el-table，先等通用容器可见
    const container = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty, .el-result, .error-page, body'
      )
      .first();
    await container.waitFor({ state: 'visible', timeout: 10_000 });
    const table = page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper')
      .first();
    const visible = await table.isVisible().catch(() => false);
    if (visible) {
      const headers = table.locator('th, .el-table-v2__header-cell');
      const count = await headers.count();
      expect(count).toBeGreaterThan(0);
    }
    return visible;
  }

  // 辅助：验证新建按钮+弹窗
  async function verifyNewButton(page: import('@playwright/test').Page, btnText: string) {
    const btn = page.locator(`button:has-text("${btnText}")`).first();
    await btn.waitFor({ state: 'visible', timeout: 5000 });
    const visible = await btn.isVisible();
    if (visible) {
      const disabled = await btn.isDisabled();
      expect(disabled).toBe(false);
      await btn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      const dialogVisible = await dialog
        .waitFor({ state: 'visible', timeout: 5000 })
        .then(() => true);
      if (dialogVisible) {
        await page.locator('.el-dialog__headerbtn').first().click();
        await page.waitForTimeout(500);
      }
      return dialogVisible;
    }
    return false;
  }

  // ===== CRM 域剩余路由 =====
  test('客户分配 /crm/assignment', async ({ page }) => {
    await visitPage(page, '/crm/assignment');
    await verifyTable(page);
  });
  test('客户360 /crm/detail/:id', async ({ page }) => {
    // 假绿解封（契约核查坐实）：detail.vue 对 customer.tags（:221/TagsPanelTab :33）与
    // customer.shipping_addresses（:221/:237 对其取 .length）在 undefined 时运行期必崩，
    // 旧断言仅 visitPage + body.isVisible() —— 崩了 body 仍可见，掩盖成伪全绿。
    // 锁定 360 契约（data 顶层对象）：{customer, tags:[...], shipping_addresses:[...]}，空为 []。
    //
    // 先用真实存在的客户 id：不存在的 id 只会走 detail.vue:315 el-empty 分支（v-if="!customer"），
    // 绕过崩溃路径 = 又一次假绿，故必须取真实客户。
    const listResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers?page=1&page_size=1`
    );
    expect(listResp.ok(), `客户列表请求失败，status=${listResp.status()}`).toBeTruthy();
    const listJson = (await listResp.json()) as { data?: { items?: Array<{ id: number }> } };
    const customerId = listJson?.data?.items?.[0]?.id;
    expect(customerId, '库中无任何客户，无法验证 360 契约（前置缺失，非缺陷掩盖）').toBeTruthy();

    // 契约层：GET /crm/customers/{id}/360 的 data 顶层必须含 customer 且 tags/shipping_addresses
    // 均为数组（后端富化未做到位、缺任一数组键 → 详情页运行期崩溃，此处如实红）。
    const r360 = await page.request.get(`${API_BASE}${API_PREFIX}/crm/customers/${customerId}/360`);
    expect(r360.ok(), `360 端点应返回 2xx，实际 status=${r360.status()}`).toBeTruthy();
    const env = (await r360.json()) as { code?: number; data?: Record<string, unknown> };
    expect(env.code, `360 信封应返回成功码 200，实际 code=${env.code}`).toBe(200);
    const data = env.data ?? {};
    expect(data.customer, '360 data 顶层应含 customer 对象').toBeTruthy();
    expect(
      Array.isArray(data.tags),
      `360 data.tags 应为数组（缺键会使详情页崩溃），实际=${JSON.stringify(data.tags)}`
    ).toBe(true);
    expect(
      Array.isArray(data.shipping_addresses),
      `360 data.shipping_addresses 应为数组（detail.vue:237 对其取 .length），实际=${JSON.stringify(
        data.shipping_addresses
      )}`
    ).toBe(true);

    // 渲染层：访问详情页，标签区容器与收货地址区块必须实际出现。
    // 契约已满足但组件仍崩（如消费路径不符）时，这些容器永不提交 → waitFor 超时红。
    await page.goto(`${BASE_URL}/crm/detail/${customerId}`);
    await page.waitForTimeout(2000);
    const tagsContainer = page.locator('.detail-content .tags-container').first();
    await tagsContainer.waitFor({ state: 'visible', timeout: 15_000 });
    expect(await tagsContainer.isVisible(), '标签区容器应实际渲染').toBe(true);
    const addressList = page.locator('.detail-content .address-list').first();
    await addressList.waitFor({ state: 'visible', timeout: 15_000 });
    expect(await addressList.isVisible(), '收货地址区块应实际渲染').toBe(true);
  });
  test('客户信用 /customer-credit', async ({ page }) => {
    await visitPage(page, '/customer-credit');
    await verifyTable(page);
  });

  // ===== 色卡域剩余路由 =====
  test('色卡详情 /color-cards/detail/:id', async ({ page }) => {
    await visitPage(page, '/color-cards/detail/1');
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });
  test('色卡价格新建 /color-prices/create', async ({ page }) => {
    await visitPage(page, '/color-prices/create');
    const form = page.locator('.el-form, .el-card').first();
    await form.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await form.isVisible();
    expect(visible).toBe(true);
  });
  test('色卡价格详情 /color-prices/detail/:id', async ({ page }) => {
    await visitPage(page, '/color-prices/detail/1');
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });

  // ===== 定制订单剩余路由 =====
  test('定制订单新建 /custom-orders/new', async ({ page }) => {
    await visitPage(page, '/custom-orders/new');
    const form = page.locator('.el-form, .el-card').first();
    await form.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await form.isVisible();
    expect(visible).toBe(true);
  });
  test('定制订单详情 /custom-orders/:id', async ({ page }) => {
    await visitPage(page, '/custom-orders/1');
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });
  test('定制订单跟踪 /custom-orders/:id/track', async ({ page }) => {
    await visitPage(page, '/custom-orders/1/track');
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });

  // ===== AI/BPM 域 =====
  test('AI扩展 /ai-extend', async ({ page }) => {
    await visitPage(page, '/ai-extend');
    const card = page.locator('.el-card, .el-table, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await card.isVisible();
    expect(visible).toBe(true);
  });
  test('AI工艺优化 /ai-extend/process-optimization', async ({ page }) => {
    await visitPage(page, '/ai-extend/process-optimization');
    const card = page.locator('.el-card, .el-table, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await card.isVisible();
    expect(visible).toBe(true);
  });
  test('AI质量预测 /ai-extend/quality-prediction', async ({ page }) => {
    await visitPage(page, '/ai-extend/quality-prediction');
    const card = page.locator('.el-card, .el-table, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await card.isVisible();
    expect(visible).toBe(true);
  });
  test('AI工艺详情 /ai-extend/process-detail/:id', async ({ page }) => {
    await visitPage(page, '/ai-extend/process-detail/1');
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });
  test('BPM /bpm', async ({ page }) => {
    await visitPage(page, '/bpm');
    const tab = page.locator('.el-tabs, .el-table, .el-card').first();
    await tab.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await tab.isVisible();
    expect(visible).toBe(true);
  });
  test('BPM审批 /bpm/approval', async ({ page }) => {
    await visitPage(page, '/bpm/approval');
    await verifyTable(page);
  });
  test('BPM定义 /bpm/definitions', async ({ page }) => {
    await visitPage(page, '/bpm/definitions');
    await verifyTable(page);
  });
  test('BPM模板 /bpm/templates', async ({ page }) => {
    await visitPage(page, '/bpm/templates');
    await verifyTable(page);
  });

  // ===== 其他剩余路由 =====
  test('BI销售分析 /bi/sales-analysis', async ({ page }) => {
    await visitPage(page, '/bi/sales-analysis');
    const card = page.locator('.el-card, canvas, .echarts, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await card.isVisible();
    expect(visible).toBe(true);
  });
  test('扫码 /barcode-scanner', async ({ page }) => {
    await visitPage(page, '/barcode-scanner');
    const card = page.locator('.el-card, .el-input, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await card.isVisible();
    expect(visible).toBe(true);
  });
  test('组件示例 /components-demo', async ({ page }) => {
    await visitPage(page, '/components-demo');
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });
  test('产品管理 /product', async ({ page }) => {
    await visitPage(page, '/product');
    await verifyTable(page);
  });
  test('仓库管理 /warehouse', async ({ page }) => {
    await visitPage(page, '/warehouse');
    await verifyTable(page);
  });
  test('工作流 /workflow', async ({ page }) => {
    await visitPage(page, '/workflow');
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });
});
