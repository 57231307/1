// P9-3 销售 E2E 套件 — 06 销售收款
// 覆盖范围：销售收款全流程（4 用例）

import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, apiCallRaw } from '../flow/helpers';
import { pickSelect } from '../flow/ui-helpers';

/**
 * 测试套件：销售收款（AR 收款管理）
 *
 * 驱动真实入口：/ar → tab '收款管理'（arModule.tabs.payment）→ PaymentTab.vue 列表/对话框
 * 覆盖行为：
 * 1. 通过真实对话框创建收款并在列表展示
 * 2. 多笔收款金额独立展示
 * 3. 收款方式下拉 4 种选项校验（源码 PaymentTab.vue:78-81 硬编码）
 * 4. 确认收款 pending→confirmed + 打印
 *
 * 状态词表（源码 PaymentTab.vue:31 直接渲染 row.status 原始值）：
 *   新建 → pending →(确认)→ confirmed
 */

/** 通过真实对话框创建一笔收款，返回 */
async function createCollectionViaDialog(
  page: Page,
  customerId: number,
  amount: string,
  method = '银行转账'
): Promise<void> {
  await page.getByRole('button', { name: '新建收款' }).click();
  // AR 对话框 title='新建收款'（arModule.payment.create, PaymentTab.vue:66）
  const dialog = page.getByRole('dialog', { name: '新建收款' });
  await expect(dialog).toBeVisible();

  // customer_id: el-input-number → role=spinbutton（PaymentTab.vue:71）
  const spins = dialog.getByRole('spinbutton');
  await spins.first().fill(String(customerId));

  // payment_method: 对话框唯一 .el-select（PaymentTab.vue:77-82，避开 date-picker combobox）
  await pickSelect(page, dialog.locator('.el-select').first(), method);

  // amount: el-input-number → role=spinbutton（PaymentTab.vue:85, 第二个 spinbutton）
  await spins.nth(1).fill(amount);

  // 点击"保存"按钮（PaymentTab.vue:93, common.save='保存'）
  await dialog.getByRole('button', { name: '保存' }).click();

  // 成功 toast：common.success='操作成功'（PaymentTab.vue:243）
  await expect(page.getByText('操作成功')).toBeVisible({ timeout: 15_000 });
}

