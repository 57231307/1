import { test, expect } from '../diagnose-fixture';
import { loginViaUI, loginAsRole, apiCall } from './helpers';

/**
 * 产品导出完整审批链回归（申请人侧流程对应的服务端契约）
 *
 * 流程：manager 创建 product 导出审批申请 至 admin 一级审批通过 至 生成一次性下载令牌 至
 *       verify-token 判有效 至 持令牌导出 200 且文件体非空 至 同令牌二次下载被拒。
 *
 * 对照钉死两件事：
 * - 旧路径（无令牌直连 products/export）必 403（fail-closed 生效，缺陷根因）；
 * - 新路径（审批后持令牌）必 200 且 blob 非空（申请通道打通，缺陷已修）。
 *
 * 前置：后端禁止申请人自审（审批人不能是申请人本人），故申请用 manager 角色账号，
 *       审批切回分片 admin 账号。estimated_rows=10 命中低风险，approval_level=1，
 *       一级 approve 即签发令牌。写请求统一走 helpers.apiCall（CSRF 有界重放），
 *       GET 为安全方法不消费 CSRF，直连 page.request。
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

interface ApprovalModel {
  id: number;
  status: string;
  resource_type: string;
  file_format: string;
  approval_level: number;
  download_token?: string | null;
}

async function createProductApproval(page: import('@playwright/test').Page): Promise<number> {
  await loginAsRole(page, 'manager');
  const res = await apiCall<ApprovalModel>(page, 'POST', '/export-approvals', {
    resource_type: 'product',
    export_params: {},
    estimated_rows: 10,
    file_format: 'xlsx',
  });
  const created = res.data;
  expect(
    created?.id,
    `manager 创建 product 导出审批申请应返回有效 id：${JSON.stringify(created).slice(0, 200)}`
  ).toBeGreaterThan(0);
  // 申请通道打通的直接证据：product 已被敏感清单接受，返回 pending（不再是
  // "资源类型不在敏感清单"的校验拒绝，apiCall 对非 200 信封已判红抛错）
  expect(created?.status, '创建成功后初始状态应为 pending').toBe('pending');
  expect(created?.resource_type, '申请应绑定 product 资源类型').toBe('product');
  expect(created?.file_format, '申请应按提交的文件格式落库').toBe('xlsx');
  console.log(`[39d] manager 已创建 product 导出审批申请 id=${created?.id}`);
  return created.id;
}

async function approveAsAdmin(
  page: import('@playwright/test').Page,
  approvalId: number
): Promise<ApprovalModel> {
  await loginViaUI(page, undefined, undefined, true);
  const res = await apiCall<ApprovalModel>(
    page,
    'POST',
    `/export-approvals/${approvalId}/approve`,
    { comments: 'E2E 产品导出审批链' }
  );
  const approved = res.data;
  expect(
    approved?.status,
    `低风险一级审批通过后状态应为 approved，实际：${JSON.stringify(approved).slice(0, 200)}`
  ).toBe('approved');
  expect(
    approved?.download_token,
    `审批通过后应签发 download_token，实际：${JSON.stringify(approved).slice(0, 200)}`
  ).toBeTruthy();
  console.log(`[39d] admin 已审批产品导出申请 ${approvalId}`);
  return approved;
}

test.describe('产品导出完整审批链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('对照：无令牌直连产品导出必 403（旧路径 fail-closed）', async ({ page }) => {
    const resp = await page.request.get(`${API_BASE}${API_PREFIX}/products/export`);
    expect(
      resp.status(),
      '无 download_token 的产品导出必须被 fail-closed 拒绝 403'
    ).toBe(403);
  });

  test('完整链：申请至审批至verify-token有效至持令牌200至二次用token被拒', async ({ page }) => {
    const approvalId = await createProductApproval(page);
    const approved = await approveAsAdmin(page, approvalId);
    const token = approved.download_token;

    // verify-token：GET 安全方法直连；后端出参为完整审批 Model（非 valid 布尔信封），
    // 有效令牌判据是 status approved 且 resource_type product
    await loginViaUI(page, undefined, undefined, true);
    const verified = await apiCall<ApprovalModel>(
      page,
      'GET',
      `/export-approvals/verify-token?token=${token}`
    );
    expect(verified.data?.status, 'verify-token 对已签发令牌应返回 approved').toBe('approved');
    expect(verified.data?.resource_type, 'verify-token 应回显 product 资源绑定').toBe('product');

    // 新路径：持令牌导出 200 且文件体非空（真实二进制，不是 JSON 错误体）
    const exportResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/products/export?download_token=${token}`
    );
    expect(exportResp.status(), '持有效令牌导出应 200').toBe(200);
    const contentType = exportResp.headers()['content-type'] ?? '';
    expect(
      contentType,
      `导出响应应为 xlsx 二进制流，实际 content-type=${contentType}`
    ).toContain('spreadsheetml');
    const bytes: Uint8Array = await exportResp.body();
    expect(bytes.byteLength, '导出文件内容必须非空').toBeGreaterThan(0);

    // 一次性语义：同令牌二次下载必须 403（download_count 达上限），钉死防重放
    const replayResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/products/export?download_token=${token}`
    );
    expect(replayResp.status(), '令牌已消费，二次下载应被拒 403').toBe(403);
  });

  test('跨资源防护：product 令牌不可用于客户导出', async ({ page }) => {
    const approvalId = await createProductApproval(page);
    const approved = await approveAsAdmin(page, approvalId);
    const token = approved.download_token;
    const crossResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${token}`
    );
    expect(crossResp.status(), '令牌绑定资源类型，跨资源使用应被拒 403').toBe(403);
  });
});
