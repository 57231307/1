// 财务域全流程契约级 E2E — 14 固定资产（台账/折旧/处置/盘点）+ 坏账/催收 + 出口退税
//
// 契约真值源：
//   CreateAssetRequestDto handlers/fixed_asset_handler.rs:54-67（location 键 → 落库 use_location，
//     fixed_asset_service.rs create: put_in_date→in_service_date 映射）；Model models/fixed_asset.rs:9-41；
//     初始 status=active（小写，master_data）
//   UpdateAssetDto handler:26-37（单层 Option：asset_name/asset_category/specification/use_location；
//     null=保持原值，语义同 10-02 注）；名称长度校验 #[validate(max=100)]
//   折旧：DepreciateRequest{period} handler:70-73；straight_line=(原值-残值)/(年限×12)
//     （service:221-238 注释）；重复期间 → AppError::validation("该资产此期间已计提折旧")
//     （service insert_depreciation_record:380-393，validation 族脱敏 ⇒ 只断机器码+回读无痕）；
//     状态门仅 active（:324 business_displayable 可外显）
//   处置：DisposalRequestDto handler:76-83；dispose→status=disposed（service:604）；
//     删除门仅 INACTIVE/disposed（service:788-792）；list_disposals 裸数组
//     （handler:243-254，fixed_asset_disposal::Model）
//   盘点：create_count_plan handler:643-687（出参 **非信封** {code:200,message,data:Model}）；
//     count_result 词值 consistent/surplus/shortage/damaged（service:1091 注释）；
//     plan DRAFT→COUNTING（首条录入，service:1121-1123）→COMPLETED（complete，:1259）；
//     shortage 资产置 INACTIVE（:1245）；重复 complete → validation（脱敏族，断码+回读）
//   坏账计提：POST /bad-debts/run-provision {period_year,period_month}；账龄桶 within_1y 费率 5%
//     （services/bad_debt_service.rs:89,99）；计提源=approval_status=APPROVED 且 unpaid>0
//     （:218-226）；confirm draft→confirmed、reverse confirmed→reversed
//     （models/status/finance.rs:151-163 小写）；非法态 InvalidState→AppError::business（脱敏⇒断码）
//   坏账核销：CreateWriteoffRequest models/bad_debt_dto.rs:41-48；二级审批词表
//     pending→finance_approved→approved（finance.rs:168-186）；超额 WriteoffAmountExceeds→business
//   催收：CreateTaskRequest models/collection_task_dto.rs:19-30；task_type ∈ phone/visit/email/letter
//     （collection_task_service.rs:409）；状态 pending/in_progress/completed/cancelled（:61-76）；
//     contact/cancel/reassign 门仅 pending/in_progress（:504/546/584）
//   退税：CreateCustomsDeclarationRequest services/export_refund_service.rs:29-42（单号唯一 business
//     脱敏⇒断码）；RefundCalculationInput/Result :62-90（免抵退四算式精确断言）；
//     generate 申报基数仅 status=verified 报关单（:250+ 注释）⇒ 新建 pending 不入基数为源码事实
// 诚实标注：处置/折旧端点 data 返回 String 消息而非模型，流转真值一律以 GET 回查为准；
//   /auth/me 提供当前用户 id（routes/auth.rs:31）供催收 assigned_to 引用，不硬编码。
import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  tryCleanup,
  APP_ERROR_CODES,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

const DESENSITIZED_CONSTANTS = ['请求参数验证失败', '业务处理失败'];

function expectKeyValue(
  obj: Record<string, unknown>,
  key: string,
  expected: unknown,
  label: string
): void {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺少后端真实键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  expect(obj[key], `${label}：键 "${key}" 期望=${JSON.stringify(expected)}`).toEqual(expected);
}

function expectDecimal(obj: Record<string, unknown>, key: string, expected: number, label: string) {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：缺金额键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n)) {
    throw new Error(`${label}：${key} 非数字，raw=${JSON.stringify(obj[key])}`);
  }
  expect(Math.abs(n - expected), `${label}：${key} 期望=${expected} 实际=${n}`).toBeLessThan(0.005);
}

