// CRM 公海规则 / 客户联系人 / 销售目标 写口契约 E2E — crm/07
//
// 功能（被测业务规则，逐条对后端源码核实）：
//   1) 公海规则是可启停的配置行：rule_type 与 customer_type 受取值域门约束，负数值另立一道门，
//      三处都用 validation_displayable（出参外显真实原因）；PUT 是按列的局部更新，
//      未提交的 name / rule_type / customer_type 必须原样保留；DELETE 是硬删。
//   2) 客户联系人是 customer_contacts 表的真实行（不再从 crm_lead 拼伪数据），
//      每个客户最多一个主联系人：新提交 is_primary=true 时先清其他主联系人；
//      PUT 只改提交了的列（Some 才 Set，None 保留原值）。
//   3) 销售目标落 sales_statistics 表并按 statistic_type='target' 归类，
//      创建时把起止日期拼进 dimension_name 存字符串；PUT 走 period 定位，
//      提交的 remarks 会覆盖 dimension_name，未提交的 status 回落 master_data::ACTIVE。
// 被测端点与注册点：
//   - POST/GET /crm/pool/rules、PUT/DELETE /crm/pool/rules/{id}
//     backend/src/routes/crm.rs:283-289 → handlers/crm_pool_handler.rs
//     （create_pool_rule:391 取值域门 / update_pool_rule:445 / delete_pool_rule:464 / list_pool_rules:381）
//     规则实现 services/crm/pool.rs（list_rules:310 order by id asc / create_rule:319 固定 is_enabled=true /
//     update_rule:345 局部更新，缺 id → not_found / delete_rule:373 硬删）
//   - POST /crm/customers/{cid}/contacts、GET /crm/customers/{cid}/contacts、
//     PUT|DELETE /crm/customers/{cid}/contacts/{contactId}
//     backend/src/routes/crm.rs:242-247 → handlers/crm_customer_handler.rs
//     （create_contact:432 / update_contact:456 / delete_contact:479 / list_contacts:405）
//     实现 services/customer_ops/contact.rs（list_customer_contacts:25 排序 is_primary desc, name asc /
//     create_customer_contact:40 / update_customer_contact:75 / apply_contact_updates:109 /
//     clear_primary_contacts_txn:145 / delete_customer_contact:135）
//   - POST /crm/sales-analysis/targets、GET /crm/sales-analysis/targets、
//     PUT /crm/sales-analysis/targets/{period}
//     backend/src/routes/crm.rs:202-212 → handlers/sales_analysis_handler.rs
//     （create_target:165 / get_targets:146 / update_sales_target:221）
//     实现 services/sales_analysis_service.rs（get_targets:263 order by id desc /
//     create_target:285 / update_target:468）
// 入参（字段名逐字符对齐 handler 的 Deserialize 结构）：
//   - 公海规则创建：name（规则名，同时作本用例唯一标记）、rule_type（protection_period /
//     claim_limit / max_holdings 三值之一）、rule_value（整数，负数被拒）、customer_type
//     （all / wholesale / retail / vip 四值之一，注意这是「规则作用域」档位，与
//     customers.customer_type 列的词表无关，运行时只有 all 被 pool.rs 消费）、notes（可空）
//   - 公海规则更新：rule_value / is_enabled / notes 三枚均可选
//   - 联系人创建：name（必填 1-50 字）、title（可选 ≤100）、phone（必填 1-50，宽松校验，
//     兼容手机/座机/国际号）、email（可选，需邮箱格式）、is_primary（缺省 false）、
//     remarks（可选 ≤500）；更新版全部字段可选
//   - 销售目标创建：target_type、target_id（写入 dimension_id）、period（唯一标记，
//     同时是 PUT 的路径定位键）、target_amount、start_date、end_date；
//     更新：target_amount / status / remarks 三枚可选
// 传给谁 / 存哪里：
//   customer_pool_rules（name/rule_type/rule_value/customer_type/is_enabled/notes）、
//   customer_contacts（customer_id/name/title/phone/email/is_primary/remarks/created_by）、
//   sales_statistics（statistic_type='target'/period/dimension_type/dimension_id/dimension_name/total_amount）。
//   出参一律 PaginatedResponse 之外的信封形状：GET /crm/pool/rules 与
//   GET /crm/customers/{cid}/contacts 与 GET /crm/sales-analysis/targets 的 data 都是**裸数组**。
//   联系人读写出口经 crm_customer_handler.rs:98 `apply_customer_field_permission` 统一脱敏，
//   utils/field_mask.rs:101-104 对 admin 直接原样放行——本 worker 是 admin 会话，
//   故 phone/email 断言按明文钉；换成非 admin 角色时这两列会变掩码，断言面须随之收窄。
//   DECIMAL 出参是序列化字符串（未开 serde-float），用例只钉「字符串 + 数值相等」，
//   不在测试内二次舍入来凑库侧口径。
//
// 清理与隔离事实（必读）：
//   公海规则与联系人都有在册 DELETE，用例结尾必删并回读「已不在列表内」。
//   销售目标只有 GET/POST/PUT 三方法、无 DELETE 端点（routes/crm.rs 未注册），
//   本卡不为凑绿向后端新增删除端点，改为唯一 period 标记 + 只按自建 id 回读隔离；
//   留下的 target 行属既存缺口（目标行无法回收，且 update_target 的 actual_amount 直接
//   取 total_amount、completion_rate 恒为自身比自身 ⇒ 100，不反映任何真实成交），
//   本文件把它当既成出参事实钉住并在 07-05 注释点名，不当作业务正确性背书。
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

