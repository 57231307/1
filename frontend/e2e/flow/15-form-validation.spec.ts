import { test, expect } from '../diagnose-fixture';
import { loginViaUI, BASE_URL } from './helpers';

// 表单校验真实 UI 交互（规则 2：禁止 catch 吞错，错误必须显式处理或传播）
// goto 用 waitUntil:domcontentloaded 防前端慢加载永久挂起（shard 15 历史教训）

test.describe('表单校验真实 UI 交互', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('采购订单表单：必填字段校验', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`, { waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(3000);

    // 等待表格加载
    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 点击新建按钮（真实文本"新建采购单"）
    const newBtn = page.locator('button:has-text("新建采购单")').first();
    await newBtn.click();

    await page.waitForTimeout(1000);

    // 等待弹窗（el-dialog）出现
    const dialog = page.locator('.el-dialog').first();
    await dialog.waitFor({ state: 'visible', timeout: 10_000 });

    // 直接点保存（不填任何字段），应触发必填校验
    const saveBtn = page
      .locator(
        '.el-dialog button:has-text("保存"), .el-dialog button:has-text("确定"), .el-dialog button:has-text("提交")'
      )
      .first();
    await saveBtn.click();

    await page.waitForTimeout(1000);

    // 验证有校验错误提示
    const hasError = await page
      .locator('.el-form-item__error, .el-message--error')
      .first()
      .waitFor({ state: 'visible', timeout: 5000 })
      .then(() => true);
    expect(hasError).toBe(true);

    // 关闭弹窗
    const closeBtn = page.locator('.el-dialog__headerbtn').first();
    await closeBtn.click();
  });

  test('销售订单表单：数量为负数校验', async ({ page }) => {
    await page.goto(`${BASE_URL}/sales`, { waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(3000);

    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 点击新建按钮（真实文本"新建订单"）
    const newBtn = page.locator('button:has-text("新建订单")').first();
    await newBtn.click();

    await page.waitForTimeout(1000);
    const dialog = page.locator('.el-dialog').first();
    await dialog.waitFor({ state: 'visible', timeout: 10_000 });

    // 在数量输入框中输入负数
    const qtyInput = page
      .locator('.el-dialog .el-input-number input, .el-dialog input[placeholder*="数量"]')
      .first();
    await qtyInput.fill('-5');

    // 触发表单校验
    await page
      .locator('.el-dialog button:has-text("保存"), .el-dialog button:has-text("确定")')
      .first()
      .click();

    await page.waitForTimeout(1000);

    // 验证校验提示
    const hasError = await page
      .locator('.el-form-item__error, .el-message--error')
      .first()
      .waitFor({ state: 'visible', timeout: 5000 })
      .then(() => true);
    expect(hasError).toBe(true);

    const closeBtn = page.locator('.el-dialog__headerbtn').first();
    await closeBtn.click();
  });

  test('凭证表单：借贷不平衡校验', async ({ page }) => {
    await page.goto(`${BASE_URL}/voucher`, { waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(3000);

    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-form')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 点击新建按钮（真实文本"新增凭证"）
    const newBtn = page.locator('button:has-text("新增凭证")').first();
    await newBtn.click();

    await page.waitForTimeout(1000);
    const dialog = page.locator('.el-dialog').first();
    await dialog.waitFor({ state: 'visible', timeout: 10_000 });

    // 输入借方金额（不输入贷方，制造借贷不平衡）。
    // el-input-number 的 v-model 仅在 change/blur（回车或失焦）时提交，
    // 单纯 fill() 只改 DOM 值不更新模型（CI 截图证据：填入 1000 但"借方合计"仍 0.00），
    // 导致 useVchrLst.ts:269 的 deep watch 不触发、total_debit 不重算、不平衡分支不命中。
    // 故 fill 后回车 + 失焦提交，再等待 watch 链（子 localForm→emit→父 form.entries→calculateTotals）回灌。
    const debitInput = page.locator('.el-dialog .el-input-number input').first();
    await debitInput.click();
    await debitInput.fill('1000');
    await debitInput.press('Enter');
    await debitInput.blur();
    await page.waitForTimeout(500);

    // 借贷不平衡提示（VoucherListForm.vue:108-110 class="error"）仅在 |借-贷|>0.01 时渲染，
    // 先断言它可见，可证明确已提交并命中"不平衡"分支（而非空分录等其它校验的假绿）。
    const imbalanceTip = page.locator('.el-dialog .total-item .error').first();
    await imbalanceTip.waitFor({ state: 'visible', timeout: 5000 });

    // 提交
    await page
      .locator('.el-dialog button:has-text("保存"), .el-dialog button:has-text("确定")')
      .first()
      .click();

    await page.waitForTimeout(1000);

    // 凭证表单校验反馈统一走 msg.warning（useVchrLst.ts:220 entriesUnbalanced → el-message--warning），
    // 与采购/销售内联错误不同，故选择器需覆盖 el-message--warning；
    // 断言出现校验提示消息（配合上方不平衡提示可见，共同证明不平衡校验生效）。
    const hasError = await page
      .locator('.el-form-item__error, .el-message--error, .el-message--warning')
      .first()
      .waitFor({ state: 'visible', timeout: 5000 })
      .then(() => true);
    expect(hasError).toBe(true);

    const closeBtn = page.locator('.el-dialog__headerbtn').first();
    await closeBtn.click();
  });
});
