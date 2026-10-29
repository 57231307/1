// 供应商评估记录 与 评分聚合 的建改读契约 E2E — 建后按记录 id 与父供应商逐字段回读，越界得分被公开范围规则点名拒绝
//
// 本文件守的账：评估记录写端点必须按 supplier_id+indicator_id 真落库，加权分=得分×权重/满分由服务端派生；
// 记录读回视图必须把父供应商名与指标名 JOIN 富化回填（外键行缺失为 null，禁止用编码/id 冒充名称）；
// 评分聚合 average_score=Σ加权分/Σ权重×100 必须与单条记录的满分比一致；更新只动提交的 score/remark 列。
// 负例锁真实拒绝：得分越界 [0,max_score] 经服务端校验回 VALIDATION_ERROR，并在拒绝后按记录列表断零写入，
// 绝不把被拒当健康。指标无删除端点（配置类不可回收），本用例如实登记，残留交数据卫生复核。
import { test, expect } from '../diagnose-fixture';
import { type Page } from '@playwright/test';
import {
  loginViaUI,
  ensureTestEntities,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  APP_ERROR_CODES,
  genCode,
  tryCleanup,
} from './helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function requireId(row: Record<string, unknown>, label: string): number {
  const id = Number(row.id);
  if (!Number.isInteger(id) || id <= 0) {
    throw new Error(`${label}：未取到有效 id，实际 ${JSON.stringify(row)}`);
  }
  return id;
}

// ───────────── 十进制字符串比较（DECIMAL 出参为字符串，禁浮点、禁二次舍入）─────────────
function requireDecString(v: unknown, label: string): string {
  if (typeof v !== 'string') {
    throw new Error(`${label}：后端 Decimal 出参必须是字符串，实际类型=${typeof v} 值=${JSON.stringify(v)}`);
  }
  const text = v.trim();
  if (!/^-?\d+(?:\.\d+)?$/.test(text)) throw new Error(`${label}：无法按十进制解析，raw=${JSON.stringify(v)}`);
  return text;
}
function decToBigInt(text: string): { v: bigint; s: number } {
  const m = /^(-?)(\d+)(?:\.(\d+))?$/.exec(text);
  if (!m) throw new Error(`十进制解析失败：${text}`);
  const sign = m[1] === '-' ? -1n : 1n;
  return { v: sign * BigInt(`${m[2]}${m[3] ?? ''}`), s: (m[3] ?? '').length };
}
function expectDecEquals(actual: unknown, expected: string, label: string): void {
  const a = decToBigInt(requireDecString(actual, label));
  const b = decToBigInt(requireDecString(expected, label));
  const scale = Math.max(a.s, b.s);
  const av = a.v * 10n ** BigInt(scale - a.s);
  const bv = b.v * 10n ** BigInt(scale - b.s);
  expect(av === bv, `${label}：期望十进制值 ${expected}，实际 ${JSON.stringify(actual)}`).toBe(true);
}

async function newSupplier(page: Page, tag: string): Promise<{ id: number; name: string }> {
  const name = `152供应商${tag}${genCode('S')}`;
  const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/suppliers', { supplier_name: name, supplier_type: 'normal' });
  const id = requireId(created, `供应商 ${name}`);
  CLEANUP.push({ path: `/purchase/suppliers/${id}`, label: `supplier#${id}` });
  return { id, name };
}

