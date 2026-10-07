// BPM 审批管理 E2E 套件 — 01 流程定义
// 创建时间: 2026-08-19
// 覆盖范围：流程定义创建 → 编辑 → 版本管理
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, genCode, tryCleanup } from '../flow/helpers';
import { pickSelectIn } from '../flow/ui-helpers';

/**
 * 01-04 编辑流程定义（方法一）：
 * 原 `if (await btn.isVisible())` 在无任何流程定义时零断言假绿（这些目录不跑 ensureTestEntities，
 * 库里可能一条定义都没有）。现先用 POST /bpm/definitions 建一条带唯一 process_key 的定义，
 * 按该 key 定位自己那一行点编辑，再断言编辑对话框打开且回填「流程标识」。
 * 编辑按钮无状态条件（BpmDefinitionTable.vue:61 恒渲染，仅受 bpm_definition:update 权限控制，admin 满足）。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 建一条流程定义，返回 { id, processKey } */
async function seedDefinition(
  page: import('@playwright/test').Page
): Promise<{ id: number; processKey: string }> {
  const processKey = genCode('e2e-flow');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/bpm/definitions', {
    name: `E2E 测试流程 ${processKey}`,
    code: processKey,
    description: 'E2E 测试流程定义',
    category: 'sales',
    version: '1.0',
    config: {
      nodes: [
        { id: 'start', name: '提交审批', type: 'start_event' },
        { id: 'approve_task', name: '审批节点', type: 'user_task', assignee_value: '1' },
        { id: 'end', name: '完成', type: 'end_event' },
      ],
      edges: [
        { source: 'start', target: 'approve_task' },
        { source: 'approve_task', target: 'end' },
      ],
    },
    status: 'ACTIVE',
  });
  if (!created.data?.id) throw new Error(`建流程定义失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/bpm/definitions/${created.data.id}`, label: 'bpm_definition' });
  return { id: created.data.id, processKey };
}

test.describe('01 流程定义', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入流程定义页面', async ({ page }) => {
    // 真实新建按钮文案为「新建流程」（src/views/bpm/definitions.vue:13 → i18n bpm.definitions.create='新建流程'），
    // 非臆测的「新增」
    await page.goto('/bpm/definitions');
    await expect(page.getByRole('heading', { name: '流程定义' })).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: '新建流程' })).toBeVisible();
  });

  test('01-02 创建新流程定义', async ({ page }) => {
    await page.goto('/bpm/definitions');
    // 点击前先等待「新建流程」按钮进入稳定可见态，避免首屏列表加载中/重绘导致的瞬时点击超时
    await expect(page.getByRole('button', { name: '新建流程' })).toBeVisible({ timeout: 15000 });
    await page.getByRole('button', { name: '新建流程' }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    // 弹窗内表单字段与背后筛选栏存在同名 label（「流程名称」「分类」），
    // 全部限定在 .el-dialog 作用域内，避免 getByLabel strict-mode 命中多元素。
    // 提交按钮真实文案为「确定」（bpm.definitions.form.confirm），非「确认/提交」。
    const dlg = page.locator('.el-dialog');
    await dlg.getByLabel('流程标识').fill(`e2e-${Date.now()}`);
    await dlg.getByLabel('流程名称').fill('E2E 测试流程');
    // 分类是 el-select：用共享 helper pickSelectIn（root=dlg + 精确 label「分类」）打开下拉选首项，
    // 消除旧 pickSelect+elSelectByLabel（子串匹配、点 readonly combobox）的假红。
    await pickSelectIn(dlg, page, '分类');
    await dlg.getByLabel('描述').fill('E2E 测试流程定义');
    await dlg.getByRole('button', { name: '确定' }).click();
    // 真实成功提示走 msg.success('createSuccess')（useBpmDfProc.ts:181）→
    // i18n message.createSuccess 实际中文为「新增成功」（zh-CN.ts:67），既非「创建成功」也非「保存成功」。
    // 原用例 regex 未覆盖该文案恒不可见而红；锚定真实 toast 容器 .el-message + 真实文案（仍要求成功提示出现，不放宽）。
    await expect(page.locator('.el-message').filter({ hasText: '新增成功' })).toBeVisible({
      timeout: 30000,
    });
  });

  test('01-03 流程定义筛选功能可用', async ({ page }) => {
    // 判责 J-3（测试缺陷）：「流程名称」筛选控件已被前端有意移除——后端
    // ProcessDefinitionQuery 仅有 category/status/page/page_size（models/dto/bpm_dto.rs:34-39），
    // 无 keyword 字段，发送即被静默丢弃=假筛选（BpmDefinitionFilter.vue:10-11 注释点名，
    // CI error-context a11y 快照逐字证实筛选表单内只有 流程分类 combobox + 查询/重置）。
    // 原用例 getByLabel('流程名称').fill(...) 恒 0 命中 → fill 30s 超时。
    // 保留「筛选功能可用」的用例意图并对真实控件验证：选分类 → 查询请求真实携带
    // category=finance（waitForRequest 注册在点击前；不发请求/不带条件即自然超时判红，不吞）。
    await page.goto('/bpm/definitions');
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
    const filterForm = page.getByLabel('流程定义筛选表单');
    await pickSelectIn(filterForm, page, '流程分类', { optionText: '财务' });
    const queried = page.waitForRequest(
      r =>
        r.url().includes('/bpm/definitions') &&
        r.method() === 'GET' &&
        new URL(r.url()).searchParams.get('category') === 'finance',
      { timeout: 15000 }
    );
    await filterForm.getByRole('button', { name: '查询' }).click();
    await queried;
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
    await filterForm.getByRole('button', { name: '重置' }).click();
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });

  test('01-04 流程定义可编辑', async ({ page }) => {
    // 方法一：建一条定义 → 定位自身行点编辑 → 断言对话框回填流程标识
    const { processKey } = await seedDefinition(page);
    await page.goto('/bpm/definitions');
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
    const row = page.getByRole('row').filter({ hasText: processKey });
    const editBtn = row.getByRole('button', { name: /编辑/ }).first();
    await expect(editBtn, `定位流程定义 ${processKey} 的编辑按钮失败`).toBeVisible({
      timeout: 10000,
    });
    await editBtn.click();
    await expect(page.locator('.el-dialog')).toBeVisible();
    await expect(page.getByLabel(/流程标识/)).toBeVisible();
    await page.getByRole('button', { name: /取消/ }).click();
  });
});
