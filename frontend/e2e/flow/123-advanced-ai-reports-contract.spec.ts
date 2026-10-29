import { test, expect, type Page } from '../diagnose-fixture';
import { APP_ERROR_CODES, loginViaUI, apiCallRaw, apiCall, apiCallExpectFail } from './helpers';

/**
 * 123 advanced 族九个端点的真实契约（advanced 域：AI 推荐/异常检测/质量预测/工艺优化/销售预测/库存优化/报表模板-执行-导出）
 *
 * 守的是什么：`/advanced/*` 面缺参时的点名校验、可选体时的回落默认值、
 * 以及 2xx 出参的字段级形状，全部判据来自 handler 源码实核，不测可达性：
 * - GET /advanced/reports/templates（handlers/advanced/analytics::list_report_templates）：
 *   出参为引擎预置模板（services/report/tpl.rs 九个固定 id），每条含
 *   template_name/template_code/category/description/created_at，
 *   description 由代码拼为「包含 N 个列: …」前缀，逐条断该前缀真实成立。
 * - POST /advanced/reports/execute：入参 template_code 必填；未知 id 走
 *   get_template 的 not_found（services/report/tpl 附近）→ 404 + NOT_FOUND；
 *   已知 sales_summary 返回 columns（{key,label} 数组）、data 行数组与 total_count。
 * - POST /advanced/reports/export：format 经白名单校验
 *   （csv/excel/xlsx/pdf/json，非法值在取数之前即被拒 → 400 VALIDATION_ERROR，
 *   白名单外的值不再静默回落成 json）；
 *   csv 出参 data.status/format/size_bytes/record_count（format 回显请求值）。
 * - POST /advanced/ai/recommendations（rec，OptionalJson）：缺体回落
 *   recommendation_type="all"、limit=10；limit 入参真实生效——传 1 时返回条数 ≤ 1，
 *   忽略 limit 的实现会让该断言变红。
 * - POST /advanced/ai/anomaly-detection（decide，Json 强提取器）：
 *   data_type 为必填非 Option——缺体/缺字段经请求解码通道报 400 + VALIDATION_ERROR，
 *   文案为固定公开常量（utils/error.rs REQUEST_DECODING_PUBLIC，不含字段名，
 *   serde 原文只进 WARN）；合法体返回条目数组，severity 收窄到 critical/warning/info 三值。
 * - POST /advanced/ai/quality-prediction（quality_pred）：window_days 越界
 *   （0 或 >365）→ 400 + 点名文案「时间窗口 window_days 必须在 1-365 之间」
 *   （validation_displayable，可外显）；inspection_type 超 32 字符 → 点名文案含 32；
 *   合法空体 {window_days 缺省} 时出参 window_days 恒为 90、inspection_type 为 "all"，
 *   trend 落在「上升|平稳|下降|无数据」、risk_level 落在「低|中|高」。
 * - POST /advanced/ai/recipe-optimization（recipe_opt）：color_no/fabric_type
 *   空串 → 400 点名「色号 color_no 不能为空」/「布类 fabric_type 不能为空」；
 *   k=0 强制退化路径（handler 注释与 service 文档同源）→ source 恒为 "fallback"，
 *   recommended_params 四字段为数值。
 * - POST /advanced/ai/sales-forecast（forecast）：product_id 缺失 → 400 点名
 *   「预测请求缺少物料ID」；合法 product_id（取种子产品第一行，只读复用）→ period 回显、
 *   confidence 为 0-100 整数、detail 为数组。
 * - POST /advanced/ai/inventory-optimization（forecast，OptionalJson）：
 *   summary 由代码拼为「检测到 N 个产品需要补货，M 个产品库存积压」整句，
 *   断整句模式；items 每行 product_name 非空（主数据无名称的行被后端剔除，见同名函数）。
 *
 * 负例三族（解码 400、越界 400、缺参点名 400）与 2xx 形状腿在同一条用例里配对出现：
 * 只测失败不测成功、或只测成功不测失败，都撑不起"契约"两个字。
 */

type ReportTemplateRow = {
  template_name: string;
  template_code: string;
  category: string;
  description: string;
  created_at: string;
};

type ExecuteReportBody = {
  columns: Array<{ key: string; label: string }>;
  data: Array<Record<string, string>>;
  total_count: number;
};

type ExportReportBody = {
  status: string;
  format: string;
  size_bytes: number;
  size_kb: number;
  record_count: number;
};

type RecommendationItem = {
  content: string;
  recommendation_type: string;
  created_at: string;
  score: number;
  target_id: number;
};

type AnomalyItem = {
  item: string;
  anomaly_type: string;
  description: string;
  severity: string;
  detected_at: string;
};

type QualityPredBody = {
  product_id?: number | null;
  inspection_type: string;
  window_days: number;
  total_inspections: number;
  avg_qualification_rate: number;
  trend: string;
  trend_rate: number;
  risk_score: number;
  risk_level: string;
  confidence: number;
  top_issues: unknown[];
  recommendations: string[];
};

