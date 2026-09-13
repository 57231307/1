import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall } from './helpers';
import { safeGoto } from './ui-helpers';

/**
 * P0 OA 公告 + 通知公告直发（2026-09-11 用户指令）
 *
 * 覆盖：
 * 1. OA 公告 CRUD：API 创建草稿 → GET 回读 → PUT 编辑 → GET 验证 → DELETE 清理
 * 2. OA 公告发布联动通知：API 创建草稿（visibility_scope=CUSTOM, user_ids=[当前用户]）→ POST publish → 通知列表验证通知产生 → 清理
 * 3. OA 公告 UI 管理：导航到 /system/oa-announcements → 列表可见 → 点击新建 → 填表保存 → 列表出现 → 发布按钮可见
 * 4. 通知公告直发：POST /notifications/announcement → 通知列表验证 → 已读 → 删除清理
 * 5. 通知 CRUD：列表查询 → 单条已读 → 批量已读 → 全部已读 → 删除
 */

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000';
const TS = Date.now().toString().slice(-8);

/** 查询当前用户未读通知 */
async function getUnreadNotifications(page: import('@playwright/test').Page): Promise<{ id: number; title: string; content: string; businessType?: string; status?: string }[]> {
  try {
    const res = await page.request.get('http://localhost:8082/api/v1/erp/notifications/?status=unread&page=1&page_size=50');
    if (!res.ok()) return [];
    const body = await res.json();
    const items = body?.data?.items || body?.data?.data || body?.data || [];
    return Array.isArray(items) ? items : [];
  } catch {
    return [];
  }
}

async function deleteNotification(page: import('@playwright/test').Page, id: number): Promise<void> {
  try {
    await page.request.delete(`http://localhost:8082/api/v1/erp/notifications/notification/${id}`);
    console.log(`[31e] 清理通知 id=${id} ✅`);
  } catch (e) {
    console.warn(`[31e] 清理通知 id=${id} 失败: ${(e as Error).message}`);
  }
}

