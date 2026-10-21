// 生产链深度契约 E2E — BOM 用量 → 排程确认 → 染色批次 → 验布打卷 → 质检报废审批
//
// 本文件守的是同一族缺陷：**结果写了但没人推进、维度跨环漂移、数量口径混用、
// 审批门可被跳过**。与既有用例的分工：既有生产用例多为端点可达性与状态标签校验，
// 本文件补的正是那半条没断的账——排程结果未确认时工单必须原地不动、确认后必须真推进；
// 缸号/色号/批次/匹号在「染色档案 → 验布记录 → 打卷生成的实物匹」每一环必须逐字符同源；
// 米数/公斤/匹数三口径各归各位不得互相顶替；报废必须两级且不可跳级。
//
// 断言纪律：十进制出参按字符串定标比较（禁 JS number 累加）；每个写动作后 GET 读回逐字段；
// 负例锁 HTTP 码 + 机器码，可外显站点逐字符锁文案，被脱敏站点只锁码并点名根因；
// 一切基准由本用例自建的唯一命名实体导出，不引用库中历史行。
import { test, expect } from '../diagnose-fixture';
import { type Page } from '@playwright/test';
import {
  loginViaUI,
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  APP_ERROR_CODES,
  genCode,
  genDyeLotNo,
  tryCleanup,
  pickDyeableWarehouse,
  seedDyedPieceChain,
  fetchAvailableDyedPieces,
} from './helpers';
import { pickListArray } from './ui-helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

// ───────────────────────────── 十进制字符串对账（禁浮点）─────────────────────────────

interface Dec {
  v: bigint;
  s: number;
}

function decFromRaw(raw: unknown, label: string): Dec {
  const text = typeof raw === 'string' ? raw.trim() : typeof raw === 'number' ? String(raw) : null;
  if (text === null) {
    throw new Error(
      `${label}：后端 Decimal 出参必须是字符串，实际类型=${typeof raw} 值=${JSON.stringify(raw)}`
    );
  }
  const m = /^(-?)(\d+)(?:\.(\d+))?$/.exec(text);
  if (!m) throw new Error(`${label}：无法按十进制解析，raw=${JSON.stringify(raw)}`);
  const sign = m[1] === '-' ? -1n : 1n;
  return { v: sign * BigInt(`${m[2]}${m[3] ?? ''}`), s: (m[3] ?? '').length };
}

function decLiteral(text: string, label: string): Dec {
  return decFromRaw(text, label);
}

function alignDec(a: Dec, b: Dec): [bigint, bigint, number] {
  const s = Math.max(a.s, b.s);
  return [a.v * 10n ** BigInt(s - a.s), b.v * 10n ** BigInt(s - b.s), s];
}

function decMul(a: Dec, b: Dec): Dec {
  return { v: a.v * b.v, s: a.s + b.s };
}

function decCmp(a: Dec, b: Dec): number {
  const [x, y] = alignDec(a, b);
  return x < y ? -1 : x > y ? 1 : 0;
}

function decText(d: Dec): string {
  const sign = d.v < 0n ? '-' : '';
  const digits = (d.v < 0n ? -d.v : d.v).toString().padStart(d.s + 1, '0');
  return d.s === 0 ? `${sign}${digits}` : `${sign}${digits.slice(0, -d.s)}.${digits.slice(-d.s)}`;
}

/** 十进制相等（定标对齐后比整数） */
function expectDecEquals(actual: unknown, expected: Dec, label: string): void {
  const a = decFromRaw(actual, label);
  expect(
    decCmp(a, expected) === 0,
    `${label}：十进制期望=${decText(expected)} 实际=${decText(a)}（raw=${JSON.stringify(actual)}）`
  ).toBe(true);
}

/** 出参必须是十进制字符串（Decimal 序列化契约） */
function expectDecimalString(raw: unknown, label: string): Dec {
  expect(
    typeof raw,
    `${label}：Decimal 出参应为 JSON 字符串，实际类型=${typeof raw} 值=${JSON.stringify(raw)}`
  ).toBe('string');
  return decFromRaw(raw, label);
}

function requireId(row: Record<string, unknown>, label: string): number {
  const id = Number(row?.id);
  if (!Number.isFinite(id) || id <= 0) {
    throw new Error(`${label}：未取到有效 id，实际键=${Object.keys(row ?? {}).join(',')}`);
  }
  return id;
}

