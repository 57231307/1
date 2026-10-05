import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  failureCode,
  verifyEndpointHealthy,
  verifyDownloadEndpointHealthy,
  APP_ERROR_CODES,
  BASE_URL,
  type ApiFailureResult,
} from './helpers';

/**
 * 69 职业健康合规域（环保合规域最后一块零覆盖区）——落库回读 + 状态机 + 词表负例
 *
 * 覆盖审计结论：occupational-health 三子域（危害因素监测/体检档案/PPE）此前**整页零 e2e**
 * （仅 46 巡检与 traversal 端点扫描的宽松健康），本 spec 补齐真实契约覆盖。
 *
 * 端点真实性（routes/occupational_health.rs，mod.rs:499 nest "/api/v1/erp"）：
 * - POST/GET /occupational-health/hazard-monitorings                routes/occupational_health.rs:13-20
 * - GET      /occupational-health/hazard-monitorings/{id}/print     同文件 :21-24（docx 二进制 → download helper）
 * - POST/GET /occupational-health/health-exams                      同文件 :31-36
 * - POST     /occupational-health/health-exams/scan-expiry-warnings 同文件 :38-41
 * - GET      /occupational-health/health-exams/{id}/print           同文件 :43-45
 * - POST/GET /occupational-health/ppe-distributions                 同文件 :51-57
 * - POST     /occupational-health/ppe-distributions/scan-expired    同文件 :59-62
 * - POST     /occupational-health/ppe-distributions/{id}/return     同文件 :63-66
 * - GET      /occupational-health/ppe-distributions/{id}/print      同文件 :67-69
 * **未注册**：三域均无 GET /{id} 详情、无 DELETE——落库回读只能走列表端点，
 * 不给未注册端点写断言（详见交付报告）。
 *
 * 响应信封真相（逐 handler 核对，禁止 items/total 想当然）：
 * - 三个列表端点 data={list,total}（occupational_health_handler.rs:36/:60/:94
 *   `json!({"list":..,"total":..})`，**不是** PaginatedResponse 的 items 形状，也没有 page/page_size 回显）；
 * - scan-expiry-warnings data=Vec<ExamExpiryWarning>（{exam,level,days_until_expiry}，handler :65-72）裸数组；
 * - scan-expired data=Vec<PpeModel>（handler :110-117）裸数组；
 * - 创建/回收 data=对应 Model 单对象（models/occupational_hazard_monitoring.rs /
 *   occupational_health_exam.rs / ppe_distribution_record.rs，snake_case）。
 *
 * 状态/词表真相（写入方唯一真源，services/occupational_health_service.rs）：
 * - hazard_type ∈ chemical/physical/dust/biological（:554-561）；
 * - exam_type ∈ pre_employment/in_service/resignation（:566-573）；
 * - exam_result ∈ normal/abnormal/contraindication（:575-584）；
 * - ppe_type ∈ mask/gloves/goggles/earplug/respirator/suit（:587-594）；
 * - PPE 状态 distributed → returned（:479-497）/ scan 侧 distributed→expired（:499-523）。
 *
 * 前端源码现状与缺陷（以 frontend/src/views/occupational-health/index.vue 当前实现为准）：
 * ① 体检/危害/PPE 三个弹窗的候选值已是后端词表的同源常量（EXAM_TYPES/EXAM_RESULTS/
 *    HAZARD_TYPES/PPE_TYPES），提交体带齐必填；本 spec 仍保留「复刻缺必填的请求体 → 400」
 *    的负例（69-02 内）——它钉的是**后端 DTO 契约**，与 UI 当前是否犯这个错无关，
 *    日志里那条 422→400 `missing field hazard_type`
 *    （backend.log xr19:11850/11853/11854）正是这条负例的预期拒绝，不是建单失败；
 * ② 「过期扫描」按钮真实调用 POST /ppe-distributions/scan-expired；
 * ③ **曾有缺陷（#4671 69-02 判红，本批已在源码侧修复）**：危害监测/PPE 表格列曾由 index.vue
 *    `colsOf(rows, ['id'], 6)` 采样——取 JSON 键序前 6 个非对象键，而 serde_json 序列化
 *    Model 时键按字母序排 ⇒ 实际渲染 created_at/created_by/exceeding_ratio/hazard_name/
 *    hazard_type/is_exceeding，**监测点位 monitoring_point 永不渲染**（#4671 69-02 失败现场
 *    ARIA 快照实证）。用户看不到危害监测的核心维度，属功能缺陷；现视图显式声明列
 *    （HAZARD_COLUMNS / PPE_COLUMNS，prop 逐一对齐后端 Model），本 spec 的表头断言 +
 *    行内容断言即该修复的活体回归锁，禁止反过来删断言换绿。
 *
 * 假绿防线：
 * - 已注册 GET 端点全部严格 verifyEndpointHealthy（404/403 判红）；本域端点均已注册，无 optional 场景；
 * - 每次写操作后按唯一标记从 GET 列表回读逐字段断言落库真值；被拒的写回读断"无痕"；
 * - rust_decimal 出参是字符串（rust_decimal serde 默认），一律 Number() 归一后比较；
 * - 信封显式钉桩：list 键必须为数组、裸数组端点必须 Array.isArray，禁 `?? []` 兜底；
 * - 数据自建自流转（唯一 code 用 genCode，worker_id 取当前登录用户真实 id），不依赖并行用例残留。
 *
 * 不可 CI 化项（不造假）：quarantine 隔离态——chemical 侧同理，此处职业健康无对应概念；
 * 体检禁忌预警（exam_result=contraindication）的**推送触达**只能看服务端日志
 * （service :329-335 tracing::warn），e2e 不可断言投递，本 spec 只断落库真值。
 */

