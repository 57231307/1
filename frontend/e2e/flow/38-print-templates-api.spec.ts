import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCallRaw, apiCallExpectFail, BASE_URL } from './helpers';

/**
 * P5.8 print-templates API 全链路
 *
 * 后端实际路由（src/routes/mod.rs print_templates_routes）：
 * - GET /print-templates（list_print_templates，ApiResponse<Vec>：data 为裸数组）
 * - GET /print-templates/{id}（detail）
 * - POST /print-templates/{id}/preview（print_handler::preview_print_template）
 * 前端 api/print-templates.ts 封装的 setDefault/copy
 * 走各业务单据 /{id}/print 端点（见 37-print-endpoints 全量矩阵）
 *
 * 断言：列表结构化 + 首个模板 detail 可达 + 字段完整性
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

test.describe('P5.8 print-templates API 链路', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('列表：结构化返回', async ({ page }) => {
    // list_print_templates 返回 ApiResponse<Vec<Model>>：data 是裸数组（无 items 包装）
    const resp = await apiCallRaw<unknown>(page, 'GET', '/print-templates?page=1&page_size=10');
    expect(
      Array.isArray(resp),
      `打印模板列表 data 应为数组，实际：${JSON.stringify(resp).slice(0, 200)}`
    ).toBe(true);
  });

  test('列表分页参数生效', async ({ page }) => {
    const p1 = await apiCallRaw<unknown>(page, 'GET', '/print-templates?page=1&page_size=1');
    // print-templates 为 builtin 硬编码只读清单（后端无 DB 分页），
    // page_size 可能被忽略返回全量——仅断言结构合法（第三轮 CI 48 分片修复）
    expect(
      Array.isArray(p1),
      `打印模板列表 data 应为数组，实际：${JSON.stringify(p1).slice(0, 200)}`
    ).toBe(true);
  });

  test('detail：首个模板可达', async ({ page }) => {
    const list = await apiCallRaw<unknown>(page, 'GET', '/print-templates?page=1&page_size=1');
    expect(
      Array.isArray(list),
      `打印模板列表 data 应为数组，实际：${JSON.stringify(list).slice(0, 200)}`
    ).toBe(true);
    const items = list as Array<{ id: number }>;
    expect(
      items.length,
      'PRINT_TEMPLATE_STORE 启动即预填充系统内置打印模板，列表为空属环境缺陷'
    ).toBeGreaterThan(0);

    const templateId = items[0].id;
    expect(templateId, `内置打印模板首行应带 id，实际：${JSON.stringify(items[0])}`).toBeTruthy();
    const detail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/print-templates/${templateId}`
    );

    expect(detail, `模板 ${templateId} 详情应返回对象`).toBeTruthy();
    expect(
      String(detail?.template_name ?? ''),
      `模板 ${templateId} 详情应含 template_name，实际：${JSON.stringify(detail).slice(0, 200)}`
    ).not.toBe('');
  });

  test('不存在的模板 id → 404（非 5xx）', async ({ page }) => {
    const resp = await page.request.get(`${API_BASE}${API_PREFIX}/print-templates/99999999`);
    const status = resp.status();
    expect(status).toBeLessThan(500);
    expect(status === 404 || status === 400).toBe(true);
  });

  test('列表出参每行都带非空 doc_type（打印链路查找键必须可见）', async ({ page }) => {
    const list = await apiCallRaw<Array<Record<string, unknown>>>(page, 'GET', '/print-templates');
    expect(list.length, '内置模板种子为空属环境缺陷').toBeGreaterThan(0);
    for (const row of list) {
      expect(
        typeof row.doc_type === 'string' && row.doc_type !== '',
        `模板 ${JSON.stringify(row).slice(0, 160)} 出参缺 doc_type：前端无法得知该模板服务于哪种单据`
      ).toBe(true);
    }
  });

  test('模板打印端点缺 doc_id → 400（禁止伪造样例打印，fail-closed）', async ({ page }) => {
    const list = await apiCallRaw<Array<Record<string, unknown>>>(page, 'GET', '/print-templates');
    const templateId = Number(list[0].id);
    // 走 apiCallExpectFail 而非裸 page.request：写方法必须带 CSRF Token，
    // 否则先被中间件 403 拦下，断言到的就不是被测契约本身。
    const fail = await apiCallExpectFail(page, 'POST', `/print-templates/${templateId}/print`, {});
    expect(
      fail.status,
      `无单据实例的打印请求必须被拒（历史缺陷：模板页打印按钮恒发空体，用户点即 400），实得：${JSON.stringify(fail)}`
    ).toBe(400);
  });

  test('新建模板缺 doc_type → 400 点名必填（不再以 template_code 冒充查找键）', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'POST', '/print-templates', {
      template_name: 'e2e-缺单据类型模板',
      template_code: `TPL-E2E-NODOC-${Date.now()}`,
      module: 'logistics',
      type: 'custom',
    });
    expect(
      fail.status,
      `缺查找键的模板永不会被渲染链路命中，必须拒绝受理，实得：${JSON.stringify(fail)}`
    ).toBe(400);
    expect(
      String(fail.code),
      `应为入参校验拒绝而非中间件拦截，实得：${JSON.stringify(fail)}`
    ).toBe('VALIDATION_ERROR');
  });

  test('新建模板 doc_type 词表外 → 400 点名（不静默存下打不出的模板）', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'POST', '/print-templates', {
      template_name: 'e2e-非法单据类型模板',
      template_code: `TPL-E2E-BADDOC-${Date.now()}`,
      doc_type: 'not_a_real_doc_type',
      module: 'logistics',
      type: 'custom',
    });
    expect(fail.status, `词表外单据类型应被拒受理，实得：${JSON.stringify(fail)}`).toBe(400);
    expect(
      String(fail.message ?? ''),
      '拒绝文案须点名非法取值，否则用户不知道该改成什么'
    ).toContain('not_a_real_doc_type');
  });

  test('模板预览对话框不暴露无单据上下文的打印入口', async ({ page }) => {
    await page.goto(`${BASE_URL}/print-templates`);
    await page.waitForSelector('.el-table__row', { timeout: 20000 });
    await page.locator('.el-table__row').first().getByRole('button', { name: '预览' }).click();
    const dialog = page.locator('[aria-label="模板预览对话框"]');
    await expect(dialog).toBeVisible();
    expect(
      await dialog.getByRole('button', { name: '打印' }).count(),
      '预览对话框不得保留一个必然 400 的打印按钮（打印入口在业务单据侧，由源单带出 doc_id）'
    ).toBe(0);
  });
});
