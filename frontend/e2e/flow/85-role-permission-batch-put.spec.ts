// 角色授权整批替换（PUT /roles/{id}/permissions）行为契约 + 权限目录词表化显示
//
// 本文件锁住的事实：
//  1) 保存一次授权只发一次 PUT 到 /roles/{id}/permissions，且不出现逐点
//     POST /roles/{id}/permissions 与 DELETE /roles/permissions/{id}：整批替换是该端点的
//     行为契约，逐点写会留下"部分失败仍报成功"的假成功面。PUT 请求体同时被钉成
//     [{resource_type,action,allowed}] 裸数组（字段名与后端批量授予项结构逐字一致）。
//  2) PUT 之后 GET 读回的角色级授予与提交的目标终集合逐元素相等（不多不少、每项 allowed
//     相同、resource_id 恒为 null），证明提示与落库真值一致，不是"提示成功但没落库"；
//     取消勾选（删除态）与新增勾选走同一条断言链。
//  3) 授权请求被拒时失败可见：后端 message 出现在页面上、无成功提示、对话框不关闭、
//     库里集合零漂移。
//  4) 非管理员会话：该 PUT 被精确拒绝（403 + FORBIDDEN 机器码，不用状态区间宽容），
//     且权限配置对话框不渲染保存按钮，即不存在"点了必失败"的死入口。
//  5) 权限目录页的资源/动作列：动作一律出可读标签（后端可授予动作都有词条）；资源 token
//     在词表内出标签、不在词表内按原码显示，两者都不许出现空白或伪造中文名。
//
// 夹具纪律：角色/账号全部自建且唯一命名，用例结束在 finally 里清理，不触碰受保护种子；
// 写请求一律走 helpers 中带 CSRF 头的 apiCall/apiCallRaw/apiCallExpectFail；
// 角色权限出参按后端真实形状（裸数组）断言，不按分页 items 断言。
import { test, expect, type Locator, type Page } from '../diagnose-fixture';
import type { Request } from '@playwright/test';
import {
  APP_ERROR_CODES,
  BASE_URL,
  apiCallExpectFail,
  apiCallRaw,
  failureCode,
  loginViaUI,
  tryCleanup,
} from './helpers';

interface RoleRow {
  id: number;
  name: string;
  code: string;
  is_system: boolean;
}

interface GrantRow {
  id: number;
  resource_type: string;
  resource_id: number | null;
  action: string;
  allowed: boolean;
}

interface CatalogRow {
  resource_type: string;
  action: string;
  allowed: boolean;
}

/** PUT/POST 端点：/roles/{roleId}/permissions；单条删除端点：/roles/{permissionId} */
const ROLE_PERM_URL = /\/api\/v1\/erp\/roles\/\d+\/permissions$/;
const PERM_ROW_URL = /\/api\/v1\/erp\/roles\/permissions\/\d+$/;

const READ_USERS = { resource_type: 'users', action: 'read' };
const READ_DEPARTMENTS = { resource_type: 'departments', action: 'read' };
const READ_ROLES = { resource_type: 'roles', action: 'read' };
const READ_PERMISSIONS = { resource_type: 'permissions', action: 'read' };

const grantKey = (row: { resource_type: string; action: string }): string =>
  `${row.resource_type}:${row.action}`;

/** 唯一命名自建角色（非内置、可写可删），返回 GET 回读确认过的角色行 */
async function createScratchRole(page: Page, tag: string): Promise<RoleRow> {
  const stamp = `${Date.now().toString().slice(-6)}${Math.floor(Math.random() * 1000)
    .toString()
    .padStart(3, '0')}`;
  const code = `e2bput${tag.toLowerCase()}${stamp}`;
  const name = `E2E整批授权${tag}${stamp}`;
  const created = await apiCallRaw<{ id: number }>(page, 'POST', '/roles', { name, code });
  expect(Number.isFinite(created.id), `建角色应返回 id，实际：${JSON.stringify(created)}`).toBe(
    true
  );
  const row = await apiCallRaw<RoleRow>(page, 'GET', `/roles/${created.id}`);
  expect(row.code, `角色回读 code 应为 ${code}`).toBe(code);
  expect(row.is_system, '自建角色不应是内置角色（内置角色不可改授权）').toBe(false);
  return { id: row.id, name, code: row.code, is_system: row.is_system };
}

