import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  deferCleanup,
  flushDeferredCleanups,
  expectStateGateRejection,
  type DeferredCleanup,
} from './helpers';

/**
 * 93 能耗契约缺口补测（计量设备建账 能耗登记服务端算量与读数回写 分摊记录算比与确认门）
 *
 * 与既有覆盖的分工。24 号对本域只发过 verifyEndpointHealthy 与 safePostAction
 * 纯状态探针（判定器 b1 b3 档），31b 号只探计量设备删除守卫。本 spec 是能耗三类
 * 写端点的首个真契约覆盖，钉金额数量列的库内真值、跨实体读数回写与状态门禁。
 *
 * 端点与信封真相（backend/src/routes/production.rs 的 energy 路由组，挂载前缀
 * /api/v1/erp/production）：
 * POST 计量设备创建（energy_handler::create_energy_meter）、GET 详情（get_energy_meter）、
 * GET 按编号（get_energy_meter_by_no）、DELETE 软删（delete_energy_meter）。
 * POST 能耗登记（create_energy_consumption）、GET 按编号（get_energy_consumption_by_no）、
 * POST confirm cancel 两条状态流转。
 * POST 分摊记录创建（create_energy_allocation）、GET 详情（get_energy_allocation）、
 * PUT 更新（update_energy_allocation）、POST confirm cancel 两条状态流转。
 * 单对象端点出参 data 为模型本体，分页端点出参 PaginatedResponse 四键。
 *
 * 词表与算量真相（backend/src/models/status/wage_energy_chemical_business.rs 与
 * services/energy_service.rs 纯函数、services/energy_ops 各文件）：
 * 能源类型 water electricity steam gas compressed_air（energy_type，
 * validate_meter_type 拒绝其余值）。录入方式 manual iot auto_calc，缺省 manual。
 * 记录状态 draft confirmed cancelled（energy_record_status），设备状态缺省 active。
 * 消耗量 compute_consumption 规则为当前读数减上次读数，当前读数小于上次读数
 * 时钳为 0（表计回零语义，是成文契约不是缺陷）。总成本为消耗量乘单价。
 * 分摊缺省比例为 1，分摊消耗量与分摊成本缺省按比例从总量派生，
 * 单位能耗为分摊消耗量除以产量，产量缺省时存 NULL。
 * 单号统一生成器取号，设备 EM 能耗 EC 分摊 EAR，均为前缀加日期八位加流水三位。
 * 能耗登记与设备读数回写同事务（consumption.rs::sync_meter_readings），
 * 登记确认后设备表读数必须等于该笔登记的读数，这是跨实体副作用的真实落点。
 * 状态门 能耗 confirm 只放行 draft，cancel 拒绝已 cancelled 行，
 * 分摊 confirm 与 update 与 delete 均只放行 draft。
 *
 * 假绿防线。金额数量列一律 Number 归一后比数值（rust_decimal 出参为字符串）。
 * 负例 apiCallExpectFail 加 expectStateGateRejection 钉 HTTP 400 与状态门机器码族，
 * 不读文案（AppError 业务文案出参被掩码），每条被拒写操作紧跟 GET 回读断零漂移。
 * 可空未提交列断 NULL。周期与引用负例在拒绝后按唯一 workshop 分页探针断零写入。
 * 数据自建自流转，唯一 workshop 码隔离，清理走延迟队列（能耗与分摊软删只放行
 * draft，已流转行由状态门拒绝删除，tryCleanup 告警留痕，设备无删除门必清）。
 */

type Row = Record<string, unknown>;

/** 延迟清理队列（全部断言之后统一 flush，队列本文件私有） */
const CLEANUP: DeferredCleanup[] = [];
test.afterEach(async ({ page }) => {
  await flushDeferredCleanups(page, CLEANUP);
});

/** rust_decimal 经 JSON 序列化为字符串，比较前归一 */
const toNum = (v: unknown): number => Number(String(v));

const isoNow = (): string => new Date().toISOString();
const isoPlusHours = (h: number): string => new Date(Date.now() + h * 3_600_000).toISOString();