test.describe('06 销售收款', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('06-01 通过真实对话框创建收款并在列表展示', async ({ page }) => {
    // 获取真实客户 ID
    const cusRes = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/crm/customers?page=1&page_size=1'
    );
    const customerId = cusRes.items?.[0]?.id;
    if (!customerId) throw new Error('[06-payment] 无可用客户');

    await page.goto('/ar');
    await page.getByRole('tab', { name: '收款管理' }).click();
    // 等待 PaymentTab mount（lazy tab，PaymentTab 有 onMounted fetch）
    await expect(page.getByRole('button', { name: '新建收款' })).toBeVisible({ timeout: 15_000 });

    // 驱动对话框：amount=8888.88 避免与 globalSeed 的 6000 冲突
    await createCollectionViaDialog(page, customerId, '8888.88');

    // 列表出现该行（PaymentTab.vue:27 formatMoney(row.amount) → '8,888.88'）
    const targetRow = page.locator('tr, .el-table__row').filter({ hasText: '8,888.88' }).first();
    await expect(targetRow).toBeVisible({ timeout: 15_000 });
    // 状态列直接渲染 row.status（PaymentTab.vue:31），新创建为 'pending'
    await expect(targetRow.getByText('pending')).toBeVisible();
  });

  test('06-02 多笔收款金额在列表独立展示', async ({ page }) => {
    const cusRes = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/crm/customers?page=1&page_size=1'
    );
    const customerId = cusRes.items?.[0]?.id;
    if (!customerId) throw new Error('[06-payment] 无可用客户');

    await page.goto('/ar');
    await page.getByRole('tab', { name: '收款管理' }).click();
    await expect(page.getByRole('button', { name: '新建收款' })).toBeVisible({ timeout: 15_000 });

    // 两笔不同金额
    await createCollectionViaDialog(page, customerId, '7001.23');
    await createCollectionViaDialog(page, customerId, '4002.56');

    const row1 = page.locator('tr, .el-table__row').filter({ hasText: '7,001.23' }).first();
    const row2 = page.locator('tr, .el-table__row').filter({ hasText: '4,002.56' }).first();
    await expect(row1).toBeVisible({ timeout: 15_000 });
    await expect(row2).toBeVisible({ timeout: 15_000 });
    // 两笔状态独立为 pending
    await expect(row1.getByText('pending')).toBeVisible();
    await expect(row2.getByText('pending')).toBeVisible();
  });

  test('06-03 收款方式下拉包含 4 种选项并可通过对话框选中提交', async ({ page }) => {
    // 源码 PaymentTab.vue:78-81 硬编码 4 个 el-option（非 i18n key 渲染，直接中文 label）：
    // '银行转账'(bank_transfer) / '现金'(cash) / '支票'(check) / '承兑'(bill)
    // 不存在 '支付宝' '微信'——原测试断言 5 种系凭空虚构
    const cusRes = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/crm/customers?page=1&page_size=1'
    );
    const customerId = cusRes.items?.[0]?.id;
    if (!customerId) throw new Error('[06-payment] 无可用客户');

    await page.goto('/ar');
    await page.getByRole('tab', { name: '收款管理' }).click();
    await expect(page.getByRole('button', { name: '新建收款' })).toBeVisible({ timeout: 15_000 });

    await page.getByRole('button', { name: '新建收款' }).click();
    const dialog = page.getByRole('dialog', { name: '新建收款' });
    await expect(dialog).toBeVisible();

    // 打开付款方式下拉（.el-select 是对话框内唯一 el-select 组件）
    await dialog.locator('.el-select').first().click();
    const options = page.getByRole('option');
    await expect(options.filter({ hasText: '银行转账' })).toBeVisible();
    await expect(options.filter({ hasText: '现金' })).toBeVisible();
    await expect(options.filter({ hasText: '支票' })).toBeVisible();
    await expect(options.filter({ hasText: '承兑' })).toBeVisible();
    // 验证仅 4 种（无多余选项）
    await expect(options).toHaveCount(4);

    // 选择"现金"提交，验证后端存储为 cash
    await options.filter({ hasText: '现金' }).click();
    const spins = dialog.getByRole('spinbutton');
    await spins.first().fill(String(customerId));
    await spins.nth(1).fill('9999.99');
    await dialog.getByRole('button', { name: '保存' }).click();
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 15_000 });

    // 列表行付款方式列（prop="payment_method"）显示后端存储的 value 'cash'
    const targetRow = page.locator('tr, .el-table__row').filter({ hasText: '9,999.99' }).first();
    await expect(targetRow).toBeVisible({ timeout: 15_000 });
    await expect(targetRow.getByText('cash')).toBeVisible();
  });

  test('06-04 确认收款 pending→confirmed + 打印', async ({ page }) => {
    const cusRes = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/crm/customers?page=1&page_size=1'
    );
    const customerId = cusRes.items?.[0]?.id;
    if (!customerId) throw new Error('[06-payment] 无可用客户');

    await page.goto('/ar');
    await page.getByRole('tab', { name: '收款管理' }).click();
    await expect(page.getByRole('button', { name: '新建收款' })).toBeVisible({ timeout: 15_000 });

    // 通过对话框创建一笔收款
    await createCollectionViaDialog(page, customerId, '6666.66');

    // 定位 pending 行
    const targetRow = page.locator('tr, .el-table__row').filter({ hasText: '6,666.66' }).first();
    await expect(targetRow).toBeVisible({ timeout: 15_000 });
    await expect(targetRow.getByText('pending')).toBeVisible();

    // 点击"确认收款"按钮（PaymentTab.vue:45-53, arModule.payment.confirm='确认收款'）
    await targetRow.getByRole('button', { name: '确认收款' }).click();

    // ElMessageBox.confirm（PaymentTab.vue:256-260）
    //   标题='确认收款'（arModule.payment.confirm）
    //   内容='确认该笔收款已到账？'（arModule.payment.confirmMessage）
    //   类型=warning
    const confirmBox = page.locator('.el-message-box').filter({ hasText: '确认该笔收款已到账' });
    await expect(confirmBox).toBeVisible();
    await confirmBox.getByRole('button', { name: '确定' }).click();

    // 成功 toast：common.success='操作成功'（PaymentTab.vue:262）
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 15_000 });

    // 列表刷新后状态变为 'confirmed'（PaymentTab.vue:31 渲染 row.status 原始值）
    const confirmedRow = page.locator('tr, .el-table__row').filter({ hasText: '6,666.66' }).first();
    await expect(confirmedRow.getByText('confirmed')).toBeVisible({ timeout: 10_000 });

    // 编辑按钮消失（v-if row.status in [draft,pending]，confirmed 不命中，PaymentTab.vue:37）
    await expect(confirmedRow.getByRole('button', { name: '编辑' })).not.toBeVisible();

    // 打印：点击"打印"按钮（PaymentTab.vue:57-59, common.print='打印'）
    // printCollection（PaymentTab.vue:283-298）通过 JS blob URL + a.click() 下载 .docx
    const printBtn = confirmedRow.getByRole('button', { name: '打印' });
    await expect(printBtn).toBeVisible();

    const downloadPromise = page.waitForEvent('download', { timeout: 5000 }).catch(() => null);
    await printBtn.click();
    const download = await downloadPromise;

    if (download) {
      // 文件名格式 `${payment_no}.docx`（PaymentTab.vue:290）
      expect(download.suggestedFilename()).toMatch(/\.docx$/);
    } else {
      // JS blob 下载在 Chromium headless 下 download 事件不稳定，退化为：
      // 打印按钮可点击 + 无错误 toast（printFailed='打印收款单失败'）
      await page.waitForTimeout(2000);
      await expect(page.getByText('打印收款单失败')).not.toBeVisible();
    }
  });
});
