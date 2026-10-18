import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  APP_ERROR_CODES,
} from './helpers';

/**
 * 85 排程结果确认闭环（DRAFT → CONFIRMED 状态门 + 明细为空时的如实呈现）
 *
 * 判据来源（backend 逐行核实）：
 * - 排程结果词表唯一事实来源 `models/status/production.rs` 的 `scheduling` 模块：DRAFT / CONFIRMED；
 * - 状态门 `services/scheduling_query.rs::confirm_schedule_result`：
 *   非 DRAFT 确认 → `business_displayable("只有草稿状态的排程结果可以确认，当前状态：{status}")`
 *   → HTTP 400 / code=BUSINESS_ERROR 且 message 外显真实文案（含当前状态值）；
 * - 明细为空的排程结果（auto_schedule 在待排池为空或无可用工作中心时经
 *   `scheduling_auto.rs::build_empty_schedule_result` 及全组冲突路径合法产出 schedule_details=[]）
 *   确认后 handler 消息必须如实携带"无明细，未更新任何生产订单"，禁止假称已应用；
 * - 确认后详情接口 status 必须读回 CONFIRMED（防"置成功但未落库"假成功）。
 *
 * 本用例通过 `work_center_ids=[i32 上界]`（不可能存在的过滤值）把 auto_schedule 稳定收敛为
 * 空明细路径：该过滤使 load_active_work_centers 返回空集，所有待排组因无可用产能全部计入
 * conflicts，不产生任何可应用明细 ⇒ 不会改动共享库中他人的生产订单，用例可安全并行。
 */

// i32 上界：scheduling_result/schedule 过滤参数为 i32，该值不可能命中任何 work_center.id
const ABSENT_WORK_CENTER_ID = 2147483647;

interface AutoScheduleResponse {
  id?: number;
  batch_no?: string;
  schedule_details?: unknown[] | null;
}

interface ScheduleResultRow {
  id: number;
  batch_no: string;
  status: string;
}

test.describe.serial('Shard 86: 排程结果确认闭环（状态门与空明细如实呈现）', () => {
  let scheduleResultId: number | undefined;
  let scheduleBatchNo: string | undefined;

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('86-1 自动排程（空可用工作中心）产出 DRAFT 排程结果', async ({ page }) => {
    const res = await apiCall<AutoScheduleResponse>(page, 'POST', '/scheduling/auto-schedule', {
      work_center_ids: [ABSENT_WORK_CENTER_ID],
      strategy: 'priority',
    });
    scheduleResultId = res.data?.id;
    scheduleBatchNo = res.data?.batch_no;
    expect(
      scheduleResultId,
      `auto-schedule 未返回排程结果 id: ${JSON.stringify(res)}`
    ).toBeTruthy();
    expect(scheduleBatchNo, 'auto-schedule 未返回 batch_no').toBeTruthy();
    // 该过滤下无任何工单能排上，明细必为空（与 scheduling_auto.rs 全组冲突路径一致）；
    // 键缺失(undefined)直接判红，null/空数组才是"确无可排明细"的合法形状
    const details = res.data?.schedule_details;
    expect(
      details === null || (Array.isArray(details) && details.length === 0),
      `空工作中心过滤下明细键形状异常或缺失: ${JSON.stringify(res.data?.schedule_details)}`
    ).toBe(true);

    const detail = await apiCallRaw<ScheduleResultRow>(
      page,
      'GET',
      `/scheduling/results/${scheduleResultId}`
    );
    expect(detail.status, '新排程结果初始状态应为 DRAFT').toBe('DRAFT');
  });

  test('86-2 确认空明细排程结果：成功但消息如实告知未更新生产订单', async ({ page }) => {
    expect(scheduleResultId, '86-1 未产出排程结果，本用例前置失败').toBeTruthy();

    const res = await apiCall<ScheduleResultRow>(
      page,
      'POST',
      `/scheduling/results/${scheduleResultId}/confirm`
    );
    expect(res.data?.status, '确认响应未回显 CONFIRMED').toBe('CONFIRMED');
    expect(res.message, '空明细确认必须如实告知未更新任何生产订单').toContain('未更新任何生产订单');

    // 读回校验：状态确已落库（防假成功）
    const readback = await apiCallRaw<ScheduleResultRow>(
      page,
      'GET',
      `/scheduling/results/${scheduleResultId}`
    );
    expect(readback.status, '确认后详情读回状态非 CONFIRMED').toBe('CONFIRMED');
  });

  test('86-3 二次确认被状态门拒绝：精确码 BUSINESS_ERROR + 外显当前状态', async ({ page }) => {
    expect(scheduleResultId, '86-2 未确认成功，本用例前置失败').toBeTruthy();

    const fail = await apiCallExpectFail(
      page,
      'POST',
      `/scheduling/results/${scheduleResultId}/confirm`
    );
    expect(fail.status, `二次确认应恰好 400，实际 ${fail.status}`).toBe(400);
    expect(
      failureCode(fail),
      `二次确认机器码应为 BUSINESS_ERROR，实际体: ${JSON.stringify(fail)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    // business_displayable 链路：message 必须外显真实拒绝原因并点名当前状态值
    expect(fail.message ?? '', '拒绝文案应点名草稿门规则').toContain('只有草稿状态');
    expect(fail.message ?? '', '拒绝文案应点名当前状态 CONFIRMED').toContain('CONFIRMED');
  });

  test('86-4 历史列表读回已确认批次（列表刷新口径）', async ({ page }) => {
    expect(scheduleBatchNo, '86-1 未记录 batch_no，本用例前置失败').toBeTruthy();

    const list = await apiCallRaw<{ items: ScheduleResultRow[] }>(
      page,
      'GET',
      '/scheduling/history?page=1&page_size=50'
    );
    const row = (list.items ?? []).find(item => item.batch_no === scheduleBatchNo);
    expect(row, `历史列表未包含本批次 ${scheduleBatchNo}`).toBeTruthy();
    expect(row?.status, '历史列表中本批次状态应读回 CONFIRMED').toBe('CONFIRMED');
  });
});
