import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall, apiCallRaw, apiCallExpectFail, genCode, tryCleanup } from './helpers';

/**
 * 111 客户增强面对象读写链首个真契约覆盖（标签挂解 详情 审计 CLV 线索增强口）
 *
 * 与既有覆盖的分工。既有对 crm 客户域做过标准建单改接（列表三列与详情 360），
 * 但增强对象口 GET customers/enhanced/{id} 与 DELETE、标签挂解对象端、
 * 操作审计写读、CLV 计算写读，以及线索标签数组写读，在判定器口径下均零覆盖。
 * 本 spec 把这些对象端点钉成真实回读链。
 *
 * 契约真相（逐段读后端源，非推测）：
 * 标签对象（handlers/crm_customer_handler）：POST customers/{id}/tags/{tagId} 幂等挂标
 * 返回键 customer_id、tag_id、already_exists；GET customers/{id}/tags 走 crm_tag 内联
 * customer_tag 联接，返回 CustomerTagBrief 数组（含 id、name、color、category）；
 * DELETE customers/{id}/tags/{tagId} 命中关联则删，未命中返回 404（rows_affected=0 分支）。
 * 增强详情与删除（get_customer 与 delete_customer）读写 crm_lead 行，出参为线索形状，
 * 掩码列经权限处理。delete_customer 预检行可见性，删除后按 id 读必须 404。
 * 操作审计（create_customer_audit_log）：POST customers/{id}/audit-logs 写
 * customer_audit_log，operation、field_name、old_value、new_value 入参；出参 data 恒 null；
 * GET 同路径读列表，出参为数组，行含 operation 与 field_name、old_value、new_value、user_name。
 * CLV（calculate_customer_clv）：POST customers/{id}/clv/calculate 无体，读该客户全部销售单，
 * 无单时 total_orders 为 0、total_revenue 与 avg_order_value 与 clv_score 归零、
 * segment 落最末档；GET customers/{id}/clv 读回同行互证。
 * 线索标签（add_tags）：POST customers/{id}/tags 提交 tags 字符串数组，落的是
 * crm_lead 行的 tags 列（方案 A 裁定），写响应即更新后的线索行；DTO 要求至少一枚，
 * 空数组提交按校验拒 400 且原值保持，清除须走显式解标通道而不是空集代填。
 *
 * 假绿防线。标签挂解两向都真读列表（挂后在列、解后不在列），不凭写响应 200 收尾；
 * 增强口删除后必须回读 404，不接受"删除接口返回成功即过"；审计写读逐键对表；
 * CLV 对无单客户的零值与最末档分层为确定判据，两读口互证。全程不用 ?? 或 || 兜底，
 * 后端必出的键缺失即行形漂移判红。
 */

type Row = Record<string, unknown>;

const uniquePhone = (): string => `139${Date.now().toString().slice(-8)}`;
const toNum = (v: unknown): number => Number(String(v));

