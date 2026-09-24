// P9-4 采购 E2E 套件 — 02 采购订单审批
// 覆盖范围：草稿提交 / 审批通过 / 驳回（列表行内按钮驱动，非详情页按钮）

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

/**
 * 真实 UI 事实（据 views/purchase/index.vue、components/PurchaseTable.vue、usePurchAct.ts、locales 核对）：
 * - 采购为扁平单页 /purchase（无 /purchase/order/list、无 /purchase/order/detail/:id 路由），
 *   订单动作为 PurchaseTable 行内按钮，不存在“进详情页点提交/审批”。
 * - 行内按钮按状态渲染（purchase.table.*）：
 *   DRAFT/REJECTED → '提交'(submit) / '编辑'；DRAFT → '删除'；APPROVED → '收货'(receive)；
 *   PENDING_APPROVAL → '审批'(approve) / '驳回'(reject)。均有 '详情' 入口。
 * - 状态标签中文（purchase.statusLabels，大写键）：DRAFT='草稿' PENDING_APPROVAL='待审批'
 *   APPROVED='已审批' REJECTED='已驳回' PARTIAL_RECEIVED='部分收货' COMPLETED='已完成'。
 *   原用例用 '待审核/已审核' 与真实 '待审批/已审批' 不符。
 * - usePurchAct：handleApprove → ElMessageBox.confirm → msg.success('purchaseOrderApproved')=
 *   '采购单 {orderNo} 审批成功'（message 键存在，可断言 '审批成功'）；
 *   handleSubmitOrder / handleReject 分别用 purchaseOrderSubmitted / purchaseOrderRejected，
 *   二者在 locales 缺失（真实 i18n 缺口，toast 会回显键名），故改断言成功提示元素
 *   .el-message--success（仅成功分支产生），不放宽为无断言。
 */
test.describe('02 采购订单审批', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/purchase');
  });

  test('02-01 草稿采购订单行内可提交进入审批', async ({ page }) => {
    const draft = page.getByRole('row').filter({ hasText: '草稿' }).first();
    await expect(draft).toBeVisible();
    await draft.getByRole('button', { name: '提交', exact: true }).click();
    // ElMessageBox.confirm('确定提交采购单 ... 进入审批流程吗？')
    await page.getByRole('button', { name: '确定', exact: true }).click();
    // 提交成功（purchaseOrderSubmitted 键缺失→断言成功提示元素）
    await expect(page.locator('.el-message--success')).toBeVisible({ timeout: 30000 });
  });

  test('02-02 待审批采购订单行内可审批通过', async ({ page }) => {
    const pending = page.getByRole('row').filter({ hasText: '待审批' }).first();
    await expect(pending).toBeVisible();
    await pending.getByRole('button', { name: '审批', exact: true }).click();
    await page.getByRole('button', { name: '确定', exact: true }).click();
    // msg.success('purchaseOrderApproved') = '采购单 {orderNo} 审批成功'
    await expect(page.getByText('审批成功')).toBeVisible({ timeout: 30000 });
  });

  test('02-03 待审批采购订单行内可驳回（原因必填）', async ({ page }) => {
    const pending = page.getByRole('row').filter({ hasText: '待审批' }).first();
    await expect(pending).toBeVisible();
    await pending.getByRole('button', { name: '驳回', exact: true }).click();
    // ElMessageBox.prompt('请输入驳回原因', ...)
    const msgBox = page.locator('.el-message-box');
    await expect(msgBox.getByText('请输入驳回原因')).toBeVisible();
    await msgBox.getByRole('textbox').fill('E2E 测试驳回：数量超预算');
    await msgBox.getByRole('button', { name: '确定', exact: true }).click();
    // 驳回成功（purchaseOrderRejected 键缺失→断言成功提示元素）
    await expect(page.locator('.el-message--success')).toBeVisible({ timeout: 30000 });
  });
});
