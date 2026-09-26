import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  tryCleanup,
  ensureTestEntities,
  getCtx,
  listNotifications,
  type NotificationItem,
} from './helpers';
import { safeGoto } from './ui-helpers';

/**
 * P0 OA 公告 + 通知公告直发（2026-09-11 用户指令）
 *
 * 覆盖：
 * 1. OA 公告 CRUD：API 创建草稿 → GET 回读 → PUT 编辑 → GET 验证 → DELETE 清理
 * 2. OA 公告发布联动通知：创建草稿（visibility_scope=CUSTOM）→ POST publish → 通知产生验证 → 归档清理
 * 3. OA 公告 UI 管理：导航 /system/oa-announcements → 标题可见 → 新建 → 弹窗与输入框可交互
 * 4. 通知公告直发：POST /notifications/announcement → 通知产生 → 已读 → 删除清理
 * 5. 通知 CRUD：列表查询 → 单条已读 → 批量已读 → 全部已读 → 删除
 *
 * iter23 修正：原实现把接口地址硬编码为 http://localhost:8082（绕过 API_BASE 与 CSRF 注入），
 * 并按 data.items 读通知列表——后端 list_notifications 的 key 是 data.list，
 * 于是读取恒为 0 条，所有「没找到通知就只 warn」的分支全部空转，
 * 本文件实际上一条业务断言都没有跑到（31e-5 的 >=3 断言正是因此失败）。
 * 现统一走 helpers 的 apiCall/apiCallRaw，软分支改为真实断言。
 */

const TS = Date.now().toString().slice(-8);

/** 发布/直发均为 commit 后发布事件、由监听器异步写库，需给落库留窗口 */
const NOTIF_SETTLE_MS = 3000;

function diffById(before: NotificationItem[], after: NotificationItem[]): NotificationItem[] {
  return after.filter(n => !before.some(b => b.id === n.id));
}

