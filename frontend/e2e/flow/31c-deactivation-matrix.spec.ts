import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall, apiCallRaw, tryCleanup } from './helpers';
import { safeGoto, findTableRow } from './ui-helpers';

/**
 * P0 停用系统覆盖（2026-09-11 用户指令："删除/停用测试需要系统覆盖"）
 *
 * 现状核实：绝大多数列表页无行内停用开关，启停用入口在【编辑弹窗】内
 * （客户 status radio、用户 status switch、产品 is_active switch、部门 status switch）。
 *
 * 每资源验证（真实 UI 点击 + 真实后端回读）：
 * 1. API 创建新记录（不污染 seed 数据）
 * 2. UI 导航到列表 → 找到该行 → 点击"编辑" → 等待弹窗
 * 3. UI 切换状态控件（radio/switch）→ 点击确定保存
 * 4. API GET 详情回读 → 断言 status 已真变更
 * 5. UI 再次打开编辑弹窗 → 断言控件回显正确（二次访问一致性）
 * 6. API 删除清理（闭环，不留测试残留）
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';
const TS = Date.now().toString().slice(-8);
const BASE_URL = process.env.BASE_URL || 'http://localhost:3000';

/** openEditDialog 失败原因（模块级，供测试诊断链引用） */
let editFailReason = '';

/** 在列表中按唯一文本找行 → 点编辑 → 等弹窗可见；skipNavigation=true 时跳过 goto（已在目标页且已切 Tab） */
async function openEditDialog(
  page: import('@playwright/test').Page,
  route: string,
  rowText: string,
  skipNavigation = false
): Promise<boolean> {
  if (!skipNavigation) {
    await safeGoto(page, route);
    await page.waitForTimeout(1500);
  }
  const target = await findTableRow(page, rowText);
  if (!target) {
    editFailReason = `未找到目标行 ${rowText}`;
    console.error(`[31c] ${editFailReason}`);
    return false;
  }
  const editBtn = target.locator('button:has-text("编辑"), button:has-text("修改")').first();
  if (
    !(await editBtn.isVisible({ timeout: 5000 }).catch(e => {
      editFailReason = `编辑按钮不可见: ${(e as Error).message}`;
      console.warn(`[31c] ${editFailReason}`);
      return false;
    }))
  ) {
    console.error(`[31c] ${route} 行内无编辑按钮`);
    return false;
  }
  await editBtn.click();
  console.log('[31c] 已点击编辑按钮');
  const dialog = page.locator('.el-dialog:visible').first();
  const visible = await dialog
    .waitFor({ state: 'visible', timeout: 8000 })
    .then(() => true)
    .catch(e => {
      editFailReason = `编辑弹窗未出现: ${(e as Error).message}`;
      console.warn(`[31c] ${editFailReason}`);
      return false;
    });
  if (visible) console.log('[31c] 编辑弹窗已打开');
  return visible;
}

