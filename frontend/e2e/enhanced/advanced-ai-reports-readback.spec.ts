// [法#3 范围锁定] 目标：glob='frontend/e2e/enhanced/advanced-ai-reports-readback.spec.ts'
// Advanced 分析域零覆盖补齐 E2E — 预置报表模板 报表执行 报表导出 六枚 AI 推断面
//
// 功能：把 advanced 域九枚在册端点钉成契约面真值链。报表三腿钉静态模板集
// 合与列装配形状与字节计量恒等式；AI 六腿按两类钉：入参门（displayable 逐文案）
// 钉确定性拒绝，形状合同（枚举词表 恒等式 上限截断 强制退化常量表）钉
// 输出装配面，不断言依赖库内容的具体数值。
// 调用方：路由注册在 backend/src/routes/analytics.rs 的 advanced 分组
// （nest 前缀 advanced），报表三腿在 backend/src/handlers/advanced/analytics.rs，
// AI 腿在 backend/src/handlers/advanced 的 decide forecast quality_pred rec
// recipe_opt，报表引擎在 backend/src/services/report，AI 服务在
// backend/src/services/ai；前端调用面为智能分析页。
// 入参：报表 execute 与 export 必带 template_code（export 另带 format）；
// sales-forecast 必带 period 与 product_id（缺物料 ID 即拒）；
// quality-prediction 的 window_days 限 1 到 365，inspection_type 限 32 字符；
// recipe-optimization 必带非空 color_no 与 fabric_type，k=0 强制退化路径；
// anomaly-detection 必带 data_type；recommendations 与 inventory-optimization
// 走 OptionalJson 可空体。
// 传给谁：报表腿读预置模板集合并查询报表数据源；export 生成字节后仅回
// 计量元数据，异步写导出审计；AI 腿读销售 库存 质检 染配历史聚合。
// 存什么：本文件只读，不落业务行；导出审计为 best-effort 异步写 audit_logs。
// 存哪里：预置模板在 services/report/tpl.rs 静态数组（九条：sales_summary
// sales_detail inventory_status purchase_summary ar_aging top_products
// customer_analysis profit_analysis inventory_turnover）。
// 本文件覆盖端点：GET advanced/reports/templates、POST advanced/reports/execute、
// POST advanced/reports/export、POST advanced/ai/sales-forecast、
// POST advanced/ai/inventory-optimization、POST advanced/ai/anomaly-detection、
// POST advanced/ai/recommendations、POST advanced/ai/quality-prediction、
// POST advanced/ai/recipe-optimization。
// 隔离事实：不造行不删行；sales-forecast 用不存在物料号走 fallback 分支，
// 分支输出长度由 period 映射决定（3m 到 90 条），与库内容无耦合的部分逐值钉死。
// 假绿防线：AI 输出数值随库漂移，故只钉三类可判红事实——形状（键集与类型）
// 词表（枚举值域闭包）恒等式（size_kb 与 size_bytes 之商 退化参数常量表）；
// 空结果循环不假称数据契约，缺参门与入参门逐文案或逐机器码钉。
// 脱敏事实：not_found 出参恒为资源未找到；validation_displayable 与 validator
// 文案外显真实原因；提取器缺参拒绝由中间件收口为 400 加 VALIDATION_ERROR。
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import { APP_ERROR_CODES, apiCallExpectFail, apiCallRaw, failureCode } from '../flow/helpers';

type Row = Record<string, unknown>;
const str = (v: unknown): string => String(v);
const num = (v: unknown): number => Number(String(v));

const PREDEFINED_TEMPLATE_CODES = [
  'sales_summary',
  'sales_detail',
  'inventory_status',
  'purchase_summary',
  'ar_aging',
  'top_products',
  'customer_analysis',
  'profit_analysis',
  'inventory_turnover',
];

