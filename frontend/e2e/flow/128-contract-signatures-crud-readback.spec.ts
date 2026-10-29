import { test, expect } from '../diagnose-fixture';
import {
  APP_ERROR_CODES,
  loginViaUI,
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  deferCleanup,
  flushDeferredCleanups,
  failureCode,
  type DeferredCleanup,
} from './helpers';

/**
 * 128 合同签名端点族的签署验证撤销读回（finance 域，contract-signatures 族）
 *
 * 功能：守住电子签章面（表 sales_contracts 的签名列组）确实把提交值落库、
 * 哈希可验证、撤销可清除，覆盖以下四个真实端点，均由 contract_signature_handler 产出：
 * - GET /contract-signatures 列表已签章合同（按 signed_at IS NOT NULL 过滤，
 *   行级数据权限下推归属列 created_by），出参 ApiResponse<Vec>。
 * - POST /contract-signatures/sign 对合同进行电子签章，
 *   入参 SignContractRequest（contract_id/i32 必填、signed_by_user_id/i32 必填、
 *   signature_image_url/String 可选、signature_certificate/String 可选）；
 *   合同已签章则返回 400 且码为 BUSINESS_ERROR（不可重复签章），
 *   合同不存在返回 404 且码为 NOT_FOUND。
 *   成功后落库 signed_at/signed_by_user_id/signature_hash（SHA-256 由合同号+客户 ID+总金额+签订日期+付款条款拼接哈希）。
 * - GET /contract-signatures/{id}/verify 验证签名完整性（重算哈希比对 stored_hash），
 *   出参 SignatureVerificationResult（contract_id/contract_no/is_valid/stored_hash/
 *   computed_hash/signed_at/signed_by_user_id）。
 *   合同不存在返回 404。
 * - POST /contract-signatures/{id}/revoke 撤销签章（清空 signed_at/signed_by_user_id/
 *   signature_hash/signature_image_url/signature_certificate），
 *   出参 ApiResponse<Model>（更新后的合同对象）。合同不存在返回 404。
 *
 * 调用方：routes/contract_signature.rs 的 contract_signatures 函数注册，
 * handler 在 handlers/contract_signature_handler.rs。
 * 入参：依赖 sales_contracts 表，合同通过 POST /sales-contracts 自建
 *   （必填 contract_name、customer_id 取 EntityContext.customerId、total_amount）。
 * 传给谁：ContractSignatureService → sea_orm → PostgreSQL（sales_contracts 签名列组）。
 * 存什么：signed_at/signed_by_user_id/signature_hash/signature_image_url/signature_certificate。
 * 存哪里：表 sales_contracts 对应列，路由前缀 /api/v1/erp/contract-signatures。
 *
 * 断言均为值判据，写操作落库回读，不用恒真、不只判 ok()。
 */

type ContractRow = {
  id: number;
  contract_no: string;
  contract_name: string;
  customer_id: number;
  total_amount: string;
  signed_at: string | null;
  signed_by_user_id: number | null;
  signature_hash: string | null;
  signature_image_url: string | null;
};

type VerifyResult = {
  contract_id: number;
  contract_no: string;
  is_valid: boolean;
  stored_hash: string | null;
  computed_hash: string;
  signed_at: string | null;
  signed_by_user_id: number | null;
};

