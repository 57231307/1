import { test, expect } from '../diagnose-fixture';
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
import { fillFieldByLabel, formItemByExactLabel, pickSelectIn } from './ui-helpers';

/**
 * 60 环保合规整页功能覆盖（e2e 补齐 A 路 / 覆盖审计缺口）
 *
 * 端点真实性（逐条 grep 后端注册处核实，未注册端点不写 strict 断言）：
 * - GET/POST   /pollution-permits                       routes/pollution_permit.rs:13-20（mod.rs:498 nest /api/v1/erp）
 * - GET        /pollution-permits/expiry-warnings       routes/pollution_permit.rs:22-25
 * - GET        /pollution-permits/{id}                  routes/pollution_permit.rs:27-30
 * - POST       /pollution-permits/{id}/revoke           routes/pollution_permit.rs:31-34
 * - GET        /pollution-permits/{id}/print            routes/pollution_permit.rs:36-39（docx 二进制 → download helper）
 * - GET/POST   /pollution-monitoring/records            routes/pollution_monitoring.rs:13-20
 * - POST       /pollution-monitoring/solid-waste-disposals            routes/pollution_monitoring.rs:21-24
 * - PUT        /pollution-monitoring/solid-waste-disposals/{id}/status routes/pollution_monitoring.rs:25-28
 * - GET        /pollution-monitoring/exceedance-alerts  routes/pollution_monitoring.rs:29-32
 * - GET        /pollution-monitoring/solid-waste-disposals/{id}/print  routes/pollution_monitoring.rs:34-37
 * 注意：固废联单**没有注册 GET 列表/详情**，其落库回读只能经 PUT 状态端点的
 * 真实响应模型完成（本 spec 如此处理），不给未注册端点写 strict 断言。
 *
 * 假绿防线：
 * - 已注册端点全部走 verifyEndpointHealthy（404/403 判红）；吞 404/403 的宽松「可选端点」
 *   健康探测类别不存在，此类 helper 已删除，不得复活；
 * - docx 打印走 verifyDownloadEndpointHealthy（JSON helper 会对 200 的二进制做 JSON.parse 假红）；
 * - 每个创建/撤销动作之后都按 id / permit_no / monitoring_point 从 GET 端点回读
 *   真实落库字段值（类型、许可量、状态、超标判定、自动过期回写），toast 只作过程信号。
 *
 * 文案口径：environmental-compliance/index.vue 与 compliance.ts 是**非 i18n 页面**
 * （组件内为裸中文源文本），断言所用 toast/标题词与该组件源文件逐字符一致；
 * locales 中不存在对应 key，故不适用「i18n key 口径」，不存在裸字面量漂移风险。
 */

/**
 * 断言精确拒绝契约（收紧原「任意 4xx + 三族机器码来者不拒」的假绿）：
 * - HTTP 必须恰为 400（backend/utils/error.rs:356-373：ValidationError/BusinessError/
 *   BadRequest 全部映射 BAD_REQUEST 状态，本域拒绝不存在 409/其它 4xx 分支）；
 * - 机器码必须逐条等于调用点期望族——词表/参数校验族 = BAD_REQUEST
 *   （error.rs:791），状态门/唯一性/内控规则族 = BUSINESS_ERROR（error.rs:796-797）。
 * 两族可区分"后端做了参数校验"与"后端做了业务门禁"，混发即门别串号，判红交后端。
 * 永不读取 message 文案（AppError::business 出参脱敏为常量，文案不可判）。
 */
function expectRejected(r: ApiFailureResult, what: string, expectedCode: string): void {
  expect(
    r.status,
    `${what}：应恰为 HTTP 400（error.rs:356-373），实际 ${r.status} ${JSON.stringify(r)}`
  ).toBe(400);
  expect(
    failureCode(r),
    `${what}：机器码应为 ${expectedCode}，实际 ${failureCode(r)}（${JSON.stringify(r)}）`
  ).toBe(expectedCode);
}

const ymd = (daysFromToday: number): string =>
  new Date(Date.now() + daysFromToday * 86400_000).toISOString().slice(0, 10);

/** GET 列表信封真实性钉桩：/pollution-permits 与 /pollution-monitoring/records 的
 *  handler（pollution_permit_handler.rs:34-37 / pollution_monitoring_handler.rs:45-48）
 *  返回 data={items,total}。**显式断言 items 为数组**再取数——禁止 `?? []` 把缺键吞成空列表。 */
