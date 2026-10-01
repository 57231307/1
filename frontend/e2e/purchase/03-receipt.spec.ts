// P9-4 采购 E2E 套件 — 03 采购收货（入库）
// 覆盖范围：已审批采购订单行内「收货」→ PurchaseReceiveDialog 登记收货
//
// 并行/种子隔离改造：
// 原四例各自用 `getByRole('row').filter({ hasText: '已审批' }).first()` 抓 globalSetup
// 共享 APPROVED 行，并行被其他用例收货后行消失/状态变更导致找不到行。
// 改为：每例进入前用 API 造一张「本例专属 APPROVED 态」的采购订单，
// 用后端生成的唯一 order_no 走列表筛选精确锚定该行后再操作。
//
// 真实 UI 事实（据 PurchaseTable.vue / components/PurchaseReceiveDialog.vue / usePurchRcv.ts / locales 核对）：
// - 收货入口：/purchase 列表已审批行（状态 '已审批'，PURCHASE_ORDER_STATUS.APPROVED）行内
//   按钮 purchase.table.receive = '收货' → index.vue rcv.handleReceive → 打开 PurchaseReceiveDialog。
// - rcv.handleReceive 先回源 GET /purchase/orders/{id} 取明细再打开对话框：列表出参
//   PurchaseOrderDto（backend/src/services/po/order.rs:19）不含 items 键，明细只在详情 handler
//   里单查 purchase_order_item + LEFT JOIN products 挂到 order_json["items"]
//   （backend/src/handlers/purchase_order_handler.rs:100-127）。
// - PurchaseReceiveDialog（aria-label purchase.index.receiveDlgAriaLabel = '收货对话框'，标题 '采购收货'）：
//   只读采购单号/供应商；收货日期(默认今日 date)；仓库 el-select(label '仓库')；
//   明细 el-table 列 产品/订购数量/已收货/本次收货(el-input-number)/辅助数量(el-input-number)/
//   单价/批次号/备注；底部 '取消' / '确定收货'。
//   本次收货 el-input-number :max = 订购数量 - 已收货，超限输入被钳制。
// - 校验（submitReceive）：未选仓库 → msg.warning('pleaseSelectWarehouse') = '请选择收货仓库'；
//   全部本次收货为 0 → msg.warning('pleaseFillItem') = '请填写至少一项收货数量'；
//   批次号为空 → msg.warning('receiveBatchRequired') = '请为每行收货录入批次号'；
//   辅助数量未录入（创建契约必填键，显式 0 才是合法实收值）
//   → msg.warning('receiptItemAltQtyRequired') = '第 {line} 行未录入辅助数量…'。
// - 成功：createPurchaseReceipt → msg.success('receiveSuccess') = '收货成功'。

import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  tryCleanup,
  genCode,
} from '../flow/helpers';
import { pickSelectIn } from '../flow/ui-helpers';
import { pickListArray } from '../flow/ui-helpers';

/** 后端 PurchaseOrderDto 中本套件用到的字段 */
interface PurchaseOrderLite {
  id: number;
  order_no: string;
  status: string;
}

/**
 * PurchaseReceiptDto（services/purchase_receipt_dto.rs:16）中本用例用到的字段。
 * total_quantity/total_quantity_alt 是 DECIMAL，经 JSON 序列化为字符串（如 "5.0000"），
 * 断言前 Number() 归一。
 */
interface PurchaseReceiptLite {
  id: number;
  order_id: number | null;
  receipt_status: string;
  total_quantity: number | string;
  total_quantity_alt: number | string;
}

/** GET /purchase/receipts/{id}/items 明细行 = purchase_receipt_item::Model 原键（十进制列为字符串） */
interface ReceiptItemLite {
  id: number;
  batch_no: string | null;
  quantity: number | string;
  quantity_alt: number | string;
}

/** 本 spec 内所有用例创建的专属订单 id，afterEach 尽力清理（已流转的删除失败仅告警属预期） */
const CREATED_ORDER_IDS: number[] = [];
test.afterEach(async ({ page }) => {
  while (CREATED_ORDER_IDS.length) {
    const id = CREATED_ORDER_IDS.pop();
    if (id != null)
      await tryCleanup(page, 'DELETE', `/purchase/orders/${id}`, `purchase_order#${id}`);
  }
});

/**
 * 用 API 造一张本例专属的 APPROVED 态采购订单。
 * 创建 DRAFT → submit → PENDING_APPROVAL → approve → APPROVED。
 * 每步后回查 status，不达预期立即抛错（禁止默认值兜底）。
 * 唯一标记：notes 带 genCode 生成的唯一码供追溯；定位用后端生成的唯一 order_no。
 */