type Row = Record<string, unknown>;

/** rust_decimal 经 JSON 序列化为字符串，比较前归一 */
const toNum = (v: unknown): number => Number(String(v));

const ymd = (daysFromToday: number): string =>
  new Date(Date.now() + daysFromToday * 86_400_000).toISOString().slice(0, 10);

/**
 * 断言精确拒绝契约（收紧原「任意 4xx + 三族机器码来者不拒」的假绿）：
 * - HTTP 必须恰为 400（backend/utils/error.rs:356-373：本域拒绝全部映射 BAD_REQUEST 状态）；
 * - 机器码逐条等于调用点期望族。写入方真相 occupational_health_service.rs：
 *   枚举词表/数值范围/日期先后 = AppError::bad_request → BAD_REQUEST
 *   （validate_hazard_type/validate_exam_type/validate_exam_result/validate_ppe_type、
 *   create_hazard_monitoring 限值门、create_health_exam 日期门、create_ppe_distribution 数量/效期门）；
 *   业务内控门/状态门 = AppError::business → BUSINESS_ERROR
 *   （in_service 必带 next_exam_date、return_ppe 仅 distributed 可回收）。
 * 永不读取文案（business 族出参脱敏，不可判）。
 */
function expectRejected(r: ApiFailureResult, what: string, expectedCode: string): void {
  expect(r.status, `${what}：应恰为 HTTP 400，实际 ${r.status} ${JSON.stringify(r)}`).toBe(400);
  expect(
    failureCode(r),
    `${what}：机器码应为 ${expectedCode}，实际 ${failureCode(r)}（${JSON.stringify(r)}）`
  ).toBe(expectedCode);
}

/** axum 提取器拒绝（缺必填 JSON 字段）：原文只进日志，出参由 trace_context 归一成
 *  统一 AppError 信封（HTTP 400 + code=VALIDATION_ERROR，serde 原文不外显）。
 *  实证 xr19/backend.log:11850（rejecting 422 missing field `hazard_type`）→ :11854
 *  （extractor.rejection_normalized：已转统一 AppError 信封 400 + code=VALIDATION_ERROR）。
 *  判据 = 真实 status 400 **且** 机器码 VALIDATION_ERROR（两者都是后端成文契约，缺一即漂移）。 */
