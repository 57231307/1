// BPM 审批管理 E2E 套件 — 02 审批中心
// 创建时间: 2026-08-19
// 覆盖范围：待办审批（同意/拒绝/转交） → 已办查看 → 审批链追溯
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

/**
 * 方法二（原因：该状态无法经 UI 消费路径构造出来，属真缺陷，见下）。
 *
 * 待办/已办任务列表的渲染前提被一个前后端状态大小写不一致彻底打断：
 *   存储：bpm_task 状态用小写词表 pending/completed
 *     （backend/src/models/status/bpm_crm_contract.rs:100 bpm_task；写入见
 *      services/bpm_ops/instance.rs:77 建任务 Set(task_status::PENDING) 与
 *      services/bpm_ops/task.rs:116 审批完成 Set(task_status::COMPLETED)）。
 *   查询：handler 把过滤值硬编码为大写
 *     （backend/src/handlers/bpm_handler.rs:262 status="PENDING"、:276 status="COMPLETED"）。
 *   结果：/bpm/tasks/pending、/bpm/tasks/completed 的 Status.eq("PENDING"/"COMPLETED") 恒不命中
 *     小写行 → 待办/已办列表永远为空 → BpmApprovalPendingTable 的「同意/拒绝」与
 *     BpmApprovalCompletedTable 的「审批链」按钮对任何真实数据都不渲染。
 * 即便我用 /bpm/process/start 造出 pending 任务，列表查询仍按大写过滤而取不到，
 * 无法让按钮出现，故无法采用「定位自身行→点击→断言」的方法一。
 * 这里改为显式断言前置存在（必然执行），用例在缺陷修复前应稳定红，以暴露该大小写不一致，
 * 缺陷连同证据记入 .monkeycode/doto.md。另注：/bpm/approval/execute 还要求 handler_id/handler_name，
 * 而前端 ApprovalAction 不发（bpm-enhanced.ts executeBpmApproval），审批点击链路亦为独立契约缺陷。
 */
test.describe('02 审批中心', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 进入审批中心页面', async ({ page }) => {
    // 真实 tab 文案为「待审批」/「已审批」（src/views/bpm/approval/index.vue:105-106 →
    // i18n bpm.approval.tab.pending='待审批'、completed='已审批'），非臆测的「待办」/「已办」。
    // 二者互不为子串（待审批 vs 已审批），按名精确匹配各命中 1 个 tab。
    await page.goto('/bpm/approval');
    await expect(page.getByRole('heading', { name: '审批中心' })).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('tab', { name: '待审批' })).toBeVisible();
    await expect(page.getByRole('tab', { name: '已审批' })).toBeVisible();
  });

  test('02-02 待办任务可审批同意', async ({ page }) => {
    await page.goto('/bpm/approval');
    await page.getByRole('tab', { name: '待审批' }).click();
    const approveBtn = page.getByRole('button', { name: /同意|审批/ }).first();
    await expect(
      await approveBtn.isVisible(),
      '缺少可同意的待办任务：/bpm/tasks/pending 以大写 PENDING 过滤而任务实际存小写 pending，' +
        '待办列表恒空（前后端状态大小写不一致，见 .monkeycode/doto.md）'
    ).toBe(true);
    await approveBtn.click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 3000 });
    await page.getByLabel(/审批意见/).fill('E2E 测试：审批同意');
    await page.getByRole('button', { name: '确定' }).click();
    // 成功提示锚定 .el-message__content：审批对话框标题亦为"审批通过"（i18n bpm.approval
    // .approvalDialog.approveTitle, zh-CN.ts:2837），不限定则 getByText strict 双命中。
    await expect(page.locator('.el-message__content').filter({ hasText: /审批通过/ })).toBeVisible({
      timeout: 30000,
    });
  });

  test('02-03 待办任务可审批拒绝', async ({ page }) => {
    await page.goto('/bpm/approval');
    await page.getByRole('tab', { name: '待审批' }).click();
    const rejectBtn = page.getByRole('button', { name: /拒绝/ }).first();
    await expect(
      await rejectBtn.isVisible(),
      '缺少可拒绝的待办任务：待办列表因 PENDING/pending 大小写不一致恒空（见 .monkeycode/doto.md）'
    ).toBe(true);
    await rejectBtn.click();
    await page.getByLabel(/审批意见/).fill('E2E 测试：审批拒绝');
    await page.getByRole('button', { name: '确定' }).click();
    // 同 approve 分析：限定 toast 容器避免对话框/其他文本误匹配
    await expect(page.locator('.el-message__content').filter({ hasText: /审批拒绝/ })).toBeVisible({
      timeout: 30000,
    });
  });

  test('02-04 已办任务可追溯审批链', async ({ page }) => {
    await page.goto('/bpm/approval');
    await page.getByRole('tab', { name: '已审批' }).click();
    const chainBtn = page.getByRole('button', { name: /审批链/ }).first();
    await expect(
      await chainBtn.isVisible(),
      '缺少可追溯的已办任务：/bpm/tasks/completed 以大写 COMPLETED 过滤而已办任务实际存小写 completed，' +
        '已办列表恒空（见 .monkeycode/doto.md）'
    ).toBe(true);
    await chainBtn.click();
    await expect(page.locator('.el-dialog')).toBeVisible();
    await page.getByRole('button', { name: /关闭/ }).click();
  });
});