async function seedApprovedPO(page: Page): Promise<PurchaseOrderLite> {
  const ctx = getCtx();
  if (!ctx.supplierId) throw new Error('前置缺失：ctx.supplierId 未就绪（APPROVED）');
  if (!ctx.productIds[0]) throw new Error('前置缺失：ctx.productIds[0] 未就绪（APPROVED）');
  if (!ctx.warehouseIds[0]) throw new Error('前置缺失：ctx.warehouseIds[0] 未就绪（APPROVED）');
  if (!ctx.departmentIds[0]) throw new Error('前置缺失：ctx.departmentIds[0] 未就绪（APPROVED）');
  const marker = genCode('P3');

  const created = await apiCall<PurchaseOrderLite>(page, 'POST', '/purchase/orders', {
    supplier_id: ctx.supplierId,
    order_date: new Date().toISOString().slice(0, 10),
    warehouse_id: ctx.warehouseIds[0],
    department_id: ctx.departmentIds[0],
    items: [{ material_id: ctx.productIds[0], quantity_ordered: '20', unit_price: '15.00' }],
    notes: `E2E-P3-${marker}`,
  });
  const orderNo = created.data?.order_no;
  const id = created.data?.id;
  if (!id || !orderNo) {
    throw new Error(`前置失败：采购订单创建未返回 id/order_no：${JSON.stringify(created)}`);
  }
  CREATED_ORDER_IDS.push(id);

  // submit: DRAFT → PENDING_APPROVAL
  await apiCall(page, 'POST', `/purchase/orders/${id}/submit`);
  const afterSubmit = await apiCallRaw<PurchaseOrderLite>(page, 'GET', `/purchase/orders/${id}`);
  if (afterSubmit.status !== 'PENDING_APPROVAL') {
    throw new Error(`submit 后状态应为 PENDING_APPROVAL（实际 ${afterSubmit.status}，id=${id}）`);
  }

  // approve: PENDING_APPROVAL → APPROVED
  await apiCall(page, 'POST', `/purchase/orders/${id}/approve`);
  const afterApprove = await apiCallRaw<PurchaseOrderLite>(page, 'GET', `/purchase/orders/${id}`);
  if (afterApprove.status !== 'APPROVED') {
    throw new Error(`approve 后状态应为 APPROVED（实际 ${afterApprove.status}，id=${id}）`);
  }

  return { id, order_no: orderNo, status: 'APPROVED' };
}

/**
 * 用「关键词」筛选（placeholder='订单号/供应商名'，后端 order_no LIKE 下推）
 * 把列表收敛到本例那一行，返回该行定位器。
 */
async function locateRowByOrderNo(page: Page, orderNo: string) {
  await page.goto('/purchase');
  await page.getByPlaceholder('订单号/供应商名').fill(orderNo);
  await page.getByRole('button', { name: '查询', exact: true }).click();
  const row = page.getByRole('row').filter({ hasText: orderNo });
  await expect(row, `按订单号 ${orderNo} 应筛选出本例专属行`).toHaveCount(1, {
    timeout: 30000,
  });
  return row;
}

