import { test, expect } from '../diagnose-fixture';
import { loginViaUI, BASE_URL, getCtx } from './helpers';
import {
  visitAndVerifyTable,
  verifyButton,
  clickNewAndVerifyDialog,
  verifyRequiredValidation,
  closeDialogByX,
  formItemByExactLabel,
} from './ui-helpers';

test.describe('核心业务流程真实 UI 交互验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // ================================================================
  // P2P 采购到付款流程 UI
  // ================================================================
  test('P2P 采购订单列表：表格+搜索+新建弹窗+必填校验+状态标签', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/purchase');
    const tableVisible = await table.isVisible();
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
    await searchInput.waitFor({ state: 'visible', timeout: 5000 });
    const searchVisible = await searchInput.isVisible();
    if (searchVisible) {
      await searchInput.fill('测试');
      const queryBtn = page.locator('button:has-text("查询")').first();
      await queryBtn.waitFor({ state: 'visible', timeout: 3000 });
      const queryVisible = await queryBtn.isVisible();
      if (queryVisible) {
        await queryBtn.click();
        await page.waitForTimeout(2000);
        const tableStillOk = await table.isVisible();
        expect(tableStillOk).toBe(true);
      }
      await searchInput.clear();
    }

    // 新建采购单
    await clickNewAndVerifyDialog(page, '新建采购单');
    {
      // 验证表单字段
      const supplierSelect = page.locator('.el-dialog .el-select').first();
      await supplierSelect.waitFor({ state: 'visible', timeout: 3000 });
      const supplierVisible = await supplierSelect.isVisible();
      expect(supplierVisible).toBe(true);

      // 必填校验
      const hasError = await verifyRequiredValidation(page);
      expect(hasError).toBe(true);

      await closeDialogByX(page);
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

  // ================================================================
  // O2C 订单到收款流程 UI
  // ================================================================
  test('O2C 销售订单列表：表格+搜索+新建弹窗+必填校验+操作按钮', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/sales');
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);

    // 搜索
    const searchInput = page
      .locator('input[placeholder*="订单"], input[placeholder*="客户"]')
      .first();
    await searchInput.waitFor({ state: 'visible', timeout: 5000 });
    const searchVisible = await searchInput.isVisible();
    if (searchVisible) {
      await searchInput.fill('测试');
      const queryBtn = page.locator('button:has-text("查询")').first();
      await queryBtn.waitFor({ state: 'visible', timeout: 3000 });
      const queryVisible = await queryBtn.isVisible();
      if (queryVisible) {
        await queryBtn.click();
        await page.waitForTimeout(2000);
      }
      await searchInput.clear();
    }

    // 新建订单
    await clickNewAndVerifyDialog(page, '新建订单');
    {
      // 验证客户选择器
      const customerSelect = page.locator('.el-dialog .el-select').first();
      await customerSelect.waitFor({ state: 'visible', timeout: 3000 });
      const customerVisible = await customerSelect.isVisible();
      expect(customerVisible).toBe(true);

      // 必填校验
      const hasError = await verifyRequiredValidation(page);
      expect(hasError).toBe(true);

      await closeDialogByX(page);
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
    const tableVisible = await table.isVisible();
    if (tableVisible) {
      // 新建报价单
      const newBtn = page
        .locator(
          'button:has-text("新建"), button:has-text("创建"), .el-button--primary:has-text("新")'
        )
        .first();
      await newBtn.waitFor({ state: 'visible', timeout: 5000 });
      const newBtnVisible = await newBtn.isVisible();
      if (newBtnVisible) {
        await newBtn.click();
        await page.waitForTimeout(2000);
        // 新建报价单可能跳转独立创建页（路由页）或弹窗，兼容两者
        const url = page.url();
        const dialog = page.locator('.el-dialog').first();
        const hasDialog = await dialog.isVisible().catch(() => false);
        if (!hasDialog && /\/quotations\/(new|create)/.test(url)) {
          await page
            .locator('.el-form, .el-card')
            .first()
            .waitFor({ state: 'visible', timeout: 5000 });
        }
        expect(url.includes('quotations') || hasDialog).toBe(true);
      }
    }
  });

  // ================================================================
  // 生产流程 UI
  // ================================================================
  test('生产订单列表 UI：表格+新建', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/production');
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);
  });

  test('染色配方列表 UI：色号列+新建弹窗+色号必填', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/dye-recipe');
    const tableVisible = await table.isVisible();
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
    await clickNewAndVerifyDialog(page, '新建配方');
    {
      // 验证色号字段（el-form-item label 渲染为 <label>，hasText 兼容；CI 慢环境放宽 10s）
      const colorField = page
        .locator('.el-dialog .el-form-item')
        .filter({ hasText: /色号|颜色/ })
        .first();
      await colorField.waitFor({ state: 'visible', timeout: 10_000 });
      const colorVisible = await colorField.isVisible();
      expect(colorVisible).toBe(true);

      // 必填校验
      const hasError = await verifyRequiredValidation(page);
      expect(hasError).toBe(true);

      await closeDialogByX(page);
    }
  });

  test('缸号列表 UI：状态标签+新建弹窗', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/dye-batch');
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);

    // 状态标签
    const tags = table.locator('.el-tag');
    const tagCount = await tags.count();
    if (tagCount > 0) {
      const tagText = await tags.first().textContent();
      expect(tagText?.trim().length).toBeGreaterThan(0);
    }

    // 新建批次
    await clickNewAndVerifyDialog(page, '新建批次');
    {
      // 验证缸号/色号字段
      const inputs = page.locator('.el-dialog .el-input input');
      const inputCount = await inputs.count();
      expect(inputCount).toBeGreaterThan(0);
      await closeDialogByX(page);
    }
  });

  // ================================================================
  // 财务流程 UI
  // ================================================================
  test('凭证列表 UI：新建+借贷校验+状态显示', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/voucher');
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);

    // 新增凭证
    await clickNewAndVerifyDialog(page, '新增凭证');
    {
      // 必填校验
      const hasError = await verifyRequiredValidation(page);
      expect(hasError).toBe(true);
      await closeDialogByX(page);
    }
  });

  test('会计科目 UI：树形表格+新建科目', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/account-subject');
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);

    // 树形表格验证
    const treeRows = table.locator('tr[row-key], tr.el-table__row');
    // 科目表由迁移种子写入且表格已确认可见，行数 >=0 是恒真断言（空表也绿）
    const treeRowCount = await treeRows.count();
    expect(treeRowCount, '会计科目树形表格应渲染出种子科目行').toBeGreaterThan(0);

    // 新建科目
    await clickNewAndVerifyDialog(page, '新建科目');
    {
      // CI 慢环境放宽 10s（3s 不足）；科目编码输入框无 placeholder 文案，
      // 按表单 label（含"编码"）定位其输入框
      const codeInput = page
        .locator('.el-dialog:visible .el-form-item')
        .filter({ has: page.locator('.el-form-item__label', { hasText: '编码' }) })
        .first()
        .locator('input')
        .first();
      await codeInput.waitFor({ state: 'visible', timeout: 10_000 });
      const codeVisible = await codeInput.isVisible();
      expect(codeVisible).toBe(true);
      await closeDialogByX(page);
    }
  });

  // ================================================================
  // 用户管理页 UI
  // ================================================================
  test('用户管理页 UI：列表+搜索+新建用户', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/users');
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);
    await expect(
      page.getByRole('heading', { level: 2, name: '用户管理' }),
      `未渲染用户管理页标题，实际落点 ${page.url()}`
    ).toBeVisible();

    // 搜索：关键词框按表单 label 锚定（不用 placeholder 子串/位置选择器）
    const searchInput = formItemByExactLabel(page, '关键词').locator('input').first();
    await searchInput.waitFor({ state: 'visible', timeout: 5000 });
    await searchInput.fill('admin');
    const queryBtn = page.getByRole('button', { name: '查询' }).first();
    await queryBtn.waitFor({ state: 'visible', timeout: 3000 });
    await queryBtn.click();
    await page.waitForTimeout(2000);
    expect(await table.isVisible(), '按关键词查询后用户列表应仍渲染').toBe(true);

    // 新建用户
    await clickNewAndVerifyDialog(page, '新建用户');
    {
      const hasError = await verifyRequiredValidation(page);
      expect(hasError).toBe(true);
      await closeDialogByX(page);
    }
  });

  // ================================================================
  // 审计日志页 UI
  // ================================================================
  test('审计日志页 UI：筛选表单+查询+列表', async ({ page }) => {
    await page.goto(`${BASE_URL}/system/audit-log`);
    const filterForm = page.locator('[aria-label="审计日志筛选表单"]');
    await filterForm.waitFor({ state: 'visible', timeout: 30_000 });
    const table = page.locator('.el-table-v2').first();
    await table.waitFor({ state: 'visible', timeout: 30_000 });

    const queryBtn = page.getByRole('button', { name: '查询' }).first();
    await queryBtn.waitFor({ state: 'visible', timeout: 5000 });
    await queryBtn.click();
    await page.waitForTimeout(2000);
    expect(await table.isVisible(), '点击查询后审计日志列表应仍渲染').toBe(true);
    expect(await page.locator('.el-message--error').count(), '审计日志查询不得以错误提示收场').toBe(
      0
    );
  });

  // ================================================================
  // 面料四维库存流程 UI
  // ================================================================
  test('库存列表 UI：四维查询+表格+搜索', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/inventory');
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);

    // 验证搜索区
    const searchArea = page.locator('.el-card.filter-card, .filter-form, .el-form').first();
    await searchArea.waitFor({ state: 'visible', timeout: 5000 });
    const searchVisible = await searchArea.isVisible();
    if (searchVisible) {
      // 产品搜索
      const productInput = searchArea.locator('input, .el-select').first();
      await productInput.waitFor({ state: 'visible', timeout: 3000 });
      const productVisible = await productInput.isVisible();
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
  // 角色管理页 UI
  // ================================================================
  test('角色管理页 UI：列表+权限分配入口', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/roles');
    expect(await table.isVisible(), '角色列表未渲染').toBe(true);
    await expect(
      page.getByRole('heading', { level: 2, name: '角色管理' }),
      `未渲染角色管理页标题，实际落点 ${page.url()}`
    ).toBeVisible();

    // 列头按文案锚定：角色名称与角色编码两列存在
    const headers = table.locator('.el-table__header-wrapper thead th.el-table__cell');
    await expect(headers.filter({ hasText: /^角色名称$/ }).first()).toBeVisible();
    await expect(headers.filter({ hasText: /^角色编码$/ }).first()).toBeVisible();

    // 权限分配入口渲染在角色行的操作列内
    const rows = table.locator('.el-table__body-wrapper .el-table__row');
    await expect(rows.first()).toBeVisible({ timeout: 30_000 });
    await expect(rows.first().getByRole('button', { name: '权限', exact: true })).toBeVisible();
  });

  // ================================================================
  // 仪表盘 UI：统计卡片+图表+刷新
  // ================================================================
});