/** customer_pool_rule::Model 出参（models/customer_pool_rule.rs:27-52） */
interface PoolRuleRow {
  id: number;
  name: string;
  rule_type: string;
  rule_value: number;
  customer_type: string;
  is_enabled: boolean;
  notes: string | null;
}

/** customer_contact::Model 出参（models/customer_contact.rs:16-36） */
interface ContactRow {
  id: number;
  customer_id: number;
  name: string;
  title: string | null;
  phone: string;
  email: string | null;
  is_primary: boolean;
  remarks: string | null;
  created_by: number | null;
}

/** sales_analysis::Model 出参（models/sales_analysis.rs:11-24），DECIMAL 列为字符串 */
interface SalesTargetRow {
  id: number;
  statistic_type: string;
  period: string;
  dimension_type: string;
  dimension_id: number | null;
  dimension_name: string | null;
  total_amount: string;
}

/** SalesTargetDto 出参（models/dto/sales_analysis_dto.rs:62-70），computed 字段非落库值 */
interface SalesTargetDto {
  id: number;
  period: string;
  target_amount: string;
  actual_amount: string;
  completion_rate: string;
  variance: string;
  status: string;
}

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 拉全量公海规则（GET /crm/pool/rules 的 data 是裸数组，按 id 升序） */
async function listPoolRules(page: import('@playwright/test').Page): Promise<PoolRuleRow[]> {
  const rules = await apiCallRaw<PoolRuleRow[]>(page, 'GET', '/crm/pool/rules');
  expect(
    Array.isArray(rules),
    `GET /crm/pool/rules 出参应为数组，实际 ${JSON.stringify(rules)}`
  ).toBe(true);
  return rules;
}

/** 建一个最小客户作联系人宿主，并登记清理 */
async function seedCustomer(
  page: import('@playwright/test').Page,
  marker: string
): Promise<number> {
  const created = await apiCall<{ id: number }>(page, 'POST', '/crm/customers', {
    customer_name: `E2E 联系人宿主客户 ${marker}`,
  });
  CLEANUP.push({ path: `/crm/customers/${created.data.id}`, label: 'crm_customer' });
  return created.data.id;
}

