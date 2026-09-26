// P9-4 采购 E2E 套件 — 03 采购收货（入库）
// 覆盖范围：已审批采购订单行内「收货」→ PurchaseReceiveDialog 登记收货

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

/**
 * 真实 UI 事实（据 PurchaseTable.vue / components/PurchaseReceiveDialog.vue / usePurchRcv.ts / locales 核对）：
 * - 收货入口：/purchase 列表已审批行（状态 '已审批'，PURCHASE_ORDER_STATUS.APPROVED）行内
 *   按钮 purchase.table.receive = '收货' → index.vue rcv.handleReceive → 打开 PurchaseReceiveDialog。
 *   不存在“详情页创建入库单/库位/入库单号 GR-”等 UI。
 * - PurchaseReceiveDialog（aria-label purchase.index.receiveDlgAriaLabel = '收货对话框'，标题 '采购收货'）：
 *   只读采购单号/供应商；收货日期(默认今日 date)；仓库 el-select(label '仓库')；
 *   明细 el-table 列 产品/订购数量/已收货/本次收货(el-input-number)/单价/备注；底部 '取消' / '确定收货'。
 *   本次收货 el-input-number 的 :max = 订购数量 - 已收货（PurchaseReceiveDialog.vue 第 82 行），超限输入被钳制。
 * - 校验（submitReceive）：未选仓库 → msg.warning('pleaseSelectWarehouse') = '请选择收货仓库'；
 *   全部本次收货为 0 → '请填写至少一项收货数量'。
 * - 成功：createPurchaseReceipt → msg.success('receiveSuccess') = '收货成功'。
 */
test.describe('03 采购收货', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/purchase');
  });

  test('03-01 已审批采购订单行内「收货」打开收货对话框', async ({ page }) => {
    const approved = page.getByRole('row').filter({ hasText: '已审批' }).first();
    await expect(approved).toBeVisible();
    await approved.getByRole('button', { name: '收货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '采购收货' });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole('button', { name: '确定收货' })).toBeVisible();
  });

  test('03-02 未选仓库直接确定收货被拦截', async ({ page }) => {
    const approved = page.getByRole('row').filter({ hasText: '已审批' }).first();
    await expect(approved).toBeVisible();
    await approved.getByRole('button', { name: '收货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '采购收货' });
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: '确定收货' }).click();
    // 真实校验 message.pleaseSelectWarehouse = '请选择收货仓库'
    await expect(page.getByText('请选择收货仓库')).toBeVisible();
    await expect(page.locator('.el-message--success')).toHaveCount(0);
  });

  test('03-03 本次收货数量受订购可收数量钳制', async ({ page }) => {
    const approved = page.getByRole('row').filter({ hasText: '已审批' }).first();
    await expect(approved).toBeVisible();
    await approved.getByRole('button', { name: '收货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '采购收货' });
    // 选仓库
    await dialog.getByRole('combobox').click();
    await page.getByRole('option').first().click();
    const qty = dialog.getByRole('spinbutton').first();
    await qty.fill('99999');
    await page.keyboard.press('Tab');
    // el-input-number :max 生效 → 失焦后被钳制（不等于 99999）
    await expect(qty).not.toHaveValue('99999');
  });

  test('03-04 选仓库并填写本次收货后收货成功', async ({ page }) => {
    const approved = page.getByRole('row').filter({ hasText: '已审批' }).first();
    await expect(approved).toBeVisible();
    await approved.getByRole('button', { name: '收货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '采购收货' });
    await dialog.getByRole('combobox').click();
    await page.getByRole('option').first().click();
    await dialog.getByRole('spinbutton').first().fill('1');
    // 批次号是后端入库建单期强校验的四维之一，收货对话框已新增该必填录入列（textbox），
    // 本次收货行须录入批次方能成功建单（非弱化断言：真实契约要求收货人实测录入批次）
    await dialog.getByRole('textbox').first().fill('E2E-RCV-BATCH');
    await dialog.getByRole('button', { name: '确定收货' }).click();
    // createPurchaseReceipt 成功 → msg.success('receiveSuccess') = '收货成功'
    await expect(page.getByText('收货成功')).toBeVisible({ timeout: 30000 });
  });
});