function uniqTag(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

test.describe.serial('128 合同签名端点族签署验证撤销读回', () => {
  const cleanup: DeferredCleanup[] = [];

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test.afterEach(async ({ page }) => {
    await flushDeferredCleanups(page, cleanup);
  });

  async function seedContract(page: import('@playwright/test').Page, tag: string): Promise<number> {
    const ctx = getCtx();
    expect(ctx.customerId, '前置：EntityContext.customerId 缺失').toBeTruthy();
    const ct = await apiCall<ContractRow>(page, 'POST', '/sales-contracts', {
      contract_name: `e2e签章合同-${tag}`,
      customer_id: ctx.customerId,
      total_amount: '10000.00',
    });
    expect(ct.code, '建合同应 200').toBe(200);
    const id = ct.data.id;
    deferCleanup(cleanup, 'DELETE', `/sales-contracts/${id}`, `cs-ct-${tag}`);
    return id;
  }

  test('POST sign 后合同签名列组落库并由 verify 读回 is_valid', async ({ page }) => {
    const tag = uniqTag();
    const contractId = await seedContract(page, tag);

    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
    const userId = Number(me.id);
    expect(Number.isInteger(userId) && userId > 0, '/auth/me 必须回正整数 id').toBe(true);

    const signed = await apiCall<ContractRow>(page, 'POST', '/contract-signatures/sign', {
      contract_id: contractId,
      signed_by_user_id: userId,
    });
    expect(signed.code, '签署应 200').toBe(200);
    expect(signed.data.signed_at, '签署后 signed_at 不应为 null').not.toBeNull();
    expect(signed.data.signed_by_user_id).toBe(userId);
    expect(signed.data.signature_hash, '哈希不应为空').toBeTruthy();
    expect(signed.data.signature_hash!.length, 'SHA-256 十六进制应 64 字符').toBe(64);

    // verify 端点读回
    const verify = await apiCall<VerifyResult>(
      page,
      'GET',
      `/contract-signatures/${contractId}/verify`
    );
    expect(verify.code, '验证应 200').toBe(200);
    expect(verify.data.is_valid, '签署后验证应为 true').toBe(true);
    expect(verify.data.contract_id).toBe(contractId);
    expect(verify.data.stored_hash, 'stored_hash 应与签署写入一致').toBe(
      signed.data.signature_hash
    );
    expect(verify.data.signed_by_user_id).toBe(userId);

    // 列表端点应包含已签合同
    const list = await apiCall<ContractRow[]>(page, 'GET', '/contract-signatures');
    const hit = list.data.find(c => c.id === contractId);
    expect(hit, `已签合同列表应含 id=${contractId}`).toBeDefined();
    expect(hit!.signature_hash, '列表中哈希应与签署值一致').toBe(signed.data.signature_hash);
  });

  test('重复签署返回 400 且码为 BUSINESS_ERROR', async ({ page }) => {
    const tag = uniqTag();
    const contractId = await seedContract(page, tag);
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
    const userId = Number(me.id);

    await apiCall<ContractRow>(page, 'POST', '/contract-signatures/sign', {
      contract_id: contractId,
      signed_by_user_id: userId,
    });

    const fail = await apiCallExpectFail(page, 'POST', '/contract-signatures/sign', {
      contract_id: contractId,
      signed_by_user_id: userId,
    });
    expect(fail.status, '重复签署应 400').toBe(400);
    expect(
      failureCode(fail),
      `重复签署码应为 BUSINESS_ERROR，实际：${JSON.stringify(fail)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });

  test('POST revoke 后签名列组清空', async ({ page }) => {
    const tag = uniqTag();
    const contractId = await seedContract(page, tag);
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
    const userId = Number(me.id);

    await apiCall<ContractRow>(page, 'POST', '/contract-signatures/sign', {
      contract_id: contractId,
      signed_by_user_id: userId,
    });

    const revoked = await apiCall<ContractRow>(
      page,
      'POST',
      `/contract-signatures/${contractId}/revoke`
    );
    expect(revoked.code, '撤销应 200').toBe(200);
    expect(revoked.data.signed_at, '撤销后 signed_at 应为 null').toBeNull();
    expect(revoked.data.signed_by_user_id, '撤销后 signed_by_user_id 应为 null').toBeNull();

    // verify 读回确认清空
    const verify = await apiCall<VerifyResult>(
      page,
      'GET',
      `/contract-signatures/${contractId}/verify`
    );
    expect(verify.data.is_valid, '撤销后验证应为 false').toBe(false);
    expect(verify.data.stored_hash, '撤销后 stored_hash 应为 null').toBeNull();
  });

  test('POST sign 不存在的合同 id 返回 404', async ({ page }) => {
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
    const userId = Number(me.id);

    const fail = await apiCallExpectFail(page, 'POST', '/contract-signatures/sign', {
      contract_id: 999999999,
      signed_by_user_id: userId,
    });
    expect(fail.status).toBe(404);
    expect(failureCode(fail)).toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('GET verify 不存在的合同 id 返回 404', async ({ page }) => {
    const fail = await apiCallExpectFail(
      page,
      'GET',
      '/contract-signatures/999999999/verify'
    );
    expect(fail.status).toBe(404);
    expect(failureCode(fail)).toBe(APP_ERROR_CODES.NOT_FOUND);
  });
});