/** 拉某客户联系人列表（data 裸数组，排序 is_primary desc, name asc） */
async function listContacts(
  page: import('@playwright/test').Page,
  customerId: number
): Promise<ContactRow[]> {
  const rows = await apiCallRaw<ContactRow[]>(page, 'GET', `/crm/customers/${customerId}/contacts`);
  expect(Array.isArray(rows), 'GET contacts 出参应为数组').toBe(true);
  return rows;
}

test.describe('07 公海规则 / 联系人 / 销售目标 写口契约', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('07-01 公海规则创建-局部更新-删除，每步都回读列表真值', async ({ page }) => {
    const ruleName = genCode('E2E-POOL');

    const created = await apiCall<PoolRuleRow>(page, 'POST', '/crm/pool/rules', {
      name: ruleName,
      rule_type: 'claim_limit',
      rule_value: 5,
      customer_type: 'all',
      notes: '初始备注',
    });
    expect(created.message, '创建成功文案由 crm_pool_handler.rs:437-440 固定').toBe(
      '公海规则创建成功'
    );
    expect(created.data.name).toBe(ruleName);
    expect(created.data.rule_type).toBe('claim_limit');
    expect(created.data.rule_value).toBe(5);
    expect(created.data.customer_type, 'all 是运行时唯一被 pool.rs 消费的作用域档').toBe('all');
    expect(created.data.is_enabled, 'create_rule 固定写入 true（services/crm/pool.rs:334）').toBe(
      true
    );
    expect(created.data.notes).toBe('初始备注');

    // 回读：列表端点独立查库
    const afterCreate = (await listPoolRules(page)).find(row => row.id === created.data.id);
    expect(afterCreate, '列表回读应能查到刚创建的规则').toBeTruthy();
    expect(afterCreate!.rule_value).toBe(5);
    expect(afterCreate!.is_enabled).toBe(true);

    // 局部更新：只提交 rule_value / is_enabled / notes
    const updated = await apiCall<PoolRuleRow>(page, 'PUT', `/crm/pool/rules/${created.data.id}`, {
      rule_value: 9,
      is_enabled: false,
      notes: '改后备注',
    });
    expect(updated.message, '更新成功文案由 crm_pool_handler.rs:457-460 固定').toBe(
      '公海规则更新成功'
    );
    expect(updated.data.id, '更新的应是同一条规则').toBe(created.data.id);
    expect(updated.data.rule_value, '提交的 rule_value 应落库').toBe(9);
    expect(updated.data.is_enabled, '提交的 is_enabled=false 应落库（不得被当成未提交）').toBe(
      false
    );
    expect(updated.data.notes).toBe('改后备注');
    // update_rule 不触碰未提交的列（services/crm/pool.rs:357-366 三处 if let Some）
    expect(updated.data.name, '局部更新不得改动 name').toBe(ruleName);
    expect(updated.data.rule_type, '局部更新不得改动 rule_type').toBe('claim_limit');
    expect(updated.data.customer_type, '局部更新不得改动 customer_type').toBe('all');

    const afterUpdate = (await listPoolRules(page)).find(row => row.id === created.data.id);
    expect(afterUpdate!.rule_value, '回读 rule_value 应为 9').toBe(9);
    expect(afterUpdate!.is_enabled, '回读 is_enabled 应为 false').toBe(false);
    expect(afterUpdate!.notes).toBe('改后备注');

    // 删除 + 回读消失
    const deleted = await apiCall<{ id: number }>(
      page,
      'DELETE',
      `/crm/pool/rules/${created.data.id}`
    );
    expect(deleted.message, '删除成功文案由 crm_pool_handler.rs:473-476 固定').toBe(
      '公海规则删除成功'
    );
    expect(deleted.data.id, '删除响应应回显被删规则 id').toBe(created.data.id);

    const afterDelete = await listPoolRules(page);
    expect(
      afterDelete.filter(row => row.id === created.data.id),
      '硬删后列表不得再含该规则'
    ).toHaveLength(0);
    expect(
      afterDelete.filter(row => row.name === ruleName),
      '按唯一名称过滤也应查无此规则'
    ).toHaveLength(0);
  });

  test('07-02 公海规则三处取值域门各返回 400 并外显真实原因，且零落库', async ({ page }) => {
    const marker = genCode('E2E-POOLBAD');

    const badType = await apiCallExpectFail(page, 'POST', '/crm/pool/rules', {
      name: `${marker}-T`,
      rule_type: 'not_a_rule_type',
      rule_value: 1,
      customer_type: 'all',
      notes: null,
    });
    expect(badType.status, `非法 rule_type 应恰为 400，实际 ${badType.status}`).toBe(400);
    expect(failureCode(badType), `应 VALIDATION_ERROR，实际 code=${badType.code}`).toBe(
      'VALIDATION_ERROR'
    );
    expect(
      String(badType.message ?? ''),
      `create_pool_rule 用 validation_displayable，须外显取值域原文，实际 message=${badType.message}`
    ).toBe('规则类型必须为 protection_period / claim_limit / max_holdings');

    const badCustomerType = await apiCallExpectFail(page, 'POST', '/crm/pool/rules', {
      name: `${marker}-C`,
      rule_type: 'claim_limit',
      rule_value: 1,
      customer_type: 'not_a_customer_type',
      notes: null,
    });
    expect(badCustomerType.status).toBe(400);
    expect(failureCode(badCustomerType)).toBe('VALIDATION_ERROR');
    expect(String(badCustomerType.message ?? ''), `实际 message=${badCustomerType.message}`).toBe(
      '客户类型必须为 all / wholesale / retail / vip'
    );

    const negativeValue = await apiCallExpectFail(page, 'POST', '/crm/pool/rules', {
      name: `${marker}-V`,
      rule_type: 'claim_limit',
      rule_value: -1,
      customer_type: 'all',
      notes: null,
    });
    expect(negativeValue.status).toBe(400);
    expect(failureCode(negativeValue)).toBe('VALIDATION_ERROR');
    expect(String(negativeValue.message ?? ''), `实际 message=${negativeValue.message}`).toBe(
      '规则数值不能为负数'
    );

    // 三次非法提交应零落库：列表里按唯一标记前缀过滤必须一条没有
    const rules = await listPoolRules(page);
    expect(
      rules.filter(row => row.name.startsWith(marker)),
      '非法公海规则不得落库'
    ).toHaveLength(0);
  });

  test('07-03 联系人 PUT 只改提交的列，admin 出参按明文钉 phone/email', async ({ page }) => {
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
    const customerId = await seedCustomer(page, genCode('E2E-CTC'));
    const phone = '13800138001';
    const email = 'e2e-contact@test.com';

    const created = await apiCall<ContactRow>(
      page,
      'POST',
      `/crm/customers/${customerId}/contacts`,
      {
        name: 'E2E 初始姓名',
        title: '采购主管',
        phone,
        email,
        is_primary: true,
        remarks: '初始备注',
      }
    );
    expect(created.message, '创建成功文案由 crm_customer_handler.rs:448-451 固定').toBe(
      '联系人创建成功'
    );
    expect(created.data.customer_id, '联系人应挂到路径指定的客户').toBe(customerId);
    expect(created.data.name).toBe('E2E 初始姓名');
    expect(
      created.data.phone,
      'admin 会话出参不脱敏（utils/field_mask.rs:102）应等于提交明文'
    ).toBe(phone);
    expect(created.data.email).toBe(email);
    expect(created.data.is_primary).toBe(true);
    expect(created.data.created_by, 'created_by 应落当前登录用户').toBe(me.id);

    // 只改 name 与 title：其余列必须原样保留（apply_contact_updates 每列 if let Some）
    const updated = await apiCall<ContactRow>(
      page,
      'PUT',
      `/crm/customers/${customerId}/contacts/${created.data.id}`,
      { name: 'E2E 改后姓名', title: '销售经理' }
    );
    expect(updated.message, '更新成功文案由 crm_customer_handler.rs:472-475 固定').toBe(
      '联系人更新成功'
    );
    expect(updated.data.id, '更新的应是同一条联系人').toBe(created.data.id);
    expect(updated.data.name).toBe('E2E 改后姓名');
    expect(updated.data.title).toBe('销售经理');
    expect(updated.data.phone, '未提交 phone 不得被清空或改写').toBe(phone);
    expect(updated.data.email, '未提交 email 不得被清空').toBe(email);
    expect(updated.data.is_primary, '未提交 is_primary 不得被改动').toBe(true);
    expect(updated.data.remarks, '未提交 remarks 不得被清空').toBe('初始备注');

    const row = (await listContacts(page, customerId)).find(item => item.id === created.data.id);
    expect(row, '列表回读应能查到该联系人').toBeTruthy();
    expect(row!.name, '回读 name 应为改后值').toBe('E2E 改后姓名');
    expect(row!.title).toBe('销售经理');
    expect(row!.customer_id).toBe(customerId);
  });

  test('07-04 主联系人唯一性置换与 DELETE 精确删除，回读列表真值', async ({ page }) => {
    const customerId = await seedCustomer(page, genCode('E2E-CTD'));

    const first = await apiCall<ContactRow>(page, 'POST', `/crm/customers/${customerId}/contacts`, {
      name: 'E2E 甲联系人',
      phone: '13800138002',
      is_primary: true,
    });
    expect(first.data.is_primary, '首个主联系人应直接落 true').toBe(true);

    // 提交新的 is_primary=true ⇒ create_customer_contact 事务内先清旧主联系人
    const second = await apiCall<ContactRow>(
      page,
      'POST',
      `/crm/customers/${customerId}/contacts`,
      { name: 'E2E 乙联系人', phone: '13800138003', is_primary: true }
    );
    expect(second.data.is_primary).toBe(true);

    const afterSecond = await listContacts(page, customerId);
    const rowFirst = afterSecond.find(item => item.id === first.data.id);
    const rowSecond = afterSecond.find(item => item.id === second.data.id);
    expect(rowFirst, '列表应含甲联系人').toBeTruthy();
    expect(rowSecond, '列表应含乙联系人').toBeTruthy();
    expect(rowFirst!.is_primary, `每客户最多一个主联系人：甲应被降为 false`).toBe(false);
    expect(rowSecond!.is_primary).toBe(true);
    // list_customer_contacts 排序口径：is_primary desc, name asc（services/customer_ops/contact.rs:31-32）
    const primaries = afterSecond.filter(item => item.is_primary);
    expect(primaries, '主联系人应唯一').toHaveLength(1);
    expect(afterSecond[0].id, '主联系人应排在列表首位').toBe(second.data.id);

    // 精确删除乙：甲必须仍在，且删除不会把主联系人身份回补给甲
    const deleted = await apiCall<null>(
      page,
      'DELETE',
      `/crm/customers/${customerId}/contacts/${second.data.id}`
    );
    expect(deleted.message, '删除成功文案由 crm_customer_handler.rs:521-524 固定').toBe(
      '联系人删除成功'
    );

    const afterDelete = await listContacts(page, customerId);
    expect(
      afterDelete.filter(item => item.id === second.data.id),
      '已删联系人不得再出现在列表'
    ).toHaveLength(0);
    const surviving = afterDelete.find(item => item.id === first.data.id);
    expect(surviving, '同客户其他联系人不应被连带删除').toBeTruthy();
    expect(surviving!.is_primary, '删除主联系人不会自动把甲升回主').toBe(false);
    expect(surviving!.phone, '删除动作不得改动留存列的值').toBe('13800138002');
  });

  test('07-05 销售目标创建落 dimension_name 拼接串，PUT 按 period 定位改总额并覆盖备注', async ({
    page,
  }) => {
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
    const period = genCode('E2E-TGT');
    const startDate = '2026-11-01';
    const endDate = '2026-11-30';

    const created = await apiCall<SalesTargetRow>(page, 'POST', '/crm/sales-analysis/targets', {
      target_type: 'overall',
      target_id: me.id,
      period,
      target_amount: 123456,
      start_date: startDate,
      end_date: endDate,
    });
    expect(created.data.statistic_type, '目标行按 statistic_type=target 归类').toBe('target');
    expect(created.data.period, 'period 应原样落库').toBe(period);
    expect(created.data.dimension_type, 'target_type 写入 dimension_type').toBe('overall');
    expect(created.data.dimension_id, 'target_id 写入 dimension_id').toBe(me.id);
    expect(
      created.data.dimension_name,
      'create_target 把起止日期拼成固定中文串存 dimension_name（services/sales_analysis_service.rs:297-300）'
    ).toBe(`开始日期: ${startDate}, 结束日期: ${endDate}`);
    expect(
      typeof created.data.total_amount,
      `DECIMAL 出参应是字符串（未开 serde-float），实际 ${JSON.stringify(created.data.total_amount)}`
    ).toBe('string');
    expect(Number(created.data.total_amount), 'total_amount 数值应等于提交值').toBe(123456);

    // 回读：GET /crm/sales-analysis/targets 按 id 倒序分页，新建行必在首页
    const listed = await apiCallRaw<SalesTargetRow[]>(
      page,
      'GET',
      '/crm/sales-analysis/targets?page=1&page_size=100'
    );
    expect(Array.isArray(listed), 'GET targets 出参应为裸数组').toBe(true);
    const row = listed.find(item => item.id === created.data.id);
    expect(row, '列表回读应含刚建的目标行').toBeTruthy();
    expect(row!.period).toBe(period);
    expect(Number(row!.total_amount)).toBe(123456);
    expect(row!.dimension_name).toBe(`开始日期: ${startDate}, 结束日期: ${endDate}`);

    // PUT 按 period 定位；未提交 status ⇒ 回落 master_data::ACTIVE="active"
    const updated = await apiCall<SalesTargetDto>(
      page,
      'PUT',
      `/crm/sales-analysis/targets/${created.data.period}`,
      { target_amount: 654321, remarks: '改后目标备注' }
    );
    expect(updated.data.id, 'PUT 命中的应是同一条目标行').toBe(created.data.id);
    expect(updated.data.period, 'PUT 出参 period 应等于路径定位键').toBe(period);
    expect(Number(updated.data.target_amount), '提交的 target_amount 应落库').toBe(654321);
    expect(updated.data.status, '未提交 status 时回落 models/status/general.rs:71 ACTIVE').toBe(
      'active'
    );
    // 既成出参事实（非业务正确性背书）：update_target 把 actual_amount 直接取 total_amount，
    // completion_rate 于是恒为自身比自身=100、variance 恒为 0，与真实成交无关。
    // 这条缺口按既存缺陷登记并交调查与复审取证，用例只钉当前线格式。
    expect(Number(updated.data.actual_amount), 'actual_amount 当前实现等于 total_amount').toBe(
      654321
    );
    expect(Number(updated.data.completion_rate), 'completion_rate 当前恒为 100').toBe(100);
    expect(Number(updated.data.variance), 'variance 当前恒为 0').toBe(0);
    expect(
      typeof updated.data.completion_rate,
      'SalesTargetDto 的 DECIMAL 字段同样应是序列化字符串'
    ).toBe('string');

    const afterUpdate = (
      await apiCallRaw<SalesTargetRow[]>(
        page,
        'GET',
        '/crm/sales-analysis/targets?page=1&page_size=100'
      )
    ).find(item => item.id === created.data.id);
    expect(afterUpdate, 'PUT 后回读应仍含该目标行').toBeTruthy();
    expect(Number(afterUpdate!.total_amount), '回读 total_amount 应为改后值').toBe(654321);
    expect(
      afterUpdate!.dimension_name,
      'update_target 用 remarks 覆盖 dimension_name（service:489），原起止日期串不再存在'
    ).toBe('改后目标备注');
    expect(afterUpdate!.statistic_type).toBe('target');
  });
});
