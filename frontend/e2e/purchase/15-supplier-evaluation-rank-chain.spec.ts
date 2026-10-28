// 供应商评估指标到记录到排名链路契约 — 15 自建指标与新建供应商的零漂移回读
// 契约真相（逐段读后端源，非推测）：
//   POST purchase/supplier-evaluations/indicators（handler create_indicator，
//   service services/supplier_evaluation_service.rs 的 CreateEvaluationIndicatorRequest）
//   入参 indicator_name、indicator_code（重复码 400）、category、weight、max_score。
//   POST purchase/supplier-evaluations（handler create_evaluation，同一 Request，
//   create_evaluation_record）入参 supplier_id、evaluation_period、
//   indicator_id、score、remark 可选；供应商或指标行不存在均 404 NOT_FOUND；
//   score 超出 [0, max_score] 报 validation_displayable「得分 x 超出有效范围 [0, n]」
//   400 VALIDATION_ERROR 外显；落库补 max_score 与 weighted_score（score*weight/max_score），
//   evaluator_id 取会话用户。rust_decimal 序列化为 JSON 字符串，断言走 Number 数值化。
//   GET purchase/supplier-evaluations/{id}（get_evaluation 与 get_evaluation_record_view
//   get_evaluation_record_view）出参 EvaluationRecordView，JOIN 富化列 supplier_name、
//   indicator_name、evaluator_name 实查回填；id 不存在 404 文案「评估记录不存在」。
//   GET purchase/supplier-evaluations/rankings（get_rankings 与 get_supplier_rankings
//   get_supplier_rankings）出参 SupplierScoreResponse 数组，键 supplier_id、
//   supplier_name、average_score、total_records、rating、latest_evaluation_date，
//   按 average_score 降序截 limit（默认 10，clamp 1..100），留榜行 supplier_name 实查回填。
// 正向零漂移：新建供应商只有本用例一条记录，排名榜 average_score 恒等于该 score、
//   total_records 恒等于 1，不受存量数据干扰。
// 假绿防线：详情与排名回读逐字段比对提交值；负例钉 HTTP 码与机器码并读外显文案；
//   删后再读钉 404，不接受把删除成功当响应体断言。指标无删除路由，用 genCode 唯一码
//   隔离并在收尾说明登记。
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  tryCleanup,
} from '../flow/helpers';

interface IndicatorRow {
  id: number;
  indicator_code: string;
  indicator_name: string;
  weight: string | number;
  max_score: number;
}

interface EvaluationRecordModel {
  id: number;
  supplier_id: number;
  evaluation_period: string;
  indicator_id: number;
  score: string | number;
  max_score: number | null;
  weighted_score: string | number | null;
  evaluator_id: number | null;
  remark: string | null;
}

interface EvaluationRecordView extends EvaluationRecordModel {
  supplier_name: string | null;
  indicator_name: string | null;
  evaluator_name: string | null;
}

interface RankingRow {
  supplier_id: number;
  supplier_name: string | null;
  average_score: string | number;
  total_records: number;
  rating: string;
  latest_evaluation_date: string | null;
}

const CLEANUP: Array<{ path: string; label: string }> = [];

test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 自建供应商（seedSupplier 口径与 finance/04-ap-ar-settlement.spec.ts 一致） */
async function seedSupplier(page: import('@playwright/test').Page, name: string): Promise<number> {
  const res = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
    supplier_name: name,
    supplier_short_name: 'E2E评',
    contact_phone: '13800002222',
  });
  if (!res.data?.id) throw new Error(`建供应商失败：${JSON.stringify(res)}`);
  CLEANUP.push({ path: `/purchase/suppliers/${res.data.id}`, label: 'supplier' });
  return res.data.id;
}

/** 自建评估指标（weight=1、max_score=100，加权得分可精确预置为 score/100；指标无删除路由，唯一码隔离） */
async function seedIndicator(page: import('@playwright/test').Page): Promise<IndicatorRow> {
  const code = genCode('E2E-IND');
  const ind = await apiCallRaw<IndicatorRow>(
    page,
    'POST',
    '/purchase/supplier-evaluations/indicators',
    {
      indicator_name: `E2E 交付及时率 ${code}`,
      indicator_code: code,
      category: 'DELIVERY',
      weight: 1,
      max_score: 100,
    }
  );
  expect(ind.id, `指标创建应回 id，实际 ${JSON.stringify(ind)}`).toBeGreaterThan(0);
  expect(String(ind.indicator_code), `指标码应回显提交值 ${code}`).toBe(code);
  return ind;
}

