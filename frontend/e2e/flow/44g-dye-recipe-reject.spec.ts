import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  ensureTestEntities,
  apiCall,
  apiCallExpectFail,
  tryCleanup,
  expectStateGateRejection,
} from './helpers';

/**
 * 染色配方「审核拒绝」全流转契约级验证（建单→提交审核→拒绝→回读）。
 *
 * rule provenance（对照后端 handler/服务实际契约，勿改后端）：
 * - 建单：POST /production/dye-recipes（dye_recipe_handler.rs，CreateDyeRecipeRequest
 *   color_code 必填 NOT NULL，染色域建表迁移即此形态），初始态 draft。
 * - 提交：POST /production/dye-recipes/{id}/submit（draft→pending_approval）。
 * - 拒绝：POST /production/dye-recipes/{id}/reject，请求体 {reason}（必填非空），
 *   成功返回整行配方（含新列 rejected_reason），状态写 rejected；
 *   service.reject 的状态门 pending_approval→rejected，非 pending_approval → 400 BUSINESS_ERROR。
 * - 回读：GET /production/dye-recipes/{id} 断言 status/rejected_reason 落库。
 */

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

test.describe.serial('染色配方审核拒绝全流转', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('DR-1 draft→submit→reject：status=rejected 且 rejected_reason 落库', async ({ page }) => {
    const suffix = Date.now().toString().slice(-6);
    const created = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-recipes', {
      recipe_name: `E2E拒绝配方${suffix}`,
      color_code: `DR${suffix}`,
      color_name: `E2E 拒绝色名 ${suffix}`,
    });
    const id = created?.data?.id;
    expect(id, '建单应返回 id').toBeTruthy();
    CLEANUP.push({ path: `/production/dye-recipes/${id}`, label: `[DR-1] 配方 ${id}` });

    await apiCall(page, 'POST', `/production/dye-recipes/${id}/submit`);

    const reason = `色牢度不达标，退回重配 ${suffix}`;
    const rejectRes = await apiCall<{ status?: string; rejected_reason?: string | null }>(
      page,
      'POST',
      `/production/dye-recipes/${id}/reject`,
      { reason }
    );
    expect(rejectRes?.data?.status, '拒绝响应应回写 rejected').toBe('rejected');
    expect(rejectRes?.data?.rejected_reason, '拒绝响应应回传 rejected_reason').toBe(reason);

    // 回读列表/详情，确认 status 与 rejected_reason 真实落库，而非仅接口回显
    const readBack = await apiCall<{ status?: string; rejected_reason?: string | null }>(
      page,
      'GET',
      `/production/dye-recipes/${id}`
    );
    expect(readBack?.data?.status, '回读 status 应为 rejected').toBe('rejected');
    expect(readBack?.data?.rejected_reason, '回读 rejected_reason 应等于提交理由').toBe(reason);
  });

  test('DR-2 非 pending_approval（draft）拒绝被状态门拒绝 400 BUSINESS_ERROR', async ({ page }) => {
    const suffix = Date.now().toString().slice(-6);
    const created = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-recipes', {
      recipe_name: `E2E未提交配方${suffix}`,
      color_code: `DR${suffix}`,
      color_name: `E2E 未提交色名 ${suffix}`,
    });
    const id = created?.data?.id;
    expect(id, '建单应返回 id').toBeTruthy();
    CLEANUP.push({ path: `/production/dye-recipes/${id}`, label: `[DR-2] 配方 ${id}` });

    // draft 未提交审核，直接拒绝应被 pending_approval→rejected 状态门拒绝
    const fail = await apiCallExpectFail(page, 'POST', `/production/dye-recipes/${id}/reject`, {
      reason: '草稿态不应允许拒绝',
    });
    expectStateGateRejection(fail, '对 draft 配方执行 reject 应被状态门拒绝（400 BUSINESS_ERROR）');
  });
});
