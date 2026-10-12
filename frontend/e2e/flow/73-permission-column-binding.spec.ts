import { test, expect, type Locator, type Page } from '../diagnose-fixture';
import { loginViaUI, apiCallRaw, BASE_URL } from './helpers';

// 权限域列表列绑定回归：验证曾被判空的列（用户 部门/角色、数据权限 角色/创建时间、
// 字段权限 角色/可读/可写、权限管理 资源类型/操作/状态）在真实数据下逐格出值。
// 判据以「单元格文本非空」为准，不接受仅断无 5xx 的软断言；种子缺失处以真实接口造数再验。

const PASSWORD = 'E2ePassw0rd!';

// 在激活 Tab 面板内定位可见表格根节点（未激活面板以 display:none 隐藏，:visible 天然排除）
function visibleTable(page: Page): Locator {
  return page.locator('.el-tab-pane:visible .el-table').first();
}

// 按列头文案求主表头中的列下标（跳过无对应正文单元格的滚动条 gutter 列头）
async function columnIndex(page: Page, headerText: string): Promise<number> {
  const table = visibleTable(page);
  const headers = table.locator('.el-table__header-wrapper thead th.el-table__cell:not(.gutter)');
  const count = await headers.count();
  const seen: string[] = [];
  for (let i = 0; i < count; i++) {
    const text = (await headers.nth(i).innerText()).trim();
    seen.push(text);
    if (text === headerText) return i;
  }
  throw new Error(`列头「${headerText}」未找到，实际列头=${JSON.stringify(seen)}`);
}

// 取主表体某列全部单元格文本（主表体在 .el-table__body-wrapper，fixed 克隆在别处不串列）
async function columnCellTexts(page: Page, index: number): Promise<string[]> {
  const table = visibleTable(page);
  const cells = table.locator(
    `.el-table__body-wrapper .el-table__row td.el-table__cell:nth-child(${index + 1}) .cell`
  );
  const texts = await cells.allInnerTexts();
  return texts.map(s => s.trim());
}

// 取指定数据行某列单元格文本
async function rowCellText(row: Locator, index: number): Promise<string> {
  const cell = row.locator(`td.el-table__cell:nth-child(${index + 1}) .cell`).first();
  return (await cell.innerText()).trim();
}

async function rowCount(page: Page): Promise<number> {
  return visibleTable(page).locator('.el-table__body-wrapper .el-table__row').count();
}

// 切换系统设置左侧 Tab（按 Tab 文案点名），等待该 Tab 表格渲染
async function openSystemTab(page: Page, tabText: string): Promise<void> {
  await page.goto(`${BASE_URL}/system`);
  await page.locator('.el-tabs__item').first().waitFor({ state: 'visible', timeout: 30_000 });
  const tab = page
    .locator('.el-tabs__item')
    .filter({ hasText: new RegExp(`^${tabText}$`) })
    .first();
  await tab.click();
  await visibleTable(page)
    .waitFor({ state: 'visible', timeout: 30_000 })
    .catch(e =>
      console.warn(`[perm-col-regress] 等待 ${tabText} 表格失败: ${(e as Error).message}`)
    );
  await page.waitForTimeout(600);
}

