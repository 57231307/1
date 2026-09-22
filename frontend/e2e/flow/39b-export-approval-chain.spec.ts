import { test, expect } from '../diagnose-fixture';
import { loginViaUI, loginAsRole } from './helpers';

/**
 * P5.9b 敏感导出完整审批链
 *
 * 流程：创建审批申请 → 一级审批（approve）→ 生成下载令牌 →
 *       持令牌导出 200 → 令牌二次消费拒绝 → 跨资源令牌拒绝
 *
 * 前置：export_approval_service.rs:224 明确禁止申请人自审
 *       （"审批人不能是申请人本人（防自审批）"），因此完整链必须双人：
 *       由 manager 角色账号创建申请，再切回分片 admin 账号审批。
 *       estimated_rows=10 → 风险等级 low → approval_level=1，一次 approve 即出令牌。
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

interface ApprovalModel {
  id: number;
  status: string;
  download_token?: string | null;
  resource_type: string;
}

const JSON_HEADERS = {
  'X-Requested-With': 'XMLHttpRequest',
  'Content-Type': 'application/json',
} as const;

/** manager 身份创建 customer 导出审批申请，返回审批单 id */
async function createApprovalAsManager(page: import('@playwright/test').Page): Promise<number> {
  await loginAsRole(page, 'manager');
  const resp = await page.request.post(`${API_BASE}${API_PREFIX}/export-approvals`, {
    data: {
      resource_type: 'customer',
      export_params: { page: 1, page_size: 10 },
      estimated_rows: 10,
      file_format: 'xlsx',
    },
    headers: JSON_HEADERS,
  });
  const body = (await resp.json().catch(() => null)) as { data?: ApprovalModel } | null;
  const id = body?.data?.id;
  expect(
    resp.ok() && id,
    `manager 创建导出审批申请失败 status=${resp.status()} body=${JSON.stringify(body).slice(0, 200)}`
  ).toBe(true);
  console.log(`[39b] manager 已创建 customer 导出审批申请 id=${id}`);
  return id as number;
}

/** 切回分片 admin 账号审批该申请，返回审批后的模型（含 download_token） */
async function approveAsAdmin(
  page: import('@playwright/test').Page,
  approvalId: number,
  comments: string
): Promise<ApprovalModel> {
  await loginViaUI(page, undefined, undefined, true);
  const resp = await page.request.post(
    `${API_BASE}${API_PREFIX}/export-approvals/${approvalId}/approve`,
    { data: { comments }, headers: JSON_HEADERS }
  );
  const body = (await resp.json().catch(() => null)) as { data?: ApprovalModel } | null;
  expect(
    resp.ok(),
    `admin 审批申请 ${approvalId} 失败 status=${resp.status()} body=${JSON.stringify(body).slice(0, 200)}`
  ).toBe(true);
  const approved = body?.data;
  expect(
    approved?.status ?? '',
    `审批后状态应含 approved，实际：${JSON.stringify(approved).slice(0, 200)}`
  ).toContain('approved');
  console.log(`[39b] admin 已审批申请 ${approvalId}，status=${approved?.status}`);
  return approved as ApprovalModel;
}

test.describe('P5.9b 敏感导出完整审批链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('客户导出审批链：创建→审批→令牌导出→二次消费拒', async ({ page }) => {
    // 1-2. manager 创建申请 + admin 审批（自审被后端禁止，必须双人）
    const approvalId = await createApprovalAsManager(page);
    const approved = await approveAsAdmin(page, approvalId, 'E2E 审批链测试');

    // 3. 一级审批通过必须签发下载令牌（无令牌 = 链断在这里，不能再降级为可选分支）
    const token = approved.download_token;
    expect(
      token,
      `审批通过后应签发 download_token，实际：${JSON.stringify(approved).slice(0, 200)}`
    ).toBeTruthy();

    // 4. 持令牌导出
    const exportResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${token}`
    );
    expect(exportResp.status(), '持有效令牌导出应 200').toBe(200);

    // 5. 令牌二次消费拒绝（download_count 上限/一次性）
    const secondResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${token}`
    );
    expect(secondResp.status(), '令牌二次消费应被拒（403）').toBe(403);
  });

  test('跨资源令牌拒绝：customer 令牌用于 product 导出', async ({ page }) => {
    // 创建 customer 审批并拿到令牌（manager 申请 + admin 审批）
    const approvalId = await createApprovalAsManager(page);
    const approved = await approveAsAdmin(page, approvalId, 'E2E 跨资源测试');
    const token = approved.download_token;
    expect(
      token,
      `审批通过后应签发 download_token，实际：${JSON.stringify(approved).slice(0, 200)}`
    ).toBeTruthy();

    // customer 令牌用于 product 导出 → resource_type 不匹配 403
    const crossResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/products/export?download_token=${token}`
    );
    expect(crossResp.status(), '跨资源令牌应 403').toBe(403);
  });

  test('审批状态机：未审批申请直接导出 403（fail-closed 兜底）', async ({ page }) => {
    // pending 状态的申请其 token 为 null——直接带空/伪 token 导出被拒
    const resp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=not-a-real-token`
    );
    expect(resp.status()).toBe(403);
  });
});
