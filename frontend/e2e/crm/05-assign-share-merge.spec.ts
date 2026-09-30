// CRM 线索/客户 分配·共享·合并 流程 E2E — crm/05
// 逐端点对后端源码核实（routes/crm.rs + routes/customer_team_share.rs），非编造：
//   分配：POST /crm/assignments(assign_customer) · /crm/assignments/batch · /crm/assignments/transfer
//         POST /crm/assignments/claim · GET /crm/assignments(workload/history)  （handler crm_assignment_handler.rs）
//   共享：POST /customer-shares · /customer-shares/revoke · GET /customer-shares/by-customer · /check  （customer_team_share_handler.rs）
//   团队：POST /customer-team-members · GET /customer-team-members/by-customer/{id} · DELETE /{member_id}
//   合并：POST /crm/customers/merge（customer_merge_handler.rs，转移订单/联系人并置源客户 status='merged'）
//         POST /crm/leads/detect-duplicates · POST /crm/leads/merge（crm_handler.rs，副线索置 lead_status='lost'）
// 归属回读（创建/变更后必回读真实落库字段，不只看 toast/200）：
//   - 转移/认领后 GET /crm/leads/{id} 回读 owner_id 真的变了；
//   - 客户合并后 GET /crm/customers/{target}/summary 的 total_orders/total_order_amount
//     等于用例内自建 seed 订单的真实汇总，且源客户 summary total_orders 归零、status='merged'；
//   - 线索合并后回读副线索 lead_status='lost'（lost_reason 落库）。
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, apiCallRaw, genCode, tryCleanup, verifyEndpointHealthy } from '../flow/helpers';

const LEAD_STATUS_ASSIGNED = 'assigned';
const LEAD_STATUS_LOST = 'lost';

/** 取两个与当前登录用户不同的活跃用户 id（分配/共享的目标人） */
async function pickTwoOtherUserIds(
  page: import('@playwright/test').Page,
  selfId: number
): Promise<[number, number]> {
  // GET /users 契约（handlers/user_handler.rs:392-397 UserListResponse）：
  // data = { users: UserResponse[], total, page, page_size }，users 必为数组——
  // 不做 `?? []` 兜底，缺键/形状漂移立即判红。且该端点仅限 admin（非 admin 403），
  // 本 spec 全部以默认 e2e_admin 上下文运行，满足前置。
  const res = await apiCallRaw<{ users: Array<{ id: number; is_active: boolean }> }>(
    page,
    'GET',
    '/users?page=1&page_size=100'
  );
  expect(Array.isArray(res.users), `GET /users 响应缺 data.users 数组`).toBe(true);
  const ids = res.users.filter(u => u.id !== selfId && u.is_active).map(u => u.id);
  if (ids.length < 2) {
    throw new Error(`可用第二/第三用户不足（需 >=2 个非自身活跃用户，实际 ${ids.length}）`);
  }
  return [ids[0], ids[1]];
}