test.describe('Advanced 报表与 AI 契约回读', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('AD-01 预置模板集合：九条 id 与列描述形状逐值钉', async ({ page }) => {
    const list = await apiCallRaw<Row[]>(page, 'GET', '/advanced/reports/templates');
    expect(Array.isArray(list), `模板应出数组，实际 ${JSON.stringify(list)}`).toBe(true);
    const codes = list.map(t => str(t.template_code)).sort();
    expect(codes, `预置模板集合应逐一对齐，实际 ${JSON.stringify(codes)}`).toEqual(
      [...PREDEFINED_TEMPLATE_CODES].sort()
    );
    for (const t of list) {
      expect(
        /^包含 \d+ 个列: .+$/.test(str(t.description)),
        `模板描述应带列数与列名清单，实际 ${t.description}`
      ).toBe(true);
      expect(str(t.template_name).length, `模板名应非空`).toBeGreaterThan(0);
      expect(str(t.category).length, `模板分类应非空`).toBeGreaterThan(0);
      expect(
        /^\d{4}-\d{2}-\d{2}$/.test(str(t.created_at)),
        `created_at 应为 YYYY-MM-DD（装配日），实际 ${t.created_at}`
      ).toBe(true);
    }
  });

  test('AD-02 报表执行：列与行同构且 total_count 等于行数，未知模板 404', async ({ page }) => {
    const result = await apiCallRaw<Row>(page, 'POST', '/advanced/reports/execute', {
      template_code: 'sales_summary',
    });
    const columns = result.columns as Row[];
    const rows = result.data as Row[];
    expect(
      Array.isArray(columns) && columns.length,
      `sales_summary 应装配非空列，实际 ${JSON.stringify(result.columns)}`
    ).toBeTruthy();
    for (const c of columns) {
      expect(
        str(c.key).length > 0 && str(c.label).length > 0,
        `列应带 key 与 label，实际 ${JSON.stringify(c)}`
      ).toBe(true);
    }
    expect(Array.isArray(rows), `data 应出行数组，实际 ${JSON.stringify(result.data)}`).toBe(true);
    // 引擎恒等式：total_count 等于装配行数（rows.len 同源）
    expect(
      num(result.total_count),
      `total_count 应等于 data 行数，实际 ${result.total_count} 对 ${rows.length}`
    ).toBe(rows.length);
    for (const r of rows) {
      for (const c of columns) {
        expect(
          Object.prototype.hasOwnProperty.call(r, str(c.key)),
          `行应含列键 ${c.key}，实际 ${JSON.stringify(r)}`
        ).toBe(true);
      }
    }

    // 缺参门：必填 template_code 缺失走提取器拒绝统一信封
    const miss = await apiCallExpectFail(page, 'POST', '/advanced/reports/execute', {});
    expect(miss.status, `缺 template_code 应恰 400，实际 ${miss.status}`).toBe(400);
    expect(failureCode(miss)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 未知模板：not_found 脱敏，404 加 NOT_FOUND 加固定文案
    const unknown = await apiCallExpectFail(page, 'POST', '/advanced/reports/execute', {
      template_code: 'e2e_ghost_template',
    });
    expect(unknown.status, `未知模板应恰 404，实际 ${unknown.status}`).toBe(404);
    expect(failureCode(unknown)).toBe(APP_ERROR_CODES.NOT_FOUND);
    expect(String(unknown.message ?? ''), `not_found 出参必须脱敏`).toBe('资源未找到');
  });

  test('AD-03 报表导出：计量恒等式与格式回显，未知模板 404', async ({ page }) => {
    const exported = await apiCallRaw<Row>(page, 'POST', '/advanced/reports/export', {
      template_code: 'sales_summary',
      format: 'csv',
    });
    expect(str(exported.status), `导出状态应为 success，实际 ${JSON.stringify(exported)}`).toBe(
      'success'
    );
    expect(str(exported.format), `format 应回显提交值`).toBe('csv');
    const sizeBytes = num(exported.size_bytes);
    expect(Number.isInteger(sizeBytes) && sizeBytes > 0, `csv 字节应正整数`).toBe(true);
    // 计量恒等式：size_kb 必为 size_bytes 整除 1024 的商
    expect(
      num(exported.size_kb),
      `size_kb 应等于 floor(size_bytes/1024)，实际 ${exported.size_kb} 对 ${sizeBytes}`
    ).toBe(Math.floor(sizeBytes / 1024));
    expect(
      Number.isInteger(num(exported.record_count)),
      `record_count 应为整数行数，实际 ${exported.record_count}`
    ).toBe(true);

    // excel 格式腿：同一恒等式再钉一次（格式映射另一分支）
    const xl = await apiCallRaw<Row>(page, 'POST', '/advanced/reports/export', {
      template_code: 'inventory_status',
      format: 'excel',
    });
    expect(str(xl.format), `excel 腿 format 回显`).toBe('excel');
    expect(num(xl.size_kb)).toBe(Math.floor(num(xl.size_bytes) / 1024));

    const unknown = await apiCallExpectFail(page, 'POST', '/advanced/reports/export', {
      template_code: 'e2e_ghost_template',
      format: 'csv',
    });
    expect(unknown.status).toBe(404);
    expect(failureCode(unknown)).toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('AD-04 销售预测：缺物料 ID 门与 3m 腿的确定性行数与置信度值域', async ({ page }) => {
    // 入参门：product_id 缺失即拒（validation_displayable，文案外显）
    const miss = await apiCallExpectFail(page, 'POST', '/advanced/ai/sales-forecast', {
      period: '3m',
    });
    expect(miss.status, `缺物料 ID 应恰 400，实际 ${miss.status}`).toBe(400);
    expect(failureCode(miss)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      String(miss.message ?? ''),
      `displayable 出参应外显缺料原因，实际 ${miss.message}`
    ).toContain('预测请求缺少物料ID');

    // 正腿：不存在物料无日销史（少于 7 天）走全局退化分支——
    // 分支逐日 push 由 period 映射决定（3m 到 90 天 90 行），detail 截前 30 行，
    // 置信与趋势是 fallback 常量（0.55 与 STABLE），与库内容无耦合，逐值钉死
    const fc = await apiCallRaw<Row>(page, 'POST', '/advanced/ai/sales-forecast', {
      period: '3m',
      product_id: 9999999,
    });
    expect(str(fc.period), `period 应回显 3m`).toBe('3m');
    expect(num(fc.order_count), `3m 映射 90 天，退化分支应生成 90 行`).toBe(90);
    const detail = fc.detail as Row[];
    expect(Array.isArray(detail), `detail 应出数组`).toBe(true);
    expect(detail.length, `detail 应截前 30 行`).toBe(30);
    expect(num(fc.confidence), `fallback 置信常量 0.55 应放大为整数 55`).toBe(55);
    expect(str(fc.trend), `fallback 趋势常量应外显 STABLE`).toBe('STABLE');
    expect(
      Number.isFinite(num(fc.sales_amount)),
      `sales_amount 应为有限数值（库量漂移不锁值，只锁形状），实际 ${fc.sales_amount}`
    ).toBe(true);
    for (const d of detail) {
      expect(Number.isNaN(Date.parse(str(d.date))), `detail.date 应可解析，实际 ${d.date}`).toBe(
        false
      );
      expect(
        Number.isFinite(num(d.predicted_quantity)),
        `predicted_quantity 应为有限数值串，实际 ${d.predicted_quantity}`
      ).toBe(true);
    }
  });

  test('AD-05 库存优化：汇总句式与优先级词表', async ({ page }) => {
    const opt = await apiCallRaw<Row>(page, 'POST', '/advanced/ai/inventory-optimization', {});
    expect(
      /^检测到 \d+ 个产品需要补货，\d+ 个产品库存积压$/.test(str(opt.summary)),
      `summary 应按固定句式装配，实际 ${opt.summary}`
    ).toBe(true);
    const items = opt.items as Row[];
    expect(Array.isArray(items), `items 应出数组，实际 ${JSON.stringify(opt.items)}`).toBe(true);
    for (const it of items) {
      expect(
        ['high', 'medium', 'low'].includes(str(it.priority)),
        `优先级应落三档词表，实际 ${JSON.stringify(it)}`
      ).toBe(true);
      expect(str(it.product_name).length, `建议行产品名应取主数据真值非空`).toBeGreaterThan(0);
      expect(str(it.suggestion).length, `建议文案应非空`).toBeGreaterThan(0);
    }
  });

  test('AD-06 异常检测：data_type 必填门与 sales 过滤的实体前缀词表', async ({ page }) => {
    const miss = await apiCallExpectFail(page, 'POST', '/advanced/ai/anomaly-detection', {});
    expect(miss.status, `缺 data_type 应恰 400，实际 ${miss.status}`).toBe(400);
    expect(failureCode(miss)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    const det = await apiCallRaw<Row[]>(page, 'POST', '/advanced/ai/anomaly-detection', {
      data_type: 'sales',
      date_range: '30',
    });
    expect(Array.isArray(det), `异常应出数组，实际 ${JSON.stringify(det)}`).toBe(true);
    for (const a of det) {
      // data_type=sales 走 entity_type 过滤腿：命中行实体前缀必为 SALES
      expect(
        str(a.item).startsWith('SALES #'),
        `sales 过滤下 item 应以 SALES 加主键形态出现，实际 ${a.item}`
      ).toBe(true);
      expect(
        ['critical', 'warning', 'info'].includes(str(a.severity)),
        `severity 应落小写三档词表（内部大写枚举不得外漏），实际 ${a.severity}`
      ).toBe(true);
      expect(!Number.isNaN(Date.parse(str(a.detected_at))), `detected_at 应可解析`).toBe(true);
      expect(str(a.anomaly_type).length, `anomaly_type 应非空（映射文案或原值）`).toBeGreaterThan(
        0
      );
      expect(str(a.description).length, `description 应非空`).toBeGreaterThan(0);
    }
  });

  test('AD-07 智能推荐：limit 截断契约与类型标签词表', async ({ page }) => {
    const recs = await apiCallRaw<Row[]>(page, 'POST', '/advanced/ai/recommendations', {
      recommendation_type: 'all',
      limit: 3,
    });
    expect(Array.isArray(recs), `推荐应出数组，实际 ${JSON.stringify(recs)}`).toBe(true);
    expect(recs.length, `limit=3 的服务端截断契约`).toBeLessThanOrEqual(3);
    for (const r of recs) {
      expect(
        ['补货建议', '关联推荐', '趋势推荐', '价格调整', '综合推荐'].includes(
          str(r.recommendation_type)
        ),
        `推荐类型应落中文标签词表（内部枚举不得外漏），实际 ${JSON.stringify(r)}`
      ).toBe(true);
      expect(Number.isInteger(num(r.target_id)), `target_id 应为整数`).toBe(true);
      expect(Number.isFinite(num(r.score)), `score 应为有限数值，实际 ${r.score}`).toBe(true);
      expect(
        /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$/.test(str(r.created_at)),
        `created_at 应为 YYYY-MM-DD HH:mm，实际 ${r.created_at}`
      ).toBe(true);
      expect(str(r.content).length, `content 推荐理由应非空`).toBeGreaterThan(0);
    }
    // 空体腿：OptionalJson 允许无请求体（缺省 all 与 limit 10），仍应出数组
    const bare = await apiCallRaw<Row[]>(page, 'POST', '/advanced/ai/recommendations', {});
    expect(Array.isArray(bare), `空体推荐应出数组，实际 ${JSON.stringify(bare)}`).toBe(true);
    expect(bare.length).toBeLessThanOrEqual(10);
  });

  test('AD-08 质量预测：窗口与类型两门逐文案拒绝与正腿词表值域', async ({ page }) => {
    const smallWin = await apiCallExpectFail(page, 'POST', '/advanced/ai/quality-prediction', {
      window_days: 0,
    });
    expect(smallWin.status, `window_days=0 应恰 400，实际 ${smallWin.status}`).toBe(400);
    expect(failureCode(smallWin)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      String(smallWin.message ?? ''),
      `displayable 出参应外显窗口范围，实际 ${smallWin.message}`
    ).toContain('时间窗口 window_days 必须在 1-365 之间');
    const bigWin = await apiCallExpectFail(page, 'POST', '/advanced/ai/quality-prediction', {
      window_days: 366,
    });
    expect(bigWin.status).toBe(400);
    expect(String(bigWin.message ?? '')).toContain('时间窗口 window_days 必须在 1-365 之间');
    const longType = await apiCallExpectFail(page, 'POST', '/advanced/ai/quality-prediction', {
      inspection_type: '超'.repeat(33),
    });
    expect(longType.status).toBe(400);
    expect(String(longType.message ?? '')).toContain('检验类型 inspection_type 长度不能超过 32');

    const pred = await apiCallRaw<Row>(page, 'POST', '/advanced/ai/quality-prediction', {
      product_id: 9999999,
      window_days: 1,
    });
    expect(num(pred.window_days), `窗口应回显提交值 1`).toBe(1);
    expect(str(pred.inspection_type), `未限定检验类型应回显 all`).toBe('all');
    expect(
      ['上升', '平稳', '下降', '无数据'].includes(str(pred.trend)),
      `趋势应落中文词表，实际 ${pred.trend}`
    ).toBe(true);
    expect(
      ['低', '中', '高'].includes(str(pred.risk_level)),
      `风险等级应落三档词表，实际 ${pred.risk_level}`
    ).toBe(true);
    expect(
      num(pred.confidence) >= 0 && num(pred.confidence) <= 1,
      `置信度应落 0 到 1，实际 ${pred.confidence}`
    ).toBe(true);
    expect(
      num(pred.avg_qualification_rate) >= 0 && num(pred.avg_qualification_rate) <= 100,
      `合格率应落百分比区间，实际 ${pred.avg_qualification_rate}`
    ).toBe(true);
    expect(
      Number.isInteger(num(pred.risk_score)),
      `风险评分应为整数，实际 ${pred.risk_score}`
    ).toBe(true);
    expect(Array.isArray(pred.top_issues), `top_issues 应出数组`).toBe(true);
    expect(Array.isArray(pred.recommendations), `recommendations 应出数组`).toBe(true);
  });

  test('AD-09 染配工艺推荐：两空字段门与 k=0 强制退化的典型参数常量表', async ({ page }) => {
    const noColor = await apiCallExpectFail(page, 'POST', '/advanced/ai/recipe-optimization', {
      color_no: '',
      fabric_type: '棉',
    });
    expect(noColor.status, `空色号应恰 400，实际 ${noColor.status}`).toBe(400);
    expect(failureCode(noColor)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      String(noColor.message ?? ''),
      `displayable 出参应外显色号必填，实际 ${noColor.message}`
    ).toContain('色号 color_no 不能为空');
    const noFabric = await apiCallExpectFail(page, 'POST', '/advanced/ai/recipe-optimization', {
      color_no: 'E2E-BL-9999',
      fabric_type: '   ',
    });
    expect(noFabric.status).toBe(400);
    expect(String(noFabric.message ?? '')).toContain('布类 fabric_type 不能为空');

    // k=0 强制走内置典型参数表——温度 80 时间 45 pH 6.0 浴比 8.0 为常量装配
    const opt = await apiCallRaw<Row>(page, 'POST', '/advanced/ai/recipe-optimization', {
      color_no: 'E2E-BL-9999',
      fabric_type: '棉',
      k: 0,
    });
    expect(str(opt.source), `k=0 应走 fallback 来源标识`).toBe('fallback');
    expect(str(opt.reason), `退化原因应逐字装配`).toBe('k=0，已强制走典型参数表');
    expect(num(opt.similar_cases), `强制退化不应有相似案例`).toBe(0);
    expect(opt.degraded, `算法退化腿 degraded 应为 false`).toBe(false);
    expect(opt.cache_hit, `首次计算 cache_hit 应为 false`).toBe(false);
    const params = opt.recommended_params as Row;
    expect(num(params.temperature), `典型温度常量 80`).toBe(80);
    expect(num(params.time_minutes), `典型时间常量 45 分钟`).toBe(45);
    expect(num(params.ph_value), `典型 pH 常量 6.0`).toBe(6);
    expect(num(params.liquor_ratio), `典型浴比常量 1:8 记 8`).toBe(8);
    expect(num(opt.confidence), `fallback 置信度常量 0.6`).toBeCloseTo(0.6, 5);
    expect(Array.isArray(opt.candidates), `candidates 应出数组`).toBe(true);
    expect((opt.candidates as Row[]).length, `强制退化候选应为空集`).toBe(0);
  });
});
