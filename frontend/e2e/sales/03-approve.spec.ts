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
//   handleReject → useActionPrompts.promptRejectReason()（ElMessageBox.prompt，
//   message=actionForm.rejectReasonTip='请填写审批拒绝理由（必填）'，locales/zh-CN.ts:5801）
//   → rejectSalesOrder → msg.success('rejectSuccess')。
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
 * 用「订单号」筛选把列表收敛到本例那一行，并断言筛选结果确为本例专属单（唯一 order_no → 1 行）。
 * 筛选是唯一能跨分页、跨并发稳定锚定自造单的手段：全局种子行会被并发流转/翻页挤掉。
 *
 * 行内操作按钮的定位（V2Table 真实 DOM，非用例臆测）：
 * 销售订单列表是 V2Table（el-table-v2 虚拟滚动）。操作列声明为 fixed:'right'，
 * EP TableV2 会把固定列渲染到独立的 overlay Grid（另一组 role="row" 节点），
 * 与承载「订单号」单元格的主 Grid 行不是同一个 DOM row（EP row.mjs：一个 role="row" 只含本 Grid 的列）。
 * 因此 `row.getByRole('button')` 命中 0 个 → click 30s 超时（既往假红根因）。
 * 正确定位（对齐 production/01、system/02 既有 V2Table 范式）：先按唯一订单号把列表筛到 1 行，
 * 再在页面/表格作用域内按精确按钮名定位——筛选后仅存这一行，动作按钮名在整页唯一。
 */
async function locateRowByOrderNo(page: Page, orderNo: string) {
  await page.goto('/sales');
  await page.getByPlaceholder('订单号').fill(orderNo);
  await page.getByRole('button', { name: '查询', exact: true }).click();
  const row = page.getByRole('row').filter({ hasText: orderNo });
  await expect(row, `按订单号 ${orderNo} 应筛选出本例专属行`).toHaveCount(1, {
    timeout: 30000,
  });
}

/** 筛选到本例唯一行后，在页面作用域按精确名取行内操作按钮（V2Table 固定列与主行分属不同 DOM row） */
function orderActionBtn(page: Page, name: string) {
  return page.getByRole('button', { name, exact: true }).first();
}

test.describe('03 销售订单审批', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    // 补齐 ctx（客户/产品/用户/BPM 审批定义），供本例 API 造单引用
    await ensureTestEntities(page);
  });

  test('03-01 草稿订单行内可提交进入审批', async ({ page }) => {
    const { id, order_no: orderNo } = await seedSalesOrder(page, 'draft');
    await locateRowByOrderNo(page, orderNo);
    // 真实行内按钮 sales.table.submit = '提交'
    await orderActionBtn(page, '提交').click();
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
    await locateRowByOrderNo(page, orderNo);
    // sales.table.approve = '审批'
    await orderActionBtn(page, '审批').click();
    await page.getByRole('button', { name: '确定', exact: true }).click();
    // approveSalesOrder 成功 → msg.success('approveSuccess') = '审批成功'
    await expect(page.getByText('审批成功')).toBeVisible({ timeout: 30000 });
    // 后端真实状态字面量：审批后为 approved
    const after = await apiCallRaw<SalesOrderLite>(page, 'GET', `/sales/orders/${id}`);
    expect(after.status, `审批后状态应为 approved（实际 ${after.status}）`).toBe('approved');
  });

  test('03-03 待审批订单行内可驳回（原因必填）', async ({ page }) => {
    const { id, order_no: orderNo } = await seedSalesOrder(page, 'pending');
    await locateRowByOrderNo(page, orderNo);
    // 双列锁前置基线：先回读建单 notes（seed 写入的唯一码），reject 后必须逐字不变
    //（本轮止毁缺陷回归：销售订单 reject 曾挪用覆写 notes；现 reject 落 rejected_reason 专列，notes 回归备注语义）
    const before = await apiCallRaw<{ notes: string | null }>(page, 'GET', `/sales/orders/${id}`);
    expect(before.notes, '建单 notes 基线应已落库（seed 唯一码）').toBeTruthy();
    // sales.table.reject = '驳回'，触发统一采集器 useActionPrompts.promptRejectReason()
    //（composables/useActionPrompts.ts:68-87：ElMessageBox.prompt，
    //  message=actionForm.rejectReasonTip、title=actionForm.rejectReasonTitle、
    //  inputPlaceholder=actionForm.rejectReasonPlaceholder，zh 值见 locales/zh-CN.ts:5800-5802）
    await orderActionBtn(page, '驳回').click();
    const msgBox = page.locator('.el-message-box');
    // 弹窗文案断言=同步实现真值（UI 文案允许断言；脱敏红线只禁断后端错误 message）。
    // tip/标题/输入占位三源齐断，禁止退化成"只断弹窗出现"的弱断言。
    await expect(msgBox.getByText('请填写审批拒绝理由（必填）')).toBeVisible();
    await expect(msgBox.getByText('审批拒绝理由', { exact: true })).toBeVisible();
    await expect(msgBox.getByPlaceholder('请输入审批拒绝理由', { exact: true })).toBeVisible();
    await msgBox.getByRole('textbox').fill('E2E 测试驳回：价格不符合规范');
    await msgBox.getByRole('button', { name: '确定', exact: true }).click();
    // rejectSalesOrder 成功后 refresh + msg.success(...)：因 message.rejectSuccess 缺键，
    // 断言成功提示元素出现（仅成功分支渲染），不依赖缺失的中文文案
    await expect(page.locator('.el-message--success')).toBeVisible({ timeout: 30000 });
    // 后端真实状态字面量：驳回后为 rejected
    const after = await apiCallRaw<{
      status: string;
      rejected_reason: string | null;
      notes: string | null;
    }>(page, 'GET', `/sales/orders/${id}`);
    expect(after.status, `驳回后状态应为 rejected（实际 ${after.status}）`).toBe('rejected');
    // 理由必须经接口逐字回读 rejected_reason 专列（半级假绿防线：不止断状态）。⚠️ 已知出参缺口
    //（与报价单 F5 同族）：GET /sales/orders/{id} 出参 SalesOrderDetail
    //（backend/src/services/so/mod.rs:43-83）尚无 rejected_reason 键——断言保持严格逐字回读
    // 不放宽，后端补出参前如实判红，禁止改成断键缺失/undefined 蒙绿。
    expect(after.rejected_reason, '驳回理由应逐字落 rejected_reason 专列').toBe(
      'E2E 测试驳回：价格不符合规范'
    );
    // 双列锁：reject 不得覆写 notes——另一列必须"没被写"（止毁回归的行为级锁）
    expect(after.notes, 'reject 不得覆写 notes（应逐字保持建单基线）').toBe(before.notes);
  });
});
