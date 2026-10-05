// P9-4 采购 E2E 套件 — 02 采购订单审批
// 覆盖范围：草稿提交 / 审批通过 / 驳回（列表行内按钮驱动，非详情页按钮）
//
// 并行/种子隔离改造：
// 原三例各自用 `getByRole('row').filter({ hasText: '草稿'|'待审批' }).first()` 抓 globalSetup
// 共享的多状态行。同分片 fullyParallel 多 worker 下，多例抢同一行/流转掉该行后找不到的成片红。
// 改为：每例进入前用 API 造一张「本例专属、状态正确」的采购订单，并用后端生成的唯一 order_no
// 走列表筛选精确锚定该行后再操作，彻底脱离共享行。
//
// 真实 UI 事实（据 views/purchase/index.vue、components/PurchaseTable.vue、usePurchAct.ts、locales 核对）：
// - 采购为扁平单页 /purchase（无 /purchase/order/list、无 /purchase/order/detail/:id 路由），
//   订单动作为 PurchaseTable 行内按钮，不存在"进详情页点提交/审批"。
// - 行内按钮按状态渲染（purchase.table.*）：
//   DRAFT/REJECTED → '提交'(submit) / '编辑'；DRAFT → '删除'；APPROVED → '收货'(receive)；
//   PENDING_APPROVAL → '审批'(approve) / '驳回'(reject)。均有 '详情' 入口。
// - 状态标签中文（purchase.statusLabels，大写键）：DRAFT='草稿' PENDING_APPROVAL='待审批'
//   APPROVED='已审批' REJECTED='已驳回' PARTIAL_RECEIVED='部分收货' COMPLETED='已完成'。
// - usePurchAct：
//   handleSubmitOrder → ElMessageBox.confirm → msg.success('purchaseOrderSubmitted')=
//     '采购订单 {orderNo} 已提交'（message 键存在，可断言 '已提交'）；
//   handleApprove → ElMessageBox.confirm → msg.success('purchaseOrderApproved')=
//     '采购单 {orderNo} 审批成功'（可断言 '审批成功'）；
//   handleReject → ElMessageBox.prompt → rejectPurchaseOrder → msg.success('purchaseOrderRejected')=
//     '采购订单 {orderNo} 已驳回'（可断言 '已驳回'）。
// - 列表可按「关键词」输入框（placeholder='订单号/供应商名'）做后端 order_no LIKE 模糊下推，
//   唯一 order_no 使筛选结果收敛为本例那一行。

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

/** 后端 PurchaseOrderDto 中本套件用到的字段（create / GET /purchase/orders/{id} 均返回） */
interface PurchaseOrderLite {
  id: number;
  order_no: string;
  /** 后端 DTO order_status 字段经 serde(rename="status") 序列化，JSON 键为 status（全大写词表） */
  status: string;
}

/** 本 spec 内所有用例创建的专属订单 id，afterEach 尽力清理（DRAFT 可删；已流转的删除会告警，属预期） */
const CREATED_ORDER_IDS: number[] = [];
test.afterEach(async ({ page }) => {
  while (CREATED_ORDER_IDS.length) {
    const id = CREATED_ORDER_IDS.pop();
    if (id != null)
      await tryCleanup(page, 'DELETE', `/purchase/orders/${id}`, `purchase_order#${id}`);
  }
});

/**
 * 用 API 造一张本例专属采购订单并推进到目标状态，返回 {id, orderNo}。
 * 目标状态经真实后端状态机达成（非直接改库）：
 *   DRAFT             → 仅 POST /purchase/orders
 *   PENDING_APPROVAL  → POST + /submit
 *   APPROVED          → POST + /submit + /approve
 * 每一步后回查 GET /purchase/orders/{id} 的 status，不达预期立即抛错（禁止默认值兜底）。
 * 唯一标记：notes 带 genCode 生成的唯一码供追溯；定位用后端生成的唯一 order_no。
 * 状态词表为全大写（backend/src/models/status/purchase_inventory.rs），比较点与写入值逐字符一致。
 */
