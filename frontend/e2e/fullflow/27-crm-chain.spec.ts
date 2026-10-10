// CRM 域全流程契约级 E2E — 27 线索→客户→商机→赢单/输单→销售订单（数据真实落库与状态流转回读）
//
// **声明：本文件未在本地实跑（本机禁跑 Playwright），仅按后端源码契约编写，待 CI/联调验证。**
//
// 本链证明什么（覆盖此前只测"页面能打开/toast"的 CRM 链）：
//   1) 线索创建：单号服务端生成（前缀 LD，services/crm/lead.rs:41-51）、lead_source 缺省
//      "OTHER"（lead.rs:52）、owner_id=登录人（lead.rs:53/:87）；lead_status 词表门禁
//      （lead.rs:514-526 ensure_valid_lead_status，词表 models/status/bpm_crm_contract.rs:119-143
//      小写 new/contacted/qualified/assigned/converted/pool/lost）——非法取值 400 VALIDATION_ERROR，
//      不存在线索 404。
//   2) 线索转化"一转三写"（lead.rs `convert_lead_to_customer`：建客户 + mark_lead_converted
//      + 自动建商机，同事务）：lead→converted+converted_customer_id+converted_at；
//      新客 status=active（词表 general.rs:53）/source='lead'/customer_type='other'
//      ——customer_type 是**渠道**列，线索转化时渠道未知，写入点 lead.rs
//      `build_customer_active` 的缺省分支补 `constants::customer_type::OTHER`；钉 'other' 而非
//      分层词的理由：potential 属 CLV 分层 segment 词表（champion/loyal/potential/at_risk/lost），
//      混维写进渠道列就是缺陷值，读侧按渠道精确匹配永不命中，故本链只能钉渠道 token；
//      自动生成"初步接洽"商机 stage=QUALIFICATION、status=OPEN、
//      赢率 20（词表 bpm_crm_contract.rs:153-187 大写）；
//      重复转化被状态门拒绝（lead.rs `validate_lead_for_conversion` BUSINESS_ERROR）。
//   3) 商机创建门：客户不存在 404（services/crm/opp.rs:75-78）、阶段越表 400 VALIDATION
//      （opp.rs:56-63）；阶段流转机（opp.rs:278-305）非法跳转 400 BUSINESS、合法流转自动重算
//      默认赢率（opp.rs:394-401 + :41-47：NEEDS_ANALYSIS=25）。
//   4) 输单关单（opp.rs:589-635）：原因必填（空/超500 → VALIDATION_ERROR）、CLOSED_LOST 落
//      status+stage、赢率归 0、actual_close_date=当日、lost_reason 回读；重复关单与关单后
//      修改均 400 BUSINESS（opp.rs:613-615/:336-346）。
//   5) 赢单转销售订单（opp.rs:452-585）：draft 订单落库（status='draft'，词表
//      models/status/sales.rs:14；写入点 opp.rs:521）total_amount=商机预估金额、
//      opportunity_id/customer_id 外键回读；商机 CLOSED_WON + actual_amount 迁移 +
//      赢率 100（opp.rs:563-574）；重复转化 400 BUSINESS（opp.rs:480-483）、
//      赢单商机不可删除 400 BUSINESS（opp.rs:437-438）。
//   6) 公海状态机（handlers/crm_pool_handler.rs:170-212/:130-168）：回收→lead_status=pool
//      并出现在 /crm/pool 列表；领取→回到 new；重复回收/领非公海 400 BUSINESS。
//      公海规则（每日领取上限/最大持有数/保护期）对**两条领取入口统一生效**
//      （services/crm/pool.rs，用户 2026-10-02 拍板 ①）：保护期判据为领取事件列
//      last_claimed_at（回收刷新 updated_at 不再误伤"回收→立即领取"合法链），
//      原领取人本人重领豁免（last_claimed_by 判定，保护期本义防他人抢单）。
//
// CI 测不到（显式声明）：
//   - 他人（不同用户）在保护期内抢领他人公海行的负例：本链为单账号会话，
//     无第二领取人上下文，由后端契约测试覆盖（services_crm_pool_claim_rules_test.rs A 组）；
//   - 线索评分/RFM/CLV/漏斗等只读分析端点（纯聚合，无落库流转可钉）；
//   - 商机阶段停留记录 stage-change（独立表追加写，与主状态机正交）。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  tryCleanup,
  APP_ERROR_CODES,
} from '../flow/helpers';
import { pickListArray } from '../flow/ui-helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** AppError 机器码取源 flow/helpers 的 APP_ERROR_CODES（NOT_FOUND 登记依据 utils/error.rs:739） */
const ERR_NOT_FOUND = APP_ERROR_CODES.NOT_FOUND;

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0)
    throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

