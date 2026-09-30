// 财务管理 E2E 套件 — 09 预算「方案头 + 明细 + 期间分解」端到端回读 + 预算一致性负向
//
// 任务 #942 缺口 6。方案B 重构后预算为「方案头 + 明细行 + 按月/季期间分解」两级三层：
//   09-01 端到端回读：建方案 → 建带期间分解的明细 → GET 明细回读 item.planned_amount 与
//         periods[] 数组（长度与 Σ期间 = 年度计划金额）→ GET 控制数据回读方案头 total_amount。
//   09-02 期间分解和 ≠ 年度计划 → create_item 内 normalize_periods 报 400 VALIDATION_ERROR（预算控制负向）。
//   09-03 Σ明细计划金额 > 方案总额 → approve_plan 的 validate_plan_items_consistency 报 400（"超"方案头被拒）。
// 说明：两处拒绝文案构造点均为脱敏 `AppError::validation`（文案含金额数字/方案 ID，
//   按 error.rs 安全边界不外显），出参 message 恒为常量"请求参数验证失败"，
//   services/budget_management_service.rs:124、:170；真实原因只进服务端日志（trace_id 可查）。
// 说明：花费超出"可用预算"的同步拦截（enforce_budget_available）挂在采购下单预算门
//   （services/po/order_ops/crud.rs），属采购域，不在财务域自控端点内；本套件覆盖财务域自有、
//   同步、可确定复现的预算一致性拒绝，全部断真实状态码 + 机器码 + 回读数值。
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
  CLEANUP.push({ path: `/budgets/plans/${id}`, label: 'budget_plan' });
  return id;
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
    const sum = detail.periods.reduce(
      (acc, p) => acc + num(p as Record<string, unknown>, 'planned_amount'),
      0
    );
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
  });

  test('09-03 明细合计超过方案总额 → 审批方案被拒（400 VALIDATION_ERROR）', async ({ page }) => {
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
    // 装配点 budget_management_service.rs:170 用 AppError::validation（脱敏变体）：
    // 文案含 Σ明细/方案总额数字与方案 ID，不外显原文，message 为固定常量（utils/messages.rs:41）。
    // 注意该站点先于 approve_plan 的状态门（:489 business_displayable）执行，故本用例命中 validation。
    expect(failureCode(fail), `机器码应为 VALIDATION_ERROR，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    expect(fail.message, '脱敏站点 message 应为固定验证失败常量').toBe('请求参数验证失败');
  });
});