function expectExtractorReject(r: ApiFailureResult, what: string): void {
  expect(r.status, `${what}：应被 400 拒绝（提取器层），实际 ${JSON.stringify(r)}`).toBe(400);
  expect(
    failureCode(r),
    `${what}：提取器拒绝归一后的机器码应为 VALIDATION_ERROR（不得是 FORBIDDEN/CSRF_* 等权限面），实际 ${JSON.stringify(r)}`
  ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
}

/** 列表信封钉桩：本域三个列表端点 data={list,total}（handler :36/:60/:94），非 items 形状 */
async function readList(
  page: Page,
  path: string,
  endpoint: string
): Promise<Array<Record<string, unknown>>> {
  const data = await apiCallRaw<{ list?: unknown; total?: unknown }>(page, 'GET', path);
  expect(
    Array.isArray(data?.list),
    `${endpoint} 响应 data.list 必须为数组（真实信封 {list,total}），实际 ${JSON.stringify(data)}`
  ).toBe(true);
  expect(
    typeof data.total,
    `${endpoint} total 必须存在且为数字（禁 ?? 0 兜底），实际 ${JSON.stringify(data)}`
  ).toBe('number');
  return data.list as Array<Record<string, unknown>>;
}

/** 裸数组信封钉桩（scan 类端点 to_value(Vec)） */
function requirePlainArray(data: unknown, endpoint: string): Row[] {
  expect(
    Array.isArray(data),
    `${endpoint} 出参 data 必须为数组（handler 直接 to_value(Vec<...>)），实际 ${JSON.stringify(data)}`
  ).toBe(true);
  return data as Row[];
}

function findRowById(items: Row[], id: number, endpoint: string): Row {
  const row = items.find(it => Number(it.id) === id);
  expect(
    row,
    `${endpoint}：应回读到自建记录 id=${id}，实际 ${JSON.stringify(items).slice(0, 500)}`
  ).toBeTruthy();
  return row as Row;
}

test.describe.serial('69 职业健康合规：危害监测 + 体检档案 + PPE 发放', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('69-01 页面可达 + 三列表端点严格健康 + Tab 渲染（404/403 即判红）', async ({ page }) => {
    await page.goto(`${BASE_URL}/occupational-health`);
    await expect(page.getByRole('tab', { name: '职业健康体检', exact: true })).toBeVisible();
    await expect(page.getByRole('tab', { name: '危害因素监测', exact: true })).toBeVisible();
    await expect(page.getByRole('tab', { name: '劳保用品发放', exact: true })).toBeVisible();

    await verifyEndpointHealthy(page, '/occupational-health/hazard-monitorings?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/occupational-health/health-exams?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/occupational-health/ppe-distributions?page=1&page_size=5');
  });

  test('69-02 危害因素监测：超标判定真值/边界=限值不超标/only_exceeding 过滤/UI 回读/打印', async ({
    page,
  }) => {
    const me = await apiCall<{ id?: number }>(page, 'GET', '/auth/me');
    expect(Number(me?.data?.id), `未取得当前用户 id：${JSON.stringify(me)}`).toBeGreaterThan(0);

    // 超标正例：苯 12 > 限值 6 → is_exceeding=true，ratio=12/6-1=1（service.rs:526-536）
    const exceedPoint = genCode('E2E69HX');
    const created = await apiCall<Row>(page, 'POST', '/occupational-health/hazard-monitorings', {
      hazard_type: 'chemical',
      hazard_name: '苯',
      monitoring_point: exceedPoint,
      measured_value: '12',
      unit: 'mg/m³',
      limit_value: '6',
      monitoring_date: ymd(0),
      monitoring_organization: 'E2E69检测机构',
      report_url: '/reports/e2e-69-hazard.pdf',
    });
    const id = Number(created.data?.id);
    expect(id, `危害监测创建应返回 id：${JSON.stringify(created)}`).toBeGreaterThan(0);
    expect(created.data?.is_exceeding, '实测 12 > 限值 6 应自动判定超标').toBe(true);
    expect(toNum(created.data?.exceeding_ratio), '超标倍数应 = 12/6-1 = 1').toBeCloseTo(1, 4);

    // 边界对照：实测 == 限值 → 不超标且 ratio 为 null（判定用 >，非 >=）
    const eqPoint = genCode('E2E69HE');
    const eq = await apiCall<Row>(page, 'POST', '/occupational-health/hazard-monitorings', {
      hazard_type: 'physical',
      hazard_name: '噪声',
      monitoring_point: eqPoint,
      measured_value: '85',
      unit: 'dB',
      limit_value: '85',
      monitoring_date: ymd(0),
    });
    expect(eq.data?.is_exceeding, '实测等于限值不得判为超标').toBe(false);
    expect(eq.data?.exceeding_ratio, '未超标时 exceeding_ratio 应落 NULL').toBeNull();

    // 列表回读逐字段（data={list,total} 信封）
    const list = await readList(
      page,
      `/occupational-health/hazard-monitorings?hazard_name=${encodeURIComponent('苯')}&page=1&page_size=50`,
      'GET /hazard-monitorings?hazard_name=苯'
    );
    const rec = findRowById(list, id, 'GET /hazard-monitorings');
    expect(rec.hazard_type, '危害类型应落库 chemical').toBe('chemical');
    expect(rec.hazard_name, '危害名称应逐字落库「苯」').toBe('苯');
    expect(rec.monitoring_point, '唯一监测点应回读命中').toBe(exceedPoint);
    expect(toNum(rec.measured_value), '实测值应落库 12').toBe(12);
    expect(toNum(rec.limit_value), '限值应落库 6').toBe(6);
    expect(rec.unit, '单位应落库 mg/m³').toBe('mg/m³');
    expect(rec.monitoring_date, '监测日期应逐字落库').toBe(ymd(0));
    expect(rec.monitoring_organization, '检测机构应落库').toBe('E2E69检测机构');
    expect(rec.report_url, '报告地址应落库').toBe('/reports/e2e-69-hazard.pdf');
    expect(rec.created_by, 'created_by 应由后端从会话注入 = 当前登录用户（handler :22-23）').toBe(
      Number(me.data?.id)
    );

    // only_exceeding=true 过滤真实生效：含超标行、不含等值对照行（非仅 200）
    const onlyExc = await readList(
      page,
      '/occupational-health/hazard-monitorings?only_exceeding=true&page=1&page_size=100',
      'GET /hazard-monitorings?only_exceeding=true'
    );
    expect(
      onlyExc.some(it => Number(it.id) === id),
      `only_exceeding=true 应命中超标记录 id=${id}`
    ).toBe(true);
    expect(
      onlyExc.some(it => Number(it.id) === Number(eq.data?.id)),
      'only_exceeding=true 不得混入未超标记录（边界=限值）'
    ).toBe(false);
    for (const it of onlyExc) {
      expect(
        it.is_exceeding,
        `only_exceeding 过滤结果行必须逐条 is_exceeding=true：${JSON.stringify(it)}`
      ).toBe(true);
    }

    // 负例：非法危害类型（词表外，service :554-561 → AppError::bad_request 统一信封）
    const badType = await apiCallExpectFail(
      page,
      'POST',
      '/occupational-health/hazard-monitorings',
      {
        hazard_type: 'radiation',
        hazard_name: '辐射',
        monitoring_point: genCode('E2E69HB'),
        measured_value: '1',
        unit: 'uSv/h',
        limit_value: '2.5',
        monitoring_date: ymd(0),
      }
    );
    // validate_hazard_type → AppError::bad_request（词表族）
    expectRejected(badType, '非法危害类型 radiation 应被拒绝', APP_ERROR_CODES.BAD_REQUEST);
    // 负例：限值 ≤ 0（:200-202）
    const zeroLimit = await apiCallExpectFail(
      page,
      'POST',
      '/occupational-health/hazard-monitorings',
      {
        hazard_type: 'chemical',
        hazard_name: '甲醛',
        monitoring_point: genCode('E2E69HL'),
        measured_value: '1',
        unit: 'mg/m³',
        limit_value: '0',
        monitoring_date: ymd(0),
      }
    );
    // create_hazard_monitoring 限值门 → AppError::bad_request
    expectRejected(zeroLimit, '限值=0 应被拒绝', APP_ERROR_CODES.BAD_REQUEST);
    // 负例：复刻 UI 危害弹窗真实请求体 {hazard_factor, monitor_value, monitor_point}
    //（视图 :110-121，缺后端全部必填 → 提取器 400，见头部缺陷②）
    const driftPoint = genCode('E2E69DP');
    const uiDrift = await apiCallExpectFail(
      page,
      'POST',
      '/occupational-health/hazard-monitorings',
      {
        hazard_factor: '苯',
        monitor_value: 12,
        monitor_point: driftPoint,
      }
    );
    expectExtractorReject(uiDrift, 'UI 复刻危害请求体（缺全部必填键）应 400');
    // 被拒的写必须无痕：chemical 过滤集合内不得出现该监测点
    const afterDrift = await readList(
      page,
      '/occupational-health/hazard-monitorings?hazard_type=chemical&page=1&page_size=200',
      'GET /hazard-monitorings?hazard_type=chemical（UI 复刻无痕回读）'
    );
    expect(
      afterDrift.some(it => it.monitoring_point === driftPoint),
      `UI 复刻被拒后不得落库监测点 ${driftPoint}`
    ).toBe(false);

    // UI 列表回读：页面 mount 即 GET 三列表（unwrapList 读 data.list）
    await page.goto(`${BASE_URL}/occupational-health`);
    await page.getByRole('tab', { name: '危害因素监测', exact: true }).click();

    // 先用表内唯一锚点（首列 ID = 本用例自建行）定位，再断**该行渲染出来的单元格内容**：
    // 危害类型必须逐字等于我提交的 chemical、危害名称等于「苯」——不止"有一行看得见"。
    const ownRow = page
      .locator('.el-table__row')
      .filter({ has: page.locator(`td:first-child:text-is("${id}")`) })
      .first();
    await expect(ownRow, `危害监测表格应存在本用例自建行 id=${id}`).toBeVisible({
      timeout: 15_000,
    });
    const ownRowText = await ownRow.innerText();
    expect(
      ownRowText,
      `UI 行应逐字回读提交的 hazard_type=chemical，实际行文本=${JSON.stringify(ownRowText)}`
    ).toContain('chemical');
    expect(
      ownRowText,
      `UI 行应回读 hazard_name=苯，实际行文本=${JSON.stringify(ownRowText)}`
    ).toContain('苯');

    // —— 列渲染回归锁（#4671 69-02 真红根因，已在源码侧修复，此断言防回潮）——
    // 该用例当时失败的原因既不是"没提交 hazard_type"（该键自始随 POST 提交，并已在上面 API
    // 回读处逐字段断过落库=chemical），也不是时序：失败现场 ARIA 快照实证表头渲染为
    // created at | created by | exceeding ratio | hazard name | hazard type | is exceeding
    // ——**监测点位根本没进表格**，于是"行内含唯一监测点"在 UI 层恒不可达（等 15s 后 not found）。
    // 根因 frontend/src/views/occupational-health/index.vue 旧 hazardCols = colsOf(rows, ['id'], 6)
    // 取的是「JSON 键序的前 6 个非对象键」，而 handler 用 serde_json 序列化 Model 时对象键按
    // **字母序**落 map（未开 preserve_order）⇒ 前 6 恒为那六个键，monitoring_point 排在其后、
    // 永不渲染，用户看不到危害监测的核心追溯维度。现视图改为显式声明列（HAZARD_COLUMNS/
    // PPE_COLUMNS，prop 逐一对齐后端 Model），以下两条即该修复的活体锁：谁把列声明改回按键序
    // 采样、或删掉监测点位列，用例立刻打红。禁止反过来删断言/放宽换绿。
    const headerTexts = (await page.locator('.el-table__header th').allInnerTexts())
      .map(s => s.trim())
      .filter(Boolean);
    expect(
      headerTexts,
      `危害监测表格必须渲染「监测点位」列（i18n 键 occupationalHealth.columns.monitoringPoint），实际表头=${JSON.stringify(headerTexts)}`
    ).toContain('监测点位');
    const row = page.locator('.el-table__row').filter({ hasText: exceedPoint }).first();
    await expect(row, `危害监测表格应回读 ${exceedPoint}`).toBeVisible({ timeout: 15_000 });

    // 打印 docx（二进制 → download 严格健康）
    await verifyDownloadEndpointHealthy(
      page,
      `/occupational-health/hazard-monitorings/${id}/print`
    );
  });

  test('69-03 体检档案：正例落库/in_service 门控/词表负例与 UI 复刻无痕/到期预警分级/打印', async ({
    page,
  }) => {
    const me = await apiCall<{ id?: number }>(page, 'GET', '/auth/me');
    const workerId = Number(me?.data?.id);
    expect(workerId, `未取得当前用户 id：${JSON.stringify(me)}`).toBeGreaterThan(0);

    // 上岗前正例：全字段 + hazard_exposure JSON 落库回读
    const pre = await apiCall<Row>(page, 'POST', '/occupational-health/health-exams', {
      worker_id: workerId,
      exam_type: 'pre_employment',
      exam_date: ymd(0),
      exam_organization: genCode('E2E69ORG'),
      exam_result: 'normal',
      hazard_exposure: [{ hazard: '苯', years: 3 }],
      remarks: 'E2E69 上岗前体检',
    });
    const preId = Number(pre.data?.id);
    expect(preId, `上岗前体检应创建成功：${JSON.stringify(pre)}`).toBeGreaterThan(0);
    const listed = await readList(
      page,
      `/occupational-health/health-exams?worker_id=${workerId}&exam_type=pre_employment&page=1&page_size=50`,
      'GET /health-exams?exam_type=pre_employment'
    );
    const preRec = findRowById(listed, preId, 'GET /health-exams');
    expect(preRec.exam_result, '体检结果应落库 normal').toBe('normal');
    expect(preRec.exam_date, '体检日期应逐字落库').toBe(ymd(0));
    expect(preRec.worker_id, 'worker_id 应落库当前用户').toBe(workerId);
    expect(
      Array.isArray(preRec.hazard_exposure) &&
        (preRec.hazard_exposure as Row[])[0]?.hazard === '苯',
      `hazard_exposure JSONB 应原样落库，实际 ${JSON.stringify(preRec.hazard_exposure)}`
    ).toBe(true);

    // 在岗期间体检门控：缺 next_exam_date 必拒（service :292-296，business 400）
    const missingNext = await apiCallExpectFail(page, 'POST', '/occupational-health/health-exams', {
      worker_id: workerId,
      exam_type: 'in_service',
      exam_date: ymd(0),
      exam_result: 'normal',
    });
    // create_health_exam 内控门（in_service 必带 next_exam_date）→ AppError::business
    expectRejected(
      missingNext,
      'in_service 缺 next_exam_date 应被拒绝',
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    // next_exam_date 必须晚于 exam_date（:298-303）
    const badNext = await apiCallExpectFail(page, 'POST', '/occupational-health/health-exams', {
      worker_id: workerId,
      exam_type: 'in_service',
      exam_date: ymd(10),
      next_exam_date: ymd(10),
      exam_result: 'normal',
    });
    // create_health_exam 日期先后门 → AppError::bad_request
    expectRejected(badNext, '下次体检=本次体检应被拒绝', APP_ERROR_CODES.BAD_REQUEST);

    // 词表负例（UI 复刻）：exam_type 'periodic' 不在后端词表 → 400（头部缺陷①），
    // 且被拒写必须无痕：exam_type=periodic 过滤集合恒空（该 token 仅能经此端点入库）
    const periodic = await apiCallExpectFail(page, 'POST', '/occupational-health/health-exams', {
      worker_id: workerId,
      exam_type: 'periodic',
      exam_date: ymd(0),
      exam_result: 'normal',
    });
    // validate_exam_type → AppError::bad_request（词表族）
    expectRejected(
      periodic,
      "exam_type='periodic'（UI 默认值）应被词表拒绝",
      APP_ERROR_CODES.BAD_REQUEST
    );
    const ghostType = await readList(
      page,
      '/occupational-health/health-exams?exam_type=periodic&page=1&page_size=20',
      'GET /health-exams?exam_type=periodic（无痕回读）'
    );
    expect(ghostType.length, "被拒后不得存在任何 exam_type='periodic' 落库记录").toBe(0);
    // 中文结论 '正常'（UI 自由输入框口径）同样必须被拒
    const zhResult = await apiCallExpectFail(page, 'POST', '/occupational-health/health-exams', {
      worker_id: workerId,
      exam_type: 'pre_employment',
      exam_date: ymd(0),
      exam_result: '正常',
    });
    // validate_exam_result → AppError::bad_request（词表族；exam_type 侧本例为合法值）
    expectRejected(
      zhResult,
      "exam_result='正常'（中文自由文本）应被词表拒绝",
      APP_ERROR_CODES.BAD_REQUEST
    );
    // 缺必填字段（提取器层 400，非统一信封）
    const missingField = await apiCallExpectFail(
      page,
      'POST',
      '/occupational-health/health-exams',
      {
        worker_id: workerId,
        exam_date: ymd(0),
      }
    );
    expectExtractorReject(missingField, '缺 exam_type/exam_result 必填应 400');

    // 到期预警分级：+15 天 → Critical；-3 天 → Expired（service :539-551/:373-401）
    const crit = await apiCall<Row>(page, 'POST', '/occupational-health/health-exams', {
      worker_id: workerId,
      exam_type: 'in_service',
      exam_date: ymd(-350),
      next_exam_date: ymd(15),
      exam_result: 'normal',
      remarks: 'E2E69-Critical',
    });
    const expired = await apiCall<Row>(page, 'POST', '/occupational-health/health-exams', {
      worker_id: workerId,
      exam_type: 'in_service',
      exam_date: ymd(-370),
      next_exam_date: ymd(-3),
      exam_result: 'abnormal',
      remarks: 'E2E69-Expired',
    });
    const warnings = requirePlainArray(
      await apiCallRaw<unknown>(
        page,
        'POST',
        '/occupational-health/health-exams/scan-expiry-warnings'
      ),
      'POST /health-exams/scan-expiry-warnings'
    );
    const critHit = warnings.find(w => Number((w.exam as Row)?.id) === Number(crit.data?.id));
    const expHit = warnings.find(w => Number((w.exam as Row)?.id) === Number(expired.data?.id));
    expect(critHit, `到期扫描应含 +15 天记录 id=${crit.data?.id}`).toBeTruthy();
    expect(critHit?.level, '15 天后到期应分级 Critical').toBe('Critical');
    expect(
      Number(critHit?.days_until_expiry),
      '剩余天数应 ≈15（本地日界容差 ±2）'
    ).toBeGreaterThanOrEqual(13);
    expect(Number(critHit?.days_until_expiry)).toBeLessThanOrEqual(17);
    expect(expHit, `到期扫描应含已过期记录 id=${expired.data?.id}`).toBeTruthy();
    expect(expHit?.level, '已过期应分级 Expired').toBe('Expired');

    // UI 列表回读（体检 Tab 默认页；exam_organization 唯一值可见）
    await page.goto(`${BASE_URL}/occupational-health`);
    const orgText = String(preRec.exam_organization);
    const examRow = page.locator('.el-table__row').filter({ hasText: orgText }).first();
    await expect(examRow, `体检表格应回读机构 ${orgText}`).toBeVisible({ timeout: 15_000 });

    await verifyDownloadEndpointHealthy(page, `/occupational-health/health-exams/${preId}/print`);
  });

  test('69-04 PPE 发放：建发→回收状态机/非法流转/404/scan-expired 自动回写/负例/打印', async ({
    page,
  }) => {
    const me = await apiCall<{ id?: number }>(page, 'GET', '/auth/me');
    const workerId = Number(me?.data?.id);
    expect(workerId, `未取得当前用户 id：${JSON.stringify(me)}`).toBeGreaterThan(0);

    // 建发：初始 status=distributed（service :430）
    const dist = await apiCall<Row>(page, 'POST', '/occupational-health/ppe-distributions', {
      worker_id: workerId,
      ppe_name: `防毒面具${genCode('E2E69P')}`,
      ppe_type: 'respirator',
      specification: '6800型',
      quantity: 2,
      distribution_date: ymd(0),
      expiry_date: ymd(300),
      hazard_type: 'chemical',
    });
    const distId = Number(dist.data?.id);
    expect(distId, `PPE 发放应创建成功：${JSON.stringify(dist)}`).toBeGreaterThan(0);
    expect(dist.data?.status, '新建 PPE 初始状态应 distributed').toBe('distributed');

    // 负例：quantity≤0 / 到期≤发放 / 词表外类型
    const badQty = await apiCallExpectFail(page, 'POST', '/occupational-health/ppe-distributions', {
      worker_id: workerId,
      ppe_name: '负数量测试',
      ppe_type: 'mask',
      quantity: 0,
      distribution_date: ymd(0),
    });
    // create_ppe_distribution 数量门 → AppError::bad_request
    expectRejected(badQty, 'quantity=0 应被拒绝', APP_ERROR_CODES.BAD_REQUEST);
    const badExpiry = await apiCallExpectFail(
      page,
      'POST',
      '/occupational-health/ppe-distributions',
      {
        worker_id: workerId,
        ppe_name: '倒挂效期测试',
        ppe_type: 'gloves',
        quantity: 1,
        distribution_date: ymd(10),
        expiry_date: ymd(10),
      }
    );
    // create_ppe_distribution 效期先后门 → AppError::bad_request
    expectRejected(badExpiry, '到期=发放日应被拒绝', APP_ERROR_CODES.BAD_REQUEST);
    const badType = await apiCallExpectFail(
      page,
      'POST',
      '/occupational-health/ppe-distributions',
      {
        worker_id: workerId,
        ppe_name: '词表外类型',
        ppe_type: 'helmet',
        quantity: 1,
        distribution_date: ymd(0),
      }
    );
    // validate_ppe_type → AppError::bad_request（词表族）
    expectRejected(badType, "ppe_type='helmet' 不在词表应被拒绝", APP_ERROR_CODES.BAD_REQUEST);

    // 回收 distributed → returned，落库回读
    await apiCall(page, 'POST', `/occupational-health/ppe-distributions/${distId}/return`);
    const afterReturn = await readList(
      page,
      `/occupational-health/ppe-distributions?worker_id=${workerId}&status=returned&page=1&page_size=100`,
      'GET /ppe-distributions?status=returned'
    );
    const returned = findRowById(afterReturn, distId, 'GET /ppe-distributions?status=returned');
    expect(returned.status, '回收后应落库 returned').toBe('returned');
    expect(returned.ppe_type, '回收回读不得丢字段').toBe('respirator');

    // 重复回收必须被拒（状态机 :485-490），且拒绝为无痕：状态仍 returned
    const reReturn = await apiCallExpectFail(
      page,
      'POST',
      `/occupational-health/ppe-distributions/${distId}/return`
    );
    // return_ppe 状态门（仅 distributed 可回收）→ AppError::business
    expectRejected(reReturn, 'returned 再回收应被状态机拒绝', APP_ERROR_CODES.BUSINESS_ERROR);
    const stillReturned = await readList(
      page,
      `/occupational-health/ppe-distributions?worker_id=${workerId}&status=returned&page=1&page_size=100`,
      'GET /ppe-distributions（重复回收无痕）'
    );
    expect(
      findRowById(stillReturned, distId, '重复回收无痕回读').status,
      '被拒回收后状态应仍为 returned'
    ).toBe('returned');

    // 不存在 id 回收 → 404 NOT_FOUND（:480-483；本域无 GET /{id}，404 判定即读路径）
    const ghost = await apiCallExpectFail(
      page,
      'POST',
      '/occupational-health/ppe-distributions/999999999/return'
    );
    expect(ghost.status, '不存在记录回收应 404').toBe(404);
    expect(failureCode(ghost), '404 机器码应为 NOT_FOUND').toBe('NOT_FOUND');

    // scan-expired 副作用回写：过期 distributed → expired（:499-523）
    const expiredPpe = await apiCall<Row>(page, 'POST', '/occupational-health/ppe-distributions', {
      worker_id: workerId,
      ppe_name: `过期面具${genCode('E2E69X')}`,
      ppe_type: 'respirator',
      quantity: 1,
      distribution_date: ymd(-300),
      expiry_date: ymd(-2),
    });
    const okPpe = await apiCall<Row>(page, 'POST', '/occupational-health/ppe-distributions', {
      worker_id: workerId,
      ppe_name: `有效面具${genCode('E2E69Y')}`,
      ppe_type: 'respirator',
      quantity: 1,
      distribution_date: ymd(0),
      expiry_date: ymd(100),
    });
    const scanList = requirePlainArray(
      await apiCallRaw<unknown>(
        page,
        'POST',
        '/occupational-health/ppe-distributions/scan-expired'
      ),
      'POST /ppe-distributions/scan-expired'
    );
    expect(
      scanList.some(it => Number(it.id) === Number(expiredPpe.data?.id)),
      `过期扫描应返回自建过期记录 id=${expiredPpe.data?.id}`
    ).toBe(true);
    const afterScan = await readList(
      page,
      `/occupational-health/ppe-distributions?worker_id=${workerId}&status=expired&page=1&page_size=100`,
      'GET /ppe-distributions?status=expired'
    );
    expect(
      findRowById(afterScan, Number(expiredPpe.data?.id), '过期回写落库回读').status,
      '扫描后过期 PPE 应自动落库 expired'
    ).toBe('expired');
    // expired 记录不可回收（仅 distributed 可回收）
    const returnExpired = await apiCallExpectFail(
      page,
      'POST',
      `/occupational-health/ppe-distributions/${Number(expiredPpe.data?.id)}/return`
    );
    // return_ppe 状态门（expired 非 distributed）→ AppError::business
    expectRejected(returnExpired, 'expired 状态回收应被状态机拒绝', APP_ERROR_CODES.BUSINESS_ERROR);
    // 防过杀：未过期记录扫描后仍 distributed
    const stillDist = await readList(
      page,
      `/occupational-health/ppe-distributions?worker_id=${workerId}&status=distributed&page=1&page_size=100`,
      'GET /ppe-distributions?status=distributed（防过杀）'
    );
    expect(
      findRowById(stillDist, Number(okPpe.data?.id), '未过期记录防过杀回读').status,
      '未过期 PPE 不得被扫描误伤'
    ).toBe('distributed');

    // UI 列表回读（劳保 Tab；唯一 ppe_name 可见）——注意页面"过期扫描"按钮是假按钮（缺陷③），
    // 不在 UI 点击它，只验证真实数据经 GET 渲染进表格
    await page.goto(`${BASE_URL}/occupational-health`);
    await page.getByRole('tab', { name: '劳保用品发放', exact: true }).click();
    const ppeName = String(dist.data?.ppe_name);
    const ppeRow = page.locator('.el-table__row').filter({ hasText: ppeName }).first();
    await expect(ppeRow, `PPE 表格应回读 ${ppeName}`).toBeVisible({ timeout: 15_000 });

    await verifyDownloadEndpointHealthy(
      page,
      `/occupational-health/ppe-distributions/${distId}/print`
    );
  });
});
