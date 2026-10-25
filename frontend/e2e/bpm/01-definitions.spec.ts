// BPM 审批管理 E2E 套件 — 01 流程定义
// 创建时间: 2026-08-19
// 覆盖范围：流程定义创建 → 编辑 → 版本管理
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, genCode, tryCleanup } from '../flow/helpers';
import { pickSelect, pickSelectIn } from '../flow/ui-helpers';

/**
 * 01-04 编辑流程定义的前置构造：
 * 本目录不跑 ensureTestEntities，库里可能一条流程定义都没有，"按钮可见才断言"
 * 会退化为零断言假绿。故先 POST /bpm/definitions 建一条带唯一 process_key 的定义，
 * 按该 key 定位自己那一行点编辑，再断言编辑对话框打开且回填「流程标识」。
 * 编辑按钮无状态条件（BpmDefinitionTable.vue:61 恒渲染，仅受 bpm_definition:update 权限控制，admin 满足）。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/**
 * 节点行内第一个 el-select（节点类型列）按可见下拉的文本选项。
 * 调用方：01-02 走 UI 建定义时把首行设为「开始」、末行设为「结束」。
 * 选项文本来自 i18n bpm.definitions.nodeType（zh-CN 值 开始/审批/结束），
 * 提交值才是引擎 token，界面不裸露英文常量。
 * 等待口径复用仓内既有下拉助手：点下拉外壳展开，把选项限定在最后打开的可见面板内，
 * 等选项可见且稳定后再点，点完等面板收起才返回。
 * 三行节点下拉的选项文本同名，面板又都挂到 body，跨面板取首个可见选项会命中上一行
 * 正在收起的旧面板，表现为选项一度解析到之后一直不可见直至超时；限定到最新面板并
 * 等收起收尾即消除该竞态。选项缺失或面板不收的真实缺陷仍会超时判红，不放宽。
 */
async function pickRowOption(
  page: import('@playwright/test').Page,
  row: import('@playwright/test').Locator,
  text: string
) {
  await pickSelect(page, row.locator('.el-select').first(), text, { timeout: 15_000 });
}

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
    // label 精确匹配限定在弹窗作用域内，不直接点 readonly combobox input。
    await pickSelectIn(dlg, page, '分类');
    await dlg.getByLabel('描述').fill('E2E 测试流程定义');

    // 负向：不配任何节点直接提交必须被拒。引擎解析不到可执行审批节点的定义，
    // 过去会被后端自动完成并判批准（审批绕过），故前端提交边界先拦下并点名节点顺序要求，
    // 不放行一个"保存成功但发起即绕过审批"的定义。
    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(
      page
        .locator('.el-message')
        .filter({ hasText: '节点顺序必须为：开始 → 一个或多个审批 → 结束' })
    ).toBeVisible({ timeout: 15000 });

    // 正向：按界面顺序补出 开始、审批、结束 三节点，审批节点选定审批人类型后提交成功。
    for (let i = 0; i < 3; i++) {
      await dlg.getByRole('button', { name: '添加节点' }).click();
    }
    // el-table 把 aria-label 透传到外层容器而非内层 table 元素，命中范围取容器再下钻表体行
    const nodeRows = dlg.locator('[aria-label="流程节点列表"] tbody tr');
    await expect(nodeRows, '添加三次应得到三行节点').toHaveCount(3);
    // 首行改「开始」、末行改「结束」，中间行保持默认的审批节点
    await pickRowOption(page, nodeRows.nth(0), '开始');
    await pickRowOption(page, nodeRows.nth(2), '结束');
    // 中间行保持默认审批节点，行内第二个 el-select 是审批人类型列，选项全量渲染，取首项。
    // 与节点类型列走同一下拉助手，选项作用域限定在最后打开的可见面板，点完等面板收起，
    // 避免残留面板盖住后续确定按钮；面板或候选取不到即超时判红，不放宽。
    await pickSelect(page, nodeRows.nth(1).locator('.el-select').nth(1), undefined, {
      timeout: 30_000,
    });

    await dlg.getByRole('button', { name: '确定' }).click();
    // 真实成功提示走 ElMessage.success(i18n.global.t('common.message.createSuccess'))
    // （useBpmDfProc.ts:199-202）→ 实际中文为「新增成功」（zh-CN.ts:68），
    // 既非「创建成功」（顶层 message.createSuccess）也非「保存成功」。
    // 断言锚定真实 toast 容器 .el-message + 真实文案（仍要求成功提示出现，不放宽）。
    await expect(page.locator('.el-message').filter({ hasText: '新增成功' })).toBeVisible({
      timeout: 30000,
    });
  });

  test('01-03 流程定义筛选功能可用', async ({ page }) => {
    // 「流程名称」筛选控件已被前端有意移除：后端 ProcessDefinitionQuery 仅有
    // category/status/page/page_size（models/dto/bpm_dto.rs:34-39），无 keyword 字段，
    // 发送即被静默丢弃=假筛选（移除理由见 BpmDefinitionFilter.vue:10-11 注释）。
    // 筛选表单的真实控件：流程分类 combobox + 查询/重置按钮。
    // 本用例对真实控件验证"筛选功能可用"：选分类 → 查询请求真实携带
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