/** 在可见弹窗内切换状态控件（radio 文案或 switch），点击确定 */
async function toggleStatusInDialog(
  page: import('@playwright/test').Page,
  inactiveText: string,
  confirmText: RegExp
): Promise<boolean> {
  const dialog = page.locator('.el-dialog:visible').first();
  // 优先级：radio → switch → 状态下拉 el-select（如 /departments 页，选项文案"启用/禁用"）
  // 注意：isVisible() 是立即检查（timeout 参数被忽略），dialog 打开动画期间控件未到
  // visible 态会误判"未找到"，必须用 waitFor 真等待
  const inactivePattern = /停用|禁用|关闭|inactive/i;
  const radio = dialog
    .locator(`.el-radio:has-text("${inactiveText}"), .el-radio-button:has-text("${inactiveText}")`)
    .first();
  const sw = dialog.locator('.el-switch').first();
  const radioVisible = await radio
    .waitFor({ state: 'visible', timeout: 4000 })
    .then(() => true)
    .catch(() => false);
  let toggledOk = false;
  if (radioVisible) {
    await radio.click();
    console.log(`[31c] 已点击 radio「${inactiveText}」`);
    toggledOk = true;
  } else {
    const swVisible = await sw
      .waitFor({ state: 'visible', timeout: 4000 })
      .then(() => true)
      .catch(() => false);
    if (swVisible) {
      const before = await sw.getAttribute('class');
      if (before?.includes('is-checked')) {
        await sw.click(); // 开→关
        console.log('[31c] switch 已从开启切为关闭');
      } else {
        console.warn('[31c] switch 初始为关闭（期望开启态），仍点击尝试');
        await sw.click();
      }
      toggledOk = true;
    } else {
      // 第三优先：状态下拉（el-select + teleport 到 body 的下拉面板）
      // 注意：部门页弹窗有多个 select（el-tree-select 上级部门在前），.first() 会误点
      // 上级部门树选择器（其选项含历史"停用"部门名），必须按 label「状态」精确定位
      let statusSelect = dialog
        .locator('.el-form-item')
        .filter({ hasText: /状态/ })
        .locator('.el-select')
        .first();
      if ((await statusSelect.count()) === 0) {
        statusSelect = dialog.locator('.el-select').last();
      }
      const selectVisible = await statusSelect
        .waitFor({ state: 'visible', timeout: 4000 })
        .then(() => true)
        .catch(() => false);
      if (selectVisible) {
        await statusSelect.click();
        const option = page
          .locator('.el-select-dropdown:visible .el-select-dropdown__item')
          .filter({ hasText: inactivePattern })
          .first();
        const optionVisible = await option
          .waitFor({ state: 'visible', timeout: 4000 })
          .then(() => true)
          .catch(() => false);
        if (optionVisible) {
          await option.click();
          console.log('[31c] 已从状态下拉选择停用/禁用项');
          toggledOk = true;
        } else {
          console.error('[31c] 下拉已打开但未找到停用/禁用选项');
        }
      } else {
        console.error('[31c] 弹窗内未找到状态控件（radio/switch/select）');
      }
    }
  }
  if (!toggledOk) {
    return false;
  }
  const confirmBtn = dialog.getByRole('button', { name: confirmText }).last();
  const confirmVisible = await confirmBtn
    .waitFor({ state: 'visible', timeout: 4000 })
    .then(() => true)
    .catch(() => false);
  if (!confirmVisible) {
    console.error('[31c] 弹窗确定按钮不可见');
    return false;
  }
  await confirmBtn.click();
  console.log('[31c] 已点击确定保存');
  await page.waitForTimeout(2500);
  return true;
}

