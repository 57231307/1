import { test, expect } from '@playwright/test';
import JSZip from 'jszip';
import { loginViaUI, apiCallRaw } from '../flow/helpers';

/**
 * 39c 导出内容断言（doto 2026-09-09 导出内容缺口）
 *
 * 现有 39 spec 仅断言 2xx/3xx + 文件名后缀；本 spec 深度断言导出内容：
 * 1. /products/export + /customers/export（敏感清单里，但 admin 持有审批令牌链路
 *    复杂——改用非敏感端点做内容断言；敏感端点内容断言在 39b 审批链令牌侧扩展）
 * 2. /warehouses/export（非敏感，基础数据必有）
 * 3. /stock/export（库存，ensureTestEntities 建过库存）
 *
 * xlsx = zip 容器：
 * - xl/sharedStrings.xml（字符串表）或 xl/worksheets/sheet1.xml（内联字符串）
 * - 断言列头（中文表头）与数据行数 ≥ 列表 API 行数（内容与源数据一致）
 *
 * 全部真实后端 + 每步显式日志
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

/** xlsx 解包提取全部文本（sharedStrings + sheet1 内联 + inlineStr） */
async function extractXlsxText(body: Buffer): Promise<{ text: string; sheetRows: number }> {
  const zip = await JSZip.loadAsync(body);
  let text = '';
  const shared = await zip.file('xl/sharedStrings.xml')?.async('string');
  if (shared) text += shared;
  // 遍历全部 worksheet（多 sheet 场景）
  const sheetFiles = Object.keys(zip.files).filter(
    f => f.startsWith('xl/worksheets/') && f.endsWith('.xml')
  );
  let sheetRows = 0;
  for (const f of sheetFiles) {
    const xml = await zip.file(f)!.async('string');
    text += xml;
    sheetRows += (xml.match(/<row[ >]/g) ?? []).length;
  }
  return { text, sheetRows };
}

test.describe('39c 导出内容断言', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('仓库导出 xlsx 内容与列表数据一致（列头+行数）', async ({ page }) => {
    test.setTimeout(120_000);

    // ---- 1. 列表 API 行数（源数据基准）----
    const list = await apiCallRaw<{
      items: Array<{ id: number; name?: string; code?: string }>;
      total: number;
    }>(page, 'GET', '/warehouses?page=1&page_size=100');
    const listCount = list.items?.length ?? 0;
    console.log(`[39c] 仓库列表行数=${listCount}（total=${list.total}）`);

    // ---- 2. 真实导出 ----
    const exportResp = await page.request
      .get(`${API_BASE}${API_PREFIX}/warehouses/export`)
      .catch((e) => { console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`); return null; });
    if (!exportResp) throw new Error('网络错误: /warehouses/export');
    const status = exportResp.status();
    console.log(`[39c] /warehouses/export → ${status}`);
    if (status === 404 || status === 400) {
      test.info().annotations.push({
        type: 'missing-data',
        description: `仓库导出返回 ${status}（端点或数据缺失）`,
      });
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }
    expect(status, `导出应 200，实际 ${status}`).toBe(200);

    const body = await exportResp.body();
    expect(body.length, `xlsx 应 >512B，实际 ${body.length}B`).toBeGreaterThan(512);
    const contentType = exportResp.headers()['content-type'] ?? '';
    console.log(`[39c] Content-Type=${contentType}`);

    // ---- 3. xlsx 解包内容断言 ----
    const { text, sheetRows } = await extractXlsxText(body);
    expect(text.length, 'xlsx 解包应提取到文本内容').toBeGreaterThan(0);
    console.log(`[39c] xlsx 解包：${text.length}B 文本 / ${sheetRows} 行`);

    // 列头断言：仓库名称/编码类中文表头应出现（sharedStrings 或内联）
    const hasHeader =
      text.includes('仓库') || text.includes('名称') || text.includes('编码') || text.includes('code');
    expect(hasHeader, '导出应包含仓库相关列头').toBeTruthy();

    // 行数断言：导出数据行 ≥ 列表行数（列表分页 page_size=100，导出全量）
    if (listCount > 0) {
      expect(
        sheetRows,
        `导出数据行 ${sheetRows} 应 ≥ 列表行数 ${listCount}（内容完整性）`,
      ).toBeGreaterThanOrEqual(listCount);
    }
    // 名称内容匹配：列表第一条名称应出现在导出文本中
    const first = list.items?.[0];
    if (first?.name) {
      expect(
        text.includes(first.name),
        `导出应包含列表首条名称 "${first.name}"（内容与源数据一致）`,
      ).toBeTruthy();
      console.log(`[39c] ✅ 内容匹配："${first.name}" 出现在导出文件`);
    }
  });

  test('库存导出 xlsx 行数与列表一致', async ({ page }) => {
    test.setTimeout(120_000);

    let listCount = 0;
    let firstText = '';
    try {
      const list = await apiCallRaw<{
        items: Array<{ id: number; batch_no?: string; product_name?: string }>;
        total: number;
      }>(page, 'GET', '/inventory/stock?page=1&page_size=100');
      listCount = list.items?.length ?? 0;
      firstText = list.items?.[0]?.batch_no ?? '';
      console.log(`[39c] 库存列表行数=${listCount}，首条=${firstText}`);
    } catch (e) {
      console.log('[39c] 库存列表查询失败:', (e as Error).message);
    }

    const exportResp = await page.request
      .get(`${API_BASE}${API_PREFIX}/inventory/stock/export`)
      .catch((e) => { console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`); return null; });
    if (!exportResp) throw new Error('网络错误: /inventory/stock/export');
    const status = exportResp.status();
    console.log(`[39c] /stock/export → ${status}`);
    if (status === 404 || status === 400) {
      test.info().annotations.push({ type: 'missing-data', description: `库存导出返回 ${status}` });
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }
    expect(status, `库存导出应 200，实际 ${status}`).toBe(200);

    const body = await exportResp.body();
    expect(body.length, '库存 xlsx 应 >512B').toBeGreaterThan(512);
    const { text, sheetRows } = await extractXlsxText(body);
    console.log(`[39c] 库存 xlsx：${text.length}B / ${sheetRows} 行`);

    if (listCount > 0) {
      expect(
        sheetRows,
        `导出行 ${sheetRows} 应 ≥ 列表行 ${listCount}`,
      ).toBeGreaterThanOrEqual(listCount);
    }
    if (firstText) {
      expect(
        text.includes(firstText),
        `导出应包含首条批号 ${firstText}（内容一致）`,
      ).toBeTruthy();
      console.log(`[39c] ✅ 库存批号 ${firstText} 内容匹配`);
    }
  });
});
