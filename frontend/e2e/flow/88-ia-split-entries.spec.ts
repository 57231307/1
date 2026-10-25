/**
 * 系统设置聚合页拆分后的入口与回补能力回归 E2E
 *
 * 功能：锁住三件在 Tab 拆分波次里最容易被拆丢的事实。
 *   一，系统管理域的唯一入口：旧聚合地址 /system 只作重定向，落点必须是用户管理页，
 *   侧栏不再保留聚合入口，而拆分出的六个功能页入口逐一在位。
 *   二，数据权限页的跨角色只读概览：概览视图必须按后端富化出的 role_name 与 created_at 出值，
 *   且概览是只读面，写面入口与操作列在该视图下不得出现。
 *   三，通知页的系统公告发送卡：标题正文收件人三个控件在位，收件人候选取自用户列表同源数据，
 *   缺项提交必须是前端拦截并给出对应文案，而不是把非法请求发给后端。
 * 调用方：E2E 全量套件（Playwright），由 ci-cd 的 e2e 分片执行（flow 目录整目录 hash 分片）。
 * 入参：BASE_URL 与登录账号来自 helpers 环境变量；只在数据权限页 upsert 一条既有契约的权限记录，
 *   公告卡只验前端拦截分支不发送请求，部门与公司的改造由各自波次与既有 spec 负责，本文件不触碰。
 * 传给谁：断言结果只进 Playwright 报告。
 * 存什么/存哪里：数据权限页造的那条记录为幂等 upsert，与列绑定回归用例共用同一条种子，不额外落库。
 */
import { test, expect, type Locator } from '../diagnose-fixture';
import { loginViaUI, apiCallRaw, BASE_URL } from './helpers';
import { formItemByExactLabel, pickSelectIn } from './ui-helpers';
import { columnCellTexts, rowCount } from './table-column';

/**
 * 读 el-table 实际渲染的表头文案清单（列缺失或多余都以此判定，不用位置选择器）。
 * 本函数依赖 Element Plus 标准表的表头 DOM 结构（header-wrapper 内的 thead th，且排除无正文单元格的
 * gutter 列）。若 EP 升级改变表头结构，本条会以列头取不到而误报，属预期的锁失效，
 * 应交人工复核判据而不是判定为产品缺陷，也不要把断言放宽回可见性判据。
 */
async function headerTextsOf(table: Locator): Promise<string[]> {
  const headers = table.locator('.el-table__header-wrapper thead th.el-table__cell:not(.gutter)');
  return (await headers.allInnerTexts()).map(text => text.trim());
}

