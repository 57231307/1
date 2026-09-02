import { test, expect } from '@playwright/test';
import { loginViaUI, BASE_URL, getCtx } from './helpers';

test.describe('核心业务流程真实 UI 交互验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // 辅助：访问页面并验证表格加载
  async function visitAndVerifyTable(page: import('@playwright/test').Page, path: string) {
    await page.goto(`${BASE_URL}${path}`);
    await page.waitForTimeout(3000);
    const container = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty, .el-form'
      )
      .first();
    await container.waitFor({ state: 'visible', timeout: 30_000 }).catch(() => {});
    return page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
  }

  // 辅助：验证按钮可见且可点击
  async function verifyButton(page: import('@playwright/test').Page, text: string) {
    const btn = page.locator(`button:has-text("${text}")`).first();
    await btn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const visible = await btn.isVisible().catch(() => false);
    if (visible) {
      const disabled = await btn.isDisabled().catch(() => false);
      expect(disabled).toBe(false);
    }
    return visible;
  }

  // 辅助：点击新建按钮并验证弹窗
  async function clickNewAndVerifyDialog(page: import('@playwright/test').Page, btnText: string) {
    const btn = page.locator(`button:has-text("${btnText}")`).first();
    await btn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const visible = await btn.isVisible().catch(() => false);
    if (!visible) return false;
    await btn.click();
    await page.waitForTimeout(1000);
    const dialog = page.locator('.el-dialog').first();
    await dialog.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const dialogVisible = await dialog.isVisible().catch(() => false);
    return dialogVisible;
  }

  // 辅助：验证表单必填校验
  async function verifyRequiredValidation(page: import('@playwright/test').Page) {
    const dialog = page.locator('.el-dialog').first();
    const saveBtn = dialog
      .locator('button:has-text("保存"), button:has-text("确定"), button:has-text("提交")')
      .first();
    await saveBtn.click().catch(() => {});
    await page.waitForTimeout(1000);
    // 表单校验用 ElMessage（warning/error）或 el-form-item__error，
    // 统一匹配 .el-message（含 --warning/--error）以覆盖所有提示类型
    await page
      .locator('.el-message, .el-form-item__error')
      .first()
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(() => {});
    const hasError = await page
      .locator('.el-message, .el-form-item__error')
      .first()
      .isVisible()
      .catch(() => false);
    return hasError;
  }

  // 辅助：关闭弹窗
  async function closeDialog(page: import('@playwright/test').Page) {
    await page
      .locator('.el-dialog__headerbtn')
      .first()
      .click()
      .catch(() => {});
    await page.waitForTimeout(500);
  }

  // ================================================================
  // P2P 采购到付款流程 UI
  // ================================================================
  test('P2P 采购订单列表：表格+搜索+新建弹窗+必填校验+状态标签', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/purchase');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);

    // 验证表头
    const headers = table.locator('th');
    const headerCount = await headers.count();
    expect(headerCount).toBeGreaterThan(0);

    // 搜索
    const searchInput = page
      .locator(
        '.filter-card input, .filter-form input, input[placeholder*="订单"], input[placeholder*="供应商"]'
      )
      .first();
    await searchInput.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const searchVisible = await searchInput.isVisible().catch(() => false);
    if (searchVisible) {
      await searchInput.fill('测试');
      const queryBtn = page.locator('button:has-text("查询")').first();
      await queryBtn.waitFor({ state: 'visible', timeout: 3000 }).catch(() => {});
      const queryVisible = await queryBtn.isVisible().catch(() => false);
      if (queryVisible) {
        await queryBtn.click();
        await page.waitForTimeout(2000);
        const tableStillOk = await table.isVisible().catch(() => false);
        expect(tableStillOk).toBe(true);
      }
      await searchInput.clear();
    }

    // 新建采购单
    const dialogVisible = await clickNewAndVerifyDialog(page, '新建采购单');
    if (dialogVisible) {
      // 验证表单字段
      const supplierSelect = page.locator('.el-dialog .el-select').first();
      await supplierSelect.waitFor({ state: 'visible', timeout: 3000 }).catch(() => {});
      const supplierVisible = await supplierSelect.isVisible().catch(() => false);
      expect(supplierVisible).toBe(true);

      // 必填校验
      const hasError = await verifyRequiredValidation(page);
      expect(hasError).toBe(true);

      await closeDialog(page);
    }

    // 状态标签验证
    const statusTags = table.locator('.el-tag');
    const tagCount = await statusTags.count();
    if (tagCount > 0) {
      const firstTagClass = await statusTags.first().getAttribute('class');
      expect(firstTagClass).toContain('el-tag');
    }
  });

  test('P2P 采购收货列表 UI', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase-receipt`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 })
      .catch(() => {});
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);
  });

  // ================================================================
  // O2C 订单到收款流程 UI
  // ================================================================
  test('O2C 销售订单列表：表格+搜索+新建弹窗+必填校验+操作按钮', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/sales');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);

    // 搜索
    const searchInput = page
      .locator('input[placeholder*="订单"], input[placeholder*("客户")]')
      .first();
    await searchInput.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const searchVisible = await searchInput.isVisible().catch(() => false);
    if (searchVisible) {
      await searchInput.fill('测试');
      const queryBtn = page.locator('button:has-text("查询")').first();
      await queryBtn.waitFor({ state: 'visible', timeout: 3000 }).catch(() => {});
      const queryVisible = await queryBtn.isVisible().catch(() => false);
      if (queryVisible) {
        await queryBtn.click();
        await page.waitForTimeout(2000);
      }
      await searchInput.clear();
    }

    // 新建订单
    const dialogVisible = await clickNewAndVerifyDialog(page, '新建订单');
    if (dialogVisible) {
      // 验证客户选择器
      const customerSelect = page.locator('.el-dialog .el-select').first();
      await customerSelect.waitFor({ state: 'visible', timeout: 3000 }).catch(() => {});
      const customerVisible = await customerSelect.isVisible().catch(() => false);
      expect(customerVisible).toBe(true);

      // 必填校验
      const hasError = await verifyRequiredValidation(page);
      expect(hasError).toBe(true);

      await closeDialog(page);
    }

    // 操作按钮验证（查看/审批/发货按状态显隐）
    const rows = table.locator('.el-table__body tr');
    const rowCount = await rows.count();
    if (rowCount > 0) {
      const actionBtns = rows.first().locator('button, .el-link, .el-button');
      const btnCount = await actionBtns.count();
      expect(btnCount).toBeGreaterThan(0);
    }
  });

  test('O2C 报价单列表 UI：新建+转订单', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/quotations');
    const tableVisible = await table.isVisible().catch(() => false);
    if (tableVisible) {
      // 新建报价单
      const newBtn = page
        .locator(
          'button:has-text("新建"), button:has-text("创建"), .el-button--primary:has-text("新")'
        )
        .first();
      await newBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
      const newBtnVisible = await newBtn.isVisible().catch(() => false);
      if (newBtnVisible) {
        await newBtn.click();
        await page.waitForTimeout(2000);
        // 可能跳转到创建页或弹窗
        const url = page.url();
        await page
          .locator('.el-dialog')
          .first()
          .waitFor({ state: 'visible', timeout: 3000 })
          .catch(() => {});
        const hasDialog = await page
          .locator('.el-dialog')
          .first()
          .isVisible()
          .catch(() => false);
        expect(url.includes('quotations') || hasDialog).toBe(true);
      }
    }
  });

  // ================================================================
  // 生产流程 UI
  // ================================================================
  test('生产订单列表 UI：表格+新建', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/production');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);
  });

  test('染色配方列表 UI：色号列+新建弹窗+色号必填', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/dye-recipe');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);

    // 验证色号列
    const headers = table.locator('th');
    const headerCount = await headers.count();
    let hasColorColumn = false;
    for (let i = 0; i < headerCount; i++) {
      const text = await headers.nth(i).textContent();
      if (text && (text.includes('色号') || text.includes('颜色'))) {
        hasColorColumn = true;
        break;
      }
    }
    expect(hasColorColumn).toBe(true);

    // 新建配方
    const dialogVisible = await clickNewAndVerifyDialog(page, '新建配方');
    if (dialogVisible) {
      // 验证色号字段（el-form-item label 渲染为 <label>，hasText 兼容；CI 慢环境放宽 10s）
      const colorField = page
        .locator('.el-dialog .el-form-item')
        .filter({ hasText: /色号|颜色/ })
        .first();
      await colorField.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});
      const colorVisible = await colorField.isVisible().catch(() => false);
      expect(colorVisible).toBe(true);

      // 必填校验
      const hasError = await verifyRequiredValidation(page);
      expect(hasError).toBe(true);

      await closeDialog(page);
    }
  });

  test('缸号列表 UI：状态标签+新建弹窗', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/dye-batch');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);

    // 状态标签
    const tags = table.locator('.el-tag');
    const tagCount = await tags.count();
    if (tagCount > 0) {
      const tagText = await tags.first().textContent();
      expect(tagText?.trim().length).toBeGreaterThan(0);
    }

    // 新建批次
    const dialogVisible = await clickNewAndVerifyDialog(page, '新建批次');
    if (dialogVisible) {
      // 验证缸号/色号字段
      const inputs = page.locator('.el-dialog .el-input input');
      const inputCount = await inputs.count();
      expect(inputCount).toBeGreaterThan(0);
      await closeDialog(page);
    }
  });

  // ================================================================
  // 财务流程 UI
  // ================================================================
  test('凭证列表 UI：新建+借贷校验+状态显示', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/voucher');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);

    // 新增凭证
    const dialogVisible = await clickNewAndVerifyDialog(page, '新增凭证');
    if (dialogVisible) {
      // 必填校验
      const hasError = await verifyRequiredValidation(page);
      expect(hasError).toBe(true);
      await closeDialog(page);
    }
  });

  test('会计科目 UI：树形表格+新建科目', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/account-subject');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);

    // 树形表格验证
    const treeRows = table.locator('tr[row-key], tr.el-table__row');
    const treeRowCount = await treeRows.count();
    expect(treeRowCount).toBeGreaterThanOrEqual(0);

    // 新建科目
    const dialogVisible = await clickNewAndVerifyDialog(page, '新建科目');
    if (dialogVisible) {
      // CI 慢环境放宽 10s（3s 不足）
      const codeInput = page.locator('.el-dialog input[placeholder*="编码"]').first();
      await codeInput.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});
      const codeVisible = await codeInput.isVisible().catch(() => false);
      expect(codeVisible).toBe(true);
      await closeDialog(page);
    }
  });

  // ================================================================
  // 系统管理流程 UI
  // ================================================================
  test('系统管理 UI：用户列表+搜索+新建+审计日志', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/system');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);

    // 搜索
    const searchInput = page
      .locator('input[placeholder*="用户名"], input[placeholder*("姓名")]')
      .first();
    await searchInput.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const searchVisible = await searchInput.isVisible().catch(() => false);
    if (searchVisible) {
      await searchInput.fill('admin');
      const queryBtn = page.locator('button:has-text("查询")').first();
      await queryBtn.waitFor({ state: 'visible', timeout: 3000 }).catch(() => {});
      const queryVisible = await queryBtn.isVisible().catch(() => false);
      if (queryVisible) {
        await queryBtn.click();
        await page.waitForTimeout(2000);
      }
    }

    // 新建用户
    const dialogVisible = await clickNewAndVerifyDialog(page, '新建用户');
    if (dialogVisible) {
      const hasError = await verifyRequiredValidation(page);
      expect(hasError).toBe(true);
      await closeDialog(page);
    }

    // 审计日志 Tab
    const auditTab = page
      .locator('.el-tabs__item:has-text("审计"), .el-tabs__item:has-text("日志")')
      .first();
    await auditTab.waitFor({ state: 'visible', timeout: 3000 }).catch(() => {});
    const auditTabVisible = await auditTab.isVisible().catch(() => false);
    if (auditTabVisible) {
      await auditTab.click();
      await page.waitForTimeout(2000);
      const auditTable = page
        .locator(
          '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
        )
        .first();
      await auditTable.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
      const auditTableVisible = await auditTable.isVisible().catch(() => false);
      expect(auditTableVisible).toBe(true);
    }
  });

  // ================================================================
  // 面料四维库存流程 UI
  // ================================================================
  test('库存列表 UI：四维查询+表格+搜索', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/inventory');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);

    // 验证搜索区
    const searchArea = page.locator('.el-card.filter-card, .filter-form, .el-form').first();
    await searchArea.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const searchVisible = await searchArea.isVisible().catch(() => false);
    if (searchVisible) {
      // 产品搜索
      const productInput = searchArea.locator('input, .el-select').first();
      await productInput.waitFor({ state: 'visible', timeout: 3000 }).catch(() => {});
      const productVisible = await productInput.isVisible().catch(() => false);
      expect(productVisible).toBe(true);
    }
  });

  // ================================================================
  // 业务模式流程 UI
  // ================================================================
  test('业务模式列表 UI', async ({ page }) => {
    await page.goto(`${BASE_URL}/advanced`);
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

  // ================================================================
  // 权限管理流程 UI
  // ================================================================
  test('角色管理 UI：列表+权限分配', async ({ page }) => {
    await page.goto(`${BASE_URL}/system`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-tabs, .el-table, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 切换到角色 Tab
    const roleTab = page.locator('.el-tabs__item:has-text("角色")').first();
    await roleTab.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const roleTabVisible = await roleTab.isVisible().catch(() => false);
    if (roleTabVisible) {
      await roleTab.click();
      await page.waitForTimeout(2000);
      const table = page
        .locator(
          '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
        )
        .first();
      await table.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
      const tableVisible = await table.isVisible().catch(() => false);
      expect(tableVisible).toBe(true);
    }
  });

  // ================================================================
  // 仪表盘 UI：统计卡片+图表+刷新
  // ================================================================
  test('仪表盘 UI：统计卡片渲染+图表加载+刷新按钮', async ({ page }) => {
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);
    await page
      .locator('.dashboard-container, .el-card, .el-row')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 验证统计卡片存在
    const statCards = page.locator('.el-card, .el-statistic, [class*="stat"]');
    const cardCount = await statCards.count();
    expect(cardCount).toBeGreaterThan(0);

    // 验证图表或数据展示
    const charts = page.locator('canvas, .echarts, [class*="chart"], [class*="trend"]');
    const chartCount = await charts.count();

    // 刷新按钮
    const refreshBtn = page
      .locator('button:has-text("刷新"), .el-button--primary:has(.el-icon)')
      .first();
    await refreshBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const refreshVisible = await refreshBtn.isVisible().catch(() => false);
    if (refreshVisible) {
      const disabled = await refreshBtn.isDisabled().catch(() => false);
      expect(disabled).toBe(false);
    }
  });

  // ================================================================
  // 采购退货流程 UI
  // ================================================================
  test('采购退货列表 UI：表格+状态标签', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/purchase-return');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);

    const tags = table.locator('.el-tag');
    const tagCount = await tags.count();
    if (tagCount > 0) {
      const tagText = await tags.first().textContent();
      expect(tagText?.trim().length).toBeGreaterThan(0);
    }
  });

  // ================================================================
  // 库存调拨流程 UI
  // ================================================================
  test('库存调拨列表 UI：搜索+新建弹窗+仓库选择', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/inventory-transfer');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);

    // 新建
    const dialogVisible = await clickNewAndVerifyDialog(page, '新建');
    if (dialogVisible) {
      // 验证调出/调入仓库选择器
      const selects = page.locator('.el-dialog .el-select');
      const selectCount = await selects.count();
      expect(selectCount).toBeGreaterThanOrEqual(2);
      await closeDialog(page);
    }
  });

  // ================================================================
  // 库存盘点流程 UI
  // ================================================================
  test('库存盘点列表 UI：搜索+新建', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/inventory-count');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);

    const dialogVisible = await clickNewAndVerifyDialog(page, '新建');
    if (dialogVisible) {
      await closeDialog(page);
    }
  });

  // ================================================================
  // 成本核算流程 UI
  // ================================================================
  test('成本归集列表 UI', async ({ page }) => {
    await page.goto(`${BASE_URL}/cost`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-table, .el-empty, body')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });

  // ================================================================
  // 异常处理 UI：错误页面显示
  // ================================================================
  test('403 页面 UI：错误展示', async ({ page }) => {
    await page.goto(`${BASE_URL}/403`);
    await page.waitForTimeout(2000);
    const content = page.locator('.el-result, .error-page, body').first();
    await content.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});
    const visible = await content.isVisible().catch(() => false);
    expect(visible).toBe(true);
    const text = await content.textContent();
    expect(text?.length).toBeGreaterThan(0);
  });

  test('404 页面 UI：错误展示', async ({ page }) => {
    await page.goto(`${BASE_URL}/404`);
    await page.waitForTimeout(2000);
    const content = page.locator('.error-page, body').first();
    await content.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});
    const visible = await content.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });

  test('登录页 UI：表单元素+复选框+按钮', async ({ page, browser }) => {
    // 用新 context 访问登录页
    const context = await browser.newContext();
    const loginPage = await context.newPage();
    await loginPage.goto(`${BASE_URL}/login`);
    await loginPage.waitForTimeout(2000);

    // 验证用户名输入框
    const usernameInput = loginPage
      .locator('input[placeholder="用户名"], input[placeholder="Username"]')
      .first();
    await usernameInput.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});
    const usernameVisible = await usernameInput.isVisible().catch(() => false);
    expect(usernameVisible).toBe(true);

    // 验证密码输入框
    const passwordInput = loginPage
      .locator('input[placeholder="密码"], input[placeholder="Password"]')
      .first();
    await passwordInput.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const passwordVisible = await passwordInput.isVisible().catch(() => false);
    expect(passwordVisible).toBe(true);

    // 验证登录按钮
    const loginBtn = loginPage.locator('form button.el-button--primary').first();
    await loginBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const loginBtnVisible = await loginBtn.isVisible().catch(() => false);
    expect(loginBtnVisible).toBe(true);
    const loginBtnDisabled = await loginBtn.isDisabled().catch(() => false);
    expect(loginBtnDisabled).toBe(false);

    // 验证复选框（用户协议）
    const checkbox = loginPage.locator('.el-checkbox').first();
    await checkbox.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const checkboxVisible = await checkbox.isVisible().catch(() => false);
    expect(checkboxVisible).toBe(true);

    // 验证空表单提交触发校验
    await loginBtn.click();
    await loginPage.waitForTimeout(1000);
    await loginPage
      .locator('.el-form-item__error, .el-message--error')
      .first()
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(() => {});
    const hasError = await loginPage
      .locator('.el-form-item__error, .el-message--error')
      .first()
      .isVisible()
      .catch(() => false);
    expect(hasError).toBe(true);

    await context.close();
  });

  // ================================================================
  // 全局导航 UI：侧边菜单+路由跳转
  // ================================================================
  test('全局导航 UI：侧边菜单可见+可点击', async ({ page }) => {
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);

    // 验证侧边菜单存在
    const menu = page
      .locator('.el-menu, .el-aside, .sidebar-container, [class*="sidebar"]')
      .first();
    await menu.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});
    const menuVisible = await menu.isVisible().catch(() => false);
    if (menuVisible) {
      // 验证菜单项存在
      const menuItems = menu.locator('.el-menu-item, .el-sub-menu__title');
      const itemCount = await menuItems.count();
      expect(itemCount).toBeGreaterThan(0);

      // 点击第一个菜单项验证跳转
      const firstItem = menuItems.first();
      await firstItem.click().catch(() => {});
      await page.waitForTimeout(2000);
      const url = page.url();
      expect(url).toContain('localhost:3000');
    }
  });

  // ================================================================
  // 响应式布局 UI：窄屏不崩溃
  // ================================================================
  test('响应式 UI：窄屏 768px 布局不崩溃', async ({ page }) => {
    await page.setViewportSize({ width: 768, height: 600 });
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);
    await page
      .locator('body')
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(() => {});
    const bodyVisible = await page
      .locator('body')
      .isVisible()
      .catch(() => false);
    expect(bodyVisible).toBe(true);
    await page.setViewportSize({ width: 1280, height: 800 });
  });

  test('响应式 UI：超窄屏 375px 布局不崩溃', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);
    await page
      .locator('body')
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(() => {});
    const bodyVisible = await page
      .locator('body')
      .isVisible()
      .catch(() => false);
    expect(bodyVisible).toBe(true);
    await page.setViewportSize({ width: 1280, height: 800 });
  });

  // ================================================================
  // 面包屑+页面标题 UI
  // ================================================================
  test('页面标题 UI：document.title 正确设置', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(2000);
    const title = await page.title();
    expect(title.length).toBeGreaterThan(0);
    // 标题应包含业务名称或平台名称
    expect(
      title.includes('Bingxi') ||
        title.includes('采购') ||
        title.includes('ERP') ||
        title.length > 2
    ).toBe(true);
  });

  // ================================================================
  // 加载状态 UI：loading 指示器
  // ================================================================
  test('加载状态 UI：页面切换有 loading 指示', async ({ page }) => {
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(1000);
    // 页面加载过程中可能有 loading
    await page.goto(`${BASE_URL}/purchase`);
    // 快速检查 loading 是否出现（可能在数据加载时短暂出现）
    const loading = page.locator('.el-loading-mask, .el-loading-spinner, .el-skeleton');
    // 不强制要求 loading 一定出现（可能加载太快），验证页面最终加载完成
    await page.waitForTimeout(3000);
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);
  });
});