/** 自建计量设备（缺省 active，unit 缺省为度，编号 EM 前缀） */
async function createMeter(
  page: Page,
  workshop: string,
  overrides: Record<string, unknown> = {}
): Promise<{ id: number; meterNo: string }> {
  const res = await apiCall<Row>(page, 'POST', '/production/energy-meters', {
    meter_name: `93号电表${workshop}`,
    meter_type: 'electricity',
    workshop,
    unit_price: '2.00',
    previous_reading: '0',
    current_reading: '100',
    ...overrides,
  });
  const id = Number(res.data?.id);
  expect(id, `计量设备创建应返回 id，实际 ${JSON.stringify(res)}`).toBeGreaterThan(0);
  const meterNo = String(res.data?.meter_no);
  deferCleanup(CLEANUP, 'DELETE', `/production/energy-meters/${id}`, `[93] meter ${meterNo}`);
  return { id, meterNo };
}

/** 自建能耗登记（读数由调用方给定，返回 id 与单号供回读） */
async function createConsumption(
  page: Page,
  meterId: number,
  workshop: string,
  prev: string,
  cur: string,
  overrides: Record<string, unknown> = {}
): Promise<{ id: number; recordNo: string }> {
  const res = await apiCall<Row>(page, 'POST', '/production/energy-consumptions', {
    meter_id: meterId,
    meter_type: 'electricity',
    workshop,
    previous_reading: prev,
    current_reading: cur,
    unit_price: '2.00',
    period_start: isoNow(),
    period_end: isoPlusHours(1),
    ...overrides,
  });
  const id = Number(res.data?.id);
  expect(id, `能耗登记应返回 id，实际 ${JSON.stringify(res)}`).toBeGreaterThan(0);
  const recordNo = String(res.data?.record_no);
  deferCleanup(CLEANUP, 'DELETE', `/production/energy-consumptions/${id}`, `[93] cons ${recordNo}`);
  return { id, recordNo };
}

/** 自建分摊记录（缺省比例 1，分摊量与成本由服务端从总量派生） */
async function createAllocation(
  page: Page,
  workshop: string,
  overrides: Record<string, unknown> = {}
): Promise<{ id: number; allocationNo: string }> {
  const res = await apiCall<Row>(page, 'POST', '/production/energy-allocations', {
    period_start: isoNow(),
    period_end: isoPlusHours(24),
    meter_type: 'water',
    workshop,
    allocation_basis: 'by_output',
    total_consumption: '200',
    total_cost: '800',
    allocation_basis_value: '50',
    ...overrides,
  });
  const id = Number(res.data?.id);
  expect(id, `分摊记录创建应返回 id，实际 ${JSON.stringify(res)}`).toBeGreaterThan(0);
  const allocationNo = String(res.data?.allocation_no);
  deferCleanup(
    CLEANUP,
    'DELETE',
    `/production/energy-allocations/${id}`,
    `[93] alloc ${allocationNo}`
  );
  return { id, allocationNo };
}

/** 按 workshop 精确过滤的分页探针（负例零写入判据，四键全断） */
async function countByWorkshop(page: Page, endpoint: string, workshop: string): Promise<number> {
  const data = await apiCallRaw<{
    items?: unknown;
    total?: unknown;
    page?: unknown;
    page_size?: unknown;
  }>(page, 'GET', `/production/${endpoint}?workshop=${workshop}&page=1&page_size=5`);
  expect(
    Array.isArray(data?.items),
    `${endpoint} 分页 items 必须为数组，实际 ${JSON.stringify(data)}`
  ).toBe(true);
  expect(typeof data.total, `${endpoint} total 必须为数字（禁兜底默认值）`).toBe('number');
  expect(typeof data.page, `${endpoint} page 必须回显`).toBe('number');
  expect(typeof data.page_size, `${endpoint} page_size 必须回显`).toBe('number');
  return Number(data.total);
}

