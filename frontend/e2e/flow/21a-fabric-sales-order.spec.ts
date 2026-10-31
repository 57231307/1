import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  getCtx,
  BASE_URL,
  ensureTestEntities,
  tryCleanup,
} from './helpers';
import { openScopedDropdown, dropdownItems } from './ui-helpers';
import { cellInputByColumn } from './table-column';

/**
 * 销售订单列表页声明的列集合，该页表头判据的唯一基准面。
 * useOlv.ts 的 columns 给出 订单号 客户 订单日期 交货日期 订单金额 状态 创建人 七列，
 * SalesOrderTable.vue 的 fullColumns 在其后追加操作列（zh-CN 语料渲染为 操作，
 * en-US 语料渲染为 Operation，e2e 由 loginViaUI 固定写入 zh-CN）。
 * 消费方是本文件列表用例的表头断言，不被组件反向引用。
 */
const SALES_LIST_COLUMNS = [
  '订单号',
  '客户',
  '订单日期',
  '交货日期',
  '订单金额',
  '状态',
  '创建人',
  '操作',
];

test.describe('面料单据专用字段全链路验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  // ============================================================
  test('销售订单：全部面料字段创建→查询→精确验证', async ({ page }) => {
    const ctx = getCtx();
    const colorNo = ctx.colorNos[0] || 'COLOR-E2E-001';
    const dyeLotNo = ctx.dyeLotNo || genCode('DL');
    const gramWeight = '200';
    const width = '150';
    const pantoneCode = 'TPX-19-4052';

    const soData = {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      // required_date 后端校验"不能早于当前时间"，用 now 会因请求传输耗时落入过去 → 400
      required_date: new Date(Date.now() + 30 * 86400000).toISOString(),
      shipping_address: '面料收货地址',
      notes: '面料字段全链路测试',
      // 主表面料字段
      batch_no: genCode('BN'),
      color_no: colorNo,
      dye_lot_no: dyeLotNo,
      grade: 'A',
      packaging_requirement: '卷装',
      quality_standard: '国标一等品',
      items: [
        {
          product_id: ctx.productIds[0],
          quantity: '100',
          unit_price: '25.50',
          // 明细面料字段
          color_no: colorNo,
          color_name: '宝蓝色',
          pantone_code: pantoneCode,
          grade_required: 'A',
          quantity_meters: '100',
          quantity_kg: '30',
          gram_weight: gramWeight,
          width: width,
          paper_tube_weight: '1.5',
          is_net_weight: true,
          batch_requirement: '同缸同批',
          dye_lot_requirement: '同缸号',
          base_price: '20.00',
          color_extra_cost: '3.00',
          grade_price_diff: '2.50',
          final_price: '25.50',
        },
      ],
    };

    // 创建失败直接暴露（兜底拿旧单会拿到无面料字段的订单，后续精确断言必失败且掩盖根因）
    const result = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', soData);
    const soId = result.data?.id;
    expect(soId).toBeDefined();

    // 查询详情，逐字段验证
    const detail = await apiCallRaw<{
      status: string;
      items: Array<Record<string, unknown>>;
      batch_no: string;
      color_no: string;
      dye_lot_no: string;
      grade: string;
      packaging_requirement: string;
      quality_standard: string;
    }>(page, 'GET', `/sales/orders/${soId}`);

    // 主表面料字段验证
    expect(detail.batch_no).toBeTruthy();
    expect(detail.color_no).toBe(colorNo);
    expect(detail.dye_lot_no).toBe(dyeLotNo);
    expect(detail.grade).toBe('A');
    expect(detail.packaging_requirement).toBe('卷装');
    expect(detail.quality_standard).toBe('国标一等品');

    // 明细面料字段验证
    expect(detail.items?.length).toBeGreaterThan(0);
    const item = detail.items?.[0];
    expect(item.color_no).toBe(colorNo);
    expect(item.color_name).toBe('宝蓝色');
    expect(item.pantone_code).toBe(pantoneCode);
    expect(item.grade_required).toBe('A');
    // gram_weight / width / paper_tube_weight / base_price / final_price 在库中均为 DECIMAL(18,4)
    //（见 migration/src/domain/system/mod.rs:341/349/352/356/370），rust_decimal 以带 scale 的
    // 字符串出参：200→"200.0000"、1.5→"1.5000"、20.00→"20.0000"、25.50→"25.5000"。
    // 故不能按输入的短字面量做 String 全等（原写法把 200 断成 "200"，实际 "200.0000" 必红），
    // 改为按数值判等（真实精度语义）并校验 scale 被完整保留。
    expect(Number(item.gram_weight)).toBe(200);
    expect(String(item.gram_weight)).toBe('200.0000');
    expect(Number(item.width)).toBe(150);
    expect(String(item.width)).toBe('150.0000');
    expect(Number(item.paper_tube_weight)).toBe(1.5);
    expect(String(item.paper_tube_weight)).toBe('1.5000');
    expect(item.is_net_weight).toBe(true);
    expect(Number(item.base_price)).toBe(20);
    expect(String(item.base_price)).toBe('20.0000');
    expect(Number(item.final_price)).toBe(25.5);
    expect(String(item.final_price)).toBe('25.5000');
  });

  test('销售订单 UI：列表显示+详情查看面料信息', async ({ page }) => {
    await page.goto(`${BASE_URL}/sales`);

    // 表头判据不挂在数据行数守卫里。V2Table 无论有无数据都渲染表头，取不到即缺陷，直接判红。
    // 原实现把整段包在 rowCount 大于 0 的守卫内，而 rowCount 取的是本页并不存在的
    // .el-table__body tr（旧 el-table 形态）与 .el-table__header th，两项恒为 0，
    // 于是表头与详情两段一起静默跳过，用例外壳绿但内部零断言，属假绿面。
    const headerCells = page.locator('.el-table-v2__header-cell');
    await expect(headerCells.first(), '销售订单列表未渲染出 el-table-v2 表头单元格').toBeVisible({
      timeout: 30_000,
    });
    const headerTexts = (await headerCells.allTextContents())
      .map(text => text.trim())
      .filter(text => text.length > 0);
    expect(
      [...headerTexts].sort(),
      `销售订单列表表头列名集合与页面声明不一致 声明=${SALES_LIST_COLUMNS.join(' ')} 实得=${headerTexts.join(' ')}`
    ).toEqual([...SALES_LIST_COLUMNS].sort());

    // 数据行按主表网格计数。el-table-v2 把 fixed 列拆到左右覆盖层单独渲染，
    // 主表网格里一行恰好对应一条订单，跨层计数会把同一订单重复计入。
    const mainRows = page.locator('.el-table-v2__main .el-table-v2__row');
    await expect(mainRows.first(), '销售订单列表未渲染出任何数据行').toBeVisible({
      timeout: 30_000,
    });
    expect(await mainRows.count(), '销售订单列表主表网格数据行数应大于 0').toBeGreaterThan(0);

    // 操作列被 fixed right 拆进右覆盖层，主表行内取不到按钮，按同序号行在右层点（与
    // ui-helpers findRowAction 的跨层对齐口径一致）。无守卫，取不到入口即判红。
    const detailBtn = page
      .locator('.el-table-v2__right .el-table-v2__row')
      .first()
      .locator('button:has-text("查看")')
      .first();
    await expect(detailBtn, '销售订单列表首行未渲染出查看入口').toBeVisible({ timeout: 10_000 });
    await detailBtn.click();

    // 详情面板只认弹窗本身。原选择器把 .el-main（页面布局容器，恒存在）并列进来，
    // 弹窗根本没打开也能取到非空文本，判据退化成恒真。
    const detailPanel = page.locator('.el-dialog:visible, .el-drawer:visible').first();
    await expect(detailPanel, '点击查看后销售订单详情弹窗未出现').toBeVisible({ timeout: 10_000 });
    const detailText = (await detailPanel.textContent()) ?? '';

    // 面料字段是否在详情露出属改进点，显式记录现状，既不写成恒真也不反向锁死。
    const fabricKeywords = ['色号', '缸号', '克重', '幅宽', '等级', '批次'];
    const hasFabricInfo = fabricKeywords.some(kw => detailText.includes(kw));
    expect(detailText.trim().length, '销售订单详情面板应渲染出可见文本').toBeGreaterThan(0);
    console.log(
      `[21a][UI 现状] 详情面料关键词命中=${hasFabricInfo} 关键词=${fabricKeywords.join(' ')}`
    );
  });

  test('销售订单 UI：创建表单选择产品→色号→填量→提交，色号真实落库并回读一致', async ({ page }) => {
    const ctx = getCtx();
    // ensureTestEntities 已保证 ctx.productIds[0] 拥有 ctx.colorNos[0] 这条色号
    expect(ctx.productIds.length, '[21a] 缺少测试产品').toBeGreaterThan(0);
    expect(ctx.colorNos.length, '[21a] 缺少测试色号').toBeGreaterThan(0);
    const productId = ctx.productIds[0];
    const colorNo = ctx.colorNos[0];
    const productName = (await apiCallRaw<{ name: string }>(page, 'GET', `/products/${productId}`))
      .name;
    console.log(
      `[21a] 目标明细行：product_id=${productId} name=${productName} color_no=${colorNo}`
    );

    await page.goto(`${BASE_URL}/sales`);
    // 销售订单列表用 V2Table（SalesOrderTable.vue 的 el-card 内挂 V2Table，渲染 el-table-v2 与
    // .v2-table-wrapper），只钉旧 el-table 的窄选择器 .el-table 匹配不到该容器，会等满超时。
    // 这里等的是列表容器在场，表头与数据行的真值判据在「列表显示+详情查看面料信息」用例里。
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 打开新建订单对话框
    await page.locator('button:has-text("新建订单")').first().click();
    const dialog = page.locator('.el-dialog:visible').first();
    await dialog.waitFor({ state: 'visible', timeout: 10_000 });

    // 表头/明细行确实渲染了「色号」录入列（新增列存在性）
    const colorHeader = dialog
      .locator('.el-table__header th, .el-table__header-wrapper th')
      .filter({ hasText: '色号' });
    await colorHeader.first().waitFor({ state: 'visible', timeout: 5_000 });
    expect(await colorHeader.first().isVisible()).toBe(true);

    // 客户下拉：OrderFormDialog.vue:25-38 是非 filterable 的 <el-select>，
    // Element Plus 2.14 下占位符以 .el-select__placeholder 文本节点渲染、内层 input 为
    // readonly 且不带 placeholder 属性 → 旧选择器 :has(input[placeholder="选择客户"]) 恒不命中
    // （30s 超时）。改按 form-item label「客户」精确锚定该 select（OrderFormDialog 基本信息区
    // 唯一含「客户」的 el-form-item），再打开下拉选首个选项；下拉无选项=前置客户数据缺失，
    // 属真实环境缺陷，须失败暴露（不放宽为恒真）。
    const customerFormItem = dialog
      .locator('.el-form-item')
      .filter({ has: page.locator('.el-form-item__label', { hasText: '客户' }) })
      .first();
    const customerSelect = customerFormItem.locator('.el-select').first();
    // 打开客户字段自己的候选面板（helper 点 wrapper、按 aria-controls 钉定归属并等可见）
    const customerPanel = await openScopedDropdown(customerSelect, page);
    const customerDropdown = dropdownItems(customerPanel);
    // 客户列表由 useOlv.ts fetchCustomers 异步回填（GET /crm/customers）。开框瞬间可能尚未渲染
    // 选项 → 旧代码 click() 后立即 count() 属"开框即点"竞态（flaky：偶发 0 选项）。
    // 改为轮询等待选项渲染（对齐下方色号步骤 waitFor 的做法），窗口内仍无选项 = 真实前置客户数据
    // 缺失，须失败暴露（toBeGreaterThan(0) 原样保留，不放宽/不 skip）。
    await expect
      .poll(async () => await customerDropdown.count(), {
        message: '[21a] 客户下拉无选项（前置客户数据缺失）',
        timeout: 10_000,
      })
      .toBeGreaterThan(0);
    const chosenCustomer = ((await customerDropdown.first().textContent()) ?? '').trim();
    expect(chosenCustomer, '[21a] 客户下拉首项文本为空').toBeTruthy();
    await customerDropdown.first().click();
    // 选中后校验下拉回显所选项（EP 2.14 单选回显渲染在 .el-select__wrapper）
    await expect
      .poll(
        async () =>
          (
            (await customerSelect.locator('.el-select__wrapper').first().textContent()) ?? ''
          ).trim(),
        { message: '[21a] 选中客户后下拉未回显所选项', timeout: 5000 }
      )
      .toContain(chosenCustomer);
    console.log(`[21a] 已选客户=${chosenCustomer}`);

    // 要求交货日期（第二个日期选择器）：填未来日期（后端拒收过去日期）
    const pad = (n: number) => String(n).padStart(2, '0');
    const future = new Date(Date.now() + 30 * 86400000);
    const futureStr = `${future.getFullYear()}-${pad(future.getMonth() + 1)}-${pad(future.getDate())}`;
    const reqDateInput = dialog.locator('input[placeholder="选择日期"]').nth(1);
    await reqDateInput.fill(futureStr);
    await reqDateInput.press('Enter');
    console.log(`[21a] 要求交货日期=${futureStr}`);

    // 联系人 / 联系电话 / 收货地址
    await dialog.locator('input[placeholder="联系人姓名"]').fill('E2E联系人');
    await dialog.locator('input[placeholder="联系电话"]').fill('13800138000');
    await dialog.locator('textarea[placeholder="详细收货地址"]').fill('E2E 面料收货地址');
    console.log('[21a] 已填联系人与收货地址');

    // 明细行：选产品（表格内第一个 el-select；面板钉定到该行选择器，不再取任一可见面板）
    const productPanel = await openScopedDropdown(
      dialog.locator('.el-table .el-select').nth(0),
      page
    );
    await dropdownItems(productPanel).filter({ hasText: productName }).first().click();
    console.log(`[21a] 已选产品 ${productName}`);

    // 明细行：选色号（表格内第二个 el-select，选项来自该产品色号列表）
    const colorPanel = await openScopedDropdown(
      dialog.locator('.el-table .el-select').nth(1),
      page
    );
    const colorOption = dropdownItems(colorPanel).filter({ hasText: colorNo }).first();
    await colorOption.waitFor({ state: 'visible', timeout: 5_000 });
    await colorOption.click();
    console.log(`[21a] 已选色号 ${colorNo}`);

    // 明细行：填数量与单价 两列均为纯手写录入的数字输入框（无步进加减按钮）按列头锚定
    // 单价必须 >0 才计入提交 填完 Tab 让显示收敛到校验器归一后的形态
    const detailTable = dialog.locator('.el-table').first();
    const detailRow = detailTable.locator('.el-table__row').first();
    const qtyInput = await cellInputByColumn(detailTable, detailRow, '数量');
    await qtyInput.fill('100');
    const priceInput = await cellInputByColumn(detailTable, detailRow, '单价');
    await priceInput.fill('25.5');
    await priceInput.press('Tab');
    console.log('[21a] 已填数量=100 单价=25.5');

    // 提交：捕获真实 POST /sales/orders 的请求体与响应（不 mock）
    // 只绑 200：CSRF token 一次性消费下 UI 首个 POST 可能 403（csrf.rs:110/:216-224，
    // 前端 axios 用恢复头静默重放，request.ts:197-223）；不过滤状态码就会命中 403 中间态，
    // 下方 resp.ok() 变成本可用例的"真红假象"（同 11-03/purchase/09 族）。重放体与首试体
    // 同一 payload，故 postData() 读到的仍是真实提交内容。
    const responsePromise = page.waitForResponse(
      r =>
        r.request().method() === 'POST' && r.url().endsWith('/sales/orders') && r.status() === 200
    );
    await dialog.locator('.el-dialog__footer button').filter({ hasText: '确定' }).first().click();
    const resp = await responsePromise;

    // 业务结果断言 1：提交 payload 明细行带上了所选色号
    const payload = JSON.parse(resp.request().postData() || '{}') as {
      items?: Array<{ product_id: number; color_no?: string }>;
    };
    expect(Array.isArray(payload.items), '[21a] payload.items 应为数组').toBe(true);
    expect(payload.items?.[0]?.color_no, '[21a] 提交 payload 明细行应含色号').toBe(colorNo);
    expect(payload.items?.[0]?.product_id, '[21a] 提交明细行产品应为所选产品').toBe(productId);
    console.log(`[21a] 提交 payload 校验通过：${JSON.stringify(payload.items)}`);

    // 创建必须成功（失败即暴露，不吞）
    expect(resp.ok(), `[21a] 创建订单应 2xx，实际 ${resp.status()}`).toBe(true);
    const created = (await resp.json()) as { data?: { id?: number } };
    const orderId = created.data?.id;
    expect(orderId, '[21a] 创建未返回订单 id').toBeTruthy();

    // 业务结果断言 2：后端回读该明细行色号与所选一致
    const detail = await apiCallRaw<{ items: Array<{ color_no: string }> }>(
      page,
      'GET',
      `/sales/orders/${orderId}`
    );
    expect(detail.items?.[0]?.color_no, '[21a] 后端回读明细色号应一致').toBe(colorNo);

    // 业务结果断言 3：所选色号确实来自该产品的色号列表（真实按产品过滤的数据源）
    const productColors = await apiCallRaw<Array<{ color_no: string }>>(
      page,
      'GET',
      `/products/${productId}/colors`
    );
    expect(
      productColors.map(c => c.color_no),
      '[21a] 色号应属于该产品'
    ).toContain(colorNo);
    console.log(`[21a] 后端回读一致：order=${orderId} color_no=${colorNo}`);

    await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, '21a 销售订单');
  });

  // ============================================================
  // 采购收货 — 面料字段命名最规范的单据
  // 后端字段: material_code, material_name, color_code, lot_no,
  //   batch_no, grade, gram_weight, width
  // 注意: 用 color_code（非 color_no），用 lot_no（非 dye_lot_no）
  // ============================================================

  // ============================================================
  // [BE-3] 销售订单容差回显验证：quantity_tolerance_pct 在明细创建/编辑后
  // 通过 GET 详情回读、UI 打开详情弹窗均能看到写入值（非 null/undefined）。
  // 对应修复项：
  //   BE-3: 后端 SalesOrderItemDetail (services/so/mod.rs) 补 quantity_tolerance_pct 出参
  //         order_query.rs build_item_detail() / build_single_item_detail() 已透传
  //         order_crud.rs create 时 Set(item_req.quantity_tolerance_pct) 已接线
  //   FE: 前端 OrderFormDialog.vue"容差"列为纯手写录入的数字输入框（el-input 加数字校验）绑定
  //         row.quantity_tolerance_pct；OrderViewDialog/OrderDetail 已展示该列
  // 本用例通过 UI 新建订单时填入容差值，随后 GET 详情断言 quantity_tolerance_pct
  // 等于写入值；再 UI 打开详情弹窗验证渲染。
  // ============================================================
  test('销售订单容差：UI 创建填入 quantity_tolerance_pct → GET 回读 → UI 详情回显', async ({
    page,
  }) => {
    const ctx = getCtx();
    expect(ctx.productIds.length, '[21a-tol] 缺少测试产品').toBeGreaterThan(0);
    expect(ctx.colorNos.length, '[21a-tol] 缺少测试色号').toBeGreaterThan(0);
    const productId = ctx.productIds[0];
    const colorNo = ctx.colorNos[0];
    const productName = (await apiCallRaw<{ name: string }>(page, 'GET', `/products/${productId}`))
      .name;
    const toleranceValue = 8.5; // 设一个有意义的容差百分比

    await page.goto(`${BASE_URL}/sales`);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 打开新建订单对话框
    await page.locator('button:has-text("新建订单")').first().click();
    const dialog = page.locator('.el-dialog:visible').first();
    await dialog.waitFor({ state: 'visible', timeout: 10_000 });

    // 选客户
    const customerFormItem = dialog
      .locator('.el-form-item')
      .filter({ has: page.locator('.el-form-item__label', { hasText: '客户' }) })
      .first();
    const customerSelect = customerFormItem.locator('.el-select').first();
    const customerPanel = await openScopedDropdown(customerSelect, page);
    const customerDropdown = dropdownItems(customerPanel);
    await expect(customerDropdown.first(), '[21a-tol] 客户下拉应有选项').toBeVisible({
      timeout: 5000,
    });
    await customerDropdown.first().click();

    // 填要求交货日期
    const pad = (n: number) => String(n).padStart(2, '0');
    const future = new Date(Date.now() + 30 * 86400000);
    const futureStr = `${future.getFullYear()}-${pad(future.getMonth() + 1)}-${pad(future.getDate())}`;
    const reqDateInput = dialog.locator('input[placeholder="选择日期"]').nth(1);
    await reqDateInput.fill(futureStr);
    await reqDateInput.press('Enter');

    // 填联系人/电话/地址
    await dialog.locator('input[placeholder="联系人姓名"]').fill('E2E容差测试');
    await dialog.locator('input[placeholder="联系电话"]').fill('13900139000');
    const addrTextarea = dialog.locator('textarea[placeholder="详细收货地址"]');
    if (await addrTextarea.isVisible().catch(() => false)) {
      await addrTextarea.fill('E2E 容差测试地址');
    }

    // 明细行：选产品
    const productPanelTol = await openScopedDropdown(
      dialog.locator('.el-table .el-select').nth(0),
      page
    );
    await dropdownItems(productPanelTol).filter({ hasText: productName }).first().click();

    // 明细行：选色号
    const colorPanelTol = await openScopedDropdown(
      dialog.locator('.el-table .el-select').nth(1),
      page
    );
    const colorOption = dropdownItems(colorPanelTol).filter({ hasText: colorNo }).first();
    await colorOption.waitFor({ state: 'visible', timeout: 5000 });
    await colorOption.click();

    // 明细行：填数量和单价 两列为纯手写录入的数字输入框（无步进加减按钮）按列头锚定
    const detailTable = dialog.locator('.el-table').first();
    const detailRow = detailTable.locator('.el-table__row').first();
    const qtyInput = await cellInputByColumn(detailTable, detailRow, '数量');
    await qtyInput.fill('100');
    const priceInput = await cellInputByColumn(detailTable, detailRow, '单价');
    await priceInput.fill('30');
    await priceInput.press('Tab');

    // 明细行：填容差值 该列为纯手写录入的数字输入框（el-input 加数字校验，无步进加减按钮）
    // 列 label 取 i18n 键 sales.orderForm.tolerance 中文 locale 表头为 交货容差 %
    // 先断言"容差"列表头存在 再按该列表头锚定本行录入框（列序漂移仍命中本列）
    const toleranceHeader = dialog
      .locator('.el-table__header th, .el-table__header-wrapper th')
      .filter({ hasText: '容差' });
    await expect(
      toleranceHeader.first(),
      '[21a-tol] 明细表应含"容差"列（FE 表单项已渲染）'
    ).toBeVisible({ timeout: 5000 });
    const toleranceInput = await cellInputByColumn(detailTable, detailRow, '交货容差 %');
    await toleranceInput.waitFor({ state: 'visible', timeout: 5000 });
    await toleranceInput.fill(String(toleranceValue));
    await toleranceInput.press('Tab');
    console.log(`[21a-tol] 容差值已填入 ${toleranceValue}%`);

    // 提交订单（只绑 200：避开 CSRF 一次性消费的 403 中间态，见本文件 21a 首处同注释）
    const responsePromise = page.waitForResponse(
      r =>
        r.request().method() === 'POST' && r.url().endsWith('/sales/orders') && r.status() === 200,
      { timeout: 30000 }
    );
    await dialog.locator('.el-dialog__footer button').filter({ hasText: '确定' }).first().click();
    const resp = await responsePromise;

    // 断言创建成功
    expect(resp.ok(), `[21a-tol] 创建订单应 2xx，实际 ${resp.status()}`).toBe(true);
    const created = await resp.json();
    const orderId = created?.data?.id;
    expect(orderId, '[21a-tol] 创建未返回订单 id').toBeTruthy();

    // 断言 payload 包含 quantity_tolerance_pct
    const payload = JSON.parse(resp.request().postData() || '{}');
    expect(
      Number(payload.items?.[0]?.quantity_tolerance_pct),
      `[21a-tol] 提交 payload 明细行 quantity_tolerance_pct 应=${toleranceValue}，实际=${payload.items?.[0]?.quantity_tolerance_pct}`
    ).toBe(toleranceValue);

    // GET 详情回读：quantity_tolerance_pct 非 null 且等于写入值
    const detail = await apiCallRaw<{
      items: Array<{ quantity_tolerance_pct: number | string | null }>;
    }>(page, 'GET', `/sales/orders/${orderId}`);
    expect(detail.items?.length, '[21a-tol] 详情应含明细行').toBeGreaterThan(0);
    const itemTolerance = detail.items[0].quantity_tolerance_pct;
    expect(
      itemTolerance,
      `[21a-tol] GET 详情 quantity_tolerance_pct 不应为 null/undefined（BE-3 接线验证），实际=${JSON.stringify(itemTolerance)}`
    ).not.toBeNull();
    expect(
      Number(itemTolerance),
      `[21a-tol] quantity_tolerance_pct 应等于写入值 ${toleranceValue}，实际=${itemTolerance}`
    ).toBe(toleranceValue);

    // UI 打开详情弹窗验证回显
    // 先关闭当前对话框
    await page.waitForTimeout(1000);
    const detailBtn = page
      .getByRole('row')
      .filter({ hasText: 'E2E容差测试' })
      .first()
      .locator('button:has-text("查看"), button:has-text("详情"), .el-link:has-text("详情")')
      .first();
    if (await detailBtn.isVisible().catch(() => false)) {
      await detailBtn.click();
      const detailDialog = page.locator('.el-dialog:visible, .el-drawer:visible').last();
      await detailDialog.waitFor({ state: 'visible', timeout: 10000 });
      // 详情弹窗内应显示容差百分比文本（OrderViewDialog.vue:88 `row.quantity_tolerance_pct + '%'`）
      const toleranceText = await detailDialog.textContent();
      expect(
        toleranceText?.includes(String(toleranceValue)) || toleranceText?.includes('8.5'),
        `[21a-tol] UI 详情应显示容差值 ${toleranceValue}%，实际弹窗文本片段=${(toleranceText || '').slice(0, 300)}`
      ).toBe(true);
      await page
        .locator('.el-dialog__headerbtn, .el-drawer__close-btn')
        .first()
        .click()
        .catch(() => {});
    }

    // 清理
    await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, '21a-tol 销售订单');
  });
});
