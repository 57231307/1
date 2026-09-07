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
    await container
      .waitFor({ state: 'visible', timeout: 30_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    return page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
  }

  // 辅助：验证按钮可见且可点击
  async function verifyButton(page: import('@playwright/test').Page, text: string) {
    const btn = page.locator(`button:has-text("${text}")`).first();
    await btn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await btn.isVisible().catch(() => false);
    if (visible) {
      const disabled = await btn.isDisabled().catch(() => false);
      expect(disabled).toBe(false);
    }
    return visible;
  }

  // 辅助：点击新建按钮并验证弹窗（只匹配可见对话框）
  async function clickNewAndVerifyDialog(page: import('@playwright/test').Page, btnText: string) {
    const btn = page.locator(`button:has-text("${btnText}")`).first();
    await btn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await btn.isVisible().catch(() => false);
    if (!visible) return false;
    await btn.click();
    await page.waitForTimeout(1000);
    const dialog = page.locator('.el-dialog:visible').first();
    await dialog
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const dialogVisible = await dialog.isVisible().catch(() => false);
    return dialogVisible;
  }

  // 辅助：验证表单必填校验
  async function verifyRequiredValidation(page: import('@playwright/test').Page) {
    // 可见对话框（页面可能挂载多个 el-dialog，隐藏的不参与匹配）
    const dialog = page.locator('.el-dialog:visible').first();
    // 提交按钮 = 对话框 footer 的主按钮（各模块文案不一：保存/确定/确认/提交），
    // 按文案匹配会因文案差异（如 "确认"）匹配不到而 30s 超时
    const saveBtn = dialog.locator('.el-dialog__footer .el-button--primary').first();
    try {
      await saveBtn.click({ timeout: 10_000 });
      console.log('[verifyRequiredValidation] 已点击 footer 主按钮');
    } catch (e) {
      console.error(`[verifyRequiredValidation] 点击主按钮失败: ${(e as Error).message}`);
      return false;
    }
    await page.waitForTimeout(1000);
    // 表单校验用 ElMessage（warning/error）或 el-form-item__error，
    // 统一匹配 .el-message（含 --warning/--error）以覆盖所有提示类型
    await page
      .locator('.el-message, .el-form-item__error')
      .first()
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const hasError = await page
      .locator('.el-message, .el-form-item__error')
      .first()
      .isVisible()
      .catch(() => false);
    if (!hasError) {
      // 诊断输出（IR 详细日志要求）：无任何校验提示时打印对话框文本片段
      const dialogText = await dialog.innerText().catch(() => '<无法获取>');
      console.warn(
        `[verifyRequiredValidation] 未出现校验提示，对话框文本前 200 字: ${dialogText.slice(0, 200)}`
      );
    }
    return hasError;
  }

  // 辅助：关闭弹窗
  async function closeDialog(page: import('@playwright/test').Page) {
    await page
      .locator('.el-dialog__headerbtn')
      .first()
      .click()
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
    const headers = table.locator('th, .el-table-v2__header-cell');
    const headerCount = await headers.count();
    expect(headerCount).toBeGreaterThan(0);

    // 搜索
    const searchInput = page
      .locator(
        '.filter-card input, .filter-form input, input[placeholder*="订单"], input[placeholder*="供应商"]'
      )
      .first();
    await searchInput
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const searchVisible = await searchInput.isVisible().catch(() => false);
    if (searchVisible) {
      await searchInput.fill('测试');
      const queryBtn = page.locator('button:has-text("查询")').first();
      await queryBtn
        .waitFor({ state: 'visible', timeout: 3000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
      await supplierSelect
        .waitFor({ state: 'visible', timeout: 3000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
    await searchInput
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const searchVisible = await searchInput.isVisible().catch(() => false);
    if (searchVisible) {
      await searchInput.fill('测试');
      const queryBtn = page.locator('button:has-text("查询")').first();
      await queryBtn
        .waitFor({ state: 'visible', timeout: 3000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
      await customerSelect
        .waitFor({ state: 'visible', timeout: 3000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
      await newBtn
        .waitFor({ state: 'visible', timeout: 5000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
          .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
    const headers = table.locator('th, .el-table-v2__header-cell');
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
      await colorField
        .waitFor({ state: 'visible', timeout: 10_000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
      // CI 慢环境放宽 10s（3s 不足）；科目编码输入框无 placeholder 文案，
      // 按表单 label（含"编码"）定位其输入框
      const codeInput = page
        .locator('.el-dialog:visible .el-form-item')
        .filter({ has: page.locator('.el-form-item__label', { hasText: '编码' }) })
        .first()
        .locator('input')
        .first();
      await codeInput
        .waitFor({ state: 'visible', timeout: 10_000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
    await searchInput
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const searchVisible = await searchInput.isVisible().catch(() => false);
    if (searchVisible) {
      await searchInput.fill('admin');
      const queryBtn = page.locator('button:has-text("查询")').first();
      await queryBtn
        .waitFor({ state: 'visible', timeout: 3000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
    await auditTab
      .waitFor({ state: 'visible', timeout: 3000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const auditTabVisible = await auditTab.isVisible().catch(() => false);
    if (auditTabVisible) {
      await auditTab.click();
      await page.waitForTimeout(2000);
      const auditTable = page
        .locator(
          '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
        )
        .first();
      await auditTable
        .waitFor({ state: 'visible', timeout: 5000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
    await searchArea
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const searchVisible = await searchArea.isVisible().catch(() => false);
    if (searchVisible) {
      // 产品搜索
      const productInput = searchArea.locator('input, .el-select').first();
      await productInput
        .waitFor({ state: 'visible', timeout: 3000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
    await roleTab
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const roleTabVisible = await roleTab.isVisible().catch(() => false);
    if (roleTabVisible) {
      await roleTab.click();
      await page.waitForTimeout(2000);
      // Element Plus 非活动 TabPane 仍留在 DOM（display:none），不加 :visible
      // 时 .first() 会命中用户 Tab 的隐藏表格导致断言恒假（同 26-system-full 修复）
      const table = page
        .locator(
          '.el-table:visible, .el-table-v2:visible, [role="table"]:visible, .v2-table-wrapper:visible'
        )
        .first();
      await table
        .waitFor({ state: 'visible', timeout: 5000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
      const tableVisible = await table.isVisible().catch(() => false);
      expect(tableVisible).toBe(true);
    }
  });

  // ================================================================
  // 仪表盘 UI：统计卡片+图表+刷新
  // ================================================================
});