/** 单条赋权端点写一条角色级授予（仅作夹具前置，不在被测保存路径内） */
async function seedRoleGrant(
  page: Page,
  roleId: number,
  grant: { resource_type: string; action: string }
): Promise<void> {
  await apiCallRaw<GrantRow>(page, 'POST', `/roles/${roleId}/permissions`, {
    resource_type: grant.resource_type,
    action: grant.action,
    allowed: true,
  });
}

/** 读回角色级授予：出参必须是裸数组（后端 Vec<PermissionResponse>，无分页信封） */
async function readRoleGrants(page: Page, roleId: number): Promise<GrantRow[]> {
  const rows = await apiCallRaw<GrantRow[]>(page, 'GET', `/roles/${roleId}/permissions`);
  expect(
    Array.isArray(rows),
    `角色权限应为裸数组，实际：${JSON.stringify(rows).slice(0, 200)}`
  ).toBe(true);
  return rows;
}

/**
 * 记录保存路径实际发出的授权写请求（方法 + PUT 请求体）。
 * 必须在夹具前置写请求之后开始记录，否则把自建夹具的 POST 也算进被测面。
 */
function trackGrantRequests(page: Page): {
  count: (method: 'PUT' | 'POST' | 'DELETE') => number;
  putBodies: () => string[];
  stop: () => void;
} {
  const seen: Request[] = [];
  const onRequest = (req: Request) => {
    if (ROLE_PERM_URL.test(req.url()) || PERM_ROW_URL.test(req.url())) seen.push(req);
  };
  page.on('request', onRequest);
  return {
    count: method => seen.filter(req => req.method() === method).length,
    putBodies: () => seen.filter(req => req.method() === 'PUT').map(req => req.postData() ?? ''),
    stop: () => page.off('request', onRequest),
  };
}

/** 进系统设置-角色管理 Tab，按角色名打开其权限配置对话框 */
async function openPermissionDialog(page: Page, roleName: string): Promise<Locator> {
  await page.goto(`${BASE_URL}/system`);
  await page.locator('.el-tabs__item').first().waitFor({ state: 'visible', timeout: 30_000 });
  await page
    .locator('.el-tabs__item')
    .filter({ hasText: /^角色管理$/ })
    .first()
    .click();
  const table = page.locator('.el-tab-pane:visible .el-table').first();
  await table.waitFor({ state: 'visible', timeout: 30_000 });
  const row = table
    .locator('.el-table__body-wrapper .el-table__row')
    .filter({ hasText: roleName })
    .first();
  await row.waitFor({ state: 'visible', timeout: 30_000 });
  await row
    .locator('button.el-button')
    .filter({ hasText: /^权限$/ })
    .first()
    .click();
  // 对话框标题含角色名，用它把权限配置对话框与同页其它对话框区分开
  const dialog = page.locator('.el-dialog:visible').filter({ hasText: roleName }).first();
  await dialog.waitFor({ state: 'visible', timeout: 30_000 });
  await dialog.locator('.el-tree-node').first().waitFor({ state: 'visible', timeout: 30_000 });
  return dialog;
}

/** 定位某个资源+动作的叶子节点行（分组行文本只有资源名，不会与 `资源:动作` 相混） */
function leafRow(dialog: Locator, resourceType: string, action: string): Locator {
  return dialog
    .locator('.el-tree-node__content')
    .filter({ hasText: new RegExp(`^${resourceType}:${action}$`) })
    .first();
}

/** 读取叶子节点勾选态：el-tree 的复选框状态在原生 input 上 */
async function leafChecked(
  dialog: Locator,
  resourceType: string,
  action: string
): Promise<boolean> {
  const leaf = leafRow(dialog, resourceType, action);
  await leaf.waitFor({ state: 'visible', timeout: 30_000 });
  return await leaf.locator('.el-checkbox input').first().isChecked();
}

