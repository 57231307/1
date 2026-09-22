import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallExpectFail,
  trackPageHealth,
  assertPageHealthy,
  BROWSER_NETWORK_NOISE,
} from './helpers';

/**
 * P5.6 预览测试
 * 依赖：P3.2（trackPageHealth/assertPageHealthy）
 *
 * 验证：
 * - print-templates 预览（字段出现在 DOM）
 * - report-templates 预览
 * - BPM /templates/{id}/preview API 内容断言
 * - 预览容器无截断
 */
test.describe('P5.6 预览', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('print-templates 列表 + 预览 API', async ({ page }) => {
    const collector = trackPageHealth(page);

    // print_handler::list_print_templates 返回 ApiResponse<Vec<Model>>：
    // data 是裸数组（无 items 包装），此前按 items 取值恒为 undefined → 整条用例假绿跳过
    const listResp = await apiCall<Array<{ id: number; template_name?: string }>>(
      page,
      'GET',
      '/print-templates'
    );
    const items = listResp.data;
    expect(
      Array.isArray(items),
      `打印模板列表 data 应为数组，实际：${JSON.stringify(listResp).slice(0, 200)}`
    ).toBe(true);
    expect(
      items.length,
      'PRINT_TEMPLATE_STORE 启动即预填充系统内置模板，列表为空属环境缺陷'
    ).toBeGreaterThan(0);

    // 对第一个模板调预览 API（routes/mod.rs:305 注册为 POST /print-templates/{id}/preview）
    const templateId = items[0].id;
    expect(templateId, `打印模板首行应带 id，实际：${JSON.stringify(items[0])}`).toBeTruthy();
    const preview = await apiCall<{ html?: string; variables?: unknown }>(
      page,
      'POST',
      `/print-templates/${templateId}/preview`,
      {}
    );
    // 预览必须返回渲染后的 HTML（handler 拼 <html>…<h3>模板名</h3>…）
    expect(
      typeof preview?.data?.html,
      `预览响应应含 html 字段，实际：${JSON.stringify(preview).slice(0, 200)}`
    ).toBe('string');
    expect(preview.data.html, '预览 HTML 不应为空').not.toBe('');
    expect(preview.data.html, '预览 HTML 应为完整 HTML 文档').toContain('<html');
    await assertPageHealthy(page, collector, { consoleNoisePatterns: BROWSER_NETWORK_NOISE });
  });

  test('report-templates 列表 + 预览端点契约', async ({ page }, testInfo) => {
    const collector = trackPageHealth(page);

    // report_engine_handler::list_templates 返回 ApiResponse<Vec<Model>>：data 是裸数组
    // （内容由 ReportEngineService::get_predefined_templates 提供，恒定非空）。
    // 此前按 items 取值恒为 undefined → 用例每轮静默跳过（假绿）。
    const listResp = await apiCall<Array<{ id?: string | number; name?: string }>>(
      page,
      'GET',
      '/report-templates'
    );
    const items = listResp.data;
    expect(
      Array.isArray(items),
      `报表模板列表 data 应为数组，实际：${JSON.stringify(listResp).slice(0, 200)}`
    ).toBe(true);
    expect(items.length, '预置报表模板应至少 1 条').toBeGreaterThan(0);
    for (const tpl of items.slice(0, 5)) {
      expect(
        String(tpl?.id ?? tpl?.name ?? ''),
        `报表模板行应含 id/name，实际：${JSON.stringify(tpl)}`
      ).not.toBe('');
    }

    // 预览端点当前契约：routes/analytics.rs 的 report_engine 组只注册了
    // /report-templates、/execute、/export、/aggregate，没有 /report-templates/{id}/preview；
    // report_enhanced 的 /templates/{id}/preview 消费的是另一张模板表，id 不通用。
    // 这里把"未实现"钉成显式断言：后端补上端点后本行会失败，届时升级为内容断言。
    const previewStatus = await apiCallExpectFail(
      page,
      'GET',
      `/report-templates/${items[0].id}/preview`
    );
    testInfo.annotations.push({
      type: 'known-gap',
      description: 'GET /report-templates/{id}/preview 后端未注册（report_engine 预览缺口）',
    });
    expect(
      previewStatus.status,
      `报表模板预览端点未实现时应 404，实际 ${previewStatus.status}`
    ).toBe(404);
    await assertPageHealthy(page, collector, { consoleNoisePatterns: BROWSER_NETWORK_NOISE });
  });

  test('BPM 模板预览 API', async ({ page }, testInfo) => {
    const collector = trackPageHealth(page);

    // 原实现按 /bpm/templates 列表首行取 id、列表为空即 test.skip()：
    // (1) /bpm/templates 无 POST 创建路由（routes/system.rs 仅 GET list_templates +
    //     GET/DELETE /templates/{id} + POST /definitions/{id}/template save_as_template），
    //     CI 空库下列表恒空 → 每轮静默跳过（假绿）；
    // (2) /bpm/templates/{id}/preview 端点根本未注册（routes/system.rs 无该路由），
    //     即使命中模板，preview 也恒 404 → 原 `expect(previewResp !== null)` 永远走不到。
    // 与同文件 report-templates 用例一致，把“端点未实现”钉成显式契约断言：
    // 后端补上 preview 端点后本行会失败，届时再升级为内容断言。
    const preview = await apiCallExpectFail(page, 'GET', '/bpm/templates/1/preview');
    testInfo.annotations.push({
      type: 'known-gap',
      description: 'GET /bpm/templates/{id}/preview 后端未注册（BPM 模板预览缺口），钉为 404 契约',
    });
    expect(preview.status, `BPM 模板预览端点未实现时应 404，实际 ${preview.status}`).toBe(404);
    await assertPageHealthy(page, collector, { consoleNoisePatterns: BROWSER_NETWORK_NOISE });
  });
});
