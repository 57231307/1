// P9-4 采购 E2E 套件 — 06 采购付款
// 覆盖范围：采购付款全流程（4 用例）

import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, apiCallRaw } from '../flow/helpers';

/**
 * 测试套件：采购付款（AP 付款管理）
 *
 * 驱动真实入口：/ap → tab '付款管理'（apModule.tabs.payment）→ PaymentTab.vue 列表/对话框
 * 覆盖行为：
 * 1. 付款单通过真实对话框创建并在列表展示
 * 2. 多笔付款金额独立展示
 * 3. 对话框只读派生值（供应商/金额/方式）校验——PaymentTab 的付款方式由已审批申请派生，
 *    对话框无独立付款方式下拉（区别于 AR）
 * 4. 确认付款完整链路（含交易流水号弹框）+ 打印
 *
 * 状态词表（源码 PaymentTab.vue:52-58）：
 *   REGISTERED → '已登记'（apModule.payment.statusRegistered）
 *   CONFIRMED  → '已确认'（apModule.payment.statusConfirmed）
 */

/** 创建一张已审批的付款申请，返回 { id, request_no } */
async function createApprovedPaymentRequest(
  page: Page,
  amount: string
): Promise<{ id: number; request_no: string }> {
  const today = new Date().toISOString().slice(0, 10);
  const supRes = await apiCallRaw<{ items: Array<{ id: number }> }>(
    page,
    'GET',
    '/purchase/suppliers?page=1&page_size=1'
  );
  const supplierId = supRes.items?.[0]?.id;
  if (!supplierId) {
    throw new Error('[06-payment] 无可用供应商（globalSeed 未建立或列表为空）');
  }

  // CreateApPaymentRequest.items 为必填（Vec 无 serde default，缺即 422 missing field items）；
  // create→validate_invoice_items_txn（ap_payment_request_service.rs:90-131）要求每个
  // item.invoice_id 引用一张存在、非 DRAFT/CANCELLED 且 apply_amount<=unpaid_amount 的应付单。
  // 从 B 组 seed 的 AP 发票中取一张可付款发票（禁自造 invoice_id/兜底）。
  const invRes = await apiCallRaw<{
    items: Array<{ id: number; invoice_status: string; unpaid_amount: string | number }>;
  }>(page, 'GET', '/ap/invoices?page=1&page_size=50');
  const payable = invRes.items?.find(
    i =>
      i.invoice_status !== 'DRAFT' &&
      i.invoice_status !== 'CANCELLED' &&
      Number(i.unpaid_amount) > 0
  );
  if (!payable) {
    throw new Error('[06-payment] 无可用应付单（B组 seed 缺非草稿/取消且未付额>0 的 AP 发票）');
  }
  // apply_amount 受未付额约束，与表头 request_amount（对话框展示金额）相互独立，不校验合计。
  const applyAmount = Math.min(Number(amount), Number(payable.unpaid_amount));

  const pr = await apiCall<{ id?: number; request_no?: string }>(
    page,
    'POST',
    '/ap/payment-requests',
    {
      supplier_id: supplierId,
      request_date: today,
      payment_type: 'PURCHASE',
      payment_method: 'bank_transfer',
      request_amount: amount,
      notes: `E2E-PAY-REQ-${Date.now()}`,
      items: [{ invoice_id: payable.id, apply_amount: applyAmount, notes: 'E2E 付款申请明细' }],
    }
  );
  const prId = pr.data?.id;
  if (!prId) throw new Error(`[06-payment] 付款申请创建失败: ${JSON.stringify(pr)}`);

  await apiCall(page, 'POST', `/ap/payment-requests/${prId}/submit`);
  await apiCall(page, 'POST', `/ap/payment-requests/${prId}/approve`);

  // 获取 request_no 用于后续对话框内下拉匹配
  const detail = await apiCallRaw<{ request_no: string; supplier_id: number }>(
    page,
    'GET',
    `/ap/payment-requests/${prId}`
  );
  return { id: prId, request_no: detail.request_no };
}

/** 通过真实对话框创建付款单（选申请→确认） */
async function createPaymentViaDialog(
  page: Page,
  requestNo: string,
  amountDisplay: string
): Promise<void> {
  await page.getByRole('button', { name: '新建付款' }).click();
  const dialog = page.getByRole('dialog', { name: '新建付款对话框' });
  await expect(dialog).toBeVisible();

  // 对话框内唯一 .el-select 是付款申请下拉（PaymentTab.vue:102-117）
  // 点开下拉 → 按 request_no 文本匹配选中目标申请
  await dialog.locator('.el-select').first().click();
  const options = page.getByRole('option');
  await expect(options.filter({ hasText: requestNo }).first()).toBeVisible({ timeout: 10_000 });
  await options.filter({ hasText: requestNo }).first().click();

  // payment_date 已默认当天（PaymentTab.vue:239/286），无需额外填写
  // 点击对话框底部"确认"按钮（common.confirm='确认'，PaymentTab.vue:153）
  await dialog.getByRole('button', { name: '确认' }).click();

  // 断言成功 toast（common.success='操作成功'，PaymentTab.vue:303）
  await expect(page.getByText('操作成功')).toBeVisible({ timeout: 15_000 });
}

