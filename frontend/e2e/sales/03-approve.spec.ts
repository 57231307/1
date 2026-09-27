// P9-3 销售 E2E 套件 — 03 销售订单审批
// 覆盖范围：销售订单提交/审批通过/驳回（列表行内按钮驱动，非详情页按钮）
//
// 并行/种子隔离改造（PR #941 run #4656 红簇取证）：
// 原三例各自用 `getByRole('row').filter({ hasText: '草稿'|'待审批' }).first()` 抓 globalSetup
// 共享的多状态行。同分片 fullyParallel 多 worker 下：① 多例抢同一行；② 本批用例的提交/审批/驳回
// 会把该行流转掉（draft→pending→approved/rejected），后进入的用例就找不到目标状态行 → 成片红。
// 这属真 seed/并行串扰，补选择器解决不了。改为：每例进入前用 API 造一张「本例专属、状态正确」
// 的销售订单，并用后端生成的唯一 order_no 走列表筛选精确锚定该行后再操作，彻底脱离共享行。
//
// 真实 UI 事实（据 SalesOrderTable.vue / useOlvProc.ts / locales 核对）：
// - 销售为扁平单页 /sales，订单动作全部是列表行内按钮（SalesOrderTable.vue renderCell），
//   不存在"进入详情页再点动作按钮"的入口（OrderDetail.vue 仅描述+明细，无动作按钮）。
// - 行内按钮按状态条件渲染：
//   draft → '提交'(submit) / '删除'；pending → '审批'(approve) / '驳回'(reject)；approved → '发货'。
//   状态标签中文（sales.statusLabels）：draft='草稿' pending='待审批' approved='已审批' rejected='已驳回'。
// - useOlvProc：
//   handleSubmitOrder → ElMessageBox.confirm('确定提交此订单进入审批流程吗？') → msg.success('submitSuccess')='提交成功'；
//   handleApprove → ElMessageBox.confirm('确定审批此订单吗？') → msg.success('approveSuccess')='审批成功'；
//   handleReject → ElMessageBox.prompt('请输入驳回原因') → rejectSalesOrder → msg.success('rejectSuccess')。
//   注：message.rejectSuccess 键在 locales 缺失（真实产品 i18n 缺口），成功 toast 文案不可依赖，
//   故驳回用例改断言成功提示元素 .el-message--success 出现（仅在成功分支产生），不放宽为"无断言"。
// - 列表可按「订单号」子串精确筛选（SalesOrderFilter order_no 输入 → 后端 order_no LIKE 下推，
//   order_query.rs:164 contains），唯一 order_no 使筛选结果收敛为本例那一行。

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

/** 后端 SalesOrderDetail 中本套件用到的字段（create / GET /orders/{id} 均返回） */
interface SalesOrderLite {
  id: number;
  order_no: string;
  status: string;
}

/** 本 spec 内所有用例创建的专属订单 id，afterEach 尽力清理（draft 可删；已流转的删除会告警，属预期） */
const CREATED_ORDER_IDS: number[] = [];
test.afterEach(async ({ page }) => {
  while (CREATED_ORDER_IDS.length) {
    const id = CREATED_ORDER_IDS.pop();
    if (id != null) await tryCleanup(page, 'DELETE', `/sales/orders/${id}`, `sales_order#${id}`);
  }
});

/**
 * 用 API 造一张本例专属销售订单并推进到目标状态，返回 {id, orderNo}。
 * 目标状态经真实后端状态机达成（非直接改库）：
 *   draft    → 仅 POST /sales/orders
 *   pending  → POST + /submit（后端 start_bpm_process 挂 sales_order_approval 的 user_task，
 *              停在 pending，等待审批；ensureTestEntities 已幂等建该 BPM 定义）
 *   approved → POST + /submit + /approve
 * 每一步后回查 GET /sales/orders/{id} 的 status，不达预期立即抛错（禁止默认值兜底）。
 * 唯一标记：notes 带 genCode 生成的唯一码供追溯；定位用后端生成的唯一 order_no。
 */
