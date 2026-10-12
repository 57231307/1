// 采购合同导出「真 OOXML xlsx」契约级 e2e
//
// 背景：新增 GET /purchase/purchase-contracts/export（backend purchase_contract_handler.rs
//   export_purchase_contracts → utils/xlsx_export.rs build_xlsx_response_with_watermark），
//   用 rust_xlsxwriter 生成真 OOXML xlsx 字节流：
//     - Content-Type: application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
//     - Content-Disposition: attachment; filename="...xlsx"
//     - body 以 PK\x03\x04（zip 魔数）开头（xlsx 本质是 zip 容器），非 HTML 假 xls。
//
// 「假阳性防线」：以「拿原始字节」方式（page.request.get → .body() Buffer，绝不当 JSON 解析）
//   断前 4 字节 = 0x50 0x4B 0x03 0x04（zip 魔数），并反证 body 头部不是 HTML/XML 起头，
//   从而证明是「真 xlsx」而非「改名的 HTML 假 xls」。
//   空库（无合同数据）时后端仍返回带表头的合法 xlsx，PK 魔数断言同样成立 → 诚实覆盖两种情况。
import { test, expect } from '../diagnose-fixture';
import { API_BASE, API_PREFIX, loginViaUI, apiCall, genCode, tryCleanup } from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 取导出端点的原始响应（不解析 JSON，保留二进制 body）。 */
async function fetchExportRaw(page: import('@playwright/test').Page) {
  return page.request.get(`${API_BASE}${API_PREFIX}/purchase/purchase-contracts/export`, {
    headers: { 'X-Requested-With': 'XMLHttpRequest' },
    timeout: 60_000,
  });
}

/** 核心 xlsx 真身断言：200 + OOXML content-type + attachment + PK zip 魔数 + 非 HTML 起头。 */
async function assertRealXlsxBlob(
  resp: Awaited<ReturnType<typeof fetchExportRaw>>,
  ctx: string
): Promise<void> {
  expect(resp.status(), `${ctx}：导出应 HTTP 200`).toBe(200);

  const ct = resp.headers()['content-type'] ?? '';
  expect(ct, `${ctx}：Content-Type 应为 OOXML 表格类型（spreadsheetml），实际=${ct}`).toContain(
    'spreadsheetml'
  );

  const cd = resp.headers()['content-disposition'] ?? '';
  expect(cd, `${ctx}：应带 attachment 下载头，实际=${cd}`).toContain('attachment');

  // 关键：以原始字节取 body（勿 JSON 解析），断前 4 字节为 zip 魔数 PK\x03\x04。
  const buf = await resp.body();
  expect(buf.length, `${ctx}：xlsx 字节流长度应 >= 4（能读魔数）`).toBeGreaterThanOrEqual(4);
  expect(
    [buf[0], buf[1], buf[2], buf[3]],
    `${ctx}：前 4 字节应为 zip 魔数 0x50 0x4B 0x03 0x04（真 xlsx=zip 容器），实际=` +
      [buf[0], buf[1], buf[2], buf[3]].map(b => '0x' + b.toString(16).padStart(2, '0')).join(' ')
  ).toEqual([0x50, 0x4b, 0x03, 0x04]);

  // 反证：不是 HTML/XML 假 xls 起头（真 xlsx 头部 ascii 化为 "PK\x03\x04..."）。
  const headAscii = buf.subarray(0, 16).toString('latin1').toLowerCase();
  expect(
    headAscii,
    `${ctx}：body 头部不应是 HTML/XML 假 xls（应以 zip 魔数 PK 起头），实际头部=${JSON.stringify(headAscii)}`
  ).not.toMatch(/<html|<!doctype|<\?xml|<table|<worksheet/);
}

test.describe('采购合同导出：真 OOXML xlsx blob（真实字节断言）', () => {
  test.beforeEach(async ({ page }) => {
    // 复用真实 admin 会话（导出端点 handler 要求 AuthContext，GET 不需 CSRF）。
    await loginViaUI(page);
  });

  test('GET /purchase/purchase-contracts/export 返回真 xlsx（200 + spreadsheetml + attachment + PK zip 魔数，非 HTML 假 xls）', async ({
    page,
  }) => {
    // 空库或有数据都应成立：rust_xlsxwriter 即便 rows 为空也产出含表头的合法 xlsx（PK 魔数不变）。
    const resp = await fetchExportRaw(page);
    await assertRealXlsxBlob(resp, '导出端点');
  });

  test('有真实合同数据时导出仍为合法 xlsx（种子→导出→断 PK 魔数与表头类型）', async ({ page }) => {
    // 建真实外键供应商 + draft 合同（走真实 API），让导出查询命中「有数据」分支
    // （含创建人姓名 LEFT JOIN），再验证导出依旧是真 xlsx。
    const supName = `E2E供导出${Date.now().toString().slice(-8)}`;
    const sup = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
      supplier_name: supName,
      supplier_short_name: 'E2E供导',
      contact_phone: '13800000002',
    });
    const supplierId = sup.data?.id;
    if (!supplierId) throw new Error(`种子供应商失败：${JSON.stringify(sup)}`);
    CLEANUP.push({ path: `/purchase/suppliers/${supplierId}`, label: 'supplier' });

    const contractNo = genCode('E2E-PCX');
    const created = await apiCall<{ id?: number }>(page, 'POST', '/purchase/purchase-contracts', {
      contract_no: contractNo,
      contract_name: `E2E 导出合同 ${contractNo}`,
      supplier_id: supplierId,
      total_amount: 66000,
      delivery_date: '2026-12-31',
      payment_terms: 'E2E 导出账期',
    });
    const contractId = created.data?.id;
    if (!contractId) throw new Error(`种子合同失败：${JSON.stringify(created)}`);
    CLEANUP.push({
      path: `/purchase/purchase-contracts/${contractId}`,
      label: 'purchase_contract',
    });

    // 导出：证明「有数据」下仍是真 xlsx。
    const resp = await fetchExportRaw(page);
    await assertRealXlsxBlob(resp, `含种子合同 ${contractNo} 的导出`);
  });
});