/**
 * 线索/商机列表恒出 {data,total,page,page_size}（services/crm/lead.rs:163-168、
 * services/crm/opp.rs:181-186 手搓 json!，非 PaginatedResponse 的 items 键）。
 * 用 pickListArray 单形状直读，信封漂移当场红。
 */
function listData(res: unknown, context: string): Record<string, unknown>[] {
  return pickListArray<Record<string, unknown>>(res, 'data', context);
}

function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

/** API 建一条带预估金额/需求描述的线索，返回 id 与响应体 */
async function seedLead(
  page: Page,
  tag: string
): Promise<{ id: number; lead: Record<string, unknown> }> {
  const marker = genCode(tag);
  const lead = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/crm/leads', {
    company_name: `E2E纺城${marker}`,
    contact_name: `联系人${marker}`,
    mobile_phone: '13800000001',
    estimated_amount: '5000',
    requirement_desc: `E2E需求-${marker}`,
    expected_delivery_date: todayStr(),
  });
  const id = requireNum(lead.id, '建线索');
  CLEANUP.push({ path: `/crm/leads/${id}`, label: `crm_lead#${id}` });
  return { id, lead };
}

/** 建客户（本用例自用，不消耗全局种子行），返回 id */
async function seedCustomer(page: Page, tag: string): Promise<number> {
  const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/crm/customers', {
    customer_name: `E2E商机客户${genCode(tag)}`,
  });
  const id = requireNum(created.id, '建客户');
  CLEANUP.push({ path: `/crm/customers/${id}`, label: `customer#${id}` });
  return id;
}

/** 建 OPEN 商机（走服务端取号，禁止手输单号） */
async function seedOpportunity(
  page: Page,
  customerId: number,
  estimatedAmount: string
): Promise<Record<string, unknown>> {
  const opp = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/crm/opportunities', {
    opportunity_name: `E2E商机${genCode('F27')}`,
    customer_id: customerId,
    estimated_amount: estimatedAmount,
    expected_close_date: todayStr(),
  });
  const id = requireNum(opp.id, '建商机');
  CLEANUP.push({ path: `/crm/opportunities/${id}`, label: `crm_opportunity#${id}` });
  return opp;
}