async function seedSalesOrder(
  page: Page,
  target: 'draft' | 'pending' | 'approved'
): Promise<SalesOrderLite> {
  const ctx = getCtx();
  if (!ctx.customerId) throw new Error(`前置缺失：ctx.customerId 未就绪（${target}）`);
  if (!ctx.productIds[0]) throw new Error(`前置缺失：ctx.productIds[0] 未就绪（${target}）`);
  const marker = genCode('S3');

  const created = await apiCall<SalesOrderLite>(page, 'POST', '/sales/orders', {
    customer_id: ctx.customerId,
    order_date: new Date().toISOString(),
    // quantity/unit_price 后端为 Decimal，字符串入参避免 serde 类型不匹配
    items: [{ product_id: ctx.productIds[0], quantity: '10', unit_price: '25.00' }],
    notes: `E2E-S3-${marker}`,
  });
  const orderNo = created.data?.order_no;
  const id = created.data?.id;
  if (!id || !orderNo) {
    throw new Error(`前置失败：销售订单创建未返回 id/order_no：${JSON.stringify(created)}`);
  }
  CREATED_ORDER_IDS.push(id);

  let status = created.data.status;
  if (target !== 'draft') {
    await apiCall(page, 'POST', `/sales/orders/${id}/submit`);
    const afterSubmit = await apiCallRaw<SalesOrderLite>(page, 'GET', `/sales/orders/${id}`);
    status = afterSubmit.status;
    if (status !== 'pending') {
      throw new Error(`submit 后状态应为 pending（实际 ${status}，id=${id}）`);
    }
  }
  if (target === 'approved') {
    await apiCall(page, 'POST', `/sales/orders/${id}/approve`);
    const afterApprove = await apiCallRaw<SalesOrderLite>(page, 'GET', `/sales/orders/${id}`);
    status = afterApprove.status;
    if (status !== 'approved') {
      throw new Error(`approve 后状态应为 approved（实际 ${status}，id=${id}）`);
    }
  }
  return { id, order_no: orderNo, status };
}

/**
 * 用「订单号」筛选把列表收敛到本例那一行，返回该行定位器。
 * 筛选是唯一能跨分页、跨并发稳定锚定自造单的手段：全局种子行会被并发流转/翻页挤掉。
 */
async function locateRowByOrderNo(page: Page, orderNo: string) {
  await page.goto('/sales');
  await page.getByPlaceholder('订单号').fill(orderNo);
  await page.getByRole('button', { name: '查询', exact: true }).click();
  const row = page.getByRole('row').filter({ hasText: orderNo });
  await expect(row, `按订单号 ${orderNo} 应筛选出本例专属行`).toHaveCount(1, {
    timeout: 30000,
  });
  return row;
}

test.describe('03 销售订单审批', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    // 补齐 ctx（客户/产品/用户/BPM 审批定义），供本例 API 造单引用
    await ensureTestEntities(page);
  });

  test('03-01 草稿订单行内可提交进入审批', async ({ page }) => {
    const { id, order_no: orderNo } = await seedSalesOrder(page, 'draft');
    const row = await locateRowByOrderNo(page, orderNo);
    // 真实行内按钮 sales.table.submit = '提交'
    await row.getByRole('button', { name: '提交', exact: true }).click();
    // ElMessageBox.confirm 默认确认按钮 '确定'
    await page.getByRole('button', { name: '确定', exact: true }).click();
    // 成功提示 message.submitSuccess = '提交成功'
    await expect(page.getByText('提交成功')).toBeVisible({ timeout: 30000 });
    // 后端真实流转：提交后应离开 draft
    const after = await apiCallRaw<SalesOrderLite>(page, 'GET', `/sales/orders/${id}`);
    expect(after.status, `提交后状态应离开 draft（实际 ${after.status}）`).not.toBe('draft');
  });

  test('03-02 待审批订单行内可审批通过', async ({ page }) => {
    const { id, order_no: orderNo } = await seedSalesOrder(page, 'pending');
    const row = await locateRowByOrderNo(page, orderNo);
    // sales.table.approve = '审批'
    await row.getByRole('button', { name: '审批', exact: true }).click();
    await page.getByRole('button', { name: '确定', exact: true }).click();
    // approveSalesOrder 成功 → msg.success('approveSuccess') = '审批成功'
    await expect(page.getByText('审批成功')).toBeVisible({ timeout: 30000 });
    // 后端真实状态字面量：审批后为 approved
    const after = await apiCallRaw<SalesOrderLite>(page, 'GET', `/sales/orders/${id}`);
    expect(after.status, `审批后状态应为 approved（实际 ${after.status}）`).toBe('approved');
  });

  test('03-03 待审批订单行内可驳回（原因必填）', async ({ page }) => {
    const { id, order_no: orderNo } = await seedSalesOrder(page, 'pending');
    const row = await locateRowByOrderNo(page, orderNo);
    // sales.table.reject = '驳回'，触发 ElMessageBox.prompt('请输入驳回原因')
    await row.getByRole('button', { name: '驳回', exact: true }).click();
    const msgBox = page.locator('.el-message-box');
    await expect(msgBox.getByText('请输入驳回原因')).toBeVisible();
    await msgBox.getByRole('textbox').fill('E2E 测试驳回：价格不符合规范');
    await msgBox.getByRole('button', { name: '确定', exact: true }).click();
    // rejectSalesOrder 成功后 refresh + msg.success(...)：因 message.rejectSuccess 缺键，
    // 断言成功提示元素出现（仅成功分支渲染），不依赖缺失的中文文案
    await expect(page.locator('.el-message--success')).toBeVisible({ timeout: 30000 });
    // 后端真实状态字面量：驳回后为 rejected
    const after = await apiCallRaw<SalesOrderLite>(page, 'GET', `/sales/orders/${id}`);
    expect(after.status, `驳回后状态应为 rejected（实际 ${after.status}）`).toBe('rejected');
  });
});
