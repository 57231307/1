/**
 * 报价单双人审批链：销售代表提单、销售经理批准与驳回、管理员凭审计快照核对
 *
 * 功能：一条三棒接力加管理员收口，把两张报价单从草稿推到终态，并逐个角色核对该做什么、被挡住什么：
 *   第一棒 admin 建报价专用产品，报价行单位取后端落库真值；
 *   第二棒 sales_rep 建客户与两张报价单（各 20 万，金额落在销售经理审批档），提交并回读待审批，
 *     再对自家报价单调批准与拒绝端点，断言被权限门 403 拒绝（建单与审批分岗，提交人不得自批）；
 *   第三棒 sales_manager 批准一张、驳回另一张且都带理由，断言响应里的状态与理由列；
 *     同时断言其按 id 读他人报价单被行级数据范围 403 拒绝；
 *   管理员收口由本文件用 page 夹具的 admin 会话完成：回读两张单终态与理由列、按资源 id 取
 *     auto_audit 审计详情断言 before 到 after 快照里的状态迁移与操作人、最后在详情页核对界面状态标签。
 *
 * 调用方：Playwright E2E 套件，由 CI 的 e2e flow 分片按文件哈希收集执行。
 *
 * 入参：BASE_URL 与 API_BASE 取环境变量；三个角色凭证取 env 优先，回退 e2e/.auth/role-credentials.json。
 *
 * 传给谁：多角色会话与身份钉由 fixtures/relay 的 createRelay 与 runSteps 建与查；
 *   端点正负例走 flow/helpers 的 apiCall、apiCallRaw、apiCallExpectFail 与 expectDenied。
 *
 * 存什么：客户一行、产品一行、报价单两行与各自明细行、每次状态迁移一条 auto_audit 审计行
 *   （含 before 与 after 整模型快照、操作人 user_id 与 username、变更时间）。
 *
 * 存哪里：客户、产品、报价单落各自业务表，审计行落 audit_log 表；
 *   会话 cookie 只在各棒 BrowserContext 内，随 context 关闭消失；
 *   报价单族没有 DELETE 路由，两张报价单与明细行按业务事实留库不回收；
 *   明细行 product_id 对产品是 NO ACTION 外键（migration/domain/sales_crm 建表语句），
 *   产品被明细行钉住删不掉，故产品不登记清理、随单留库；
 *   客户删除是软删（customer_ops/crud.rs 走 status=inactive），不受该外键影响，
 *   由第二棒清理队列在接力收尾时用产出者自己的会话发起。
 *
 * 为什么第三棒用的是部门档角色却只能批不能读：sales_manager 的 data_scope 是 dept，
 *   而 e2e 角色账号按既有可达性用例的口径不补部门，DataScopeContext 的可见成员集合为空时
 *   Dept 档退化为 self（backend/src/utils/data_scope.rs 注释与实现同源），故按 id 读他人单据
 *   会 fail-closed 403。批准与拒绝端点的 handler 不做数据范围归属门（只做状态门与权限门），
 *   因此这条链判的是两件事：审批权确实只在销售经理手里，且无部门账号的数据范围确实收敛。
 */
import { test, expect, type Page } from '../diagnose-fixture';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  expectDenied,
  genCode,
  loginViaUI,
  BASE_URL,
} from './helpers';
import { createRelay, expectEndpointDenied } from '../fixtures/relay';

type Row = Record<string, unknown>;

/** 两张报价单的金额：20 万落在销售经理审批档（10 万到 50 万），提交后进 BPM 待审而不是自批 */
const APPROVE_AMOUNT = 200000;

/** 审批通过理由，落 sales_quotations.approval_reason 列，收口时逐字回读 */
const APPROVE_REASON = 'E2E 双人审批链 通过';

/** 驳回理由，落 sales_quotations.rejection_reason 列，收口时逐字回读 */
const REJECT_REASON = 'E2E 双人审批链 驳回';

/** 审计行的资源类型：报价审批服务每次状态迁移都以 auto_audit 落一行 UPDATE 审计 */
const AUDIT_RESOURCE_TYPE = 'auto_audit';

/** 审计行轮询上限与间隔：快照审计在同事务内直写，理论立即可见，留出读副本与索引延迟余量 */
const AUDIT_POLL_TIMES = 20;
const AUDIT_POLL_INTERVAL_MS = 500;

function isoDate(offsetDays = 0): string {
  return new Date(Date.now() + offsetDays * 86400000).toISOString().slice(0, 10);
}

