// 财务域全流程契约级 E2E — 13 预算（方案/明细/调整/执行预警）+ 资金（账户/划拨）
//
// 定位：与 finance/09（方案+明细+期间分解一致性负向）互补，本文件补其未覆盖的
// 「全字段回读 + 三态清空 + 审批链词值 + 资金余额真值」契约链（缺陷①②靶心）。
// 契约真值源：
//   方案 CreateBudgetPlanRequest handlers/budget_management_handler.rs:129-139（全键可选但
//     department_id 缺失 handler 即 4xx：:319-366 validation_displayable("预算编制请求缺少部门ID")）
//   方案 Model models/budget_plan.rs:12-42；状态词表小写 draft/rejected/approved/active
//     models/status/bpm_crm_contract.rs:19-32
//   方案门 services/budget_management_service.rs:485-495（approve 仅 DRAFT/REJECTED→approved）、
//     :523-554（execute 仅 approved→active，business_displayable 文案可外显）、:891-899
//     （reject 仅 DRAFT→rejected，business_displayable）
//   审批请求体 BudgetApproveRequest{approval_comment} handler:163-169；reject_plan 同体 handler:706
//   调整 AdjustBudgetRequest{item_id(=方案ID),adjust_amount,reason} models/dto/budget_dto.rs:5-9；
//     adjust_budget 建 PENDING（服务 :718-770，approval 词表大写
//     models/status/bpm_crm_contract.rs:9-16；budget_adjustment Model models/budget_adjustment.rs）；
//     approve_adjustment 应用金额到方案总额（:777-834）；reject_adjustment 不动金额（:841-873）；
//     门控文案均 business_displayable 可外显
//   执行明细 CreateBudgetExecutionRequest handler:163-172；Model models/budget_execution.rs:12-40
//   预警 GET /budgets/execution-warnings 出参裸 {code,data}（handler:1031-1045），
//     BudgetWarning 键 models/dto/budget_management_dto.rs:159-173（yellow≥80%）
//   明细 UpdateBudgetItemRequest.account_subject_id = Option<Option<i32>>
//     models/dto/budget_management_dto.rs:82 ——**真三态域**：显式 null=清空
//     （budget_management_service.rs:308-309 Set(inner)），省略键=保持；本文件用它打「清空后值仍在」靶心
//   资金账户 CreateFundAccountRequest handlers/fund_management_handler.rs:33-42；Model
//     models/fund_management.rs:9-27；create 初始三余额=0、status=active（服务 :113-116）
//   存/取/冻/解门 services/fund_management_service.rs:193/233/237/273/308（余额不足=validation 脱敏族，
//     仅断机器码+回读无痕）；删除门 :余额非零 business_displayable（:333 附近）
//   划拨 TransferFundRequest models/dto/fund_dto.rs:5-15：≤10000 自动 APPROVED 并执行、
//     >10000 PENDING 需 approve/reject（服务 :347-370、门 :436/:472）、>100000 需 confirm_large（:547）
// 诚实标注：budget plans **无 DELETE 端点**（routes/finance.rs:438-469）⇒ 方案残留按唯一 plan_no 可查，
//   列入后端缺口清单；/budgets/versions、create-with-mode、variance-analysis 属分析聚合面，
//   内容级依赖跨年度种子数据，本批不断言（不造假）。
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

async function seedDepartment(page: import('@playwright/test').Page, tag: string): Promise<number> {
  const code = genCode(`DPT${tag}`);
  const res = await apiCall<{ id?: number }>(page, 'POST', '/departments', {
    name: `E2E13部门${code}`,
    code,
  });
  if (!res.data?.id) throw new Error(`建部门失败：${JSON.stringify(res)}`);
  CLEANUP.push({ path: `/departments/${res.data.id}`, label: 'department' });
  return res.data.id;
}