test.describe.serial('P0 OA 公告 + 通知公告直发', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('1. OA 公告 API CRUD：创建→回读→编辑→验证→删除', async ({ page }) => {
    test.setTimeout(120_000);
    const title = `P0公告CRUD${TS}`;
    const created = await apiCall<{ id?: number }>(page, 'POST', '/oa-announcements', {
      title,
      content: 'P0公告CRUD测试内容',
      announcement_type: 'NOTICE',
      publish_date: new Date().toISOString().slice(0, 10),
      effective_date: new Date().toISOString().slice(0, 10),
      visibility_scope: 'ALL',
    });
    const id = created.data?.id;
    expect(id, '[31e-1] OA 公告创建未返回 id').toBeTruthy();
    console.log(`[31e-1] 创建成功 id=${id}`);

    const got = await apiCallRaw<{ title: string; content: string; status: string }>(
      page,
      'GET',
      `/oa-announcements/${id}`
    );
    console.log(`[31e-1] 回读 title=${got.title} status=${got.status}`);
    expect(got.title, '[31e-1] 回读 title 应匹配').toBe(title);
    expect(got.status, '[31e-1] 新建应为草稿状态').toBe('DRAFT');

    const editedTitle = `P0公告编辑后${TS}`;
    await apiCall(page, 'PUT', `/oa-announcements/${id}`, {
      title: editedTitle,
      content: '编辑后内容',
    });

    const afterEdit = await apiCallRaw<{ title: string; content: string }>(
      page,
      'GET',
      `/oa-announcements/${id}`
    );
    console.log(`[31e-1] 编辑后回读 title=${afterEdit.title}`);
    expect(afterEdit.title, '[31e-1] 编辑后 title 应更新').toBe(editedTitle);
    expect(afterEdit.content, '[31e-1] 编辑后 content 应更新').toBe('编辑后内容');

    await tryCleanup(page, 'DELETE', `/oa-announcements/${id}`, '[31e-1]');
  });

  test('2. OA 公告发布联动通知：CUSTOM 范围→发布→通知产生验证', async ({ page }) => {
    test.setTimeout(180_000);
    await ensureTestEntities(page);
    const ctx = getCtx();
    const currentUserId = ctx.userIds[0];
    expect(
      currentUserId,
      '[31e-2] 当前用户 id 未就绪（ensureTestEntities 的 /auth/me 步骤未成功）'
    ).toBeTruthy();

    const title = `P0发布联动${TS}`;
    const created = await apiCall<{ id?: number }>(page, 'POST', '/oa-announcements', {
      title,
      content: 'P0发布联动通知测试内容',
      announcement_type: 'ANNOUNCEMENT',
      publish_date: new Date().toISOString().slice(0, 10),
      effective_date: new Date().toISOString().slice(0, 10),
      visibility_scope: 'CUSTOM',
      visible_scope_config: { user_ids: [currentUserId] },
    });
    const announcementId = created.data?.id;
    expect(announcementId, '[31e-2] 公告创建未返回 id').toBeTruthy();
    console.log(`[31e-2] 公告创建成功 id=${announcementId} 目标用户=${currentUserId}`);

    const before = await listNotifications(page);

    // oa_announcement_handler.rs:83 发布时按 visibility_scope 解析目标用户并联动站内通知
    const published = await apiCallRaw<{ notified_count?: number }>(
      page,
      'POST',
      `/oa-announcements/${announcementId}/publish`
    );
    console.log(`[31e-2] 发布成功 notified_count=${published?.notified_count}`);
    expect(
      published?.notified_count,
      '[31e-2] 发布应至少联动投递 1 条站内通知'
    ).toBeGreaterThanOrEqual(1);

    await page.waitForTimeout(NOTIF_SETTLE_MS);
    const after = await listNotifications(page);
    const newOnes = diffById(before, after);
    const announcementNotif = newOnes.find(n => n.title === title);
    console.log(`[31e-2] 新增通知 ${newOnes.length} 条，匹配公告通知: ${!!announcementNotif}`);
    expect(announcementNotif, `[31e-2] 通知列表中找不到标题为「${title}」的联动通知`).toBeTruthy();
    expect(announcementNotif!.content, '[31e-2] 通知内容应匹配公告内容').toBe(
      'P0发布联动通知测试内容'
    );

    await tryCleanup(page, 'DELETE', `/notifications/${announcementNotif!.id}`, '[31e-2] 通知');
    // PUBLISHED 状态不可直接删除，先归档
    await apiCall(page, 'POST', `/oa-announcements/${announcementId}/archive`);
    console.log(`[31e-2] 归档公告 id=${announcementId}`);
  });

  test('3. OA 公告 UI 管理：导航→列表→新建→弹窗可交互', async ({ page }) => {
    test.setTimeout(120_000);
    await safeGoto(page, '/system/oa-announcements');
    await page.waitForTimeout(2000);

    await expect(page.locator('.page-title').first(), '[31e-3] OA 公告页面标题应可见').toBeVisible({
      timeout: 8000,
    });

    const createBtn = page.locator('button:has-text("新建")').first();
    await expect(createBtn, '[31e-3] 新建按钮应可见').toBeVisible({ timeout: 5000 });
    await createBtn.click();

    const dialog = page.locator('.el-dialog:visible').first();
    await expect(dialog, '[31e-3] 点击新建后应打开表单弹窗').toBeVisible({ timeout: 5000 });

    const titleInput = dialog.locator('input').first();
    await expect(titleInput, '[31e-3] 弹窗内应有标题输入框').toBeVisible();
    const uiTitle = `P0-UI公告${TS}`;
    await titleInput.fill(uiTitle);
    expect(await titleInput.inputValue(), '[31e-3] 标题输入框应回填成功').toBe(uiTitle);

    // 不保存，避免残留数据
    await page.keyboard.press('Escape');
  });

  test('4. 通知公告直发：POST /notifications/announcement→通知产生→已读→删除', async ({ page }) => {
    test.setTimeout(120_000);
    await ensureTestEntities(page);
    const ctx = getCtx();
    const currentUserId = ctx.userIds[0];
    expect(currentUserId, '[31e-4] 当前用户 id 未就绪').toBeTruthy();

    const title = `P0直发通知${TS}`;
    const before = await listNotifications(page);

    // notification_handler.rs::create_announcement 仅管理员可发，返回 { delivered_count }
    const sent = await apiCallRaw<{ delivered_count?: number }>(
      page,
      'POST',
      '/notifications/announcement',
      { user_ids: [currentUserId], title, content: `P0直发通知内容 ${TS}` }
    );
    console.log(`[31e-4] 公告发送成功 delivered_count=${sent?.delivered_count}`);
    expect(sent?.delivered_count, '[31e-4] 应恰好投递 1 个目标用户').toBe(1);

    await page.waitForTimeout(NOTIF_SETTLE_MS);
    const after = await listNotifications(page);
    const newOnes = diffById(before, after);
    const directNotif = newOnes.find(n => n.title === title);
    console.log(`[31e-4] 新增通知 ${newOnes.length} 条，匹配直发通知: ${!!directNotif}`);
    expect(directNotif, `[31e-4] 通知列表中找不到标题为「${title}」的直发通知`).toBeTruthy();

    await apiCall(page, 'POST', `/notifications/${directNotif!.id}/read`);

    const afterRead = await listNotifications(page);
    expect(
      afterRead.some(n => n.id === directNotif!.id),
      '[31e-4] 已读后应不在 UNREAD 列表'
    ).toBe(false);

    await tryCleanup(page, 'DELETE', `/notifications/${directNotif!.id}`, '[31e-4]');
  });

  test('5. 通知 CRUD：列表→单条已读→批量已读→全部已读→删除', async ({ page }) => {
    test.setTimeout(120_000);
    await ensureTestEntities(page);
    const ctx = getCtx();
    const currentUserId = ctx.userIds[0];
    expect(currentUserId, '[31e-5] 当前用户 id 未就绪').toBeTruthy();

    // 三条通知 content 各异，规避 5 分钟去重窗口内同 dedup_key 折叠
    const titles = [`P0-CRUD-1-${TS}`, `P0-CRUD-2-${TS}`, `P0-CRUD-3-${TS}`];
    for (const [i, t] of titles.entries()) {
      const r = await apiCallRaw<{ delivered_count?: number }>(
        page,
        'POST',
        '/notifications/announcement',
        { user_ids: [currentUserId], title: t, content: `CRUD 测试 ${i + 1}-${TS}` }
      );
      console.log(`[31e-5] 发送「${t}」delivered_count=${r?.delivered_count}`);
      expect(r?.delivered_count, `[31e-5] 通知「${t}」投递失败`).toBe(1);
    }
    await page.waitForTimeout(NOTIF_SETTLE_MS);

    const list = await listNotifications(page);
    const myNotifs = list.filter(n => titles.includes(n.title));
    console.log(`[31e-5] 发送 3 条，找到 ${myNotifs.length} 条匹配通知`);
    expect(myNotifs.length, '[31e-5] 发送 3 条通知后应能检索到 3 条').toBe(3);

    // 单条已读
    await apiCall(page, 'POST', `/notifications/${myNotifs[0].id}/read`);
    // 批量已读（notification_handler.rs:31 BatchOperationRequest { ids }，返回处理条数）
    const batchCount = await apiCallRaw<number>(page, 'POST', '/notifications/batch-read', {
      ids: [myNotifs[1].id],
    });
    console.log(`[31e-5] 批量已读返回 count=${batchCount}`);
    expect(batchCount, '[31e-5] 批量已读应处理 1 条').toBe(1);
    // 全部已读：剩下的第 3 条由它处理
    const readAllCount = await apiCallRaw<number>(page, 'POST', '/notifications/read-all');
    console.log(`[31e-5] 全部已读返回 count=${readAllCount}`);
    expect(readAllCount, '[31e-5] 全部已读至少应处理剩余的 1 条').toBeGreaterThanOrEqual(1);

    const afterAll = await listNotifications(page);
    const stillUnread = afterAll.filter(n => titles.includes(n.title));
    console.log(`[31e-5] 全部已读后仍未读: ${stillUnread.length}`);
    expect(stillUnread.length, '[31e-5] 全部已读后 3 条测试通知应无未读').toBe(0);

    for (const n of myNotifs) {
      await tryCleanup(page, 'DELETE', `/notifications/${n.id}`, '[31e-5]');
    }
  });
});