test.describe('03 采购收货', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    // 补齐 ctx（供应商/仓库/部门/产品），供本例 API 造单引用
    await ensureTestEntities(page);
  });

  test('03-01 已审批采购订单行内「收货」打开收货对话框', async ({ page }) => {
    const { order_no: orderNo } = await seedApprovedPO(page);
    const row = await locateRowByOrderNo(page, orderNo);
    // 真实行内按钮 purchase.table.receive = '收货'
    await row.getByRole('button', { name: '收货', exact: true }).click();
    // PurchaseReceiveDialog 标题 '采购收货'（purchase.receiveDlg.title）
    const dialog = page.getByRole('dialog', { name: '采购收货' });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole('button', { name: '确定收货' })).toBeVisible();
  });

  test('03-02 未选仓库直接确定收货被拦截', async ({ page }) => {
    const { order_no: orderNo } = await seedApprovedPO(page);
    const row = await locateRowByOrderNo(page, orderNo);
    await row.getByRole('button', { name: '收货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '采购收货' });
    await expect(dialog).toBeVisible();
    // 不选仓库，先填数量以绕过"请填写至少一项收货数量"校验，再直接提交
    await dialog.getByRole('spinbutton').first().fill('1');
    // 批次号也是必填，填上绕过该校验；辅量留空不影响本例——仓库校验先于辅量拦截
    await dialog.locator('input[placeholder="收货批次号"]').first().fill('BATCH-NO-WH');
    await dialog.getByRole('button', { name: '确定收货' }).click();
    // 真实校验 message.pleaseSelectWarehouse = '请选择收货仓库'
    await expect(page.getByText('请选择收货仓库')).toBeVisible();
    await expect(page.locator('.el-message--success')).toHaveCount(0);
  });

  test('03-03 本次收货数量受订购可收数量钳制', async ({ page }) => {
    const { order_no: orderNo } = await seedApprovedPO(page);
    const row = await locateRowByOrderNo(page, orderNo);
    await row.getByRole('button', { name: '收货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '采购收货' });
    await expect(dialog).toBeVisible();
    const qty = dialog.getByRole('spinbutton').first();
    // quantity_ordered='20'，received_quantity=0 → :max=20
    await qty.fill('99999');
    await page.keyboard.press('Tab');
    // el-input-number :max 生效 → 失焦后被钳制（不等于 99999）
    await expect(qty).not.toHaveValue('99999');
  });

  test('03-04 选仓库并填写本次收货后收货成功', async ({ page }) => {
    const { id, order_no: orderNo } = await seedApprovedPO(page);
    const row = await locateRowByOrderNo(page, orderNo);
    await row.getByRole('button', { name: '收货', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '采购收货' });
    await expect(dialog).toBeVisible();
    // 选仓库（el-select，label='仓库'）
    await pickSelectIn(dialog, page, '仓库');
    // 填收货数量
    await dialog.getByRole('spinbutton').first().fill('5');
    // 辅助数量（创建契约必填，placeholder='请输入辅助数量，无辅量填0'）：实录 10
    await dialog.locator('input[placeholder="请输入辅助数量，无辅量填0"]').first().fill('10');
    // 批次号：placeholder='收货批次号'
    const batchNo = `E2E-${genCode('RCV')}`;
    await dialog.locator('input[placeholder="收货批次号"]').first().fill(batchNo);
    await dialog.getByRole('button', { name: '确定收货' }).click();
    // createPurchaseReceipt 成功 → msg.success('receiveSuccess') = '收货成功'
    await expect(page.getByText('收货成功')).toBeVisible({ timeout: 30000 });

    // 落库核对：收货登记建的是 DRAFT 入库单（purchase_receipt_service.rs:75 build_receipt_active_model
    // 写 status::purchase_receipt::DRAFT），并携带对话框录入的数量与批次号。
    const receiptList = pickListArray<PurchaseReceiptLite>(
      await apiCallRaw<unknown>(
        page,
        'GET',
        `/purchase/receipts?order_id=${id}&page=1&page_size=10`
      ),
      'items',
      '按单收货后的入库单列表'
    );
    const mine = receiptList.find(r => r.order_id === id);
    expect(
      mine,
      `订单 ${orderNo} 应按单生成入库单，实际列表=${JSON.stringify(receiptList)}`
    ).toBeTruthy();

    const receipt = mine!;
    expect(receipt.receipt_status, '入库单初始状态应为写入方原值 DRAFT').toBe('DRAFT');
    expect(Number(receipt.total_quantity), '入库单主表数量应等于本次收货 5').toBe(5);
    expect(Number(receipt.total_quantity_alt), '入库单辅量合计应等于对话框实录 10').toBe(10);

    // 明细回源：GET /purchase/receipts/{id}/items 的 data 是裸数组
    // （handlers/purchase_receipt_handler.rs list_receipt_items）
    const lines = pickListArray<ReceiptItemLite>(
      await apiCallRaw<unknown>(page, 'GET', `/purchase/receipts/${receipt.id}/items`),
      'bare',
      '入库单明细'
    );
    const line = lines.find(l => l.batch_no === batchNo);
    expect(line, `入库明细应带回收货时录入的批次号 ${batchNo}`).toBeTruthy();
    expect(Number(line!.quantity), '入库明细数量应等于本次收货 5').toBe(5);
    expect(Number(line!.quantity_alt), '入库明细辅量应等于对话框实录 10').toBe(10);

    // 采购订单进度与库存/应付都在「确认入库」里同事务落账
    // （purchase_receipt_ops/state.rs:43-49 update_order_received_quantity + update_inventory_txn），
    // 收货登记本身不推进订单状态，故此刻仍应是 APPROVED。
    // 原用例断言「收货后状态离开 APPROVED」把两步合成了一个动作，与写入方语义不符（据实修正）。
    const after = await apiCallRaw<PurchaseOrderLite>(page, 'GET', `/purchase/orders/${id}`);
    expect(
      after.status,
      `收货登记只建 DRAFT 入库单，采购订单应仍为 APPROVED（实际 ${after.status}）`
    ).toBe('APPROVED');
  });
});
