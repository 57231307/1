// CRM 线索分配规则 / 渠道 ROI / 规则驱动自动分配 / 客户信用 / 增强客户更新 契约 E2E — crm/09
//
// 功能（被测业务规则，逐条对后端源码核实，非编造）：
//   1) 线索分配规则是可配置的匹配行：创建时 daily_limit / priority / is_active 三枚缺省为
//      0 / 0 / true，assigned_user_ids 与 weights 以 JSON 原样存；规则列表可按 is_active 过滤。
//   2) 渠道 ROI 是"按区间统计后落一行"的计算端点：线索数按 lead_source+创建时间区间计，
//      商机数与订单数只按创建时间区间计（不按渠道过滤），转化率为零线索时取 0，
//      ROI 为 (收入-成本)/成本×100；结果行落 lead_source_roi 表。
//   3) 单条线索自动分配按规则表匹配：取激活规则按 priority 倒序，逐条比 source_filter
//      （支持 `*` 通配）与 industry_filter，命中后取 assigned_user_ids 数组第一个元素作归属人，
//      改写 crm_lead.owner_id 并返回 assigned_user_id。
//   4) 客户信用每客户恒一行，PUT 的路径参数是**客户自然键**（customer_credit_ratings.customer_id），
//      不是行主键；available_credit 恒等于 credit_limit - used_credit；客户行不存在时以 404 拒建。
//   5) 增强入口 PUT /crm/customers/enhanced/{id} 落 customers 表行，先过行级可见门再过
//      跨 owner 写门，customer_type 走唯一词表校验（校验在任何触库之前）。
// 被测端点与注册点：
//   - POST/GET /crm/leads/allocation-rules → handlers/crm_handler::create_allocation_rule 与
//     crm_handler::list_allocation_rules → services/crm/lead::create_allocation_rule（落
//     lead_allocation_rule 行，缺省口径见该函数体内的填充分支）
//   - POST /crm/leads/calculate-channel-roi → handlers/crm_handler::calculate_channel_roi
//     → services/crm/lead::calculate_channel_roi（统计口径与算式在函数体内）
//   - POST /crm/leads/{id}/auto-assign → handlers/crm_handler::auto_assign_lead
//     （跨 owner 写门）→ services/crm/lead::auto_assign_lead
//     （规则匹配与 owner_id 改写均在函数体内）
//   - POST/PUT/GET /crm/customer-credits 与 /crm/customer-credits/{id}
//     → handlers/customer_credit_handler::create_credit、customer_credit_handler::update_credit、
//     customer_credit_handler::get_credit、customer_credit_handler::list_credits →
//     services/customer_credit_limit::set_credit_rating（更新与创建两分支的缺省保持口径不同）
//   - PUT /crm/customers/enhanced/{id} → handlers/crm_customer_handler::update_customer
//     （先词表校验、再双门、最后落库，顺序在函数体内）
// 入参（字段名逐字符对齐 handler 的 Deserialize 结构）：
//   - 分配规则 CreateAllocationRuleRequest（services/crm/lead）：rule_name、rule_type、
//     source_filter、industry_filter、region_filter、assigned_user_ids（JSON 数组）、
//     weights（JSON）、daily_limit、priority、is_active
//   - 渠道 ROI CalculateChannelRoiRequest（handlers/crm_handler）：source、
//     start_date（YYYY-MM-DD）、end_date（YYYY-MM-DD）、cost（Decimal）
//   - 单条自动分配 AutoAssignRequest（handlers/crm_handler）：source、industry（可空）
//   - 信用创建 CreditRatingRequestDto（customer_credit_handler）：customer_id、
//     credit_level（≤20 字）、credit_score、credit_limit（Option<Decimal>，创建缺省 0）、
//     credit_days、remark（≤500 字）
//   - 信用更新 UpdateCreditDto（同在 customer_credit_handler）：credit_level / credit_score / credit_limit /
//     credit_days / remark 五枚均可选
//   - 增强客户更新 UpdateEnhancedCustomerRequest（crm_customer_handler）：customer_name、
//     contact_person、contact_phone、contact_email、address、customer_type、tax_number
//     （前端名 tax_number → 后端 tax_id）、credit_limit、bank_name、bank_account、status
// 传给谁 / 存哪里：
//   lead_allocation_rule、lead_source_roi（models/lead_source_roi，列 source/period_start/
//   period_end/cost/lead_count/converted_count/opportunity_count/order_count/revenue/
//   conversion_rate/roi/created_at）、customer_credit_ratings（models/customer_credit，
//   列 customer_id/customer_name/credit_level/credit_score/credit_limit/used_credit/
//   available_credit/credit_days/status/created_by/…）、crm_lead.owner_id、customers 行。
//   DECIMAL 出参是序列化字符串（未开 serde-float），用例只钉"字符串 + 数值相等"；
//   GET /crm/leads/allocation-rules 的 data 是裸数组，GET /crm/customer-credits 的 data 是
//   { items, total, page, page_size } 分页信封（customer_credit_handler 自述四键齐全），
//   GET /crm/customer-credits/{id} 的 data 是单对象。
//
// 清理与隔离事实（必读，未向后端新增任何删除端点）：
//   - lead_allocation_rule 与 lead_source_roi 与 customer_credit_ratings 都**没有物理删除端点**
//     （route-snapshot.txt 里 allocation-rules 只在册 GET/POST；customer-credits 的 DELETE 是
//     软停用 status→inactive，保留原行）。本片一律用唯一标记隔离：规则与 ROI 行的 source 取
//     本用例独有的随机串，信用行只按自建 customer_id 读；残留行不参与任何全局断言。
//   - 规则用 priority=999999 排到激活规则首位，但作用面靠 source_filter 的唯一串收窄，
//     不会劫持其他渠道的线索自动分配（lead_source 是无词表校验的自由文本，
//     services/crm/lead 仅缺省 OTHER，故可用唯一串精确绑定）。
//   - 信用"更新分支把未提交的 credit_level/credit_score/credit_days 覆写成缺省 B/60/30"
//     （services/customer_credit_limit，只有 credit_limit 做了保持原值）是既存缺陷：
//     本片只断"提交了的列真的落库"与 available_credit 的算术，不把 B/60/30 当期望值钉死，
//     以免将来修复时被测试锁住；缺陷本身交回研发处置。
//   - 单条自动分配只改 owner_id，不改 lead_status（auto_assign_lead 写列不含 lead_status），
//     本片按此现状断 owner_id 改写并回读 lead_status 仍为提交值，属可回归的线格式记录。
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  tryCleanup,
} from '../flow/helpers';

