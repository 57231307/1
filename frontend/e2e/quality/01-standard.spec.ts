// 质量管理 E2E 套件 — 01 质量标准
// 创建时间: 2026-08-19
// 覆盖范围：质量标准创建 → 审批 → 发布（完整状态流转）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, genCode, tryCleanup } from '../flow/helpers';
import { pickSelectIn, fillFieldByLabel, formItemByExactLabel } from '../flow/ui-helpers';

/**
 * 前置数据构造（方法一）：
 * 原 `if (await btn.isVisible()) {...}` 使无对应状态记录时零断言假绿。
 * 后端 quality_standard 词表（backend/src/models/status/quality_dyeing.rs quality_standard）：
 *   draft / approved / rejected（建单强制 draft，见 quality_standard_service.rs:121），
 *   approve → approved（:263），publish → master_data::ACTIVE（:366）。
 * 前端 StandardTab.vue 审批按钮渲染条件 status==='draft'（:109），发布按钮 status==='approved'（:117）。
 * 故 01-03 建一张 draft 标准即可；01-04 需先建单再审批至 approved 才出发布按钮。
 *
 * 另记真缺陷：publish 成功后后端把状态置为 'active'（master_data::ACTIVE），
 * 而前端状态标签词表（StandardTab.vue:158-166）只认 draft/approved/published/rejected，
 * 无 'active'——已发布标准在列表里会显示原始字符串而非「已发布」。详见 .monkeycode/doto.md。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 建一张 draft 质量标准，返回 { id, code }；登记清理 */
async function seedStandard(
  page: import('@playwright/test').Page
): Promise<{ id: number; code: string }> {
  const code = genCode('E2E-QS');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/quality-standards', {
    standard_code: code,
    standard_name: `E2E 测试质量标准 ${code}`,
    standard_type: 'product',
    version: '1.0',
    content: 'E2E 测试质量标准内容',
  });
  if (!created.data?.id) throw new Error(`建质量标准失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/quality-standards/${created.data.id}`, label: 'quality_standard' });
  return { id: created.data.id, code };
}

async function gotoStandardTab(page: import('@playwright/test').Page): Promise<void> {
  await page.goto('/quality');
  await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
}

test.describe('01 质量标准', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入质量管理页面', async ({ page }) => {
    await page.goto('/quality');
    await expect(page.getByRole('tab', { name: /标准/ })).toBeVisible({ timeout: 30000 });
  });

  test('01-02 新建质量标准', async ({ page }) => {
    await page.goto('/quality');
    await page.getByRole('button', { name: '新建标准' }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    // 真实 label 为「标准编号」（quality.standardDialog.standardCode，原用例误写「标准编码」）、
    // 「标准内容」；版本默认预置 1.0、类型默认 product（必填已满足），提交按钮真实「确定」，
    // 成功提示为 quality.message.operationSuccess「操作成功」（非「创建成功/保存成功」）。限定 .el-dialog。
    const dlg = page.locator('.el-dialog:visible').last();
    await fillFieldByLabel(dlg, page, '标准编号', `QS-${Date.now()}`);
    await fillFieldByLabel(dlg, page, '标准名称', 'E2E 测试质量标准');
    // 「类型」为 el-select（选项 产品标准/工艺标准）：旧 pickSelect+elSelectByLabel 命中只读
    // 内层 input（被 placeholder 拦 → click 超时）。改用冻结 helper 精确 label 锚定，选首项。
    await pickSelectIn(dlg, page, '类型');
    // 标准内容为 el-input type=textarea（真 <textarea>），fillFieldByLabel 只命中 input，单独定位
    await formItemByExactLabel(dlg, '标准内容')
      .locator('textarea')
      .first()
      .fill('E2E 测试质量标准内容');
    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 30000 });
  });

  test('01-03 草稿标准可审批通过', async ({ page }) => {
    // 方法一：建 draft 标准（审批按钮 status==='draft'），定位自身行点击审批
    const { code } = await seedStandard(page);
    await gotoStandardTab(page);
    const row = page.getByRole('row').filter({ hasText: code });
    const approveBtn = row.getByText('审批', { exact: false }).first();
    await expect(approveBtn, `定位 draft 标准 ${code} 的审批按钮失败`).toBeVisible({
      timeout: 10000,
    });
    await approveBtn.click();
    // 审批对话框字段与主表单同名（标准编号/标准名称/版本），审批意见为必填 textarea：
    // 限定到可见对话框 + 精确 label 填充，避免跨容器命中；「通过」按钮真实文案 pass='通过'。
    const approveDlg = page.locator('.el-dialog:visible').last();
    await expect(approveDlg).toBeVisible({ timeout: 10000 });
    // 审批意见为 el-input type=textarea（真 <textarea>），fillFieldByLabel 只命中 input，单独定位
    await formItemByExactLabel(approveDlg, '审批意见')
      .locator('textarea')
      .first()
      .fill('E2E 测试：审批通过');
    await approveDlg.getByRole('button', { name: '通过' }).click();
    // 注：审批经 ElMessageBox 之外的 confirmApprove→approveQualityStandard(id) 落库，成功弹
    // quality.message.approveSuccess=「审批成功」。后端 QualityApproveRequest.approval_comment
    // 为 Option（UI 不传 body 亦可通过）→ 若本断言仍红，属登录用户审批权限/状态机侧真实缺陷，
    // 不改断言方向掩盖（需带真实后端复核）。
    await expect(page.getByText(/审批成功/)).toBeVisible({ timeout: 30000 });
  });

  test('01-04 已审批标准可发布', async ({ page }) => {
    // 方法一：建单后经 API 审批至 approved（发布按钮渲染条件 status==='approved'），
    // 再定位自身行点击发布
    const { id, code } = await seedStandard(page);
    await apiCall(page, 'POST', `/quality-standards/${id}/approve`, {
      approval_comment: 'E2E：审批以构造发布前置',
    });
    await gotoStandardTab(page);
    const row = page.getByRole('row').filter({ hasText: code });
    const publishBtn = row.getByText('发布', { exact: false }).first();
    await expect(publishBtn, `定位 approved 标准 ${code} 的发布按钮失败`).toBeVisible({
      timeout: 10000,
    });
    await publishBtn.click();
    await page.getByRole('button', { name: /确定/ }).click();
    await expect(page.getByText(/发布成功/)).toBeVisible({ timeout: 30000 });
  });
});