test.describe.serial('权限域列绑定回归：真实数据逐格出值（防显示空白复发）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('系统设置-用户 Tab：部门列出 department_name、角色列出 role_names', async ({ page }) => {
    // 种子账号未带部门，department_name 为 null 属真实数据而非绑定缺陷；故造一条同时挂部门+角色的用户做读后写
    const roles = await apiCallRaw<{ roles: Array<{ id: number; name: string }> }>(
      page,
      'GET',
      '/roles'
    );
    expect(roles.roles.length, '无角色可用，无法造带角色的用户').toBeGreaterThan(0);
    const role = roles.roles[0];

    let depts = await apiCallRaw<{ items: Array<{ id: number; name: string }> }>(
      page,
      'GET',
      '/departments?page=1&page_size=5'
    );
    if (!depts.items || depts.items.length === 0) {
      const code = `e2e-col-d-${String(Date.now()).slice(-6)}`;
      await apiCallRaw<{ id: number }>(page, 'POST', '/departments', {
        name: `列回归部门${code}`,
        code,
      });
      depts = await apiCallRaw<{ items: Array<{ id: number; name: string }> }>(
        page,
        'GET',
        '/departments?page=1&page_size=5'
      );
    }
    expect(depts.items.length, '无部门可用，无法造带部门的用户').toBeGreaterThan(0);
    const dept = depts.items[0];

    const username = `e2ecol${String(Date.now()).slice(-7)}`;
    await apiCallRaw(page, 'POST', '/users', {
      username,
      password: PASSWORD,
      role_id: role.id,
      department_id: dept.id,
      real_name: '列回归用户',
    });

    await openSystemTab(page, '用户管理');
    // 关键字过滤定位自建用户，规避分页翻找
    const keyword = page.locator('.filter-card input').first();
    await keyword.fill(username);
    await keyword.press('Enter');
    await page
      .locator('.el-tab-pane:visible .el-table__row')
      .first()
      .waitFor({ state: 'visible', timeout: 15_000 });

    const deptIdx = await columnIndex(page, '部门');
    const roleIdx = await columnIndex(page, '角色');
    const targetRow = visibleTable(page)
      .locator('.el-table__body-wrapper .el-table__row')
      .filter({ hasText: username })
      .first();
    await targetRow.waitFor({ state: 'visible', timeout: 10_000 });

    const deptCell = await rowCellText(targetRow, deptIdx);
    const roleCell = await rowCellText(targetRow, roleIdx);
    expect(deptCell, '部门列未渲染 department_name（疑似仍绑到后端不返回的键）').toContain(
      dept.name
    );
    expect(roleCell, '角色列未渲染 role_names（疑似仍绑到后端不返回的键）').toContain(role.name);
  });

  test('系统设置-数据权限 Tab：角色列出 role_name、创建时间列出 created_at', async ({ page }) => {
    const roles = await apiCallRaw<{ roles: Array<{ id: number; name: string }> }>(
      page,
      'GET',
      '/roles'
    );
    expect(roles.roles.length, '无角色可挂数据权限').toBeGreaterThan(0);
    const role = roles.roles[0];
    // upsert 一条数据权限作为种子，保证列表至少一行且带 role_name/created_at
    await apiCallRaw(page, 'POST', '/data-permissions', {
      role_id: role.id,
      resource_type: 'customer',
      scope_type: 'DEPT',
    });

    await openSystemTab(page, '数据权限');
    expect(
      await rowCount(page),
      '数据权限列表行数应为正（造数后仍空即绑定/取数缺陷）'
    ).toBeGreaterThan(0);

    const roleIdx = await columnIndex(page, '角色');
    const createdIdx = await columnIndex(page, '创建时间');
    // 造数行必然出值：断言至少一格非空，证明列绑到真实 role_name/created_at 而非空键
    const roleCells = (await columnCellTexts(page, roleIdx)).filter(t => t.length > 0);
    const createdCells = (await columnCellTexts(page, createdIdx)).filter(t => t.length > 0);
    expect(roleCells.length, '角色列全部为空（role_name 未生效）').toBeGreaterThan(0);
    expect(roleCells.join('|')).toContain(role.name);
    expect(createdCells.length, '创建时间列全部为空（created_at 未生效）').toBeGreaterThan(0);
  });

  test('系统设置-字段权限 Tab：角色列出 role_name、可读/可写列渲染是否', async ({ page }) => {
    const roles = await apiCallRaw<{ roles: Array<{ id: number; name: string }> }>(
      page,
      'GET',
      '/roles'
    );
    expect(roles.roles.length, '无角色可挂字段权限').toBeGreaterThan(0);
    const role = roles.roles[0];
    const field = `e2e_col_${String(Date.now()).slice(-6)}`;
    await apiCallRaw(page, 'POST', '/field-permissions', {
      role_id: role.id,
      resource_type: 'customer',
      field_name: field,
      can_read: true,
      can_write: false,
    });

    await openSystemTab(page, '字段权限');
    expect(await rowCount(page), '字段权限列表行数应为正').toBeGreaterThan(0);

    const roleIdx = await columnIndex(page, '角色');
    const readIdx = await columnIndex(page, '可读');
    const writeIdx = await columnIndex(page, '可写');

    const targetRow = visibleTable(page)
      .locator('.el-table__body-wrapper .el-table__row')
      .filter({ hasText: field })
      .first();
    await targetRow.waitFor({ state: 'visible', timeout: 10_000 });

    const roleCell = await rowCellText(targetRow, roleIdx);
    expect(roleCell, '角色列未渲染 role_name').toContain(role.name);
    // can_read=true → 是，can_write=false → 否（布尔渲染走是否，非空白）
    expect(await rowCellText(targetRow, readIdx), '可读列未按 can_read 渲染').toBe('是');
    expect(await rowCellText(targetRow, writeIdx), '可写列未按 can_write 渲染').toBe('否');
  });

  test('系统设置-权限管理 Tab：资源类型/操作/状态逐格出值', async ({ page }) => {
    await openSystemTab(page, '权限管理');
    const n = await rowCount(page);
    expect(n, '权限目录行数应为正（GET /permissions 裸数组取数失效则整表空白）').toBeGreaterThan(0);

    const typeIdx = await columnIndex(page, '资源类型');
    const actionIdx = await columnIndex(page, '操作');
    const statusIdx = await columnIndex(page, '状态');

    const types = await columnCellTexts(page, typeIdx);
    const actions = await columnCellTexts(page, actionIdx);
    const statuses = await columnCellTexts(page, statusIdx);

    for (let i = 0; i < Math.min(n, 10); i++) {
      expect(types[i], `资源类型第 ${i} 行为空`).not.toBe('');
      expect(actions[i], `操作第 ${i} 行为空`).not.toBe('');
      // allowed 布尔经 el-tag 渲染为「允许/禁止」，不应出现空白单元格
      expect(['允许', '禁止'], `状态第 ${i} 行非预期取值: ${statuses[i]}`).toContain(statuses[i]);
    }
  });
});
