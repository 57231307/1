import { test, expect, type Locator, type Page } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCallRaw,
  apiCallExpectFail,
  BASE_URL,
  verifyEndpointHealthy,
  listNotifications,
  tryCleanup,
  failureCode,
  APP_ERROR_CODES,
} from './helpers';
import { formItemByExactLabel } from './ui-helpers';
import {
  columnCellTexts,
  columnIndex,
  openPageTable,
  rowByText,
  rowCellText,
  rowCount,
} from './table-column';

// 审计日志页的表格是 V2Table 包装的 el-table-v2（虚拟表），列头节点是 header-cell、
// 单元格是 row-cell，与共享原语 table-column 所覆盖的标准 el-table 的 th 与 td 不同源。
// 直接套用标准表原语会让行选择器恒空，断言会退化成"什么都没断到却通过"，
// 因此这里按同一口径实现虚拟表版本：列下标一律由列头文案实时求得，列取不到即抛错并附实际列头。
// 本页的操作列是 fixed 右侧列，el-table-v2 把 fixed 列拆到独立覆盖层渲染，
// 主层（类名 el-table-v2__main）的行只含非 fixed 列，故表头与单元格都限定在主层内取，二者下标天然对齐。
// Element Plus 升级若改变表头 DOM 结构，本组选择器会失效并判红，属预期的锁失效，交人工复核判据。
async function v2MainGrid(page: Page): Promise<Locator> {
  const grid = page.locator('.el-table-v2__main').first();
  await grid.waitFor({ state: 'visible', timeout: 30_000 });
  return grid;
}

async function v2HeaderTexts(grid: Locator): Promise<string[]> {
  return (await grid.locator('.el-table-v2__header-cell').allInnerTexts()).map(text => text.trim());
}

async function v2ColumnIndex(grid: Locator, headerText: string): Promise<number> {
  const texts = await v2HeaderTexts(grid);
  const index = texts.indexOf(headerText);
  if (index < 0) {
    throw new Error(`虚拟表列头 ${headerText} 未找到，实际列头=${JSON.stringify(texts)}`);
  }
  return index;
}

async function v2RowCellText(row: Locator, index: number): Promise<string> {
  return (await row.locator('> .el-table-v2__row-cell').nth(index).innerText()).trim();
}