/** lead_allocation_rule 行（services/crm/lead 的写入列） */
interface AllocationRuleRow {
  id: number;
  rule_name: string;
  rule_type: string;
  source_filter: string | null;
  industry_filter: string | null;
  region_filter: string | null;
  assigned_user_ids: unknown;
  weights: unknown;
  daily_limit: number;
  priority: number;
  is_active: boolean;
}

/** lead_source_roi 行（models/lead_source_roi） */
interface ChannelRoiRow {
  id: number;
  source: string;
  period_start: string;
  period_end: string;
  cost: string;
  lead_count: number;
  converted_count: number;
  opportunity_count: number;
  order_count: number;
  revenue: string;
  conversion_rate: string;
  roi: string;
}

/** CustomerCreditView（services/customer_credit_service） */
interface CreditView {
  id: number;
  customer_id: number;
  customer_name: string | null;
  credit_level: string | null;
  credit_score: number | null;
  credit_limit: string;
  used_credit: string;
  available_credit: string;
  credit_days: number | null;
  status: string;
  created_by: number | null;
}

/** 分页信封（本项目统一 { items, total, page, page_size }） */
interface CreditList {
  items: CreditView[];
  total: number;
  page: number;
  page_size: number;
}

interface LeadRow {
  id: number;
  lead_no: string;
  lead_source: string;
  lead_status: string | null;
  owner_id: number;
}

interface CustomerRow {
  id: number;
  customer_name: string;
  contact_person: string | null;
  customer_type: string | null;
  status: string | null;
}

const LEAD_STATUS_NEW = 'new';
/** 规则优先级取极大值，保证本用例规则排在激活规则首位（services/crm/lead 按 priority 倒序） */
const E2E_RULE_PRIORITY = 999_999;

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

async function selfUserId(page: import('@playwright/test').Page): Promise<number> {
  const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
  expect(typeof me.id, `GET /auth/me 出参 id 应为数字，实际 ${JSON.stringify(me.id)}`).toBe(
    'number'
  );
  return me.id;
}

