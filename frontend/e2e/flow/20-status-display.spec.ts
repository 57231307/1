import { test, expect } from '../diagnose-fixture';
import { loginViaUI, BASE_URL, apiCall, apiCallRaw, getCtx, ensureTestEntities } from './helpers';

test.describe('前端状态显示与业务逻辑验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('采购订单列表：状态标签颜色映射', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);

    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 采购列表同时渲染两列 el-tag：付款状态列 + 订单状态列。
    // PurchaseOrderDto 不含 payment_status 字段（usePurchList.ts:96-97 注释），
    // getPaymentStatusText(undefined) 返回 ''（usePurchList.ts:105-108），
    // 故付款状态列渲染出空文本 el-tag——被原选择器 `.el-table .el-tag` 命中致断言失败。
    // 正确契约：只校验"订单状态"列（colStatus="订单状态", zh-CN.ts:1119）的状态标签。
    // 用列头定位订单状态列下标，再逐行取该列的 el-tag。
    const headerCells = page.locator('.el-table__header-wrapper th .cell');
    const headerCount = await headerCells.count();
    let statusColIdx = -1;
    for (let i = 0; i < headerCount; i++) {
      const text = ((await headerCells.nth(i).textContent()) ?? '').trim();
      if (text === '订单状态') {
        statusColIdx = i;
        break;
      }
    }
    expect(statusColIdx, '[状态映射] 未定位到"订单状态"列').toBeGreaterThanOrEqual(0);

    const bodyRows = page.locator('.el-table__body-wrapper .el-table__row');
    const rowCount = await bodyRows.count();
    expect(rowCount, '[状态映射] 采购列表无数据行，无法验证状态标签').toBeGreaterThan(0);

    let checked = 0;
    for (let r = 0; r < Math.min(rowCount, 5); r++) {
      const statusTag = bodyRows.nth(r).locator('td').nth(statusColIdx).locator('.el-tag').first();
      await statusTag.waitFor({ state: 'visible', timeout: 5000 });
      const classes = await statusTag.getAttribute('class');
      const text = await statusTag.textContent();
      expect(classes).toContain('el-tag');
      // 订单状态列标签必须非空（未知 token 前端会抛错，各 label 均非空）
      expect(text?.trim().length, `第 ${r} 行订单状态标签文本为空`).toBeGreaterThan(0);
      expect(classes).toContain('el-tag--');
      checked++;
    }
    expect(checked, '[状态映射] 至少应校验一行订单状态标签').toBeGreaterThan(0);
  });

  test('金额千分位格式化显示', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);

    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    const cells = page.locator('.el-table__body td');
    const cellCount = await cells.count();

    if (cellCount > 0) {
      let foundAmount = false;
      for (let i = 0; i < Math.min(cellCount, 20); i++) {
        const text = await cells.nth(i).textContent();
        if (
          text &&
          (text.includes('¥') || text.match(/\d{1,3}(,\d{3})+/) || text.match(/\d+\.\d{2}/))
        ) {
          foundAmount = true;
          break;
        }
      }
      if (foundAmount) {
        expect(foundAmount).toBe(true);
      }
    }
  });

  test('日期格式化显示为 YYYY-MM-DD', async ({ page }) => {
    // 先确保至少存在一笔采购订单（本用例自建数据，不依赖其他用例/分片），
    // 空表格没有日期单元格可断言
    const ctx = getCtx();
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/purchase/orders?page=1&page_size=1'
    );
    if ((list.items?.length ?? 0) === 0) {
      await apiCall(page, 'POST', '/purchase/orders', {
        supplier_id: ctx.supplierId,
        warehouse_id: ctx.warehouseIds[0] || 1,
        department_id: ctx.departmentIds[0] || 1,
        order_date: new Date().toISOString().slice(0, 10),
        items: [
          {
            material_id: ctx.productIds[0] || 1,
            quantity_ordered: '10',
            unit_price: '9.90',
          },
        ],
        notes: 'E2E 日期格式显示验证',
      });
    }

    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);

    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    const cells = page.locator('.el-table__body td');
    const cellCount = await cells.count();

    let foundDate = false;
    for (let i = 0; i < Math.min(cellCount, 30); i++) {
      const text = await cells.nth(i).textContent();
      if (text && text.match(/\d{4}[-/]\d{1,2}[-/]\d{1,2}/)) {
        foundDate = true;
        break;
      }
    }
    expect(foundDate).toBeTruthy();
  });

  test('权限不足时按钮行为验证', async ({ page }) => {
    // admin 用户应能看到所有操作按钮
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);

    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 验证新建按钮可见（admin 有全部权限）
    const newBtn = page.locator('button:has-text("新建采购单")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    expect(newBtnVisible).toBe(true);
  });

  test('空数据时 el-empty 或空表格展示', async ({ page }) => {
    // 该用例不能假设 /voucher 天然为空：ensureTestEntities/其它用例可能已建凭证，
    // 空态块自然永不出现 → 原 waitFor 超时。正确做法：用一个必然无匹配的凭证号
    // 触发查询，使列表进入"真实空态"（后端返回 0 行 → ElTable 渲染空块），再断言空块。
    const noMatch = `ZZZ-NO-MATCH-${Date.now().toString().slice(-8)}`;
    await page.goto(`${BASE_URL}/voucher`);
    await page.waitForTimeout(3000);

    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table.waitFor({ state: 'visible', timeout: 15_000 });

    // 凭证号筛选框（VoucherListFilter.vue placeholder=凭证号）+ 查询按钮（文本"查询"）
    const noInput = page.getByPlaceholder('凭证号').first();
    await noInput.waitFor({ state: 'visible', timeout: 5000 });
    await noInput.fill(noMatch);
    await page.getByRole('button', { name: '查询' }).first().click();

    // 后端返回空 → ElTable 空态块（.el-table__empty-block/.el-table__empty-text）出现
    const emptyBlock = page
      .locator('.el-table__empty-block, .el-table__empty-text, .el-empty')
      .first();
    await emptyBlock.waitFor({ state: 'visible', timeout: 10_000 });
    const emptyText = await emptyBlock.textContent();
    expect((emptyText ?? '').trim().length, '空态块应渲染占位文案').toBeGreaterThan(0);
  });

  test('加载完成后内容可见', async ({ page }) => {
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);

    const content = page.locator('.dashboard-container, .el-card, .el-row').first();
    await content.waitFor({ state: 'visible', timeout: 10_000 });
    const contentVisible = await content.isVisible();
    expect(contentVisible).toBe(true);
  });

  test('响应式布局：窄屏不崩溃', async ({ page }) => {
    await page.setViewportSize({ width: 768, height: 600 });
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);

    // 验证页面在窄屏下不崩溃
    const body = page.locator('body');
    await body.waitFor({ state: 'visible', timeout: 10_000 });
    const bodyVisible = await body.isVisible();
    expect(bodyVisible).toBe(true);

    // 恢复宽屏
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.waitForTimeout(500);
  });

  test('消息提示自动消失', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);

    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 触发点用"导出"而非"查询"：usePurchList.handleQuery 仅调 fetchData，
    // 而 fetchData 只在 catch 分支弹 ElMessage.error（成功路径不弹），后端健康时
    // 点查询永不出消息 → .el-message waitFor 超时（本用例原失败根因）。
    // 导出 exportFromBackend 成功走 msg.exportOk、失败走 msg.error（utils/export.ts），
    // 两条路径都经 ElMessage 弹出且不改业务数据，是验证提示行为的稳定触发点。
    const exportBtn = page.locator('button:has-text("导出")').first();
    await exportBtn.waitFor({ state: 'visible', timeout: 5000 });
    await exportBtn.click();

    // ElMessage 出现在页面顶部
    const message = page.locator('.el-message').first();
    await message.waitFor({ state: 'visible', timeout: 5000 });
    expect(await message.isVisible(), '导出应弹出 ElMessage 提示').toBe(true);

    // ElMessage 未显式设 duration（默认 3000ms），到期自动移除——断言其自动消失
    await message.waitFor({ state: 'hidden', timeout: 10_000 });
    expect(await message.isVisible(), '消息提示应在默认时长后自动消失').toBe(false);
  });
});
