import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCallRaw, BASE_URL, tryCleanup } from './helpers';
import { formItemByExactLabel } from './ui-helpers';
import {
  columnIndex,
  columnCellTexts,
  openPageTable,
  rowByText,
  rowCellText,
  rowCount,
  waitFirstRow,
} from './table-column';

// 权限域各功能页列绑定回归：逐页验证曾被判空的列（用户 部门/角色、数据权限 角色/权限范围、
// 字段权限 角色/可读/可写、权限管理 资源类型/操作/状态、角色 名称/编码/创建时间）在真实数据
// 下逐格出值。判据以单元格文本按后端字段出值为准，不接受仅断无 5xx 的软断言；种子缺失处以
// 真实接口造数再验。入口为各功能页自身路由：/users、/data-permission、/field-permissions、
// /permissions、/roles。
// 【取数口径】列值一律经列原语整列批量读取（一次查询取回整列全部单元格文本），再在断言侧
// 逐格比对真实列值与期望映射；不采用逐格串行往返，因权限目录为资源与动作的全量叉乘、
// 行数千级，逐格往返在该规模下必然拖爆用例时限。批量读不缩水断言：权限管理页对全部行
// 逐格断言非空与状态词表命中，且先断整列单元格数与行数对齐。

const PASSWORD = 'Xp9#Qr4Lm7!Zt2';