/** 取一个非自身的活跃用户 id（自动分配的归属目标） */
async function pickOtherUserId(
  page: import('@playwright/test').Page,
  selfId: number
): Promise<number> {
  const res = await apiCallRaw<{ users: Array<{ id: number; is_active: boolean }> }>(
    page,
    'GET',
    '/users?page=1&page_size=100'
  );
  expect(Array.isArray(res.users), 'GET /users 响应缺 data.users 数组').toBe(true);
  const candidate = res.users.find(u => u.id !== selfId && u.is_active);
  if (!candidate) throw new Error('库内没有可用作分配目标的活跃用户');
  return candidate.id;
}

/** 建指定 lead_source 的线索并登记清理；lead_source 是无词表校验的自由文本，可用作唯一作用域键 */
async function seedLeadWithSource(
  page: import('@playwright/test').Page,
  leadSource: string
): Promise<LeadRow> {
  const leadNo = genCode('E2E-ALLOC');
  const created = await apiCall<LeadRow>(page, 'POST', '/crm/leads', {
    lead_no: leadNo,
    lead_status: LEAD_STATUS_NEW,
    lead_source: leadSource,
    company_name: `E2E 规则分配线索 ${leadNo}`,
    contact_name: 'E2E 联系人',
    mobile_phone: '13800138004',
    priority: 'MEDIUM',
  });
  CLEANUP.push({ path: `/crm/leads/${created.data.id}`, label: 'crm_lead' });
  return created.data;
}

/** 建自建客户并登记清理，返回行 */
async function seedCustomer(
  page: import('@playwright/test').Page,
  marker: string
): Promise<CustomerRow> {
  const created = await apiCall<CustomerRow>(page, 'POST', '/crm/customers', {
    customer_name: `E2E 信用宿主客户 ${marker}`,
  });
  CLEANUP.push({ path: `/crm/customers/${created.data.id}`, label: 'crm_customer' });
  return created.data;
}

