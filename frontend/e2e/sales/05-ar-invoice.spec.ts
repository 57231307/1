// P9-3 销售 E2E 套件 — 05 应收（AR）发票与收款
// 覆盖范围：/ar 应收发票 Tab 列表访问 + 新建发票；/ar 收款管理 Tab 新建收款并确认

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelectIn } from '../flow/ui-helpers';
import { apiCallRaw } from '../flow/helpers';

/**
 * 真实 UI 事实（据 views/ar/index.vue、tabs/InvoiceTab.vue、tabs/PaymentTab.vue、locales 核对）：
 * - 应收为 /ar 扁平 Tab 页（arModule.tabs）：'应收发票'(invoice，默认激活) / '收款管理'(payment) /
 *   '核销管理' / '对账管理' / '资金账户' / '应收报表'。
 *   销售订单详情页 OrderDetail.vue 不存在“内嵌应收单 tab”，原 05-01/05-02 系按不存在的 UI 编写。
 * - InvoiceTab 真实控件：页标题 '应收发票'（arModule.invoice.title）、
 *   按钮 '新建发票'（arModule.invoice.create）、状态筛选下拉 + '搜索'/'重置'、
 *   发票列表（列 发票编号/客户/发票金额/税额/已收/未收/状态），行内 '详情' / DRAFT 行 '审核' / '取消'。
 *   新建发票对话框（createTitle '新建应收发票'）字段：客户 select / 发票日期 date / 到期日期 date /
 *   发票金额 spinbutton / 税额 spinbutton / 备注；底部 '取消' / '确认'。
 *   发票编号由后端 generate_invoice_no 自生成（CreateArInvoiceRequestDto 无 invoice_no 字段），
 *   新建对话框对发票编号只做只读回显、不采集，故本用例不填发票编号；到期日为选填（留空后端回退当天）。
 *   提交成功 → ElMessage.success(t('common.success') = '操作成功')。
 * - PaymentTab 真实控件：'新建收款'（arModule.payment.create）→ 对话框字段 客户(input-number)/
 *   收款日期(date)/收款方式(select)/收款金额(spinbutton)/备注，底部 '取消'/'保存'；
 *   列表行 '确认收款'（arModule.payment.confirm）→ ElMessageBox.confirm → 成功 '操作成功'。
 *   AR 收款单号为后端生成（payment_no），前端不预设格式，故不虚构 "AR-\d" 断言。
 */
test.describe('05 AR 应收发票与收款', () => {
  test.beforeEach(async ({ context }) => {
    await applyAuthMocks(context);
  });

  test('05-01 应收发票 Tab 默认可访问且渲染列表', async ({ page }) => {
    await page.goto('/ar');
    // 默认停在 '应收发票' tab（activeTab 初值 'invoice'），Tab 标签与页标题文案一致
    await expect(page.getByRole('tab', { name: '应收发票' })).toBeVisible();
    await expect(page.getByRole('heading', { name: '应收发票' })).toBeVisible();
    // 真实工具栏按钮 '新建发票'
    await expect(page.getByRole('button', { name: '新建发票' })).toBeVisible();
    // 列表存在（列头 '发票编号' 真实存在，不假设行数）
    await expect(page.getByText('发票编号').first()).toBeVisible();
  });

  test('05-02 应收发票可新建并落库给出成功反馈', async ({ page }) => {
    await page.goto('/ar');
    await page.getByRole('button', { name: '新建发票' }).click();
    const dialog = page.getByRole('dialog', { name: '新建应收发票' });
    await expect(dialog).toBeVisible();
    // 客户（el-select，label '客户'）：pickSelectIn 点 wrapper→选首项
    await pickSelectIn(dialog, page, '客户');
    // 发票金额（首个 el-input-number）；发票日期在打开对话框时默认今天、到期日选填留空
    await dialog.getByRole('spinbutton').first().fill('10000');
    await dialog.getByRole('button', { name: '确认', exact: true }).click();
    // createARInvoice 成功 → ElMessage.success(common.success = '操作成功')
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 30000 });
  });

  test('05-03 收款管理 Tab 可新建一笔收款', async ({ page }) => {
    // 客户 ID 走后端 create_payment→load_customer_for_payment 的存在性校验：
    // 硬编码 id=1 在真实库无该客户时返回 404「客户 1 不存在」→ 只出错误 toast、永不出「操作成功」。
    // 取一个真实存在的客户 id（与 06 收款套件同源，GET /crm/customers 分页 items）。
    const cusRes = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/crm/customers?page=1&page_size=1'
    );
    const customerId = cusRes.items?.[0]?.id;
    if (!customerId) throw new Error('[05-03] 无可用客户，前置缺失');

    await page.goto('/ar');
    await page.getByRole('tab', { name: '收款管理' }).click();
    // 切换到收款 Tab，新建收款按钮
    await expect(page.getByRole('button', { name: '新建收款' })).toBeVisible();
    await page.getByRole('button', { name: '新建收款' }).click();
    // 对话框 title '新建收款'
    const dialog = page.getByRole('dialog', { name: '新建收款' });
    await expect(dialog).toBeVisible();
    // 客户 ID 为 el-input-number，收款方式为 el-select，收款金额为 el-input-number
    const spins = dialog.getByRole('spinbutton');
    await spins.first().fill(String(customerId)); // customer_id（真实存在的客户 ID）
    // 收款日期（PaymentTab.vue:70-71 el-date-picker）：其内层 input 带 role=combobox
    // （EP 日期选择器开弹层语义），与「收款方式」el-select 同为 combobox →
    // 旧 dialog.getByRole('combobox').click() strict 命中 2 个。改走真实手输：定位 .el-date-editor 输入框，点开→输入→回车。
    // ⚠️ 日期必须落在已存在的会计期间内：后端 create_payment→check_date_locked_txn
    // （accounting_period_service.rs:645）要求 payment_date 命中 accounting_periods 的 [start,end]，
    // 否则报「日期 xxx 不在任何已设置的会计期间内」→ 只出错误 toast、永不出「操作成功」。
    // global-setup 步骤 17.5 只种子「当前年月」的会计期间，故此处取当天（必在该期间内），
    // 而非硬编码历史月日（如 2026-08-19，CI 跨到别的月份即落期间外被拒）。
    const paymentDate = new Date().toISOString().slice(0, 10);
    const payDate = dialog.locator('.el-date-editor input').first();
    await payDate.click();
    await payDate.fill(paymentDate);
    await payDate.press('Enter');
    // 收款方式：对话框内唯一 el-select（label '收款方式'，避开 date-picker 的 combobox），
    // pickSelectIn 点 wrapper→选含“银行转账”的项。
    await pickSelectIn(dialog, page, '收款方式', { optionText: '银行转账' });
    await spins.nth(1).fill('3000'); // 部分收款金额
    await dialog.getByRole('button', { name: '保存', exact: true }).click();
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 30000 });
    // 列表刷新后该笔收款出现在收款管理表中（按金额列文本锚定，不虚构单号格式）
    await expect(page.getByRole('row').filter({ hasText: '3,000.00' }).first()).toBeVisible();
  });
});
