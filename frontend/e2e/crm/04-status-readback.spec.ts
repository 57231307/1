// CRM 状态流转回读 E2E — 04 线索/商机状态变更 API 级回读
// 覆盖范围（补齐现有 spec 仅 toast 断言的缺口）：
//   - 建线索 → PUT status 变更 → GET 回读 lead_status 字段实际变
//   - 建商机 → PUT 修改 opportunity_stage → GET 回读 opportunity_stage 字段变
//   - 非法 lead_status → 400 VALIDATION_ERROR（取值域校验，displayable 外显真实原因；负例）
//   - 非法商机阶段流转 → 400 BUSINESS_ERROR（阶段流转门，脱敏出参「业务处理失败」；负例）
// 真实后端 + 回读，不依赖 UI toast。
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  tryCleanup,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 建一条指定状态的线索，返回 { id, leadNo } */
async function seedLead(
  page: import('@playwright/test').Page,
  leadStatus: string
): Promise<{ id: number; leadNo: string }> {
  const leadNo = genCode('E2E-RB');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/crm/leads', {
    lead_no: leadNo,
    lead_status: leadStatus,
    lead_source: 'WEBSITE',
    company_name: `E2E 回读测试 ${leadNo}`,
    contact_name: '测试联系人',
    mobile_phone: '13800138000',
    email: 'e2e-rb@test.com',
    priority: 'MEDIUM',
  });
  if (!created.data?.id) throw new Error(`建线索失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/crm/leads/${created.data.id}`, label: 'crm_lead' });
  return { id: created.data.id, leadNo };
}

/** 建一条指定阶段的商机，返回 { id, oppNo } */
async function seedOpportunity(
  page: import('@playwright/test').Page,
  stage: string
): Promise<{ id: number; oppNo: string }> {
  // 确保有可用客户
  const list = await apiCallRaw<{ items?: Array<{ id: number }> }>(
    page,
    'GET',
    '/crm/customers?page=1&page_size=1'
  );
  let customerId: number;
  if (list.items?.[0]?.id) {
    customerId = list.items[0].id;
  } else {
    const cust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `E2E 客户 ${genCode('C')}`,
    });
    if (!cust.data?.id) throw new Error('无法准备客户');
    customerId = cust.data.id;
  }

  const oppNo = genCode('E2E-OPP-RB');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/crm/opportunities', {
    opportunity_no: oppNo,
    opportunity_name: oppNo,
    customer_id: customerId,
    opportunity_stage: stage,
    estimated_amount: 50000,
    win_probability: 70,
    expected_close_date: '2026-12-31',
  });
  if (!created.data?.id) throw new Error(`建商机失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/crm/opportunities/${created.data.id}`, label: 'crm_opportunity' });
  return { id: created.data.id, oppNo };
}