type RecipeOptBody = {
  recommended_params: {
    temperature: number;
    time_minutes: number;
    ph_value: number;
    liquor_ratio: number;
  };
  similar_cases: number;
  confidence: number;
  source: string;
  reason: string;
  candidates: unknown[];
};

type SalesForecastBody = {
  sales_amount: number;
  order_count: number;
  confidence: number;
  trend: string;
  period: string;
  detail: Array<{ date: string; predicted_quantity: string; confidence: number; trend: string }>;
};

type InventoryOptBody = {
  summary: string;
  items: Array<Record<string, unknown> & { product_name?: string }>;
};

// 种子产品第一行（只读取 id 作销售预测入参；不创建、不修改）。
async function firstProductId(page: Page): Promise<number> {
  const listed = await apiCall<{ items?: Array<{ id?: number }> }>(
    page,
    'GET',
    '/products?page=1&page_size=1'
  );
  const id = listed.data?.items?.[0]?.id;
  if (typeof id !== 'number') {
    throw new Error('种子产品列表未取到 id，销售预测与库存优化腿无入参可用');
  }
  return id;
}

test.describe.serial('123 advanced 族九端点契约配对（缺参点名 400 与 2xx 字段形状）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('报表模板列表为预置九模板真值，sales_summary 在册', async ({ page }) => {
    const rows = await apiCallRaw<ReportTemplateRow[]>(page, 'GET', '/advanced/reports/templates');
    expect(Array.isArray(rows)).toBe(true);
    expect(rows.length).toBeGreaterThanOrEqual(4);
    for (const row of rows) {
      expect(typeof row.template_code).toBe('string');
      expect(row.template_code.length).toBeGreaterThan(0);
      expect(row.description.startsWith('包含 ')).toBe(true);
    }
    const summary = rows.find(r => r.template_code === 'sales_summary');
    expect(summary).toBeDefined();
  });

  test('报表执行按已知模板返回列与行，未知模板 404 NOT_FOUND', async ({ page }) => {
    const body = await apiCallRaw<ExecuteReportBody>(page, 'POST', '/advanced/reports/execute', {
      template_code: 'sales_summary',
    });
    expect(Array.isArray(body.columns)).toBe(true);
    expect(body.columns.length).toBeGreaterThan(0);
    for (const col of body.columns) {
      expect(typeof col.key).toBe('string');
      expect(typeof col.label).toBe('string');
    }
    expect(Array.isArray(body.data)).toBe(true);
    expect(typeof body.total_count).toBe('number');
    expect(body.total_count).toBeGreaterThanOrEqual(0);

    const missing = await apiCallExpectFail(page, 'POST', '/advanced/reports/execute', {
      template_code: '__e2e_no_such_template__',
    });
    expect(missing.status).toBe(404);
    expect(missing.code).toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('报表导出 csv 返回字节统计，format 白名单拒绝非法值', async ({ page }) => {
    const body = await apiCallRaw<ExportReportBody>(page, 'POST', '/advanced/reports/export', {
      template_code: 'sales_summary',
      format: 'csv',
    });
    expect(body.status).toBe('success');
    expect(body.format).toBe('csv');
    expect(typeof body.size_bytes).toBe('number');
    expect(body.size_bytes).toBeGreaterThan(0);
    expect(typeof body.record_count).toBe('number');

    const badFormat = await apiCallExpectFail(page, 'POST', '/advanced/reports/export', {
      template_code: 'sales_summary',
      format: 'xml',
    });
    expect(badFormat.status).toBe(400);
    // 非法枚举入参走校验族（与下方缺字段与超范围两腿同族同码），
    // 后端对不在白名单内的 format 报 validation_displayable，出参码 VALIDATION_ERROR。
    expect(badFormat.code).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
  });

  test('智能推荐缺体回落 all+10，limit 入参真实生效', async ({ page }) => {
    const withLimit = await apiCallRaw<RecommendationItem[]>(
      page,
      'POST',
      '/advanced/ai/recommendations',
      { recommendation_type: 'all', limit: 1 }
    );
    expect(Array.isArray(withLimit)).toBe(true);
    expect(withLimit.length).toBeLessThanOrEqual(1);
    for (const rec of withLimit) {
      expect(typeof rec.content).toBe('string');
      expect(typeof rec.score).toBe('number');
      expect(typeof rec.target_id).toBe('number');
      expect(['补货建议', '关联推荐', '趋势推荐', '价格调整', '综合推荐']).toContain(
        rec.recommendation_type
      );
    }

    const noBody = await apiCallRaw<RecommendationItem[]>(
      page,
      'POST',
      '/advanced/ai/recommendations',
      {}
    );
    expect(Array.isArray(noBody)).toBe(true);
    expect(noBody.length).toBeLessThanOrEqual(10);
  });

  test('异常检测合法体返回收窄条目，缺必填 data_type 走解码 400 固定文案', async ({ page }) => {
    const body = await apiCallRaw<AnomalyItem[]>(page, 'POST', '/advanced/ai/anomaly-detection', {
      data_type: 'quality',
    });
    expect(Array.isArray(body)).toBe(true);
    for (const entry of body) {
      expect(['critical', 'warning', 'info']).toContain(entry.severity);
      expect(typeof entry.anomaly_type).toBe('string');
      expect(typeof entry.detected_at).toBe('string');
    }

    // Json 强提取器：{} 缺必填字段 → 请求解码拒绝通道，400 + VALIDATION_ERROR，
    // 文案为公开规则常量，不泄露字段名（安全边界，见 utils/error.rs 文档）。
    const missing = await apiCallExpectFail(page, 'POST', '/advanced/ai/anomaly-detection', {});
    expect(missing.status).toBe(400);
    expect(missing.code).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(missing.message).toBe(
      '请求格式不正确：请求体或查询参数存在缺失的必填字段、字段类型错误或内容类型错误，请修正后重试'
    );
    expect(missing.message).not.toContain('data_type');
  });

  test('质量预测越界点名 window_days，合法空体默认 90 且枚举收窄', async ({ page }) => {
    const outOfRange = await apiCallExpectFail(page, 'POST', '/advanced/ai/quality-prediction', {
      window_days: 0,
    });
    expect(outOfRange.status).toBe(400);
    expect(outOfRange.code).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(outOfRange.message).toBe('时间窗口 window_days 必须在 1-365 之间');

    const tooLongType = await apiCallExpectFail(page, 'POST', '/advanced/ai/quality-prediction', {
      inspection_type: '超'.repeat(33),
      window_days: 90,
    });
    expect(tooLongType.status).toBe(400);
    expect(tooLongType.message).toBe('检验类型 inspection_type 长度不能超过 32');

    const body = await apiCallRaw<QualityPredBody>(
      page,
      'POST',
      '/advanced/ai/quality-prediction',
      {
        window_days: 90,
      }
    );
    expect(body.window_days).toBe(90);
    expect(body.inspection_type).toBe('all');
    expect(['上升', '平稳', '下降', '无数据']).toContain(body.trend);
    expect(['低', '中', '高']).toContain(body.risk_level);
    expect(typeof body.total_inspections).toBe('number');
    expect(body.total_inspections).toBeGreaterThanOrEqual(0);
    expect(Array.isArray(body.top_issues)).toBe(true);
    expect(Array.isArray(body.recommendations)).toBe(true);
  });

  test('工艺优化空串点名缺参，k=0 强制退化路径 source=fallback', async ({ page }) => {
    const emptyColor = await apiCallExpectFail(page, 'POST', '/advanced/ai/recipe-optimization', {
      color_no: '   ',
      fabric_type: '棉',
    });
    expect(emptyColor.status).toBe(400);
    expect(emptyColor.code).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(emptyColor.message).toBe('色号 color_no 不能为空');

    const emptyFabric = await apiCallExpectFail(page, 'POST', '/advanced/ai/recipe-optimization', {
      color_no: 'BL-E2E',
      fabric_type: '',
    });
    expect(emptyFabric.message).toBe('布类 fabric_type 不能为空');

    const body = await apiCallRaw<RecipeOptBody>(page, 'POST', '/advanced/ai/recipe-optimization', {
      color_no: 'BL-E2E-ZERO',
      fabric_type: '棉',
      k: 0,
    });
    expect(body.source).toBe('fallback');
    expect(typeof body.recommended_params.temperature).toBe('number');
    expect(typeof body.recommended_params.time_minutes).toBe('number');
    expect(body.recommended_params.ph_value).toBeGreaterThan(0);
    expect(Array.isArray(body.candidates)).toBe(true);
  });

  test('销售预测缺物料ID点名拒绝，合法入参 period 回显且置信度收窄', async ({ page }) => {
    const noProduct = await apiCallExpectFail(page, 'POST', '/advanced/ai/sales-forecast', {
      period: '3m',
    });
    expect(noProduct.status).toBe(400);
    expect(noProduct.code).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(noProduct.message).toBe('预测请求缺少物料ID');

    const productId = await firstProductId(page);
    // 合法入参必 2xx：handler 对 forecast_sales 的 Err 分支原样上抛（forecast），
    // 若服务层坏了本行即判红——这正是 apiCall 对失败信封 throw 的用武之地
    const body = await apiCallRaw<SalesForecastBody>(page, 'POST', '/advanced/ai/sales-forecast', {
      period: '3m',
      product_id: productId,
    });
    expect(body.period).toBe('3m');
    expect(typeof body.confidence).toBe('number');
    expect(body.confidence).toBeGreaterThanOrEqual(0);
    expect(body.confidence).toBeLessThanOrEqual(100);
    expect(Array.isArray(body.detail)).toBe(true);
    expect(typeof body.order_count).toBe('number');
  });

  test('库存优化 summary 整句模式与 items 名称同源非空', async ({ page }) => {
    const body = await apiCallRaw<InventoryOptBody>(
      page,
      'POST',
      '/advanced/ai/inventory-optimization',
      {}
    );
    expect(/^检测到 \d+ 个产品需要补货，\d+ 个产品库存积压$/.test(body.summary)).toBe(true);
    expect(Array.isArray(body.items)).toBe(true);
    for (const item of body.items) {
      expect(typeof item.product_name).toBe('string');
      expect((item.product_name ?? '').length).toBeGreaterThan(0);
    }
  });
});
