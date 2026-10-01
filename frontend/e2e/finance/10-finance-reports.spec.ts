// 财务管理 E2E 套件 — 10 财务报表：试算平衡恒等 + 资产负债表结构一致性 + 真实 xlsx 导出字节
//
// 任务 #942 缺口 7。断真实数值/字节，禁 toBeTruthy / verifyEndpointHealthy / >=400 / 仅 toast。
//   10-01 试算平衡：真实 seed 一张过账凭证 + refresh 科目余额，避免空库 vacuity；
//         断①全局 |Σ期末借 − Σ期末贷| < 0.01（非恒真的会计不变量）②两个被 seed 的科目在
//         entries 里携带精确借贷金额（内容级，证数据真的进了报表，非 0/0）。
//   10-02 资产负债表：断"total = 各分项之和"这一【非恒真】装配一致性（能抓出把 total 写死的 bug）。
//         注意：后端 build_balance_sheet_response 里 total_equity = total_assets − total_liabilities
//         是定义式恒等（对任意数据都成立），故【不】把它当会计恒等式断言——那正是 vacuity 假绿陷阱。
//   10-03/04 导出真 xlsx：取原始 body 断 PK(zip) 魔数 + spreadsheetml content-type（范式抄 purchase/12），
//         证明确是 OOXML xlsx 而非改名 HTML 假 xls。subjects/export、budgets/export 无审批令牌门，直连可导。
//   10-05 敏感报表导出 fail-closed：balance-sheet/export 未带 download_token → 403（真实权限拒绝码，非 >=400）。
import { test, expect } from '../diagnose-fixture';
import {
  API_BASE,
  API_PREFIX,
  loginViaUI,
  apiCall,
  apiCallRaw,
  tryCleanup,
  genCode,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function num(obj: Record<string, unknown>, key: string): number {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`响应缺少后端真实键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n)) throw new Error(`键 "${key}" 非数字，raw=${JSON.stringify(obj[key])}`);
  return n;
}

function currentPeriodStr(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}`;
}

interface ReportItem {
  name: string;
  amount: unknown;
}

// balance_direction 写入方权威词表＝backend models/status/finance.rs 的 account_subject 常量（debit/credit），#198 起禁灌中文。
async function seedLeafSubject(
  page: import('@playwright/test').Page,
  direction: 'debit' | 'credit'
): Promise<{ id: number; code: string }> {
  const code = genCode('E2E-RPT');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/subjects', {
    code,
    name: `E2E 报表科目${direction}${code}`,
    level: 1,
    balance_direction: direction,
  });
  const id = created.data?.id;
  if (!id) throw new Error(`建科目失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/subjects/${id}`, label: 'account_subject' });
  return { id, code };
}

/** 取导出端点原始响应（不当 JSON 解析，保留二进制 body）。 */
async function fetchExportRaw(
  page: import('@playwright/test').Page,
  erpPath: string
): Promise<Awaited<ReturnType<typeof page.request.get>>> {
  return page.request.get(`${API_BASE}${API_PREFIX}${erpPath}`, {
    headers: { 'X-Requested-With': 'XMLHttpRequest' },
    timeout: 60_000,
  });
}

/** 真 xlsx 断言：200 + spreadsheetml + attachment + PK(zip) 魔数 + 非 HTML 起头。 */
async function assertRealXlsxBlob(
  resp: Awaited<ReturnType<typeof fetchExportRaw>>,
  ctx: string
): Promise<void> {
  expect(resp.status(), `${ctx}：导出应 HTTP 200`).toBe(200);
  const ct = resp.headers()['content-type'] ?? '';
  expect(ct, `${ctx}：Content-Type 应为 OOXML spreadsheetml，实际=${ct}`).toContain(
    'spreadsheetml'
  );
  const cd = resp.headers()['content-disposition'] ?? '';
  expect(cd, `${ctx}：应带 attachment 下载头，实际=${cd}`).toContain('attachment');

  const buf = await resp.body();
  expect(buf.length, `${ctx}：字节流长度应 >= 4（可读魔数）`).toBeGreaterThanOrEqual(4);
  expect(
    [buf[0], buf[1], buf[2], buf[3]],
    `${ctx}：前 4 字节应为 zip 魔数 0x50 0x4B 0x03 0x04（真 xlsx=zip 容器），实际=` +
      [buf[0], buf[1], buf[2], buf[3]].map(b => '0x' + b.toString(16).padStart(2, '0')).join(' ')
  ).toEqual([0x50, 0x4b, 0x03, 0x04]);

  const headAscii = buf.subarray(0, 16).toString('latin1').toLowerCase();
  expect(
    headAscii,
    `${ctx}：body 头部不应是 HTML/XML 假 xls，实际头部=${JSON.stringify(headAscii)}`
  ).not.toMatch(/<html|<!doctype|<\?xml|<table|<worksheet/);
}