/**
 * 凭证列表定位分摊确认生成的能耗凭证（allocation_record.rs::create_energy_voucher_and_collect_cost）。
 * 入参是分摊单号；GET /vouchers 的查询链（voucher_handler.rs::VoucherQuery 与
 * voucher_ops/crud.rs::get_list）在册过滤仅 voucher_no、voucher_type、status 与日期段，
 * 没有按 source_bill_no 的通道，未知查询参数会被反序列化静默忽略，所以只能带
 * voucher_type 与 status 两记在册过滤走服务端缩小（凭证表按 voucher_date 降序出参，
 * 本轮新建凭证日期最新，缺省首页即可命中），取回裸数组后按行级
 * source_bill_no、source_module 与 source_type 逐字段等值锁定，取不到即判红，
 * 不做客户端扩大翻页扫描。传给用例做凭证头断言与 GET /vouchers/{id} 明细回读。
 */
async function findEnergyVoucher(page: Page, allocationNo: string): Promise<Row> {
  const rows = await apiCallRaw<Row[]>(
    page,
    'GET',
    '/vouchers?voucher_type=transfer&status=posted&page=1&page_size=100'
  );
  expect(Array.isArray(rows), `GET /vouchers 出参必须为裸数组，实际 ${JSON.stringify(rows)}`).toBe(
    true
  );
  const hit = (rows as Row[]).find(
    v =>
      String(v.source_bill_no) === allocationNo &&
      String(v.source_module) === 'energy_allocation_record' &&
      String(v.source_type) === 'energy'
  );
  expect(
    hit,
    `首页窗口内应含 source_bill_no=${allocationNo} 且 source_module=energy_allocation_record 的已过账凭证；` +
      '凭证由分摊确认旁路生成，后端生成失败仅记日志不阻断确认，缺失即该旁路断链判红'
  ).toBeTruthy();
  return hit as Row;
}