/** 建预算方案（键集=CreateBudgetPlanRequest 全集）。plans 无 DELETE ⇒ 唯一号残留并告警。 */
async function createPlan(
  page: import('@playwright/test').Page,
  departmentId: number,
  totalAmount: number,
  overrides: Record<string, unknown> = {}
): Promise<Record<string, unknown>> {
  const no = genCode('PLAN');
  const plan = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/budgets/plans', {
    plan_no: no,
    plan_name: `E2E13方案${no}`,
    budget_year: 2032,
    budget_type: '年度预算',
    department_id: departmentId,
    total_amount: totalAmount,
    remark: 'E2E13-方案备注',
    ...overrides,
  });
  const id = requireId(plan, '建方案');
  console.warn(
    `[13] 预算方案 id=${id} plan_no=${no} 无法清理（后端无 DELETE /budgets/plans/{id}，见缺口清单）`
  );
  return plan;
}

async function seedAccount(page: import('@playwright/test').Page, tag: string, currency = 'CNY') {
  const no = genCode('ACCT');
  const acc = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/fund-management/accounts', {
    account_name: `E2E13${tag}账户`,
    account_no: no,
    account_type: '银行',
    bank_name: 'E2E 测试银行',
    currency,
    opened_date: '2026-01-15',
    remark: `E2E13-${tag}`,
  });
  const id = requireId(acc, '建资金账户');
  CLEANUP.push({ path: `/fund-management/accounts/${id}`, label: 'fund_account' });
  return { id, no, acc };
}

async function accountDetail(page: import('@playwright/test').Page, id: number) {
  return apiCallRaw<Record<string, unknown>>(page, 'GET', `/fund-management/accounts/${id}`);
}

