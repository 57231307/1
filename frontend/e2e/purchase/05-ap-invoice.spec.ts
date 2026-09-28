// P9-4 采购 E2E 套件 — 05 应付（AP）发票与付款
// 覆盖范围：/ap 应付发票 Tab 访问 + 新建发票；/ap 付款管理 Tab 新建付款

import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, apiCallRaw } from '../flow/helpers';
import { pickSelectIn } from '../flow/ui-helpers';

/**
 * 建一张「已审批」付款申请（真实 API 前置，非 mock）并返回 request_no。
 * 付款单只能从已审批付款申请派生（方案B，见 PaymentTab.vue），故「新建付款」用例
 * 必须先存在一张未被付款占用的 APPROVED 申请，否则对话框下拉为空、pickSelectIn 超时。
 * 与 purchase/06-payment.spec.ts 同口径：items.invoice_id 引用真实非 DRAFT/非 CANCELLED
 * 且 unpaid_amount>0 的应付单（globalSeed 步骤14 已建 3 张 AUDITED 发票）。
 */
async function seedApprovedPaymentRequest(
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
  if (!supplierId) throw new Error('[05-03] 无可用供应商（globalSeed 缺供应商）');

  const invRes = await apiCallRaw<{
    items: Array<{ id: number; invoice_status: string; unpaid_amount: string | number }>;
  }>(page, 'GET', '/ap/invoices?page=1&page_size=50');
  const payable = invRes.items?.find(
    i =>
      i.invoice_status !== 'DRAFT' &&
      i.invoice_status !== 'CANCELLED' &&
      Number(i.unpaid_amount) > 0
  );
  if (!payable) throw new Error('[05-03] 无可付款应付单（globalSeed 缺 AUDITED AP 发票）');
  const applyAmount = Math.min(Number(amount), Number(payable.unpaid_amount));

  const pr = await apiCall<{ id?: number }>(page, 'POST', '/ap/payment-requests', {
    supplier_id: supplierId,
    request_date: today,
    payment_type: 'PURCHASE',
    payment_method: 'bank_transfer',
    request_amount: amount,
    notes: `E2E-AP-PAY-REQ-${Date.now()}`,
    items: [{ invoice_id: payable.id, apply_amount: applyAmount, notes: 'E2E 付款申请明细' }],
  });
  const prId = pr.data?.id;
  if (!prId) throw new Error(`[05-03] 付款申请创建失败: ${JSON.stringify(pr)}`);
  await apiCall(page, 'POST', `/ap/payment-requests/${prId}/submit`);
  await apiCall(page, 'POST', `/ap/payment-requests/${prId}/approve`);
  const detail = await apiCallRaw<{ request_no: string }>(
    page,
    'GET',
    `/ap/payment-requests/${prId}`
  );
  return { id: prId, request_no: detail.request_no };
}

/**
 * 真实 UI 事实（据 views/ap/index.vue、tabs/InvoiceTab.vue、tabs/PaymentTab.vue、locales 核对）：
 * - 应付为 /ap 扁平 Tab 页（apModule.tabs）：'应付发票'(invoice，默认激活) / '付款管理'(payment) /
 *   '付款申请' / '核销管理' / '对账管理' / '应付报表'。
 *   采购订单查看对话框 PurchaseViewDialog 无“内嵌应付单 tab”，原 05-01/05-02 系按不存在的 UI 编写。
 * - InvoiceTab：页标题 '应付发票'、按钮 '新建发票'、列表列 发票编号/供应商/发票金额/税额/已付/未付/状态、
 *   行内 '详情' / DRAFT 行 '审核' / '取消'。新建应付发票对话框（createAria '新建应付发票对话框'）
 *   字段：供应商 select / 发票编号 input(占位 '请输入发票编号') / 发票日期 / 到期日期 /
 *   发票金额 spinbutton / 税额 spinbutton；底部 '取消' / '确认'。提交成功 '操作成功'(common.success)。
 * - PaymentTab：'新建付款' → 对话框(createAria '新建付款对话框') 供应商 select / 付款日期 /
 *   付款金额 spinbutton / 付款方式 select(默认银行转账) / 银行账号 / 备注；底部 '确认'。
 *   成功 ElMessage.success(common.success)='操作成功'。AP 付款金额列以 toLocaleString 两位小数展示。
 */
test.describe('05 AP 应付发票与付款', () => {
  test.beforeEach(async ({ context }) => {
    await applyAuthMocks(context);
  });

  test('05-01 应付发票 Tab 默认可访问且渲染列表', async ({ page }) => {
    await page.goto('/ap');
    await expect(page.getByRole('tab', { name: '应付发票' })).toBeVisible();
    await expect(page.getByRole('heading', { name: '应付发票' })).toBeVisible();
    await expect(page.getByRole('button', { name: '新建发票' })).toBeVisible();
    await expect(page.getByText('发票编号').first()).toBeVisible();
  });

  test('05-02 应付发票可新建并落库给出成功反馈', async ({ page }) => {
    await page.goto('/ap');
    await page.getByRole('button', { name: '新建发票' }).click();
    const dialog = page.getByRole('dialog', { name: '新建应付发票' });
    await expect(dialog).toBeVisible();
    // 供应商（el-select，对话框内 label '供应商'；pickSelectIn 按精确 label 点 wrapper→选首项）
    await pickSelectIn(dialog, page, '供应商');
    await dialog.getByPlaceholder('请输入发票编号').fill('E2E-AP-TEST-001');
    // 发票日期为前端必填（InvoiceTab invoiceRules.invoice_date required），据实填写
    const dateInput = dialog.getByLabel('发票日期');
    await dateInput.click();
    await dateInput.fill('2026-08-01');
    await page.keyboard.press('Enter');
    await dialog.getByRole('spinbutton').first().fill('8000');
    await dialog.getByRole('button', { name: '确认', exact: true }).click();
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 30000 });
  });

  test('05-03 付款管理 Tab 可新建一笔付款', async ({ page }) => {
    // 前置：真实 API 建一张已审批付款申请（金额 5000，付款单从该申请只读派生）
    const { request_no } = await seedApprovedPaymentRequest(page, '5000');

    await page.goto('/ap');
    await page.getByRole('tab', { name: '付款管理' }).click();
    await expect(page.getByRole('button', { name: '新建付款' })).toBeVisible();
    await page.getByRole('button', { name: '新建付款' }).click();
    const dialog = page.getByRole('dialog', { name: '新建付款' });
    await expect(dialog).toBeVisible();
    // 付款申请（AP 付款对话框内唯一 el-select 为 label '付款申请'，供应商为只读展示非 select；
    // 按 request_no 精确定位本用例前置建的申请，避免误选其它已审批申请）
    await pickSelectIn(dialog, page, '付款申请', { optionText: request_no });
    // 点击对话框底部"确认"（PaymentTab 无独立付款金额输入，金额由服务端从申请派生）
    await dialog.getByRole('button', { name: '确认', exact: true }).click();
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 30000 });
    // 列表刷新后该笔付款按金额列文本出现（PaymentTab.formatMoney 渲染 '5,000.00'）
    await expect(page.getByRole('row').filter({ hasText: '5,000.00' }).first()).toBeVisible();
  });
});
