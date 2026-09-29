// AI 模型管理 E2E 套件 — 03 模型版本 + 评估指标 + 漂移检测 + 准确率报告
// 覆盖范围：
//   - 创建模型版本 → GET 回读确认字段落库
//   - 创建模型评估 → GET 回读指标值（非仅端点可达）
//   - 漂移检测端点返回真实结构（has_drift / expected_accuracy / actual_accuracy / drift_score）
//   - 准确率报告 GET 列表可达且结构正确
//   - 非法评估指标（accuracy > 1.0）→ 400 VALIDATION_ERROR
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  tryCleanup,
} from '../flow/helpers';

const CLEANUP: Array<{ method: 'DELETE' | 'POST'; path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) {
    await tryCleanup(page, c.method as 'DELETE', c.path, c.label);
  }
  CLEANUP.length = 0;
});

test.describe('03 AI 模型版本与评估指标', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('03-01 创建模型版本并 GET 回读确认字段落库', async ({ page }) => {
    const modelName = `e2e-model-${genCode('m')}`;
    const version = '1.0.0';
    const created = await apiCall<{
      id?: number;
      model_name?: string;
      version?: string;
      status?: string;
      approval_status?: string;
    }>(page, 'POST', '/ai-models/versions', {
      model_name: modelName,
      version,
      algorithm: 'random_forest',
      parameters_json: { max_depth: 10, n_estimators: 100 },
      training_dataset_size: 5000,
      change_reason: 'E2E 测试建版',
    });
    expect(created.data?.id, `创建模型版本失败：${JSON.stringify(created)}`).toBeTruthy();
    const versionId = created.data!.id!;
    CLEANUP.push({
      method: 'DELETE',
      path: `/ai-models/versions/${versionId}`,
      label: 'ai_model_version',
    });

    // GET 回读：通过列表端点按 model_name 过滤
    const list = await apiCallRaw<
      Array<{ id: number; model_name: string; status: string; approval_status: string }>
    >(page, 'GET', `/ai-models/versions?model_name=${encodeURIComponent(modelName)}`);
    expect(Array.isArray(list), '列表响应应为数组').toBe(true);
    const found = list.find(v => v.id === versionId);
    expect(found, `列表中应包含刚创建的版本 id=${versionId}`).toBeDefined();
    expect(found!.model_name).toBe(modelName);
    // 新建版本默认 status=draft, approval_status=pending
    expect(found!.status).toBe('draft');
    expect(found!.approval_status).toBe('pending');
  });

  test('03-02 创建模型评估并 GET 回读真实指标值', async ({ page }) => {
    // 先建一个模型版本作为评估目标
    const modelName = `e2e-eval-${genCode('m')}`;
    const verResp = await apiCall<{ id?: number }>(page, 'POST', '/ai-models/versions', {
      model_name: modelName,
      version: '1.0.0',
      algorithm: 'xgboost',
    });
    const versionId = verResp.data!.id!;
    CLEANUP.push({
      method: 'DELETE',
      path: `/ai-models/versions/${versionId}`,
      label: 'ai_model_version',
    });

    // 创建评估记录，指标值设为确定性数值
    const accuracy = 0.92;
    const precision = 0.88;
    const recall = 0.85;
    const f1Score = 0.865;
    const sampleCount = 1200;

    const evalResp = await apiCall<{
      id?: number;
      accuracy?: number;
      precision?: number;
      recall?: number;
      f1_score?: number;
      sample_count?: number;
    }>(page, 'POST', '/ai-models/evaluations', {
      model_version_id: versionId,
      accuracy,
      precision,
      recall,
      f1_score: f1Score,
      sample_count: sampleCount,
      evaluation_report: 'E2E 测试评估报告',
    });
    expect(evalResp.data?.id, `创建评估失败：${JSON.stringify(evalResp)}`).toBeTruthy();
    // 断言 POST 响应本身携带的指标值
    expect(Number(evalResp.data!.accuracy)).toBeCloseTo(accuracy, 2);
    expect(Number(evalResp.data!.precision)).toBeCloseTo(precision, 2);
    expect(Number(evalResp.data!.sample_count)).toBe(sampleCount);

    // GET 回读评估列表，断言指标值与写入一致
    const evals = await apiCallRaw<
      Array<{
        id: number;
        accuracy: number | null;
        precision: number | null;
        recall: number | null;
        sample_count: number;
      }>
    >(page, 'GET', `/ai-models/evaluations/${versionId}`);
    expect(Array.isArray(evals), '评估列表应为数组').toBe(true);
    expect(evals.length).toBeGreaterThanOrEqual(1);
    const found = evals.find(e => e.id === evalResp.data!.id);
    expect(found, 'GET 回读列表中应包含刚创建的评估').toBeDefined();
    expect(Number(found!.accuracy)).toBeCloseTo(accuracy, 2);
    expect(Number(found!.precision)).toBeCloseTo(precision, 2);
    expect(Number(found!.recall)).toBeCloseTo(recall, 2);
    expect(found!.sample_count).toBe(sampleCount);
  });

  test('03-03 漂移检测端点返回真实结构', async ({ page }) => {
    // 建版本 + 两条评估以触发漂移计算逻辑
    const modelName = `e2e-drift-${genCode('m')}`;
    const verResp = await apiCall<{ id?: number }>(page, 'POST', '/ai-models/versions', {
      model_name: modelName,
      version: '1.0.0',
      algorithm: 'neural_network',
    });
    const versionId = verResp.data!.id!;
    CLEANUP.push({
      method: 'DELETE',
      path: `/ai-models/versions/${versionId}`,
      label: 'ai_model_version',
    });

    // 历史评估 accuracy=0.95（baseline）
    await apiCall(page, 'POST', '/ai-models/evaluations', {
      model_version_id: versionId,
      accuracy: 0.95,
      sample_count: 500,
    });
    // 最新评估 accuracy=0.80（下降 > 5%，应触发漂移）
    await apiCall(page, 'POST', '/ai-models/evaluations', {
      model_version_id: versionId,
      accuracy: 0.8,
      sample_count: 500,
    });

    const drift = await apiCallRaw<{
      has_drift: boolean;
      expected_accuracy: number | null;
      actual_accuracy: number | null;
      drift_score: number;
    }>(page, 'GET', `/ai-models/evaluations/${versionId}/drift`);
    // 断言真实结构
    expect(typeof drift.has_drift).toBe('boolean');
    expect(typeof drift.drift_score).toBe('number');
    // 最新 accuracy=0.80, baseline=0.95 → drift = (0.80-0.95)/0.95*100 ≈ -15.8%
    // 超过 -5% 阈值，has_drift 应为 true
    expect(drift.has_drift, `accuracy 从 0.95 降至 0.80（降幅 > 5%）应触发漂移`).toBe(true);
    expect(drift.drift_score).toBeLessThan(-5);
    expect(Number(drift.actual_accuracy)).toBeCloseTo(0.8, 1);
    expect(Number(drift.expected_accuracy)).toBeCloseTo(0.95, 1);
  });

  test('03-04 准确率报告端点返回真实结构', async ({ page }) => {
    const reports = await apiCallRaw<
      Array<{ id: number; report_period: string; total_predictions: number }>
    >(page, 'GET', '/ai-models/accuracy-reports?limit=5');
    expect(Array.isArray(reports), '准确率报告应为数组').toBe(true);
    // CI 环境可能无预置报告数据；有则断言结构正确
    for (const r of reports) {
      expect(typeof r.id).toBe('number');
      expect(typeof r.report_period).toBe('string');
      expect(typeof r.total_predictions).toBe('number');
    }
  });

  test('03-05 非法评估指标（accuracy > 1.0）返回 400', async ({ page }) => {
    // 先建版本
    const modelName = `e2e-bad-${genCode('m')}`;
    const verResp = await apiCall<{ id?: number }>(page, 'POST', '/ai-models/versions', {
      model_name: modelName,
      version: '1.0.0',
      algorithm: 'logistic_regression',
    });
    const versionId = verResp.data!.id!;
    CLEANUP.push({
      method: 'DELETE',
      path: `/ai-models/versions/${versionId}`,
      label: 'ai_model_version',
    });

    // accuracy=1.5 超出 [0, 1] 范围 → 应被 validate_metric_range 拒绝
    const fail = await apiCallExpectFail(page, 'POST', '/ai-models/evaluations', {
      model_version_id: versionId,
      accuracy: 1.5,
      sample_count: 100,
    });
    expect(fail.status).toBe(400);
    expect(failureCode(fail)).toBe('VALIDATION_ERROR');
  });

  test('03-06 审批模型版本（pending → approved）状态变更回读', async ({ page }) => {
    const modelName = `e2e-approve-${genCode('m')}`;
    const verResp = await apiCall<{ id?: number; approval_status?: string }>(
      page,
      'POST',
      '/ai-models/versions',
      {
        model_name: modelName,
        version: '1.0.0',
        algorithm: 'svm',
      }
    );
    const versionId = verResp.data!.id!;
    CLEANUP.push({
      method: 'DELETE',
      path: `/ai-models/versions/${versionId}`,
      label: 'ai_model_version',
    });
    expect(verResp.data!.approval_status).toBe('pending');

    // 获取当前用户 ID
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');

    // 审批通过
    const approved = await apiCall<{ id: number; approval_status: string }>(
      page,
      'POST',
      `/ai-models/versions/${versionId}/approve`,
      { approved_by: me.id, approval_status: 'approved' }
    );
    expect(approved.data!.approval_status).toBe('approved');

    // GET 回读确认
    const list = await apiCallRaw<Array<{ id: number; approval_status: string }>>(
      page,
      'GET',
      `/ai-models/versions?model_name=${encodeURIComponent(modelName)}`
    );
    const found = list.find(v => v.id === versionId);
    expect(found!.approval_status, '审批后回读 approval_status 应为 approved').toBe('approved');
  });

  test('03-07 非法审批状态被 400 拒绝', async ({ page }) => {
    const modelName = `e2e-bad-approve-${genCode('m')}`;
    const verResp = await apiCall<{ id?: number }>(page, 'POST', '/ai-models/versions', {
      model_name: modelName,
      version: '1.0.0',
      algorithm: 'svm',
    });
    const versionId = verResp.data!.id!;
    CLEANUP.push({
      method: 'DELETE',
      path: `/ai-models/versions/${versionId}`,
      label: 'ai_model_version',
    });

    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');

    // approval_status='invalid_status' 非法
    const fail = await apiCallExpectFail(page, 'POST', `/ai-models/versions/${versionId}/approve`, {
      approved_by: me.id,
      approval_status: 'invalid_status',
    });
    expect(fail.status).toBe(400);
    expect(failureCode(fail)).toBe('VALIDATION_ERROR');
  });
});
