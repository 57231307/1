// 定制订单 E2E 测试
// Playwright 测试定制订单全流程
// 创建时间: 2026-06-17
//
// v8 复审 P0-3 修复（2026-06-30）：
// 对齐批次 28 P0-1 fail-secure 模式，凭据从环境变量注入，禁止硬编码 admin/admin123。
//
// 本轮根因订正（extras 分片 16 红之一族，全部为测试侧）：
// 1) 登录：原 `BASE_URL = process.env.BASE_URL || 'http://localhost:8080'` + 手写 login()
//    导航到 :8080/login —— CI 前端由 playwright.config.ts 的 webServer 起在 :3000（后端 :8082），
//    :8080 无监听 → net::ERR_CONNECTION_REFUSED，四条用例全灭于第一步。改为复用
//    globalSetup 注入的真实登录态 storageState（playwright.config.ts use.storageState）+ 相对导航，
//    与 dashboard/fabric 等同级 spec 一致；不再自行拼绝对端口，也不重复登录。
// 2) 文案：列表页真实标题为「定制订单列表」（customOrders.list.title，非 meta 的「定制订单管理」）、
//    新建入口按钮为「新建订单」（list.createButton，非页面名「新建定制订单」）、
//    跟踪页真实标题为「订单跟踪」（tracking.title，非「工艺跟踪」）。原用例逐条对不上。
// 3) 数据依赖：原用例硬编码 /custom-orders/1，依赖环境恰好存在 id=1 的订单（extras 分片独立运行
//    并无此保证）。改为经真实 API（ensureTestEntities 提供合法外键 customer_id/product_id）造草稿单，
//    用其真实 id 驱动详情/推进/跟踪/质量异常。
// 4) 跟踪大屏节点（纱线采购/染整/后整理/交付/售后）来自后端真实工艺节点，草稿单无节点 → 原断言恒空；
//    改断跟踪页真实渲染（标题 + 订单号 + 操作日志分隔），不再断不存在的节点名。

import { test, expect, type Page } from './diagnose-fixture';
import { applyAuthMocks } from './smoke/_helpers';
import { apiCall, ensureTestEntities, getCtx, tryCleanup } from './flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];

test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/**
 * 真实造一条草稿定制订单：先 ensureTestEntities 保证存在合法客户/产品外键，再 POST /custom-orders。
 * 字段与后端 CreateCustomOrderDto 对齐：customer_id(i64) / product_id(i64) / spec(非空) /
 * quantity(>0) / unit(m|kg|pcs)。返回真实 id 与 order_no，登记清理。
 */
async function seedCustomOrder(page: Page): Promise<{ id: number; orderNo: string }> {
  await ensureTestEntities(page);
  const ctx = getCtx();
  const customerId = ctx.customerId;
  const productId = ctx.productIds[0];
  if (!customerId || !productId) {
    throw new Error(
      `seedCustomOrder：ensureTestEntities 未提供 customerId/productId（${JSON.stringify({
        customerId,
        productIds: ctx.productIds,
      })}）`
    );
  }
  const res = await apiCall<{ id?: number; order_no?: string }>(page, 'POST', '/custom-orders', {
    customer_id: customerId,
    product_id: productId,
    spec: 'E2E 100% 棉 200g/m²',
    quantity: 100,
    unit: 'm',
  });
  const id = res.data?.id;
  if (!id) throw new Error(`创建定制订单失败：${JSON.stringify(res).slice(0, 200)}`);
  CLEANUP.push({ path: `/custom-orders/${id}`, label: 'custom_order' });
  return { id, orderNo: res.data?.order_no ?? '' };
}