function requireItems(data: { items?: unknown }, endpoint: string): Array<Record<string, unknown>> {
  expect(
    Array.isArray(data?.items),
    `${endpoint} 响应 data.items 必须为数组（真实信封 {items,total}），实际 ${JSON.stringify(data)}`
  ).toBe(true);
  return data.items as Array<Record<string, unknown>>;
}

/** API 直建许可证（自给自足造数，不依赖其它用例），返回真实落库模型 */
async function createPermitApi(
  page: import('@playwright/test').Page,
  body: Record<string, unknown>
): Promise<Record<string, unknown>> {
  const res = await apiCall<Record<string, unknown>>(page, 'POST', '/pollution-permits', body);
  const model = res.data;
  expect(model?.id, `许可证创建应返回 id：${JSON.stringify(res)}`).toBeTruthy();
  return model;
}

test.describe.serial('60 环保合规：排污许可证 + 污染物监测 + 固废处置', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('60-01 页面可达 + 已注册端点严格健康（404/403 即判红）', async ({ page }) => {
    await page.goto(`${BASE_URL}/environmental-compliance`);
    await expect(page.getByRole('tab', { name: '排污许可证', exact: true })).toBeVisible();
    await expect(page.getByRole('tab', { name: '污染物监测', exact: true })).toBeVisible();

    await verifyEndpointHealthy(page, '/pollution-permits?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/pollution-permits/expiry-warnings');
    await verifyEndpointHealthy(page, '/pollution-monitoring/records?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/pollution-monitoring/exceedance-alerts');
  });

  test('60-02 UI 新建许可证（类型=固废/许可量 5000 t/a）→ 列表回读 → API 全字段落库真值', async ({
    page,
  }) => {
    const permitNo = genCode('E2E-PP');
    await page.goto(`${BASE_URL}/environmental-compliance`);
    await page.getByRole('button', { name: '新建许可证' }).click();
    const dialog = page.getByRole('dialog', { name: '新建排污许可证' });
    await expect(dialog).toBeVisible();

    // 必填星号是 CSS ::before 生成内容，不进 label textContent → 用 .is-required 类计数
    // 校验「必填标记真实渲染」，而不是断 label 含 *。
    await expect(
      dialog.locator('.el-form-item.is-required'),
      '许可证弹窗应有 5 个必填项（编号/类型/发证日期/到期日期/发证机关）'
    ).toHaveCount(5);

    await fillFieldByLabel(dialog, page, '许可证编号', permitNo);
    // 默认类型是废水——必须真实改选「固废」，验证 select 交互与值提交链路
    await pickSelectIn(dialog, page, '类型', { optionText: '固废' });
    // el-select 选中值读法：内层 .el-select__selected-item.el-select__placeholder 且非透明态
    await expect(
      formItemByExactLabel(dialog, '类型')
        .locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)')
        .first(),
      '类型选中值应回显「固废」'
    ).toHaveText('固废');
    await fillFieldByLabel(dialog, page, '发证日期', ymd(-10));
    await fillFieldByLabel(dialog, page, '到期日期', ymd(3650));
    await fillFieldByLabel(dialog, page, '发证机关', 'E2E生态环境局');
    await formItemByExactLabel(dialog, '许可量').locator('.el-input-number input').fill('5000');
    await fillFieldByLabel(dialog, page, '许可量单位', 't/a');
    await dialog.getByRole('button', { name: '保存' }).click();

    // toast 只作过程信号；真值以回读为准（防「只看 toast 不落库」假绿）
    await expect(page.getByText('许可证已创建')).toBeVisible({ timeout: 15_000 });

    // UI 列表回读：刷新后表格行含新编号（组件保存后自会 loadPermits，这里再点刷新走同一 GET）
    const row = page.locator('.el-table__row').filter({ hasText: permitNo }).first();
    await expect(row, `列表未回读到许可证 ${permitNo}`).toBeVisible({ timeout: 15_000 });
    await expect(row).toContainText('solid_waste');

    // API 落库真值回读（逐字段）
    const data = await apiCallRaw<{ items?: unknown }>(
      page,
      'GET',
      '/pollution-permits?permit_type=solid_waste&page=1&page_size=50'
    );
    const created = requireItems(data, 'GET /pollution-permits?permit_type=solid_waste').find(
      it => it.permit_no === permitNo
    );
    expect(created, `按 permit_type=solid_waste 筛选应命中 ${permitNo}`).toBeTruthy();
    expect(created?.permit_type, '落库 permit_type 应为 solid_waste').toBe('solid_waste');
    expect(Number(created?.permitted_capacity), '落库许可量应为 5000').toBe(5000);
    expect(created?.capacity_unit, '落库许可量单位应为 t/a').toBe('t/a');
    expect(created?.issuing_authority, '发证机关应逐字落库').toBe('E2E生态环境局');
    expect(created?.issue_date, '发证日期应逐字落库').toBe(ymd(-10));
    expect(created?.expiry_date, '到期日期应逐字落库').toBe(ymd(3650));
    expect(created?.status, '新建许可证初始状态应为 active').toBe('active');
  });

  test('60-03 UI 撤销许可证 → status=revoked 落库回读；重复撤销 API 判红', async ({ page }) => {
    const permitNo = genCode('E2E-RV');
    const created = await createPermitApi(page, {
      permit_no: permitNo,
      permit_type: 'wastewater',
      issue_date: ymd(-30),
      expiry_date: ymd(365),
      issuing_authority: 'E2E撤销测试局',
    });
    const id = Number(created.id);

    await page.goto(`${BASE_URL}/environmental-compliance`);
    const row = page.locator('.el-table__row').filter({ hasText: permitNo }).first();
    await expect(row, `撤销前列表应可见许可证 ${permitNo}`).toBeVisible({ timeout: 15_000 });
    await row.getByRole('button', { name: '撤销' }).click();
    await page.locator('.el-message-box__btns .el-button--primary').click();
    await expect(page.getByText('已撤销')).toBeVisible({ timeout: 15_000 });

    const detail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/pollution-permits/${id}`
    );
    expect(detail.status, '撤销后落库 status 应为 revoked').toBe('revoked');

    // 重复撤销：服务层显式拒绝（pollution_permit_service.rs::revoke，AppError::business）
    const dup = await apiCallExpectFail(page, 'POST', `/pollution-permits/${id}/revoke`);
    expectRejected(dup, `重复撤销许可证 ${permitNo}`, APP_ERROR_CODES.BUSINESS_ERROR);
  });

  test('60-04 许可证负例：非法类型（含 UI 选项 exhaust_gas/noise）、到期≤发证、编号重复', async ({
    page,
  }) => {
    // 后端词表 wastewater/exhaust/solid_waste（pollution_permit_service.rs:240-248）。
    // 前端弹窗选项 exhaust_gas 与 noise（environmental-compliance/index.vue:81-82）
    // 均不在词表内 → 选这两个项保存必 400。此处按后端真相判：UI 选项值一律被拒，
    // 作为「前端选项与后端词表漂移」的可回归证据（源码缺陷见 spec 交付报告，不改测试掩盖）。
    for (const badType of ['exhaust_gas', 'noise', 'radiation']) {
      const r = await apiCallExpectFail(page, 'POST', '/pollution-permits', {
        permit_no: genCode('E2E-BADT'),
        permit_type: badType,
        issue_date: ymd(-1),
        expiry_date: ymd(365),
        issuing_authority: 'E2E负例局',
      });
      // pollution_permit_service.rs::validate_permit_type → AppError::bad_request（词表族）
      expectRejected(r, `非法许可证类型 ${badType} 应被拒绝`, APP_ERROR_CODES.BAD_REQUEST);
    }
    // 到期日期必须晚于发证日期（同文件 create：AppError::bad_request，参数校验族）
    const same = await apiCallExpectFail(page, 'POST', '/pollution-permits', {
      permit_no: genCode('E2E-DT'),
      permit_type: 'wastewater',
      issue_date: ymd(10),
      expiry_date: ymd(10),
      issuing_authority: 'E2E负例局',
    });
    expectRejected(same, '到期=发证应被拒绝', APP_ERROR_CODES.BAD_REQUEST);

    const permitNo = genCode('E2E-DUP');
    await createPermitApi(page, {
      permit_no: permitNo,
      permit_type: 'wastewater',
      issue_date: ymd(-5),
      expiry_date: ymd(300),
      issuing_authority: 'E2E唯一性局',
    });
    const dup = await apiCallExpectFail(page, 'POST', '/pollution-permits', {
      permit_no: permitNo,
      permit_type: 'solid_waste',
      issue_date: ymd(-5),
      expiry_date: ymd(300),
      issuing_authority: 'E2E唯一性局',
    });
    // pollution_permit_service.rs::create 编号唯一性 → AppError::business
    expectRejected(dup, `重复编号 ${permitNo} 应被拒绝`, APP_ERROR_CODES.BUSINESS_ERROR);
  });

  test('60-05 UI 新建超标监测记录 → 超标标签 → 落库真值 + exceedance-alerts 命中', async ({
    page,
  }) => {
    const point = genCode('E2E-MP');
    const monitoringTime = new Date().toISOString();
    await page.goto(`${BASE_URL}/environmental-compliance`);
    await page.getByRole('tab', { name: '污染物监测', exact: true }).click();
    await page.getByRole('button', { name: '新建监测记录' }).click();
    const dialog = page.getByRole('dialog', { name: '新建监测记录' });
    await expect(dialog).toBeVisible();

    // 类型保持默认「废水」（后端词表合法项）；选中值按约定选择器回显断言
    await expect(
      formItemByExactLabel(dialog, '监测类型')
        .locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)')
        .first(),
      '监测类型默认应回显「废水」'
    ).toHaveText('废水');
    await fillFieldByLabel(dialog, page, '监测点', point);
    await fillFieldByLabel(dialog, page, '污染物', 'COD');
    await formItemByExactLabel(dialog, '实测值').locator('.el-input-number input').fill('120');
    await formItemByExactLabel(dialog, '限值').locator('.el-input-number input').fill('80');
    await fillFieldByLabel(dialog, page, '监测时间', monitoringTime);
    await dialog.getByRole('button', { name: '保存' }).click();
    await expect(page.getByText('监测记录已创建')).toBeVisible({ timeout: 15_000 });

    // UI 表格回读：超标标签真实渲染（is_exceeding 驱动）
    const row = page.locator('.el-table__row').filter({ hasText: point }).first();
    await expect(row, `监测列表应回读 ${point}`).toBeVisible({ timeout: 15_000 });
    await expect(
      row.getByRole('cell', { name: '超标', exact: true }),
      '超标行应显示红色超标标签'
    ).toBeVisible();

    // API 落库真值：is_exceeding=true、exceeding_ratio = 120/80 - 1 = 0.5
    const listed = await apiCallRaw<{ items?: unknown }>(
      page,
      'GET',
      '/pollution-monitoring/records?page=1&page_size=50'
    );
    const rec = requireItems(listed, 'GET /pollution-monitoring/records').find(
      it => it.monitoring_point === point
    );
    expect(rec, `records 列表应命中监测点 ${point}`).toBeTruthy();
    expect(rec?.is_exceeding, '实测 120 > 限值 80 应自动判定超标').toBe(true);
    expect(Number(rec?.exceeding_ratio), '超标倍数应为 120/80-1=0.5').toBeCloseTo(0.5, 4);
    expect(Number(rec?.measured_value), '实测值应落库 120').toBe(120);
    expect(Number(rec?.limit_value), '限值应落库 80').toBe(80);
    expect(rec?.pollutant_name, '污染物名应落库 COD').toBe('COD');

    // 未超标对照（同用例自建自流转）：实测 40 < 限值 80 → is_exceeding=false
    const okPoint = genCode('E2E-OK');
    await apiCall(page, 'POST', '/pollution-monitoring/records', {
      monitoring_type: 'wastewater',
      monitoring_point: okPoint,
      pollutant_name: 'COD',
      measured_value: '40',
      unit: 'mg/L',
      limit_value: '80',
      monitoring_time: new Date().toISOString(),
    });
    const listed2 = await apiCallRaw<{ items?: unknown }>(
      page,
      'GET',
      '/pollution-monitoring/records?page=1&page_size=50'
    );
    const okRec = requireItems(listed2, 'GET /pollution-monitoring/records(对照)').find(
      it => it.monitoring_point === okPoint
    );
    expect(okRec?.is_exceeding, '实测低于限值不得误判超标').toBe(false);

    // 超标扫描端点必须能扫到本用例制造的超标记录（真实包含关系，非只判 200）
    const alerts = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      '/pollution-monitoring/exceedance-alerts'
    );
    expect(Array.isArray(alerts), 'exceedance-alerts 应返回数组').toBe(true);
    const hit = alerts.find(
      a => (a.record as { monitoring_point?: string })?.monitoring_point === point
    );
    expect(hit, `超标扫描应包含新建记录 ${point}`).toBeTruthy();
    expect(Number(hit?.exceeding_ratio), '扫描出的超标倍数应为 0.5').toBeCloseTo(0.5, 4);
  });

  test('60-06 监测负例：限值≤0、非法类型（含 UI 选项 exhaust_gas/soil）', async ({ page }) => {
    // limit_value 是后端 DTO 必填非 Option 字段（pollution_monitoring_service.rs:31-45 的 :39
    // `pub limit_value: Decimal`），缺失时 serde 解码即失败、走归一后的 400 VALIDATION_ERROR，
    // 断言根本到不了服务层词表门。取 65：仅满足">0"使其能通过解码与限值门，
    // 取值本身不参与被测的 monitoring_type 词表校验（validate_monitoring_type 在 :126 先执行）。
    const base = {
      monitoring_type: 'wastewater',
      monitoring_point: genCode('E2E-MN'),
      pollutant_name: 'COD',
      measured_value: '10',
      unit: 'mg/L',
      limit_value: '65',
      monitoring_time: new Date().toISOString(),
    };
    const zeroLimit = await apiCallExpectFail(page, 'POST', '/pollution-monitoring/records', {
      ...base,
      limit_value: '0',
    });
    // pollution_monitoring_service.rs::create_monitoring_record 限值≤0 → AppError::bad_request
    expectRejected(zeroLimit, '限值=0 应被拒绝', APP_ERROR_CODES.BAD_REQUEST);

    // 后端词表 wastewater/exhaust/noise/solid_waste（pollution_monitoring_service.rs:340-347）。
    // 前端选项 exhaust_gas / soil（index.vue:119/121）不在词表 → 一律被拒（词表漂移证据）。
    for (const badType of ['exhaust_gas', 'soil', 'air']) {
      const r = await apiCallExpectFail(page, 'POST', '/pollution-monitoring/records', {
        ...base,
        monitoring_type: badType,
      });
      // pollution_monitoring_service.rs::validate_monitoring_type → AppError::bad_request
      expectRejected(r, `非法监测类型 ${badType} 应被拒绝`, APP_ERROR_CODES.BAD_REQUEST);
    }
    // 词表合法对照：noise 必须能通过（防校验过紧误杀）
    await apiCall(page, 'POST', '/pollution-monitoring/records', {
      ...base,
      monitoring_type: 'noise',
      limit_value: '65',
    });
  });

  test('60-07 到期预警：Warning30Days/Expired 分级命中，扫描后过期许可 status 自动回写 expired', async ({
    page,
  }) => {
    const warnNo = genCode('E2E-W30');
    const expiredNo = genCode('E2E-EXP');
    const warn = await createPermitApi(page, {
      permit_no: warnNo,
      permit_type: 'wastewater',
      issue_date: ymd(-355),
      expiry_date: ymd(15), // 15 天后到期 → 30 天级预警
      issuing_authority: 'E2E预警局',
    });
    const expired = await createPermitApi(page, {
      permit_no: expiredNo,
      permit_type: 'solid_waste',
      issue_date: ymd(-400),
      expiry_date: ymd(-2), // 已过期 → Expired + 状态自动回写
      issuing_authority: 'E2E过期局',
    });

    const warnings = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      '/pollution-permits/expiry-warnings'
    );
    const wHit = warnings.find(w => (w.permit as { permit_no?: string })?.permit_no === warnNo);
    const eHit = warnings.find(e => (e.permit as { permit_no?: string })?.permit_no === expiredNo);
    expect(wHit, `expiry-warnings 应包含 15 天后到期的 ${warnNo}`).toBeTruthy();
    expect(wHit?.level, '15 天后到期应分级为 Warning30Days').toBe('Warning30Days');
    expect(eHit, `expiry-warnings 应包含已过期的 ${expiredNo}`).toBeTruthy();
    expect(eHit?.level, '已过期应分级为 Expired').toBe('Expired');

    // 扫描副作用落库：过期许可 status 由 active 自动回写 expired（服务层 :242-252 真相）
    const detail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/pollution-permits/${Number(expired.id)}`
    );
    expect(detail.status, '扫描后过期许可证应自动落库 status=expired').toBe('expired');
    // 预警级许可证不受影响（防过杀）
    const detail2 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/pollution-permits/${Number(warn.id)}`
    );
    expect(detail2.status, '30 天预警（未到期）应保持 active').toBe('active');
  });

  test('60-08 许可证 docx 打印端点 2xx（二进制走 download 严格健康）', async ({ page }) => {
    const created = await createPermitApi(page, {
      permit_no: genCode('E2E-PRT'),
      permit_type: 'wastewater',
      issue_date: ymd(-1),
      expiry_date: ymd(3650),
      issuing_authority: 'E2E打印局',
    });
    await verifyDownloadEndpointHealthy(page, `/pollution-permits/${Number(created.id)}/print`);
  });

  test('60-09 固废联单：建单→运输→处置全链路状态落库 + 危废缺证/非法流转拒绝 + 打印', async ({
    page,
  }) => {
    const manifestNo = genCode('E2E-SW');
    const created = await apiCall<Record<string, unknown>>(
      page,
      'POST',
      '/pollution-monitoring/solid-waste-disposals',
      {
        manifest_no: manifestNo,
        waste_type: 'chemical_waste',
        waste_category: 'general',
        waste_amount: '120.50',
        waste_unit: 'kg',
        generation_date: ymd(0),
        disposal_method: 'incineration',
      }
    );
    const id = Number(created.data?.id);
    expect(id, `固废联单创建应返回 id：${JSON.stringify(created)}`).toBeGreaterThan(0);
    expect(created.data?.manifest_no, '联单号应逐字落库').toBe(manifestNo);
    expect(created.data?.status, '新建联单初始状态应为 pending').toBe('pending');

    // pending → transporting → disposed（词表与流转校验：pollution_monitoring_service.rs:283-289）
    const t1 = await apiCall<Record<string, unknown>>(
      page,
      'PUT',
      `/pollution-monitoring/solid-waste-disposals/${id}/status`,
      { status: 'transporting' }
    );
    expect(t1.data?.status, '状态应流转到 transporting').toBe('transporting');
    const disposeDate = ymd(0);
    const t2 = await apiCall<Record<string, unknown>>(
      page,
      'PUT',
      `/pollution-monitoring/solid-waste-disposals/${id}/status`,
      { status: 'disposed', disposal_date: disposeDate }
    );
    expect(t2.data?.status, '状态应流转到 disposed').toBe('disposed');
    expect(t2.data?.disposal_date, '处置日期应随流转落库').toBe(disposeDate);

    // 终态后非法回退
    const bad = await apiCallExpectFail(
      page,
      'PUT',
      `/pollution-monitoring/solid-waste-disposals/${id}/status`,
      { status: 'transporting' }
    );
    // 状态门拒绝（pollution_monitoring_service.rs::update_waste_status 非法流转 → AppError::business）
    expectRejected(bad, 'disposed 回退 transporting 应被拒绝', APP_ERROR_CODES.BUSINESS_ERROR);

    // 非法跨级：pending 直接 disposed（服务层只允许 pending→transporting/cancelled）
    const skip = await apiCall<Record<string, unknown>>(
      page,
      'POST',
      '/pollution-monitoring/solid-waste-disposals',
      {
        manifest_no: genCode('E2E-SW2'),
        waste_type: 'sludge',
        waste_category: 'general',
        waste_amount: '10',
        generation_date: ymd(0),
        disposal_method: 'landfill',
      }
    );
    const skipId = Number(skip.data?.id);
    const skipRes = await apiCallExpectFail(
      page,
      'PUT',
      `/pollution-monitoring/solid-waste-disposals/${skipId}/status`,
      { status: 'disposed' }
    );
    expectRejected(skipRes, 'pending 跨级到 disposed 应被拒绝', APP_ERROR_CODES.BUSINESS_ERROR);

    // 危废双证必填（pollution_monitoring_service.rs::create_solid_waste_disposal → AppError::business）
    const hz = await apiCallExpectFail(
      page,
      'POST',
      '/pollution-monitoring/solid-waste-disposals',
      {
        manifest_no: genCode('E2E-HZ'),
        waste_type: 'chemical_waste',
        waste_category: 'hazardous',
        waste_amount: '5',
        generation_date: ymd(0),
        disposal_method: 'incineration',
      }
    );
    expectRejected(hz, '危废缺运输/处置许可证号应被拒绝', APP_ERROR_CODES.BUSINESS_ERROR);

    // 非法处置方式词表外拒绝
    const badMethod = await apiCallExpectFail(
      page,
      'POST',
      '/pollution-monitoring/solid-waste-disposals',
      {
        manifest_no: genCode('E2E-BM'),
        waste_type: 'sludge',
        waste_category: 'general',
        waste_amount: '5',
        generation_date: ymd(0),
        disposal_method: 'ocean_dumping',
      }
    );
    // pollution_monitoring_service.rs::validate_disposal_method 词表外 → AppError::bad_request
    expectRejected(badMethod, '词表外处置方式应被拒绝', APP_ERROR_CODES.BAD_REQUEST);

    await verifyDownloadEndpointHealthy(
      page,
      `/pollution-monitoring/solid-waste-disposals/${id}/print`
    );
  });
});