test.describe('CRM 分配·共享·合并流程', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('线索转移(transfer)与认领(claim)后回读 owner_id 真实变更', async ({ page }) => {
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
    const [assigneeA, assigneeB] = await pickTwoOtherUserIds(page, me.id);

    // 建一条 new 线索（owner=当前用户）
    const leadNo = genCode('E2E-ASG');
    const created = await apiCall<{ id?: number }>(page, 'POST', '/crm/leads', {
      lead_no: leadNo,
      lead_status: 'new',
      lead_source: 'WEBSITE',
      company_name: `E2E 分配线索 ${leadNo}`,
      contact_name: '测试联系人',
      mobile_phone: '13611112222',
    });
    const leadId = created.data?.id;
    expect(leadId, `建线索失败：${JSON.stringify(created)}`).toBeTruthy();

    try {
      // 初始 owner 断言（前置，避免把"归属本就不同"当成转移生效）
      const before = await apiCallRaw<{ owner_id: number; lead_status: string | null }>(
        page,
        'GET',
        `/crm/leads/${leadId}`
      );
      expect(before.owner_id, '前置：新建线索 owner 应为当前用户').toBe(me.id);

      // ---- 转移 assigneeA ----
      await apiCall(page, 'POST', '/crm/assignments/transfer', {
        lead_id: leadId,
        to_user_id: assigneeA,
        reason: 'E2E 归属转移',
        notes: 'transfer-flow',
      });
      const afterTransfer = await apiCallRaw<{ owner_id: number }>(
        page,
        'GET',
        `/crm/leads/${leadId}`
      );
      expect(
        afterTransfer.owner_id,
        `转移后 owner_id 应为 ${assigneeA}，实际 ${afterTransfer.owner_id}`
      ).toBe(assigneeA);

      // ---- 再转移给 assigneeB（当前 owner=assigneeA，非自转，合法）----
      await apiCall(page, 'POST', '/crm/assignments/transfer', {
        lead_id: leadId,
        to_user_id: assigneeB,
        reason: 'E2E 二次转移',
      });
      const afterB = await apiCallRaw<{ owner_id: number }>(page, 'GET', `/crm/leads/${leadId}`);
      expect(afterB.owner_id, `二次转移后 owner_id 应为 ${assigneeB}`).toBe(assigneeB);

      // ---- 分配历史回读：含 TRANSFER 记录 ----
      // list_assignment_history 返回 data.items（AssignmentHistoryModel[]），必为数组
      const history = await apiCallRaw<{ items: Array<{ lead_id: number; action: string }> }>(
        page,
        'GET',
        `/crm/assignments/history?lead_id=${leadId}&page=1&page_size=50`
      );
      expect(Array.isArray(history.items), '分配历史响应缺 data.items 数组').toBe(true);
      const transferRecords = history.items.filter(
        h => h.lead_id === leadId && h.action === 'TRANSFER'
      );
      expect(
        transferRecords.length,
        `分配历史应回读到本次 TRANSFER 记录（实际 ${transferRecords.length}）`
      ).toBeGreaterThanOrEqual(1);
    } finally {
      await tryCleanup(page, 'DELETE', `/crm/leads/${leadId}`, 'crm_lead');
    }
  });

  test('assign_customer 置 assigned 状态；claim 认领 new 线索落库 owner 变更', async ({ page }) => {
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
    const [assigneeA] = await pickTwoOtherUserIds(page, me.id);

    const leadNo = genCode('E2E-CLAIM');
    const created = await apiCall<{ id?: number }>(page, 'POST', '/crm/leads', {
      lead_no: leadNo,
      lead_status: 'new',
      lead_source: 'WEBSITE',
      company_name: `E2E 认领线索 ${leadNo}`,
      contact_name: '认领测试',
    });
    const leadId = created.data?.id;
    expect(leadId, '建线索失败').toBeTruthy();

    try {
      // 分配（assign_customer）：源码只置 lead_status=assigned、写 ASSIGN 历史，
      // **不改 crm_lead.owner_id**（handlers/crm_assignment_handler.rs:60-68，真实缺陷，
      // 移交编排方）——故此处只钉 lead_status 落库真值，不钉 owner_id。
      await apiCall(page, 'POST', '/crm/assignments', {
        lead_id: leadId,
        assignee_id: assigneeA,
        assignee_name: `e2e-assignee-${assigneeA}`,
        notes: 'assign-flow',
      });
      const afterAssign = await apiCallRaw<{ lead_status: string | null }>(
        page,
        'GET',
        `/crm/leads/${leadId}`
      );
      expect(
        afterAssign.lead_status,
        `assign 后 lead_status 应落库为 assigned，实际 ${afterAssign.lead_status}`
      ).toBe(LEAD_STATUS_ASSIGNED);

      // 认领（claim）：new 状态线索被 assigneeA 主动认领，owner 落库为认领人
      const claimLead = await apiCall<{ id?: number }>(page, 'POST', '/crm/leads', {
        lead_no: genCode('E2E-CLAIM2'),
        lead_status: 'new',
        lead_source: 'WEBSITE',
        company_name: `E2E 认领线索2 ${genCode('C2')}`,
        contact_name: '认领测试2',
      });
      const claimId = claimLead.data?.id;
      expect(claimId, '建第二条线索失败').toBeTruthy();
      try {
        await apiCall(page, 'POST', '/crm/assignments/claim', {
          user_id: assigneeA,
          lead_id: claimId,
        });
        const afterClaim = await apiCallRaw<{ owner_id: number; lead_status: string | null }>(
          page,
          'GET',
          `/crm/leads/${claimId}`
        );
        expect(
          afterClaim.owner_id,
          `认领后 owner_id 应落库为认领人 ${assigneeA}，实际 ${afterClaim.owner_id}`
        ).toBe(assigneeA);
        expect(afterClaim.lead_status, '认领后 lead_status 应落库 assigned').toBe(
          LEAD_STATUS_ASSIGNED
        );
      } finally {
        await tryCleanup(page, 'DELETE', `/crm/leads/${claimId}`, 'crm_lead2');
      }
    } finally {
      await tryCleanup(page, 'DELETE', `/crm/leads/${leadId}`, 'crm_lead');
    }
  });

  test('线索合并：detect-duplicates 命中同手机号组，merge 后副线索落库 lost', async ({ page }) => {
    const sharedPhone = `139${Date.now().toString().slice(-8)}`;
    const ids: number[] = [];
    for (let i = 0; i < 2; i++) {
      const c = await apiCall<{ id?: number }>(page, 'POST', '/crm/leads', {
        lead_no: genCode('E2E-DUP'),
        lead_status: 'new',
        lead_source: 'WEBSITE',
        company_name: `E2E 重复合并 ${sharedPhone}`,
        contact_name: '重复联系人',
        mobile_phone: sharedPhone,
      });
      expect(c.data?.id, `建重复线索第 ${i} 条失败`).toBeTruthy();
      ids.push(c.data!.id!);
    }
    const [primaryId, dupId] = ids;

    try {
      // 检测重复组（按手机号）——detect_duplicate_leads 的 data 是 DuplicateLeadGroup[]
      const detect = await apiCallRaw<
        Array<{ match_type: string; lead_ids: number[]; count: number }>
      >(page, 'POST', '/crm/leads/detect-duplicates', { mobile_phone: sharedPhone });
      expect(Array.isArray(detect), 'detect-duplicates 响应 data 应为重复组数组').toBe(true);
      const group = detect.find(g => g.lead_ids.includes(primaryId));
      expect(group, `detect-duplicates 应回读到含线索 ${primaryId} 的重复组`).toBeDefined();
      expect(group!.match_type, '重复组匹配类型应为 mobile_phone').toBe('mobile_phone');
      expect(group!.count, '重复组数量应 >=2').toBeGreaterThanOrEqual(2);

      // 合并到主线索
      const merged = await apiCall<{ merged_count?: number; master_lead_id?: number }>(
        page,
        'POST',
        '/crm/leads/merge',
        { primary_id: primaryId, duplicate_ids: [dupId] }
      );
      expect(merged.data?.master_lead_id, '合并结果主线索 id 应等于 primary').toBe(primaryId);
      expect(merged.data?.merged_count, '合并结果应统计合并数').toBeGreaterThanOrEqual(1);

      // 回读：副线索 lead_status 落库 lost
      const dupAfter = await apiCallRaw<{
        lead_status: string | null;
        lost_reason?: string | null;
      }>(page, 'GET', `/crm/leads/${dupId}`);
      expect(
        dupAfter.lead_status,
        `合并后副线索 lead_status 应为 lost，实际 ${dupAfter.lead_status}`
      ).toBe(LEAD_STATUS_LOST);
      // 主线索保持不变（仍为 new）
      const primaryAfter = await apiCallRaw<{ lead_status: string | null }>(
        page,
        'GET',
        `/crm/leads/${primaryId}`
      );
      expect(primaryAfter.lead_status, '主线索状态不应被合并改写').toBe('new');
    } finally {
      for (const id of ids) await tryCleanup(page, 'DELETE', `/crm/leads/${id}`, 'crm_lead');
    }
  });

  test('客户合并后关联数据归属回读：订单/联系人转移，源客户置 merged', async ({ page }) => {
    // 源/目标两个全新客户（owner=当前用户，满足共享/团队管理权限前置）
    const src = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `E2E 合并源客户 ${genCode('MS')}`,
    });
    const tgt = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `E2E 合并目标客户 ${genCode('MT')}`,
    });
    const sourceId = src.data?.id;
    const targetId = tgt.data?.id;
    expect(sourceId && targetId, '建源/目标客户失败').toBeTruthy();

    // 给源客户挂一个联系人 + 一笔销售订单（用例内自建 seed，汇总真值可控）
    const contactName = `E2E合并联系人 ${genCode('MC')}`;
    await apiCall(page, 'POST', `/crm/customers/${sourceId}/contacts`, {
      name: contactName,
      phone: '13700001111',
    });
    const order = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: sourceId,
      items: [{ product_id: await firstProductId(page), quantity: 4, unit_price: 250 }],
    });
    const orderId = order.data?.id;
    expect(orderId, `为客户合并建订单失败：${JSON.stringify(order)}`).toBeTruthy();
    // 订单总额 4*250 = 1000（so/order_crud.rs 行金额=qty*price，round 2）
    const seededOrderAmount = 4 * 250;

    try {
      // 合并前基线：目标客户无订单/联系人
      const tgtBefore = await apiCallRaw<{ total_orders: number }>(
        page,
        'GET',
        `/crm/customers/${targetId}/summary`
      );
      expect(tgtBefore.total_orders, '合并前目标客户订单数应为 0').toBe(0);

      // 执行合并
      await apiCall(page, 'POST', '/crm/customers/merge', {
        source_customer_id: sourceId,
        target_customer_id: targetId,
        reason: 'E2E 合并流程',
      });

      // 回读：目标客户 summary 承接了源订单（数值真值）
      const tgtAfter = await apiCallRaw<{ total_orders: number; total_order_amount: unknown }>(
        page,
        'GET',
        `/crm/customers/${targetId}/summary`
      );
      expect(tgtAfter.total_orders, `合并后目标客户 total_orders 应为 1`).toBe(1);
      expect(
        Number(String(tgtAfter.total_order_amount)),
        `合并后目标客户 total_order_amount 应等于 seed 汇总 ${seededOrderAmount}`
      ).toBeCloseTo(seededOrderAmount, 2);

      // 回读：源客户订单已迁走（归属不再挂在源）
      const srcAfter = await apiCallRaw<{ total_orders: number }>(
        page,
        'GET',
        `/crm/customers/${sourceId}/summary`
      );
      expect(srcAfter.total_orders, '合并后源客户 total_orders 应归零').toBe(0);

      // 回读：源客户 status='merged'（customer_merge_handler.rs:175）
      const srcRow = await apiCallRaw<{ status?: string }>(
        page,
        'GET',
        `/crm/customers/${sourceId}`
      );
      expect(String(srcRow.status), `合并后源客户 status 应为 merged，实际 ${srcRow.status}`).toBe(
        'merged'
      );

      // 回读：联系人从源迁到目标（list_contacts 的 data 是 customer_contact::Model[]）
      const tgtContacts = await apiCallRaw<Array<{ name: string }>>(
        page,
        'GET',
        `/crm/customers/${targetId}/contacts`
      );
      expect(Array.isArray(tgtContacts), '联系人列表响应 data 应为数组').toBe(true);
      expect(
        tgtContacts.some(c => c.name === contactName),
        '合并后目标客户联系人列表应包含迁移过来的联系人'
      ).toBe(true);
    } finally {
      await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, 'sales_order');
      await tryCleanup(page, 'DELETE', `/crm/customers/${sourceId}`, 'customer_src');
      await tryCleanup(page, 'DELETE', `/crm/customers/${targetId}`, 'customer_tgt');
    }
  });

  test('客户共享：share→by-customer 回读→revoke 状态回读；团队成员增删回读', async ({ page }) => {
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
    const [shareTo, teammate] = await pickTwoOtherUserIds(page, me.id);

    const cust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `E2E 共享客户 ${genCode('SH')}`,
    });
    const customerId = cust.data?.id;
    expect(customerId, '建共享载体客户失败').toBeTruthy();

    try {
      await verifyEndpointHealthy(page, `/customer-shares/by-customer?customer_id=${customerId}`);

      // ---- 共享 ----
      const share = await apiCall<{ id?: number; status?: string; permission?: string }>(
        page,
        'POST',
        '/customer-shares',
        {
          customer_id: customerId,
          shared_to_user_id: shareTo,
          permission: 'view',
          duration_days: 7,
          share_reason: 'E2E 共享流程',
        }
      );
      const shareId = share.data?.id;
      expect(shareId, `共享应返回记录 id：${JSON.stringify(share)}`).toBeTruthy();

      // 回读 by-customer：共享落库，status=active/permission=view
      const shares = await apiCallRaw<
        Array<{ id: number; shared_to_user_id: number; status: string; permission: string }>
      >(page, 'GET', `/customer-shares/by-customer?customer_id=${customerId}`);
      expect(Array.isArray(shares), '共享列表响应 data 应为数组').toBe(true);
      const row = shares.find(s => s.id === shareId);
      expect(row, `by-customer 应回读到共享 ${shareId}`).toBeDefined();
      expect(row!.shared_to_user_id, '回读被共享人 id 应等于提交').toBe(shareTo);
      expect(row!.permission, '回读权限应为 view').toBe('view');
      expect(row!.status, '新建共享 status 应为 active').toBe('active');

      // 检查共享权限
      await verifyEndpointHealthy(
        page,
        `/customer-shares/check?customer_id=${customerId}&user_id=${shareTo}`
      );

      // ---- 撤销 ----
      await apiCall(page, 'POST', '/customer-shares/revoke', {
        share_id: shareId,
        revoke_reason: 'E2E 撤销共享',
      });
      const sharesAfter = await apiCallRaw<Array<{ id: number; status: string }>>(
        page,
        'GET',
        `/customer-shares/by-customer?customer_id=${customerId}&status=revoked`
      );
      expect(Array.isArray(sharesAfter), '撤销后共享列表 data 应为数组').toBe(true);
      const revokedRow = sharesAfter.find(s => s.id === shareId);
      expect(revokedRow, `撤销后应能在 status=revoked 列表回读到共享 ${shareId}`).toBeDefined();
      expect(revokedRow!.status, '撤销后 status 应为 revoked').toBe('revoked');

      // ---- 团队成员 ----
      await verifyEndpointHealthy(page, `/customer-team-members/by-customer/${customerId}`);
      const member = await apiCall<{ id?: number }>(page, 'POST', '/customer-team-members', {
        customer_id: customerId,
        user_id: teammate,
        team_role: 'member',
        notes: 'E2E 团队成员',
      });
      const memberId = member.data?.id;
      expect(memberId, `添加团队成员应返回 id：${JSON.stringify(member)}`).toBeTruthy();

      const members = await apiCallRaw<Array<{ id: number; user_id: number; team_role: string }>>(
        page,
        'GET',
        `/customer-team-members/by-customer/${customerId}`
      );
      expect(Array.isArray(members), '团队成员列表 data 应为数组').toBe(true);
      const mRow = members.find(m => m.id === memberId);
      expect(mRow, `团队成员列表应回读到成员 ${memberId}`).toBeDefined();
      expect(mRow!.user_id, '回读成员 user_id 应等于提交').toBe(teammate);
      expect(mRow!.team_role, '回读成员角色应为 member').toBe('member');

      // 移除团队成员（remove 为软删 is_active=false，list_team_members 只列活跃成员，
      // 故移除后该成员应从活跃列表消失——真实回读，非只看 200）
      await apiCall(page, 'DELETE', `/customer-team-members/${memberId}`);
      const membersAfter = await apiCallRaw<Array<{ id: number }>>(
        page,
        'GET',
        `/customer-team-members/by-customer/${customerId}`
      );
      expect(Array.isArray(membersAfter), '移除后团队成员列表 data 应为数组').toBe(true);
      expect(
        membersAfter.some(m => m.id === memberId),
        '移除后团队成员活跃列表不应再包含该成员'
      ).toBe(false);
    } finally {
      await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, 'customer_share');
    }
  });
});

/** 取一个真实产品 id（订单明细 FK 依赖产品存在）——/products 返回 PaginatedResponse{items} */
async function firstProductId(page: import('@playwright/test').Page): Promise<number> {
  const res = await apiCallRaw<{ items: Array<{ id: number }> }>(
    page,
    'GET',
    '/products?page=1&page_size=5'
  );
  expect(Array.isArray(res.items), 'GET /products 响应缺 data.items 数组').toBe(true);
  const id = res.items[0]?.id;
  if (!id) throw new Error('前置：需要一个已存在的产品用于订单明细 FK');
  return id;
}