test.describe('13 预算+资金全流程契约链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('13-01 方案全键建单→逐字段回读→approve→词值回查；重复 approve 被拒且文案外显', async ({
    page,
  }) => {
    const deptId = await seedDepartment(page, '批');
    const plan = await createPlan(page, deptId, 100000);
    const id = requireId(plan, '方案');

    const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/budgets/plans/${id}`);
    expectKeyValue(detail, 'plan_no', plan.plan_no, '方案详情');
    expectKeyValue(detail, 'plan_name', plan.plan_name, '方案详情');
    expectKeyValue(detail, 'budget_year', 2032, '方案详情');
    expectKeyValue(detail, 'budget_type', '年度预算', '方案详情');
    expectKeyValue(detail, 'department_id', deptId, '方案详情');
    expectDecimal(detail, 'total_amount', 100000, '方案详情');
    expectKeyValue(detail, 'remark', 'E2E13-方案备注', '方案详情');
    expectKeyValue(detail, 'status', 'draft', '方案初始（bpm_crm_contract.rs:21 小写）');

    await apiCall(page, 'POST', `/budgets/plans/${id}/approve`, { approval_comment: 'E2E 批准' });
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/budgets/plans/${id}`);
    expectKeyValue(d2, 'status', 'approved', 'approve 回查（:31 approved）');
    expect(d2.approved_by !== null, 'approve 后 approved_by 应落库').toBe(true);

    const fail = await apiCallExpectFail(page, 'POST', `/budgets/plans/${id}/approve`, {
      approval_comment: '重复',
    });
    expect(fail.status, '重复 approve 应 400').toBe(400);
    expect(failureCode(fail), '重复 approve 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      !DESENSITIZED_CONSTANTS.includes(String(fail.message)),
      `方案状态门文案 business_displayable 应外显（service:485-495），实际=${JSON.stringify(fail.message)}`
    ).toBe(true);
    const d3 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/budgets/plans/${id}`);
    expectKeyValue(d3, 'status', 'approved', '重复 approve 后未漂移');
  });

  test('13-02 方案门链：draft 上 execute 被拒外显→reject→rejected→再 approve 成功（门含 REJECTED）', async ({
    page,
  }) => {
    const deptId = await seedDepartment(page, '驳');
    const plan = await createPlan(page, deptId, 50000);
    const id = requireId(plan, '方案');

    // draft 未审批执行被拒（service:523-530 business_displayable "预算方案未审批，无法执行"）
    const fExec = await apiCallExpectFail(page, 'POST', `/budgets/plans/${id}/execute`, {
      plan_id: id,
      actual_amount: 100,
      expense_type: '办公费',
      expense_date: '2032-03-01',
    });
    expect(fExec.status, 'draft execute 应 400').toBe(400);
    // 门为 business_displayable("预算方案未审批，无法执行")（service.rs:523-530）⇒ 锁具体码+具体原因
    expect(failureCode(fExec), 'draft execute 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      String(fExec.message).includes('未审批') &&
        !DESENSITIZED_CONSTANTS.includes(String(fExec.message)),
      `execute 门应外显「未审批」具体原因（service.rs:523-530），实际=${JSON.stringify(fExec.message)}`
    ).toBe(true);
    let d = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/budgets/plans/${id}`);
    expectKeyValue(d, 'status', 'draft', '被拒后仍 draft');

    await apiCall(page, 'POST', `/budgets/plans/${id}/reject`, { approval_comment: 'E2E 驳回' });
    d = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/budgets/plans/${id}`);
    expectKeyValue(d, 'status', 'rejected', 'reject 回查（:27 rejected）');

    // REJECTED 可再 approve（门 DRAFT/REJECTED，service:485-486）
    await apiCall(page, 'POST', `/budgets/plans/${id}/approve`, {
      approval_comment: 'E2E 复议批准',
    });
    d = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/budgets/plans/${id}`);
    expectKeyValue(d, 'status', 'approved', 'rejected→approved 回查');

    // approved 再 reject 被拒（仅 DRAFT，service:891-899 business_displayable）
    const fRej = await apiCallExpectFail(page, 'POST', `/budgets/plans/${id}/reject`, {
      approval_comment: '违规',
    });
    expect(fRej.status, 'approved reject 应 400').toBe(400);
    expect(failureCode(fRej), 'approved reject 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/budgets/plans/${id}`);
    expectKeyValue(d2, 'status', 'approved', '被拒后未漂移');
  });

  test('13-03 预算明细三态域：account_subject_id 显式 null 必清空（缺陷①靶心）；省略键保持原值', async ({
    page,
  }) => {
    const deptId = await seedDepartment(page, '细');
    const plan = await createPlan(page, deptId, 80000);
    const planId = requireId(plan, '方案');
    const subjCode = genCode('BMAP');
    const subj = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/subjects', {
      code: subjCode,
      name: `E2E13映射科目${subjCode}`,
      level: 1,
      // balance_direction 写入方权威词表＝backend models/status/finance.rs 的 account_subject 常量
      balance_direction: 'debit',
      assist_customer: false,
      assist_supplier: false,
      assist_batch: false,
      assist_color_no: false,
      enable_dual_unit: false,
    });
    const subjId = requireId(subj, '建科目');
    CLEANUP.push({ path: `/subjects/${subjId}`, label: 'subject' });

    // 建明细走 POST /budgets/items（handler:92-106 CreateBudgetItemRequest 真实接受
    // account_subject_id；而 POST /budgets 的 CreateBudgetDto 无该键且 handler:580
    // 硬编码 account_subject_id: None ⇒ 经 /budgets 建单映射科目必丢，已列后端缺口清单）。
    // periods 为 serde 必填 Vec：传 [] ⇒ service.normalize_periods 自动生成 {year}-FY 全量期间。
    const item = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/budgets/items', {
      item_code: genCode('BITM'),
      item_name: 'E2E13-明细-原名',
      item_type: '费用',
      plan_id: planId,
      budget_year: 2032,
      planned_amount: 20000,
      periods: [],
      remark: 'E2E13-明细-备注',
      account_subject_id: subjId,
    });
    const itemId = requireId(item, '建明细');
    CLEANUP.push({ path: `/budgets/${itemId}`, label: 'budget_item' });

    const d0 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/budgets/${itemId}`);
    expectKeyValue(d0, 'item_name', 'E2E13-明细-原名', '明细详情');
    expectKeyValue(d0, 'item_type', '费用', '明细详情');
    expectKeyValue(d0, 'plan_id', planId, '明细详情');
    expectKeyValue(d0, 'budget_year', 2032, '明细详情');
    expectDecimal(d0, 'planned_amount', 20000, '明细详情');
    expectKeyValue(d0, 'account_subject_id', subjId, '明细建单映射科目回读（丢失即缺陷①判红）');

    // PUT 三态：① remark 覆盖 ② account_subject_id 显式 null=清空（三态域，
    //    models/dto/budget_management_dto.rs:82 + budget_management_service.rs:308-309）
    // ③ item_type 省略=保持
    await apiCall(page, 'PUT', `/budgets/items/${itemId}`, {
      remark: 'E2E13-明细-改后',
      account_subject_id: null,
    });
    const d1 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/budgets/items/${itemId}`);
    expectKeyValue(d1, 'remark', 'E2E13-明细-改后', 'PUT① 覆盖');
    expectKeyValue(
      d1,
      'account_subject_id',
      null,
      'PUT② 三态域显式 null 应清空；若仍为映射值=「清空后值仍在」缺陷①，判红并报 service.rs:308'
    );
    expectKeyValue(d1, 'item_type', '费用', 'PUT③ 省略键保持');

    // 再验证「省略键」与「null」语义分离：PUT planned_amount 覆盖（须同步提交期间分解——
    // service.rs:334-346 在 periods 缺省且改金额时校验 Σ现有期间==新金额，否则显式报错），
    // account_subject_id 省略 → 保持（已清空的 null）
    await apiCall(page, 'PUT', `/budgets/items/${itemId}`, {
      planned_amount: 21000,
      periods: [{ period: '2032-FY', planned_amount: 21000 }],
    });
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/budgets/items/${itemId}`);
    expectDecimal(d2, 'planned_amount', 21000, '再次 PUT 覆盖');
    expectKeyValue(d2, 'remark', 'E2E13-明细-改后', '省略键保持（remark）');
  });

  test('13-04 预算调整链：INCREASE approve 应用到方案总额并回读；DECREASE reject 后金额不动；重复审批被拒外显', async ({
    page,
  }) => {
    const deptId = await seedDepartment(page, '调');
    const plan = await createPlan(page, deptId, 60000);
    const planId = requireId(plan, '方案');
    await apiCall(page, 'POST', `/budgets/plans/${planId}/approve`, { approval_comment: 'E2E' });

    const adj = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/budgets/adjust', {
      item_id: planId,
      adjust_amount: 15000,
      reason: 'E2E 追加预算',
    });
    const adjId = requireId(adj, '建调整单');
    expectKeyValue(adj, 'budget_id', planId, '调整单指向方案');
    expectKeyValue(adj, 'adjustment_type', 'INCREASE', '正数→INCREASE（service:741-745）');
    expectDecimal(adj, 'budget_before', 60000, '调整前总额');
    expectDecimal(adj, 'budget_after', 75000, '调整后总额');
    expectKeyValue(adj, 'approval_status', 'PENDING', '调整初始（大写，bpm_crm_contract.rs:11）');

    await apiCall(page, 'POST', `/budgets/adjust/${adjId}/approve`);
    const dPlan = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/budgets/plans/${planId}`
    );
    expectDecimal(dPlan, 'total_amount', 75000, 'approve 后方案总额应用（service:814-819）');
    // 调整单**无 GET 详情端点**（routes/finance.rs:366-381 仅 approve/reject）⇒ 审批后状态经
    // 重复审批被拒的响应侧证不可得，改以「再次 adjust 后 reject 返回体」+方案总额回读闭环；
    // 详情缺失已列入后端缺口清单（重编辑无法回显调整单历史）。

    // 重复 approve 被拒且外显（service:796-800 business_displayable）
    const fRe = await apiCallExpectFail(page, 'POST', `/budgets/adjust/${adjId}/approve`);
    expect(fRe.status, '重复审批调整应 400').toBe(400);
    expect(failureCode(fRe), '重复审批调整机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      !DESENSITIZED_CONSTANTS.includes(String(fRe.message)),
      `调整门文案应外显，实际=${JSON.stringify(fRe.message)}`
    ).toBe(true);

    // DECREASE 调整被驳回 ⇒ 方案金额不动
    const adj2 = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/budgets/adjust', {
      item_id: planId,
      adjust_amount: -5000,
      reason: 'E2E 缩减被驳',
    });
    const adj2Id = requireId(adj2, '建调整单2');
    expectKeyValue(adj2, 'adjustment_type', 'DECREASE', '负数→DECREASE');
    expectKeyValue(adj2, 'approval_status', 'PENDING', '调整单初始 PENDING（大写词表）');
    const rejected = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/budgets/adjust/${adj2Id}/reject`
    );
    expectKeyValue(
      rejected,
      'approval_status',
      'REJECTED',
      'reject 响应回读（approve/reject 返回 model 即唯一可读途径）'
    );
    expectKeyValue(rejected, 'reason', 'E2E 缩减被驳', '调整原因回读（缺失即缺陷①）');
    const dPlan2 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/budgets/plans/${planId}`
    );
    expectDecimal(dPlan2, 'total_amount', 75000, '驳回后方案金额未动');
  });

  test('13-05 预算执行：execute_plan approved→active 并生成执行明细；executions 裸数组回读金额', async ({
    page,
  }) => {
    const deptId = await seedDepartment(page, '执');
    const plan = await createPlan(page, deptId, 30000);
    const planId = requireId(plan, '方案');
    await apiCall(page, 'POST', `/budgets/plans/${planId}/approve`, { approval_comment: 'E2E' });

    await apiCall(page, 'POST', `/budgets/plans/${planId}/execute`, {
      plan_id: planId,
      actual_amount: 27000,
      expense_type: '染整费',
      expense_date: '2032-06-01',
      remark: 'E2E 执行',
    });
    const dPlan = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/budgets/plans/${planId}`
    );
    expectKeyValue(dPlan, 'status', 'active', 'execute 后 active（service:523-554）');

    const execs = await apiCallRaw<unknown>(page, 'GET', `/budgets/plans/${planId}/executions`);
    if (!Array.isArray(execs)) {
      throw new Error(
        `executions 应为裸数组（handler:482-499），实际=${JSON.stringify(execs).slice(0, 150)}`
      );
    }
    const row = (execs as Array<Record<string, unknown>>).find(r => r.expense_type === '染整费');
    if (!row) throw new Error('执行明细未含自建行');
    expectDecimal(row, 'amount', 27000, '执行明细金额');
    expectKeyValue(row, 'plan_id', planId, '执行明细归属');

    // 独立执行明细端点（create_execution）
    const ex = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/budgets/plans/${planId}/executions`,
      {
        execution_type: '使用',
        amount: 1200.5,
        expense_type: '运输费',
        expense_date: '2032-06-15',
        remark: 'E2E 直建执行',
      }
    );
    expectDecimal(ex, 'amount', 1200.5, '直建执行明细响应');
    const execs2 = (await apiCallRaw<unknown>(
      page,
      'GET',
      `/budgets/plans/${planId}/executions`
    )) as Array<Record<string, unknown>>;
    const row2 = execs2.find(r => Number(r.id) === requireId(ex, '直建执行'));
    if (!row2) throw new Error('直建执行明细未在列表回读命中');
    expectKeyValue(row2, 'remark', 'E2E 直建执行', '直建执行回读');
  });

  test('13-06 预算执行预警内容级：90% 执行率方案出现在 yellow 预警且键完整', async ({ page }) => {
    const deptId = await seedDepartment(page, '警');
    const plan = await createPlan(page, deptId, 10000);
    const planId = requireId(plan, '方案');
    const planNo = String(plan.plan_no);
    await apiCall(page, 'POST', `/budgets/plans/${planId}/approve`, { approval_comment: 'E2E' });
    // 预警扫描口径（budget_management_service.rs:1409-1429）：issued=「下达」类执行明细求和，
    // issued 为 0 的方案直接 continue 不进预警。执行率=已执行/已下达，故必须先建一条
    // execution_type="下达"（与后端比较值逐字符相同）的 10000 明细建立额度，再建「使用」9000。
    await apiCall(page, 'POST', `/budgets/plans/${planId}/executions`, {
      execution_type: '下达',
      amount: 10000,
      expense_type: '预算下达',
      expense_date: '2032-07-01',
    });
    await apiCall(page, 'POST', `/budgets/plans/${planId}/executions`, {
      execution_type: '使用',
      amount: 9000,
      expense_type: '耗材',
      expense_date: '2032-07-01',
    });

    const resp = await apiCallRaw<unknown>(
      page,
      'GET',
      '/budgets/execution-warnings?budget_year=2032'
    );
    // 该端点直出 {code:200,data:[...]}（handler:1051-1066），apiCallRaw 已剥出 data ⇒ 应为主题数组
    if (!Array.isArray(resp)) {
      throw new Error(
        `execution-warnings data 应为数组（BudgetWarning[]），实际=${JSON.stringify(resp).slice(0, 200)}`
      );
    }
    const rows = resp as Array<Record<string, unknown>>;
    const hit = rows.find(r => String(r.plan_no) === planNo);
    if (!hit) throw new Error(`90% 执行率方案 ${planNo} 未进入预警列表（≥80% 应 yellow）`);
    expectKeyValue(hit, 'plan_id', planId, '预警行');
    expectDecimal(hit, 'issued_amount', 10000, '预警行 issued=「下达」明细和');
    expectDecimal(hit, 'executed_amount', 9000, '预警行');
    expectDecimal(hit, 'available_amount', 1000, '预警行 available=issued-executed');
    expectKeyValue(hit, 'warning_level', 'yellow', '预警级别（BudgetWarning.warning_level，:173）');
  });

  test('13-07 资金账户全键建单→逐字段回读→存/冻/解/取余额恒等→超额取款拒绝无痕→删除保护与清理', async ({
    page,
  }) => {
    const { id, acc } = await seedAccount(page, '主');
    expectKeyValue(acc, 'status', 'active', '账户初始（master_data::ACTIVE 小写）');
    expectDecimal(acc, 'balance', 0, '初始余额三键均 0');
    expectDecimal(acc, 'available_balance', 0, '初始可用 0');
    expectDecimal(acc, 'frozen_balance', 0, '初始冻结 0');
    expectKeyValue(acc, 'opened_date', '2026-01-15', '开户日期回读（缺陷①靶心）');
    expectKeyValue(acc, 'remark', 'E2E13-主', '备注回读');

    const d0 = await accountDetail(page, id);
    expectKeyValue(d0, 'account_no', acc.account_no, '账户详情');
    expectKeyValue(d0, 'bank_name', 'E2E 测试银行', '账户详情');
    expectKeyValue(d0, 'currency', 'CNY', '账户详情');

    await apiCall(page, 'POST', `/fund-management/accounts/${id}/deposit`, {
      amount: 1000,
      remark: 'E2E 存款',
    });
    let d = await accountDetail(page, id);
    expectDecimal(d, 'balance', 1000, '存款后');
    expectDecimal(d, 'available_balance', 1000, '存款后可用');

    await apiCall(page, 'POST', `/fund-management/accounts/${id}/freeze`, {
      amount: 400,
      reason: 'E2E 冻结',
    });
    d = await accountDetail(page, id);
    expectDecimal(d, 'available_balance', 600, '冻结后可用');
    expectDecimal(d, 'frozen_balance', 400, '冻结额');
    // 余额恒等：balance = available + frozen
    expect(
      Math.abs(Number(d.balance) - (Number(d.available_balance) + Number(d.frozen_balance))),
      `冻结后恒等破坏：${JSON.stringify(d)}`
    ).toBeLessThan(0.005);

    // 超额取款被拒（服务 validation 族——余额不足=脱敏 validation，锁族码不断文案；回读无痕）
    const fWd = await apiCallExpectFail(page, 'POST', `/fund-management/accounts/${id}/withdraw`, {
      amount: 700,
    });
    expect(fWd.status, '超额取款应 400').toBe(400);
    expect(failureCode(fWd), '超额取款机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    d = await accountDetail(page, id);
    expectDecimal(d, 'available_balance', 600, '超额取款被拒后余额未动');

    await apiCall(page, 'POST', `/fund-management/accounts/${id}/unfreeze`, {
      amount: 100,
      reason: 'E2E 解冻',
    });
    d = await accountDetail(page, id);
    expectDecimal(d, 'available_balance', 700, '解冻后可用');
    expectDecimal(d, 'frozen_balance', 300, '解冻后冻结');

    // 余额非零删除被拒且文案外显（business_displayable :333）
    const fDel = await apiCallExpectFail(page, 'DELETE', `/fund-management/accounts/${id}`);
    expect(fDel.status, '非零删除应 400').toBe(400);
    expect(failureCode(fDel), '非零删除机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      !DESENSITIZED_CONSTANTS.includes(String(fDel.message)),
      `删除守卫应外显「余额不为零」类具体原因，实际=${JSON.stringify(fDel.message)}`
    ).toBe(true);

    // 清空后删除 → GET 404
    await apiCall(page, 'POST', `/fund-management/accounts/${id}/unfreeze`, {
      amount: 300,
      reason: '清零',
    });
    await apiCall(page, 'POST', `/fund-management/accounts/${id}/withdraw`, {
      amount: 1000,
      remark: '清零',
    });
    await apiCall(page, 'DELETE', `/fund-management/accounts/${id}`);
    const gone = await apiCallExpectFail(page, 'GET', `/fund-management/accounts/${id}`);
    expect(gone.status, '删除后 404').toBe(404);
    expect(failureCode(gone), '删除后机器码').toBe('NOT_FOUND');
  });

  test('13-08 划拨：小额自动执行双边余额真值；大额 PENDING→approve/reject 分支；10万+未确认 400 无记录', async ({
    page,
  }) => {
    const a = await seedAccount(page, '拨A');
    const b = await seedAccount(page, '拨B');
    await apiCall(page, 'POST', `/fund-management/accounts/${a.id}/deposit`, { amount: 50000 });
    await apiCall(page, 'POST', `/fund-management/accounts/${b.id}/deposit`, { amount: 10000 });

    // 小额 ≤10000 自动 APPROVED 并执行（服务 :354-370）
    const small = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/fund-management/transfer',
      {
        from_account_id: a.id,
        to_account_id: b.id,
        amount: 5000,
        reason: 'E2E 小额划拨',
      }
    );
    expectKeyValue(small, 'status', 'APPROVED', '小额自动审批');
    let da = await accountDetail(page, a.id);
    let db = await accountDetail(page, b.id);
    expectDecimal(da, 'balance', 45000, '转出户余额真值');
    expectDecimal(db, 'balance', 15000, '转入户余额真值');

    // 大额 20000 → PENDING；先 reject：双边余额不动
    const big = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/fund-management/transfer',
      {
        from_account_id: a.id,
        to_account_id: b.id,
        amount: 20000,
        reason: 'E2E 大额拒绝样本',
      }
    );
    expectKeyValue(big, 'status', 'PENDING', '大额待审批');
    const bigId = requireId(big, '大额划拨');
    await apiCall(page, 'POST', `/fund-management/transfers/${bigId}/reject`);
    const dRej = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/fund-management/transfers/${bigId}`
    );
    expectKeyValue(dRej, 'status', 'REJECTED', 'reject 回查');
    da = await accountDetail(page, a.id);
    db = await accountDetail(page, b.id);
    expectDecimal(da, 'balance', 45000, '拒绝后转出户未动');
    expectDecimal(db, 'balance', 15000, '拒绝后转入户未动');

    // 重复审批被拒（门 :436-441 AppError::business）
    const fApr = await apiCallExpectFail(
      page,
      'POST',
      `/fund-management/transfers/${bigId}/approve`
    );
    expect(fApr.status, 'REJECTED 再审批应 400').toBe(400);
    expect(failureCode(fApr), 'REJECTED 再审批机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 另一笔大额 approve → 执行且双边真值
    const big2 = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/fund-management/transfer',
      {
        from_account_id: a.id,
        to_account_id: b.id,
        amount: 20000,
        reason: 'E2E 大额批准样本',
      }
    );
    const big2Id = requireId(big2, '大额划拨2');
    await apiCall(page, 'POST', `/fund-management/transfers/${big2Id}/approve`);
    const dApp = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/fund-management/transfers/${big2Id}`
    );
    expectKeyValue(dApp, 'status', 'APPROVED', 'approve 回查');
    da = await accountDetail(page, a.id);
    db = await accountDetail(page, b.id);
    expectDecimal(da, 'balance', 25000, '执行后转出户');
    expectDecimal(db, 'balance', 35000, '执行后转入户');

    // 10万+ 未 confirm_large → 4xx 且不生成记录（服务 :547-552）
    const fLarge = await apiCallExpectFail(page, 'POST', '/fund-management/transfer', {
      from_account_id: a.id,
      to_account_id: b.id,
      amount: 150000,
    });
    expect(fLarge.status, '未二次确认的 10万+ 划拨应 400').toBe(400);
    expect(failureCode(fLarge), '机器码 VALIDATION_ERROR').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
  });
});
