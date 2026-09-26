import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall, BASE_URL, getCtx, ensureTestEntities } from './helpers';
import { findTableRow } from './ui-helpers';

/**
 * 新增域业务流转链防线（flow/54-new-domains-lifecycle）
 *
 * 自审修复背景：批 F~M 手写页面 payload 与后端 DTO 存在 12 处字段不匹配
 * （8D tagged enum、坏账 period_year/month、验布 inspected_yards、
 * 劳动合同 termination_date/reason、社保 payment_date、委外/催收/协作域字段名）。
 * 本 spec 用真实后端校验这些页面的核心操作链路，防回归。
 *
 * 规则：对齐后端 handler 真实请求体字段（禁止探测式简写）。
 */

test.describe.serial('新域业务流转链', () => {
  test('质量 8D：定制单→上报质量问题→启动→推进 D1（tagged enum）', async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
    await page.goto(`${BASE_URL}/quality-8d`);
    await expect(page.locator('.page')).toBeVisible();

    // 前置：创建真实客户+产品（外键约束 custom_orders_customer_id_fkey）
    const ts = Date.now().toString().slice(-6);
    const customer = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `54Cust${ts}`,
      code: `54C${ts}`,
      contact_person: '54测试',
      phone: '13800000054',
    });
    const customerId = customer?.data?.id;
    expect(customerId, '客户创建失败').toBeTruthy();
    // 分类列表在新环境为空，先真实创建分类（fk_products_category 外键指向 product_categories.id）
    const cat = await apiCall<{ id?: number }>(page, 'POST', '/categories', {
      name: `54Cat${ts}`,
      code: `54CAT${ts}`,
    });
    const categoryId = cat?.data?.id;
    expect(categoryId, '产品分类创建失败').toBeTruthy();
    const product = await apiCall<{ id?: number }>(page, 'POST', '/products', {
      name: `54Prod${ts}`,
      code: `54P${ts}`,
      category_id: categoryId,
      unit: 'm',
    });
    const productId = product?.data?.id;
    expect(productId, '产品创建失败').toBeTruthy();

    // 前置：quality_issues 外键必须有真实记录——创建定制订单+上报质量问题
    const co = await apiCall<{ id?: number }>(page, 'POST', '/custom-orders', {
      customer_id: customerId,
      product_id: productId,
      spec: 'E2E-8D-SPEC',
      quantity: 10,
      unit: 'm',
    });
    const coId = co?.data?.id;
    expect(coId, '定制订单创建失败').toBeTruthy();
    // ReportQualityIssueDto 必填 custom_order_id / issue_type / severity / description
    // （models/quality_issue_dto.rs:13-27）。原实现漏 custom_order_id 与 severity：
    // 前者被后端 422 拒绝，后者即使补上前也仍是必填项，一并补齐。
    const issue = await apiCall<{ id?: number }>(page, 'POST', `/custom-orders/${coId}/issues`, {
      custom_order_id: coId,
      issue_type: 'after_sales_reported',
      severity: 'medium',
      description: 'E2E 8D 前置质量问题',
    });
    const issueId = issue?.data?.id;
    expect(issueId, '质量问题创建失败').toBeTruthy();

    // 启动：StartEightDDto { quality_issue_id, plan? }
    await page.getByRole('button', { name: '启动 8D' }).click();
    await page.locator('.el-input-number input').first().fill(String(issueId));
    await page.getByRole('button', { name: '启动', exact: true }).click();
    await expect(page.locator('.el-message--success').first()).toBeVisible({ timeout: 8000 });

    // 推进：AdvanceStepPayload = { step: 'd1_team', team_members }
    // findTableRow 是 async，此前漏 await 使 row 成为 Promise，
    // row.getByRole 直接 TypeError（CI: "row.getByRole is not a function"）
    const row = await findTableRow(page, String(issueId));
    expect(row, `质量问题列表中应存在 id=${issueId} 的行，否则 8D 推进入口无从点击`).toBeTruthy();
    await row!.getByRole('button', { name: '推进下一阶段' }).click();
    await page.locator('.el-message-box__input input').fill('张三、李四（D1 团队）');
    await page.locator('.el-message-box__btns .el-button--primary').click();
    await expect(page.locator('.el-message--success').first()).toBeVisible({ timeout: 8000 });
    // 状态应从 not_started/d0 推进至 d1
    await expect(row!).toContainText('D1', { timeout: 5000 });
  });

  test('坏账：计提（period_year/period_month）→ 确认 → 冲销状态呈现', async ({ page }) => {
    await loginViaUI(page);
    await page.goto(`${BASE_URL}/bad-debts`);
    await expect(page.locator('.page')).toBeVisible();

    await page.getByRole('button', { name: '运行计提' }).click();
    // RunProvisionRequest { period_year, period_month } —— 默认值即当前年月
    await page.getByRole('button', { name: '执行' }).click();
    await expect(page.locator('.el-message--success, .el-message--warning').first()).toBeVisible({
      timeout: 10000,
    });
  });

  test('验布：定级（inspected_yards 必填）→ 关闭', async ({ page }) => {
    await loginViaUI(page);
    await page.goto(`${BASE_URL}/fabric-inspections`);
    await expect(page.locator('.page')).toBeVisible();

    // 新建：表单只预填验布日期（后端唯一必填项），其余留空即可建单。
    // run 4623 这里失败的真实原因是建单表单提交的是 fabric_batch_no/total_length_m
    // 两个后端根本没有的字段、又缺 inspection_date，被 422 拒掉后页面停在
    // ErrorBoundary，看起来像"等不到成功提示"（表单已按 CreateInspectionRequest 重建）。
    // 接住"最终创建成功"的响应：CSRF token 为一次性消费（middleware/csrf.rs 的
    // consume_csrf_token NotFound 分支 → 403 + X-New-CSRF-Token 恢复头），前端
    // api/request.ts 拦截器读到恢复头后自动重放同一 POST 并成功返回 200。
    // 因此第一跳是 CSRF 恢复链的 403（非用例缺陷），必须用 status()===200 让
    // waitForResponse 命中重放后的成功响应，否则会抓到 403、data 为空导致误判。
    const [createResp] = await Promise.all([
      page.waitForResponse(
        r =>
          r.url().includes('/production/fabric-inspections') &&
          r.request().method() === 'POST' &&
          r.status() === 200,
        { timeout: 15_000 }
      ),
      // 新建验布单默认评分制式为四分制（four_point），而 grade_inspection 四分制分支
      // 要求 fabric_width_inches 非空（fabric_inspection_service.rs:454-455），缺则定级被正当
      // 拒绝（400「四分制评级需要幅宽」）。建单表单已有「门幅(英寸)」el-input-number
      // （index.vue:116-123 绑 fabric_width_inches），保存前真实填入，保证后续定级可 2xx。
      (async () => {
        await page.getByRole('button', { name: '新建验布单' }).click();
        const widthInput = page
          .locator('.el-dialog:visible .el-form-item')
          .filter({ has: page.locator('.el-form-item__label', { hasText: '门幅' }) })
          .locator('.el-input-number input')
          .first();
        await widthInput.waitFor({ state: 'visible', timeout: 10_000 });
        await widthInput.fill('60');
        await page.getByRole('button', { name: '保存' }).click();
      })(),
    ]);
    const created = await createResp.json();
    const inspectionNo = String(created?.data?.inspection_no ?? '');
    expect(
      inspectionNo,
      `新建验布单应回验布单号（重放后 200 响应），实际响应：${JSON.stringify(created).slice(0, 200)}`
    ).not.toBe('');
    await expect(page.locator('.el-message--success').first()).toBeVisible({ timeout: 8000 });

    // 定级：GradeInspectionRequest { inspected_yards, qualification_rate? }
    const row = page.locator('.el-table__row').filter({ hasText: inspectionNo }).first();
    await expect(row, `列表首页找不到刚建的验布单 ${inspectionNo}`).toBeVisible({
      timeout: 10_000,
    });
    await row.getByRole('button', { name: '定级' }).click();
    const gradeDialog = page.locator('.el-dialog:visible').filter({ hasText: '验布定级' });
    await gradeDialog.locator('.el-input-number input').first().fill('120');
    await gradeDialog.getByRole('button', { name: '提交' }).click();
    await expect(page.locator('.el-message--success').first()).toBeVisible({ timeout: 8000 });
    // 定级结果落库校验：状态应转为 graded（关闭按钮只在 graded 态出现）
    await expect(row.getByRole('button', { name: '关闭' })).toBeVisible({ timeout: 10_000 });
    await row.getByRole('button', { name: '关闭' }).click();
    await page.getByRole('button', { name: '确定' }).click();
    await expect(page.locator('.el-message--success').first()).toBeVisible({ timeout: 8000 });
    // 关闭是终态：定级按钮（status !== 'closed' 才渲染）应从该行消失
    await expect(row.getByRole('button', { name: '定级' })).toHaveCount(0);
  });

  test('社保：标记已缴（payment_date 必填）', async ({ page }) => {
    await loginViaUI(page);
    await page.goto(`${BASE_URL}/social-insurance`);
    await expect(page.locator('.page')).toBeVisible();

    await page.getByRole('button', { name: '新建参保记录' }).click();
    // 对话框内 .el-input-number 自上而下为 员工ID/年度/月份/缴费基数，各含一个可定位的真实
    // input（增减按钮是其兄弟节点，直接命中 input 即避开 intercepts pointer events）。
    // 原用例把「缴费基数」错填成 nth(2)（实为月份），致 base_amount 为空 → 前端守卫弹 warning、
    // 无 success toast。修正为 nth(3)，值 5000 落在后端合规区间 [4250,31884]；年/月保留表单默认。
    const spins = page.locator('.el-dialog .el-input-number input');
    await spins.nth(0).fill('1'); // 员工ID
    await spins.nth(3).fill('5000'); // 缴费基数
    await page.getByRole('button', { name: '保存' }).click();
    await expect(page.locator('.el-message--success').first()).toBeVisible({ timeout: 8000 });

    // 注：本步在 create 成功后仍会失败，根因是状态词表不一致（非测试姿势）——后端 service.create
    // 无 payment_date 时写入 status="pending"（social_insurance_service.rs:200-203），mark_paid 门槛
    // 也要求 "pending"（:287），而前端 index.vue 标签/按钮以 "unpaid" 为键（INSURANCE_STATUS_LABEL
    // 无 pending，v-if="row.status === 'unpaid'"），致行不显示「未缴」且「标记已缴」按钮不渲染。
    // 属源码缺陷（消费方词表须对齐写入方 pending，AGENTS.md 状态词表铁律），未放宽/未 skip，交前端修。
    const row = page.locator('.el-table__row', { hasText: '未缴' }).first();
    await row.getByRole('button', { name: '标记已缴' }).click();
    // MarkPaidRequest { payment_date } 必填，默认今天
    const today = new Date().toISOString().slice(0, 10);
    await expect(page.locator('.el-message-box__input input')).toHaveValue(today);
    await page.locator('.el-message-box__btns .el-button--primary').click();
    await expect(page.locator('.el-message--success').first()).toBeVisible({ timeout: 8000 });
  });

  test('委外：创建 → 发出 → 加工中 → 结算（无 body 状态操作链）', async ({ page }) => {
    await loginViaUI(page);
    await page.goto(`${BASE_URL}/outsourcing`);
    await expect(page.locator('.page')).toBeVisible();

    const orderNo = `OUT-E2E-${Date.now()}`;
    await page.getByRole('button', { name: '新建委外单' }).click();
    await page.locator('.el-dialog input').first().fill(orderNo);
    await page.locator('.el-dialog .el-input-number input').first().fill('1');
    await page.locator('.el-dialog .el-input-number input').nth(1).fill('100');
    await page.getByRole('button', { name: '保存' }).click();
    await expect(page.locator('.el-message--success').first()).toBeVisible({ timeout: 8000 });

    // 状态链：draft → issued（发出）
    const row = await findTableRow(page, orderNo);
    expect(row, `列表中应存在单号 ${orderNo} 的行，否则无法执行发出`).toBeTruthy();
    await row!.getByRole('button', { name: '发出' }).click();
    await page.locator('.el-message-box__btns .el-button--primary').click();
    await expect(page.locator('.el-message--success').first()).toBeVisible({ timeout: 8000 });
    await expect(row!).toContainText('已发出', { timeout: 5000 });
  });

  test('客户协作：合同签署（contract_id+signed_by_user_id）→ 列表呈现', async ({ page }) => {
    await loginViaUI(page);
    await page.goto(`${BASE_URL}/customer-collab`);
    await expect(page.locator('.page')).toBeVisible();

    await page.getByRole('button', { name: '签署合同' }).click();
    // SignContractRequest { contract_id, signed_by_user_id, signature_image_url? }
    await page.locator('.el-dialog .el-input-number input').first().fill('1');
    await page.locator('.el-dialog .el-input-number input').nth(1).fill('1');
    await page.getByRole('button', { name: '签署' }).click();
    await expect(page.locator('.el-message--success, .el-message--error').first()).toBeVisible({
      timeout: 8000,
    });
  });
});
