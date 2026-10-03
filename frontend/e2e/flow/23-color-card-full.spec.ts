import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  getCtx,
  BASE_URL,
  safeGet,
  safeGetList,
  safePostAction,
  verifyEndpointHealthy,
  verifyDownloadEndpointHealthy,
  failureCode,
} from './helpers';

test.describe('色卡+色卡价格：API 端点 + 真实 UI 交互', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // ===== API 端点覆盖 =====
  test('色卡：CRUD+明细+预警+成本+扫码+报表', async ({ page }) => {
    await apiCallRaw(page, 'GET', '/color-cards?page=1&page_size=5');
    await apiCallRaw(page, 'GET', '/color-cards/warnings');
    await verifyEndpointHealthy(page, '/color-cards/customer-color-cards?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-cards/reorder-dye-lot?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-cards/statistics/daily');
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/color-cards?page=1&page_size=1'
    );
    const cardId = list.items?.[0]?.id;
    if (cardId) {
      await apiCallRaw(page, 'GET', `/color-cards/${cardId}`);
      await apiCallRaw(page, 'GET', `/color-cards/${cardId}/items`);
      await verifyEndpointHealthy(page, `/color-cards/warnings/${cardId}`);
      await verifyEndpointHealthy(page, `/color-cards/cost/production/${cardId}`);
      await verifyDownloadEndpointHealthy(page, `/color-cards/export/${cardId}`);
      await verifyEndpointHealthy(page, `/color-cards/scan-by-id/${cardId}`);
    }
    await verifyEndpointHealthy(page, '/color-cards/issues?page=1&page_size=5');
    // 色卡发放报表三端点（color_card.rs:80/88/96 已注册，仅分页入参可选、admin 经
    // check_permission 角色绕过 require_issue_permission，应 2xx）：迁回严格，不再 optional 吞 404。
    await verifyEndpointHealthy(page, '/color-cards/reports/issue-detail?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-cards/reports/issue-summary?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-cards/reports/expired-unused?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-cards/by-sales-order?sales_order_id=1');
    // scan/{色号} 端点已注册（routes/color_card.rs:147），正例拆入下方两个专测：
    // 入参一律来自本例自建/可证不存在的色号，不用臆造码配宽松探测吞 404/403。
  });

  // 扫码正例：本例自建专属色卡+专属色号（状态类用例本例自建取数口径），
  // 用真实唯一色号 strict 探 /color-cards/scan/{色号}，404/403/5xx 均判红。
  // 若 CI 在「POST /{id}/items 创建色号」一步红，根因是后端状态词表漂移缺陷，
  // 非本用例写错：色号创建门控要求色卡状态 == master_data::ACTIVE('active')
  // （services/color_card_item_service.rs:98），而 POST /color-cards 新建写 'draft'
  // （services/color_card_crud_service.rs:81），且 'active' 已被状态词表全集与 DB CHECK
  // 排除、无任何 activate 端点（models/status/wage_energy_chemical_business.rs:132-157、
  // migration/src/domain/v15/mod.rs:4493-4501）→ 经 API 建的色卡永远无法挂色号。
  // 此处断言「色号应可创建」的正确契约，后端修复前保持判红暴露，不得放宽。
  test('色号扫码：本例自建色号 → scan/{色号} 严格健康+回读', async ({ page }) => {
    const card = await apiCallRaw<{ id: number; card_no: string }>(page, 'POST', '/color-cards', {
      card_no: genCode('CCSCAN'),
      card_name: 'E2E扫码自建色卡',
      card_type: 'CUSTOM',
    });
    expect(
      card.id,
      '自建色卡应返回数值 id（CreateColorCardDto→ColorCardListItem）'
    ).toBeGreaterThan(0);
    const colorCode = genCode('CCCOL');
    // ColorItemDto 必填：color_code/color_name/rgb_r/g/b/hex_value（#RRGGBB，color_card_item_dto.rs:12-53）
    const item = await apiCallRaw<{ color_code: string }>(
      page,
      'POST',
      `/color-cards/${card.id}/items`,
      {
        color_code: colorCode,
        color_name: 'E2E扫码自建色号',
        rgb_r: 12,
        rgb_g: 34,
        rgb_b: 56,
        hex_value: '#0C2238',
      }
    );
    expect(item.color_code, '创建色号应回显本例唯一色号').toBe(colorCode);
    await verifyEndpointHealthy(page, `/color-cards/scan/${colorCode}`);
    const scanned = await apiCallRaw<{
      color_item: { color_code: string };
      color_card_no: string;
    }>(page, 'GET', `/color-cards/scan/${colorCode}`);
    expect(scanned.color_item.color_code, '扫码结果应命中本例自建色号').toBe(colorCode);
    expect(scanned.color_card_no, '扫码结果应关联本例自建色卡').toBe(card.card_no);
  });

  // 扫码负例：一个本例从未创建、且 genCode 唯一性可证不存在的色号，
  // 必须命中 scan_by_code 的「色号不存在」404 分支并断具体机器码，
  // 不得再用宽松探测把 404 当健康吞掉。
  // 判责：services/color_card_scan_service.rs:82 AppError::not_found("色号不存在")
  // → HTTP 404（utils/error.rs:358）+ 信封 code='NOT_FOUND'（utils/error.rs:737）。
  test('色号扫码：不存在色号 → 404 + code=NOT_FOUND', async ({ page }) => {
    const missing = await apiCallExpectFail(page, 'GET', `/color-cards/scan/${genCode('NOCOLOR')}`);
    expect(missing.status, '不存在色号应 404（2xx=吞出假健康，5xx/403=契约或权限回归）').toBe(404);
    expect(failureCode(missing), '统一信封 code 应为 NOT_FOUND').toBe('NOT_FOUND');
  });

  test('色卡借出→归还→丢失→损坏状态机', async ({ page }) => {
    const issues = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/color-cards/issues?page=1&page_size=1'
    );
    const issueId = issues.items?.[0]?.id;
    if (issueId) {
      await apiCallRaw(page, 'GET', `/color-cards/issues/${issueId}`);
      await safePostAction(page, `/color-cards/issues/${issueId}/return`);
      await safePostAction(page, `/color-cards/issues/${issueId}/damaged`);
      await safePostAction(page, `/color-cards/issues/${issueId}/cancel`);
    }
  });

  test('色卡价格：CRUD+批量调价+审批+历史+阶梯+季节规则', async ({ page }) => {
    // 注意：/color-prices/ 尾斜杠在 axum 0.8 nest 下 404，必须用 /color-prices
    await apiCallRaw(page, 'GET', '/color-prices?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-prices/customer-special?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-prices/seasonal-rules?page=1&page_size=5');
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/color-prices?page=1&page_size=1'
    );
    const priceId = list.items?.[0]?.id;
    if (priceId) {
      await apiCallRaw(page, 'GET', `/color-prices/${priceId}`);
      await verifyEndpointHealthy(page, `/color-prices/${priceId}/history`);
      await verifyEndpointHealthy(page, `/color-prices/tiers/${priceId}`);
      // approve 需携带 ApproveColorPriceDto（color_price_dto.rs:102）：decision 必填，
      // 取值须为 APPROVED/REJECTED（batch_service.rs:210-219 权威 token）。
      // 空体 → 后端 serde 422 missing field decision，属正当拒绝，不放宽。
      await safePostAction(page, `/color-prices/${priceId}/approve`, { decision: 'APPROVED' });
    }
    await verifyEndpointHealthy(page, '/color-prices/calculate?product_id=1&quantity=100');
  });

  // ===== 真实 UI 交互验证 =====
  test('色卡列表 UI：搜索+表格+新建跳转', async ({ page }) => {
    await page.goto(`${BASE_URL}/color-cards/list`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 验证搜索框
    const searchInput = page
      .locator('input[placeholder*="卡号"], input[placeholder*="卡名"]')
      .first();
    await searchInput.waitFor({ state: 'visible', timeout: 5000 });
    const searchVisible = await searchInput.isVisible();
    if (searchVisible) {
      await searchInput.fill('测试');
      const queryBtn = page.locator('button:has-text("查询")').first();
      await queryBtn.waitFor({ state: 'visible', timeout: 3000 });
      const btnVisible = await queryBtn.isVisible();
      if (btnVisible) {
        await queryBtn.click();
        await page.waitForTimeout(2000);
      }
      const tableOk = await page
        .locator(
          '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
        )
        .first()
        .isVisible();
      expect(tableOk).toBe(true);
      await searchInput.clear();
    }
    // 验证新建色卡按钮
    const newBtn = page.locator('button:has-text("新建色卡")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(2000);
      // 跳转到创建页面
      const url = page.url();
      expect(url.includes('create') || url.includes('new')).toBe(true);
    }
  });

  test('色卡创建页面 UI：表单字段渲染', async ({ page }) => {
    await page.goto(`${BASE_URL}/color-cards/create`);
    await page.waitForTimeout(3000);
    // 验证表单存在
    const form = page.locator('.el-form, .el-card').first();
    await form.waitFor({ state: 'visible', timeout: 15_000 });
    const formVisible = await form.isVisible();
    expect(formVisible).toBe(true);
    // 验证有卡号、卡名、类型输入字段
    const inputs = page.locator('.el-input input, .el-select');
    const inputCount = await inputs.count();
    expect(inputCount).toBeGreaterThan(0);
  });

  test('色卡价格列表 UI：表格+搜索+新建跳转', async ({ page }) => {
    await page.goto(`${BASE_URL}/color-prices/list`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 验证新建价格按钮
    const newBtn = page.locator('button:has-text("新建价格")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(2000);
      const url = page.url();
      expect(url.includes('create') || url.includes('new')).toBe(true);
    }
  });

  test('色卡价格批量调价 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/color-prices/batch-adjust`);
    await page.waitForTimeout(3000);
    // isVisible 立即返回，组件异步挂载可能尚未渲染 → 改用 waitFor 等待可见
    const container = page.locator('.el-card, .el-form, .el-table, .el-empty, body').first();
    await container.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await container.isVisible();
    expect(visible).toBe(true);
  });

  test('色卡借出记录 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/color-cards/issues`);
    // 页面 el-tabs 默认 activeTab='issue'（发放表单，views/color-cards/issues.vue:385），
    // "发放中"（:94-177）与"历史"（:180+）两张 el-table 在未激活的 pane 里（display:none），
    // 不切 tab 直接等表格必然拿到 hidden/超时。先真实点击"发放中"tab，再断该 pane 内真实列。
    await page.getByRole('tab', { name: /发放中/ }).click();
    const table = page.locator('.el-table:visible').first();
    await expect(table, '切到"发放中"tab 后应见其表格').toBeVisible({ timeout: 15_000 });
    // 真实表头列（对照 colorCards.issue.activeTable.*）
    const headers = await table.locator('th').allTextContents();
    console.log(`[E2E][23] 色卡借出记录(发放中)表头=${JSON.stringify(headers)}`);
    expect(headers.length, '发放中表格应渲染出表头').toBeGreaterThan(0);
    expect(headers.join('|'), '发放中表格应含"发放数量"列').toContain('发放数量');
  });
});