function requireId(obj: Record<string, unknown>, label: string): number {
  const id = Number(obj.id);
  if (!Number.isFinite(id) || id <= 0) {
    throw new Error(`${label}：无有效 id，实际键=${Object.keys(obj).join(',')}`);
  }
  return id;
}

async function seedAsset(
  page: import('@playwright/test').Page,
  overrides: Record<string, unknown> = {}
): Promise<Record<string, unknown>> {
  const no = genCode('FA');
  const a = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/fixed-assets', {
    asset_no: no,
    asset_name: `E2E14资产${no}`,
    asset_category: '机器设备',
    specification: 'E2E-SPEC-100',
    location: '一车间',
    original_value: 120000,
    useful_life: 10,
    depreciation_method: 'straight_line',
    purchase_date: '2026-01-10',
    put_in_date: '2026-01-15',
    remark: 'E2E14 备注',
    ...overrides,
  });
  const id = requireId(a, '建资产');
  CLEANUP.push({ path: `/fixed-assets/${id}`, label: 'fixed_asset' });
  return a;
}

async function seedCustomer(page: import('@playwright/test').Page, tag: string): Promise<number> {
  const res = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
    customer_name: `E2E14${tag}客${genCode('C')}`,
  });
  if (!res.data?.id) throw new Error(`建客户失败：${JSON.stringify(res)}`);
  CLEANUP.push({ path: `/crm/customers/${res.data.id}`, label: 'customer' });
  return res.data.id;
}

/** 建应收单并审批（计提/核销源），due_date 可控逾期。 */
async function seedApprovedAr(
  page: import('@playwright/test').Page,
  customerId: number,
  amount: number,
  dueDate: string
): Promise<number> {
  const inv = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/invoices', {
    customer_id: customerId,
    invoice_date: '2025-01-01',
    due_date: dueDate,
    invoice_amount: amount,
    source_type: 'MANUAL',
  });
  const id = requireId(inv, '建应收单');
  await apiCall(page, 'POST', `/ar/invoices/${id}/approve`);
  return id;
}