/** 展开资源分组（el-tree 展开后才渲染子节点），再点叶子复选框切换勾选 */
async function toggleLeaf(
  page: Page,
  dialog: Locator,
  resourceType: string,
  action: string
): Promise<void> {
  const groupRow = dialog
    .locator('.el-tree-node__content')
    .filter({ hasText: new RegExp(`^${resourceType}$`) })
    .first();
  await groupRow.waitFor({ state: 'visible', timeout: 30_000 });
  const expandIcon = groupRow.locator('.el-tree-node__expand-icon').first();
  const iconClass = (await expandIcon.getAttribute('class')) ?? '';
  if (!iconClass.includes('is-expanded')) {
    await expandIcon.click();
    await page.waitForTimeout(300);
  }
  const leaf = leafRow(dialog, resourceType, action);
  await leaf.waitFor({ state: 'visible', timeout: 30_000 });
  await leaf.locator('.el-checkbox__inner').first().click();
}

/** 点保存并等 PUT 落定，返回该 PUT 的 HTTP 状态码 */
async function clickSaveAndWaitPut(page: Page, dialog: Locator): Promise<number> {
  const putResponse = page.waitForResponse(
    res => ROLE_PERM_URL.test(res.url()) && res.request().method() === 'PUT',
    { timeout: 60_000 }
  );
  await dialog
    .locator('.el-dialog__footer button.el-button')
    .filter({ hasText: /^保存$/ })
    .first()
    .click();
  const res = await putResponse;
  return res.status();
}

/** 对话框底部的按钮计数（按按钮文案），用于断言保存入口是否渲染 */
async function footerButtonCount(dialog: Locator, text: RegExp): Promise<number> {
  return await dialog
    .locator('.el-dialog__footer button.el-button')
    .filter({ hasText: text })
    .count();
}