test.describe('10 财务报表与导出', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await apiCall(page, 'POST', '/finance/accounting-periods/init', {}).catch(e =>
      console.warn('[10] 会计期间初始化失败:', (e as Error).message)
    );
  });

  test('10-01 试算平衡：seed 过账凭证后断全局借贷差<0.01 且被 seed 科目金额精确入表', async ({
    page,
  }) => {
    const { id: debitId, code: debitCode } = await seedLeafSubject(page, 'debit');
    const { id: creditId, code: creditCode } = await seedLeafSubject(page, 'credit');
    const period = currentPeriodStr();
    const today = new Date().toISOString().slice(0, 10);

    // 建平衡凭证 1000 并推进到 posted（当期 OPEN 才允许过账）。
    const created = await apiCall<{ id?: number }>(page, 'POST', '/vouchers', {
      voucher_type: '记',
      voucher_date: today,
      items: [
        { subject_id: debitId, debit: 1000, credit: 0, summary: 'E2E 报表借' },
        { subject_id: creditId, debit: 0, credit: 1000, summary: 'E2E 报表贷' },
      ],
    });
    const voucherId = created.data?.id;
    expect(typeof voucherId, '平衡凭证应先建单成功').toBe('number');
    if (voucherId) CLEANUP.push({ path: `/vouchers/${voucherId}`, label: 'voucher' });
    await apiCall(page, 'POST', `/vouchers/${voucherId}/submit`);
    await apiCall(page, 'POST', `/vouchers/${voucherId}/review`);
    await apiCall(page, 'POST', `/vouchers/${voucherId}/post`);

    // 把过账发生额刷回科目主数据（试算平衡读的是 account_subject 余额列）。
    await apiCall(page, 'POST', `/subjects/${debitId}/refresh-balance?period=${period}`);
    await apiCall(page, 'POST', `/subjects/${creditId}/refresh-balance?period=${period}`);

    const tb = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/finance/reports/trial-balance?period=${period}`
    );
    // ① 非恒真的全局会计不变量。
    const totalDr = num(tb, 'total_ending_debit');
    const totalCr = num(tb, 'total_ending_credit');
    expect(Math.abs(totalDr - totalCr), `试算平衡：借${totalDr} vs 贷${totalCr}`).toBeLessThan(
      0.01
    );

    // ② 内容级：seed 的借贷科目在 entries 里携带精确金额（证数据真入表，非 0/0 vacuity）。
    if (!Array.isArray(tb.entries)) {
      throw new Error(`试算平衡缺少 entries 数组，实际键=${Object.keys(tb).join(',')}`);
    }
    const de = (tb.entries as Array<Record<string, unknown>>).find(
      e => String(e.subject_code) === debitCode
    );
    const ce = (tb.entries as Array<Record<string, unknown>>).find(
      e => String(e.subject_code) === creditCode
    );
    expect(de, `entries 应含借方科目 ${debitCode}`).toBeTruthy();
    expect(ce, `entries 应含贷方科目 ${creditCode}`).toBeTruthy();
    expect(Math.abs(num(de!, 'ending_debit') - 1000), '借方科目期末借方应=1000').toBeLessThan(0.01);
    expect(Math.abs(num(ce!, 'ending_credit') - 1000), '贷方科目期末贷方应=1000').toBeLessThan(
      0.01
    );
  });

  test('10-02 资产负债表：断 total = 各分项之和（非恒真的装配一致性，不碰 total_equity 定义式恒等）', async ({
    page,
  }) => {
    const bs = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      '/finance/reports/balance-sheet'
    );
    const assets = bs.assets as ReportItem[];
    const liabilities = bs.liabilities as ReportItem[];
    expect(Array.isArray(assets), 'assets 应为数组').toBeTruthy();
    expect(Array.isArray(liabilities), 'liabilities 应为数组').toBeTruthy();

    const assetSum = assets.reduce((a, it) => a + Number(it.amount), 0);
    const liabSum = liabilities.reduce((a, it) => a + Number(it.amount), 0);
    expect(
      Math.abs(num(bs, 'total_assets') - assetSum),
      'total_assets 应等于各资产分项之和'
    ).toBeLessThan(0.01);
    expect(
      Math.abs(num(bs, 'total_liabilities') - liabSum),
      'total_liabilities 应等于各负债分项之和'
    ).toBeLessThan(0.01);
    // 资产负债表结构：固定 4 资产 + 2 负债行（服务层组装，抓行数漂移）。
    expect(assets.length, '资产行应为 4').toBe(4);
    expect(liabilities.length, '负债行应为 2').toBe(2);
  });

  test('10-03 会计科目导出返回真 OOXML xlsx（PK zip 魔数，非 HTML 假 xls）', async ({ page }) => {
    const resp = await fetchExportRaw(page, '/subjects/export');
    await assertRealXlsxBlob(resp, '会计科目导出');
  });

  test('10-04 预算明细导出返回带水印真 xlsx（PK zip 魔数）', async ({ page }) => {
    const resp = await fetchExportRaw(page, '/budgets/export');
    await assertRealXlsxBlob(resp, '预算明细导出');
  });

  test('10-05 敏感报表导出 fail-closed：无 download_token 访问资产负债表导出被拒 403', async ({
    page,
  }) => {
    const resp = await fetchExportRaw(page, '/finance/reports/balance-sheet/export');
    // enforce_export_download 缺令牌 → permission_denied(403)，真实权限拒绝码（非 >=400 泛断）。
    expect(resp.status(), '未带审批令牌的报表导出应 403').toBe(403);
  });
});
