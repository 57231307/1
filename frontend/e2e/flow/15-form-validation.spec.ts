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
    // el-input-number 的 v-model 仅在 change（失焦/回车）时提交，单纯 fill() 只改 DOM 值
    // 不驱动内部 currentValue → useVchrLst.ts:269 的 deep watch 不触发、total_debit 不重算、
    // VoucherListForm.vue:109 的 |借-贷|>0.01 不平衡分支不命中，`.total-item .error` 永不出现。
    // 用真实键盘逐字符输入（pressSequentially 逐键派发 keydown/input）填满当前值，再 Tab 失焦 +
    // Enter 提交，强制触发 el-input-number 的 change → v-model 回灌 → 合计重算链。
    const debitInput = page.locator('.el-dialog .el-input-number input').first();
    await debitInput.click();
    await debitInput.press('ControlOrMeta+a');
    await debitInput.pressSequentially('1000', { delay: 30 });
    await debitInput.press('Tab');
    await debitInput.press('Enter');
    await debitInput.blur();
    await page.waitForTimeout(500);

    // 借贷不平衡提示（VoucherListForm.vue:108-110 class="error"）仅在 |借-贷|>0.01 时渲染，
    // 先断言它可见，可证明确已提交并命中"不平衡"分支（而非空分录等其它校验的假绿）。
    // 用 poll 轮询等重算链（子 localForm→emit→父 form.entries→calculateTotals→回灌）稳定收敛，
    // 而非依赖单一固定 sleep。
    await expect
      .poll(
        async () =>
          await page
            .locator('.el-dialog .total-item .error')
            .first()
            .isVisible()
            .catch(() => false),
        {
          message: '借方有值、贷方为 0 时，应渲染借贷不平衡提示（.total-item .error）',
          timeout: 10_000,
        }
      )
      .toBe(true);

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