test.describe.serial('P0 停用矩阵：编辑弹窗 UI 切状态→API 回读验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('客户：UI 编辑弹窗停用→API 回读 inactive→UI 回显→删除清理', async ({ page }) => {
    test.setTimeout(180_000);
    const name = `P0停用客户${TS}`;
    let id: number | undefined;
    try {
      const r = await apiCall<{ id?: number }>(page, 'POST', '/customers', {
        customer_name: name,
        customer_code: `P0-DIS-${TS}`,
        contact_person: 'P0联系人',
        contact_phone: '13900000002',
        contact_email: 'dis@test.com',
        address: 'P0地址',
        city: '杭州',
        customer_type: 'retail',
        notes: 'P0停用客户',
      });
      id = r?.data?.id;
    } catch (e) {
      console.error(`[31c-客户] 创建失败: ${(e as Error).message}`);
    }
    if (!id) {
      test.skip();
      return;
    }
    console.log(`[31c-客户] 创建成功 id=${id}`);

    let toggled = false;
    try {
      if (await openEditDialog(page, '/customer', name)) {
        toggled = await toggleStatusInDialog(page, '停用', /确定|确认|保存/);
      }
    } catch (e) {
      console.error(`[31c-客户] UI 操作异常: ${(e as Error).message}`);
    }
    // API 回读验证状态真变更
    let statusAfter = '';
    try {
      const chk = await page.request.get(`${API_BASE}${API_PREFIX}/customers/${id}`);
      if (chk.ok()) {
        const body = await chk.json();
        statusAfter = String(body?.data?.status ?? '');
        console.log(`[31c-客户] API 回读 status=${statusAfter}`);
      } else {
        console.warn(`[31c-客户] 回读 HTTP ${chk.status()}`);
      }
    } catch (e) {
      console.warn(`[31c-客户] 回读异常: ${(e as Error).message}`);
    }
    if (toggled) {
      expect(
        statusAfter,
        `[31c-客户] UI 停用后 API 回读 status 应为 inactive，实际 ${statusAfter}`
      ).toBe('inactive');
      // 二次访问：UI 重新打开编辑弹窗验证回显
      let reopened = false;
      try {
        if (await openEditDialog(page, '/customer', name)) {
          const checked = await page
            .locator('.el-dialog:visible .el-radio:has-text("停用")')
            .first()
            .getAttribute('class');
          reopened = checked?.includes('is-checked') ?? false;
          console.log(`[31c-客户] 二次打开回显「停用」radio ${reopened ? '✅选中' : '❌未选中'}`);
          await page.keyboard.press('Escape');
        }
      } catch (e) {
        console.warn(`[31c-客户] 二次打开异常: ${(e as Error).message}`);
      }
      expect(reopened, '[31c-客户] 二次访问编辑弹窗应回显停用状态').toBe(true);
    } else {
      console.warn('[31c-客户] UI 停用未完成（控件结构差异），已记录诊断日志');
      expect(toggled, '[31c-客户] UI 编辑弹窗停用操作应可完成（失败见诊断日志）').toBe(true);
    }
    // 清理
    await tryCleanup(page, 'DELETE', `/customers/${id}`, '[31c-客户]');
  });

  test('用户：UI 编辑弹窗停用→API 回读 is_active=false→删除清理', async ({ page }) => {
    test.setTimeout(180_000);
    const username = `p0dis${TS}`;
    let id: number | undefined;
    try {
      // 前置角色 id：编辑弹窗 role_id 必填，创建时不带则编辑回显 undefined → 校验拦截 → PUT 不发出
      const rolesResp = await apiCallRaw<{ roles?: Array<{ id: number }> }>(
        page,
        'GET',
        '/roles?page=1&page_size=1'
      ).catch(() => null);
      const roleId = rolesResp?.roles?.[0]?.id;
      const r = await apiCall<{ id?: number }>(page, 'POST', '/users', {
        username,
        password: 'P0Test!2026dE',
        email: `${username}@test.com`,
        phone: '13600000002',
        role_id: roleId,
      });
      id = r?.data?.id;
    } catch (e) {
      console.error(`[31c-用户] 创建失败: ${(e as Error).message}`);
    }
    if (!id) {
      test.skip();
      return;
    }
    console.log(`[31c-用户] 创建成功 id=${id}`);

    let toggled = false;
    let diag = '开始';
    try {
      // /system 页用户 Tab；已切 Tab 后跳过重新导航（safeGoto 会重置回默认 Tab）
      await page.goto(`${BASE_URL}/system`);
      await page.waitForTimeout(2500);
      const userTab = page.locator('.el-tabs__item:has-text("用户")').first();
      const tabVisible = await userTab.isVisible({ timeout: 5000 }).catch(() => false);
      if (tabVisible) {
        await userTab.click();
        await page.waitForTimeout(2000);
        console.log('[31c-用户] 已切到用户 Tab');
        // keyword 搜索过滤（用户列表可能分页，直接搜索新建账号确保第一页可见）
        // 只匹配当前激活 Tab 内可见的搜索框（隐藏 Tab 的 filter-card input 会先被 first() 命中）
        const keywordInput = page.locator('.filter-card input:visible').first();
        const kwVisible = await keywordInput.isVisible({ timeout: 4000 }).catch(() => false);
        if (kwVisible) {
          await keywordInput.fill(username);
          await keywordInput.press('Enter');
          // 等待搜索结果行渲染（keyword 请求+表格重渲染需要时间，固定 2s 可能落在空窗期）
          await page
            .waitForFunction(() => document.querySelectorAll('.el-table__row:visible').length > 0, {
              timeout: 10000,
            })
            .catch(() => {
              console.warn('[31c-用户] 搜索后 10s 表格无可见行');
            });
          await page.waitForTimeout(500);
          console.log('[31c-用户] 已按用户名过滤列表');
          diag += '；搜索OK';
        } else {
          diag += '；搜索框不可见';
        }
        const opened = await openEditDialog(page, '/system', username, true);
        if (!opened) {
          // API 侧诊断：keyword 搜索在 API 层是否能查到（区分"数据缺失"与"UI 渲染问题"）
          const apiSearch = await page.request
            .get(`${API_BASE}${API_PREFIX}/users?page=1&page_size=20&keyword=${username}`)
            .then(r => r.json().catch(() => null))
            .catch(() => null);
          const apiUsers = (apiSearch as { data?: { users?: Array<{ username?: string }> } } | null)
            ?.data?.users;
          const apiHit = apiUsers?.some(u => u.username === username);
          diag += `；openEditDialog=${opened}(${editFailReason})；API回读命中=${apiHit}（${apiUsers?.length ?? 'err'}条）`;
        } else {
          diag += `；openEditDialog=${opened}`;
        }
        if (opened) {
          toggled = await toggleStatusInDialog(page, '禁用', /确定|确认|保存/);
          diag += `；toggled=${toggled}`;
        }
      } else {
        diag += '；用户Tab不可见';
        console.error('[31c-用户] 用户 Tab 不可见');
      }
    } catch (e) {
      diag += `；异常:${(e as Error).message}`;
      console.error(`[31c-用户] UI 操作异常: ${(e as Error).message}`);
    }
    let activeAfter: unknown = null;
    try {
      const chk = await page.request.get(`${API_BASE}${API_PREFIX}/users/${id}`);
      if (chk.ok()) {
        const body = await chk.json();
        activeAfter = body?.data?.is_active;
        console.log(`[31c-用户] API 回读 is_active=${activeAfter}`);
      } else {
        console.warn(`[31c-用户] 回读 HTTP ${chk.status()}`);
      }
    } catch (e) {
      console.warn(`[31c-用户] 回读异常: ${(e as Error).message}`);
    }
    if (toggled) {
      expect(activeAfter, `[31c-用户] UI 停用后 is_active 应为 false，实际 ${activeAfter}`).toBe(
        false
      );
    } else {
      console.warn('[31c-用户] UI 停用未完成（控件结构差异），已记录诊断日志');
      expect(toggled, `[31c-用户] UI 编辑弹窗停用操作应可完成，诊断链: ${diag}`).toBe(true);
    }
    await tryCleanup(page, 'DELETE', `/users/${id}`, '[31c-用户]');
  });

  test('产品：UI 编辑弹窗停用→API 回读 status≠active→删除清理', async ({ page }) => {
    test.setTimeout(180_000);
    const name = `P0停用产品${TS}`;
    let id: number | undefined;
    try {
      const r = await apiCall<{ id?: number }>(page, 'POST', '/products', {
        name,
        code: `P0-DISP-${TS}`,
        category_id: 1,
        unit: '米',
        status: 'active',
        product_type: 'fabric',
        standard_price: 10,
        cost_price: 5,
        description: 'P0停用产品',
      });
      id = r?.data?.id;
    } catch (e) {
      console.error(`[31c-产品] 创建失败: ${(e as Error).message}`);
    }
    if (!id) {
      test.skip();
      return;
    }
    console.log(`[31c-产品] 创建成功 id=${id}`);

    let toggled = false;
    try {
      if (await openEditDialog(page, '/product', name)) {
        // 产品编辑弹窗 is_active switch
        const dialog = page.locator('.el-dialog:visible').first();
        const sw = dialog.locator('.el-switch').first();
        if (await sw.isVisible({ timeout: 3000 }).catch(() => false)) {
          await sw.click();
          console.log('[31c-产品] 已点击 is_active switch');
          const confirmBtn = dialog.getByRole('button', { name: /确定|确认|保存/ }).last();
          await confirmBtn.click();
          console.log('[31c-产品] 已点击确定保存');
          await page.waitForTimeout(2500);
          toggled = true;
        } else {
          console.error('[31c-产品] 弹窗内未找到 is_active switch');
        }
      }
    } catch (e) {
      console.error(`[31c-产品] UI 操作异常: ${(e as Error).message}`);
    }
    let statusAfter = '';
    try {
      const chk = await page.request.get(`${API_BASE}${API_PREFIX}/products/${id}`);
      if (chk.ok()) {
        const body = await chk.json();
        statusAfter = String(body?.data?.status ?? '');
        console.log(`[31c-产品] API 回读 status=${statusAfter}`);
      }
    } catch (e) {
      console.warn(`[31c-产品] 回读异常: ${(e as Error).message}`);
    }
    if (toggled) {
      expect(statusAfter, `[31c-产品] UI 停用后 status 应为 inactive，实际 ${statusAfter}`).toBe(
        'inactive'
      );
    } else {
      console.warn('[31c-产品] UI 停用未完成（控件结构差异），已记录诊断日志');
      expect(toggled, '[31c-产品] UI 编辑弹窗停用操作应可完成（失败见诊断日志）').toBe(true);
    }
    await tryCleanup(page, 'DELETE', `/products/${id}`, '[31c-产品]');
  });

  test('部门：UI 编辑弹窗停用→API 回读 status=0→删除清理', async ({ page }) => {
    test.setTimeout(180_000);
    const name = `P0停用部门${TS}`;
    let id: number | undefined;
    try {
      const r = await apiCall<{ id?: number }>(page, 'POST', '/departments', {
        name,
        description: 'P0停用部门',
      });
      id = r?.data?.id;
    } catch (e) {
      console.error(`[31c-部门] 创建失败: ${(e as Error).message}`);
    }
    if (!id) {
      test.skip();
      return;
    }
    console.log(`[31c-部门] 创建成功 id=${id}`);

    let toggled = false;
    try {
      if (await openEditDialog(page, '/departments', name)) {
        toggled = await toggleStatusInDialog(page, '停用', /确定|确认|保存/);
      }
    } catch (e) {
      console.error(`[31c-部门] UI 操作异常: ${(e as Error).message}`);
    }
    let statusAfter: unknown = null;
    try {
      const chk = await page.request.get(`${API_BASE}${API_PREFIX}/departments/${id}`);
      if (chk.ok()) {
        const body = await chk.json();
        statusAfter = body?.data?.is_active;
        console.log(`[31c-部门] API 回读 is_active=${statusAfter}`);
      }
    } catch (e) {
      console.warn(`[31c-部门] 回读异常: ${(e as Error).message}`);
    }
    if (toggled) {
      expect(statusAfter, `[31c-部门] UI 停用后 is_active 应为 false，实际 ${statusAfter}`).toBe(
        false
      );
    } else {
      console.warn('[31c-部门] UI 停用未完成（控件结构差异），已记录诊断日志');
      expect(toggled, '[31c-部门] UI 编辑弹窗停用操作应可完成（失败见诊断日志）').toBe(true);
    }
    await tryCleanup(page, 'DELETE', `/departments/${id}`, '[31c-部门]');
  });
});
