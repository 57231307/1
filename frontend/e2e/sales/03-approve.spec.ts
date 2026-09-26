// P9-3 销售 E2E 套件 — 03 销售订单审批
// 覆盖范围：销售订单提交/审批通过/驳回（列表行内按钮驱动，非详情页按钮）

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

/**
 * 真实 UI 事实（据 SalesOrderTable.vue / useOlvProc.ts / locales 核对）：
 * - 销售为扁平单页 /sales，订单动作全部是列表行内按钮（SalesOrderTable.vue renderCell），
 *   不存在“进入详情页再点动作按钮”的入口（OrderDetail.vue 仅描述+明细，无动作按钮）。
 * - 行内按钮按状态条件渲染：
 *   draft → '提交'(submit) / '删除'；pending → '审批'(approve) / '驳回'(reject)；approved → '发货'。
 *   状态标签中文（sales.statusLabels）：draft='草稿' pending='待审批' approved='已审批' rejected='已驳回'。
 * - useOlvProc：
 *   handleSubmitOrder → ElMessageBox.confirm('确定提交此订单进入审批流程吗？') → msg.success('submitSuccess')='提交成功'；
 *   handleApprove → ElMessageBox.confirm('确定审批此订单吗？') → msg.success('approveSuccess')='审批成功'；
 *   handleReject → ElMessageBox.prompt('请输入驳回原因') → rejectSalesOrder → msg.success('rejectSuccess')。
 *   注：message.rejectSuccess 键在 locales 缺失（真实产品 i18n 缺口），成功 toast 文案不可依赖，
 *   故驳回用例改断言成功提示元素 .el-message--success 出现（仅在成功分支产生），不放宽为“无断言”。
 */
test.describe('03 销售订单审批', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/sales');
  });

  test('03-01 草稿订单行内可提交进入审批', async ({ page }) => {
    const draft = page.getByRole('row').filter({ hasText: '草稿' }).first();
    await expect(draft).toBeVisible();
    // 真实行内按钮 sales.table.submit = '提交'
    await draft.getByRole('button', { name: '提交', exact: true }).click();
    // ElMessageBox.confirm 默认确认按钮 '确定'
    await page.getByRole('button', { name: '确定', exact: true }).click();
    // 成功提示 message.submitSuccess = '提交成功'
    await expect(page.getByText('提交成功')).toBeVisible({ timeout: 30000 });
  });

  test('03-02 待审批订单行内可审批通过', async ({ page }) => {
    const pending = page.getByRole('row').filter({ hasText: '待审批' }).first();
    await expect(pending).toBeVisible();
    // sales.table.approve = '审批'
    await pending.getByRole('button', { name: '审批', exact: true }).click();
    await page.getByRole('button', { name: '确定', exact: true }).click();
    // approveSalesOrder 成功 → msg.success('approveSuccess') = '审批成功'
    await expect(page.getByText('审批成功')).toBeVisible({ timeout: 30000 });
  });

  test('03-03 待审批订单行内可驳回（原因必填）', async ({ page }) => {
    const pending = page.getByRole('row').filter({ hasText: '待审批' }).first();
    await expect(pending).toBeVisible();
    // sales.table.reject = '驳回'，触发 ElMessageBox.prompt('请输入驳回原因')
    await pending.getByRole('button', { name: '驳回', exact: true }).click();
    const msgBox = page.locator('.el-message-box');
    await expect(msgBox.getByText('请输入驳回原因')).toBeVisible();
    await msgBox.getByRole('textbox').fill('E2E 测试驳回：价格不符合规范');
    await msgBox.getByRole('button', { name: '确定', exact: true }).click();
    // rejectSalesOrder 成功后 refresh + msg.success(...)：因 message.rejectSuccess 缺键，
    // 断言成功提示元素出现（仅成功分支渲染），不依赖缺失的中文文案
    await expect(page.locator('.el-message--success')).toBeVisible({ timeout: 30000 });
  });
});
