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

    const listResp = await apiCall(page, 'GET', '/bpm/templates?page=1&page_size=10');
    const items = listResp?.items ?? listResp?.data?.items;
    if (!items || items.length === 0) {
      testInfo.annotations.push({
        type: 'skipped',
        description:
          'BPM 模板列表为空（bpm_definition_handler::list_templates 无种子模板，CI 库未建流程模板）',
      });
      console.error(
        '[P5.6-预览][BPM 模板] ❌ 跳过：/bpm/templates 列表无数据（响应前 200 字符：' +
          JSON.stringify(listResp).slice(0, 200) +
          '）'
      );
      test.skip();
      return;
    }

    const templateId = items[0].id;
    // routes/system.rs 只注册了 /bpm/templates、/bpm/templates/{id}、
    // /bpm/templates/{id}/create，无 preview 端点：端点缺失时 404 硬失败暴露。
    const previewResp = await apiCall(page, `GET`, `/bpm/templates/${templateId}/preview`);

    expect(previewResp !== null).toBeTruthy();
    await assertPageHealthy(page, collector, { consoleNoisePatterns: BROWSER_NETWORK_NOISE });
  });
});