/**
 * 建一张报价单并回钉金额。
 * 调用方：第二棒 sales_rep 在自己会话里造两张待审单据。
 * 入参：会话 page、客户 id、业务员用户 id、产品 id 与落库单位、金额。
 * 传给谁：POST /quotations，随后 GET /quotations/{id} 回读 total_amount。
 * 存什么/存哪里：报价单与明细行落业务表，不在此处回收。
 */
async function createQuotation(
  page: Page,
  args: {
    customerId: number;
    salesUserId: number;
    productId: number;
    productUnit: string;
    amount: number;
    notes: string;
  }
): Promise<number> {
  const res = await apiCall<{ id?: number }>(page, 'POST', '/quotations', {
    customer_id: args.customerId,
    sales_user_id: args.salesUserId,
    quotation_date: isoDate(0),
    valid_until: isoDate(30),
    currency: 'CNY',
    exchange_rate: '1',
    base_currency: 'CNY',
    price_terms: 'FOB',
    tax_inclusive: true,
    tax_rate: '0',
    items: [
      {
        product_id: args.productId,
        unit: args.productUnit,
        quantity: String(args.amount),
        unit_price: '1',
        unit_price_with_tax: '1',
      },
    ],
    notes: args.notes,
  });
  const id = res.data?.id;
  expect(id, `前置失败：${args.notes} 创建未返回 id`).toBeTruthy();
  const back = await apiCallRaw<Row>(page, 'GET', `/quotations/${id}`);
  expect(back.status, `新建报价单 ${args.notes} 应为 draft`).toBe('draft');
  expect(
    Number(back.total_amount),
    `报价单 ${args.notes} 金额应可控为 ${args.amount}，决定提交后自批还是走审批`
  ).toBe(args.amount);
  return id as number;
}

/**
 * 按 id 读报价单主表。
 * 调用方：本文件各棒的写后回读与管理员收口断言。
 * 入参：会话 page 与报价单 id。
 * 传给谁：GET /quotations/{id}，返回后端下发的 QuotationResponseDto。
 * 存什么/存哪里：只读请求，不持久化。
 */
async function getQuotation(page: Page, id: number): Promise<Row> {
  return apiCallRaw<Row>(page, 'GET', `/quotations/${id}`);
}

/**
 * 取某张报价单的 auto_audit 审计行（列表层，不含快照）。
 * 调用方：管理员收口核对审计记录。
 * 入参：会话 page 与报价单 id 字符串。
 * 传给谁：GET /audit-logs 按 resource_type 过滤，再在客户端按 resource_id 精确匹配，
 *   不用 keyword 模糊搜索，避免同前缀 id 混入别家单据。
 * 存什么/存哪里：只读请求，不持久化。
 */
async function pollAuditRows(
  page: Page,
  quotationId: string
): Promise<Row[]> {
  let rows: Row[] = [];
  for (let i = 0; i < AUDIT_POLL_TIMES; i++) {
    const listed = await apiCallRaw<{ items?: Row[] }>(
      page,
      'GET',
      `/audit-logs?page=1&page_size=100&resource_type=${AUDIT_RESOURCE_TYPE}`
    );
    rows = (listed?.items ?? []).filter(row => String(row.resource_id ?? '') === quotationId);
    if (rows.length > 0) return rows;
    await new Promise(resolve => setTimeout(resolve, AUDIT_POLL_INTERVAL_MS));
  }
  return rows;
}

/**
 * 取一条审计行的详情（含 before 与 after 快照）。
 * 调用方：管理员收口断言状态迁移与操作人。
 * 入参：会话 page 与审计行 id。
 * 传给谁：GET /audit-logs/{id}，返回 AuditLogDetailResponse。
 * 存什么/存哪里：只读请求，不持久化。
 */
async function getAuditDetail(
  page: Page,
  auditId: number
): Promise<Row> {
  return apiCallRaw<Row>(page, 'GET', `/audit-logs/${auditId}`);
}

/**
 * 从一批审计行里筛出满足状态迁移与操作人的那一行详情。
 * 调用方：管理员收口分别核对提交、批准、驳回三条迁移。
 * 入参：会话 page、审计行列表、迁移前后状态字面量、期望操作人 username、断言上下文。
 * 传给谁：逐行 GET /audit-logs/{id} 读快照后按 before.status 与 after.status 与 username 匹配。
 * 存什么/存哪里：只读请求，不持久化。
 */
