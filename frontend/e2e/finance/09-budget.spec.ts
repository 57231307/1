// 财务管理 E2E 套件 — 09 预算「方案头 + 明细 + 期间分解」端到端回读 + 预算一致性负向
//
// 缺口 6。方案B 重构后预算为「方案头 + 明细行 + 按月/季期间分解」两级三层
//   09-01 端到端回读：建方案 → 建带期间分解的明细 → GET 明细回读 item.planned_amount 与
//         periods[] 数组（长度与 Σ期间 = 年度计划金额）→ GET 控制数据回读方案头 total_amount。
//   09-02 期间分解和 ≠ 年度计划 → create_item 内 normalize_periods 报 400 VALIDATION_ERROR（预算控制负向）。
//   09-03 Σ明细计划金额 > 方案总额 → approve_plan 的 validate_plan_items_consistency 报 400（"超"方案头被拒）。
// 族别口径（两站点不同族，勿混）：
//   09-02 是**字段内容校验**（期间分解 Σ ≠ 年度计划）→ services/budget_management_service.rs:124
//     用脱敏 `AppError::validation`，出参 message 恒为常量"请求参数验证失败"，真实数字只进日志。
//   09-03 是**审批前置状态门**（唯一调用点 approve_plan）→ :176 用 `business_displayable`，
//     机器码 BUSINESS_ERROR、回显可外显规则文案；方案 ID/方案号/两侧金额是查询所得实体数据，只进日志。
// 说明：花费超出"可用预算"的同步拦截（enforce_budget_available）挂在采购下单预算门
//   （services/po/order_ops/crud.rs），属采购域，不在财务域自控端点内；本套件覆盖财务域自有、
//   同步、可确定复现的预算一致性拒绝，全部断真实状态码 + 机器码 + 回读数值。
// 收尾口径：预算方案族后端没有删除腿，本轮自建方案统一走已注册的驳回跃迁腿置为 rejected 终态，
//   并回读断状态字面值真实落库、重复驳回被状态门拒绝；方案行以唯一方案号残留，断言只按本轮 id 精确回读。
import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  tryCleanup,
  failureCode,
  APP_ERROR_CODES,
  genCode,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function num(obj: Record<string, unknown>, key: string): number {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`响应缺少键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n)) throw new Error(`键 "${key}" 非数字，raw=${JSON.stringify(obj[key])}`);
  return n;
}

async function seedDepartment(page: import('@playwright/test').Page): Promise<number> {
  const code = genCode('E2E-BDEPT');
  const res = await apiCall<{ id?: number }>(page, 'POST', '/departments', {
    name: `E2E 预算部门${code}`,
    code,
  });
  if (!res.data?.id) throw new Error(`建部门失败：${JSON.stringify(res)}`);
  CLEANUP.push({ path: `/departments/${res.data.id}`, label: 'department' });
  return res.data.id;
}

async function seedPlan(
  page: import('@playwright/test').Page,
  departmentId: number,
  totalAmount: number
): Promise<number> {
  const no = genCode('E2E-BPLAN');
  const res = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/budgets/plans', {
    plan_no: no,
    plan_name: `E2E 预算方案${no}`,
    budget_year: 2031,
    budget_type: '年度预算',
    department_id: departmentId,
    total_amount: totalAmount,
  });
  const id = Number(res.id);
  if (!id) throw new Error(`建方案失败：${JSON.stringify(res)}`);
  return id;
}

// 预算方案收尾终态化：调用后端已注册的方案驳回跃迁腿，把本轮自建方案从 draft 置为 rejected。
// 调用方：09-01、09-02、09-03 三个用例在各自断言全部通过后收尾调用。
// 入参：page 为登录态页面对象，planId 为本轮 seedPlan 返回的方案 id，expectedTotal 为本轮写入的方案总额。
// 传给谁：后端 reject_plan 处端点为方案驳回跃迁腿，请求体字段 approval_comment 按 BudgetApproveRequest 契约传入。
// 存什么：方案行状态由 draft 变为 rejected（词表小写，服务层仅草稿可驳回、驳回不改金额等业务字段），断言只按本轮 id 精确回读。
// 存哪里：方案行以 rejected 终态保留在预算方案表中，不依赖任何删除腿，残留行带唯一方案号不污染后续判定。
async function retirePlanViaReject(
  page: import('@playwright/test').Page,
  planId: number,
  expectedTotal: number
): Promise<void> {
  await apiCallRaw<Record<string, unknown>>(page, 'POST', `/budgets/plans/${planId}/reject`, {
    approval_comment: 'E2E 收尾驳回',
  });
  // 回读断真腿生效：状态字面值为 rejected 族（不是只断请求返回码），且驳回不得改动方案总额。
  const after = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/budgets/plans/${planId}`);
  expect(String(after.status), '驳回落库后方案状态应为 rejected 字面值').toBe('rejected');
  expect(num(after, 'total_amount'), '驳回不得改动方案总额').toBe(expectedTotal);
  // 离开待审侧的行为学证据：方案列表接口无状态筛选参数，改用重复驳回被状态门拒绝，
  // 证明该方案已不在仅草稿可驳回的前置白名单一侧。
  const fail = await apiCallExpectFail(page, 'POST', `/budgets/plans/${planId}/reject`, {
    approval_comment: 'E2E 对已驳回方案重复驳回',
  });
  expect(fail.status, `已驳回方案重复驳回应被状态门拒 400，实际=${fail.status}`).toBe(400);
  expect(failureCode(fail), `机器码应为 BUSINESS_ERROR，实际=${fail.code}`).toBe(
    APP_ERROR_CODES.BUSINESS_ERROR
  );
  expect(fail.message, '状态门应回显仅草稿可驳回的规则文案原文').toBe(
    '预算方案状态不允许驳回（仅草稿状态可驳回）'
  );
}