async function seedPurchaseOrder(
  page: Page,
  target: 'DRAFT' | 'PENDING_APPROVAL' | 'APPROVED'
): Promise<PurchaseOrderLite> {
  const ctx = getCtx();
  if (!ctx.supplierId) throw new Error(`前置缺失：ctx.supplierId 未就绪（${target}）`);
  if (!ctx.productIds[0]) throw new Error(`前置缺失：ctx.productIds[0] 未就绪（${target}）`);
  if (!ctx.warehouseIds[0]) throw new Error(`前置缺失：ctx.warehouseIds[0] 未就绪（${target}）`);
  if (!ctx.departmentIds[0]) throw new Error(`前置缺失：ctx.departmentIds[0] 未就绪（${target}）`);
  const marker = genCode('P2');

  const created = await apiCall<PurchaseOrderLite>(page, 'POST', '/purchase/orders', {
    supplier_id: ctx.supplierId,
    order_date: new Date().toISOString().slice(0, 10),
    warehouse_id: ctx.warehouseIds[0],
    department_id: ctx.departmentIds[0],
    // quantity_ordered / unit_price 后端为 Decimal，字符串入参避免 serde 类型不匹配
    items: [{ material_id: ctx.productIds[0], quantity_ordered: '20', unit_price: '15.00' }],
    notes: `E2E-P2-${marker}`,
  });
  const orderNo = created.data?.order_no;
  const id = created.data?.id;
  if (!id || !orderNo) {
    throw new Error(`前置失败：采购订单创建未返回 id/order_no：${JSON.stringify(created)}`);
  }
  CREATED_ORDER_IDS.push(id);

  let status = created.data.status;
  if (target !== 'DRAFT') {
    await apiCall(page, 'POST', `/purchase/orders/${id}/submit`);
    const afterSubmit = await apiCallRaw<PurchaseOrderLite>(page, 'GET', `/purchase/orders/${id}`);
    status = afterSubmit.status;
    if (status !== 'PENDING_APPROVAL') {
      throw new Error(`submit 后状态应为 PENDING_APPROVAL（实际 ${status}，id=${id}）`);
    }
  }
  if (target === 'APPROVED') {
    await apiCall(page, 'POST', `/purchase/orders/${id}/approve`);
    const afterApprove = await apiCallRaw<PurchaseOrderLite>(page, 'GET', `/purchase/orders/${id}`);
    status = afterApprove.status;
    if (status !== 'APPROVED') {
      throw new Error(`approve 后状态应为 APPROVED（实际 ${status}，id=${id}）`);
    }
  }
  return { id, order_no: orderNo, status };
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

test.describe('02 采购订单审批', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    // 补齐 ctx（供应商/仓库/部门/产品），供本例 API 造单引用
    await ensureTestEntities(page);
  });

  test('02-01 草稿采购订单行内可提交进入审批', async ({ page }) => {
    const { id, order_no: orderNo } = await seedPurchaseOrder(page, 'DRAFT');
    const row = await locateRowByOrderNo(page, orderNo);
    // 真实行内按钮 purchase.table.submit = '提交'
    await row.getByRole('button', { name: '提交', exact: true }).click();
    // ElMessageBox.confirm 默认确认按钮 '确定'
    await page.getByRole('button', { name: '确定', exact: true }).click();
    // 成功提示 message.purchaseOrderSubmitted = '采购订单 {orderNo} 已提交'
    // 「已提交」在本页同时出现于成功 toast 与筛选栏「订单状态」下拉的 SUBMITTED 选项
    // （getByText 命中隐藏 option → strict resolved to 2）。据真实 DOM 判定：把断言作用域到
    // 成功 toast 容器 .el-message--success（提交只产生这一条成功提示），而非 .first() 蒙混。
    await expect(page.locator('.el-message--success').filter({ hasText: '已提交' })).toBeVisible({
      timeout: 30000,
    });
    // 该单状态确实变了（双重真证据）：① 本例专属单行的状态标签真实变为「待审批」（PENDING_APPROVAL）；
    // ② 后端回查状态字面量 PENDING_APPROVAL。二者共同证明流转生效，未依赖被放宽的文案匹配。
    await expect(row.getByText('待审批')).toBeVisible({ timeout: 30000 });
    const after = await apiCallRaw<PurchaseOrderLite>(page, 'GET', `/purchase/orders/${id}`);
    expect(after.status, `提交后状态应为 PENDING_APPROVAL（实际 ${after.status}）`).toBe(
      'PENDING_APPROVAL'
    );
  });

  test('02-02 待审批采购订单行内可审批通过', async ({ page }) => {
    const { id, order_no: orderNo } = await seedPurchaseOrder(page, 'PENDING_APPROVAL');
    const row = await locateRowByOrderNo(page, orderNo);
    // purchase.table.approve = '审批'
    await row.getByRole('button', { name: '审批', exact: true }).click();
    await page.getByRole('button', { name: '确定', exact: true }).click();
    // approvePurchaseOrder 成功 → msg.success('purchaseOrderApproved') = '采购单 {orderNo} 审批成功'
    await expect(page.getByText('审批成功')).toBeVisible({ timeout: 30000 });
    // 后端真实状态字面量（大写词表）：审批后为 APPROVED
    const after = await apiCallRaw<PurchaseOrderLite>(page, 'GET', `/purchase/orders/${id}`);
    expect(after.status, `审批后状态应为 APPROVED（实际 ${after.status}）`).toBe('APPROVED');
  });

  test('02-03 待审批采购订单行内可驳回（原因必填）', async ({ page }) => {
    const { id, order_no: orderNo } = await seedPurchaseOrder(page, 'PENDING_APPROVAL');
    const row = await locateRowByOrderNo(page, orderNo);
    // purchase.table.reject = '驳回'，触发 ElMessageBox.prompt('请输入驳回原因')
    await row.getByRole('button', { name: '驳回', exact: true }).click();
    const msgBox = page.locator('.el-message-box');
    await expect(msgBox.getByText('请输入驳回原因')).toBeVisible();
    await msgBox.getByRole('textbox').fill('E2E 测试驳回：数量超预算');
    await msgBox.getByRole('button', { name: '确定', exact: true }).click();
    // rejectPurchaseOrder 成功 → msg.success('purchaseOrderRejected') = '采购订单 {orderNo} 已驳回'
    // 「已驳回」同时出现在成功 toast 与刷新后本行状态标签（REJECTED→'已驳回'）——两个不同来源，
    // getByText 命中 2。把 toast 断言作用域到 .el-message--success 容器（精确锚定成功提示来源）。
    await expect(page.locator('.el-message--success').filter({ hasText: '已驳回' })).toBeVisible({
      timeout: 30000,
    });
    // 该单状态确实变了（双重真证据）：① 专属单行状态标签真实变为「已驳回」；② 后端回查 REJECTED。
    await expect(row.getByText('已驳回')).toBeVisible({ timeout: 30000 });
    // 后端真实状态字面量（大写词表）：驳回后为 REJECTED
    const after = await apiCallRaw<{ status: string; rejected_reason: string | null }>(
      page,
      'GET',
      `/purchase/orders/${id}`
    );
    expect(after.status, `驳回后状态应为 REJECTED（实际 ${after.status}）`).toBe('REJECTED');
    // 理由必须落库可追溯（本轮止毁：reject 落 rejected_reason 专列，非只测状态=半级假绿）：逐字回读
    expect(after.rejected_reason, '驳回理由应逐字落 rejected_reason 专列').toBe(
      'E2E 测试驳回：数量超预算'
    );
  });
});