test.describe.serial('93 能耗契约：登记算量回写与分摊确认门', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('93-01 计量设备建账落真值，单号与详情双读路径一致', async ({ page }) => {
    const workshop = genCode('E93MSA');
    const created = await createMeter(page, workshop);

    expect(
      created.meterNo,
      `设备编号应为 EM 加日期八位加流水三位，实际 ${created.meterNo}`
    ).toMatch(/^EM\d{11}$/);

    const got = await apiCallRaw<Row>(page, 'GET', `/production/energy-meters/${created.id}`);
    expect(Number(got.id), '详情 id 应等于建账 id').toBe(created.id);
    expect(got.status, '设备状态缺省应为 active（meter.rs 建账固定值）').toBe('active');
    expect(got.meter_type, '能源类型应逐字落库').toBe('electricity');
    expect(String(got.unit), '单位未提交应落缺省值度').toBe('度');
    expect(toNum(got.unit_price), '单价应落为 2.00 真值').toBe(2);
    expect(toNum(got.previous_reading), '上次读数应落 0').toBe(0);
    expect(toNum(got.current_reading), '当前读数应落 100').toBe(100);
    expect(String(got.workshop), '车间应逐字落库').toBe(workshop);
    expect(got.equipment_id, '未提交设备引用应存 NULL 而非 0 占位').toBeNull();
    expect(got.location, '未提交安装位置应存 NULL').toBeNull();

    const byNo = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/energy-meters/by-no/${created.meterNo}`
    );
    expect(Number(byNo.id), 'by-no 与详情应回同一行').toBe(created.id);

    // 非法能源类型与负单价两条建账负例被拒且该车间零写入
    const rejectWorkshop = genCode('E93MSB');
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', '/production/energy-meters', {
        meter_name: '93非法类型探针',
        meter_type: 'firewood',
        workshop: rejectWorkshop,
      }),
      '非法能源类型应被业务门拒绝'
    );
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', '/production/energy-meters', {
        meter_name: '93负单价探针',
        meter_type: 'electricity',
        workshop: rejectWorkshop,
        unit_price: '-1.00',
      }),
      '负单价应被业务门拒绝'
    );
    expect(
      await countByWorkshop(page, 'energy-meters', rejectWorkshop),
      `两条建账负例拒绝后不应落库，workshop=${rejectWorkshop}`
    ).toBe(0);
  });

  test('93-02 能耗登记服务端算量与设备读数回写，confirm 后 cancel 全程', async ({ page }) => {
    const workshop = genCode('E93CSA');
    const meter = await createMeter(page, workshop);
    const created = await createConsumption(page, meter.id, workshop, '100', '150.5');

    expect(created.recordNo, `能耗单号应为 EC 前缀，实际 ${created.recordNo}`).toMatch(
      /^EC\d{11}$/
    );

    // 登记回读钉服务端派生列真值（消耗量 50.5 与总成本 101 均由后端算出，请求体不承载）
    const got = await apiCallRaw<Row>(page, 'GET', `/production/energy-consumptions/${created.id}`);
    expect(Number(got.id), '能耗详情 id 应等于登记 id').toBe(created.id);
    expect(toNum(got.consumption), '消耗量应为读数差 50.5').toBe(50.5);
    expect(toNum(got.total_cost), '总成本应为消耗量乘单价 101').toBe(101);
    expect(toNum(got.previous_reading), '上次读数应逐字落库').toBe(100);
    expect(toNum(got.current_reading), '当前读数应逐字落库').toBe(150.5);
    expect(toNum(got.unit_price), '单价应落库 2.00').toBe(2);
    expect(got.status, '新建能耗应为 draft').toBe('draft');
    expect(got.recording_method, '录入方式未提交应落缺省 manual').toBe('manual');
    expect(got.meter_id, '设备引用应落库').not.toBeNull();
    expect(Number(got.meter_id), '设备引用应指向自建设备').toBe(meter.id);

    const byNo = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/energy-consumptions/by-no/${created.recordNo}`
    );
    expect(Number(byNo.id), 'by-no 与详情应回同一行').toBe(created.id);
    expect(toNum(byNo.total_cost), 'by-no 总成本应与详情一致').toBe(101);

    // 跨实体副作用钉桩 同事务读数回写（consumption.rs::sync_meter_readings）
    const meterAfter = await apiCallRaw<Row>(page, 'GET', `/production/energy-meters/${meter.id}`);
    expect(toNum(meterAfter.previous_reading), '登记后设备上次读数应被回写为 100').toBe(100);
    expect(toNum(meterAfter.current_reading), '登记后设备当前读数应被回写为 150.5').toBe(150.5);

    // 表计回零钳 0 契约（compute_consumption 成文规则，负读数差不落账）
    const clamp = await createConsumption(page, meter.id, workshop, '300', '200');
    const clampView = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/energy-consumptions/${clamp.id}`
    );
    expect(toNum(clampView.consumption), '当前读数回退时消耗量应钳为 0').toBe(0);
    expect(toNum(clampView.total_cost), '钳 0 后总成本应为 0 不得为负数').toBe(0);

    // confirm 只放行 draft 一次，重复确认被拒且状态零漂移
    const confirmed = await apiCall<Row>(
      page,
      'POST',
      `/production/energy-consumptions/${created.id}/confirm`
    );
    expect(String(confirmed.data?.status), 'confirm 应返回 confirmed 态').toBe('confirmed');
    expectStateGateRejection(
      await apiCallExpectFail(
        page,
        'POST',
        `/production/energy-consumptions/${created.id}/confirm`
      ),
      'confirmed 上重复 confirm 应被状态门拒绝'
    );
    const afterDup = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/energy-consumptions/${created.id}`
    );
    expect(String(afterDup.status), '被拒的重复 confirm 不应改变状态').toBe('confirmed');
    expect(toNum(afterDup.total_cost), '被拒的重复 confirm 不应改动成本原值').toBe(101);

    // confirmed 到 cancelled 合法，重复 cancel 被拒（cancel 门拒绝已 cancelled 行）
    const cancelled = await apiCall<Row>(
      page,
      'POST',
      `/production/energy-consumptions/${created.id}/cancel`
    );
    expect(String(cancelled.data?.status), 'confirmed 上 cancel 应返回 cancelled 态').toBe(
      'cancelled'
    );
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', `/production/energy-consumptions/${created.id}/cancel`),
      'cancelled 上重复 cancel 应被业务门拒绝'
    );
    const finalView = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/energy-consumptions/${created.id}`
    );
    expect(String(finalView.status), '被拒的重复 cancel 后状态应保持 cancelled').toBe('cancelled');
  });

  test('93-03 能耗登记时段与引用负例拒绝且零写入', async ({ page }) => {
    const workshop = genCode('E93CSB');
    const meter = await createMeter(page, workshop);

    const periodWorkshop = genCode('E93CSC');
    const ghostWorkshop = genCode('E93CSD');
    const methodWorkshop = genCode('E93CSE');

    // 时段倒挂（consumption.rs 校验结束时间必须晚于开始时间）
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', '/production/energy-consumptions', {
        meter_id: meter.id,
        meter_type: 'electricity',
        workshop: periodWorkshop,
        previous_reading: '1',
        current_reading: '2',
        period_start: isoPlusHours(2),
        period_end: isoNow(),
      }),
      '时段倒挂的能耗登记应被业务门拒绝'
    );

    // 幽灵设备引用（validate_create_request 校验设备存在性）
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', '/production/energy-consumptions', {
        meter_id: 999999999,
        meter_type: 'electricity',
        workshop: ghostWorkshop,
        previous_reading: '1',
        current_reading: '2',
        period_start: isoNow(),
        period_end: isoPlusHours(1),
      }),
      '不存在的设备引用应被业务门拒绝'
    );

    // 非法录入方式（resolve_recording_method 值域 manual iot auto_calc）
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', '/production/energy-consumptions', {
        meter_id: meter.id,
        meter_type: 'electricity',
        workshop: methodWorkshop,
        previous_reading: '1',
        current_reading: '2',
        period_start: isoNow(),
        period_end: isoPlusHours(1),
        recording_method: 'estimate',
      }),
      '非法录入方式应被业务门拒绝'
    );

    // 三个负例各自使用的真实 workshop 均零落库（查询的就是被拒请求携带的码）
    expect(
      await countByWorkshop(page, 'energy-consumptions', periodWorkshop),
      `时段倒挂拒绝后不应落库，workshop=${periodWorkshop}`
    ).toBe(0);
    expect(
      await countByWorkshop(page, 'energy-consumptions', ghostWorkshop),
      `幽灵引用拒绝后不应落库，workshop=${ghostWorkshop}`
    ).toBe(0);
    expect(
      await countByWorkshop(page, 'energy-consumptions', methodWorkshop),
      `非法录入方式拒绝后不应落库，workshop=${methodWorkshop}`
    ).toBe(0);
  });

  test('93-04 分摊记录缺省比例派生量与显式比例派生量都落真值，confirm 后更新被拒', async ({
    page,
  }) => {
    const workshop = genCode('E93ASA');
    const defaultRatio = await createAllocation(page, workshop, { output_quantity: '40' });
    expect(
      defaultRatio.allocationNo,
      `分摊编号应为 EAR 前缀，实际 ${defaultRatio.allocationNo}`
    ).toMatch(/^EAR\d{11}$/);

    // 缺省比例 1 时分摊量等于总量，单位能耗等于分摊量除以产量
    const got = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/energy-allocations/${defaultRatio.id}`
    );
    expect(got.status, '新建分摊应为 draft').toBe('draft');
    expect(got.meter_type, '能源类型应逐字落库 water').toBe('water');
    expect(got.allocation_basis, '分摊基准应逐字落库 by_output').toBe('by_output');
    expect(toNum(got.allocation_ratio), '比例未提交应落缺省 1').toBe(1);
    expect(toNum(got.allocated_consumption), '分摊消耗量应等于总量 200').toBe(200);
    expect(toNum(got.allocated_cost), '分摊成本应等于总成本 800').toBe(800);
    expect(toNum(got.unit_consumption), '单位能耗应为 200 除以 40 等于 5').toBe(5);
    expect(got.confirmed_by, '未确认行 confirmed_by 应存 NULL').toBeNull();
    expect(got.confirmed_at, '未确认行 confirmed_at 应存 NULL').toBeNull();
    expect(got.cost_collection_id, '未归集行 cost_collection_id 应存 NULL').toBeNull();
    expect(got.flow_card_id, '未提交流转卡引用应存 NULL').toBeNull();

    // 显式比例 0.25 时派生列按比例收缩，产量缺省时单位能耗存 NULL
    const quarter = await createAllocation(page, workshop, {
      allocation_ratio: '0.25',
      total_consumption: '200',
      total_cost: '800',
    });
    const quarterView = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/energy-allocations/${quarter.id}`
    );
    expect(toNum(quarterView.allocation_ratio), '显式比例应逐字落库 0.25').toBe(0.25);
    expect(toNum(quarterView.allocated_consumption), '分摊消耗量应为 200 乘 0.25 等于 50').toBe(50);
    expect(toNum(quarterView.allocated_cost), '分摊成本应为 800 乘 0.25 等于 200').toBe(200);
    expect(quarterView.unit_consumption, '产量未提交时单位能耗应存 NULL').toBeNull();

    // draft 期更新合法（update 门只放行 draft），改后回读真值
    await apiCall(page, 'PUT', `/production/energy-allocations/${quarter.id}`, {
      total_cost: '900',
      allocation_ratio: '0.5',
    });
    const edited = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/energy-allocations/${quarter.id}`
    );
    expect(toNum(edited.total_cost), 'draft 期更新总成本应落库 900').toBe(900);
    expect(toNum(edited.allocation_ratio), 'draft 期更新比例应落库 0.5').toBe(0.5);

    // confirm 后更新被拒且成本原值零漂移（update 门拒绝非 draft）
    const confirmed = await apiCall<Row>(
      page,
      'POST',
      `/production/energy-allocations/${defaultRatio.id}/confirm`
    );
    expect(String(confirmed.data?.status), 'confirm 应返回 confirmed 态').toBe('confirmed');
    expectStateGateRejection(
      await apiCallExpectFail(page, 'PUT', `/production/energy-allocations/${defaultRatio.id}`, {
        total_cost: '999',
      }),
      'confirmed 分摊更新应被状态门拒绝'
    );
    const afterUpdate = await apiCallRaw<Row>(
      page,
      'GET',
      `/production/energy-allocations/${defaultRatio.id}`
    );
    expect(toNum(afterUpdate.total_cost), '被拒更新不得改写成本原值').toBe(800);
    expect(String(afterUpdate.status), '被拒更新不得改变状态').toBe('confirmed');

    // 确认到账反查：分摊确认旁路生成的能耗凭证必须真实落账。
    // 凭证头锁定（voucher_type=transfer、status=posted 由在册过滤条件钉住，
    // source_bill_no 等于分摊单号、source_module 与 source_type 由定位函数逐字段核）；
    // 凭证主表无金额汇总列（voucher::Model 只有数量三列），借贷总额一律由分录腿求和推导，
    // 各腿金额列经 Number 归一与分摊 allocated_cost 比对
    const voucher = await findEnergyVoucher(page, defaultRatio.allocationNo);
    expect(String(voucher.voucher_type), '能耗凭证类型应为 transfer').toBe('transfer');
    expect(String(voucher.status), '能耗凭证应为已过账 posted 态').toBe('posted');
    expect(String(voucher.source_bill_no), '凭证来源单号应为被确认的分摊单号').toBe(
      defaultRatio.allocationNo
    );

    const detail = await apiCallRaw<{ entries?: Row[] }>(
      page,
      'GET',
      `/vouchers/${Number(voucher.id)}`
    );
    const entries = Array.isArray(detail.entries) ? detail.entries : [];
    expect(entries.length, `能耗凭证分录应为两腿，实际 ${JSON.stringify(detail)}`).toBe(2);
    const debitSum = entries.reduce((acc, e) => acc + toNum(e.debit), 0);
    const creditSum = entries.reduce((acc, e) => acc + toNum(e.credit), 0);
    expect(debitSum, '借方合计应等于分摊 allocated_cost 800').toBe(800);
    expect(creditSum, '贷方合计应等于分摊 allocated_cost 800').toBe(800);
    const debitLine = entries.find(e => String(e.subject_code) === '500103');
    expect(debitLine, '应含借方科目 500103 生产成本-制造费用一行').toBeTruthy();
    expect(toNum((debitLine as Row).debit), '借方行金额应为 allocated_cost 800').toBe(800);
    expect(toNum((debitLine as Row).credit), '借方行贷方应为 0').toBe(0);
    expect(String((debitLine as Row).summary), '借方行摘要应含能耗分摊确认与原单号').toBe(
      `能耗分摊确认 ${defaultRatio.allocationNo} (water)`
    );
    const creditLine = entries.find(e => String(e.subject_code) === '2202');
    expect(creditLine, '应含贷方科目 2202 应付账款-水电费一行').toBeTruthy();
    expect(toNum((creditLine as Row).credit), '贷方行金额应为 allocated_cost 800').toBe(800);
    expect(toNum((creditLine as Row).debit), '贷方行借方应为 0').toBe(0);

    // 负锚：不存在的分摊单号在窗口内不得命中任何凭证行
    const ghostNo = genCode('EAR99999999');
    const rowsAfter = await apiCallRaw<Row[]>(
      page,
      'GET',
      '/vouchers?voucher_type=transfer&status=posted&page=1&page_size=100'
    );
    expect(
      (rowsAfter as Row[]).some(v => String(v.source_bill_no) === ghostNo),
      `幽灵单号 ${ghostNo} 不应命中任何凭证行`
    ).toBe(false);

    // confirmed 到 cancelled 合法，重复 cancel 被拒
    const cancelled = await apiCall<Row>(
      page,
      'POST',
      `/production/energy-allocations/${defaultRatio.id}/cancel`
    );
    expect(String(cancelled.data?.status), 'confirmed 上 cancel 应返回 cancelled 态').toBe(
      'cancelled'
    );
    expectStateGateRejection(
      await apiCallExpectFail(
        page,
        'POST',
        `/production/energy-allocations/${defaultRatio.id}/cancel`
      ),
      'cancelled 上重复 cancel 应被业务门拒绝'
    );
  });

  test('93-05 分摊建单基准与时段负例拒绝且零写入', async ({ page }) => {
    const badBasisWorkshop = genCode('E93ASB');
    // 非法分摊基准（validate_allocation_basis 值域四键之外）
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', '/production/energy-allocations', {
        period_start: isoNow(),
        period_end: isoPlusHours(24),
        meter_type: 'water',
        workshop: badBasisWorkshop,
        allocation_basis: 'by_mood',
        total_consumption: '10',
        total_cost: '10',
        allocation_basis_value: '1',
      }),
      '非法分摊基准应被业务门拒绝'
    );

    const badPeriodWorkshop = genCode('E93ASC');
    // 时段倒挂（allocation_record.rs 校验结束时间必须晚于开始时间）
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', '/production/energy-allocations', {
        period_start: isoPlusHours(24),
        period_end: isoNow(),
        meter_type: 'electricity',
        workshop: badPeriodWorkshop,
        allocation_basis: 'by_duration',
        total_consumption: '10',
        total_cost: '10',
        allocation_basis_value: '1',
      }),
      '时段倒挂的分摊建单应被业务门拒绝'
    );

    const badTypeWorkshop = genCode('E93ASD');
    // 非法能源类型
    expectStateGateRejection(
      await apiCallExpectFail(page, 'POST', '/production/energy-allocations', {
        period_start: isoNow(),
        period_end: isoPlusHours(24),
        meter_type: 'firewood',
        workshop: badTypeWorkshop,
        allocation_basis: 'by_output',
        total_consumption: '10',
        total_cost: '10',
        allocation_basis_value: '1',
      }),
      '非法能源类型的分摊建单应被业务门拒绝'
    );

    // 三条拒绝各自唯一 workshop 均零落库
    expect(
      await countByWorkshop(page, 'energy-allocations', badBasisWorkshop),
      `非法基准拒绝后不应落库，workshop=${badBasisWorkshop}`
    ).toBe(0);
    expect(
      await countByWorkshop(page, 'energy-allocations', badPeriodWorkshop),
      `时段倒挂拒绝后不应落库，workshop=${badPeriodWorkshop}`
    ).toBe(0);
    expect(
      await countByWorkshop(page, 'energy-allocations', badTypeWorkshop),
      `非法类型拒绝后不应落库，workshop=${badTypeWorkshop}`
    ).toBe(0);
  });
});