test.describe.serial('权限域列绑定回归：真实数据逐格出值（防显示空白复发）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('用户管理页：部门列出 department_name、角色列出 role_names', async ({ page }) => {
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

    const table = await openPageTable(page, '/users', '用户管理', '用户列表');
    // 关键词框按表单 label 锚定（不用位置选择器），过滤出自建用户规避分页翻找
    const keyword = formItemByExactLabel(page, '关键词').locator('input').first();
    await keyword.fill(username);
    await keyword.press('Enter');
    await waitFirstRow(table);

    const deptIdx = await columnIndex(table, '部门');
    const roleIdx = await columnIndex(table, '角色');
    const targetRow = await rowByText(table, username);

    const deptCell = await rowCellText(targetRow, deptIdx);
    const roleCell = await rowCellText(targetRow, roleIdx);
    expect(deptCell, '部门列未渲染 department_name（疑似仍绑到后端不返回的键）').toContain(
      dept.name
    );
    expect(roleCell, '角色列未渲染 role_names（疑似仍绑到后端不返回的键）').toContain(role.name);
  });

  test('数据权限页：角色名出真实 role_name 且授予行按该角色取数逐格出值', async ({ page }) => {
    const roles = await apiCallRaw<{ roles: Array<{ id: number; name: string }> }>(
      page,
      'GET',
      '/roles'
    );
    expect(roles.roles.length, '无角色可挂数据权限').toBeGreaterThan(0);
    const role = roles.roles[0];
    // upsert 一条数据权限作为种子，保证该角色名下至少一行且带 resource_type/scope_type
    await apiCallRaw(page, 'POST', '/data-permissions', {
      role_id: role.id,
      resource_type: 'customer',
      scope_type: 'DEPT',
    });

    await page.goto(`${BASE_URL}/data-permission`);
    const table = page.locator('.el-table[aria-label="数据权限列表"]').first();
    // 本页以角色为取数归属：角色名渲染在左侧角色菜单与右侧面板标题里，不在表格列内
    const roleEntry = page.getByRole('menuitem', { name: role.name, exact: true });
    await roleEntry.waitFor({ state: 'visible', timeout: 30_000 });
    await roleEntry.click();
    await expect(
      page.getByRole('heading', { level: 3, name: `${role.name} - 权限设置` }),
      `选中角色后面板标题未回显真实角色名 ${role.name}`
    ).toBeVisible({ timeout: 30_000 });
    await table.waitFor({ state: 'visible', timeout: 30_000 });

    expect(
      await rowCount(table),
      '数据权限列表行数应为正（造数后仍空即绑定/取数缺陷）'
    ).toBeGreaterThan(0);

    const resourceIdx = await columnIndex(table, '资源类型');
    const scopeIdx = await columnIndex(table, '权限范围');
    const statusIdx = await columnIndex(table, '状态');
    const targetRow = await rowByText(table, 'customer');

    expect(await rowCellText(targetRow, resourceIdx), '资源类型列未渲染 resource_type').toBe(
      'customer'
    );
    expect(await rowCellText(targetRow, scopeIdx), '权限范围列未按 scope_type=DEPT 渲染').toBe(
      '本部门'
    );
    const statusCell = await rowCellText(targetRow, statusIdx);
    // is_enabled 经 el-tag 渲染为 启用/禁用，不应出现空白单元格
    expect(['启用', '禁用'], `状态列非预期取值: ${statusCell}`).toContain(statusCell);
  });

  test('数据权限页：创建时间列出 created_at', async ({ page }) => {
    const roles = await apiCallRaw<{ roles: Array<{ id: number; name: string }> }>(
      page,
      'GET',
      '/roles'
    );
    expect(roles.roles.length, '无角色可挂数据权限').toBeGreaterThan(0);
    const role = roles.roles[0];
    await apiCallRaw(page, 'POST', '/data-permissions', {
      role_id: role.id,
      resource_type: 'customer',
      scope_type: 'DEPT',
    });

    await page.goto(`${BASE_URL}/data-permission`);
    const table = page.locator('.el-table[aria-label="数据权限列表"]').first();
    await page.getByRole('menuitem', { name: role.name, exact: true }).click();
    await table.waitFor({ state: 'visible', timeout: 30_000 });
    expect(await rowCount(table), '数据权限列表行数应为正').toBeGreaterThan(0);

    const createdIdx = await columnIndex(table, '创建时间');
    const createdCells = (await columnCellTexts(table, createdIdx)).filter(t => t.length > 0);
    expect(createdCells.length, '创建时间列全部为空（created_at 未生效）').toBeGreaterThan(0);
  });

  test('字段权限页：角色列出 role_name、可读/可写列渲染是否', async ({ page }) => {
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

    const table = await openPageTable(page, '/field-permissions', '字段权限', '字段权限列表');
    expect(await rowCount(table), '字段权限列表行数应为正').toBeGreaterThan(0);

    const roleIdx = await columnIndex(table, '角色');
    const readIdx = await columnIndex(table, '可读');
    const writeIdx = await columnIndex(table, '可写');

    const targetRow = await rowByText(table, field);

    const roleCell = await rowCellText(targetRow, roleIdx);
    expect(roleCell, '角色列未渲染 role_name').toContain(role.name);
    // can_read=true 渲染为 是，can_write=false 渲染为 否（布尔渲染走是否，非空白）
    expect(await rowCellText(targetRow, readIdx), '可读列未按 can_read 渲染').toBe('是');
    expect(await rowCellText(targetRow, writeIdx), '可写列未按 can_write 渲染').toBe('否');
  });

  test('权限管理页：资源类型/操作/状态逐格出值', async ({ page }) => {
    const table = await openPageTable(page, '/permissions', '权限管理', '权限列表');
    const n = await rowCount(table);
    expect(n, '权限目录行数应为正（GET /permissions 裸数组取数失效则整表空白）').toBeGreaterThan(0);

    const typeIdx = await columnIndex(table, '资源类型');
    const actionIdx = await columnIndex(table, '操作');
    const statusIdx = await columnIndex(table, '状态');

    const types = await columnCellTexts(table, typeIdx);
    const actions = await columnCellTexts(table, actionIdx);
    const statuses = await columnCellTexts(table, statusIdx);

    // 整列单元格数与行数对齐：批量读若因行模板错位少取或多取，先在这里显式判红
    expect(types.length, `资源类型列单元格数应与行数 ${n} 对齐`).toBe(n);
    expect(actions.length, `操作列单元格数应与行数 ${n} 对齐`).toBe(n);
    expect(statuses.length, `状态列单元格数应与行数 ${n} 对齐`).toBe(n);

    // 全行逐格断言，不按比例采样：本端点无分页、目录为全量叉乘，DOM 行数即数据全量
    for (let i = 0; i < n; i++) {
      expect(types[i], `资源类型第 ${i} 行为空`).not.toBe('');
      expect(actions[i], `操作第 ${i} 行为空`).not.toBe('');
      // allowed 布尔经 el-tag 渲染为 允许/禁止，不应出现空白单元格
      expect(['允许', '禁止'], `状态第 ${i} 行非预期取值: ${statuses[i]}`).toContain(statuses[i]);
    }
  });

  test('角色管理页：名称/编码/描述/创建时间逐格出值，状态按 is_system 渲染', async ({ page }) => {
    const stamp = String(Date.now()).slice(-6);
    const name = `列回归角色${stamp}`;
    const code = `e2ecolrole${stamp}`;
    const description = `列回归角色描述${stamp}`;
    const created = await apiCallRaw<{ id: number }>(page, 'POST', '/roles', {
      name,
      code,
      description,
    });
    expect(Number.isFinite(created.id), `建角色应返回 id，实际：${JSON.stringify(created)}`).toBe(
      true
    );
    try {
      const table = await openPageTable(page, '/roles', '角色管理', '角色列表');
      expect(await rowCount(table), '角色列表行数应为正').toBeGreaterThan(0);

      const nameIdx = await columnIndex(table, '角色名称');
      const codeIdx = await columnIndex(table, '角色编码');
      const descIdx = await columnIndex(table, '描述');
      const statusIdx = await columnIndex(table, '状态');
      const createdIdx = await columnIndex(table, '创建时间');

      const targetRow = await rowByText(table, name);
      expect(await rowCellText(targetRow, nameIdx), '角色名称列未渲染 name').toBe(name);
      expect(await rowCellText(targetRow, codeIdx), '角色编码列未渲染 code').toBe(code);
      expect(await rowCellText(targetRow, descIdx), '描述列未渲染 description').toBe(description);
      // 自建角色 is_system=false，状态列按词表渲染为 自定义（非空白）
      expect(await rowCellText(targetRow, statusIdx), '状态列未按 is_system 渲染').toBe('自定义');
      expect(
        await rowCellText(targetRow, createdIdx),
        '创建时间列未渲染 created_at（疑似绑到后端不返回的键）'
      ).not.toBe('');
    } finally {
      await tryCleanup(page, 'DELETE', `/roles/${created.id}`, `role#${created.id}`);
    }
  });
});