test.describe('系统与分析模块全量：API 端点 + 真实 UI 交互', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // ===== API 端点覆盖 =====
  test('BI+Webhook+API网关+邮件+通知+扫码+导入导出+AI+报表+高级分析+跟踪+隐私+双计量+权限+审计+产品分类', async ({
    page,
  }) => {
    // BI 多维分析：后端 bi()(analytics.rs:488，nest 到 /bi) 仅注册 /bi/sales/* 一组聚合端点。
    // 旧探针 /bi/{sales,product,customer,inventory,finance,production}-analysis 与 /bi/summary
    // 全是臆造路径（后端无 GET /bi/sales-analysis、无 /bi/summary、亦无 product/customer/
    // inventory/finance/production 各域的 BI 端点），optional 长期吞其 404 = 假绿。
    // 改为真实存在路径并 strict（admin 无必填参或带合法参，应 2xx）；无真实等价物的臆造分析
    // 探针（上述 5 个非 sales 域 + summary）删除——测试对着不存在端点写属测试错，不保留 optional 掩盖。
    // by-time 的 start/end/granularity 为 handler 必填(缺参 400)，用当前年度合法日期区间作入参
    //（区间聚合、空数据仍返回 200，非实体 id 查询不会 404）；其余 7 端点无必填参。
    const biYear = new Date().getFullYear();
    await verifyEndpointHealthy(
      page,
      `/bi/sales/by-time?start_date=${biYear}-01-01&end_date=${biYear}-12-31&granularity=month`
    );
    await verifyEndpointHealthy(page, '/bi/sales/by-customer');
    await verifyEndpointHealthy(page, '/bi/sales/by-product');
    await verifyEndpointHealthy(page, '/bi/sales/by-region');
    await verifyEndpointHealthy(page, '/bi/sales/by-category');
    await verifyEndpointHealthy(page, '/bi/sales/trend');
    await verifyEndpointHealthy(page, '/bi/sales/profit');
    await verifyEndpointHealthy(page, '/bi/sales/kpi');
    // Webhook（webhooks() 在 analytics.rs:354 注册 GET /，nest 到 /webhooks → strict）
    await verifyEndpointHealthy(page, '/webhooks?page=1&page_size=5');
    // Webhook 集成（analytics.rs:611 nest /webhooks/integrations + 内部 GET /（analytics.rs:283）
    // → 最终 GET /webhooks/integrations；list_integrations（webhook_integration_handler.rs:85-88）
    // 无 Query 入参、仅 AuthContext，admin 必 2xx → strict，不再 optional 吞 404）
    await verifyEndpointHealthy(page, '/webhooks/integrations?page=1&page_size=5');
    // API 网关：endpoints 列表为真实消费面（dynamic_router 按状态门控）
    await verifyEndpointHealthy(page, '/api-gateway/endpoints?page=1&page_size=5');
    // 邮件+通知
    await verifyEndpointHealthy(page, '/email-templates?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/notifications?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/user-notification-settings');
    // 扫码+搜索（scanner/history 在 analytics.rs:108 注册，nest 到 /scanner → strict）
    await verifyEndpointHealthy(page, '/scanner/history?page=1&page_size=5');
    // 搜索（GET /search 根路径后端未注册，只注册 /search/{sales-orders,customers,products,doc-types}
    // （routes/mod.rs:405-409）；旧探针 /search?q=面料 打裸根路径必 404（臆造），optional 长期吞之=假绿。
    // 改为真实注册的 /search/products，query 参数名 q 按 handler 核实（search_api.rs:24-32 SearchParams
    // .q: Option<String>；CI 未配 ELASTICSEARCH_URL 时 search_client 走 mock 内存（container/mod.rs:439-448），
    // 空命中仍 2xx）→ strict）
    await verifyEndpointHealthy(page, '/search/products?q=面料');
    // AI（B 类：端点存在但旧探针路径写成单数/根路径，属测试侧缺陷，已改真实路径并迁 strict。
    // 工艺优化真实为复数 /ai/process-optimizations（routes/system.rs:362，system::routes()
    // 于 mod.rs:419 merge 到 /api/v1/erp 根，无额外前缀；list_process_optimizations 入参
    // ListProcessOptQuery 全 Option（ai_extend_service.rs:55-67）→ admin 必 2xx）；
    // 质量预测真实为复数 /ai/quality-predictions（system.rs:395；ListQualityPredQuery 全
    // Option（ai_extend_service.rs:102-110）→ admin 必 2xx））
    await verifyEndpointHealthy(page, '/ai/process-optimizations');
    await verifyEndpointHealthy(page, '/ai/quality-predictions');
    // AI 模型管理：GET /ai-models 根路径未注册（routes/ai_model.rs:11-61 仅有子路径，
    // nest 前缀见 mod.rs:486）；真实 GET 子路径为 /ai-models/versions
    // （ai_model.rs:17-20 → list_model_versions，入参 ModelNameQuery.model_name 为
    // Option（ai_model_management_handler.rs:21-23），无 model_name 也返回 2xx 全量列表）
    // → 改打真实子路径并 strict；旧探针携带的 page/page_size 该 handler 不支持，去除）
    await verifyEndpointHealthy(page, '/ai-models/versions');
    // 报表（A 类：/report-templates 已注册（analytics.rs:336-341 reports() 经 605 merge
    // 到 /api/v1/erp 根）；list_templates（report_engine_handler.rs:55-65）无 Query 入参，
    // 返回内置预定义模板、不依赖 seed → admin 必 2xx，迁 strict）
    await verifyEndpointHealthy(page, '/report-templates?page=1&page_size=5');
    // 报表增强：reports_enhanced()(analytics.rs:122，nest 到 /reports/enhanced) 无 GET / 根，
    // 旧探针 /reports/enhanced 打裸路径必 404（臆造）。真实 list 端点为 /reports/enhanced/templates
    // （report_enhanced_handler::list_report_templates，Query 字段全 Option）→ 改为真实路径并 strict。
    await verifyEndpointHealthy(page, '/reports/enhanced/templates?page=1&page_size=5');
    // 高级分析+跟踪+隐私（C 类裁定：/advanced/analysis 为用例臆造路径——advanced 域
    // （analytics.rs:468-484，nest 到 /advanced 见 617）仅注册 POST /advanced/ai/* 与
    // GET /advanced/reports/templates、POST /advanced/reports/{execute,export}，不存在任何
    // GET /advanced/analysis。该端点未注册，属"缺 GET 分析端点"的功能缺口还是纯臆造无法从
    // 测试侧定性，且对着不存在端点保留恒不判红的 optional 只是掩盖，故删除该探针，
    // 不为凑数保留假绿）
    await verifyEndpointHealthy(page, '/page-view/stats');
    await verifyEndpointHealthy(page, '/privacy/consents');
    // 权限+审计
    await verifyEndpointHealthy(page, '/data-permissions?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/users?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/roles?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/departments?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/audit-logs?page=1&page_size=5');
    // 慢查询审计（B 类：真实路径无 /system 前缀——system.rs:302 注册 GET /slow-queries，
    // system::routes() 经 mod.rs:419 merge 到 /api/v1/erp 根；list_slow_queries 入参
    // SlowQueryListParams 全 Option（slow_query_handler.rs:33-46，page/page_size 缺省
    // 防御式处理 :90-91）→ 旧探针 /system/slow-queries 必 404 属测试侧路径错误，
    // 改正路径后迁 strict）
    await verifyEndpointHealthy(page, '/slow-queries?page=1&page_size=5');
    // 产品分类+仓库
    await verifyEndpointHealthy(page, '/product-categories?page=1&page_size=50');
    await verifyEndpointHealthy(page, '/warehouses?page=1&page_size=5');
  });

  // ===== 真实 UI 交互验证 =====
  test('审计日志 UI：关键字筛到造出的行并逐列按后端真值出值', async ({ page }) => {
    // 造数走部门创建：后端在部门创建事件里写入审计记录，resource_name 即部门名、resource_id 即新主键。
    // 用带时间戳的唯一部门名把这一行筛出来，避免在并发数据里翻分页，也不依赖部门页的内部结构。
    const stamp = String(Date.now()).slice(-6);
    const deptName = `审计回归部门${stamp}`;
    const dept = await apiCallRaw<{ id: number }>(page, 'POST', '/departments', {
      name: deptName,
      code: `e2eaud${stamp}`,
    });
    expect(Number.isFinite(dept.id), `建部门应返回 id，实际：${JSON.stringify(dept)}`).toBe(true);

    try {
      // 审计为异步落库，先确认后端列表已能按关键字取到该行，再验 UI 出值（取不到即判红，不回落成可见性）
      await expect
        .poll(
          async () => {
            const list = await apiCallRaw<{
              items: Array<{ resource_name?: string | null }>;
            }>(
              page,
              'GET',
              `/audit-logs?page=1&page_size=5&keyword=${encodeURIComponent(deptName)}`
            );
            return list.items.filter(item => item.resource_name === deptName).length;
          },
          {
            timeout: 30_000,
            message: `后端审计列表按关键字取不到 resource_name 为 ${deptName} 的记录`,
          }
        )
        .toBeGreaterThan(0);

      await page.goto(`${BASE_URL}/system/audit-log`);
      const grid = await v2MainGrid(page);

      // 表头文案齐全是本页列定义在位的判据，资源名称列是这一波新加的那一列
      const headers = await v2HeaderTexts(grid);
      for (const col of ['资源名称', '资源类型', '操作类型', '资源 ID', '操作人']) {
        expect(headers, `表头缺少列 ${col}，实际表头=${JSON.stringify(headers)}`).toContain(col);
      }

      // 按关键字筛选到自建行（label 锚定，不用位置选择器）
      const keywordInput = formItemByExactLabel(page, '关键字').locator('input').first();
      await keywordInput.waitFor({ state: 'visible', timeout: 30_000 });
      await keywordInput.fill(deptName);
      await page.getByRole('button', { name: '查询', exact: true }).click();

      const row = grid.locator('.el-table-v2__row').filter({ hasText: deptName }).first();
      await row.waitFor({ state: 'visible', timeout: 30_000 });

      const nameIdx = await v2ColumnIndex(grid, '资源名称');
      expect(await v2RowCellText(row, nameIdx), '资源名称列未渲染 resource_name').toBe(deptName);
      const typeIdx = await v2ColumnIndex(grid, '资源类型');
      expect(await v2RowCellText(row, typeIdx), '资源类型列未渲染 resource_type').toBe(
        'department'
      );
      const opIdx = await v2ColumnIndex(grid, '操作类型');
      expect(await v2RowCellText(row, opIdx), '操作类型列未按 CREATE 渲染文案').toBe('新建');
      const ridIdx = await v2ColumnIndex(grid, '资源 ID');
      expect(await v2RowCellText(row, ridIdx), '资源 ID 列未渲染 resource_id').toBe(
        String(dept.id)
      );
      const operatorIdx = await v2ColumnIndex(grid, '操作人');
      expect(await v2RowCellText(row, operatorIdx), '操作人列为空（username 未渲染）').not.toBe('');
    } finally {
      await tryCleanup(page, 'DELETE', `/departments/${dept.id}`, `部门#${dept.id}`);
    }
  });

  test('API 网关 UI：页标题与两个页签文案在位，新建接口对话框字段齐全', async ({ page }) => {
    await page.goto(`${BASE_URL}/api-gateway`);
    await expect(
      page.getByRole('heading', { level: 2, name: 'API 网关' }),
      '未渲染 API 网关页标题'
    ).toBeVisible({ timeout: 30_000 });

    // 页签按文案逐条判定（原写法只断页签数大于 0，任一无关页签都能凑绿）
    for (const label of ['接口管理', '网关统计']) {
      await expect(
        page.locator('.el-tabs__item', { hasText: label }),
        `页签缺少 ${label}`
      ).toHaveCount(1);
    }

    const newBtn = page.getByRole('button', { name: '新建接口', exact: true });
    await expect(newBtn, '默认页签未渲染新建接口入口').toBeVisible({ timeout: 30_000 });
    await newBtn.click();

    const dialog = page.locator('.el-dialog:visible').first();
    await dialog.waitFor({ state: 'visible', timeout: 15_000 });
    await expect(dialog.locator('.el-dialog__title'), '对话框标题不是新建接口').toHaveText(
      '新建接口'
    );
    // 字段按精确 label 出位，空壳对话框在这里判红
    await expect(formItemByExactLabel(dialog, '接口路径'), '缺少接口路径字段').toBeVisible();
    await expect(formItemByExactLabel(dialog, '请求方法'), '缺少请求方法字段').toBeVisible();
    await expect(formItemByExactLabel(dialog, '描述'), '缺少描述字段').toBeVisible();
    await page.keyboard.press('Escape');
  });

  test('通知中心 UI：列表出真实公告且标记已读让未读态真实翻转', async ({ page }) => {
    // 判据从"body 可见"升级为：该页自身标题文案 + 一条 API 造数出的真实公告 +
    // 点击已读后未读样式与后端未读集合双双变化（状态确实翻转，不是只断按钮在位）
    const me = await apiCallRaw<{ id?: number }>(page, 'GET', '/auth/me');
    const currentUserId = me?.id;
    expect(currentUserId, `/auth/me 未返回数值 id，实际=${JSON.stringify(me)}`).toBeTruthy();

    const stamp = String(Date.now()).slice(-6);
    const title = `26已读回归${stamp}`;
    const sent = await apiCallRaw<{ delivered_count?: number }>(
      page,
      'POST',
      '/notifications/announcement',
      { user_ids: [currentUserId], title, content: `已读回归内容 ${stamp}` }
    );
    expect(sent?.delivered_count, '公告应投递给 1 位目标用户').toBe(1);
    // 公告由事件监听器异步落库，给写入留窗口
    await expect
      .poll(async () => (await listNotifications(page)).some(n => n.title === title), {
        timeout: 30_000,
        message: `造数后未读列表始终查不到标题为 ${title} 的公告`,
      })
      .toBe(true);

    await page.goto(`${BASE_URL}/notification`);
    await expect(
      page.locator('.notification .card-header').getByText('通知中心', { exact: true }),
      '未渲染通知中心页面标题文案'
    ).toBeVisible({ timeout: 30_000 });

    const item = page.locator('.notification-item').filter({ hasText: title }).first();
    await item.waitFor({ state: 'visible', timeout: 30_000 });
    await expect(item, '新公告应按未读态渲染').toHaveClass(/unread/);
    await expect(item.locator('.unread-dot'), '未读红点未渲染').toHaveCount(1);
    const markBtn = item.getByRole('button', { name: '标记已读' });
    await expect(markBtn, '未读条目应提供标记已读入口').toBeVisible();

    await markBtn.click();
    await expect(item, '点击后该行应脱离未读样式').not.toHaveClass(/unread/);
    await expect(item.locator('.unread-dot'), '点击后未读红点应消失').toHaveCount(0);
    await expect(
      item.getByRole('button', { name: '标记已读' }),
      '已读后单条标记入口应不再渲染'
    ).toHaveCount(0);
    await expect(
      page.locator('.el-message').filter({ hasText: '标记已读成功' }).first(),
      '未给出标记已读成功提示'
    ).toBeVisible({ timeout: 10_000 });

    const unreadAfter = await listNotifications(page);
    expect(
      unreadAfter.some(n => n.title === title),
      '标记已读后后端未读集合仍含该条，状态未真实落库'
    ).toBe(false);

    // 同一条应出现在已读集合（状态确实翻成 READ 而非被删掉），并用它做清理
    const readAfter = await listNotifications(page, 'READ');
    const target = readAfter.find(n => n.title === title);
    expect(target, `已读集合中找不到标题为 ${title} 的公告`).toBeTruthy();
    await tryCleanup(page, 'DELETE', `/notifications/${target!.id}`, `通知#${target!.id}`);
  });

  test('数据权限 UI：按角色视图出真实角色名与逐列出值，新增权限入口在位', async ({ page }) => {
    const roles = await apiCallRaw<{ roles: Array<{ id: number; name: string }> }>(
      page,
      'GET',
      '/roles'
    );
    expect(roles.roles.length, '无角色可用，无法验证按角色视图').toBeGreaterThan(0);
    const role = roles.roles[0];
    await apiCallRaw(page, 'POST', '/data-permissions', {
      role_id: role.id,
      resource_type: 'customer',
      scope_type: 'DEPT',
    });

    await page.goto(`${BASE_URL}/data-permission`);
    await expect(
      page.locator('.data-permission .card-header').getByText('数据权限管理', { exact: true }),
      '未渲染数据权限页面标题文案'
    ).toBeVisible({ timeout: 30_000 });

    const roleEntry = page.getByRole('menuitem', { name: role.name, exact: true });
    await roleEntry.waitFor({ state: 'visible', timeout: 30_000 });
    await roleEntry.click();
    await expect(
      page.getByRole('heading', { level: 3, name: `${role.name} - 权限设置` }),
      `选中角色后面板标题未回显真实角色名 ${role.name}`
    ).toBeVisible({ timeout: 30_000 });

    const table = page.locator('.el-table[aria-label="数据权限列表"]').first();
    await table.waitFor({ state: 'visible', timeout: 30_000 });
    expect(await rowCount(table), '按角色视图列表应为正行（造数后仍空即取数缺陷）').toBeGreaterThan(
      0
    );

    const resourceIdx = await columnIndex(table, '资源类型');
    const scopeIdx = await columnIndex(table, '权限范围');
    const createdIdx = await columnIndex(table, '创建时间');
    const targetRow = await rowByText(table, 'customer');
    expect(await rowCellText(targetRow, resourceIdx), '资源类型列未渲染 resource_type').toBe(
      'customer'
    );
    expect(await rowCellText(targetRow, scopeIdx), '权限范围列未按 scope_type=DEPT 渲染').toBe(
      '本部门'
    );
    expect(await rowCellText(targetRow, createdIdx), '创建时间列未渲染 created_at').not.toBe('');

    // 写面入口只在按角色视图且 admin 会话下出现（概览视图收起的判据见 88 拆分回归用例）
    await expect(
      page.getByRole('button', { name: '新增权限', exact: true }),
      '按角色视图下未渲染新增权限入口'
    ).toBeVisible();
  });

  test('部门管理 UI：列表出真实部门与编码，新建对话框标题与字段齐全', async ({ page }) => {
    const stamp = String(Date.now()).slice(-6);
    const name = `回归部门${stamp}`;
    const code = `e2edep${stamp}`;
    const created = await apiCallRaw<{ id: number }>(page, 'POST', '/departments', {
      name,
      code,
    });
    expect(Number.isFinite(created.id), `建部门应返回 id，实际：${JSON.stringify(created)}`).toBe(
      true
    );
    try {
      const table = await openPageTable(page, '/departments', '部门管理', '部门列表');
      const nameIdx = await columnIndex(table, '部门名称');
      const codeIdx = await columnIndex(table, '部门编码');
      const statusIdx = await columnIndex(table, '状态');
      const targetRow = await rowByText(table, name);
      expect(await rowCellText(targetRow, nameIdx), '部门名称列未渲染 name').toContain(name);
      expect(await rowCellText(targetRow, codeIdx), '部门编码列未渲染 code').toBe(code);
      const statusCell = await rowCellText(targetRow, statusIdx);
      expect(['启用', '禁用'], `状态列非预期取值: ${statusCell}`).toContain(statusCell);

      const newBtn = page.getByRole('button', { name: '新建部门', exact: true });
      await expect(newBtn, '未渲染新建部门按钮').toBeVisible();
      await newBtn.click();
      const dialog = page.locator('.el-dialog:visible').first();
      await dialog.waitFor({ state: 'visible', timeout: 10_000 });
      await expect(dialog.locator('.el-dialog__title'), '对话框标题不是新建部门').toHaveText(
        '新建部门'
      );
      // 表单字段按真实 label 出位，空壳弹窗在这里判红
      await expect(formItemByExactLabel(dialog, '部门名称')).toBeVisible();
      await expect(formItemByExactLabel(dialog, '部门编码')).toBeVisible();
      await expect(formItemByExactLabel(dialog, '状态')).toBeVisible();
      await page.keyboard.press('Escape');
    } finally {
      await tryCleanup(page, 'DELETE', `/departments/${created.id}`, `部门#${created.id}`);
    }
  });

  test('业务追溯 UI：页签与查询卡在位，缺参被前端拦截且查不到的 ID 按真实契约报错', async ({
    page,
  }) => {
    // 追溯链出值需要一条完整染色链路种子（坯布入库到染色到出库），本仓 e2e 尚无该种子，
    // 故本条不声称覆盖"追溯链数据出值"，只锁三件可判红的事实：
    // 页面自身页签与卡片文案、缺参时前端拦截不发请求、不存在的五维 ID 按后端真实契约报 404。
    // 追溯链数据出值仍属未覆盖风险，已交编排方裁定（补种子或归入 fullflow 波次）。
    const traceRequests: string[] = [];
    page.on('request', request => {
      if (request.url().includes('/business-trace/')) traceRequests.push(request.url());
    });

    await page.goto(`${BASE_URL}/business-trace`);
    for (const label of ['五维追溯', '正向追溯', '反向追溯']) {
      await expect(
        page.locator('.el-tabs__item', { hasText: label }),
        `页签缺少 ${label}`
      ).toHaveCount(1);
    }
    const queryCard = page
      .locator('.el-card')
      .filter({ has: page.locator('.el-card__header', { hasText: '五维追溯查询' }) })
      .first();
    await expect(queryCard, '未渲染五维追溯查询卡').toBeVisible({ timeout: 30_000 });

    const traceBtn = page.getByRole('button', { name: '追溯', exact: true });
    await expect(traceBtn, '缺少追溯提交按钮').toBeVisible();

    // 缺参提交：必须给必填文案，且不得把非法请求发给后端
    await traceBtn.click();
    await expect(
      page.locator('.el-message').filter({ hasText: '请输入五维追溯 ID' }).first(),
      '五维 ID 为空时未给出必填提示'
    ).toBeVisible({ timeout: 10_000 });
    expect(traceRequests, '五维 ID 为空时不应发追溯请求').toHaveLength(0);

    // 不存在的五维 ID：后端契约是 404 未找到追溯链（不是 500，也不是把空结果伪装成成功）
    const missingId = `E2E-NOT-EXIST-${String(Date.now()).slice(-8)}`;
    const failed = await apiCallExpectFail(
      page,
      'GET',
      `/business-trace/five-dimension/${missingId}`
    );
    expect(
      failed.status,
      `不存在的五维 ID 应报 404，实际 status=${failed.status} body=${JSON.stringify(failed).slice(0, 240)}`
    ).toBe(404);
    expect(
      failureCode(failed),
      `404 应带统一信封机器码 NOT_FOUND，实际：${JSON.stringify(failed).slice(0, 240)}`
    ).toBe(APP_ERROR_CODES.NOT_FOUND);

    // UI 侧同一条请求应真实发出并给出失败提示
    const responsePromise = page.waitForResponse(
      resp => resp.url().includes('/business-trace/five-dimension/') && resp.status() === 404,
      { timeout: 30_000 }
    );
    await queryCard.locator('input').first().fill(missingId);
    await traceBtn.click();
    const notFoundResp = await responsePromise;
    expect(
      failureCode(await notFoundResp.json().catch(() => null)),
      'UI 发出的追溯请求未拿到 NOT_FOUND 信封'
    ).toBe(APP_ERROR_CODES.NOT_FOUND);
    expect(
      page.locator('.el-message').filter({ hasText: '追溯查询失败' }).first(),
      'UI 未提示追溯查询失败'
    ).toBeVisible({ timeout: 10_000 });
    expect(traceRequests.length, '按不存在的五维 ID 查询应至少发出一次追溯请求').toBeGreaterThan(0);
  });

  test('安全设置 UI：登录日志按当前用户名筛出真实记录并逐列出值', async ({ page }) => {
    const me = await apiCallRaw<{ username?: string }>(page, 'GET', '/auth/me');
    const username = me.username;
    expect(username, `/auth/me 未返回 username，实际：${JSON.stringify(me)}`).toBeTruthy();

    // 本次会话经 UI 登录后，后端应留下该用户的登录记录；取不到即判红，不回落成可见性判据
    await expect
      .poll(
        async () => {
          const logs = await apiCallRaw<{ list: Array<{ username: string }> }>(
            page,
            'GET',
            `/login-logs?page=1&page_size=20&username=${encodeURIComponent(username ?? '')}`
          );
          return logs.list.filter(row => row.username === username).length;
        },
        { timeout: 30_000, message: `登录日志接口取不到用户 ${username} 的记录` }
      )
      .toBeGreaterThan(0);

    await page.goto(`${BASE_URL}/security`);
    // 该页标题层级是 h1，这里按文案锚定标题角色，不写死级别
    await expect(
      page.getByRole('heading', { name: '登录安全' }),
      '未渲染登录安全页标题'
    ).toBeVisible({ timeout: 30_000 });

    const logCard = page
      .locator('.el-card')
      .filter({ has: page.locator('.card-header span', { hasText: '登录日志' }) })
      .first();
    await expect(logCard, '未渲染登录日志卡').toBeVisible({ timeout: 30_000 });

    const usernameInput = formItemByExactLabel(logCard, '用户名').locator('input').first();
    await usernameInput.fill(username ?? '');
    await logCard.getByRole('button', { name: '查询', exact: true }).click();

    const table = logCard.locator('.el-table[aria-label="登录日志表"]').first();
    await table.waitFor({ state: 'visible', timeout: 30_000 });
    expect(await rowCount(table), '按用户名筛选后登录日志为空行').toBeGreaterThan(0);

    const userIdx = await columnIndex(table, '用户名');
    const statusIdx = await columnIndex(table, '状态');
    const timeIdx = await columnIndex(table, '登录时间');

    const userCells = await columnCellTexts(table, userIdx);
    for (const cell of userCells) {
      expect(cell, `用户名列出非本人记录：${cell}`).toContain(username ?? '');
    }
    const statusCells = await columnCellTexts(table, statusIdx);
    for (const cell of statusCells) {
      expect(['成功', '失败'], `状态列出现词表外取值：${cell}`).toContain(cell);
    }
    expect(
      statusCells.some(cell => cell === '成功'),
      `本次登录应在日志里留下一条成功记录，实际状态列=${JSON.stringify(statusCells)}`
    ).toBe(true);

    const timeCells = await columnCellTexts(table, timeIdx);
    expect(
      timeCells.filter(cell => cell.length > 0).length,
      '登录时间列全部为空（login_time 未渲染）'
    ).toBeGreaterThan(0);
  });

  test('打印模板 UI：造出的模板在列表按真值出值，列头与词表列齐全', async ({ page }) => {
    const stamp = String(Date.now()).slice(-6);
    const templateName = `回归模板${stamp}`;
    const templateCode = `TPL_E2E${stamp}`;
    // doc_type 是渲染链路查找键且必须属后端受支持词表，缺它建出来的模板打不出字；
    // status 显式给 active，否则后端该列落空串，页面按词表渲染成停用，判据就失去方向性
    const created = await apiCallRaw<{ id: number }>(page, 'POST', '/print-templates', {
      template_name: templateName,
      template_code: templateCode,
      doc_type: 'sales_order',
      status: 'active',
    });
    expect(Number.isFinite(created.id), `建模板应返回 id，实际：${JSON.stringify(created)}`).toBe(
      true
    );

    try {
      const table = await openPageTable(page, '/print-templates', '打印模板管理', '打印模板列表');
      expect(await rowCount(table), '打印模板列表为空行').toBeGreaterThan(0);

      const headers = await table
        .locator('.el-table__header-wrapper thead th.el-table__cell:not(.gutter)')
        .allInnerTexts();
      const headerTexts = headers.map(text => text.trim());
      for (const col of ['模板编号', '模板名称', '模块', '单据类型', '状态']) {
        expect(headerTexts, `表头缺少列 ${col}，实际表头=${JSON.stringify(headerTexts)}`).toContain(
          col
        );
      }

      const codeIdx = await columnIndex(table, '模板编号');
      const nameIdx = await columnIndex(table, '模板名称');
      const statusIdx = await columnIndex(table, '状态');
      const targetRow = await rowByText(table, templateCode);
      expect(await rowCellText(targetRow, codeIdx), '模板编号列未渲染 template_code').toBe(
        templateCode
      );
      expect(await rowCellText(targetRow, nameIdx), '模板名称列未渲染 template_name').toBe(
        templateName
      );
      expect(await rowCellText(targetRow, statusIdx), '状态列未按 status=active 渲染启用').toBe(
        '启用'
      );
    } finally {
      await tryCleanup(page, 'DELETE', `/print-templates/${created.id}`, `模板#${created.id}`);
    }
  });
});