test.describe('09 分配规则 / 渠道 ROI / 规则驱动自动分配 / 客户信用 / 增强客户更新契约', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('09-01 创建线索分配规则，缺省三值与 JSON 列原样落库并可按 is_active 回读', async ({
    page,
  }) => {
    const me = await selfUserId(page);
    const other = await pickOtherUserId(page, me);
    const ruleName = genCode('E2E-RULE');
    const sourceFilter = genCode('E2E-SRC');

    const created = await apiCall<AllocationRuleRow>(page, 'POST', '/crm/leads/allocation-rules', {
      rule_name: ruleName,
      rule_type: 'round_robin',
      source_filter: sourceFilter,
      industry_filter: null,
      region_filter: null,
      assigned_user_ids: [other],
      weights: null,
      daily_limit: 7,
      priority: E2E_RULE_PRIORITY,
      is_active: true,
    });

    expect(created.data.id, '规则应有主键').toBeGreaterThan(0);
    expect(created.data.rule_name).toBe(ruleName);
    expect(created.data.rule_type).toBe('round_robin');
    expect(created.data.source_filter, 'source_filter 应原样落库').toBe(sourceFilter);
    expect(created.data.industry_filter).toBeNull();
    expect(created.data.assigned_user_ids, 'assigned_user_ids 应以 JSON 数组原样存').toEqual([
      other,
    ]);
    expect(created.data.daily_limit, '显式 daily_limit 应落库').toBe(7);
    expect(created.data.priority).toBe(E2E_RULE_PRIORITY);
    expect(created.data.is_active).toBe(true);

    // 缺省分支（lead）：daily_limit/priority/is_active 未提交时取 0/0/true
    const withDefaults = await apiCall<AllocationRuleRow>(
      page,
      'POST',
      '/crm/leads/allocation-rules',
      {
        rule_name: genCode('E2E-RULE-D'),
        rule_type: 'weighted',
        source_filter: genCode('E2E-SRC'),
      }
    );
    expect(withDefaults.data.daily_limit, '未提交 daily_limit 时缺省 0').toBe(0);
    expect(withDefaults.data.priority, '未提交 priority 时缺省 0').toBe(0);
    expect(withDefaults.data.is_active, '未提交 is_active 时缺省 true').toBe(true);
    expect(withDefaults.data.weights, '未提交 weights 应为 NULL').toBeNull();

    // 回读：GET 列表按 is_active=true 过滤，两条自建规则都应能按 id 查到
    const rules = await apiCallRaw<AllocationRuleRow[]>(
      page,
      'GET',
      '/crm/leads/allocation-rules?is_active=true'
    );
    expect(Array.isArray(rules), 'GET allocation-rules 出参应为数组').toBe(true);
    const readback = rules.find(row => row.id === created.data.id);
    expect(readback, '列表应含刚建的规则').toBeTruthy();
    expect(readback!.source_filter).toBe(sourceFilter);
    expect(readback!.priority).toBe(E2E_RULE_PRIORITY);
    expect(
      rules.some(row => row.id === withDefaults.data.id),
      '缺省规则也应在激活列表内'
    ).toBe(true);
  });

  test('09-02 渠道 ROI 按区间统计后落行，零线索时转化率取 0 且 ROI 为负成本比', async ({
    page,
  }) => {
    const source = genCode('E2E-ROI');
    // 取未来区间：该渠道不可能有线索，且区间内也不会有商机/订单，计数与算式完全确定
    const start = '2030-01-01';
    const end = '2030-01-02';

    const created = await apiCall<ChannelRoiRow>(page, 'POST', '/crm/leads/calculate-channel-roi', {
      source,
      start_date: start,
      end_date: end,
      cost: 1000,
    });

    expect(created.data.id, 'ROI 结果应落库成行').toBeGreaterThan(0);
    expect(created.data.source).toBe(source);
    expect(created.data.period_start, 'period_start 应等于提交的 start_date').toBe(start);
    expect(created.data.period_end, 'period_end 应等于提交的 end_date').toBe(end);
    expect(created.data.lead_count, '该唯一渠道区间内不应有线索').toBe(0);
    expect(created.data.converted_count).toBe(0);
    expect(created.data.opportunity_count, '区间内无商机').toBe(0);
    expect(created.data.order_count, '区间内无订单').toBe(0);
    expect(Number(created.data.revenue), '零订单收入应为 0').toBe(0);
    expect(Number(created.data.conversion_rate), '零线索时转化率取 0（lead.rs:1402-1406）').toBe(0);
    // roi = (0 - 1000) / 1000 × 100 = -100
    expect(
      Number(created.data.roi),
      `ROI 应按 (收入-成本)/成本×100 计算，实际 ${created.data.roi}`
    ).toBe(-100);
    expect(
      typeof created.data.cost === 'string',
      `DECIMAL 出参应是序列化字符串，实际 ${JSON.stringify(created.data.cost)}`
    ).toBe(true);
    expect(Number(created.data.cost), 'cost 应原样存提交值').toBe(1000);
  });

  test('09-03 规则驱动的单条线索自动分配改写 owner_id，返回分配人并回读落库', async ({ page }) => {
    const me = await selfUserId(page);
    const other = await pickOtherUserId(page, me);
    const sourceFilter = genCode('E2E-SRC');

    const rule = await apiCall<AllocationRuleRow>(page, 'POST', '/crm/leads/allocation-rules', {
      rule_name: genCode('E2E-RULE-A'),
      rule_type: 'round_robin',
      source_filter: sourceFilter,
      assigned_user_ids: [other, me],
      priority: E2E_RULE_PRIORITY,
      is_active: true,
    });
    expect(rule.data.id, '分配规则应建成本地可匹配行').toBeGreaterThan(0);

    const lead = await seedLeadWithSource(page, sourceFilter);
    expect(lead.owner_id, '前置：新建线索 owner 应为当前用户').toBe(me);

    const assigned = await apiCall<{ assigned_user_id: number | null }>(
      page,
      'POST',
      `/crm/leads/${lead.id}/auto-assign`,
      { source: sourceFilter, industry: null }
    );
    // 命中规则后取 assigned_user_ids 的第一个元素（lead 简单轮询）
    expect(
      assigned.data.assigned_user_id,
      `应分配到规则里第一个销售 ${other}，实际 ${assigned.data.assigned_user_id}`
    ).toBe(other);

    const after = await apiCallRaw<LeadRow>(page, 'GET', `/crm/leads/${lead.id}`);
    expect(after.owner_id, `回读 owner_id 应等于分配人 ${other}`).toBe(other);
    expect(
      after.lead_status,
      `本端点只改 owner_id 不改 lead_status（lead.rs:1522-1524 无该赋值），应仍为 ${LEAD_STATUS_NEW}，实际 ${after.lead_status}`
    ).toBe(LEAD_STATUS_NEW);
  });

  test('09-04 客户信用创建与按客户自然键更新，available_credit 恒等于额度减已用', async ({
    page,
  }) => {
    const customer = await seedCustomer(page, genCode('E2E-CRD'));

    const created = await apiCall<CreditView>(page, 'POST', '/crm/customer-credits', {
      customer_id: customer.id,
      credit_level: 'A',
      credit_score: 88,
      credit_limit: 500000,
      credit_days: 45,
      remark: 'E2E 初始信用',
    });
    expect(created.message, '创建文案由 customer_credit_handler.rs:350-353 固定').toBe(
      '客户信用创建成功'
    );
    expect(created.data.customer_id, '信用行应按客户键挂到自建客户').toBe(customer.id);
    expect(created.data.customer_name, 'customer_name 应来自 customers 真值富化').toBe(
      customer.customer_name
    );
    expect(created.data.credit_level).toBe('A');
    expect(created.data.credit_score).toBe(88);
    expect(Number(created.data.credit_limit), '显式额度应落库').toBe(500000);
    expect(Number(created.data.used_credit), '新建行已用额度应为 0').toBe(0);
    expect(Number(created.data.available_credit), 'available 应等于 limit-used').toBe(500000);
    expect(created.data.credit_days).toBe(45);
    expect(created.data.status, '创建分支落 master_data::ACTIVE').toBe('active');
    // 创建分支的 created_by 不在本用例断言之列：services/customer_credit_limit 的
    // ActiveModel 用 ..Default::default() 未写 created_by，操作人只进审计侧。
    // 该"创建行不记 created_by"作为既存缺口登记在回报里，不在测试里钉成期望值。
    const rowId = created.data.id;

    // 路径参数是客户自然键：更新必须命中同一行而不是新建第二行
    const updated = await apiCall<CreditView>(page, 'PUT', `/crm/customer-credits/${customer.id}`, {
      credit_limit: 800000,
      credit_level: 'AA',
      credit_score: 95,
      credit_days: 60,
    });
    expect(updated.message, '更新文案由 customer_credit_handler.rs:388-391 固定').toBe(
      '客户信用更新成功'
    );
    expect(updated.data.id, 'PUT 应命中同一行（自然键语义，不是插新行）').toBe(rowId);
    expect(Number(updated.data.credit_limit), '提交的额度应落库').toBe(800000);
    expect(Number(updated.data.available_credit), 'available 应随额度重算（800000-0）').toBe(
      800000
    );
    expect(updated.data.credit_level, '提交的等级应落库').toBe('AA');
    expect(updated.data.credit_score, '提交的评分应落库').toBe(95);
    expect(updated.data.credit_days, '提交的天数应落库').toBe(60);

    // 独立回读 + 唯一性：同一客户在表里必须只有一行
    const readback = await apiCallRaw<CreditView>(
      page,
      'GET',
      `/crm/customer-credits/${customer.id}`
    );
    expect(readback.id, '详情回读应是同一行').toBe(rowId);
    expect(Number(readback.credit_limit)).toBe(800000);
    expect(readback.credit_level).toBe('AA');

    const list = await apiCallRaw<CreditList>(
      page,
      'GET',
      `/crm/customer-credits?customer_id=${customer.id}&page=1&page_size=10`
    );
    expect(Array.isArray(list.items), '列表出参应含 items 数组').toBe(true);
    expect(
      list.items.filter(row => row.customer_id === customer.id),
      '每客户恒一行：该客户的信用行应恰有一条'
    ).toHaveLength(1);
  });

  test('09-05 信用更新的超长等级被 validator 门拒 400 并外显原文，原值零改动', async ({ page }) => {
    const customer = await seedCustomer(page, genCode('E2E-CRDBAD'));
    const created = await apiCall<CreditView>(page, 'POST', '/crm/customer-credits', {
      customer_id: customer.id,
      credit_level: 'A',
      credit_score: 88,
      credit_limit: 300000,
      credit_days: 45,
      remark: 'E2E 负例基线',
    });
    expect(created.data.credit_level).toBe('A');

    // UpdateCreditDto 的 length(max=20)（customer_credit_handler）经
    // utils/error 的 validation_displayable 通道外显字段自写文案
    const tooLong = 'X'.repeat(21);
    const fail = await apiCallExpectFail(page, 'PUT', `/crm/customer-credits/${customer.id}`, {
      credit_level: tooLong,
    });
    expect(fail.status, `超长等级应恰为 400，实际 ${fail.status}`).toBe(400);
    expect(failureCode(fail), `应 VALIDATION_ERROR，实际 code=${fail.code}`).toBe(
      'VALIDATION_ERROR'
    );
    expect(
      String(fail.message ?? ''),
      `validator 通道必须外显自写文案，实际 message=${fail.message}`
    ).toBe('信用等级长度不能超过20字符');

    const after = await apiCallRaw<CreditView>(page, 'GET', `/crm/customer-credits/${customer.id}`);
    expect(after.credit_level, `非法更新不得改写等级，应仍为 A，实际 ${after.credit_level}`).toBe(
      'A'
    );
    expect(Number(after.credit_limit), '非法请求不得连带改动额度').toBe(300000);
  });

  test('09-06 给不存在客户建信用被 FK 前置校验拒 404，同键随后合法创建可成功', async ({ page }) => {
    const ghostCustomerId = 999_997;

    const fail = await apiCallExpectFail(page, 'POST', '/crm/customer-credits', {
      customer_id: ghostCustomerId,
      credit_level: 'A',
      credit_score: 80,
      credit_limit: 100000,
      credit_days: 30,
      remark: 'E2E 幽灵客户',
    });
    // services/customer_credit_limit 先查 customers 行，缺失即 not_found，
    // 不把驱动约束违例拍平成 500
    expect(fail.status, `不存在客户应恰为 404，实际 ${fail.status}`).toBe(404);
    expect(failureCode(fail), `应 NOT_FOUND，实际 code=${fail.code}`).toBe('NOT_FOUND');

    const detail = await apiCallExpectFail(page, 'GET', `/crm/customer-credits/${ghostCustomerId}`);
    expect(detail.status, '该客户的信用详情也应 404').toBe(404);

    // 零脏行自证：同一自然键随后按合法客户创建必须成功（若上一次真的落了脏行会撞唯一键）
    const customer = await seedCustomer(page, genCode('E2E-CRDOK'));
    const ok = await apiCall<CreditView>(page, 'POST', '/crm/customer-credits', {
      customer_id: customer.id,
      credit_level: 'B',
      credit_score: 70,
      credit_limit: 200000,
      credit_days: 30,
      remark: 'E2E 合法建信用',
    });
    expect(ok.data.customer_id).toBe(customer.id);
    expect(Number(ok.data.credit_limit)).toBe(200000);
  });

  test('09-07 增强入口 PUT 落 customers 行并改 status，非法 customer_type 被词表门拒且零改动', async ({
    page,
  }) => {
    const customer = await seedCustomer(page, genCode('E2E-ENHP'));
    const newName = `E2E 增强改名 ${genCode('C')}`;

    const updated = await apiCall<CustomerRow>(
      page,
      'PUT',
      `/crm/customers/enhanced/${customer.id}`,
      {
        customer_name: newName,
        contact_person: 'E2E 联系人甲',
        status: 'inactive',
      }
    );
    expect(updated.data.id, 'PUT 命中的应是同一客户行').toBe(customer.id);
    expect(updated.data.customer_name, '改名应落库').toBe(newName);
    expect(updated.data.contact_person, '联系人应落库').toBe('E2E 联系人甲');
    expect(
      updated.data.status,
      `增强入口的 status 是 31c 停用矩阵核心字段，应落 inactive，实际 ${updated.data.status}`
    ).toBe('inactive');

    const readback = await apiCallRaw<CustomerRow>(page, 'GET', `/crm/customers/${customer.id}`);
    expect(readback.customer_name, '详情回读应同为改后名').toBe(newName);
    expect(readback.status, '回读 status 应同为 inactive').toBe('inactive');
    expect(readback.contact_person).toBe('E2E 联系人甲');

    // 词表门（crm_customer_handler，校验在任何触库之前）：非法 customer_type 整笔拒
    const baselineType = await apiCallRaw<CustomerRow>(
      page,
      'GET',
      `/crm/customers/${customer.id}`
    );
    const badType = await apiCallExpectFail(page, 'PUT', `/crm/customers/enhanced/${customer.id}`, {
      customer_type: 'not_a_customer_type',
      customer_name: `${newName}-又被改`,
    });
    expect(badType.status, `非法 customer_type 应恰为 400，实际 ${badType.status}`).toBe(400);
    expect(failureCode(badType), `应 VALIDATION_ERROR，实际 code=${badType.code}`).toBe(
      'VALIDATION_ERROR'
    );

    const afterFail = await apiCallRaw<CustomerRow>(page, 'GET', `/crm/customers/${customer.id}`);
    expect(
      afterFail.customer_name,
      '门在任何写之前：同批次里的 customer_name 也不得被部分提交'
    ).toBe(newName);
    expect(
      afterFail.customer_type,
      `customer_type 应保持门前的值 ${JSON.stringify(baselineType.customer_type)}，实际 ${JSON.stringify(afterFail.customer_type)}`
    ).toBe(baselineType.customer_type);
  });
});
