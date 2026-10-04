import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  BASE_URL,
  ensureTestEntities,
  getCtx,
  genCode,
  genName,
} from './helpers';
import { findTableRow, pickSelect, fillFieldByLabel, pickListArray } from './ui-helpers';

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

    // 社保状态词表以写入方为准：后端 service 无 payment_date 时写 status="pending"、
    // mark_paid 门槛也要求 "pending"（social_insurance_service.rs:200-203/287）；前端
    // INSURANCE_STATUS_LABEL 与"标记已缴"v-if 已对齐 pending（标签值仍显示"未缴"），
    // 故此行显"未缴"、按钮可渲染。
    const row = page.locator('.el-table__row', { hasText: '未缴' }).first();
    await row.getByRole('button', { name: '标记已缴' }).click();
    // MarkPaidRequest { payment_date } 必填，默认今天
    const today = new Date().toISOString().slice(0, 10);
    await expect(page.locator('.el-message-box__input input')).toHaveValue(today);
    await page.locator('.el-message-box__btns .el-button--primary').click();
    await expect(page.locator('.el-message--success').first()).toBeVisible({ timeout: 8000 });
  });

  test('委外：建生产匹 → 建单 → 登记发料明细 → 发出（匹号门控真实链）', async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
    const ctx = getCtx();
    await page.goto(`${BASE_URL}/outsourcing`);
    await expect(page.locator('.page')).toBeVisible();

    // 前置（CI #4675 E7 判责：测试前提缺失，源码 fail-closed 正确）：
    // 委外发料门控对 issue 无条件生效（outsourcing_ops/order.rs::issue_order 事务内
    // reserve_pieces_for_issue → piece_domain_service.rs::validate_pieces_for_issue）：
    // 明细集合为空整单拒绝「委外订单没有发料明细，无法发料；发料必须精确到匹」，
    // 且明细必须引用真实存在、状态 AVAILABLE 的生产匹——匹号领域二期故意的 fail-closed，
    // 禁止兜底放行。门控设计符合行业口径（发料精确到匹、账实核对），源码侧无需改；
    // 正解是按生产真实流程先造出可发料的生产匹再登记明细，而不是绕门或放宽断言。
    // 链与 flow/07-fabric-four-dim.spec.ts 7-1~7-4 及 helpers.seedDyedPieceChain 步骤1-2同构：
    // 生产订单 → 流转卡（schedule→备布→完成备布）→ 工序启动 → 报工逐匹登记 1 匹生产匹。
    const productId = ctx.productIds[0];
    if (!productId) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
    if (!ctx.supplierId) throw new Error('前置缺失：ctx.supplierId 未就绪（委外单必填加工厂）');
    // 生产匹（胚布 greige）入仓：优先胚布仓，其次未设类型仓；成品仓被
    // validate_warehouse_for_piece_type 必拒（piece_domain_service.rs:82-107），
    // 列表无可入仓时显式判红（不 skip、不塞非法仓 id）。
    const whRes = await apiCallRaw<{ items?: unknown }>(
      page,
      'GET',
      '/warehouses?page=1&page_size=200'
    );
    const warehouses = pickListArray<Record<string, unknown>>(whRes, 'items', '54 仓库列表');
    const greigeWh = (warehouses.find(w => w.warehouse_type === 'greige') ??
      warehouses.find(
        w => w.warehouse_type === null || w.warehouse_type === undefined || w.warehouse_type === ''
      )) as Record<string, unknown> | undefined;
    if (!greigeWh) {
      throw new Error(
        '[54-委外] 仓库列表既无胚布仓(greige)也无未设类型仓，生产匹无处入库（成品仓必被 ' +
          'validate_warehouse_for_piece_type 拒）——真实前置造不出，显式判红'
      );
    }
    const greigeWarehouseId = Number(greigeWh.id);

    const productionOrderNo = genCode('54PO');
    const po = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/production/production-orders/orders',
      {
        order_no: productionOrderNo,
        product_id: productId,
        planned_quantity: 100,
      }
    );
    const productionOrderId = po.data?.id;
    expect(productionOrderId, '委外前置：生产订单创建应返回 id').toBeTruthy();
    const card = await apiCall<{ id?: number }>(page, 'POST', '/production/flow-cards', {
      production_order_id: productionOrderId,
      product_id: productId,
      product_name: genName('54胚布'),
      planned_fabric_weight: 100,
    });
    const cardId = card.data?.id;
    expect(cardId, '委外前置：流转卡创建应返回 id').toBeTruthy();
    await apiCall(page, 'POST', `/production/flow-cards/${cardId}/schedule`, {});
    await apiCall(page, 'POST', `/production/flow-cards/${cardId}/start-preparing`);
    await apiCall(page, 'POST', `/production/flow-cards/${cardId}/complete-preparing`, {
      actual_fabric_weight: 100,
    });
    const step = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/production/flow-cards/steps/start',
      {
        flow_card_id: cardId,
      }
    );
    const stepId = step.data?.id;
    expect(stepId, '委外前置：工序启动应返回 id').toBeTruthy();
    const pieceNo = `GR-${genCode('54P')}-001`;
    await apiCall(page, 'POST', `/production/flow-cards/steps/${stepId}/complete`, {
      actual_quantity: 100,
      qualified_quantity: 100,
      pieces: [
        {
          piece_no: pieceNo,
          machine_no: 'M-E2E-54',
          machine_operator: 'E2E开机人',
          length: 100,
          weight: 50,
          warehouse_id: greigeWarehouseId,
        },
      ],
    });
    // 写后必回读（本仓红线：不造数不假设）：生产匹必须真实存在且 AVAILABLE，
    // 其维度（batch_no=生产单号、color_no/dye_lot_no 空串）是后续发料明细唯一来源，
    // 禁止硬编码假维度。词表大写来源 models/status/purchase_inventory.rs::inventory_piece。
    interface PieceRow {
      piece_no: string;
      status: string;
      color_no: string | null;
      dye_lot_no: string | null;
      batch_no: string;
      warehouse_id: number;
      product_id: number;
    }
    const pieceRows = pickListArray<PieceRow>(
      await apiCallRaw<unknown>(
        page,
        'GET',
        `/inventory/pieces?piece_no=${encodeURIComponent(pieceNo)}&page=1&page_size=10`
      ),
      'items',
      '54 生产匹回读'
    );
    const piece = pieceRows.find(p => p.piece_no === pieceNo);
    expect(piece, `报工后应能按匹号回读到生产匹 ${pieceNo}`).toBeTruthy();
    expect(
      String(piece!.status),
      `生产匹 ${pieceNo} 入库后应为 AVAILABLE（实际 ${piece!.status}）`
    ).toBe('AVAILABLE');

    await page.getByRole('button', { name: '新建委外单' }).click();
    const dialog = page.locator('.el-dialog:visible').first();
    await expect(dialog).toBeVisible();

    // 委外单号由 openCreate() 客户端自动生成（generateUniqueDocNo），readonly 不可 fill。
    // 选类型 el-select（dialog 内唯一 el-select）、填供应商ID/发出数量/发出日期。
    await pickSelect(page, dialog.locator('.el-select').first(), '染色加工');
    await dialog.locator('.el-input-number input').first().fill(String(ctx.supplierId)); // 供应商ID
    const today = new Date().toISOString().slice(0, 10);
    const dateInput = dialog
      .locator('.el-form-item')
      .filter({ hasText: '发出日期' })
      .locator('input')
      .first();
    await dateInput.click();
    await dateInput.fill(today);
    await page.keyboard.press('Enter');
    await dialog.locator('.el-input-number input').nth(1).fill('100'); // 发出数量
    // 材料成本 + 三费均为后端 outsourcing_order NOT NULL 列（v15:3247-3249）对应的建单必填，
    // 前端 onCreate 守卫（views/outsourcing/index.vue:499-513）对 材料成本/加工费/运费/税额
    // 逐项 ==null 判空，缺任一项即 warning 拦截、不发 POST——判责 #4669 J-4：
    // CI 的 waitForResponse(/outsourcing-orders POST) 15s 超时根因是请求根本没发出
    // （测试漏填三费触发正当前端守卫），非响应慢、也非 reactive 断链类前端缺陷。
    await fillFieldByLabel(dialog, page, '材料成本', '500');
    await fillFieldByLabel(dialog, page, '加工费', '80');
    await fillFieldByLabel(dialog, page, '运费', '20');
    await fillFieldByLabel(dialog, page, '税额', '13');

    // 等待创建响应获取真实单号（readonly 自动生成的值通过 POST 回传到服务端确认）
    const [respPromise] = await Promise.all([
      page.waitForResponse(
        r =>
          r.url().includes('/production/outsourcing-orders') &&
          r.request().method() === 'POST' &&
          r.status() === 200,
        { timeout: 15_000 }
      ),
      dialog.getByRole('button', { name: '保存' }).click(),
    ]);
    const respData = await respPromise.json();
    const orderNo = String(respData?.data?.order_no ?? '');
    const orderId = Number(respData?.data?.id ?? 0);
    expect(
      orderNo,
      `创建委外单应返回 order_no，实际=${JSON.stringify(respData).slice(0, 200)}`
    ).not.toBe('');
    expect(
      orderId,
      `创建委外单应返回 id，实际=${JSON.stringify(respData).slice(0, 200)}`
    ).toBeGreaterThan(0);
    await expect(page.locator('.el-message--success').first()).toBeVisible({ timeout: 8000 });

    // 发料明细登记（POST /production/outsourcing-orders/items，CreateOutsourcingOrderItemRequest）：
    // piece_no 必填且引用上面回读的真实 AVAILABLE 生产匹；维度取自匹行回读值
    // （生产匹 batch_no=生产单号、胚布无缸号 color_no/dye_lot_no 为空串），仓库取匹行所在仓，
    // 与 outsourcing_ops/order_item.rs::create 落库键完全同源。数量 100 / 单位成本 5
    // 使明细成本合计与单头材料成本 500 一致（真实业务口径，非凑数）。
    const item = await apiCall<{ id?: number; piece_no?: string }>(
      page,
      'POST',
      '/production/outsourcing-orders/items',
      {
        outsourcing_order_id: orderId,
        product_id: piece!.product_id,
        color_no: piece!.color_no ?? '',
        dye_lot_no: piece!.dye_lot_no ?? '',
        batch_no: piece!.batch_no,
        warehouse_id: piece!.warehouse_id,
        piece_no: pieceNo,
        quantity: 100,
        unit: '米',
        unit_cost: 5,
      }
    );
    expect(
      item.data?.id,
      `发料明细创建应返回 id：${JSON.stringify(item).slice(0, 200)}`
    ).toBeTruthy();
    // 写后必回读：GET items/by-order 返回裸数组（list_outsourcing_items 信封 Vec<Model>），
    // 明细须真实挂在本单下且匹号与登记一致——否则后续"发出"点的就是空明细单。
    const itemRows = pickListArray<{
      id: number;
      piece_no: string | null;
      quantity: string | number;
    }>(
      await apiCallRaw<unknown>(
        page,
        'GET',
        `/production/outsourcing-orders/items/by-order/${orderId}`
      ),
      'bare',
      '54 发料明细回读'
    );
    const registered = itemRows.find(r => r.id === item.data?.id);
    expect(registered, `by-order 回读应含刚登记明细 id=${item.data?.id}`).toBeTruthy();
    expect(registered!.piece_no, '发料明细匹号回读应与登记一致').toBe(pieceNo);

    // 状态链：draft → issued（发出）。发料事务内 CAS 置匹 AVAILABLE→RESERVED。
    const row = await findTableRow(page, orderNo);
    expect(row, `列表中应存在单号 ${orderNo} 的行，否则无法执行发出`).toBeTruthy();
    await row!.getByRole('button', { name: '发出' }).click();
    await page.locator('.el-message-box__btns .el-button--primary').click();
    await expect(page.locator('.el-message--success').first()).toBeVisible({ timeout: 8000 });
    await expect(row!).toContainText('已发出', { timeout: 5000 });

    // 回读断言（只断 toast/行文本不算证据）：订单状态与匹占用必须真实落库。
    const orderAfter = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/production/outsourcing-orders/${orderId}`
    );
    expect(orderAfter.status, '发出后订单状态应真实落库为 issued').toBe('issued');
    const occupiedRows = pickListArray<PieceRow>(
      await apiCallRaw<unknown>(
        page,
        'GET',
        `/inventory/pieces?piece_no=${encodeURIComponent(pieceNo)}&page=1&page_size=10`
      ),
      'items',
      '54 发料占用回读'
    );
    const occupied = occupiedRows.find(p => p.piece_no === pieceNo);
    expect(occupied, `发出后应能回读生产匹 ${pieceNo}`).toBeTruthy();
    expect(
      String(occupied!.status),
      `生产匹 ${pieceNo} 应被发料占用为 RESERVED（CAS 闭环，实际 ${occupied!.status}）`
    ).toBe('RESERVED');
  });

  test('客户协作：合同签署（contract_id+signed_by_user_id）→ 列表呈现', async ({ page }) => {
    await loginViaUI(page);
    await page.goto(`${BASE_URL}/customer-collab`);
    await expect(page.locator('.page')).toBeVisible();

    await page.getByRole('button', { name: '签署合同' }).click();
    // 作用域限定到对话框：页面另有工具栏「签署合同」按钮，其可访问名包含「签署」子串，
    // getByRole(name:'签署') 默认子串匹配会同时命中「签署合同」+ 对话框「签署」→ strict 违例。
    const dlg = page.locator('.el-dialog:visible').last();
    // SignContractRequest { contract_id, signed_by_user_id, signature_image_url? }
    await dlg.locator('.el-input-number input').first().fill('1');
    await dlg.locator('.el-input-number input').nth(1).fill('1');
    await dlg.getByRole('button', { name: '签署', exact: true }).click();
    await expect(page.locator('.el-message--success, .el-message--error').first()).toBeVisible({
      timeout: 8000,
    });
  });
});