test.describe('152 供应商评估记录与评分聚合：逐字段回读 + 派生加权 + 越界拒绝', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('152-01 建记录→读视图→按供应商聚合评分→改记录：加权与平均分逐值对账，越界得分回校验码且零写入', async ({
    page,
  }) => {
    const supplier = await newSupplier(page, 'EV');

    // 自建评估指标（满分 100、权重 1，使 average_score 恰等于得分本身，避免换算歧义）
    const indicatorCode = genCode('EI152');
    const indicator = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/supplier-evaluations/indicators', {
      indicator_name: `152交付及时率${indicatorCode}`,
      indicator_code: indicatorCode,
      category: 'delivery',
      weight: '1',
      max_score: 100,
    });
    const indicatorId = requireId(indicator, '评估指标创建');

    // 负例（先做，证明非法值不落库）：得分 150 越界 [0,100] 应被 400/VALIDATION_ERROR 拒绝
    const bad = await apiCallExpectFail(page, 'POST', `/purchase/suppliers/${supplier.id}/evaluate`, {
      supplier_id: supplier.id,
      evaluation_period: '2026Q4',
      indicator_id: indicatorId,
      score: '150',
    });
    expect(bad.status, '越界得分应被 HTTP 拒绝').toBe(400);
    expect(failureCode(bad), '越界得分机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    // 拒绝后断零写入：该供应商尚无任何记录，聚合评分应如实 NOT_FOUND，证明越界请求未落半行
    const zeroWrite = await apiCallExpectFail(page, 'GET', `/purchase/supplier-evaluations/scores/${supplier.id}`);
    expect(failureCode(zeroWrite), '越界拒绝后不得有评估记录被聚合到').toBe(APP_ERROR_CODES.NOT_FOUND);

    // 正向建：得分 80、权重 1、满分 100 ⇒ 加权分=80×1/100=0.8，绑定建后响应逐字段核对
    const record = await apiCallRaw<Record<string, unknown>>(page, 'POST', `/purchase/suppliers/${supplier.id}/evaluate`, {
      supplier_id: supplier.id,
      evaluation_period: '2026Q4',
      indicator_id: indicatorId,
      score: '80',
      remark: '初评',
    });
    const recordId = requireId(record, '评估记录创建');
    CLEANUP.push({ path: `/purchase/supplier-evaluations/${recordId}`, label: `eval#${recordId}` });
    expectDecEquals(record.score, '80', '建记录响应回显得分');
    expectDecEquals(record.weighted_score, '0.8', '加权分=得分×权重/满分应落 0.8');
    expect(Number(record.max_score), '满分随指标落库').toBe(100);
    expect(Number(record.supplier_id), '记录锚定供应商').toBe(supplier.id);

    // 记录读回视图：父供应商名与指标名必须被 JOIN 富化回填，非用编码/id 顶替
    const viewRaw = await apiCallRaw(page, 'GET', `/purchase/supplier-evaluations/records/${recordId}`);
    const view = viewRaw as Record<string, unknown>;
    expect(Number(view.supplier_id), '视图记录供应商 id 回读').toBe(supplier.id);
    expect(String(view.evaluation_period), '评估周期回读').toBe('2026Q4');
    expect(String(view.supplier_name), '视图应回填父供应商真实名称').toBe(supplier.name);
    expect(String(view.indicator_name), '视图应回填指标名称').toBe(`152交付及时率${indicatorCode}`);
    expectDecEquals(view.score, '80', '视图得分回读');

    // 按供应商聚合评分：单条记录 average=80/100×100=80，等级 B（80..=89），记录数≥1
    const scoreRaw = await apiCallRaw(page, 'GET', `/purchase/supplier-evaluations/scores/${supplier.id}`);
    const score = scoreRaw as Record<string, unknown>;
    expectDecEquals(score.average_score, '80', '单条记录平均分应等于得分');
    expect(String(score.rating), '平均分 80 落等级 B').toBe('B');
    expect(Number(score.total_records) >= 1, '聚合应统计到至少一条记录').toBe(true);
    expect(String(score.supplier_name), '聚合出口回填供应商名').toBe(supplier.name);

    // 改记录：只动 score 与 remark，未提交的指标/周期应原样保留
    const updRes = await apiCallRaw<Record<string, unknown>>(page, 'PUT', `/purchase/supplier-evaluations/${recordId}`, { score: '95', remark: '复核上调' });
    expectDecEquals(updRes.score, '95', 'PUT 响应回显改后得分');
    const updRaw = await apiCallRaw(page, 'GET', `/purchase/supplier-evaluations/records/${recordId}`);
    const upd = updRaw as Record<string, unknown>;
    expectDecEquals(upd.score, '95', '得分改写落库');
    expect(String(upd.remark), '备注改写落库').toBe('复核上调');
    expect(Number(upd.indicator_id), '指标锚定不受改分影响').toBe(indicatorId);
    expect(String(upd.evaluation_period), '评估周期不受改分影响').toBe('2026Q4');

    // 删除链路：删记录后按 id 读回应为 NOT_FOUND，证明写删闭环非只信 200
    await apiCall(page, 'DELETE', `/purchase/supplier-evaluations/${recordId}`);
    const gone = await apiCallExpectFail(page, 'GET', `/purchase/supplier-evaluations/records/${recordId}`);
    expect(gone.status, '删除后记录应不可读').toBeGreaterThanOrEqual(400);
    expect(failureCode(gone), '删除后读记录机器码').toBe(APP_ERROR_CODES.NOT_FOUND);
  });
});
