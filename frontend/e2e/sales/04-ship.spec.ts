// P9-3 销售 E2E 套件 — 04 销售发货
// 覆盖范围：已审批订单行内「发货」→ DeliveryDialog 出库（真实对话框，非详情页按钮）
//
// 并行/种子隔离改造（PR #941 run #4656 红簇取证）：
// 原四例都从 /sales 列表抓 `.first()` 的「已审批」行。同分片 fullyParallel 多 worker 下四例抢
// 同一张共享 approved 行，且 04-04 真正发货后该行状态流转（approved→partially_shipped/shipped），
// 其余例就找不到 approved 行 → 成片红。属真 seed/并行串扰，补选择器解决不了。
// 改为：每例进入前用 API 造一张「本例专属、approved 态」的销售订单，用后端唯一 order_no 精确筛选
// 锚定该行后再操作，彻底脱离共享行。04-04 额外确保所选仓库存在该产品的四维库存行，保证出库可成。
//
// 真实 UI 事实（据 SalesOrderTable.vue / DeliveryDialog.vue / useOlv.ts / locales 核对）：
// - 发货入口：/sales 列表已审批行（状态标签 '已审批'）行内按钮 sales.table.deliver = '发货'
//   → OrderListView.onDelivery → olv.prepareDelivery → 打开 DeliveryDialog。
//   不存在"详情页创建发货单/保存按钮/发货单号 DN-"等 UI。
// - DeliveryDialog（aria-label sales.delivery.dialogAriaLabel='销售发货对话框'，标题 '销售发货'）真实控件：
//   只读销售单号/客户；发货日期 date picker 占位 '选择日期'；仓库 el-select（label '仓库'）；
//   明细 el-table（aria-label '销售发货明细表'）列 产品/库存行/缸号/色号/批次/订单数量/已发货/
//   本次发货(el-input-number)/单价/备注；底部 '取消' / '确定发货'。
//   出库需先选仓库加载库存行，再按四维(色号/批次/缸号)选库存行；本次发货 el-input-number
//   的 :max = 订单数量 - 已发货（DeliveryDialog.vue 第 103 行），超限输入会被钳制。
// - 校验（handleSubmit）：未选仓库 → ElMessage.warning('请选择仓库')；未填发货日期 → '请选择发货日期'。
// - 成功：handleDeliverySubmit → shipSalesOrder → msg.success('shipSuccess') = '发货成功'。

import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelect, pickSelectIn } from '../flow/ui-helpers';
import {
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  tryCleanup,
  seedFourDimStockIn,
  genCode,
} from '../flow/helpers';

/** 后端 SalesOrderDetail 中本套件用到的字段 */
interface SalesOrderLite {
  id: number;
  order_no: string;
  status: string;
}

const CREATED_ORDER_IDS: number[] = [];
test.afterEach(async ({ page }) => {
  while (CREATED_ORDER_IDS.length) {
    const id = CREATED_ORDER_IDS.pop();
    if (id != null) await tryCleanup(page, 'DELETE', `/sales/orders/${id}`, `sales_order#${id}`);
  }
});

/**
 * 用 API 造一张本例专属、approved 态的销售订单（POST → /submit → /approve），回查确认落态，
 * 返回 {id, orderNo}。每步后以真实 status 字面量断言，不达预期立即抛错（禁止默认值兜底）。
 * 订单产品取 ctx.productIds[0]（ensureTestEntities 已为其备好可出库四维库存）。
 */
async function seedApprovedOrder(page: Page): Promise<SalesOrderLite> {
  const ctx = getCtx();
  if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
  if (!ctx.productIds[0]) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
  const marker = genCode('S4');

  const created = await apiCall<SalesOrderLite>(page, 'POST', '/sales/orders', {
    customer_id: ctx.customerId,
    order_date: new Date().toISOString(),
    items: [{ product_id: ctx.productIds[0], quantity: '10', unit_price: '25.00' }],
    notes: `E2E-S4-${marker}`,
  });
  const id = created.data?.id;
  const orderNo = created.data?.order_no;
  if (!id || !orderNo) {
    throw new Error(`前置失败：销售订单创建未返回 id/order_no：${JSON.stringify(created)}`);
  }
  CREATED_ORDER_IDS.push(id);

  await apiCall(page, 'POST', `/sales/orders/${id}/submit`);
  const afterSubmit = await apiCallRaw<SalesOrderLite>(page, 'GET', `/sales/orders/${id}`);
  if (afterSubmit.status !== 'pending') {
    throw new Error(`submit 后状态应为 pending（实际 ${afterSubmit.status}，id=${id}）`);
  }
  await apiCall(page, 'POST', `/sales/orders/${id}/approve`);
  const afterApprove = await apiCallRaw<SalesOrderLite>(page, 'GET', `/sales/orders/${id}`);
  if (afterApprove.status !== 'approved') {
    throw new Error(`approve 后状态应为 approved（实际 ${afterApprove.status}，id=${id}）`);
  }
  return { id, order_no: orderNo, status: afterApprove.status };
}

