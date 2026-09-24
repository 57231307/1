// P9-3 销售 E2E 套件 — 04 销售发货
// 覆盖范围：已审批订单行内「发货」→ DeliveryDialog 出库（真实对话框，非详情页按钮）

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

/**
 * 真实 UI 事实（据 SalesOrderTable.vue / DeliveryDialog.vue / useOlv.ts / locales 核对）：
 * - 发货入口：/sales 列表已审批行（状态标签 '已审批'）行内按钮 sales.table.deliver = '发货'
 *   → OrderListView.onDelivery → olv.prepareDelivery → 打开 DeliveryDialog。
 *   不存在“详情页创建发货单/保存按钮/发货单号 DN-”等 UI。
 * - DeliveryDialog（aria-label sales.delivery.dialogAriaLabel='销售发货对话框'，标题 '销售发货'）真实控件：
 *   只读销售单号/客户；发货日期 date picker 占位 '选择日期'；仓库 el-select（label '仓库'）；
 *   明细 el-table（aria-label '销售发货明细表'）列 产品/库存行/缸号/色号/批次/订单数量/已发货/
 *   本次发货(el-input-number)/单价/备注；底部 '取消' / '确定发货'。
 *   出库需先选仓库加载库存行，再按四维(色号/批次/缸号)选库存行；本次发货 el-input-number
 *   的 :max = 订单数量 - 已发货（DeliveryDialog.vue 第 103 行），超限输入会被钳制。
 * - 校验（handleSubmit）：未选仓库 → ElMessage.warning('请选择仓库')；未填发货日期 → '请选择发货日期'。
 * - 成功：handleDeliverySubmit → shipSalesOrder → msg.success('shipSuccess') = '发货成功'。
 */
test.describe('04 销售发货', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/sales');
  });

  test('04-01 已审批订单行内「发货」打开发货对话框', async ({ page }) => {
    const approved = page.getByRole('row').filter({ hasText: '已审批' }).first();
    await expect(approved).toBeVisible();
    await approved.getByRole('button', { name: '发货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '销售发货对话框' });
    await expect(dialog).toBeVisible();
    // 底部真实按钮 sales.delivery.confirmDelivery = '确定发货'
    await expect(dialog.getByRole('button', { name: '确定发货' })).toBeVisible();
  });

  test('04-02 未选仓库直接确定发货被拦截', async ({ page }) => {
    const approved = page.getByRole('row').filter({ hasText: '已审批' }).first();
    await expect(approved).toBeVisible();
    await approved.getByRole('button', { name: '发货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '销售发货对话框' });
    await expect(dialog).toBeVisible();
    // 未选仓库即点确定发货
    await dialog.getByRole('button', { name: '确定发货' }).click();
    // 真实校验 sales.delivery.warehouseRequired = '请选择仓库'
    await expect(page.getByText('请选择仓库')).toBeVisible();
    await expect(page.locator('.el-message--success')).toHaveCount(0);
  });

  test('04-03 本次发货数量受订单可发数量钳制', async ({ page }) => {
    const approved = page.getByRole('row').filter({ hasText: '已审批' }).first();
    await expect(approved).toBeVisible();
    await approved.getByRole('button', { name: '发货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '销售发货对话框' });
    // 先选仓库以启用库存行与本次发货上限计算
    await dialog.getByRole('combobox').first().click();
    await page.getByRole('option').first().click();
    const qty = dialog.getByRole('spinbutton').first();
    // 输入远超订单数量的值
    await qty.fill('99999');
    await page.keyboard.press('Tab');
    // el-input-number :max 生效：失焦后实际值被钳制到可发上限（≠ 99999）
    await expect(qty).not.toHaveValue('99999');
  });

  test('04-04 填写发货维度并确定发货成功后给出成功提示', async ({ page }) => {
    const approved = page.getByRole('row').filter({ hasText: '已审批' }).first();
    await expect(approved).toBeVisible();
    await approved.getByRole('button', { name: '发货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '销售发货对话框' });
    // 仓库
    await dialog.getByRole('combobox').first().click();
    await page.getByRole('option').first().click();
    // 发货日期
    await dialog.getByPlaceholder('选择日期').fill('2026-12-31');
    await page.keyboard.press('Enter');
    // 库存行（第 2 个 combobox，仓库选定后启用），选第一条四维库存行
    const stockRow = dialog.getByRole('combobox').nth(1);
    await stockRow.click();
    await page.getByRole('option').first().click();
    // 本次发货数量（默认取上限）
    await dialog.getByRole('spinbutton').first().fill('1');
    await dialog.getByRole('button', { name: '确定发货' }).click();
    // 真实出库端点成功 → msg.success('shipSuccess') = '发货成功'
    await expect(page.getByText('发货成功')).toBeVisible({ timeout: 30000 });
  });
});