/** 取本用例专属产品（自建 + 回读，不引用库中 unit 未知的历史产品） */
async function ownProduct(page: Page, tag: string, unit = '米'): Promise<{
  id: number;
  code: string;
  name: string;
  unit: string;
}> {
  const ctx = getCtx();
  if (!ctx.productCategoryIds[0]) throw new Error('前置缺失：ctx.productCategoryIds[0] 未就绪');
  const code = genCode(`E96${tag}`);
  const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/products', {
    code,
    name: `E2E生产链${tag}${code}`,
    unit,
    category_id: ctx.productCategoryIds[0],
  });
  const id = requireId(created, `建产品 ${code}`);
  CLEANUP.push({ path: `/products/${id}`, label: `product#${id}` });
  const readBack = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/products/${id}`);
  expect(readBack.code, '产品编码回读').toBe(code);
  expect(readBack.unit, '产品计量单位回读').toBe(unit);
  return { id, code, name: String(readBack.name ?? ''), unit };
}

/**
 * 读回不合格品/报废台账行。
 * 该域没有按台账行主键的 GET 单条端点，唯一读取面是按质检记录锚定的列表
 * （`/production/quality-inspection/defects?record_id=`，出参 data 为裸数组），
 * 因此按 record_id 收窄后再用行主键精确定位；取不到即判红，不做空数组兜底。
 */
async function readDefectRow(
  page: Page,
  opts: { defectId: number; recordId: number; label: string }
): Promise<Record<string, unknown>> {
  const rows = pickListArray<Record<string, unknown>>(
    await apiCallRaw<unknown>(
      page,
      'GET',
      `/production/quality-inspection/defects?record_id=${opts.recordId}&page=1&page_size=100`
    ),
    'bare',
    `${opts.label} 台账列表（record_id=${opts.recordId}）`
  );
  const hit = rows.find(r => Number(r.id) === opts.defectId);
  if (!hit) {
    throw new Error(
      `${opts.label}：台账列表中未找到 id=${opts.defectId} 的行，` +
        `实际 id 集=${rows.map(r => r.id).join(',') || '(空)'}`
    );
  }
  return hit;
}

test.describe('96 生产链深度契约：BOM 用量 / 排程确认 / 验布跨环维度 / 报废二级审批', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('96-01 BOM 用量到领料需求：计划量×单耗×(1+损耗率) 逐值精确，损耗率写入/回显同口径，越界与精度非法值点名原因', async ({
    page,
  }) => {
    const finished = await ownProduct(page, 'FG');
    const material = await ownProduct(page, 'MT');

    // 单耗 2.5、损耗率按 API 百分比口径送 4（=4%）
    const perUnit = '2.5';
    const scrapPercent = '4';
    const bom = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/boms', {
      product_id: finished.id,
      version: 1,
      is_default: true,
      remarks: `E2E-F96-BOM-${finished.code}`,
      items: [
        {
          material_id: material.id,
          quantity: perUnit,
          unit: material.unit,
          scrap_rate: scrapPercent,
          sort_order: 1,
        },
      ],
    });
    const bomObj = bom.bom as Record<string, unknown> | undefined;
    const items = pickListArray<Record<string, unknown>>(bom, 'items', 'BOM 创建回显明细');
    expect(items.length, 'BOM 明细应落 1 行').toBe(1);
    const bomId = requireId(bomObj ?? {}, 'BOM 主表 id');
    CLEANUP.push({ path: `/boms/${bomId}`, label: `bom#${bomId}` });
    expect(bomObj!.product_id, 'BOM 头产品锚定').toBe(finished.id);

    // 写后必回读：单耗逐值、明细物料锚定、损耗率回显必须与提交口径同为百分比（同一字段禁止两口径）
    const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/boms/${bomId}`);
    const detailItems = pickListArray<Record<string, unknown>>(
      { items: detail.items },
      'items',
      `BOM ${bomId} 详情明细`
    );
    expect(detailItems.length, '详情明细行数').toBe(1);
    expect(detailItems[0].material_id, '明细物料 id 回读').toBe(material.id);
    expectDecimalString(detailItems[0].quantity, 'BOM 单耗');
    expectDecEquals(
      detailItems[0].quantity,
      decLiteral(perUnit, '提交单耗'),
      '单耗必须逐值落库（领料量算错的源头）'
    );
    expectDecimalString(detailItems[0].scrap_rate, 'BOM 损耗率');
    expectDecEquals(
      detailItems[0].scrap_rate,
      decLiteral(scrapPercent, '提交损耗率'),
      '损耗率回显口径必须与提交口径一致（同为百分比数值，禁一侧比率一侧百分比）'
    );

    // 领料需求 = 计划量 × 单耗 × (1+损耗率)，按十进制手算后逐值比对
    const planQty = decLiteral('100', '计划产量');
    const expected = decMul(
      decMul(planQty, decLiteral(perUnit, '单耗')),
      decLiteral('1.04', '1+损耗率')
    );
    const reqBody = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/boms/${bomId}/requirements`,
      { quantity: '100' }
    );
    const requirements = reqBody.requirements;
    if (!Array.isArray(requirements)) {
      throw new Error(
        `领料需求端点出参缺 requirements 数组，实际键=${Object.keys(reqBody).join(',')}`
      );
    }
    expect(requirements.length, '需求行数应等于叶子物料数').toBe(1);
    const reqRow = requirements[0] as Record<string, unknown>;
    expect(reqRow.product_id, '需求行物料 id').toBe(material.id);
    expect(reqRow.unit, '需求行单位取明细单位（口径不得由测试自造）').toBe(material.unit);
    expectDecimalString(reqRow.required_quantity, '需求数量');
    expectDecEquals(
      reqRow.required_quantity,
      expected,
      `计划量 100 × 单耗 ${perUnit} × (1+${scrapPercent}%) 的领料需求必须逐值命中`
    );

    // 非法损耗率：越界与超精度都必须 400 + 点名提交字段（可外显站点逐字符锁文案）
    const outOfRange = await apiCallExpectFail(page, 'POST', '/boms', {
      product_id: finished.id,
      items: [{ material_id: material.id, quantity: perUnit, scrap_rate: 150 }],
    });
    expect(outOfRange.status, `损耗率越界应 400，实际=${outOfRange.status}`).toBe(400);
    expect(failureCode(outOfRange), '损耗率越界机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(outOfRange.message, '损耗率越界可外显文案').toBe(
      'BOM 损耗率（百分比数值）必须在 0–100 范围内，实际: 150'
    );
    const tooPrecise = await apiCallExpectFail(page, 'POST', '/boms', {
      product_id: finished.id,
      items: [{ material_id: material.id, quantity: perUnit, scrap_rate: 1.234 }],
    });
    expect(tooPrecise.status, `损耗率超精度应 400，实际=${tooPrecise.status}`).toBe(400);
    expect(failureCode(tooPrecise), '损耗率超精度机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(tooPrecise.message, '损耗率超精度可外显文案').toBe(
      'BOM 损耗率最多保留两位小数（0.01% 粒度），实际: 1.234'
    );

    // 空明细不是"参数缺省"而是契约违约：必须被拒而不是建出无明细的 BOM
    const emptyItems = await apiCallExpectFail(page, 'POST', '/boms', {
      product_id: finished.id,
      items: [],
    });
    expect(emptyItems.status, `空明细建 BOM 应 400，实际=${emptyItems.status}`).toBe(400);
    expect(failureCode(emptyItems), '空明细机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(emptyItems.message, '空明细可外显文案').toBe('BOM明细不能为空');
  });

  test('96-02 排程结果确认闭环：未确认时工单原地不动，确认后状态与工单推进逐字段落库，重复确认被状态门拒且无漂移', async ({
    page,
  }) => {
    const prod = await ownProduct(page, 'WO');
    const wcCode = genCode('E96WC');
    const wc = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/production/capacity/work-centers',
      {
        code: wcCode,
        name: `E2E排程工位${wcCode}`,
        work_center_type: 'dyeing',
        daily_capacity: '999999.00',
        capacity_unit: '米',
        status: 'ACTIVE',
      }
    );
    const wcId = requireId(wc, '建工作中心');
    CLEANUP.push({ path: `/production/capacity/work-centers/${wcId}`, label: `work_center#${wcId}` });
    expect(wc.code, '工作中心编码回显').toBe(wcCode);
    expect(wc.status, '工作中心状态词值').toBe('ACTIVE');

    // 工单号由后端流水自动取号（PO{YYYYMMDD}{3位流水}），创建入参不提供注入口：用例只回读跟随
    const order = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/production/production-orders/orders',
      {
        product_id: prod.id,
        planned_quantity: '80',
        priority: 1,
        work_center_id: wcId,
      }
    );
    const orderId = requireId(order, '建生产工单');
    CLEANUP.push({
      path: `/production/production-orders/orders/${orderId}`,
      label: `production_order#${orderId}`,
    });
    const orderBefore = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/production-orders/orders/${orderId}`
    );
    // 工单号回读跟随后端号：必须是 PO{YYYYMMDD}{3位流水} 形态且建单回显与详情读回同源
    const backendOrderNo = String(orderBefore.order_no ?? '');
    expect(
      /^PO\d{11}$/.test(backendOrderNo),
      `工单号应由后端流水取号（PO+8位日期+3位流水），实际=${backendOrderNo}`
    ).toBe(true);
    expect(order.order_no, '建单回显工单号必须与详情读回同源').toBe(backendOrderNo);
    expect(orderBefore.status, '新建工单初始态（大写词表）').toBe('DRAFT');

    const scheduledDate = new Date(Date.now() + 86400000).toISOString().slice(0, 10);
    const run = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/scheduling/auto-schedule', {
      start_date: scheduledDate,
      work_center_ids: [wcId],
      strategy: 'priority',
    });
    // 后端权威键集：确认排程结果所需的 id 必须由响应给出（前端接口按另一套键名取值即恒空，
    // 是"排程结果永不确认、工单恒待排程"这条断链的起点）
    for (const key of [
      'id',
      'batch_no',
      'total_orders',
      'scheduled_orders',
      'unscheduled_orders',
      'conflicts',
      'gantt_data',
      'schedule_details',
    ]) {
      expect(
        Object.prototype.hasOwnProperty.call(run, key),
        `排程响应必须含后端权威键 "${key}"，实际键=${Object.keys(run).join(',')}`
      ).toBe(true);
    }
    const resultId = requireId(run, '排程结果 id');
    const batchNo = String(run.batch_no ?? '');
    if (!batchNo) throw new Error('排程响应 batch_no 为空，批次锚点缺失');

    const detailsRaw = run.schedule_details;
    const details = Array.isArray(detailsRaw)
      ? (detailsRaw as Array<Record<string, unknown>>)
      : [];
    const mine = details.find(d => Number(d.order_id) === orderId);
    expect(
      mine,
      `排程明细必须包含本例工单 ${orderId}（工作中心已指定、产能充足）；` +
        `实际明细工单集=${details.map(d => d.order_id).join(',') || '(空)'}`
    ).toBeTruthy();
    expect(Number(mine!.work_center_id), '排程明细分配的工作中心必须是本例自建工位').toBe(wcId);

    // 未确认：排程结果必须停 DRAFT，工单不得被推进（断"下游取不到排程结果"）
    const resultDraft = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/scheduling/results/${resultId}`
    );
    expect(resultDraft.status, '排程结果未确认时的状态词值（大写词表）').toBe('DRAFT');
    expect(resultDraft.batch_no, '排程结果批次号与响应同源').toBe(batchNo);
    const orderStillDraft = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/production-orders/orders/${orderId}`
    );
    expect(orderStillDraft.status, '排程结果未确认时工单状态不得漂移').toBe('DRAFT');
    expect(
      orderStillDraft.planned_start_date ?? null,
      '排程结果未确认时工单计划开始日不得被写入'
    ).toBeNull();

    // 确认：结果转 CONFIRMED，工单转 SCHEDULED 且日期/工位逐字段落库
    const confirmed = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/scheduling/results/${resultId}/confirm`
    );
    expect(confirmed.status, '确认后端权威状态').toBe('CONFIRMED');
    const resultAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/scheduling/results/${resultId}`
    );
    expect(resultAfter.status, '确认后排程结果状态读回').toBe('CONFIRMED');
    const orderAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/production-orders/orders/${orderId}`
    );
    expect(orderAfter.status, '确认后排程必须把工单推进到已排产（大写词表）').toBe('SCHEDULED');
    expect(
      String(orderAfter.planned_start_date ?? ''),
      '工单计划开始日必须落排程明细的开始日'
    ).toBe(String(mine!.start_date ?? mine!.planned_start ?? ''));
    expect(Number(orderAfter.work_center_id), '工单工作中心必须落排程分配的工位').toBe(wcId);

    // 重复确认被状态门拒且无漂移
    const dup = await apiCallExpectFail(page, 'POST', `/scheduling/results/${resultId}/confirm`);
    expect(dup.status, `重复确认排程结果应 400，实际=${dup.status}`).toBe(400);
    expect(failureCode(dup), '重复确认机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const resultNoDrift = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/scheduling/results/${resultId}`
    );
    expect(resultNoDrift.status, '被拒后排程结果状态不得二次漂移').toBe('CONFIRMED');

    const gone = await apiCallExpectFail(page, 'GET', '/scheduling/results/2147483647');
    expect(gone.status, '不存在的排程结果应 404').toBe(404);
    expect(failureCode(gone), '不存在机器码').toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('96-03 缸号/色号/批次/匹号跨环一致：染色档案→验布记录→打卷实物匹逐字符同源，米/公斤/匹三口径不得混用', async ({
    page,
  }) => {
    const prod = await ownProduct(page, 'DY');
    const wh = await pickDyeableWarehouse(page);
    const colorNo = `E96C${genCode('C')}`;
    const dyeLotNo = genDyeLotNo();
    const seedPieces = 2;
    const lengthPerPiece = 100;

    // 真实委外染色回仓链建档（缸号档案与染色匹的唯一生产写入方）
    const seeded = await seedDyedPieceChain(page, {
      productId: prod.id,
      warehouseId: wh.id,
      colorNo,
      dyeLotNo,
      pieceCount: seedPieces,
      lengthPerPieceMeters: lengthPerPiece,
      context: 'E96',
    });
    expect(seeded.length, '前置：委外回仓必须真实生成 2 匹染色匹').toBeGreaterThanOrEqual(seedPieces);

    // 验布记录：缸号/色号从染色档案带入，落库必须逐字符不变（跨环第一道维度校验）
    const insp = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/production/fabric-inspections', {
      dye_lot_no: dyeLotNo,
      color_no: colorNo,
      product_id: prod.id,
      product_name: prod.name,
      inspection_date: todayStr(),
      scoring_system: 'four_point',
      fabric_width_inches: '62',
      remarks: `E2E-F96-${dyeLotNo}`,
    });
    const inspId = requireId(insp, '建造验布记录');
    CLEANUP.push({ path: `/production/fabric-inspections/${inspId}`, label: `fabric_inspection#${inspId}` });
    const inspHead = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/fabric-inspections/${inspId}`
    );
    expect(inspHead.dye_lot_no, '验布记录缸号必须与染色档案逐字符相同').toBe(dyeLotNo);
    expect(inspHead.color_no, '验布记录色号必须与染色档案逐字符相同').toBe(colorNo);
    expect(inspHead.status, '验布初始态（小写词表）').toBe('pending');
    expect(inspHead.inspection_no, '验布单号必须由后端生成且非空').toBeTruthy();

    // 状态门：未开始验布直接评级必须被拒，且评级不产生任何写入
    const gradeTooEarly = await apiCallExpectFail(page, 'POST', `/production/fabric-inspections/${inspId}/grade`, {
      inspected_yards: '100',
      qualification_rate: '97',
    });
    expect(gradeTooEarly.status, `pending 直接评级应 400，实际=${gradeTooEarly.status}`).toBe(400);
    expect(failureCode(gradeTooEarly), 'pending 直接评级机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const stillPending = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/fabric-inspections/${inspId}`
    );
    expect(stillPending.status, '被拒后验布状态不得半推进').toBe('pending');
    expectDecEquals(stillPending.inspected_yards, { v: 0n, s: 0 }, '被拒后受检码数不得被写入');

    await apiCall(page, 'POST', `/production/fabric-inspections/${inspId}/start`);
    const inspecting = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/fabric-inspections/${inspId}`
    );
    expect(inspecting.status, '开始后验布态').toBe('inspecting');

    await apiCall(page, 'POST', `/production/fabric-inspections/${inspId}/grade`, {
      inspected_yards: '100',
      qualification_rate: '97',
    });
    const graded = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/fabric-inspections/${inspId}`
    );
    expect(graded.status, '评级后验布态').toBe('graded');
    // 无疵点 → 每百平方码分数 0 → 四分制首级；合格率 97 ≥ 95 → A 级
    expect(graded.grade, '四分制等级词值（小写）').toBe('first');
    expect(graded.abc_grade, 'A/B/C 分级词值').toBe('A');
    expectDecEquals(graded.points_per_100_sq_yards, { v: 0n, s: 0 }, '无疵点时每百平方码分数为 0');

    // 打卷缺实测三项：必须逐条点名缺哪个字段（成品布标签取值来源，禁止留 NULL 做"无据标签"）
    const rollNoWeight = await apiCallExpectFail(
      page,
      'POST',
      `/production/fabric-inspections/${inspId}/roll`,
      { warehouse_id: wh.id, roll_length: '120.5' }
    );
    expect(rollNoWeight.status, `缺重量打卷应 400，实际=${rollNoWeight.status}`).toBe(400);
    expect(failureCode(rollNoWeight), '缺重量打卷机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    // 后端一次点名全部缺项（分号拼接）：判据只钉本次提交确实缺失的重量子句，不锁整串拼装顺序
    expect(
      String(rollNoWeight.message ?? ''),
      '缺重量打卷可外显文案必须点名重量(kg)缺项'
    ).toContain('打卷入库必须填写重量(kg)，成品布标签重量取该实测值');
    const rollZeroLength = await apiCallExpectFail(
      page,
      'POST',
      `/production/fabric-inspections/${inspId}/roll`,
      {
        warehouse_id: wh.id,
        roll_length: '0',
        roll_weight: '30.25',
        roll_width: '160',
        roll_gram_weight: '180',
      }
    );
    expect(rollZeroLength.status, `零长度打卷应 400，实际=${rollZeroLength.status}`).toBe(400);
    expect(failureCode(rollZeroLength), '零长度打卷机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    const rollLength = '120.5';
    const rollWeight = '30.25';
    await apiCall(page, 'POST', `/production/fabric-inspections/${inspId}/roll`, {
      warehouse_id: wh.id,
      roll_length: rollLength,
      roll_weight: rollWeight,
      roll_width: '160',
      roll_gram_weight: '180',
    });
    const rolled = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/fabric-inspections/${inspId}`
    );
    expect(rolled.status, '打卷后验布态').toBe('rolled');
    // 三口径各归各位：匹数是计数、米数与公斤数是两条独立实测列，不得互相顶替
    expect(Number(rolled.total_rolls), '打卷计数（匹数口径）').toBe(1);
    expectDecEquals(rolled.total_roll_length, decLiteral(rollLength, '提交米数'), '累计米数=本次打卷米数');
    expectDecEquals(rolled.total_roll_weight, decLiteral(rollWeight, '提交公斤数'), '累计公斤数=本次打卷公斤数');

    const pieces = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/inventory/pieces?product_id=${prod.id}&warehouse_id=${wh.id}&dye_lot_no=${encodeURIComponent(
          dyeLotNo
        )}&page=1&page_size=50`
      ),
      'items',
      `缸号 ${dyeLotNo} 的实物匹`
    );
    const rolledPiece = pieces.find(p => String(p.inspection_id ?? '') === String(inspId));
    expect(
      rolledPiece,
      `打卷必须生成一张回指本验布单（inspection_id=${inspId}）的实物匹；` +
        `实际匹号与回指=${pieces.map(p => `${p.piece_no}/${p.inspection_id}`).join(',') || '(无)'}`
    ).toBeTruthy();
    expect(rolledPiece!.piece_type, '打卷产出的是染色匹（类型门）').toBe('dyed');
    expect(rolledPiece!.status, '新匹入可用态（大写词表）').toBe('AVAILABLE');
    expect(
      String(rolledPiece!.dye_lot_no),
      '实物匹缸号必须与验布记录/染色档案逐字符相同'
    ).toBe(dyeLotNo);
    expect(
      String(rolledPiece!.batch_no),
      '实物匹批次必须与缸号同源（写入方口径：批次=缸号），不得各写一套'
    ).toBe(dyeLotNo);
    expect(
      String(rolledPiece!.color_no),
      '实物匹色号必须与验布记录/染色档案逐字符相同'
    ).toBe(colorNo);
    const nextSeq = seedPieces + 1;
    expect(
      String(rolledPiece!.piece_no),
      '匹号必须由同缸序号递增生成（{缸号}-{三位序号}），跨环可追溯的锚点'
    ).toBe(`${dyeLotNo}-${String(nextSeq).padStart(3, '0')}`);
    expect(Number(rolledPiece!.piece_seq), '匹序号字段与匹号后缀一致').toBe(nextSeq);
    expectDecimalString(rolledPiece!.length, '实物匹米数');
    expectDecEquals(rolledPiece!.length, decLiteral(rollLength, '提交米数'), '实物匹米数=打卷录入米数');
    expectDecEquals(rolledPiece!.weight, decLiteral(rollWeight, '提交公斤数'), '实物匹公斤数=打卷录入公斤数');
    expect(
      decCmp(decFromRaw(rolledPiece!.length, '匹米数'), decFromRaw(rolledPiece!.weight, '匹公斤数')) !== 0,
      '米数与公斤数两列若相等即被同一数值顶替（三口径混用的判据）'
    ).toBe(true);

    // 一单一卷的状态门。评级只放行 inspecting、打卷只放行 graded，且每次打卷都把单据
    // 推进到 rolled，故已打卷的单再评级必须被拒；评级事件下游按 inspection_id 幂等，
    // 同单二次评级会被直接吞掉，第二卷不存在"回头再评级"这条路。
    const gradeAgain = await apiCallExpectFail(
      page,
      'POST',
      `/production/fabric-inspections/${inspId}/grade`,
      { inspected_yards: '50', qualification_rate: '97' }
    );
    expect(gradeAgain.status, `已打卷再评级应 400，实际=${gradeAgain.status}`).toBe(400);
    expect(failureCode(gradeAgain), '已打卷再评级机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const afterRejectedGrade = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/fabric-inspections/${inspId}`
    );
    expect(afterRejectedGrade.status, '被拒后仍是已打卷态').toBe('rolled');
    expect(Number(afterRejectedGrade.total_rolls), '被拒的评级不得推进打卷计数').toBe(1);

    // 同一缸的第二卷另起一张验布单（卷级评级与卷级疵点即后端口径）；跨卷只锁缸号内
    // 匹号继续递增且不得撞车
    const insp2 = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/production/fabric-inspections',
      {
        dye_lot_no: dyeLotNo,
        color_no: colorNo,
        product_id: prod.id,
        product_name: prod.name,
        inspection_date: todayStr(),
        scoring_system: 'four_point',
        fabric_width_inches: '62',
        remarks: `E2E-F96-2-${dyeLotNo}`,
      }
    );
    const insp2Id = requireId(insp2, '建造同缸第二张验布记录');
    CLEANUP.push({
      path: `/production/fabric-inspections/${insp2Id}`,
      label: `fabric_inspection#${insp2Id}`,
    });
    await apiCall(page, 'POST', `/production/fabric-inspections/${insp2Id}/start`);
    await apiCall(page, 'POST', `/production/fabric-inspections/${insp2Id}/grade`, {
      inspected_yards: '50',
      qualification_rate: '97',
    });
    await apiCall(page, 'POST', `/production/fabric-inspections/${insp2Id}/roll`, {
      warehouse_id: wh.id,
      roll_length: '80',
      roll_weight: '20',
      roll_width: '160',
      roll_gram_weight: '180',
    });
    const piecesAfter2 = await fetchAvailableDyedPieces(page, {
      productId: prod.id,
      warehouseId: wh.id,
      dyeLotNo,
      batchNo: dyeLotNo,
    });
    const nos = piecesAfter2.map(p => p.piece_no);
    expect(
      nos.includes(`${dyeLotNo}-${String(nextSeq + 1).padStart(3, '0')}`),
      `第二卷应生成匹号 ${dyeLotNo}-${String(nextSeq + 1).padStart(3, '0')}，实际匹号集=${nos.join(',')}`
    ).toBe(true);
    expect(new Set(nos).size, '同一缸号内匹号不得重复').toBe(nos.length);
    const rolled2 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/fabric-inspections/${insp2Id}`
    );
    expect(Number(rolled2.total_rolls), '第二张单本卷打卷计数').toBe(1);
    expectDecEquals(
      rolled2.total_roll_length,
      decLiteral('80', '第二卷米数'),
      '第二张单累计米数只含本卷'
    );
    expectDecEquals(
      rolled2.total_roll_weight,
      decLiteral('20', '第二卷公斤数'),
      '第二张单累计公斤数只含本卷'
    );
    const firstIntact = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/fabric-inspections/${inspId}`
    );
    expect(Number(firstIntact.total_rolls), '首单计数不得被同缸第二卷串改').toBe(1);
    expectDecEquals(
      firstIntact.total_roll_length,
      decLiteral(rollLength, '首单米数'),
      '首单累计米数仍等于其自身打卷米数'
    );
    expectDecEquals(
      firstIntact.total_roll_weight,
      decLiteral(rollWeight, '首单公斤数'),
      '首单累计公斤数仍等于其自身打卷公斤数'
    );
  });

  test('96-04 质检结论中文词表与报废二级审批门：越界结论点名允许清单、等级门与跳级/重复审批全部被拒且无漂移', async ({
    page,
  }) => {
    const ctx = getCtx();
    const prod = await ownProduct(page, 'QC');
    const batch = `E96QC${genCode('B')}`;

    const mkRecord = async (tag: string, result: string, rate: string) =>
      await apiCallRaw<Record<string, unknown>>(page, 'POST', '/production/quality-inspection/records', {
        inspection_no: genCode(`E96${tag}`),
        inspection_type: 'process',
        product_id: prod.id,
        batch_no: batch,
        inspection_date: todayStr(),
        total_qty: '100',
        inspected_qty: '100',
        qualified_qty: rate,
        unqualified_qty: '100',
        qualification_rate: rate,
        inspection_result: result,
      });

    // 中文结论原样落库（禁止被转写为英文码）
    const recC = await mkRecord('RC', '不合格', '60');
    const recCId = requireId(recC, '建 C 级质检记录');
    CLEANUP.push({
      path: `/production/quality-inspection/records/${recCId}`,
      label: `quality_inspection_record#${recCId}`,
    });
    const recCReadBack = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/quality-inspection/records/${recCId}`
    );
    expect(recCReadBack.inspection_result, '中文结论必须逐字符落库').toBe('不合格');
    expect(recCReadBack.grade, '合格率 60% 判级').toBe('C');
    expect(recCReadBack.batch_no, '批次维度回读').toBe(batch);

    // 英文化结论：必须被拒并点名允许清单（清单本体就是中文词表，两侧同锁）
    const enFail = await apiCallExpectFail(page, 'POST', '/production/quality-inspection/records', {
      inspection_no: genCode('E96EN'),
      inspection_type: 'process',
      product_id: prod.id,
      inspection_date: todayStr(),
      total_qty: '1',
      inspected_qty: '1',
      inspection_result: 'pass',
    });
    expect(enFail.status, `英文结论应 400，实际=${enFail.status}`).toBe(400);
    expect(failureCode(enFail), '英文结论机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(enFail.message, '越界结论必须点名允许清单').toBe(
      '检验结果 pass 不是合法取值，允许值：待检/合格/不合格'
    );
    const typeFail = await apiCallExpectFail(page, 'POST', '/production/quality-inspection/records', {
      inspection_no: genCode('E96TY'),
      inspection_type: 'incoming_check',
      product_id: prod.id,
      inspection_date: todayStr(),
      total_qty: '1',
      inspected_qty: '1',
      inspection_result: '待检',
    });
    expect(typeFail.status, `检验类型越界应 400，实际=${typeFail.status}`).toBe(400);
    expect(failureCode(typeFail), '检验类型越界机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(typeFail.message, '检验类型越界必须点名允许清单').toBe(
      '检验类型 incoming_check 不是合法取值，允许值：incoming/process/finished/outgoing/outsourcing_receipt'
    );

    // 等级门：C 级不允许降级销售，只允许返工或报废（词表 pass/fail 之外的处置码同样拒）
    const wrongMethod = await apiCallExpectFail(
      page,
      'POST',
      `/production/quality-inspection/defects/${recCId}/process`,
      {
        unqualified_qty: '40',
        unqualified_reason: `E2E-F96-${batch}`,
        handling_method: 'downgrade_sale',
      }
    );
    expect(wrongMethod.status, `C 级用降级销售应 400，实际=${wrongMethod.status}`).toBe(400);
    expect(failureCode(wrongMethod), '等级门机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    const recA = await mkRecord('RA', '合格', '100');
    const recAId = requireId(recA, '建 A 级质检记录');
    CLEANUP.push({
      path: `/production/quality-inspection/records/${recAId}`,
      label: `quality_inspection_record#${recAId}`,
    });
    const aGradeFail = await apiCallExpectFail(
      page,
      'POST',
      `/production/quality-inspection/defects/${recAId}/process`,
      {
        unqualified_qty: '1',
        unqualified_reason: `E2E-F96-A-${batch}`,
        handling_method: 'scrap',
      }
    );
    expect(aGradeFail.status, `A 级开报废应 400，实际=${aGradeFail.status}`).toBe(400);
    expect(failureCode(aGradeFail), 'A 级等级门机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 报废开单：进入二级审批初态 pending_fin
    const scrap = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/production/quality-inspection/defects/${recCId}/process`,
      {
        unqualified_qty: '40',
        unqualified_reason: `E2E-F96-SCRAP-${batch}`,
        handling_method: 'scrap',
      }
    );
    const scrapId = requireId(scrap, '报废开单');
    const scrapReadBack = await readDefectRow(page, {
      defectId: scrapId,
      recordId: recCId,
      label: '报废开单回读',
    });
    expect(scrapReadBack.scrap_approval_status, '报废开单进入财务待批态').toBe('pending_fin');
    expect(scrapReadBack.handling_status, '处置主状态待处理').toBe('pending');
    expect(scrapReadBack.handling_method, '处置方式词值').toBe('scrap');
    expect(scrapReadBack.batch_no, '报废台账批次与质检记录同源').toBe(batch);
    expect(scrapReadBack.unqualified_no, '报废单号由质检记录 id 派生').toBe(
      `UQ${String(recCId).padStart(8, '0')}`
    );

    // 重复开单守卫：同一质检记录上已有未处置完（pending）的不合格品单时不得再开一张。
    // 判据窗口必须落在台账行仍为 pending 期间——处置进终态后再开单属允许的追加处置，
    // 不是该守卫的拦截面
    const dupOpen = await apiCallExpectFail(
      page,
      'POST',
      `/production/quality-inspection/defects/${recCId}/process`,
      {
        unqualified_qty: '40',
        unqualified_reason: `E2E-F96-DUP-${batch}`,
        handling_method: 'scrap',
      }
    );
    expect(dupOpen.status, `pending 期重复开单应 400，实际=${dupOpen.status}`).toBe(400);
    expect(failureCode(dupOpen), 'pending 期重复开单机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 跳级：财务未批直接总经理审批必须被拒，且台账行状态不得动
    const gmTooEarly = await apiCallExpectFail(
      page,
      'POST',
      `/production/quality-inspection/defects/${scrapId}/scrap-approval/gm`,
      { approved: true, scrap_loss_amount: '100' }
    );
    expect(gmTooEarly.status, `跳级审批应 400，实际=${gmTooEarly.status}`).toBe(400);
    expect(failureCode(gmTooEarly), '跳级审批机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const noDrift1 = await readDefectRow(page, {
      defectId: scrapId,
      recordId: recCId,
      label: '跳级被拒后回读',
    });
    expect(noDrift1.scrap_approval_status, '跳级被拒后审批态不得推进').toBe('pending_fin');
    expect(noDrift1.approver_id_gm ?? null, '跳级被拒不得写入总经理审批人').toBeNull();

    // 一级（财务）通过 → 二级（总经理）
    const finOk = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/production/quality-inspection/defects/${scrapId}/scrap-approval/financial`,
      { approved: true }
    );
    expect(finOk.scrap_approval_status, '财务通过后进入总经理待批态').toBe('pending_gm');
    const finReadBack = await readDefectRow(page, {
      defectId: scrapId,
      recordId: recCId,
      label: '财务通过后回读',
    });
    expect(Number(finReadBack.approver_id_fin), '财务审批人取服务端会话身份并落库').toBeGreaterThan(0);
    expect(finReadBack.approved_at_fin, '财务审批时间必须落库').toBeTruthy();
    expect(finReadBack.scrap_approval_status, '读回一致').toBe('pending_gm');

    // 重复一级审批必须被拒（状态门），且已落的审批人不被第二次调用覆盖
    const finAgain = await apiCallExpectFail(
      page,
      'POST',
      `/production/quality-inspection/defects/${scrapId}/scrap-approval/financial`,
      { approved: true }
    );
    expect(finAgain.status, `重复财务审批应 400，实际=${finAgain.status}`).toBe(400);
    expect(failureCode(finAgain), '重复财务审批机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const finNoOverride = await readDefectRow(page, {
      defectId: scrapId,
      recordId: recCId,
      label: '重复财务审批后回读',
    });
    expect(
      finNoOverride.approver_id_fin,
      '重复审批被拒后财务审批人不得被改写'
    ).toBe(finReadBack.approver_id_fin);
    expect(finNoOverride.scrap_approval_status, '重复审批被拒后审批态不动').toBe('pending_gm');

    // 二级审批入参值门：拒绝态不得携带损失金额、金额不得为负（可外显点名）
    const lossWithReject = await apiCallExpectFail(
      page,
      'POST',
      `/production/quality-inspection/defects/${scrapId}/scrap-approval/gm`,
      { approved: false, scrap_loss_amount: '10' }
    );
    expect(lossWithReject.status, `拒绝态携带金额应 400，实际=${lossWithReject.status}`).toBe(400);
    expect(failureCode(lossWithReject), '拒绝态携带金额机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(lossWithReject.message, '拒绝态携带金额可外显文案').toBe(
      '仅同意的报废审批可携带报废损失金额'
    );
    const negativeLoss = await apiCallExpectFail(
      page,
      'POST',
      `/production/quality-inspection/defects/${scrapId}/scrap-approval/gm`,
      { approved: true, scrap_loss_amount: '-1' }
    );
    expect(negativeLoss.status, `负损失金额应 400，实际=${negativeLoss.status}`).toBe(400);
    expect(failureCode(negativeLoss), '负金额机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(negativeLoss.message, '负金额可外显文案').toBe('报废损失金额不得为负数');
    const stillPendingGm = await readDefectRow(page, {
      defectId: scrapId,
      recordId: recCId,
      label: '非法金额提交后回读',
    });
    expect(stillPendingGm.scrap_approval_status, '两次非法提交后仍等总经理审批').toBe('pending_gm');
    expect(stillPendingGm.scrap_loss_amount ?? null, '非法提交不得写入损失金额').toBeNull();

    const lossAmount = '1234.56';
    await apiCall(page, 'POST', `/production/quality-inspection/defects/${scrapId}/scrap-approval/gm`, {
      approved: true,
      scrap_loss_amount: lossAmount,
    });
    const gmDone = await readDefectRow(page, {
      defectId: scrapId,
      recordId: recCId,
      label: '总经理审批完成后回读',
    });
    expect(gmDone.scrap_approval_status, '二级通过后报废审批终态').toBe('approved');
    expect(gmDone.handling_status, '报废终态联动处置主状态').toBe('approved');
    expect(Number(gmDone.approver_id_gm), '总经理审批人落库').toBeGreaterThan(0);
    expectDecEquals(gmDone.scrap_loss_amount, decLiteral(lossAmount, '报废损失金额'), '报废损失金额落库');

    // 非报废处理方式不得进报废审批流
    const recC2 = await mkRecord('RB', '不合格', '70');
    const recC2Id = requireId(recC2, '建第二张 C 级记录');
    CLEANUP.push({
      path: `/production/quality-inspection/records/${recC2Id}`,
      label: `quality_inspection_record#${recC2Id}`,
    });
    const rework = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/production/quality-inspection/defects/${recC2Id}/process`,
      {
        unqualified_qty: '30',
        unqualified_reason: `E2E-F96-REWORK-${batch}`,
        handling_method: 'rework',
      }
    );
    const reworkId = requireId(rework, '返工开单');
    expect(rework.scrap_approval_status, '非报废处理方式不进审批流').toBe('not_required');
    const reworkFinFail = await apiCallExpectFail(
      page,
      'POST',
      `/production/quality-inspection/defects/${reworkId}/scrap-approval/financial`,
      { approved: true }
    );
    expect(reworkFinFail.status, `返工行走报废审批应 400，实际=${reworkFinFail.status}`).toBe(400);
    expect(failureCode(reworkFinFail), '返工走报废审批机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const reworkNoDrift = await readDefectRow(page, {
      defectId: reworkId,
      recordId: recC2Id,
      label: '返工行被拒后回读',
    });
    expect(reworkNoDrift.scrap_approval_status, '被拒后台账审批态不动').toBe('not_required');
    expect(ctx.supplierId, '前置：ctx 基础实体就绪').toBeTruthy();
  });
});