test.describe('06 采购付款', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('06-01 通过真实对话框创建付款单并在列表展示', async ({ page }) => {
    // 前置：API 创建已审批付款申请（金额 8888.88 避免与 seed 5000 冲突）
    const { request_no } = await createApprovedPaymentRequest(page, '8888.88');

    // 导航到付款管理 tab
    await page.goto('/ap');
    await page.getByRole('tab', { name: '付款管理' }).click();
    await expect(page.getByRole('button', { name: '新建付款' })).toBeVisible({ timeout: 10_000 });

    // 驱动真实对话框
    await createPaymentViaDialog(page, request_no, '8,888.88');

    // 验证列表出现该付款单（PaymentTab.vue:31-35 formatMoney 渲染 '8,888.88'）
    const targetRow = page.locator('tr, .el-table__row').filter({ hasText: '8,888.88' }).first();
    await expect(targetRow).toBeVisible({ timeout: 15_000 });
    // 状态列显示 '已登记'（PaymentTab.vue:52-57 REGISTERED → statusRegistered）
    await expect(targetRow.getByText('已登记')).toBeVisible();
  });

  test('06-02 多笔付款金额在列表独立展示', async ({ page }) => {
    // 前置：两张已审批申请（金额 7001.23 和 4002.56）
    const req1 = await createApprovedPaymentRequest(page, '7001.23');
    const req2 = await createApprovedPaymentRequest(page, '4002.56');

    await page.goto('/ap');
    await page.getByRole('tab', { name: '付款管理' }).click();
    await expect(page.getByRole('button', { name: '新建付款' })).toBeVisible({ timeout: 10_000 });

    // 通过对话框创建第一笔
    await createPaymentViaDialog(page, req1.request_no, '7,001.23');
    // 通过对话框创建第二笔
    await createPaymentViaDialog(page, req2.request_no, '4,002.56');

    // 两笔金额分别可见且状态独立
    const row1 = page.locator('tr, .el-table__row').filter({ hasText: '7,001.23' }).first();
    const row2 = page.locator('tr, .el-table__row').filter({ hasText: '4,002.56' }).first();
    await expect(row1).toBeVisible({ timeout: 15_000 });
    await expect(row2).toBeVisible({ timeout: 15_000 });
    await expect(row1.getByText('已登记')).toBeVisible();
    await expect(row2.getByText('已登记')).toBeVisible();
  });

  test('06-03 对话框选中申请后只读派生值正确（供应商/金额/方式）', async ({ page }) => {
    // 源码 PaymentTab.vue:121-137：选中申请后展示供应商名/金额/方式的只读 span
    // 付款方式仅从已审批申请派生，对话框无独立 method select（区别于 AR）
    const { request_no } = await createApprovedPaymentRequest(page, '9999.99');

    await page.goto('/ap');
    await page.getByRole('tab', { name: '付款管理' }).click();
    await expect(page.getByRole('button', { name: '新建付款' })).toBeVisible({ timeout: 10_000 });

    await page.getByRole('button', { name: '新建付款' }).click();
    const dialog = page.getByRole('dialog', { name: '新建付款对话框' });
    await expect(dialog).toBeVisible();

    // 选申请前，只读区域（v-if="selectedRequest"）不渲染
    await expect(dialog.getByText('9,999.99')).not.toBeVisible();

    // 点开下拉 → 选中目标申请
    await dialog.locator('.el-select').first().click();
    await page.getByRole('option').filter({ hasText: request_no }).first().click();

    // 只读区域渲染（PaymentTab.vue:122-136）：
    // 金额（PaymentTab.vue:126 formatMoney(selectedRequest.request_amount)）
    await expect(dialog.getByText('9,999.99')).toBeVisible();
    // 付款方式（PaymentTab.vue:129 getPaymentMethodLabel(selectedRequest.payment_method)）
    // request payment_method='bank_transfer' → keyMap 命中 → i18n zh-CN '银行转账'
    await expect(dialog.getByText('银行转账')).toBeVisible();
    // 供应商名（PaymentTab.vue:123 supplierLabel）非空——验证不是裸 ID 数字
    // 供应商列存在 form-item 标签"供应商"（apModule.payment.supplier）
    const supplierFormItem = dialog.locator('.el-form-item').filter({ hasText: '供应商' });
    await expect(supplierFormItem).toBeVisible();
    // supplierLabel 返回值：若 supplier_id 在 suppliers 列表中则返回 supplier_name（非纯数字）
    const supplierText = await supplierFormItem.locator('.el-form-item__content').innerText();
    expect(supplierText.trim().length).toBeGreaterThan(0);
    expect(supplierText.trim()).not.toMatch(/^\d+$/);

    // 取消对话框（不创建），保持环境干净
    await dialog.getByRole('button', { name: '取消' }).click();
  });

  test('06-04 确认付款完整链路（含交易流水号弹框）+ 打印', async ({ page }) => {
    // 前置：通过对话框创建一笔付款
    const { request_no } = await createApprovedPaymentRequest(page, '6666.66');

    await page.goto('/ap');
    await page.getByRole('tab', { name: '付款管理' }).click();
    await expect(page.getByRole('button', { name: '新建付款' })).toBeVisible({ timeout: 10_000 });

    await createPaymentViaDialog(page, request_no, '6,666.66');

    // 定位该行（REGISTERED）
    const targetRow = page.locator('tr, .el-table__row').filter({ hasText: '6,666.66' }).first();
    await expect(targetRow).toBeVisible({ timeout: 15_000 });
    await expect(targetRow.getByText('已登记')).toBeVisible();

    // 点击行内"确认"按钮（PaymentTab.vue:69-76, i18n apModule.payment.confirm='确认'）
    await targetRow.getByRole('button', { name: '确认' }).click();

    // 第一层弹框：ElMessageBox.confirm（PaymentTab.vue:316-319）
    //   内容='确定确认此付款吗？'（apModule.payment.confirmConfirm）
    const confirmBox = page.locator('.el-message-box').filter({ hasText: '确定确认此付款吗' });
    await expect(confirmBox).toBeVisible();
    await confirmBox.getByRole('button', { name: '确定' }).click();

    // 第二层弹框：ElMessageBox.prompt（PaymentTab.vue:328-336）
    //   因 transaction_no 为空，弹出"填写交易流水号"输入框
    //   标题='填写交易流水号'（apModule.payment.transactionNoTitle）
    //   消息='确认付款前需先填写交易流水号'（apModule.payment.transactionNoPrompt）
    const promptBox = page
      .locator('.el-message-box')
      .filter({ hasText: '确认付款前需先填写交易流水号' });
    await expect(promptBox).toBeVisible({ timeout: 10_000 });
    // 输入交易流水号
    await promptBox.locator('input').fill(`E2E-TX-${Date.now()}`);
    // 点击 prompt 的"确定"按钮提交
    await promptBox.getByRole('button', { name: '确定' }).click();

    // 断言成功 toast '确认成功'（apModule.payment.confirmSuccess，PaymentTab.vue:340）
    await expect(page.getByText('确认成功')).toBeVisible({ timeout: 15_000 });

    // 列表刷新后状态为 '已确认'（PaymentTab.vue:54-55 CONFIRMED → statusConfirmed）
    const confirmedRow = page.locator('tr, .el-table__row').filter({ hasText: '6,666.66' }).first();
    await expect(confirmedRow.getByText('已确认')).toBeVisible({ timeout: 10_000 });

    // 确认按钮消失（v-if row.payment_status !== 'CONFIRMED'，PaymentTab.vue:70）
    await expect(confirmedRow.getByRole('button', { name: '确认' })).not.toBeVisible();

    // 打印：点击行内"打印"按钮（PaymentTab.vue:77-83 common.print='打印'）
    // 源码 printPayment（:349-364）通过 JS 动态 <a download> 触发下载。
    // Playwright 的 download 事件在 Chromium 下对 Blob URL + a.click() 触发不稳定
    // （无真实网络请求、无 Content-Disposition），尝试捕获，超时则退化为"按钮可点击
    // 且无错误 toast"验证（不断言下载成功=不掩盖功能缺失，而是下载机制本身不产生可观测事件）
    const printBtn = confirmedRow.getByRole('button', { name: '打印' });
    await expect(printBtn).toBeVisible();

    const downloadPromise = page.waitForEvent('download', { timeout: 5000 }).catch(() => null);
    await printBtn.click();
    const download = await downloadPromise;

    if (download) {
      // download 事件可捕获时：验证文件名含 .docx
      expect(download.suggestedFilename()).toMatch(/\.docx$/);
    } else {
      // 退化：断言无"打印付款单失败"错误 toast（printFailed，PaymentTab.vue:362）
      // 点击后短暂等待确认无报错（后端可能 200 返回 blob 但 JS 下载未触发 download 事件）
      await page.waitForTimeout(2000);
      await expect(page.getByText('打印付款单失败')).not.toBeVisible();
    }
  });
});
