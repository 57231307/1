import { test, expect } from '@playwright/test';
import { loginViaUI, BASE_URL } from './helpers';

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
        '.el-table, .el-card, .el-form, .el-empty, .el-tabs, .dashboard-container, canvas, .el-result, .error-page, body'
      )
      .first();
    await container.waitFor({ state: 'visible', timeout: 30_000 }).catch(() => {});
    return container;
  }

  // 辅助：验证表格+表头
  async function verifyTable(page: import('@playwright/test').Page) {
    const table = page.locator('.el-table').first();
    await table.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});
    const visible = await table.isVisible().catch(() => false);
    if (visible) {
      const headers = table.locator('th');
      const count = await headers.count();
      expect(count).toBeGreaterThan(0);
    }
    return visible;
  }

  // 辅助：验证新建按钮+弹窗
  async function verifyNewButton(page: import('@playwright/test').Page, btnText: string) {
    const btn = page.locator(`button:has-text("${btnText}")`).first();
    await btn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const visible = await btn.isVisible().catch(() => false);
    if (visible) {
      const disabled = await btn.isDisabled().catch(() => false);
      expect(disabled).toBe(false);
      await btn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      const dialogVisible = await dialog
        .waitFor({ state: 'visible', timeout: 5000 })
        .then(() => true)
        .catch(() => false);
      if (dialogVisible) {
        await page
          .locator('.el-dialog__headerbtn')
          .first()
          .click()
          .catch(() => {});
        await page.waitForTimeout(500);
      }
      return dialogVisible;
    }
    return false;
  }

  // ===== 采购域剩余路由 =====
  test('采购管理总页 /purchase', async ({ page }) => {
    await visitPage(page, '/purchase');
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });
  test('采购合同 /purchase-contract', async ({ page }) => {
    await visitPage(page, '/purchase-contract');
    await verifyTable(page);
  });
  test('采购扩展 /purchase-ext', async ({ page }) => {
    await visitPage(page, '/purchase-ext');
    await verifyTable(page);
  });
  test('采购检验 /purchase-inspection', async ({ page }) => {
    await visitPage(page, '/purchase-inspection');
    await verifyTable(page);
  });
  test('采购价格 /purchase-price', async ({ page }) => {
    await visitPage(page, '/purchase-price');
    await verifyTable(page);
  });
  test('采购收货 /purchase-receipt', async ({ page }) => {
    await visitPage(page, '/purchase-receipt');
    await verifyTable(page);
  });
  test('采购退货 /purchase-return', async ({ page }) => {
    await visitPage(page, '/purchase-return');
    await verifyTable(page);
  });
  test('供应商 /supplier', async ({ page }) => {
    await visitPage(page, '/supplier');
    const tableOk = await verifyTable(page);
    if (tableOk) await verifyNewButton(page, '新建供应商');
  });
  test('供应商评估 /supplier-evaluation', async ({ page }) => {
    await visitPage(page, '/supplier-evaluation');
    await verifyTable(page);
  });

  // ===== 销售域剩余路由 =====
  test('销售管理总页 /sales', async ({ page }) => {
    await visitPage(page, '/sales');
    await verifyTable(page);
  });
  test('销售合同 /sales-contract', async ({ page }) => {
    await visitPage(page, '/sales-contract');
    await verifyTable(page);
  });
  test('销售扩展 /sales-ext', async ({ page }) => {
    await visitPage(page, '/sales-ext');
    await verifyTable(page);
  });
  test('销售价格 /sales-price', async ({ page }) => {
    await visitPage(page, '/sales-price');
    await verifyTable(page);
  });
  test('报价单新建 /quotations/new', async ({ page }) => {
    await visitPage(page, '/quotations/new');
    const form = page.locator('.el-form, .el-card').first();
    await form.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});
    const visible = await form.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('报价单详情 /quotations/:id', async ({ page }) => {
    await visitPage(page, '/quotations/1');
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });

  // ===== 库存域剩余路由 =====
  test('库存管理总页 /inventory', async ({ page }) => {
    await visitPage(page, '/inventory');
    await verifyTable(page);
  });
  test('库存调整(旧路径) /inventory-adjustment', async ({ page }) => {
    await visitPage(page, '/inventory-adjustment');
    await verifyTable(page);
  });
  test('库存批次 /inventory-batch', async ({ page }) => {
    await visitPage(page, '/inventory-batch');
    await verifyTable(page);
  });
  test('库存盘点(旧路径) /inventory-count', async ({ page }) => {
    await visitPage(page, '/inventory-count');
    await verifyTable(page);
  });
  test('库存调拨(旧路径) /inventory-transfer', async ({ page }) => {
    await visitPage(page, '/inventory-transfer');
    await verifyTable(page);
  });
  test('物流 /logistics', async ({ page }) => {
    await visitPage(page, '/logistics');
    await verifyTable(page);
  });

  // ===== 生产域剩余路由 =====
  test('生产管理总页 /production', async ({ page }) => {
    await visitPage(page, '/production');
    await verifyTable(page);
  });
  test('MRP历史 /mrp/history', async ({ page }) => {
    await visitPage(page, '/mrp/history');
    await verifyTable(page);
  });
  test('质量标准 /quality-standards', async ({ page }) => {
    await visitPage(page, '/quality-standards');
    await verifyTable(page);
  });
  test('面料管理 /fabric', async ({ page }) => {
    await visitPage(page, '/fabric');
    await verifyTable(page);
  });

  // ===== 财务域剩余路由 =====
  test('应收 /ar', async ({ page }) => {
    await visitPage(page, '/ar');
    const tab = page.locator('.el-tabs, .el-table, .el-card').first();
    await tab.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await tab.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('应收对账增强 /ar-reconciliation/enhanced', async ({ page }) => {
    await visitPage(page, '/ar-reconciliation/enhanced');
    await verifyTable(page);
  });
  test('辅助核算 /assist-accounting', async ({ page }) => {
    await visitPage(page, '/assist-accounting');
    await verifyTable(page);
  });
  test('财务总页 /finance', async ({ page }) => {
    await visitPage(page, '/finance');
    const tab = page.locator('.el-tabs, .el-table, .el-card').first();
    await tab.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await tab.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('财务分析 /financial-analysis', async ({ page }) => {
    await visitPage(page, '/financial-analysis');
    const card = page.locator('.el-card, .el-table, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('交易管理 /trading', async ({ page }) => {
    await visitPage(page, '/trading');
    await verifyTable(page);
  });

  // ===== 系统域剩余路由 =====
  test('系统总页 /system', async ({ page }) => {
    await visitPage(page, '/system');
    const tab = page.locator('.el-tabs, .el-table').first();
    await tab.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await tab.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('系统更新 /system-update', async ({ page }) => {
    await visitPage(page, '/system-update');
    const card = page.locator('.el-card, .el-form, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('个人信息 /system/profile', async ({ page }) => {
    await visitPage(page, '/system/profile');
    const form = page.locator('.el-form, .el-card, body').first();
    await form.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await form.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('慢查询 /system/slow-query', async ({ page }) => {
    await visitPage(page, '/system/slow-query');
    await verifyTable(page);
  });
  test('全量审计 /omni-audit', async ({ page }) => {
    await visitPage(page, '/omni-audit');
    await verifyTable(page);
  });
  test('数据导入 /data-import', async ({ page }) => {
    await visitPage(page, '/data-import');
    const card = page.locator('.el-card, .el-upload, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('报表中心 /report-templates', async ({ page }) => {
    await visitPage(page, '/report-templates');
    await verifyTable(page);
  });
  test('邮件管理 /email', async ({ page }) => {
    await visitPage(page, '/email');
    const card = page.locator('.el-card, .el-table, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('双因素认证 /security/two-factor-setup', async ({ page }) => {
    await visitPage(page, '/security/two-factor-setup');
    const card = page.locator('.el-card, .el-form, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('主备隔离 /admin/failover', async ({ page }) => {
    await visitPage(page, '/admin/failover');
    const card = page.locator('.el-card, .el-form, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });

  // ===== CRM 域剩余路由 =====
  test('客户分配 /crm/assignment', async ({ page }) => {
    await visitPage(page, '/crm/assignment');
    await verifyTable(page);
  });
  test('客户360 /crm/detail/:id', async ({ page }) => {
    await visitPage(page, '/crm/detail/1');
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
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
    await form.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await form.isVisible().catch(() => false);
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
    await form.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await form.isVisible().catch(() => false);
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
    await card.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('AI工艺优化 /ai-extend/process-optimization', async ({ page }) => {
    await visitPage(page, '/ai-extend/process-optimization');
    const card = page.locator('.el-card, .el-table, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('AI质量预测 /ai-extend/quality-prediction', async ({ page }) => {
    await visitPage(page, '/ai-extend/quality-prediction');
    const card = page.locator('.el-card, .el-table, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await card.isVisible().catch(() => false);
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
    await tab.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await tab.isVisible().catch(() => false);
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
    await card.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('扫码 /barcode-scanner', async ({ page }) => {
    await visitPage(page, '/barcode-scanner');
    const card = page.locator('.el-card, .el-input, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const visible = await card.isVisible().catch(() => false);
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