test.describe('定制订单全流程跟踪 E2E', () => {
  test.beforeEach(async ({ context }) => {
    // 注入真实登录态（context 级 cookie + CSRF），使后续相对导航为已登录，apiCall 可用。
    await applyAuthMocks(context);
  });

  test('创建定制订单', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const customerId = ctx.customerId;
    const productId = ctx.productIds[0];
    expect(customerId, 'ensureTestEntities 未提供 customerId').toBeTruthy();
    expect(productId, 'ensureTestEntities 未提供 productId').toBeTruthy();

    // 进入定制订单列表
    await page.goto('/custom-orders');
    await expect(page.getByText('定制订单列表')).toBeVisible({ timeout: 30_000 });

    // 点击新建（真实按钮文案 list.createButton=「新建订单」）
    await page.getByRole('button', { name: '新建订单' }).click();
    await expect(page).toHaveURL(/\/custom-orders\/new$/);

    // 填写表单（真实字段：客户ID / 产品ID / 规格 / 数量；unit 默认 'm'）
    // labelSpec='规格' 与 labelYarnSpec='纱线规格' 共享子串，须 exact 避免 strict 多命中
    // 数量项 form-item 内同时含数字 input 与 unit el-select，源码有 for="co-create-quantity"，
    // 直接锚真实 input id 填值，避免 getByLabel 命中容器 div。
    await page.getByLabel('客户ID').fill(String(customerId));
    await page.getByLabel('产品ID').fill(String(productId));
    await page.getByLabel('规格', { exact: true }).fill('E2E 100% 棉 200g/m²');
    await page.locator('#co-create-quantity').fill('100');

    // 提交（create.buttonSaveDraft=「保存草稿」）→ 成功提示（create.messageCreateSuccess）
    await page.getByRole('button', { name: '保存草稿' }).click();
    await expect(page.getByText('创建成功')).toBeVisible({ timeout: 30_000 });

    // 成功后跳转详情页，验证基本信息 Tab 真实渲染，并据 URL 登记清理
    await expect(page).toHaveURL(/\/custom-orders\/\d+$/);
    await expect(page.getByRole('tab', { name: '基本信息', exact: true })).toBeVisible();
    const id = Number((page.url().match(/\/custom-orders\/(\d+)$/) ?? [])[1]);
    expect(id, '创建后应跳转到 /custom-orders/{id}').toBeGreaterThan(0);
    CLEANUP.push({ path: `/custom-orders/${id}`, label: 'custom_order' });
  });

  test('推进订单状态', async ({ page }) => {
    const { id } = await seedCustomOrder(page);
    await page.goto(`/custom-orders/${id}`);

    // 详情页推进按钮（detail.buttonAdvance=「推进状态」，草稿态可见）
    const advanceBtn = page.getByRole('button', { name: '推进状态', exact: true });
    await expect(advanceBtn, '草稿订单应渲染「推进状态」按钮').toBeVisible({ timeout: 30_000 });
    await advanceBtn.click();

    // ElMessageBox 确认（detail.messageAdvanceTitle=「推进确认」，确认按钮「确定」）
    await page.getByRole('button', { name: '确定' }).last().click();

    // 真实结果：detail.messageAdvanceSuccess=「推进成功」
    await expect(page.getByText('推进成功')).toBeVisible({ timeout: 30_000 });
    await expect(page.getByRole('tab', { name: '基本信息', exact: true })).toBeVisible();
  });

  test('查看工艺跟踪大屏', async ({ page }) => {
    const { id, orderNo } = await seedCustomOrder(page);
    await page.goto(`/custom-orders/${id}/track`);

    // 跟踪页真实标题 tracking.title=「订单跟踪」，头部拼接订单号。
    // 原用例断「工艺跟踪」（仅路由 meta，非页面文本）及 5 个工艺节点名（纱线采购/染整/后整理/
    // 交付/售后）——节点来自后端真实 process_nodes，草稿单无节点 → 恒不渲染，断言不成立。
    await expect(page.getByText('订单跟踪')).toBeVisible({ timeout: 30_000 });
    if (orderNo) {
      await expect(page.getByText(orderNo).first(), '跟踪页头部应展示订单号').toBeVisible();
    }
    // 时间线区块真实存在（tracking.dividerOperationLog='操作日志'，渲染为 el-divider__text）。
    // 空态 el-empty description='暂无操作日志' 含子串"操作日志"导致 getByText strict 多命中，
    // 须限定到 .el-divider__text 元素（tracking.vue:68）。
    await expect(page.locator('.el-divider__text', { hasText: '操作日志' })).toBeVisible({
      timeout: 30_000,
    });
  });

  test('上报质量异常', async ({ page }) => {
    const { id } = await seedCustomOrder(page);
    await page.goto(`/custom-orders/${id}`);

    // 切换到质量异常 tab（detail.tabQualityIssues 含「质量异常」子串）
    await page.getByRole('tab', { name: /质量异常/ }).click();

    // 上报入口按钮 common.qualityCheck.reportIssue=「上报异常」
    await page.getByRole('button', { name: '上报异常' }).click();
    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog, '上报异常对话框应打开').toBeVisible({ timeout: 30_000 });

    // 异常类型下拉选「色差」（issueType.colorDiff）；严重度默认 medium 已满足必填
    // 异常类型是 el-select 的 readonly combobox，getByRole('combobox') 命中内层 input，EP 拦截其
    // 直接 click → 30s 超时。改锚含该 label 的 form-item 内 .el-select 触发，选项取 body-level
    // popper 的 .el-select-dropdown__item（对齐 8c1adf03 ai/crm 既有写法）。
    const issueTypeItem = dialog
      .locator('.el-form-item')
      .filter({ has: dialog.locator('.el-form-item__label', { hasText: '异常类型' }) })
      .first();
    await issueTypeItem.locator('.el-select').first().click();
    await page
      .locator('.el-select-dropdown:visible .el-select-dropdown__item')
      .filter({ hasText: '色差' })
      .first()
      .click();

    // 描述为必填（reportRules.description）
    await dialog.getByLabel('描述').fill('批次色差 ΔE=3.5 超过 2.0 阈值');

    // 提交（common.qualityCheck.submit=「提交」）→ 成功（reportSuccess=「异常上报成功」）
    await dialog.getByRole('button', { name: '提交' }).click();
    await expect(page.getByText('异常上报成功')).toBeVisible({ timeout: 30_000 });

    // 真实落库验证：质量异常列表出现该条记录（异常类型列展示「色差」）
    await expect(
      page.getByRole('row').filter({ hasText: '色差' }).first(),
      '上报后质量异常列表应出现色差记录'
    ).toBeVisible({ timeout: 30_000 });
  });
});