test.describe('15 供应商评估排名链路契约', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('15-01 建指标评估后详情与排名零漂移回读，删后 404 闭环', async ({ page }) => {
    const supplierName = `E2E 评估供${genCode('S')}`;
    const supplierId = await seedSupplier(page, supplierName);
    const indicator = await seedIndicator(page);

    const period = genCode('E2E-Q4');
    const created = await apiCallRaw<EvaluationRecordModel>(
      page,
      'POST',
      '/purchase/supplier-evaluations',
      {
        supplier_id: supplierId,
        evaluation_period: period,
        indicator_id: indicator.id,
        score: 88.5,
        remark: 'e2e 契约用例',
      }
    );
    expect(
      Number(created.id),
      `评估记录应回自增 id，实际 ${JSON.stringify(created)}`
    ).toBeGreaterThan(0);
    expect(Number(created.score), '落库 score 应等于提交 88.5').toBe(88.5);
    expect(Number(created.max_score), '落库 max_score 应取指标行 100').toBe(100);
    // weighted_score 的线格式契约：服务侧按 score*weight/max_score 得 0.885 不取整，落库列是
    // DECIMAL(10,2)（v15 域建表口径）由数据库半进位存 0.89，rust_decimal 出参为 JSON 字符串。
    // 断言只认这条序列化事实，绝不在测试里再实现一遍舍入（换个入参就会与库侧不同向造偶发红）。
    expect(
      String(created.weighted_score),
      `weighted_score 应按列精度两位以字符串出参，实际 ${JSON.stringify(created.weighted_score)}`
    ).toBe('0.89');
    CLEANUP.push({ path: `/purchase/supplier-evaluations/${created.id}`, label: 'evaluation' });

    // 详情回读逐字段零漂移（含 JOIN 富化列实查回填契约）
    const view = await apiCallRaw<EvaluationRecordView>(
      page,
      'GET',
      `/purchase/supplier-evaluations/${created.id}`
    );
    expect(Number(view.supplier_id), '详情 supplier_id 应等于种子供应商').toBe(supplierId);
    expect(String(view.evaluation_period), '详情 evaluation_period 应回显提交值').toBe(period);
    expect(Number(view.indicator_id), '详情 indicator_id 应等于自建指标').toBe(indicator.id);
    expect(
      String(view.supplier_name),
      `详情 supplier_name 应实查回填种子名，实际 ${view.supplier_name}`
    ).toBe(supplierName);
    expect(
      String(view.indicator_name),
      `详情 indicator_name 应实查回填指标名，实际 ${view.indicator_name}`
    ).toBe(indicator.indicator_name);

    // 排名口零漂移，新建供应商仅本用例一条记录，均值与条数确定
    const rankings = await apiCallRaw<RankingRow[]>(
      page,
      'GET',
      '/purchase/supplier-evaluations/rankings?limit=100'
    );
    expect(Array.isArray(rankings), '排名应返回数组').toBe(true);
    const row = rankings.find(r => Number(r.supplier_id) === supplierId);
    expect(row, `排名榜应含新建供应商 ${supplierId}，实际条数 ${rankings.length}`).toBeDefined();
    // average_score 不是原始分的平均，而是 sum(weighted_score)/sum(weight)*100 反推：
    // weighted_score 落库列是 DECIMAL(10,2)，0.885 存成 0.89，反推即得 89。
    // 本断言钉这条推导事实。副产品是权重的有效分辨率被压到 1 分（88.5 与 89.4 同名），
    // 该口径由列精度裁定决定，改与不改都不该让这条断言含糊通过。
    expect(Number(row!.average_score), '排名均值应由两位存储的加权分反推，权重 1 时得 89').toBe(89);
    expect(Number(row!.total_records), '新建供应商评估记录数应为 1').toBe(1);
    expect(String(row!.supplier_name), '排名 supplier_name 应实查回填').toBe(supplierName);

    // 删除后详情口 404，删除不是软删视图（active_model.delete 硬删）
    await apiCall(page, 'DELETE', `/purchase/supplier-evaluations/${created.id}`);
    const gone = await apiCallExpectFail(
      page,
      'GET',
      `/purchase/supplier-evaluations/${created.id}`
    );
    expect(gone.status, `删除后详情应 404，实际 ${gone.status}`).toBe(404);
    expect(failureCode(gone), `应 NOT_FOUND，实际 code=${gone.code}`).toBe('NOT_FOUND');
  });

  test('15-02 score 超出指标满分拒绝，400 VALIDATION_ERROR 外显范围', async ({ page }) => {
    const supplierId = await seedSupplier(page, `E2E 越界供${genCode('S')}`);
    const indicator = await seedIndicator(page);

    const fail = await apiCallExpectFail(page, 'POST', '/purchase/supplier-evaluations', {
      supplier_id: supplierId,
      evaluation_period: genCode('E2E-Q5'),
      indicator_id: indicator.id,
      score: 101,
    });
    expect(fail.status, `越界得分应 400，实际 ${fail.status}`).toBe(400);
    expect(failureCode(fail), `应 VALIDATION_ERROR，实际 code=${fail.code}`).toBe(
      'VALIDATION_ERROR'
    );
    expect(
      String(fail.message ?? ''),
      `validation_displayable 应外显有效范围 [0, 100]，实际 message=${fail.message}`
    ).toContain('超出有效范围');
  });

  test('15-03 供应商或指标不存在拒绝，404 NOT_FOUND', async ({ page }) => {
    const indicator = await seedIndicator(page);
    const ghostSupplier = await apiCallExpectFail(page, 'POST', '/purchase/supplier-evaluations', {
      supplier_id: 999999999,
      evaluation_period: genCode('E2E-Q6'),
      indicator_id: indicator.id,
      score: 50,
    });
    expect(ghostSupplier.status, `不存在供应商应 404，实际 ${ghostSupplier.status}`).toBe(404);
    expect(failureCode(ghostSupplier)).toBe('NOT_FOUND');

    const ghostIndicator = await apiCallExpectFail(page, 'POST', '/purchase/supplier-evaluations', {
      supplier_id: await seedSupplier(page, `E2E 缺指供${genCode('S')}`),
      evaluation_period: genCode('E2E-Q7'),
      indicator_id: 999999999,
      score: 50,
    });
    expect(ghostIndicator.status, `不存在指标应 404，实际 ${ghostIndicator.status}`).toBe(404);
    expect(failureCode(ghostIndicator)).toBe('NOT_FOUND');
  });
});
