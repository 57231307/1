// 生产计划 E2E 套件 — 01 生产工单创建与排产（draft → approved → 排产）
// 覆盖范围：新建生产订单、草稿编辑、已审批工单计划排产（APPROVED → 已排产）
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, ensureTestEntities, getCtx } from '../flow/helpers';
import { pickSelect, elSelectByLabel } from '../flow/ui-helpers';

/**
 * 按状态筛选生产订单列表。
 * 用于把目标状态行集中呈现，确保"造出的工单"对应的状态操作按钮一定在屏内。
 * （order_no 缺陷已修复，本 helper 仅服务状态维度；按单号定位用 filterByOrderNo。）
 */
async function filterByStatus(page: Page, statusLabel: string): Promise<void> {
  await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  await pickSelect(page, elSelectByLabel(page, /状态/), new RegExp(`^${statusLabel}$`));
  await page.getByRole('button', { name: /查询/ }).click();
}

/**
 * 按「订单编号」筛选生产订单列表。
 * 后端 production-orders 列表接口现已真实接收 order_no 并对 production_order.order_no 列做
 * like 过滤（此前该参数被 serde 静默丢弃、筛选恒不生效）。用于把列表精确收敛到目标单号。
 */
async function filterByOrderNo(page: Page, orderNo: string): Promise<void> {
  await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  await page.getByLabel(/订单编号/).fill(orderNo);
  await page.getByRole('button', { name: /查询/ }).click();
}

/**
 * 用真实产品 API 建一张生产工单（默认态 DRAFT）。
 * 显式传入唯一 order_no 以便列表按单号精确定位；返回 {id, order_no}（缺失即抛，不兜底）。
 */
async function createProductionOrder(
  page: Page,
  productId: number,
  orderNo?: string
): Promise<{ id: number; order_no: string }> {
  const created = await apiCall<{ id?: number; order_no?: string }>(
    page,
    'POST',
    '/production/production-orders/orders',
    {
      order_no: orderNo,
      product_id: productId,
      planned_quantity: 100,
      priority: 5,
    }
  );
  const id = created.data?.id;
  const order_no = created.data?.order_no;
  if (!id || !order_no) {
    throw new Error(`生产工单造数失败：${JSON.stringify(created).slice(0, 200)}`);
  }
  return { id, order_no };
}

test.describe('生产计划 - 01 工单创建与排产', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('生产计划页面可访问', async ({ page }) => {
    await page.goto('/production');
    await expect(page.getByRole('heading', { name: /生产计划|生产工单/ })).toBeVisible({
      timeout: 30000,
    });
  });

  test('新建生产工单', async ({ page }) => {
    // 真实造数：产品为外键，ProductionForm 的「产品ID / 工作中心ID / 计划数量 / 优先级」均为
    // el-input-number 数字输入（非下拉）——原用例把它们当 el-select 点 option 属臆测；
    // 且「订单编号」label 与筛选栏同名（production.form/filter.labelOrderNo='订单编号'）→ getByLabel strict 命中 2。
    await ensureTestEntities(page);
    const ctx = getCtx();
    expect(ctx.productIds.length, '前置：需要至少一个产品').toBeGreaterThanOrEqual(1);

    await page.goto('/production');
    await page.getByRole('button', { name: '新建订单' }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    // 限定到 .el-dialog 消除同名 strict；提交按钮真实文案「确定」（production.form.buttonConfirm）；
    // 成功提示为 production.index.messageCreateSuccess=「创建生产订单成功」。
    const dlg = page.locator('.el-dialog');
    await dlg.getByLabel('订单编号').fill(`E2E-${Date.now()}`);
    await dlg.getByLabel('产品ID').fill(String(ctx.productIds[0]));
    await dlg.getByLabel('计划数量').fill('100');
    await dlg.getByLabel('优先级').fill('5');
    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('创建生产订单成功')).toBeVisible({ timeout: 30000 });
  });

  test('草稿工单可编辑', async ({ page }) => {
    // 真实造数：建两张 DRAFT 工单，一张为编辑目标、一张为干扰项，
    // 保证列表必有"编辑"入口。
    // 原实现 `if (await editBtn.isVisible())`：V2Table 操作列按钮是 el-button(link)
    // → ARIA 角色是 button 而非 link，`getByRole('link')` 恒不命中 → 零断言假绿。
    // order_no 缺陷已修复：此处恢复按「订单编号」精确定位目标工单（此前因后端丢弃
    // order_no 只能用状态筛选规避，无法把结果收敛到单个单号）。
    await ensureTestEntities(page);
    const ctx = getCtx();
    expect(ctx.productIds.length, '前置：需要至少一个产品').toBeGreaterThanOrEqual(1);
    const stamp = Date.now();
    const target = await createProductionOrder(page, ctx.productIds[0], `E2E-EDIT-${stamp}`);
    const distractor = await createProductionOrder(
      page,
      ctx.productIds[0],
      `E2E-DISTRACTOR-${stamp}`
    );

    await page.goto('/production');
    await filterByOrderNo(page, target.order_no);

    // 真实生效证据：结果含目标单号，且不含干扰单号（证明 order_no 过滤真的下推到 SQL）
    await expect(page.getByText(target.order_no)).toBeVisible({ timeout: 30000 });
    await expect(page.getByText(distractor.order_no)).toHaveCount(0);

    const editBtn = page.getByRole('button', { name: '编辑', exact: true }).first();
    await expect(editBtn, '草稿工单应渲染"编辑"按钮').toBeVisible({ timeout: 30000 });
    await editBtn.click();
    // 表单对话框字段标签与筛选栏"订单编号"同名，作用域限定到对话框避免多匹配
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible({ timeout: 10000 });
    await expect(dialog.getByLabel(/订单编号/)).toBeVisible();
    await dialog.getByRole('button', { name: /取消/ }).click();
  });

  test('已审批工单可计划排产（APPROVED → 已排产）', async ({ page }) => {
    // 状态机为 DRAFT →(submit)→ PENDING_APPROVAL →(approve)→ APPROVED →(排产)→ SCHEDULED。
    // 原用例标题"草稿工单可计划排产（draft→planned）"与产品不符："计划"按钮仅在
    // APPROVED 态渲染，DRAFT 态点不到；且 `if (await scheduleBtn.isVisible())` + getByRole('link')
    // 双重失配 → 恒零断言假绿。此处先经 API 把工单推进到 APPROVED，再硬断言"计划"按钮并驱动排产。
    await ensureTestEntities(page);
    const ctx = getCtx();
    expect(ctx.productIds.length, '前置：需要至少一个产品').toBeGreaterThanOrEqual(1);
    const { id } = await createProductionOrder(page, ctx.productIds[0]);
    await apiCall(page, 'POST', `/production/production-orders/orders/${id}/submit-approval`);
    await apiCall(page, 'POST', `/production/production-orders/orders/${id}/approve`, {
      approved: true,
    });

    await page.goto('/production');
    await filterByStatus(page, '已审批');

    const planBtn = page.getByRole('button', { name: '计划', exact: true }).first();
    await expect(planBtn, '已审批工单应渲染"计划"排产按钮').toBeVisible({ timeout: 30000 });
    await planBtn.click();
    await page.getByRole('button', { name: /确定/ }).click();
    await expect(page.getByText(/状态更新成功/)).toBeVisible({ timeout: 30000 });
  });
});