test.describe('系统设置拆分后的入口唯一性与回补能力', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('旧聚合地址只重定向到用户管理页，侧栏无聚合入口且六个功能页入口齐全', async ({ page }) => {
    await page.goto(`${BASE_URL}/system`);
    await expect(page, '访问 /system 应落到用户管理页').toHaveURL(/\/users$/, {
      timeout: 30_000,
    });
    // 地址栏真实落点（history 路由下 pathname 即路由路径，防止 query 或 hash 伪装成命中）
    expect(new URL(page.url()).pathname, `实际落点 ${page.url()}`).toBe('/users');

    await expect(
      page.getByRole('heading', { level: 2, name: '用户管理' }),
      '落点页未渲染用户管理标题，重定向可能被指到空白页'
    ).toBeVisible({ timeout: 30_000 });

    const menu = page.locator('.menu');
    await expect(menu, '侧栏菜单未渲染').toBeVisible({ timeout: 30_000 });
    await expect(
      menu.locator('.el-menu-item', { hasText: '系统设置' }),
      '侧栏仍保留系统设置聚合入口'
    ).toHaveCount(0);

    const entries = ['用户管理', '角色管理', '公司信息', '权限管理', '字段权限', 'Webhook 配置'];
    for (const label of entries) {
      await expect(
        menu.locator('.el-menu-item', { hasText: label }),
        `侧栏缺少功能页入口 ${label}`
      ).toHaveCount(1);
    }
  });

  test('数据权限页全部角色概览按真实角色名与创建时间出值，且概览态收起写面', async ({ page }) => {
    const roles = await apiCallRaw<{ roles: Array<{ id: number; name: string }> }>(
      page,
      'GET',
      '/roles'
    );
    expect(roles.roles.length, '无角色可用，无法验证概览取数').toBeGreaterThan(0);
    const role = roles.roles[0];
    // 概览是跨角色只读视图，先保证该角色名下至少一条记录（与列绑定回归共用同一幂等 upsert）
    await apiCallRaw(page, 'POST', '/data-permissions', {
      role_id: role.id,
      resource_type: 'customer',
      scope_type: 'DEPT',
    });

    await page.goto(`${BASE_URL}/data-permission`);
    const overviewEntry = page.getByRole('menuitem', { name: '全部角色', exact: true });
    await overviewEntry.waitFor({ state: 'visible', timeout: 30_000 });
    await overviewEntry.click();

    await expect(
      page.getByRole('heading', { level: 3, name: '全部角色数据权限概览' }),
      '点概览后面板标题未切到概览文案'
    ).toBeVisible({ timeout: 30_000 });

    const table = page.locator('.el-table[aria-label="数据权限列表"]').first();
    await table.waitFor({ state: 'visible', timeout: 30_000 });
    expect(await rowCount(table), '概览视图列表为 0 行，跨角色取数未生效').toBeGreaterThan(0);

    const headers = await headerTextsOf(table);
    for (const col of ['角色', '资源类型', '权限范围', '状态', '创建时间']) {
      expect(headers, `概览表头缺少列 ${col}，实际表头=${JSON.stringify(headers)}`).toContain(col);
    }
    // 概览为只读面：操作列不得随概览出现
    expect(headers, `概览视图不应有操作列，实际表头=${JSON.stringify(headers)}`).not.toContain(
      '操作'
    );

    const roleCol = headers.indexOf('角色');
    const createdCol = headers.indexOf('创建时间');
    const roleCells = (await columnCellTexts(table, roleCol)).filter(text => text.length > 0);
    expect(roleCells.length, '概览角色列全部为空，后端 role_name 富化未生效').toBeGreaterThan(0);
    expect(
      roleCells,
      `概览角色列未包含造数角色 ${role.name}，实际值=${JSON.stringify(roleCells)}`
    ).toContain(role.name);

    const createdCells = (await columnCellTexts(table, createdCol)).filter(text => text.length > 0);
    expect(createdCells.length, '概览创建时间列全部为空，created_at 未渲染').toBeGreaterThan(0);

    await expect(
      page.getByRole('button', { name: '新增权限', exact: true }),
      '概览视图不应渲染新增权限入口'
    ).toHaveCount(0);
  });

  test('通知页系统公告卡控件齐全，收件人候选出真实用户且缺项提交被前端拦截', async ({ page }) => {
    const announcementRequests: string[] = [];
    page.on('request', request => {
      if (request.method() === 'POST' && request.url().includes('/notifications/announcement')) {
        announcementRequests.push(request.url());
      }
    });

    await page.goto(`${BASE_URL}/notification`);
    const card = page.locator('.announce-card');
    await expect(card, '系统公告发送卡未渲染（入口或权限门被拆丢）').toBeVisible({
      timeout: 30_000,
    });
    await expect(card.locator('.el-card__header'), '公告卡标题未渲染系统公告文案').toContainText(
      '系统公告'
    );

    const titleItem = formItemByExactLabel(card, '标题');
    const contentItem = formItemByExactLabel(card, '内容');
    const recipientsItem = formItemByExactLabel(card, '收件人');
    await expect(titleItem, '缺少标题输入控件').toBeVisible();
    await expect(contentItem, '缺少正文输入控件').toBeVisible();
    await expect(recipientsItem, '缺少收件人选择控件').toBeVisible();

    // 收件人候选必须来自用户列表同源数据，先取后端真值再验下拉出该值
    const users = await apiCallRaw<{ users: Array<{ id: number; username: string }> }>(
      page,
      'GET',
      '/users?page=1&page_size=5'
    );
    expect(users.users.length, '用户列表为空，无法验证收件人候选').toBeGreaterThan(0);
    const candidate = users.users[0].username;
    await pickSelectIn(card, page, '收件人', { openOnly: true });
    const dropdown = page.locator('.el-select-dropdown:visible').last();
    await expect(
      dropdown.locator('.el-select-dropdown__item').filter({ hasText: candidate }).first(),
      `收件人下拉未出真实候选 ${candidate}`
    ).toBeVisible({ timeout: 15_000 });
    await page.keyboard.press('Escape');

    const sendBtn = card.getByRole('button', { name: '发送公告', exact: true });
    await expect(sendBtn, '公告卡缺少发送按钮').toBeVisible();

    // 缺标题：应给出标题必填文案，且不得发出公告请求
    await sendBtn.click();
    await expect(
      page.locator('.el-message').filter({ hasText: '请输入公告标题' }).first(),
      '空标题提交未给出标题必填提示'
    ).toBeVisible({ timeout: 10_000 });
    expect(announcementRequests, '缺标题时不应发公告请求').toHaveLength(0);

    // 有标题缺内容：拦截文案必须换成内容必填，仍不发请求
    await titleItem.locator('input').first().fill('拆分回归公告标题');
    await sendBtn.click();
    await expect(
      page.locator('.el-message').filter({ hasText: '请输入公告内容' }).first(),
      '缺内容提交未给出内容必填提示'
    ).toBeVisible({ timeout: 10_000 });
    expect(announcementRequests, '缺内容时不应发公告请求').toHaveLength(0);

    // 标题内容齐备但收件人为空：拦截文案应换成收件人必填，仍不发请求
    await contentItem.locator('textarea').first().fill('拆分回归公告内容');
    await sendBtn.click();
    await expect(
      page.locator('.el-message').filter({ hasText: '请选择至少一位收件人' }).first(),
      '收件人为空提交未给出收件人必填提示'
    ).toBeVisible({ timeout: 10_000 });
    expect(announcementRequests, '收件人为空时不应发公告请求').toHaveLength(0);
  });
});