test.describe('角色授权整批替换契约与权限目录词表化', () => {
  test('85-01 保存授权只发一次 PUT，载荷为裸数组，读回终集合逐项相等', async ({ page }) => {
    await loginViaUI(page);
    const role = await createScratchRole(page, 'A1');
    try {
      const tracker = trackGrantRequests(page);
      try {
        const dialog = await openPermissionDialog(page, role.name);
        // 新建角色的授予面应为空：非空即夹具污染，后面的"逐元素相等"失去意义
        expect(await readRoleGrants(page, role.id), '新建角色的授予应为空集').toEqual([]);

        await toggleLeaf(page, dialog, READ_USERS.resource_type, READ_USERS.action);
        expect(
          await leafChecked(dialog, READ_USERS.resource_type, READ_USERS.action),
          'users:read 应被勾选'
        ).toBe(true);
        await toggleLeaf(page, dialog, READ_DEPARTMENTS.resource_type, READ_DEPARTMENTS.action);
        expect(
          await leafChecked(dialog, READ_DEPARTMENTS.resource_type, READ_DEPARTMENTS.action),
          'departments:read 应被勾选'
        ).toBe(true);

        expect(await clickSaveAndWaitPut(page, dialog), '整批替换应返回 200').toBe(200);
        expect(tracker.count('PUT'), '保存一次授权应恰好发出一次 PUT').toBe(1);
        expect(tracker.count('POST'), '不应再出现逐点 POST 赋权').toBe(0);
        expect(tracker.count('DELETE'), '不应再出现逐点 DELETE 关联行').toBe(0);

        const rawBody = tracker.putBodies()[0];
        const payload = JSON.parse(rawBody) as unknown;
        expect(Array.isArray(payload), `PUT 请求体应为裸数组，实际：${rawBody}`).toBe(true);
        const entries = payload as Array<{
          resource_type: string;
          action: string;
          allowed: unknown;
        }>;
        expect(entries.map(grantKey).sort(), 'PUT 载荷集合应为提交的目标终集合').toEqual([
          'departments:read',
          'users:read',
        ]);
        expect(
          entries.every(
            item =>
              Object.keys(item).sort().join(',') === 'action,allowed,resource_type' &&
              item.allowed === true
          ),
          `PUT 载荷字段必须与后端批量授予项结构逐字一致（resource_type/action/allowed），实际：${rawBody}`
        ).toBe(true);

        await expect(page.locator('.el-message--success').first()).toContainText('权限配置成功', {
          timeout: 10_000,
        });

        const rows = await readRoleGrants(page, role.id);
        expect(rows.map(grantKey).sort(), '读回集合应与提交的目标终集合不多不少').toEqual([
          'departments:read',
          'users:read',
        ]);
        expect(
          rows.every(row => row.allowed === true),
          '读回的每一项都应是允许态（allowed=true）'
        ).toBe(true);
        expect(
          rows.every(row => row.resource_id === null),
          '整批替换只作用于角色级授予行（resource_id 为 null）'
        ).toBe(true);
      } finally {
        tracker.stop();
      }
    } finally {
      await tryCleanup(page, 'DELETE', `/roles/${role.id}`, `role#${role.id}`);
    }
  });

  test('85-02 取消勾选也仅一次 PUT，读回集合等于剩下的终集合', async ({ page }) => {
    await loginViaUI(page);
    const role = await createScratchRole(page, 'A2');
    try {
      await seedRoleGrant(page, role.id, READ_USERS);
      await seedRoleGrant(page, role.id, READ_DEPARTMENTS);
      const before = (await readRoleGrants(page, role.id)).map(grantKey).sort();
      expect(before, '前置应有两条授予').toEqual(['departments:read', 'users:read']);

      const tracker = trackGrantRequests(page);
      try {
        const dialog = await openPermissionDialog(page, role.name);
        expect(
          await leafChecked(dialog, READ_DEPARTMENTS.resource_type, READ_DEPARTMENTS.action),
          '已落库的授予应在对话框中呈勾选态'
        ).toBe(true);
        await toggleLeaf(page, dialog, READ_DEPARTMENTS.resource_type, READ_DEPARTMENTS.action);
        expect(
          await leafChecked(dialog, READ_DEPARTMENTS.resource_type, READ_DEPARTMENTS.action),
          'departments:read 应被取消勾选'
        ).toBe(false);

        expect(await clickSaveAndWaitPut(page, dialog), '整批替换应返回 200').toBe(200);
        expect(tracker.count('PUT'), '删除态同样由一次 PUT 表达').toBe(1);
        expect(tracker.count('DELETE'), '不应再出现逐点 DELETE 关联行').toBe(0);
        expect(tracker.count('POST'), '不应再出现逐点 POST 赋权').toBe(0);

        const after = await readRoleGrants(page, role.id);
        expect(after.map(grantKey).sort(), '读回应只剩目标终集合的项').toEqual(['users:read']);
        expect(
          after.every(row => row.allowed === true),
          '剩下的项仍应是允许态'
        ).toBe(true);
      } finally {
        tracker.stop();
      }
    } finally {
      await tryCleanup(page, 'DELETE', `/roles/${role.id}`, `role#${role.id}`);
    }
  });

  test('85-03 授权被拒时失败可见：无成功提示、对话框不关闭、库里零漂移', async ({ page }) => {
    await loginViaUI(page);
    const role = await createScratchRole(page, 'A3');
    const rejectedMessage = 'E2E 注入的授权拒绝文案';
    const putPattern = '**/api/v1/erp/roles/*/permissions';
    try {
      await seedRoleGrant(page, role.id, READ_USERS);
      const before = await readRoleGrants(page, role.id);

      const tracker = trackGrantRequests(page);
      // 用统一失败信封的真实形状（403 + FORBIDDEN）替换 PUT 响应，专测前端对拒绝的处理；
      // 非管理员在权限门下的真实拒绝由 85-04 直接断言。
      await page.route(putPattern, async route => {
        if (route.request().method() !== 'PUT') {
          await route.continue();
          return;
        }
        await route.fulfill({
          status: 403,
          contentType: 'application/json',
          body: JSON.stringify({
            code: APP_ERROR_CODES.FORBIDDEN,
            message: rejectedMessage,
            trace_id: 'e2e-injected',
            timestamp: 0,
          }),
        });
      });
      try {
        const dialog = await openPermissionDialog(page, role.name);
        await toggleLeaf(page, dialog, READ_DEPARTMENTS.resource_type, READ_DEPARTMENTS.action);
        await clickSaveAndWaitPut(page, dialog);

        expect(tracker.count('PUT'), '被拒的保存同样只应发一次 PUT').toBe(1);
        expect(tracker.count('POST'), '被拒路径上不得出现逐点 POST').toBe(0);
        expect(tracker.count('DELETE'), '被拒路径上不得出现逐点 DELETE').toBe(0);

        await expect(page.getByText(rejectedMessage).first()).toBeVisible({ timeout: 10_000 });
        expect(await page.locator('.el-message--success').count(), '被拒后不得出现成功提示').toBe(
          0
        );
        await expect(dialog, '失败时对话框不应按成功关闭').toBeVisible();

        const after = await readRoleGrants(page, role.id);
        expect(after.map(grantKey).sort(), '被拒后库里集合不应发生半截漂移').toEqual(
          before.map(grantKey).sort()
        );
      } finally {
        tracker.stop();
        await page.unroute(putPattern).catch(e => {
          console.warn(`[85-03] 取消路由拦截失败: ${(e as Error).message}`);
        });
      }
    } finally {
      await tryCleanup(page, 'DELETE', `/roles/${role.id}`, `role#${role.id}`);
    }
  });

  test('85-04 非管理员：PUT 精确 403+FORBIDDEN，对话框不渲染保存按钮', async ({ page }) => {
    await loginViaUI(page);
    const targetRole = await createScratchRole(page, 'B1');
    const operatorRole = await createScratchRole(page, 'B2');
    const stamp = Date.now().toString().slice(-6);
    // 密码避开 4 连数字行子串（后端键盘序列硬错误），用字母把毫秒尾数断成两段
    const password = `E2p!${stamp.slice(0, 3)}v${stamp.slice(3)}B9qX`;
    const username = `e2bputb${stamp}`;
    let userId = 0;
    try {
      await seedRoleGrant(page, operatorRole.id, READ_USERS);
      await seedRoleGrant(page, operatorRole.id, READ_ROLES);
      await seedRoleGrant(page, operatorRole.id, READ_PERMISSIONS);
      const createdUser = await apiCallRaw<{ id: number }>(page, 'POST', '/users', {
        username,
        password,
        role_id: operatorRole.id,
      });
      expect(
        Number.isFinite(createdUser.id),
        `建用户应返回 id，实际：${JSON.stringify(createdUser)}`
      ).toBe(true);
      userId = createdUser.id;

      const before = await readRoleGrants(page, targetRole.id);

      await loginViaUI(page, username, password, true);
      const dialog = await openPermissionDialog(page, targetRole.name);
      expect(
        await footerButtonCount(dialog, /^保存$/),
        '非管理员不应看到整批替换保存按钮（该端点后端仅管理员可用）'
      ).toBe(0);
      expect(
        await footerButtonCount(dialog, /^取消$/),
        '查看授予对非管理员仍可用，取消按钮应保留'
      ).toBe(1);

      const fail = await apiCallExpectFail(page, 'PUT', `/roles/${targetRole.id}/permissions`, [
        { resource_type: READ_USERS.resource_type, action: READ_USERS.action, allowed: true },
      ]);
      expect(fail.status, `非管理员整批替换应 403，实际 ${fail.status}`).toBe(403);
      expect(
        failureCode(fail),
        `403 必须归因到机器码 ${APP_ERROR_CODES.FORBIDDEN}，实际：${JSON.stringify(fail).slice(0, 200)}`
      ).toBe(APP_ERROR_CODES.FORBIDDEN);

      const after = await readRoleGrants(page, targetRole.id);
      expect(after.map(grantKey).sort(), '被拒的整批替换不得改动目标角色授予').toEqual(
        before.map(grantKey).sort()
      );
    } finally {
      // 清理要回到管理员会话：非管理员删除用户/角色必被权限门拒绝
      await loginViaUI(page, undefined, undefined, true).catch(e => {
        console.warn(`[85-04] 恢复管理员会话失败，后续清理可能残留: ${(e as Error).message}`);
      });
      if (userId > 0) await tryCleanup(page, 'DELETE', `/users/${userId}`, `user#${userId}`);
      await tryCleanup(page, 'DELETE', `/roles/${operatorRole.id}`, `role#${operatorRole.id}`);
      await tryCleanup(page, 'DELETE', `/roles/${targetRole.id}`, `role#${targetRole.id}`);
    }
  });

  test('85-05 权限目录页：动作出可读标签，资源有词条出标签、无词条按原码', async ({ page }) => {
    await loginViaUI(page);
    const catalog = await apiCallRaw<CatalogRow[]>(page, 'GET', '/permissions');
    expect(
      Array.isArray(catalog),
      `权限目录应为裸数组，实际：${JSON.stringify(catalog).slice(0, 200)}`
    ).toBe(true);
    expect(catalog.length, '权限目录应非空，否则本页断言无意义').toBeGreaterThan(0);
    const vocab = await apiCallRaw<string[]>(page, 'GET', '/data-permissions/resource-types');
    expect(Array.isArray(vocab), '资源词表端点应返回裸数组').toBe(true);
    const vocabSet = new Set(vocab);

    await page.goto(`${BASE_URL}/system`);
    await page.locator('.el-tabs__item').first().waitFor({ state: 'visible', timeout: 30_000 });
    await page
      .locator('.el-tabs__item')
      .filter({ hasText: /^权限管理$/ })
      .first()
      .click();
    const table = page.locator('.el-tab-pane:visible .el-table').first();
    await table.waitFor({ state: 'visible', timeout: 30_000 });

    const headers = table.locator('.el-table__header-wrapper thead th.el-table__cell');
    await expect(headers.filter({ hasText: /^资源类型$/ }).first()).toBeVisible();
    await expect(headers.filter({ hasText: /^操作$/ }).first()).toBeVisible();

    const rows = table.locator('.el-table__body-wrapper .el-table__row');
    await expect(rows.first()).toBeVisible({ timeout: 30_000 });
    const rendered = await rows.count();
    expect(rendered, '渲染行数不应少于目录条数').toBeGreaterThanOrEqual(
      Math.min(catalog.length, 20)
    );
    const probeCount = Math.min(20, catalog.length, rendered);

    for (let i = 0; i < probeCount; i++) {
      const row = rows.nth(i);
      const cells = row.locator('td.el-table__cell');
      const resourceCell = (await cells.nth(0).innerText()).trim();
      const actionCell = (await cells.nth(1).innerText()).trim();
      const token = catalog[i];

      expect(resourceCell, `第 ${i} 行资源列不应为空`).not.toBe('');
      expect(actionCell, `第 ${i} 行动作列不应为空`).not.toBe('');
      // 动作列：后端可授予动作全部有词条，任何一行仍显示原码即标签化未生效
      expect(actionCell, `第 ${i} 行动作列不应显示原码 token`).not.toBe(token.action);
      // 取不到文案时 vue-i18n 会把键路径渲染到界面，这里同样判红
      expect(actionCell).not.toMatch(/^system\.permission\.action\./);
      // 资源列：词表内 token 必须有标签；词表外必须原样显示原码，不得伪造中文名
      if (vocabSet.has(token.resource_type)) {
        expect(resourceCell, `词表内资源 ${token.resource_type} 应出可读标签`).not.toBe(
          token.resource_type
        );
      } else {
        expect(resourceCell, `词表外资源 ${token.resource_type} 应原样显示`).toBe(
          token.resource_type
        );
      }
    }
  });
});