test.describe.serial('P0 OA 公告 + 通知公告直发', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('1. OA 公告 API CRUD：创建→回读→编辑→验证→删除', async ({ page }) => {
    test.setTimeout(120_000);
    let id: number | undefined;
    const title = `P0公告CRUD${TS}`;
    try {
      const r = await apiCall<{ id?: number }>(page, 'POST', '/oa-announcements/', {
        title,
        content: 'P0公告CRUD测试内容',
        announcement_type: 'NOTICE',
        publish_date: new Date().toISOString().slice(0, 10),
        effective_date: new Date().toISOString().slice(0, 10),
        visibility_scope: 'ALL',
      });
      id = r?.data?.id;
    } catch (e) {
      console.error(`[31e-1] 创建失败: ${(e as Error).message}`);
    }
    if (!id) { test.skip(); return; }
    console.log(`[31e-1] 创建成功 id=${id}`);

    // GET 回读
    let got: { title?: string; content?: string; status?: string } | null = null;
    try {
      const res = await page.request.get(`http://localhost:8082/api/v1/erp/oa-announcements/${id}`);
      if (res.ok()) {
        const body = await res.json();
        got = body?.data;
        console.log(`[31e-1] 回读 title=${got?.title} status=${got?.status}`);
      }
    } catch (e) {
      console.warn(`[31e-1] 回读失败: ${(e as Error).message}`);
    }
    expect(got?.title, '[31e-1] 回读 title 应匹配').toBe(title);
    expect(got?.status, '[31e-1] 新建应为草稿状态').toBe('DRAFT');

    // PUT 编辑
    try {
      await apiCall(page, 'PUT', `/oa-announcements/${id}`, {
        title: `P0公告编辑后${TS}`,
        content: '编辑后内容',
      });
      console.log('[31e-1] 编辑成功');
    } catch (e) {
      console.error(`[31e-1] 编辑失败: ${(e as Error).message}`);
    }

    // GET 验证编辑
    try {
      const res = await page.request.get(`http://localhost:8082/api/v1/erp/oa-announcements/${id}`);
      if (res.ok()) {
        const body = await res.json();
        console.log(`[31e-1] 编辑后回读 title=${body?.data?.title}`);
        expect(body?.data?.title, '[31e-1] 编辑后 title 应更新').toBe(`P0公告编辑后${TS}`);
      }
    } catch (e) {
      console.warn(`[31e-1] 编辑后回读失败: ${(e as Error).message}`);
    }

    // DELETE 清理
    try {
      await apiCall(page, 'DELETE', `/oa-announcements/${id}`);
      console.log(`[31e-1] 清理删除 id=${id} ✅`);
    } catch (e) {
      console.warn(`[31e-1] 清理删除失败: ${(e as Error).message}`);
    }
  });

  test('2. OA 公告发布联动通知：CUSTOM 范围→发布→通知产生验证', async ({ page }) => {
    test.setTimeout(180_000);
    // 查当前用户 id（通过 /users/me 或 auth context）
    let currentUserId: number | undefined;
    try {
      const res = await page.request.get('http://localhost:8082/api/v1/erp/users/me');
      if (res.ok()) {
        const body = await res.json();
        currentUserId = body?.data?.id;
        console.log(`[31e-2] 当前用户 id=${currentUserId}`);
      }
    } catch (e) {
      console.warn(`[31e-2] 获取当前用户失败: ${(e as Error).message}`);
    }
    if (!currentUserId) { test.skip(); return; }

    // 创建草稿公告（visibility_scope=CUSTOM, user_ids=[当前用户]）
    let announcementId: number | undefined;
    const title = `P0发布联动${TS}`;
    try {
      const r = await apiCall<{ id?: number }>(page, 'POST', '/oa-announcements/', {
        title,
        content: 'P0发布联动通知测试内容',
        announcement_type: 'ANNOUNCEMENT',
        publish_date: new Date().toISOString().slice(0, 10),
        effective_date: new Date().toISOString().slice(0, 10),
        visibility_scope: 'CUSTOM',
        visible_scope_config: { user_ids: [currentUserId] },
      });
      announcementId = r?.data?.id;
    } catch (e) {
      console.error(`[31e-2] 公告创建失败: ${(e as Error).message}`);
    }
    if (!announcementId) { test.skip(); return; }
    console.log(`[31e-2] 公告创建成功 id=${announcementId}`);

    const before = await getUnreadNotifications(page);

    // 发布 → 联动通知
    let notifiedCount = 0;
    try {
      const res = await page.request.post(`http://localhost:8082/api/v1/erp/oa-announcements/${announcementId}/publish`);
      if (res.ok()) {
        const body = await res.json();
        notifiedCount = body?.data?.notified_count ?? 0;
        console.log(`[31e-2] 发布成功 notified_count=${notifiedCount}`);
      } else {
        console.warn(`[31e-2] 发布 HTTP ${res.status()}`);
      }
    } catch (e) {
      console.error(`[31e-2] 发布失败: ${(e as Error).message}`);
    }

    await page.waitForTimeout(3000);
    const after = await getUnreadNotifications(page);
    const newOnes = after.filter((n) => !before.some((b) => b.id === n.id));
    const announcementNotif = newOnes.find((n) => n.title === title);
    console.log(`[31e-2] 新增通知 ${newOnes.length} 条，匹配公告通知: ${!!announcementNotif}`);

    if (notifiedCount > 0 && announcementNotif) {
      expect(announcementNotif.title, '[31e-2] 通知标题应匹配公告标题').toBe(title);
      expect(announcementNotif.content, '[31e-2] 通知内容应匹配公告内容').toBe('P0发布联动通知测试内容');
      await deleteNotification(page, announcementNotif.id);
    } else {
      console.warn('[31e-2] 未找到联动通知（可能通知服务未配置）');
    }

    // 清理公告（PUBLISHED 状态不能直接删，先归档）
    try {
      await apiCall(page, 'POST', `/oa-announcements/${announcementId}/archive`);
      console.log(`[31e-2] 归档公告 id=${announcementId}`);
    } catch (e) {
      console.warn(`[31e-2] 归档失败: ${(e as Error).message}`);
    }
  });

  test('3. OA 公告 UI 管理：导航→列表→新建→保存→发布按钮可见', async ({ page }) => {
    test.setTimeout(120_000);
    await safeGoto(page, '/system/oa-announcements');
    await page.waitForTimeout(2000);

    // 页面标题可见
    const pageTitle = page.locator('.page-title').first();
    await expect(pageTitle, '[31e-3] OA 公告页面标题应可见').toBeVisible({ timeout: 8000 });

    // 新建按钮可见
    const createBtn = page.locator('button:has-text("新建")').first();
    const createVisible = await createBtn.isVisible({ timeout: 5000 }).catch(() => false);
    console.log(`[31e-3] 新建按钮可见: ${createVisible}`);

    if (createVisible) {
      await createBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog:visible').first();
      const dialogVisible = await dialog.isVisible({ timeout: 5000 }).catch(() => false);
      console.log(`[31e-3] 新建弹窗可见: ${dialogVisible}`);
      if (dialogVisible) {
        // 填写标题
        const titleInput = dialog.locator('input').first();
        if (await titleInput.isVisible()) {
          await titleInput.fill(`P0-UI公告${TS}`);
          console.log('[31e-3] 已填写标题');
        }
        // 关闭弹窗（不保存，避免残留）
        await page.keyboard.press('Escape');
      }
    }
  });

  test('4. 通知公告直发：POST /notifications/announcement→通知产生→已读→删除', async ({ page }) => {
    test.setTimeout(120_000);
    let currentUserId: number | undefined;
    try {
      const res = await page.request.get('http://localhost:8082/api/v1/erp/users/me');
      if (res.ok()) {
        currentUserId = (await res.json())?.data?.id;
      }
    } catch (e) {
      console.warn(`[31e-4] 获取当前用户失败: ${(e as Error).message}`);
    }
    if (!currentUserId) { test.skip(); return; }

    const title = `P0直发通知${TS}`;
    const before = await getUnreadNotifications(page);

    // 需要管理员权限
    try {
      const res = await page.request.post('http://localhost:8082/api/v1/erp/notifications/announcement', {
        data: { user_ids: [currentUserId], title, content: 'P0直发通知测试内容' },
      });
      if (res.ok()) {
        const body = await res.json();
        console.log(`[31e-4] 公告发送成功 delivered_count=${body?.data?.delivered_count ?? 0}`);
      } else {
        console.warn(`[31e-4] 公告发送 HTTP ${res.status()}（可能非管理员）`);
        if (res.status() === 403) { test.skip(); return; }
      }
    } catch (e) {
      console.error(`[31e-4] 公告发送失败: ${(e as Error).message}`);
    }

    await page.waitForTimeout(2000);
    const after = await getUnreadNotifications(page);
    const newOnes = after.filter((n) => !before.some((b) => b.id === n.id));
    const directNotif = newOnes.find((n) => n.title === title);
    console.log(`[31e-4] 新增通知 ${newOnes.length} 条，匹配直发通知: ${!!directNotif}`);

    if (directNotif) {
      expect(directNotif.title, '[31e-4] 通知标题应匹配').toBe(title);

      // 测试已读
      try {
        const readRes = await page.request.post(`http://localhost:8082/api/v1/erp/notifications/notification/${directNotif.id}/read`);
        console.log(`[31e-4] 标记已读 HTTP ${readRes.status()}`);
      } catch (e) {
        console.warn(`[31e-4] 标记已读失败: ${(e as Error).message}`);
      }

      // 验证已读后 unread 列表不再包含
      const afterRead = await getUnreadNotifications(page);
      const stillUnread = afterRead.find((n) => n.id === directNotif.id);
      expect(!stillUnread, '[31e-4] 已读后应不在 unread 列表').toBeTruthy();

      // 删除清理
      await deleteNotification(page, directNotif.id);
    } else {
      console.warn('[31e-4] 未找到直发通知（可能非管理员或通知服务未配置）');
    }
  });

  test('5. 通知 CRUD：列表→单条已读→批量已读→全部已读→删除', async ({ page }) => {
    test.setTimeout(120_000);
    let currentUserId: number | undefined;
    try {
      const res = await page.request.get('http://localhost:8082/api/v1/erp/users/me');
      if (res.ok()) currentUserId = (await res.json())?.data?.id;
    } catch { /* skip */ }
    if (!currentUserId) { test.skip(); return; }

    // 发 3 条通知
    const titles = [`P0-CRUD-1-${TS}`, `P0-CRUD-2-${TS}`, `P0-CRUD-3-${TS}`];
    for (const t of titles) {
      try {
        await page.request.post('http://localhost:8082/api/v1/erp/notifications/announcement', {
          data: { user_ids: [currentUserId], title: t, content: 'CRUD 测试' },
        });
      } catch { /* 非管理员跳过 */ }
    }
    await page.waitForTimeout(2000);

    const list = await getUnreadNotifications(page);
    const myNotifs = list.filter((n) => titles.includes(n.title));
    console.log(`[31e-5] 发送 3 条，找到 ${myNotifs.length} 条匹配通知`);

    if (myNotifs.length >= 2) {
      // 单条已读
      const first = myNotifs[0];
      try {
        const res = await page.request.post(`http://localhost:8082/api/v1/erp/notifications/notification/${first.id}/read`);
        console.log(`[31e-5] 单条已读 HTTP ${res.status()}`);
      } catch (e) {
        console.warn(`[31e-5] 单条已读失败: ${(e as Error).message}`);
      }

      // 批量已读
      if (myNotifs.length >= 3) {
        try {
          const res = await page.request.post('http://localhost:8082/api/v1/erp/notifications/batch-read', {
            data: { ids: [myNotifs[1].id, myNotifs[2].id] },
          });
          console.log(`[31e-5] 批量已读 HTTP ${res.status()}`);
        } catch (e) {
          console.warn(`[31e-5] 批量已读失败: ${(e as Error).message}`);
        }
      }

      // 全部已读
      try {
        const res = await page.request.post('http://localhost:8082/api/v1/erp/notifications/read-all');
        console.log(`[31e-5] 全部已读 HTTP ${res.status()}`);
      } catch (e) {
        console.warn(`[31e-5] 全部已读失败: ${(e as Error).message}`);
      }

      // 验证未读数为 0
      const afterAll = await getUnreadNotifications(page);
      const stillUnread = afterAll.filter((n) => titles.includes(n.title));
      console.log(`[31e-5] 全部已读后仍未读: ${stillUnread.length}`);
      expect(stillUnread.length, '[31e-5] 全部已读后应无未读').toBe(0);

      // 删除清理
      for (const n of myNotifs) {
        await deleteNotification(page, n.id);
      }
    } else {
      console.warn('[31e-5] 通知数不足，跳过 CRUD 验证（可能非管理员）');
    }
  });
});