test.describe('04 线索/商机状态流转回读', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('04-01 线索状态变更后 GET 回读 lead_status 实际变', async ({ page }) => {
    // 建 new 状态线索
    const { id } = await seedLead(page, 'new');

    // GET 确认初始状态为 new
    const before = await apiCallRaw<{ id: number; lead_status: string }>(
      page,
      'GET',
      `/crm/leads/${id}`
    );
    expect(before.lead_status).toBe('new');

    // PUT 变更状态为 contacted
    await apiCall(page, 'PUT', `/crm/leads/${id}/status`, { status: 'contacted' });

    // GET 回读验证 lead_status 字段实际变
    const after = await apiCallRaw<{ id: number; lead_status: string }>(
      page,
      'GET',
      `/crm/leads/${id}`
    );
    expect(
      after.lead_status,
      `状态变更后 lead_status 应为 contacted（实际=${after.lead_status}）`
    ).toBe('contacted');
  });

  test('04-02 线索完整生命周期状态变更均可回读', async ({ page }) => {
    const { id } = await seedLead(page, 'new');

    // new → contacted → qualified → converted 逐步推进
    const transitions = ['contacted', 'qualified', 'converted'];
    for (const targetStatus of transitions) {
      await apiCall(page, 'PUT', `/crm/leads/${id}/status`, { status: targetStatus });
      const current = await apiCallRaw<{ lead_status: string }>(page, 'GET', `/crm/leads/${id}`);
      expect(current.lead_status, `状态推进到 ${targetStatus} 后回读应为该值`).toBe(targetStatus);
    }
  });

  test('04-03 非法线索状态 PUT 返回 400 VALIDATION_ERROR', async ({ page }) => {
    const { id } = await seedLead(page, 'new');

    // 'invalid_status_xyz' 不在 lead_status::ALL 中：拒绝点 services/crm/lead.rs:517-525
    // `AppError::validation_displayable("非法线索状态 '{}'，合法取值为：…")`
    // —— 提交字段取值校验归 VALIDATION_ERROR 族且外显真实原因（回显值为用户自己提交）。
    const fail = await apiCallExpectFail(page, 'PUT', `/crm/leads/${id}/status`, {
      status: 'invalid_status_xyz',
    });
    expect(fail.status, `非法状态应返回 400，实际 ${fail.status}`).toBe(400);
    expect(failureCode(fail)).toBe('VALIDATION_ERROR');
    expect(
      String(fail.message ?? ''),
      `validation_displayable 出参必须外显真实校验原因（含回显的非法值），实际 message=${fail.message}`
    ).toContain("非法线索状态 'invalid_status_xyz'");

    // 回读确认状态未被篡改
    const current = await apiCallRaw<{ lead_status: string }>(page, 'GET', `/crm/leads/${id}`);
    expect(current.lead_status, '非法更新被拒后状态应保持 new').toBe('new');
  });

  test('04-04 商机阶段变更后 GET 回读 opportunity_stage 实际变', async ({ page }) => {
    // 建 QUALIFICATION 阶段商机
    const { id } = await seedOpportunity(page, 'QUALIFICATION');

    // GET 确认初始阶段
    const before = await apiCallRaw<{ id: number; opportunity_stage: string }>(
      page,
      'GET',
      `/crm/opportunities/${id}`
    );
    expect(before.opportunity_stage).toBe('QUALIFICATION');

    // PUT 修改阶段为 PROPOSAL
    await apiCall(page, 'PUT', `/crm/opportunities/${id}`, {
      opportunity_stage: 'PROPOSAL',
    });

    // GET 回读验证阶段变更
    const after = await apiCallRaw<{ id: number; opportunity_stage: string }>(
      page,
      'GET',
      `/crm/opportunities/${id}`
    );
    expect(
      after.opportunity_stage,
      `阶段变更后 opportunity_stage 应为 PROPOSAL（实际=${after.opportunity_stage}）`
    ).toBe('PROPOSAL');
  });

  test('04-05 非法商机阶段 PUT 返回 400 BUSINESS_ERROR（阶段流转门，脱敏出参）', async ({
    page,
  }) => {
    const { id } = await seedOpportunity(page, 'QUALIFICATION');

    // 'INVALID_STAGE_XYZ' 不在 QUALIFICATION 的合法后继集内：拒绝点在
    // services/crm/opp.rs:298-303 `AppError::business("商机阶段不允许从 {} 流转到 {}")`
    // —— 状态门/前置未满足归 BUSINESS_ERROR 族（本轮错误族归一口径），
    // 且文案含状态 token，按 utils/error.rs 安全边界保持脱敏，出参 message 恒为
    // err_msg::BUSINESS_PUBLIC（utils/messages.rs:43）。
    // 创建入口的取值域校验是另一条口径：crm/opp.rs:56-61 非法阶段 → VALIDATION_ERROR。
    const fail = await apiCallExpectFail(page, 'PUT', `/crm/opportunities/${id}`, {
      opportunity_stage: 'INVALID_STAGE_XYZ',
    });
    expect(fail.status, `非法阶段流转应恰为 400，实际 ${fail.status}`).toBe(400);
    expect(failureCode(fail), `应 BUSINESS_ERROR，实际 code=${fail.code}`).toBe('BUSINESS_ERROR');
    expect(
      String(fail.message ?? ''),
      `business（非 displayable）出参必须脱敏，实际 message=${fail.message}`
    ).toBe('业务处理失败');

    // 被拒后阶段不得被改写（回读钉原值）
    const after = await apiCallRaw<{ opportunity_stage: string }>(
      page,
      'GET',
      `/crm/opportunities/${id}`
    );
    expect(after.opportunity_stage, '非法流转被拒后阶段应保持 QUALIFICATION').toBe('QUALIFICATION');
  });

  test('04-06 创建线索时传入非法状态返回 400', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'POST', '/crm/leads', {
      lead_no: genCode('E2E-BAD'),
      lead_status: 'not_a_valid_status',
      company_name: '非法状态测试',
      contact_name: '测试',
      lead_source: 'WEBSITE',
      priority: 'MEDIUM',
    });
    // create_lead 写入口同口径（services/crm/lead.rs:64 → ensure_valid_lead_status:517
    // validation_displayable）：400 + VALIDATION_ERROR + 外显真实原因。
    expect(fail.status).toBe(400);
    expect(failureCode(fail)).toBe('VALIDATION_ERROR');
    expect(
      String(fail.message ?? ''),
      `validation_displayable 出参必须外显真实校验原因（含回显的非法值），实际 message=${fail.message}`
    ).toContain("非法线索状态 'not_a_valid_status'");
  });
});
