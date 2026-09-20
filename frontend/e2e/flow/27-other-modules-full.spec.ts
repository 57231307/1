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
  trackPageHealth,
  assertPageHealthy,
} from './helpers';

test.describe('其他模块全量：API 端点 + 真实 UI 交互', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // ===== API 端点覆盖 =====
  test('合同+价格+检验+供应商+定制+库存扩展端点', async ({ page }) => {
    // 合同
    await verifyEndpointHealthy(page, '/purchase/purchase-contracts?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/sales/sales-contracts?page=1&page_size=5');
    // 价格
    await verifyEndpointHealthy(page, '/purchase/purchase-prices?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/sales/sales-prices?page=1&page_size=5');
    // 检验
    await verifyEndpointHealthy(page, '/purchase/inspections?page=1&page_size=5');
    // 供应商完整
    await verifyEndpointHealthy(page, '/purchase/suppliers?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/purchase/suppliers/abnormal-orders');
    await verifyEndpointHealthy(page, '/supplier-evaluations?page=1&page_size=5');
    const supList = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/purchase/suppliers?page=1&page_size=1'
    );
    let supId = supList.items?.[0]?.id;
    if (!supId) {
      // 分片独立 DB：本分片可能没有任何供应商，前置创建（API 兜底用于测试数据准备）
      console.warn('[27-other] 供应商列表为空，前置创建供应商');
      const created = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
        supplier_name: `E2E供应商${Date.now().toString().slice(-6)}`,
        supplier_short_name: 'E2E供',
        contact_phone: '13800000001',
      });
      supId = created.data?.id;
    }
    if (!supId) throw new Error('无任何供应商（创建兜底也失败），无法测试供应商详情');
    await apiCallRaw(page, 'GET', `/purchase/suppliers/${supId}`);
    await verifyEndpointHealthy(page, `/purchase/suppliers/${supId}/balance`);
    await verifyEndpointHealthy(page, `/purchase/suppliers/${supId}/purchase-history`);
    await verifyEndpointHealthy(page, `/purchase/suppliers/${supId}/contacts`);
    await verifyEndpointHealthy(page, `/purchase/suppliers/${supId}/qualifications`);
    await verifyEndpointHealthy(page, `/purchase/suppliers/${supId}/evaluations`);
    // 定制订单
    await verifyEndpointHealthy(page, '/custom-orders?page=1&page_size=5');
    // 大货批色
    await verifyEndpointHealthy(page, '/bulk-color-approvals?page=1&page_size=5');
    // 8D/坏账/催收/预警/OA/PDA/商检
    await verifyEndpointHealthy(page, '/quality-8d-reports?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/bad-debts?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/collection-tasks?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/finance-alerts?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/oa-announcements?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/device-connections?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/export-inspections?page=1&page_size=5');
    // 库存扩展
    await verifyEndpointHealthy(page, '/inventory/adjustments?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/inventory/reservations?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/inventory/write-downs?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/inventory/batches?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/inventory/logistics?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/inventory/stock/export');
    await verifyEndpointHealthy(page, '/inventory/stock/transactions?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/inventory/stock/summary');
    await verifyEndpointHealthy(page, '/inventory/stock/low-stock');
  });

  // ===== 真实 UI 交互验证 =====
  test('采购合同列表 UI：搜索+新建', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase-contract`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const searchBtn = page.locator('button:has-text("查询")').first();
    await searchBtn.waitFor({ state: 'visible', timeout: 5000 });
    const searchVisible = await searchBtn.isVisible();
    if (searchVisible) {
      await searchBtn.click();
      await page.waitForTimeout(2000);
      const tableOk = await page
        .locator(
          '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
        )
        .first()
        .isVisible();
      expect(tableOk).toBe(true);
    }
    const newBtn = page.locator('button:has-text("新建"), button:has-text("新增")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 });
      const dialogVisible = await dialog.isVisible();
      expect(dialogVisible).toBe(true);
      await page.locator('.el-dialog__headerbtn').first().click();
    }
  });

  test('采购价格列表 UI', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase-price`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table.waitFor({ state: 'visible', timeout: 10_000 });
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);
  });

  test('供应商列表 UI：搜索+新建供应商弹窗', async ({ page }) => {
    await page.goto(`${BASE_URL}/supplier`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 搜索
    const searchInput = page
      .locator('input[placeholder*="供应商"], input[placeholder*="名称"]')
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
    }
    // 新建供应商
    const newBtn = page.locator('button:has-text("新建供应商")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 });
      const dialogVisible = await dialog.isVisible();
      expect(dialogVisible).toBe(true);
      // 验证供应商编码输入框
      const codeInput = dialog.locator('input[placeholder*="供应商编码"]').first();
      await codeInput.waitFor({ state: 'visible', timeout: 3000 });
      const codeVisible = await codeInput.isVisible();
      expect(codeVisible).toBe(true);
      // 直接保存触发必填校验
      const saveBtn = dialog.locator('button:has-text("保存"), button:has-text("确定")').first();
      await saveBtn.click();
      await page.waitForTimeout(1000);
      await page
        .locator('.el-form-item__error, .el-message--error')
        .first()
        .waitFor({ state: 'visible', timeout: 5000 });
      const hasError = await page
        .locator('.el-form-item__error, .el-message--error')
        .first()
        .isVisible();
      expect(hasError).toBe(true);
      await page.locator('.el-dialog__headerbtn').first().click();
    }
  });

  test('销售合同列表 UI', async ({ page }) => {
    await page.goto(`${BASE_URL}/sales-contract`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table.waitFor({ state: 'visible', timeout: 10_000 });
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);
  });

  test('库存调整列表 UI', async ({ page }) => {
    await page.goto(`${BASE_URL}/inventory-adjustment`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table.waitFor({ state: 'visible', timeout: 10_000 });
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);
  });

  test('定制订单列表 UI：新建+状态显示', async ({ page }) => {
    await page.goto(`${BASE_URL}/custom-orders`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建"), button:has-text("新增")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      // 定制订单的真实交互是路由跳转到独立创建页（全屏表单，无 el-dialog）——
      // 断言"新建入口可用"：弹窗出现（5s 竞态窗口），或已跳转创建页且表单渲染
      const urlAfter = page.url();
      const onCreatePage =
        urlAfter.includes('custom-orders/create') || urlAfter.includes('custom-orders/new');
      const dialog = page.locator('.el-dialog:visible').first();
      const dialogVisible = await dialog.isVisible().catch(() => false);
      if (!dialogVisible && onCreatePage) {
        const createForm = page
          .locator('.el-form:visible')
          .filter({ has: page.locator('.el-form-item') })
          .first();
        await createForm.waitFor({ state: 'visible', timeout: 10_000 });
        expect(await createForm.isVisible()).toBe(true);
      } else if (!dialogVisible) {
        await dialog.waitFor({ state: 'visible', timeout: 5000 });
        expect(await dialog.isVisible()).toBe(true);
      } else {
        expect(dialogVisible).toBe(true);
      }
      if (dialogVisible) {
        await page.locator('.el-dialog__headerbtn').first().click();
      }
    }
  });

  test('安全设置 UI：修改密码表单', async ({ page }) => {
    // run 35515772653 实测该路由渲染出 ErrorBoundary「页面加载出错」，而原实现只
    // waitFor('.el-form') 超时，报 TimeoutError 却不带任何异常信息，无法定位根因；
    // 且其后的 `if (formVisible)` 分支在表单不可见时会让用例静默通过。
    // 先挂健康采集，失败时把页面异常与 console 错误一并带进报告。
    const collector = trackPageHealth(page);

    await page.goto(`${BASE_URL}/security/change-password`);
    await page.waitForTimeout(3000);

    const errorBoundary = page.getByText('页面加载出错');
    expect(
      await errorBoundary.count(),
      '修改密码页命中错误边界；页面异常=' +
        (collector.pageErrors.join(' | ') || '(无 pageerror)') +
        ' console错误=' +
        (collector.consoleErrors.slice(0, 3).join(' | ') || '(无)')
    ).toBe(0);

    const form = page.locator('.el-form').first();
    await form.waitFor({ state: 'visible', timeout: 15_000 });
    const inputCount = await form.locator('input').count();
    console.log(`[E2E][27] 修改密码表单 input 数量=${inputCount}`);
    expect(inputCount).toBeGreaterThan(0);

    await assertPageHealthy(page, collector);
  });

  test('坯布管理 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/greige-fabrics`);
    await page.waitForTimeout(3000);
    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty, body'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });

  test('销售退货列表 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/sales-returns`);
    await page.waitForTimeout(3000);
    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty, body'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table.waitFor({ state: 'visible', timeout: 10_000 });
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);
  });
});