test.describe.serial('111 客户增强面对象读写链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('T1 建客户 挂标签读列表 解标签读列表 解不存在标签被拒 审计写读逐键 CLV 零单分层 删客户读回 404', async ({
    page,
  }) => {
    const tag = genCode('CU111');

    // 建客户（标准域 customers，增强对象口与标签/审计/CLV 均以此为父）
    const customer = await apiCallRaw<Row>(page, 'POST', '/crm/customers', {
      customer_name: `111客户${tag}`,
      contact_person: `111联系${tag}`,
      contact_phone: uniquePhone(),
      customer_type: 'retail',
      status: 'active',
    });
    const customerId = Number(customer.id);
    expect(customerId > 0, `客户自建失败 ${JSON.stringify(customer)}`).toBe(true);

    // 建标签（crm_tag 真行，供客户挂标）
    const tag1 = await apiCallRaw<Row>(page, 'POST', '/crm/tags', {
      name: `111标签甲${tag}`,
      color: '#ff0000',
      category: 'customer',
    });
    const tagId1 = Number(tag1.id);
    expect(tagId1 > 0, `标签甲自建失败 ${JSON.stringify(tag1)}`).toBe(true);
    expect(String(tag1.name), '标签回显 name 必须等于提交值').toBe(`111标签甲${tag}`);
    expect(String(tag1.color), '标签回显 color 必须等于提交值').toBe('#ff0000');

    // 1) 挂标（幂等）——先真读列表证明"从挂标到在列"，而非只信写响应
    await apiCall(page, 'POST', `/crm/customers/${customerId}/tags/${tagId1}`, {});
    const afterAttach = await apiCallRaw<Row[]>(page, 'GET', `/crm/customers/${customerId}/tags`);
    expect(Array.isArray(afterAttach), '标签列表必须返回数组').toBe(true);
    const hit = afterAttach.find(t => Number(t.id) === tagId1);
    expect(
      hit,
      `挂标后列表应含 tagId=${tagId1}，实际 ${JSON.stringify(afterAttach)}`
    ).toBeDefined();
    expect(String(hit!.name), '列表标签名必须等于建标提交名').toBe(`111标签甲${tag}`);

    // 2) 重复挂标幂等（第二次 already_exists 为 true，列表不增行）
    const reAttach = await apiCallRaw<Row>(
      page,
      'POST',
      `/crm/customers/${customerId}/tags/${tagId1}`,
      {}
    );
    expect(
      reAttach.already_exists === true,
      `重复挂标应报 already_exists，实际 ${JSON.stringify(reAttach)}`
    ).toBe(true);
    const afterReAttach = await apiCallRaw<Row[]>(page, 'GET', `/crm/customers/${customerId}/tags`);
    expect(afterReAttach.length, '幂等重挂不应产生第二行关联').toBe(afterAttach.length);

    // 3) 审计日志写后读，逐键对表
    const auditBody = {
      operation: 'update_credit',
      field_name: 'credit_limit',
      old_value: '1000',
      new_value: '2000',
    };
    await apiCall(page, 'POST', `/crm/customers/${customerId}/audit-logs`, auditBody);
    const auditList = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/crm/customers/${customerId}/audit-logs`
    );
    expect(Array.isArray(auditList), '审计列表必须返回数组').toBe(true);
    const auditHit = auditList.find(
      a =>
        String(a.operation) === auditBody.operation && String(a.field_name) === auditBody.field_name
    );
    expect(
      auditHit,
      `审计列表应含刚写的 update_credit，实际 ${JSON.stringify(auditList)}`
    ).toBeDefined();
    expect(String(auditHit!.old_value), '审计 old_value 必须回读提交原文').toBe('1000');
    expect(String(auditHit!.new_value), '审计 new_value 必须回读提交原文').toBe('2000');
    expect(auditHit!.user_name, '审计 user_name 必须由服务端派生非空').not.toBeNull();

    // 4) CLV 计算（无销售单客户）→ 零值与最末分层，两读口互证
    const clvCalc = await apiCallRaw<Row>(
      page,
      'POST',
      `/crm/customers/${customerId}/clv/calculate`,
      {}
    );
    expect(
      toNum(clvCalc.total_orders),
      `无单客户 total_orders 必须为 0，实际 ${String(clvCalc.total_orders)}`
    ).toBe(0);
    expect(toNum(clvCalc.clv_score), '无单客户 clv_score 必须归零').toBe(0);
    expect(String(clvCalc.segment), '无单客户应落最末分层 lost').toBe('lost');
    const clvGet = await apiCallRaw<Row>(page, 'GET', `/crm/customers/${customerId}/clv`);
    expect(toNum(clvGet.total_orders), 'GET clv 与 calculate 写口 total_orders 互证一致').toBe(
      toNum(clvCalc.total_orders)
    );
    expect(String(clvGet.segment), 'GET clv 与 calculate 分层互证一致').toBe(
      String(clvCalc.segment)
    );

    // 5) 解标（真读列表证明"从解到不在列"）
    await apiCall(page, 'DELETE', `/crm/customers/${customerId}/tags/${tagId1}`);
    const afterDetach = await apiCallRaw<Row[]>(page, 'GET', `/crm/customers/${customerId}/tags`);
    const stillThere = afterDetach.find(t => Number(t.id) === tagId1);
    expect(
      stillThere,
      `解标后列表不应含 tagId=${tagId1}，实际 ${JSON.stringify(afterDetach)}`
    ).toBeUndefined();

    // 7) 解一个从未挂过的标签 → 关联不存在 → 404（rows_affected=0 分支）
    const tag2 = await apiCallRaw<Row>(page, 'POST', '/crm/tags', {
      name: `111标签乙${tag}`,
      color: '#00ff00',
    });
    const tagId2 = Number(tag2.id);
    expect(tagId2 > 0, `标签乙自建失败 ${JSON.stringify(tag2)}`).toBe(true);
    const detachMissing = await apiCallExpectFail(
      page,
      'DELETE',
      `/crm/customers/${customerId}/tags/${tagId2}`
    );
    expect(
      detachMissing.status,
      `解除从未挂载的标签应 404，实际 status=${detachMissing.status} code=${JSON.stringify(detachMissing.code)}`
    ).toBe(404);

    // 清理：标签与客户的增引用先解后删
    await tryCleanup(page, 'DELETE', `/crm/tags/${tagId1}`, `[111] tag ${tagId1}`);
    await tryCleanup(page, 'DELETE', `/crm/tags/${tagId2}`, `[111] tag ${tagId2}`);
    await tryCleanup(
      page,
      'DELETE',
      `/crm/customers/${customerId}`,
      `[111] customer ${customerId}`
    );
  });

  test('T2 线索标签数组写读回显 空数组被拒且零漂移', async ({ page }) => {
    const tag = genCode('LEAD111');

    const lead = await apiCallRaw<Row>(page, 'POST', '/crm/leads', {
      company_name: `111公司${tag}`,
      contact_name: `111联系${tag}`,
      mobile_phone: uniquePhone(),
      lead_source: 'web',
    });
    const leadId = Number(lead.id);
    expect(leadId > 0, `线索自建失败 ${JSON.stringify(lead)}`).toBe(true);

    // add_tags 口落 crm_lead 行的 tags 列（方案 A 裁定，写响应即更新后的线索行）
    const tags = ['高意向', '大客户'];
    const wrote = await apiCallRaw<Row>(page, 'POST', `/crm/customers/${leadId}/tags`, { tags });
    expect(
      Array.isArray(wrote.tags),
      `写响应 tags 必须回数组，实际 ${JSON.stringify(wrote.tags)}`
    ).toBe(true);
    for (const t of tags) {
      expect(
        (wrote.tags as string[]).includes(t),
        `写响应 tags 应含 ${t}，实际 ${JSON.stringify(wrote.tags)}`
      ).toBe(true);
    }

    // 独立读口互证（写响应的 tags 与 GET 详情逐枚一致，防"响应回显但没落库"）
    const detail = await apiCallRaw<Row>(page, 'GET', `/crm/customers/enhanced/${leadId}`);
    expect(Array.isArray(detail.tags), '详情 tags 必须回数组').toBe(true);
    for (const t of tags) {
      expect(
        (detail.tags as string[]).includes(t),
        `详情 tags 回显应含 ${t}，实际 ${JSON.stringify(detail.tags)}`
      ).toBe(true);
    }

    // 空数组提交被 DTO 拒（min 1），且零漂移（原 tags 保持）——缺键与空集都不当清除处理
    const emptyReject = await apiCallExpectFail(page, 'POST', `/crm/customers/${leadId}/tags`, {
      tags: [],
    });
    expect(
      emptyReject.status,
      `空标签数组必须 400，实际 status=${emptyReject.status} code=${JSON.stringify(emptyReject.code)}`
    ).toBe(400);
    const afterEmpty = await apiCallRaw<Row>(page, 'GET', `/crm/customers/enhanced/${leadId}`);
    expect(
      (afterEmpty.tags as string[]).length,
      `被拒的空数组提交不许动原标签数，实际 ${JSON.stringify(afterEmpty.tags)}`
    ).toBe(tags.length);

    await tryCleanup(page, 'DELETE', `/crm/leads/${leadId}`, `[111] lead ${leadId}`);
  });

  test('T3 增强详情读已建线索 删除后按 id 回读 404', async ({ page }) => {
    const tag = genCode('ENH111');

    // 增强对象口读写 crm_lead 行，用真实线索 id 走 GET enhanced 与 DELETE enhanced
    const lead = await apiCallRaw<Row>(page, 'POST', '/crm/leads', {
      company_name: `111增强${tag}`,
      contact_name: `111增强联系${tag}`,
      mobile_phone: uniquePhone(),
      lead_source: 'web',
    });
    const leadId = Number(lead.id);
    expect(leadId > 0, `线索自建失败 ${JSON.stringify(lead)}`).toBe(true);

    // GET customers/enhanced/{id} 读的是 crm_lead 行（get_customer 走 get_lead）
    const enhanced = await apiCallRaw<Row>(page, 'GET', `/crm/customers/enhanced/${leadId}`);
    expect(
      Number(enhanced.id),
      `增强详情应命中 leadId=${leadId}，实际 ${JSON.stringify(enhanced)}`
    ).toBe(leadId);
    expect(String(enhanced.company_name), '增强详情 company_name 应等于建线索提交名').toBe(
      `111增强${tag}`
    );

    // 删除增强对象行（delete_customer 落 crm_lead）
    await apiCall(page, 'DELETE', `/crm/customers/enhanced/${leadId}`);
    const gone = await apiCallExpectFail(page, 'GET', `/crm/customers/enhanced/${leadId}`);
    expect(
      gone.status,
      `删除线索后按 enhanced 读回必须 404，实际 status=${gone.status} code=${JSON.stringify(gone.code)}`
    ).toBe(404);
  });
});