test.describe('27 CRM 线索→商机→订单契约链', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await ensureTestEntities(page);
  });

  test('27-01 线索创建回读 + 状态词表门禁（非法 400 VALIDATION / 不存在 404）', async ({
    page,
  }) => {
    const ctx = getCtx();
    const { id, lead } = await seedLead(page, 'F27A');

    // 回读：单号服务端生成 + 缺省来源 + 归属人=登录人（lead.rs:41-53）
    expect(String(lead.lead_no ?? ''), 'lead_no 前缀 LD（服务端取号，禁手输的镜像钉）').toMatch(
      /^LD/
    );
    expect(lead.lead_source, '缺省 lead_source=OTHER（lead.rs:52）').toBe('OTHER');
    expect(Number(lead.owner_id), 'owner_id=当前用户').toBe(ctx.userIds[0]);

    const back = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/crm/leads/${id}`);
    expect(String(back.company_name), '公司名落库').toBe(String(lead.company_name));
    // estimated_amount 为 Decimal 序列化的十进制字符串（models/crm_lead.rs:64）
    expect(Number(back.estimated_amount), '预估金额 5000 落库').toBe(5000);
    expect(
      back.converted_customer_id ?? null,
      '未转化前 converted_customer_id 必须 NULL'
    ).toBeNull();

    // 词表内流转：new 家族 contacted（bpm_crm_contract.rs:124）
    await apiCall(page, 'PUT', `/crm/leads/${id}/status`, { status: 'contacted' });
    const afterStatus = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/crm/leads/${id}`);
    expect(afterStatus.lead_status, '状态回读 contacted（小写词表）').toBe('contacted');

    // 负例 1：词表外取值 → 400 VALIDATION_ERROR（lead.rs:517-526）
    const bad = await apiCallExpectFail(page, 'PUT', `/crm/leads/${id}/status`, {
      status: 'CONTACTED_UPPER',
    });
    expect(bad.status, '词表外状态应 400').toBe(400);
    expect(failureCode(bad), '非法枚举机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const unchanged = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/crm/leads/${id}`);
    expect(unchanged.lead_status, '被拒后状态零漂移').toBe('contacted');

    // 负例 2：不存在的线索改状态 → 404
    const missing = await apiCallExpectFail(page, 'PUT', '/crm/leads/99999999/status', {
      status: 'new',
    });
    expect(missing.status, '不存在线索应 404').toBe(404);
    expect(failureCode(missing), '404 机器码').toBe(ERR_NOT_FOUND);

    // 建单入口同门禁（lead.rs:60-65）
    const badCreate = await apiCallExpectFail(page, 'POST', '/crm/leads', {
      company_name: `E2E非法${genCode('F27B')}`,
      lead_status: 'bogus',
    });
    expect(badCreate.status, '创建带非法状态应 400').toBe(400);
    expect(failureCode(badCreate), '创建非法状态机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
  });

  test('27-02 线索转化"一转三写"：lead/customer/opportunity 全回读 + 重复转化 400 BUSINESS', async ({
    page,
  }) => {
    const ctx = getCtx();
    const { id: leadId, lead } = await seedLead(page, 'F27C');

    const conv = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/crm/leads/${leadId}/convert`,
      {
        notes: 'E2F27 转化备注',
      }
    );
    const customerId = requireNum(conv.customer_id, '转化返回客户 id');
    CLEANUP.push({
      path: `/crm/customers/${customerId}`,
      label: `customer(lead convert)#${customerId}`,
    });

    // 1) 线索回读：converted（bpm_crm_contract.rs:133）+ 外键 + 时间戳（lead.rs:620-639）
    const leadBack = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/crm/leads/${leadId}`);
    expect(leadBack.lead_status, '转化后 lead_status=converted').toBe('converted');
    expect(Number(leadBack.converted_customer_id), 'converted_customer_id 指向新客户').toBe(
      customerId
    );
    expect(leadBack.converted_at, 'converted_at 必须真实写入').toBeTruthy();

    // 2) 客户回读：active 词表（general.rs:53）/source=lead/customer_type=other
    //    （写入点 lead.rs `build_customer_active`，渠道列缺省 = constants::customer_type::OTHER）
    const cust = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/customers/${customerId}`
    );
    expect(cust.status, '转化客户默认 active（小写主数据词表）').toBe('active');
    expect(String(cust.customer_code ?? ''), '客户编码服务端取号 CUS 前缀').toMatch(/^CUS/);
    expect(cust.customer_name, '客户名继承线索公司名').toBe(String(lead.company_name));
    expect(cust.source, '来源标记 lead（lead.rs build_customer_active 的 source 字段）').toBe(
      'lead'
    );
    expect(
      cust.customer_type,
      '转化缺省渠道 other（lead.rs build_customer_active 缺省分支写 constants::customer_type::OTHER）'
    ).toBe('other');
    expect(
      cust.notes,
      'notes 取请求 notes（lead.rs build_customer_active 的 notes 字段：req.notes 优先）'
    ).toBe('E2F27 转化备注');
    expect(Number(cust.created_by ?? 0), 'created_by=操作人').toBe(ctx.userIds[0]);

    // 3) 自动商机回读：QUALIFICATION/OPEN/赢率20（lead.rs:649-673；大写词表 bpm_crm_contract.rs:153-187）
    const oppList = listData(
      await apiCallRaw<unknown>(page, 'GET', '/crm/opportunities?page=1&page_size=100'),
      '27-02 转化后商机列表'
    );
    const autoOpp = oppList.find(o => Number(o.lead_id) === leadId);
    expect(
      autoOpp,
      `应存在 lead_id=${leadId} 的自动商机，实际=${JSON.stringify(oppList).slice(0, 300)}`
    ).toBeTruthy();
    expect(autoOpp!.opportunity_stage, '自动商机阶段 QUALIFICATION').toBe('QUALIFICATION');
    expect(autoOpp!.opportunity_status, '自动商机状态 OPEN').toBe('OPEN');
    expect(Number(autoOpp!.win_probability), '初步接洽默认赢率 20（lead.rs:658）').toBe(20);
    expect(Number(autoOpp!.estimated_amount), '商机预估金额继承线索（lead.rs:659）').toBe(5000);
    expect(Number(autoOpp!.customer_id), '商机挂在转化客户下').toBe(customerId);
    expect(String(autoOpp!.opportunity_no ?? ''), '商机号服务端取号 OPP 前缀').toMatch(/^OPP/);
    CLEANUP.push({
      path: `/crm/opportunities/${requireNum(autoOpp!.id, '自动商机 id')}`,
      label: 'crm_opportunity(auto convert)',
    });

    // 4) 负例：已转化线索重复转化 → 400 BUSINESS（lead.rs:562-564，business() 文案脱敏只断码）
    const again = await apiCallExpectFail(page, 'POST', `/crm/leads/${leadId}/convert`, {});
    expect(again.status, '重复转化应 400').toBe(400);
    expect(failureCode(again), '重复转化机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });

  test('27-03 商机创建门与阶段流转机：引用不存在 404 / 非法阶段 VALIDATION / 跳阶段 BUSINESS / 赢率自动重算', async ({
    page,
  }) => {
    // 负例 1：customer_id 不存在 → 404（opp.rs:75-78）
    const missingCust = await apiCallExpectFail(page, 'POST', '/crm/opportunities', {
      opportunity_name: `E2E幽灵商机${genCode('F27D')}`,
      customer_id: 99999999,
    });
    expect(missingCust.status, '引用不存在客户应 404').toBe(404);
    expect(failureCode(missingCust), '引用缺失机器码').toBe(ERR_NOT_FOUND);

    const customerId = await seedCustomer(page, 'F27E');

    // 负例 2：阶段越表 → 400 VALIDATION（opp.rs:56-63，取值源 ALL_STAGES）
    const badStage = await apiCallExpectFail(page, 'POST', '/crm/opportunities', {
      opportunity_name: `E2E非法阶段${genCode('F27F')}`,
      customer_id: customerId,
      opportunity_stage: 'QUALIFED_TYPO',
    });
    expect(badStage.status, '非法阶段应 400').toBe(400);
    expect(failureCode(badStage), '非法阶段机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    const opp = await seedOpportunity(page, customerId, '1000');
    const oppId = requireNum(opp.id, '商机 id');
    expect(opp.opportunity_stage, '缺省阶段 QUALIFICATION').toBe('QUALIFICATION');
    expect(Number(opp.win_probability), '建单自动赢率 10（opp.rs:41 QUALIFICATION）').toBe(10);

    // 负例 3：跳阶段 QUALIFICATION→NEGOTIATION 非法（opp.rs:285-296）→ 400 BUSINESS
    const skip = await apiCallExpectFail(page, 'PUT', `/crm/opportunities/${oppId}`, {
      opportunity_stage: 'NEGOTIATION',
    });
    expect(skip.status, '跳阶段应 400').toBe(400);
    expect(failureCode(skip), '跳阶段机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const unchanged = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/opportunities/${oppId}`
    );
    expect(unchanged.opportunity_stage, '被拒后阶段零漂移').toBe('QUALIFICATION');

    // 正向：QUALIFICATION→NEEDS_ANALYSIS 合法，未显式传赢率时自动重算为 25（opp.rs:394-401 + :42）
    await apiCall(page, 'PUT', `/crm/opportunities/${oppId}`, {
      opportunity_stage: 'NEEDS_ANALYSIS',
    });
    const moved = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/opportunities/${oppId}`
    );
    expect(moved.opportunity_stage, '合法流转落库').toBe('NEEDS_ANALYSIS');
    expect(Number(moved.win_probability), '阶段联动赢率自动 25').toBe(25);
  });

  test('27-04 输单关单契约：空原因 VALIDATION / CLOSED_LOST 全字段回读 / 重复关单与关单后修改 BUSINESS', async ({
    page,
  }) => {
    const customerId = await seedCustomer(page, 'F27G');
    const opp = await seedOpportunity(page, customerId, '2000');
    const oppId = requireNum(opp.id, '商机 id');

    // 负例：原因空串 → 400 VALIDATION（opp.rs:595-599）
    const emptyReason = await apiCallExpectFail(
      page,
      'POST',
      `/crm/opportunities/${oppId}/close-lost`,
      { lost_reason: '   ' }
    );
    expect(emptyReason.status, '空流失原因应 400').toBe(400);
    expect(failureCode(emptyReason), '空原因机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const beforeClose = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/opportunities/${oppId}`
    );
    expect(beforeClose.opportunity_status, '被拒后仍 OPEN').toBe('OPEN');

    // 正向关单：status+stage=CLOSED_LOST、赢率 0、actual_close_date=今天、lost_reason 回读
    const reason = `E2E价格因素流失-${genCode('F27H')}`;
    await apiCall(page, 'POST', `/crm/opportunities/${oppId}/close-lost`, { lost_reason: reason });
    const lost = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/opportunities/${oppId}`
    );
    // 词表 bpm_crm_contract.rs:161 CLOSED_LOST（status 与 stage 同值，opp.rs:619-620）
    expect(lost.opportunity_status, '输单后状态 CLOSED_LOST').toBe('CLOSED_LOST');
    expect(lost.opportunity_stage, '输单后阶段 CLOSED_LOST').toBe('CLOSED_LOST');
    expect(Number(lost.win_probability), '输单赢率归 0（opp.rs:621）').toBe(0);
    expect(String(lost.actual_close_date ?? '').slice(0, 10), '实际关闭日=当日（opp.rs:623）').toBe(
      todayStr()
    );
    expect(lost.lost_reason, '流失原因真实落库').toBe(reason);

    // 负例：重复关单 → BUSINESS（opp.rs:613-615）；关单后修改 → BUSINESS（opp.rs:336-346）
    const again = await apiCallExpectFail(page, 'POST', `/crm/opportunities/${oppId}/close-lost`, {
      lost_reason: 'E2E重复关单负例',
    });
    expect(again.status, '重复关单应 400').toBe(400);
    expect(failureCode(again), '重复关单机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const editClosed = await apiCallExpectFail(page, 'PUT', `/crm/opportunities/${oppId}`, {
      opportunity_name: 'E2E篡改已关闭商机',
    });
    expect(editClosed.status, '关闭后修改应 400').toBe(400);
    expect(failureCode(editClosed), '关闭后修改机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });

  test('27-05 赢单转化销售订单：draft 订单外键/金额回读 + 商机 CLOSED_WON + 重复转化与删除赢单商机被拒', async ({
    page,
  }) => {
    const customerId = await seedCustomer(page, 'F27I');
    const opp = await seedOpportunity(page, customerId, '8000');
    const oppId = requireNum(opp.id, '商机 id');

    const res = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/crm/opportunities/${oppId}/convert`
    );
    const orderId = requireNum(res.order_id, '转化生成的销售订单 id');
    CLEANUP.push({ path: `/sales/orders/${orderId}`, label: `sales_order#${orderId}` });
    expect(String(res.order_no ?? ''), '订单号服务端取号（SO 前缀，opp.rs:496-509）').toMatch(
      /^SO/
    );

    // 订单回读：draft（词表 models/status/sales.rs:14，写入点 opp.rs:521）、外键、金额、余额=总额
    const order = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/sales/orders/${orderId}`
    );
    expect(order.status, '商机转来的订单必须 draft').toBe('draft');
    expect(Number(order.customer_id), '订单客户=商机关联客户').toBe(customerId);
    expect(Number(order.opportunity_id), '订单回写 opportunity_id 外键（opp.rs:517）').toBe(oppId);
    expect(Number(order.total_amount), '订单总额=商机预估金额').toBe(8000);
    expect(Number(order.balance_amount), '余额=总额（opp.rs:528）').toBe(8000);

    // 商机回读：双字段 CLOSED_WON、actual_amount=estimated、赢率 100（opp.rs:563-574）
    const won = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/opportunities/${oppId}`
    );
    expect(won.opportunity_status, '赢单后状态 CLOSED_WON（bpm_crm_contract.rs:158）').toBe(
      'CLOSED_WON'
    );
    expect(won.opportunity_stage, '赢单后阶段 CLOSED_WON').toBe('CLOSED_WON');
    expect(Number(won.actual_amount), '预估金额迁移到 actual_amount').toBe(8000);
    expect(Number(won.win_probability), '赢单赢率 100（opp.rs:566）').toBe(100);
    expect(won.actual_close_date, '赢单关闭日写入').toBeTruthy();

    // 负例：重复转化 → BUSINESS（opp.rs:480-483）；赢单商机删除 → BUSINESS（opp.rs:437-438）
    const again = await apiCallExpectFail(page, 'POST', `/crm/opportunities/${oppId}/convert`);
    expect(again.status, '重复转化应 400').toBe(400);
    expect(failureCode(again), '重复转化机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const delWon = await apiCallExpectFail(page, 'DELETE', `/crm/opportunities/${oppId}`);
    expect(delWon.status, '赢单商机不可删除应 400').toBe(400);
    expect(failureCode(delWon), '赢单不可删机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const stillThere = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/opportunities/${oppId}`
    );
    expect(stillThere.opportunity_status, '删除被拒后商机仍在且仍 CLOSED_WON').toBe('CLOSED_WON');
  });

  test('27-06 公海回收/领取状态机：pool 列表真实入池出池 + 重复回收与领取非公海线索被拒', async ({
    page,
  }) => {
    const ctx = getCtx();
    const { id: leadId } = await seedLead(page, 'F27J');

    // 回收 → lead_status=pool（词表 bpm_crm_contract.rs:136；写入点 crm_pool_handler.rs:186-195）
    await apiCall(page, 'POST', '/crm/pool/recycle', { lead_id: leadId, reason: 'E2E回收' });
    const recycled = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/crm/leads/${leadId}`);
    expect(recycled.lead_status, '回收后状态 pool（小写词表）').toBe('pool');

    // 公海列表真实出现（handler 出 {items,...}：crm_pool_handler.rs:121-126，'items' 单形状）
    const poolItems = pickListArray<Record<string, unknown>>(
      await apiCallRaw<unknown>(page, 'GET', '/crm/pool?page=1&page_size=100'),
      'items',
      '27-06 公海列表'
    );
    const inPool = poolItems.find(it => Number(it.id) === leadId);
    expect(inPool, `线索 ${leadId} 应出现在 /crm/pool 列表`).toBeTruthy();
    expect(String(inPool!.lead_no ?? ''), '公海行携带真实线索号').toBe(String(recycled.lead_no));

    // 领取（两条入口统一公海规则校验后，/pool/claim 与 /pool/{id}/claim 同判据；
    // 本链"回收→立即领取"合法的依据：存量/领取事件列 last_claimed_at 判保护期，
    // 回收刷新的 updated_at 不再参与判定——services/crm/pool.rs 头注释）
    await apiCall(page, 'POST', '/crm/pool/claim', { lead_id: leadId });
    const claimed = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/crm/leads/${leadId}`);
    expect(claimed.lead_status, '领取后状态 new（bpm_crm_contract.rs:121）').toBe('new');
    // 领取事件列由唯一归属实现 build_claimed_active 落库（保护期/每日计数判据）
    expect(
      claimed.last_claimed_at,
      '领取必须写 last_claimed_at（领取事件唯一记录点）'
    ).toBeTruthy();
    expect(
      Number(claimed.last_claimed_by),
      'last_claimed_by=领取人（原领取人本人重领豁免的依据）'
    ).toBe(ctx.userIds[0]);

    // 负例 1：线索回到 new 后再回收合法（200），随后重复回收撞"已在公海"门 → BUSINESS
    const recycleAgain = await apiCall(page, 'POST', '/crm/pool/recycle', { lead_id: leadId });
    expect(recycleAgain.code, 'new 状态再回收应成功').toBe(200);
    const dup = await apiCallExpectFail(page, 'POST', '/crm/pool/recycle', { lead_id: leadId });
    expect(dup.status, '重复回收应 400').toBe(400);
    expect(failureCode(dup), '重复回收机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 出池恢复现场：改用**批量定向入口** `/pool/{id}/claim`（与 /pool/claim 同一套
    // 校验+同一归属实现）。刚回收的行 updated_at=刚刚，旧判据下此步必被 7 天保护期
    // 静默判负（claimed=0→400）；新判据下成立——本会话就是原领取人（last_claimed_by
    // 命中，本人重领豁免），两条路径语义统一在此钉死。
    const batchClaim = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/crm/pool/${leadId}/claim`
    );
    // apiCallRaw 解出的即 handler data 层 {claimed:n}（claim_specific 出参形状）
    expect(Number(batchClaim.claimed), '定向领取应成功 1 条（批量路径校验+归属与单条同源）').toBe(
      1
    );
    const reclaimed = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/leads/${leadId}`
    );
    expect(reclaimed.lead_status, '定向领取（批量路径）后状态 new').toBe('new');

    // 负例 2：领取非池状态线索 → 400 BUSINESS（handler :141-143 "该客户不在公海中"）
    const notInPool = await apiCallExpectFail(page, 'POST', '/crm/pool/claim', { lead_id: leadId });
    expect(notInPool.status, '领取非公海线索应 400').toBe(400);
    expect(failureCode(notInPool), '领取门机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const finalState = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/leads/${leadId}`
    );
    expect(finalState.lead_status, '被拒后状态零漂移（仍 new）').toBe('new');
  });
});