test.describe('14 资产/坏账/催收/退税契约链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('14-01 资产台账全键建单→逐字段回读；PUT 单层 Option 三态与名称长度校验', async ({
    page,
  }) => {
    const a = await seedAsset(page);
    const id = requireId(a, '建资产');
    expectKeyValue(a, 'status', 'active', '资产初始');

    const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/fixed-assets/${id}`);
    expectKeyValue(detail, 'asset_name', a.asset_name, '资产详情');
    expectKeyValue(detail, 'asset_category', '机器设备', '资产详情');
    expectKeyValue(detail, 'specification', 'E2E-SPEC-100', '资产详情');
    // create 的 location 键映射到 use_location（fixed_asset_service.rs create：use_location: Set(req.location)）
    expectKeyValue(detail, 'use_location', '一车间', '资产详情(location 映射)');
    expectDecimal(detail, 'original_value', 120000, '资产详情');
    expectDecimal(detail, 'net_value', 120000, '初始净值=原值');
    expectDecimal(detail, 'accumulated_depreciation', 0, '初始累计折旧 0');
    expectKeyValue(detail, 'useful_life', 10, '资产详情');
    expectKeyValue(detail, 'depreciation_method', 'straight_line', '资产详情');
    expectKeyValue(detail, 'purchase_date', '2026-01-10', '资产详情');
    expectKeyValue(detail, 'in_service_date', '2026-01-15', '资产详情(put_in_date 映射)');

    const list = await apiCallRaw<unknown>(page, 'GET', `/fixed-assets?page=1&page_size=100`);
    if (!Array.isArray(list)) {
      throw new Error(
        `GET /fixed-assets data 应为裸数组（handler:88-105），实际=${JSON.stringify(list).slice(0, 150)}`
      );
    }
    const row = (list as Array<Record<string, unknown>>).find(r => Number(r.id) === id);
    if (!row) throw new Error('资产列表未含自建资产');
    expectDecimal(row, 'original_value', 120000, '列表行');

    // PUT 单层 Option：① name/use_location 覆盖 ② specification 显式 null=保持原值
    await apiCall(page, 'PUT', `/fixed-assets/${id}`, {
      asset_name: 'E2E14资产改名',
      use_location: '二车间',
      specification: null,
    });
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/fixed-assets/${id}`);
    expectKeyValue(d2, 'asset_name', 'E2E14资产改名', 'PUT① 覆盖');
    expectKeyValue(d2, 'use_location', '二车间', 'PUT① 覆盖');
    expectKeyValue(
      d2,
      'specification',
      'E2E-SPEC-100',
      'PUT② 单层 Option null=保持（handler:26-37）'
    );

    // 名称超长（>100）→ 400 VALIDATION_ERROR（#[validate(length(max=100))]）
    const fLong = await apiCallExpectFail(page, 'PUT', `/fixed-assets/${id}`, {
      asset_name: 'X'.repeat(101),
    });
    expect(fLong.status, '名称超长应 400').toBe(400);
    expect(failureCode(fLong), '名称超长机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
  });

  test('14-02 折旧链：straight_line 月额精确回读+记录列表；重复期间被拒且数值未二次变动', async ({
    page,
  }) => {
    const a = await seedAsset(page);
    const id = requireId(a, '建资产');
    // (120000-0)/(10*12)=1000/月
    const dep = await apiCall(page, 'POST', `/fixed-assets/${id}/depreciate`, {
      period: '2099-01',
    });
    expect(dep.code, '计提信封 200').toBe(200);
    const d = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/fixed-assets/${id}`);
    expectDecimal(d, 'accumulated_depreciation', 1000, '计提后累计折旧');
    expectDecimal(d, 'net_value', 119000, '计提后净值');

    const recs = await apiCallRaw<unknown>(page, 'GET', `/fixed-assets/${id}/depreciation-records`);
    if (!Array.isArray(recs)) {
      throw new Error(`折旧记录应为数组，实际=${JSON.stringify(recs).slice(0, 150)}`);
    }
    const hit = (recs as Array<Record<string, unknown>>).find(r => r.period === '2099-01');
    if (!hit) throw new Error('折旧记录未含 2099-01 期');
    expectDecimal(hit, 'depreciation_amount', 1000, '折旧记录金额');

    const fDup = await apiCallExpectFail(page, 'POST', `/fixed-assets/${id}/depreciate`, {
      period: '2099-01',
    });
    expect(fDup.status, '重复期间应 400').toBe(400);
    expect(failureCode(fDup), '重复期间机器码（validation 族，service:388）').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/fixed-assets/${id}`);
    expectDecimal(d2, 'accumulated_depreciation', 1000, '重复计提被拒后累计折旧未动');
  });

  test('14-03 处置与删除门：active 删被拒外显→dispose→disposed+处置记录→删 disposed→404', async ({
    page,
  }) => {
    const a = await seedAsset(page);
    const id = requireId(a, '建资产');

    const fDel = await apiCallExpectFail(page, 'DELETE', `/fixed-assets/${id}`);
    expect(fDel.status, 'active 删除应 400').toBe(400);
    expect(
      !DESENSITIZED_CONSTANTS.includes(String(fDel.message)),
      `删除门应外显「只能删除未使用或已处置…」类原因（service:788-792），实际=${JSON.stringify(fDel.message)}`
    ).toBe(true);

    const disp = await apiCall(page, 'POST', `/fixed-assets/${id}/dispose`, {
      disposal_type: '变卖',
      disposal_value: 30000,
      disposal_date: '2026-06-30',
      reason: 'E2E 处置',
      buyer_info: 'E2E 买家',
    });
    expect(disp.code, '处置信封 200').toBe(200);
    const d = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/fixed-assets/${id}`);
    expectKeyValue(d, 'status', 'disposed', '处置回查（finance.rs:129 小写）');
    expectKeyValue(d, 'disposal_date', '2026-06-30', '处置日期回读');

    const list = await apiCallRaw<unknown>(page, 'GET', '/fixed-assets/disposals');
    if (!Array.isArray(list)) {
      throw new Error(
        `disposals 应为裸数组（handler:243-254），实际=${JSON.stringify(list).slice(0, 150)}`
      );
    }
    const hit = (list as Array<Record<string, unknown>>).find(r => Number(r.asset_id) === id);
    if (!hit) throw new Error('处置记录列表未含自建处置');
    // 出参键=fixed_asset_disposal::Model 列名 disposal_amount（入参 disposal_value 落库即改名，
    // service:571 disposal_amount: Set(req.disposal_value)）
    expectDecimal(hit, 'disposal_amount', 30000, '处置记录价值');

    await apiCall(page, 'DELETE', `/fixed-assets/${id}`);
    const gone = await apiCallExpectFail(page, 'GET', `/fixed-assets/${id}`);
    expect(gone.status, '删除后 404').toBe(404);
    expect(failureCode(gone), '删除后机器码').toBe('NOT_FOUND');
  });

  test('14-04 盘点链：建计划→录入(consistent/shortage)→plan 进入 COUNTING→complete→资产 INACTIVE；重复 complete 被拒', async ({
    page,
  }) => {
    const a1 = await seedAsset(page);
    const a2 = await seedAsset(page);
    const id1 = requireId(a1, '资产1');
    const id2 = requireId(a2, '资产2');

    const planRes = await apiCall<{ id?: number }>(page, 'POST', '/fixed-assets/count-plans', {
      plan_name: `E2E14盘点${genCode('CP')}`,
      asset_category: '机器设备',
      count_date: '2026-07-01',
      notes: 'E2E 盘点',
    });
    const planId = requireId(planRes.data as Record<string, unknown>, '盘点计划');

    await apiCall(page, 'POST', '/fixed-assets/count-items', {
      count_id: planId,
      asset_id: id1,
      count_result: 'consistent',
    });
    await apiCall(page, 'POST', '/fixed-assets/count-items', {
      count_id: planId,
      asset_id: id2,
      count_result: 'shortage',
      remarks: 'E2E 盘亏',
    });

    await apiCall(page, 'PUT', `/fixed-assets/count-plans/${planId}/complete`);
    // shortage 资产被置 INACTIVE（service:1245）；另一资产保持 active
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/fixed-assets/${id2}`);
    expectKeyValue(d2, 'status', 'inactive', '盘亏资产 complete 后 INACTIVE');
    const d1 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/fixed-assets/${id1}`);
    expectKeyValue(d1, 'status', 'active', '一致资产不受影响');

    const items = await apiCallRaw<unknown>(
      page,
      'GET',
      `/fixed-assets/count-plans/${planId}/items`
    );
    if (!Array.isArray(items)) {
      throw new Error(`盘点明细应为数组，实际=${JSON.stringify(items).slice(0, 150)}`);
    }
    const hit = (items as Array<Record<string, unknown>>).find(r => Number(r.asset_id) === id2);
    if (!hit) throw new Error('盘点明细未含自建行');
    expectKeyValue(hit, 'count_result', 'shortage', '盘点结果词值回读');

    // 重复 complete：service:1214 门（COMPLETED 不可再完成，AppError::validation 脱敏族）
    const fRe = await apiCallExpectFail(
      page,
      'PUT',
      `/fixed-assets/count-plans/${planId}/complete`
    );
    expect(fRe.status, '重复 complete 应 4xx').toBe(400);
    expect(failureCode(fRe), '重复 complete 机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const d2b = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/fixed-assets/${id2}`);
    expectKeyValue(d2b, 'status', 'inactive', '重复 complete 被拒后资产状态未二次变动');
  });

  test('14-05 坏账计提链：run-provision 精确费率→confirm→reverse 全链回读；非法态被拒且未漂移', async ({
    page,
  }) => {
    const custId = await seedCustomer(page, '计提');
    // due_date 100 天前 → within_1y，费率 5%（bad_debt_service.rs:99）
    const invId = await seedApprovedAr(page, custId, 5000, '2025-06-01');
    const now = new Date();
    const y = now.getUTCFullYear();
    const m = now.getUTCMonth() + 1;

    const run = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/bad-debts/run-provision',
      {
        period_year: y,
        period_month: m,
      }
    );
    const created = run.created as Array<Record<string, unknown>>;
    expect(Array.isArray(created), 'run-provision.created 应为数组').toBe(true);
    expect(Number(run.created_count), 'created_count 应为数字').toBeGreaterThanOrEqual(1);
    const mine = created.find(
      r => Number(r.customer_id) === custId && r.aging_bucket === 'within_1y'
    );
    if (!mine)
      throw new Error(
        `自建客户 ${custId} 未生成 within_1y 计提：created=${JSON.stringify(created).slice(0, 300)}`
      );
    expectDecimal(mine, 'provision_rate', 0.05, 'within_1y 费率');
    expectDecimal(mine, 'provision_amount', 5000 * 0.05, '计提额=基数×5%');
    expectKeyValue(mine, 'status', 'draft', '计提初始（finance.rs:153）');
    const provId = requireId(mine, '计提记录');

    await apiCall(page, 'POST', `/bad-debts/${provId}/confirm`);
    const d1 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/bad-debts/${provId}`);
    expectKeyValue(d1, 'status', 'confirmed', 'confirm 回读');
    expect(d1.confirmed_at !== null, 'confirm 后 confirmed_at 应落库').toBe(true);

    await apiCall(page, 'POST', `/bad-debts/${provId}/reverse`, {
      reverse_voucher_id: null,
      remark: 'E2E 转回',
    });
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/bad-debts/${provId}`);
    expectKeyValue(d2, 'status', 'reversed', 'reverse 回读');
    expectKeyValue(d2, 'remark', 'E2E 转回', '转回备注回读');

    // reversed 再 confirm 被拒（InvalidState→business 脱敏⇒仅断码）且未漂移
    const f = await apiCallExpectFail(page, 'POST', `/bad-debts/${provId}/confirm`);
    expect(f.status, 'reversed confirm 应 400').toBe(400);
    expect(failureCode(f), '非法态机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const d3 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/bad-debts/${provId}`);
    expectKeyValue(d3, 'status', 'reversed', '非法 confirm 后未漂移');
    // 关联应收单不因计提链被改动（资金无痕）
    const inv = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/invoices/${invId}`);
    expectDecimal(inv, 'unpaid_amount', 5000, '计提不影响应收未付额');
  });

  test('14-06 坏账核销二级审批链：pending→finance_approved→approved 并核销应收；超额拒绝无痕', async ({
    page,
  }) => {
    const custId = await seedCustomer(page, '核销');
    const invId = await seedApprovedAr(page, custId, 3000, '2025-05-01');

    const wo = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/bad-debts/writeoffs', {
      customer_id: custId,
      ar_invoice_id: invId,
      writeoff_amount: 1000,
      reason: 'E2E 核销申请',
      remark: 'E2E 备注',
    });
    const woId = requireId(wo, '建核销申请');
    expectKeyValue(wo, 'approval_status', 'pending', '核销初始（finance.rs:170）');
    expectKeyValue(wo, 'reason', 'E2E 核销申请', '原因回读');

    await apiCall(page, 'POST', `/bad-debts/writeoffs/${woId}/finance-approve`, {
      comment: 'E2E 财务通过',
    });
    const d1 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/bad-debts/writeoffs/${woId}`
    );
    expectKeyValue(d1, 'approval_status', 'finance_approved', '一级审批回读');
    expectKeyValue(d1, 'finance_manager_comment', 'E2E 财务通过', '一级意见回读');

    await apiCall(page, 'POST', `/bad-debts/writeoffs/${woId}/general-manager-approve`, {
      comment: 'E2E 总经理通过',
    });
    const d2 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/bad-debts/writeoffs/${woId}`
    );
    expectKeyValue(d2, 'approval_status', 'approved', '二级审批终态回读');
    // 核销执行作用于应收：unpaid 减 1000
    const inv = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/invoices/${invId}`);
    expectDecimal(inv, 'unpaid_amount', 2000, '核销后应收 unpaid 精确减额');

    // 超额 → BadDebtError::WriteoffAmountExceeds → AppError::business（bad_debt_handler.rs:178，
    // error.rs:358 business→400）⇒ 锁具体 400+BUSINESS_ERROR，禁止"任意 4xx"
    const fEx = await apiCallExpectFail(page, 'POST', '/bad-debts/writeoffs', {
      customer_id: custId,
      ar_invoice_id: invId,
      writeoff_amount: 99999,
      reason: 'E2E 超额',
    });
    expect(fEx.status, `超额核销应 400（AppError::business），实际=${fEx.status}`).toBe(400);
    expect(failureCode(fEx), '超额机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const inv2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ar/invoices/${invId}`);
    expectDecimal(inv2, 'unpaid_amount', 2000, '超额被拒后应收未动');
  });

  test('14-07 催收任务链：建单→pending 回读→contact 完成→completed；completed 上再操作被拒；cancel 分支', async ({
    page,
  }) => {
    const custId = await seedCustomer(page, '催收');
    const invId = await seedApprovedAr(page, custId, 1500, '2025-07-01');
    const me = await apiCallRaw<Record<string, unknown>>(page, 'GET', '/auth/me');
    const userId = requireId(me, '当前用户');

    const due = new Date(Date.now() + 7 * 86400000).toISOString().slice(0, 10);
    const t = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/collection-tasks', {
      customer_id: custId,
      ar_invoice_id: invId,
      overdue_amount: 1500,
      overdue_days: 100,
      task_type: 'phone',
      due_date: due,
      assigned_to: userId,
      remark: 'E2E 催收',
    });
    const tid = requireId(t, '建催收任务');
    const d0 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/collection-tasks/${tid}`);
    expectKeyValue(d0, 'status', 'pending', '任务初始');
    expectKeyValue(d0, 'task_type', 'phone', '任务类型回读');
    expectKeyValue(d0, 'remark', 'E2E 催收', '备注回读');

    // 非法 task_type → 400（服务 :409）
    const fType = await apiCallExpectFail(page, 'POST', '/collection-tasks', {
      customer_id: custId,
      overdue_amount: 100,
      overdue_days: 10,
      task_type: 'FAX',
      due_date: due,
      assigned_to: userId,
    });
    expect(fType.status, '非法 task_type 应 400').toBe(400);
    expect(
      !DESENSITIZED_CONSTANTS.includes(String(fType.message)),
      `task_type 门应外显合法值清单原因（collection_task_service.rs:409-413），实际=${JSON.stringify(fType.message)}`
    ).toBe(true);

    await apiCall(page, 'POST', `/collection-tasks/${tid}/contact`, {
      contact_result: '已接通承诺付款',
      mark_completed: true,
      remark: 'E2E 联系',
    });
    const d1 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/collection-tasks/${tid}`);
    expectKeyValue(d1, 'status', 'completed', 'contact 完成后回读');

    // completed 上再 contact 被拒（门 pending/in_progress，collection_task_service.rs:503-508
    // InvalidState → AppError::business，collection_task_handler.rs:108 ⇒ 400+BUSINESS_ERROR）
    const fC = await apiCallExpectFail(page, 'POST', `/collection-tasks/${tid}/contact`, {
      contact_result: '再催',
    });
    expect(fC.status, `completed 上 contact 应 400，实际=${fC.status}`).toBe(400);
    expect(failureCode(fC), '非法态机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/collection-tasks/${tid}`);
    expectKeyValue(d2, 'status', 'completed', '非法操作后未漂移');

    // cancel 分支
    const t2 = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/collection-tasks', {
      customer_id: custId,
      overdue_amount: 200,
      overdue_days: 5,
      task_type: 'letter',
      due_date: due,
      assigned_to: userId,
    });
    const tid2 = requireId(t2, '建任务2');
    await apiCall(page, 'POST', `/collection-tasks/${tid2}/cancel`, { cancel_reason: 'E2E 取消' });
    const d3 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/collection-tasks/${tid2}`);
    expectKeyValue(d3, 'status', 'cancelled', 'cancel 回读');
  });

  test('14-08 出口退税链：报关单全键建单回读→免抵退四算式精确→申报表生成（未核验不入基数）与列表命中', async ({
    page,
  }) => {
    const custId = await seedCustomer(page, '退税');
    const no = genCode('CD');
    const cd = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/export-refunds/customs-declarations',
      {
        declaration_no: no,
        sales_order_id: null,
        customer_id: custId,
        product_id: null,
        export_date: '2026-03-10',
        destination_country: 'US',
        currency_code: 'USD',
        total_amount: 100000,
        exchange_rate: 7.2,
        customs_code: 'E2E-CUSTOMS',
        remarks: 'E2E 报关',
      }
    );
    expectKeyValue(cd, 'declaration_no', no, '报关响应');
    expectDecimal(cd, 'total_amount', 100000, '报关响应');
    expectKeyValue(cd, 'status', 'pending', '报关初始状态（service:139）');
    expectKeyValue(cd, 'customer_id', custId, '报关客户回读');

    // 单号重复 → business 脱敏 ⇒ 断码并回读原单未变
    const fDup = await apiCallExpectFail(page, 'POST', '/export-refunds/customs-declarations', {
      declaration_no: no,
      export_date: '2026-03-10',
      total_amount: 1,
      exchange_rate: 1,
    });
    expect(fDup.status, '重复报关单号应 400').toBe(400);
    expect(failureCode(fDup), '重复报关单号机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 免抵退纯计算（export_refund_service.rs:62-90）：100000×13%=13000，min(13000, 5000+1000)=6000，
    // 免抵=7000，结转=max(0,6000-13000)=0
    const calc = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/export-refunds/refund-calculation',
      {
        export_sales_amount: 100000,
        refund_rate: 0.13,
        input_vat_amount: 5000,
        carryforward_from_prev: 1000,
      }
    );
    expectDecimal(calc, 'refundable_vat_amount', 13000, '免抵退税额');
    expectDecimal(calc, 'actual_refund_amount', 6000, '应退税额=min');
    expectDecimal(calc, 'exempt_vat_amount', 7000, '免抵税额');
    expectDecimal(calc, 'carryforward_amount', 0, '结转下期待=0');

    // 申报表生成：自建报关单为 pending（未核验）⇒ 基数 0（服务注释「申报基数只计入 verified」源码事实）
    const decl = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/export-refunds/refund-declarations',
      {
        period_year: 2026,
        period_month: 3,
        refund_rate: 0.13,
        input_vat_amount: 5000,
        carryforward_from_prev: 1000,
      }
    );
    expect(decl.declaration_no !== undefined, '申报表应含 declaration_no').toBe(true);
    expectKeyValue(decl, 'period_year', 2026, '申报表期间');
    expectKeyValue(decl, 'documents_complete', false, '无可归属订单时单证不齐（判定源 :310-322）');

    const list = await apiCallRaw<unknown>(
      page,
      'GET',
      '/export-refunds/refund-declarations?period_year=2026&period_month=3'
    );
    if (!Array.isArray(list)) {
      throw new Error(
        `refund-declarations 列表应为数组，实际=${JSON.stringify(list).slice(0, 150)}`
      );
    }
    const hit = (list as Array<Record<string, unknown>>).find(
      r => String(r.declaration_no) === String(decl.declaration_no)
    );
    if (!hit) throw new Error('退税申报列表未含自建申报表');
    expectKeyValue(hit, 'period_month', 3, '列表行期间');
  });
});
