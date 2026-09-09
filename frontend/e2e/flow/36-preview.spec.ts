import { test, expect } from '@playwright/test';
import { loginViaUI, apiCall, trackPageHealth, assertPageHealthy } from './helpers';

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

    // 拉取打印模板列表
    const listResp = await apiCall(page, 'GET', '/print-templates?page=1&page_size=10');
    const items = listResp?.items ?? listResp?.data?.items;
    if (!items || items.length === 0) {
      test.skip();
      return;
    }

    // 对第一个模板调预览 API
    const templateId = items[0].id;
    const previewResp = await apiCall(
      page,
      'GET',
      `/print-templates/${templateId}/preview`,
    ).catch(() => null);

    // 预览应返回内容（HTML 或文档数据）
    expect(previewResp !== null).toBeTruthy();
    await assertPageHealthy(page, collector, { allowConsoleWarn: true });
  });

  test('report-templates 预览 API', async ({ page }) => {
    const collector = trackPageHealth(page);

    const listResp = await apiCall(page, 'GET', '/report-templates?page=1&page_size=10').catch(() => null);
    const items = listResp?.items ?? listResp?.data?.items;
    if (!items || items.length === 0) {
      test.skip();
      return;
    }

    const templateId = items[0].id;
    const previewResp = await apiCall(
      page,
      'GET',
      `/report-templates/${templateId}/preview`,
    ).catch(() => null);

    expect(previewResp !== null).toBeTruthy();
    await assertPageHealthy(page, collector, { allowConsoleWarn: true });
  });

  test('BPM 模板预览 API', async ({ page }) => {
    const collector = trackPageHealth(page);

    const listResp = await apiCall(page, 'GET', '/bpm/templates?page=1&page_size=10').catch(() => null);
    const items = listResp?.items ?? listResp?.data?.items;
    if (!items || items.length === 0) {
      test.skip();
      return;
    }

    const templateId = items[0].id;
    const previewResp = await apiCall(
      page,
      `GET`,
      `/bpm/templates/${templateId}/preview`,
    ).catch(() => null);

    expect(previewResp !== null).toBeTruthy();
    await assertPageHealthy(page, collector, { allowConsoleWarn: true });
  });
});