/** 用「订单号」筛选把列表收敛到本例那一行（跨分页/并发唯一稳定锚点），返回该行定位器 */
async function locateRowByOrderNo(page: Page, orderNo: string) {
  await page.goto('/sales');
  await page.getByPlaceholder('订单号').fill(orderNo);
  await page.getByRole('button', { name: '查询', exact: true }).click();
  const row = page.getByRole('row').filter({ hasText: orderNo });
  await expect(row, `按订单号 ${orderNo} 应筛选出本例专属行`).toHaveCount(1, { timeout: 30000 });
  return row;
}

test.describe('04 销售发货', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await ensureTestEntities(page);
  });

  test('04-01 已审批订单行内「发货」打开发货对话框', async ({ page }) => {
    const { order_no: orderNo } = await seedApprovedOrder(page);
    const row = await locateRowByOrderNo(page, orderNo);
    await row.getByRole('button', { name: '发货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '销售发货' });
    await expect(dialog).toBeVisible();
    // 底部真实按钮 sales.delivery.confirmDelivery = '确定发货'
    await expect(dialog.getByRole('button', { name: '确定发货' })).toBeVisible();
  });

  test('04-02 未选仓库直接确定发货被拦截', async ({ page }) => {
    const { order_no: orderNo } = await seedApprovedOrder(page);
    const row = await locateRowByOrderNo(page, orderNo);
    await row.getByRole('button', { name: '发货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '销售发货' });
    await expect(dialog).toBeVisible();
    // 未选仓库即点确定发货
    await dialog.getByRole('button', { name: '确定发货' }).click();
    // 真实校验 sales.delivery.warehouseRequired = '请选择仓库'
    await expect(page.getByText('请选择仓库')).toBeVisible();
    await expect(page.locator('.el-message--success')).toHaveCount(0);
  });

  test('04-03 本次发货数量受订单可发数量钳制', async ({ page }) => {
    const { order_no: orderNo } = await seedApprovedOrder(page);
    const row = await locateRowByOrderNo(page, orderNo);
    await row.getByRole('button', { name: '发货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '销售发货' });
    // 先选仓库以启用库存行与本次发货上限计算
    await pickSelectIn(dialog, page, '仓库');
    const qty = dialog.getByRole('spinbutton').first();
    // 输入远超订单数量（订单量 10）的值
    await qty.fill('99999');
    await page.keyboard.press('Tab');
    // el-input-number :max 生效：失焦后实际值被钳制到可发上限（≠ 99999）
    await expect(qty).not.toHaveValue('99999');
  });

  test('04-04 填写发货维度并确定发货成功后给出成功提示', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    if (!productId) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
    if (!ctx.warehouseIds.length) throw new Error('前置缺失：ctx.warehouseIds 为空');
    // 发货对话框「仓库」下拉的首项与 ensureTestEntities 读到的 ctx.warehouseIds 同源同序
    // （均 GET /warehouses 默认分页），但为消除 pickSelect「取首项」落到无库存仓的不确定性，
    // 对本例可见的每个仓库都灌一行该产品的完整四维库存（seedFourDimStockIn 幂等、可精确回查），
    // 无论下拉选到哪个仓库都能命中可出库库存行。
    const colorNo = ctx.colorNos[0] || 'E2E-C001';
    for (let i = 0; i < ctx.warehouseIds.length; i++) {
      const tag = genCode('S4STK');
      await seedFourDimStockIn(page, {
        productId,
        warehouseId: ctx.warehouseIds[i],
        colorNo,
        dyeLotNo: `E2E-DL-${tag}-${i}`,
        batchNo: `E2E-BN-${tag}-${i}`,
        quantityMeters: '10000',
      });
    }

    const { id, order_no: orderNo } = await seedApprovedOrder(page);
    const row = await locateRowByOrderNo(page, orderNo);
    await row.getByRole('button', { name: '发货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '销售发货' });
    // 仓库
    await pickSelectIn(dialog, page, '仓库');
    // 发货日期
    await dialog.getByPlaceholder('选择日期').fill('2026-12-31');
    await page.keyboard.press('Enter');
    // 库存行（仓库之后第 2 个 el-select，仓库选定后启用），选第一条四维库存行
    await pickSelect(page, dialog.locator('.el-select').nth(1));
    // 本次发货数量
    await dialog.getByRole('spinbutton').first().fill('1');
    await dialog.getByRole('button', { name: '确定发货' }).click();
    // 真实出库端点成功 → msg.success('shipSuccess') = '发货成功'
    await expect(page.getByText('发货成功')).toBeVisible({ timeout: 30000 });
    // 后端真实流转：成功出库后订单应离开纯 approved（部分/整单发货态），
    // 只断言"状态已流转"这一确定事实，不写死具体 token（partial/shipped 词表随版本可能调整）
    const after = await apiCallRaw<SalesOrderLite>(page, 'GET', `/sales/orders/${id}`);
    expect(after.status, `发货后状态应离开 approved（实际 ${after.status}，id=${id}）`).not.toBe(
      'approved'
    );
  });
});