test.describe('09 预算端到端与一致性', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('09-01 方案头 + 明细 + 期间分解端到端回读（Σ期间 = 年度计划）', async ({ page }) => {
    const deptId = await seedDepartment(page);
    const planId = await seedPlan(page, deptId, 1000);

    // 建明细：年度计划 1000，按月分解 400 + 600。
    const item = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/budgets', {
      item_name: `E2E 预算明细${genCode('ITM')}`,
      plan_id: planId,
      budget_year: 2031,
      planned_amount: 1000,
      periods: [
        { period: '2031-01', planned_amount: 400 },
        { period: '2031-02', planned_amount: 600 },
      ],
    });
    const itemId = Number(item.id);
    if (!itemId) throw new Error(`建明细失败：${JSON.stringify(item)}`);
    CLEANUP.push({ path: `/budgets/${itemId}`, label: 'budget_item' });

    // 明细回读：plan_id 关联正确 + 年度计划金额真实落库。
    const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/budgets/${itemId}`);
    expect(num(detail, 'planned_amount'), '明细年度计划金额应为 1000').toBe(1000);
    expect(num(detail, 'plan_id'), '明细 plan_id 应指向方案').toBe(planId);

    // 期间分解回读：数组长度 2、Σ期间 = 1000。
    if (!Array.isArray(detail.periods)) {
      throw new Error(`明细详情缺少 periods 数组，实际键=${Object.keys(detail).join(',')}`);
    }
    expect(detail.periods.length, '期间分解应有 2 条').toBe(2);
    // 逐期间对齐：2031-01 应存 400、2031-02 应存 600（只断 Σ 会掩盖期间错位）。
    const byPeriod = new Map<string, number>();
    for (const p of detail.periods as Array<Record<string, unknown>>) {
      const key = String(p.period);
      if (byPeriod.has(key)) {
        throw new Error(`期间分解出现重复期间 ${key}`);
      }
      byPeriod.set(key, num(p, 'planned_amount'));
    }
    expect(byPeriod.get('2031-01'), '2031-01 期间计划金额应为 400').toBe(400);
    expect(byPeriod.get('2031-02'), '2031-02 期间计划金额应为 600').toBe(600);
    const sum = [...byPeriod.values()].reduce((acc, v) => acc + v, 0);
    expect(sum, 'Σ期间分解应等于年度计划金额').toBe(1000);
    // 会计口径恒等：Σ期间 == planned_amount。
    expect(Math.abs(sum - num(detail, 'planned_amount')), '期间分解恒等').toBeLessThan(0.01);

    // 控制数据回读：方案头 total_amount 真实出现在预算控制查询里。
    const control = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/budgets/control/${planId}/data`
    );
    expect(num(control, 'total_amount'), '控制数据应回读方案头总额 1000').toBe(1000);

    // 收尾：方案无删除腿，走已注册的驳回跃迁腿置终态并回读生效。
    await retirePlanViaReject(page, planId, 1000);
  });

  test('09-02 期间分解之和 ≠ 年度计划 → 建明细被拒（400 VALIDATION_ERROR）', async ({ page }) => {
    const deptId = await seedDepartment(page);
    const planId = await seedPlan(page, deptId, 1000);

    const fail = await apiCallExpectFail(page, 'POST', '/budgets', {
      item_name: `E2E 不平明细${genCode('ITM')}`,
      plan_id: planId,
      budget_year: 2031,
      planned_amount: 1000,
      // Σ期间 = 800 ≠ 1000
      periods: [
        { period: '2031-01', planned_amount: 300 },
        { period: '2031-02', planned_amount: 500 },
      ],
    });
    expect(fail.status, `期间不平易被拒 400，实际=${fail.status}`).toBe(400);
    // 装配点 budget_management_service.rs:124 用 AppError::validation（脱敏变体）：
    // 文案含 Σ期间/年度计划金额数字，不外显原文，message 为固定常量（utils/messages.rs:41）。
    expect(failureCode(fail), `机器码应为 VALIDATION_ERROR，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    expect(fail.message, '脱敏站点 message 应为固定验证失败常量').toBe('请求参数验证失败');

    // 收尾：本轮方案未被任何成功写路径改动过状态，仍为草稿，走驳回跃迁腿置终态并回读生效。
    await retirePlanViaReject(page, planId, 1000);
  });

  test('09-03 明细合计超过方案总额 → 审批方案被拒（400 BUSINESS_ERROR）', async ({ page }) => {
    const deptId = await seedDepartment(page);
    // 方案总额 1000，但只建一条计划 700 的明细 → Σ明细(700) ≠ 总额(1000)。
    const planId = await seedPlan(page, deptId, 1000);
    const item = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/budgets', {
      item_name: `E2E 短明细${genCode('ITM')}`,
      plan_id: planId,
      budget_year: 2031,
      planned_amount: 700,
      periods: [{ period: '2031-01', planned_amount: 700 }],
    });
    const itemId = Number(item.id);
    if (itemId) CLEANUP.push({ path: `/budgets/${itemId}`, label: 'budget_item' });

    const fail = await apiCallExpectFail(page, 'POST', `/budgets/plans/${planId}/approve`, {
      approval_comment: 'E2E 应被拒',
    });
    expect(fail.status, `明细不平方案审批应被拒 400，实际=${fail.status}`).toBe(400);
    // 装配点 budget_management_service.rs:165-179：Σ明细 ≠ 方案总额是 approve_plan 的
    // **审批前置状态门**（不是字段格式校验）⇒ 归 BUSINESS_ERROR 族、用 business_displayable
    // 回显可外显的规则文案；方案 ID/方案号/两侧金额是查询所得实体数据，只进服务端日志。
    expect(failureCode(fail), `机器码应为 BUSINESS_ERROR，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    // 状态门文案逐字符钉死（validate_plan_items_consistency 的 business_displayable 原文；
    // 方案 ID/方案号/两侧金额只进服务端日志，不在此文案内）。
    expect(fail.message, `状态门应回显规则文案原文，实际=${JSON.stringify(fail.message)}`).toBe(
      '预算明细合计与方案总额不一致，无法审批，请先调整明细金额使合计等于方案总额'
    );

    // 零变化回读：审批被拒后方案状态不得迁移（budget 词表小写 draft，见 models/status/bpm_crm_contract.rs）。
    const planAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/budgets/plans/${planId}`
    );
    expect(String(planAfter.status), '审批被拒后方案应仍为 draft').toBe('draft');
    expect(num(planAfter, 'total_amount'), '审批被拒不得改动方案总额').toBe(1000);

    // 收尾：审批被状态门拒后方案仍为草稿，驳回跃迁腿前置可达，置终态并回读生效。
    await retirePlanViaReject(page, planId, 1000);
  });
});