async function findAuditTransition(
  page: Page,
  rows: Row[],
  fromStatus: string,
  toStatus: string,
  actorUsername: string,
  context: string
): Promise<Row> {
  const seen: string[] = [];
  for (const row of rows) {
    const detail = await getAuditDetail(page, Number(row.id));
    const before = (detail.before_snapshot ?? {}) as Row;
    const after = (detail.after_snapshot ?? {}) as Row;
    const beforeStatus = String(before.status ?? '');
    const afterStatus = String(after.status ?? '');
    const username = String(detail.username ?? '');
    seen.push(`${beforeStatus}->${afterStatus}@${username}`);
    if (beforeStatus === fromStatus && afterStatus === toStatus && username === actorUsername) {
      return detail;
    }
  }
  throw new Error(
    `${context}：审计行里找不到 ${fromStatus} 到 ${toStatus} 且操作人为 ${actorUsername} 的迁移，` +
      `已检视=${seen.join(',') || '无'}（审计行数=${rows.length}）`
  );
}

test.describe.serial('报价单双人审批链：销售代表提单、销售经理批驳、管理员审计取证', () => {
  test('三棒接力完成批与驳，管理员按审计快照与详情页核对', async ({ page, browser }) => {
    await loginViaUI(page);
    const relay = await createRelay(browser);
    try {
      await relay.runSteps([
        {
          role: 'admin',
          label: '建报价专用产品',
          run: async ({ session }) => {
            const code = genCode('E2E-REL-BIZ-P');
            const created = await apiCallRaw<Row>(session.page, 'POST', '/products', {
              code,
              name: `E2E双人审批产品${code}`,
              unit: '米',
              // 建品门要求显式提交产品类型（validate_product_create_fields 对缺省即拒）；
              // 取辅料档避开面料类的幅宽/克重/成分三列必录门，本用例的靶心是报价审批链
              product_type: '辅料',
            });
            const productId = Number(created?.id);
            if (!Number.isFinite(productId) || productId <= 0) {
              throw new Error(
                `第一棒建产品未返回有效 id，后续报价行失去单位真源，实际出参片段=` +
                  `${JSON.stringify(created)?.slice(0, 200)}`
              );
            }
            // 单位取后端落库真值：报价行 unit 须与产品交易单位逐字符相等，否则建单直接 400
            const detail = await apiCallRaw<Row>(session.page, 'GET', `/products/${productId}`);
            const unit = String(detail?.unit ?? '');
            if (!unit) {
              throw new Error(`第一棒产品 ${productId} 回读缺少 unit，报价行单位无从取值`);
            }
            return {
              artifacts: [
                { key: 'productId', value: productId },
                { key: 'productUnit', value: unit },
              ],
            };
          },
        },
        {
          role: 'sales_rep',
          label: '建客户与两张报价单并提交',
          run: async ({ session, require, registerCleanup }) => {
            const productId = Number(require('productId'));
            const productUnit = String(require('productUnit'));

            const customerName = genCode('E2E-REL-BIZ-CUS');
            const customer = await apiCallRaw<Row>(session.page, 'POST', '/crm/customers', {
              customer_name: customerName,
              contact_phone: '13800000000',
            });
            const customerId = Number(customer?.id);
            if (!Number.isFinite(customerId) || customerId <= 0) {
              throw new Error(
                `第二棒建客户未返回有效 id，报价单失去客户引用，实际出参片段=` +
                  `${JSON.stringify(customer)?.slice(0, 200)}`
              );
            }
            registerCleanup({
              method: 'DELETE',
              path: `/crm/customers/${customerId}`,
              label: `接力客户 ${customerName}`,
            });

            const quotationAId = await createQuotation(session.page, {
              customerId,
              salesUserId: session.userId,
              productId,
              productUnit,
              amount: APPROVE_AMOUNT,
              notes: 'E2E 双人审批链 待批准件',
            });
            const quotationBId = await createQuotation(session.page, {
              customerId,
              salesUserId: session.userId,
              productId,
              productUnit,
              amount: APPROVE_AMOUNT,
              notes: 'E2E 双人审批链 待驳回件',
            });

            for (const id of [quotationAId, quotationBId]) {
              const submitted = await apiCall<Row>(session.page, 'POST', `/quotations/${id}/submit`);
              expect(
                submitted.message,
                `第二棒提交报价单 ${id} 的响应文案应为已提交审批`
              ).toContain('已提交审批');
              const back = await getQuotation(session.page, id);
              expect(
                back.status,
                `第二棒提交后报价单 ${id} 应进入 pending_approval（金额 ${APPROVE_AMOUNT} 走销售经理档）`
              ).toBe('pending_approval');
              expect(back.approved_by, `第二棒提交后报价单 ${id} 不应被自批出审批人`).toBeNull();
            }

            // 建审分离：提交人对自家待审单据调批准与拒绝都必须被权限门挡住，不是状态门挡住
            await expectEndpointDenied(
              session.page,
              'POST',
              `/quotations/${quotationAId}/approve`,
              '第二棒销售代表对自家待审报价单调批准端点'
            );
            await expectEndpointDenied(
              session.page,
              'POST',
              `/quotations/${quotationBId}/reject`,
              '第二棒销售代表对自家待审报价单调拒绝端点'
            );

            return {
              artifacts: [
                { key: 'customerId', value: customerId },
                { key: 'quotationAId', value: quotationAId },
                { key: 'quotationBId', value: quotationBId },
                { key: 'repUsername', value: session.username },
              ],
            };
          },
        },
        {
          role: 'sales_manager',
          label: '批准一张、驳回一张',
          run: async ({ session, require }) => {
            const quotationAId = Number(require('quotationAId'));
            const quotationBId = Number(require('quotationBId'));

            const approved = await apiCallRaw<Row>(
              session.page,
              'POST',
              `/quotations/${quotationAId}/approve`,
              { approval_reason: APPROVE_REASON }
            );
            expect(
              approved.status,
              `第三棒批准后报价单 ${quotationAId} 应为 approved`
            ).toBe('approved');
            expect(
              String(approved.approval_reason ?? ''),
              `批准理由应逐字落 approval_reason 列`
            ).toBe(APPROVE_REASON);
            expect(
              Number(approved.approved_by),
              `approval 端点的审批人应为本棒销售经理 user_id=${session.userId}`
            ).toBe(session.userId);
            expect(approved.approved_at, '批准后必须留下决策时间').toBeTruthy();

            const rejected = await apiCallRaw<Row>(
              session.page,
              'POST',
              `/quotations/${quotationBId}/reject`,
              { reason: REJECT_REASON }
            );
            expect(
              rejected.status,
              `第三棒驳回后报价单 ${quotationBId} 应为 rejected`
            ).toBe('rejected');
            expect(
              String(rejected.rejection_reason ?? ''),
              `驳回理由应逐字落 rejection_reason 列`
            ).toBe(REJECT_REASON);
            expect(
              Number(rejected.approved_by),
              `驳回端点的决策人应为本棒销售经理 user_id=${session.userId}`
            ).toBe(session.userId);
            expect(rejected.approval_reason, '被驳回的单不应带审批通过理由').toBeNull();

            // 行级数据范围：部门档账号无部门时可见成员集合为空，Dept 退化为 self，读他人单据 fail-closed
            const denied = await apiCallExpectFail(
              session.page,
              'GET',
              `/quotations/${quotationAId}`
            );
            expectDenied(denied, '第三棒销售经理按 id 读他人报价单');
            expect(
              String(denied.message ?? ''),
              '该 403 应归因到行级数据范围（可见成员集合不含归属人），而不是权限码缺授'
            ).toContain('数据范围');

            return {
              artifacts: [
                { key: 'managerUserId', value: session.userId },
                { key: 'managerUsername', value: session.username },
              ],
            };
          },
        },
      ]);

      const quotationAId = Number(relay.artifacts.quotationAId);
      const quotationBId = Number(relay.artifacts.quotationBId);
      const managerUserId = Number(relay.artifacts.managerUserId);
      const managerUsername = String(relay.artifacts.managerUsername);
      const repUsername = String(relay.artifacts.repUsername);

      const approved = await getQuotation(page, quotationAId);
      expect(approved.status, `收口回读：报价单 ${quotationAId} 终态应为 approved`).toBe('approved');
      expect(String(approved.approval_reason ?? ''), '收口回读：批准理由应与提交值逐字一致').toBe(
        APPROVE_REASON
      );
      expect(Number(approved.approved_by), '收口回读：审批人应为第三棒的销售经理').toBe(
        managerUserId
      );
      expect(approved.approved_at, '收口回读：批准时间必须落库').toBeTruthy();
      expect(approved.rejection_reason, '已批准的单不应带驳回理由').toBeNull();

      const rejected = await getQuotation(page, quotationBId);
      expect(rejected.status, `收口回读：报价单 ${quotationBId} 终态应为 rejected`).toBe(
        'rejected'
      );
      expect(String(rejected.rejection_reason ?? ''), '收口回读：驳回理由应与提交值逐字一致').toBe(
        REJECT_REASON
      );
      expect(Number(rejected.approved_by), '收口回读：驳回决策人应为第三棒的销售经理').toBe(
        managerUserId
      );

      // 审计：每次状态迁移一条 auto_audit，含整模型 before 与 after 快照与操作人 username
      const rowsA = await pollAuditRows(page, String(quotationAId));
      expect(
        rowsA.length,
        `报价单 ${quotationAId} 应有提交与批准两条 auto_audit 审计行，实际 ${rowsA.length} 条`
      ).toBeGreaterThanOrEqual(2);
      const submitAuditA = await findAuditTransition(
        page,
        rowsA,
        'draft',
        'pending_approval',
        repUsername,
        `报价单 ${quotationAId} 的提交迁移`
      );
      expect(
        String(submitAuditA.resource_type ?? ''),
        '提交审计行的资源类型应为 auto_audit'
      ).toBe(AUDIT_RESOURCE_TYPE);
      expect(String(submitAuditA.operation_type ?? ''), '提交审计行的操作类型应为 UPDATE').toBe(
        'UPDATE'
      );
      expect(String(submitAuditA.created_at ?? ''), '提交审计行必须带变更时间').not.toBe('');

      const approveAuditA = await findAuditTransition(
        page,
        rowsA,
        'pending_approval',
        'approved',
        managerUsername,
        `报价单 ${quotationAId} 的批准迁移`
      );
      const approveBeforeA = (approveAuditA.before_snapshot ?? {}) as Row;
      const approveAfterA = (approveAuditA.after_snapshot ?? {}) as Row;
      expect(
        String(approveBeforeA.approval_reason ?? ''),
        '批准前快照不应已有审批通过理由'
      ).toBe('');
      expect(
        String(approveAfterA.approval_reason ?? ''),
        '批准后快照的 approval_reason 应与本次通过理由逐字一致'
      ).toBe(APPROVE_REASON);
      expect(
        Number(approveAfterA.approved_by),
        '批准后快照的审批人应为销售经理'
      ).toBe(managerUserId);
      expect(String(approveAuditA.created_at ?? ''), '批准审计行必须带变更时间').not.toBe('');

      const rowsB = await pollAuditRows(page, String(quotationBId));
      expect(
        rowsB.length,
        `报价单 ${quotationBId} 应有提交与驳回两条 auto_audit 审计行，实际 ${rowsB.length} 条`
      ).toBeGreaterThanOrEqual(2);
      await findAuditTransition(
        page,
        rowsB,
        'draft',
        'pending_approval',
        repUsername,
        `报价单 ${quotationBId} 的提交迁移`
      );
      const rejectAuditB = await findAuditTransition(
        page,
        rowsB,
        'pending_approval',
        'rejected',
        managerUsername,
        `报价单 ${quotationBId} 的驳回迁移`
      );
      const rejectAfterB = (rejectAuditB.after_snapshot ?? {}) as Row;
      expect(
        String(rejectAfterB.rejection_reason ?? ''),
        '驳回后快照的 rejection_reason 应与本次驳回理由逐字一致'
      ).toBe(REJECT_REASON);
      expect(
        Number(rejectAuditB.user_id),
        '驳回审计行的操作人 user_id 应为销售经理'
      ).toBe(managerUserId);

      // 界面核对：详情页按状态渲染标签与理由行
      await page.goto(`${BASE_URL}/quotations/${quotationAId}`);
      await expect(
        page.locator('.el-tag').filter({ hasText: '已批准' }).first(),
        '已批准报价单详情页应显示状态标签 已批准'
      ).toBeVisible({ timeout: 30000 });
      await expect(
        page.getByText(APPROVE_REASON).first(),
        '已批准报价单详情页应显示审批通过理由'
      ).toBeVisible({ timeout: 30000 });

      await page.goto(`${BASE_URL}/quotations/${quotationBId}`);
      await expect(
        page.locator('.el-tag').filter({ hasText: '已拒绝' }).first(),
        '已拒绝报价单详情页应显示状态标签 已拒绝'
      ).toBeVisible({ timeout: 30000 });
      await expect(
        page.getByText(REJECT_REASON).first(),
        '已拒绝报价单详情页应显示驳回理由'
      ).toBeVisible({ timeout: 30000 });
    } finally {
      await relay.close();
    }
  });
});
