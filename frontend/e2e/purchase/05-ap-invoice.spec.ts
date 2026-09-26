// P9-4 采购 E2E 套件 — 05 应付（AP）发票与付款
// 覆盖范围：/ap 应付发票 Tab 访问 + 新建发票；/ap 付款管理 Tab 新建付款

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

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
    // 供应商（对话框首个 combobox）
    await dialog.getByRole('combobox').first().click();
    await page.getByRole('option').first().click();
    await dialog.getByPlaceholder('请输入发票编号').fill('E2E-AP-TEST-001');
    await dialog.getByRole('spinbutton').first().fill('8000');
    await dialog.getByRole('button', { name: '确认', exact: true }).click();
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 30000 });
  });

  test('05-03 付款管理 Tab 可新建一笔付款', async ({ page }) => {
    await page.goto('/ap');
    await page.getByRole('tab', { name: '付款管理' }).click();
    await expect(page.getByRole('button', { name: '新建付款' })).toBeVisible();
    await page.getByRole('button', { name: '新建付款' }).click();
    const dialog = page.getByRole('dialog', { name: '新建付款' });
    await expect(dialog).toBeVisible();
    // 供应商
    await dialog.getByRole('combobox').first().click();
    await page.getByRole('option').first().click();
    // 付款金额
    await dialog.getByRole('spinbutton').fill('5000');
    await dialog.getByRole('button', { name: '确认', exact: true }).click();
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 30000 });
    // 列表刷新后该笔付款按金额列文本出现（真实展示 5,000.00，不虚构单号格式）
    await expect(page.getByRole('row').filter({ hasText: '5,000.00' }).first()).toBeVisible();
  });
});
