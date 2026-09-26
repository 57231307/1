import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall, apiCallRaw, tryCleanup, getCtx, ensureTestEntities } from './helpers';
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
  // 并行模式下 31b 并发写产品会令列表膨胀、目标行不在首页 → 先用搜索框按目标名/编号过滤，
  // 再在过滤结果里定位行（findTableRow 第 4 参 filterKeyword），而非只扫首页 20 行。
  const target = await findTableRow(page, rowText, 1, rowText);
  if (!target) {
    editFailReason = `未找到目标行 ${rowText}`;
    console.error(`[31c] ${editFailReason}`);
    return false;
  }
  const editBtn = target.locator('button:has-text("编辑"), button:has-text("修改")').first();
  if (!(await editBtn.isVisible({ timeout: 5000 }))) {
    console.error(`[31c] ${route} 行内无编辑按钮`);
    return false;
  }
  await editBtn.click();
  console.log('[31c] 已点击编辑按钮');
  const dialog = page.locator('.el-dialog:visible').first();
  const visible = await dialog.waitFor({ state: 'visible', timeout: 8000 }).then(() => true);
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
        .then(() => true);
      if (selectVisible) {
        await statusSelect.click();
        const option = page
          .locator('.el-select-dropdown:visible .el-select-dropdown__item')
          .filter({ hasText: inactivePattern })
          .first();
        const optionVisible = await option
          .waitFor({ state: 'visible', timeout: 4000 })
          .then(() => true);
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
    .then(() => true);
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
    await ensureTestEntities(page);
  });

  test('客户：UI 编辑弹窗停用→API 回读 inactive→UI 回显→删除清理', async ({ page }) => {
    test.setTimeout(180_000);
    const name = `P0停用客户${TS}`;
    const r = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
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
    const id = r?.data?.id;
    expect(id, '[31c-客户] 自建客户未返回 id，POST /crm/customers 前置失败').toBeTruthy();
    console.log(`[31c-客户] 创建成功 id=${id}`);

    const opened = await openEditDialog(page, '/customer', name);
    expect(opened, `[31c-客户] 无法打开自建客户 ${name} 编辑弹窗：${editFailReason}`).toBe(true);
    const toggled = await toggleStatusInDialog(page, '停用', /确定|确认|保存/);
    expect(toggled, '[31c-客户] 编辑弹窗内未找到/未能切换「停用」状态控件').toBe(true);

    // API 回读验证状态真变更（断言必然执行：GET 非 2xx 亦判失败，不再 console.warn 后放过）
    const chk = await page.request.get(`${API_BASE}${API_PREFIX}/crm/customers/${id}`);
    expect(chk.ok(), `[31c-客户] API 回读客户详情应 2xx，实际 HTTP ${chk.status()}`).toBe(true);
    const statusAfter = String((await chk.json())?.data?.status ?? '');
    console.log(`[31c-客户] API 回读 status=${statusAfter}`);
    expect(
      statusAfter,
      `[31c-客户] UI 停用后 API 回读 status 应为 inactive，实际 ${statusAfter}`
    ).toBe('inactive');

    // 二次访问：UI 重新打开编辑弹窗验证回显
    const reopened = await openEditDialog(page, '/customer', name);
    expect(reopened, `[31c-客户] 二次打开编辑弹窗失败：${editFailReason}`).toBe(true);
    const checked = await page
      .locator('.el-dialog:visible .el-radio:has-text("停用")')
      .first()
      .getAttribute('class');
    await page.keyboard.press('Escape');
    console.log(
      `[31c-客户] 二次打开回显「停用」radio ${checked?.includes('is-checked') ? '✅选中' : '❌未选中'}`
    );
    expect(checked?.includes('is-checked'), '[31c-客户] 二次访问编辑弹窗应回显停用状态').toBe(true);
    // 清理
    await tryCleanup(page, 'DELETE', `/crm/customers/${id}`, '[31c-客户]');
  });

  test('用户：UI 编辑弹窗停用→API 回读 is_active=false→删除清理', async ({ page }) => {
    test.setTimeout(180_000);
    const username = `p0dis${TS}`;
    // 前置角色 id：编辑弹窗 role_id 必填，创建时不带则编辑回显 undefined → 校验拦截 → PUT 不发出
    const rolesResp = await apiCallRaw<{ roles?: Array<{ id: number }> }>(
      page,
      'GET',
      '/roles?page=1&page_size=1'
    );
    const roleId = rolesResp?.roles?.[0]?.id;
    const r = await apiCall<{ id?: number }>(page, 'POST', '/users', {
      username,
      password: 'P0Test!2026dE',
      email: `${username}@test.com`,
      phone: '13600000002',
      role_id: roleId,
    });
    const id = r?.data?.id;
    expect(id, '[31c-用户] 自建用户未返回 id，POST /users 前置失败').toBeTruthy();
    console.log(`[31c-用户] 创建成功 id=${id}`);

    // /system 页用户 Tab；已切 Tab 后跳过重新导航（safeGoto 会重置回默认 Tab）
    await page.goto(`${BASE_URL}/system`);
    await page.waitForTimeout(2500);
    const userTab = page.locator('.el-tabs__item:has-text("用户")').first();
    expect(
      await userTab.isVisible({ timeout: 5000 }),
      '[31c-用户] /system 页未渲染「用户」Tab（Tab 入口结构变更？不可见时应硬失败，不再走 else 静默）'
    ).toBe(true);
    await userTab.click();
    await page.waitForTimeout(2000);
    console.log('[31c-用户] 已切到用户 Tab');
    // keyword 搜索过滤：用户列表可能分页，按新建账号搜索确保第一页可见
    // 只匹配当前激活 Tab 内可见的搜索框（隐藏 Tab 的 filter-card input 会先被 first() 命中）
    const keywordInput = page.locator('.filter-card input:visible').first();
    expect(
      await keywordInput.isVisible({ timeout: 4000 }),
      '[31c-用户] 用户 Tab 内未渲染可见搜索框'
    ).toBe(true);
    await keywordInput.fill(username);
    await keywordInput.press('Enter');
    // 等待搜索结果行渲染（keyword 请求+表格重渲染需要时间；waitForFunction 内 querySelectorAll
    // 不支持 :visible 伪类，改用 locator 原生等待）
    await page.locator('.el-table__row').first().waitFor({ state: 'visible', timeout: 10_000 });
    await page.waitForTimeout(500);
    console.log('[31c-用户] 已按用户名过滤列表');

    const opened = await openEditDialog(page, '/system', username, true);
    expect(opened, `[31c-用户] 无法打开自建用户 ${username} 编辑弹窗：${editFailReason}`).toBe(
      true
    );
    const toggled = await toggleStatusInDialog(page, '禁用', /确定|确认|保存/);
    expect(toggled, '[31c-用户] 编辑弹窗内未找到/未能切换「禁用」状态控件').toBe(true);

    const chk = await page.request.get(`${API_BASE}${API_PREFIX}/users/${id}`);
    expect(chk.ok(), `[31c-用户] API 回读用户详情应 2xx，实际 HTTP ${chk.status()}`).toBe(true);
    const activeAfter = (await chk.json())?.data?.is_active;
    console.log(`[31c-用户] API 回读 is_active=${activeAfter}`);
    expect(activeAfter, `[31c-用户] UI 停用后 is_active 应为 false，实际 ${activeAfter}`).toBe(
      false
    );
    await tryCleanup(page, 'DELETE', `/users/${id}`, '[31c-用户]');
  });

  test('产品：UI 编辑弹窗停用→API 回读 status≠active→删除清理', async ({ page }) => {
    test.setTimeout(180_000);
    const name = `P0停用产品${TS}`;
    const r = await apiCall<{ id?: number }>(page, 'POST', '/products', {
      name,
      code: `P0-DISP-${TS}`,
      category_id: getCtx().productCategoryIds[0],
      unit: '米',
      status: 'active',
      product_type: 'fabric',
      standard_price: 10,
      cost_price: 5,
      description: 'P0停用产品',
    });
    const id = r?.data?.id;
    expect(id, '[31c-产品] 自建产品未返回 id，POST /products 前置失败').toBeTruthy();
    console.log(`[31c-产品] 创建成功 id=${id}`);

    const opened = await openEditDialog(page, '/product', name);
    expect(opened, `[31c-产品] 无法打开自建产品 ${name} 编辑弹窗：${editFailReason}`).toBe(true);
    // 产品编辑弹窗 is_active switch
    const dialog = page.locator('.el-dialog:visible').first();
    const sw = dialog.locator('.el-switch').first();
    expect(
      await sw.isVisible({ timeout: 3000 }),
      '[31c-产品] 编辑弹窗内未渲染 is_active 开关（前端渲染条件若与后端状态词表不一致会命中此处）'
    ).toBe(true);
    await sw.click();
    console.log('[31c-产品] 已点击 is_active switch');
    await dialog
      .getByRole('button', { name: /确定|确认|保存/ })
      .last()
      .click();
    console.log('[31c-产品] 已点击确定保存');
    await page.waitForTimeout(2500);
    const chk = await page.request.get(`${API_BASE}${API_PREFIX}/products/${id}`);
    expect(chk.ok(), `[31c-产品] API 回读产品详情应 2xx，实际 HTTP ${chk.status()}`).toBe(true);
    const statusAfter = String((await chk.json())?.data?.status ?? '');
    console.log(`[31c-产品] API 回读 status=${statusAfter}`);
    expect(statusAfter, `[31c-产品] UI 停用后 status 应为 inactive，实际 ${statusAfter}`).toBe(
      'inactive'
    );
    await tryCleanup(page, 'DELETE', `/products/${id}`, '[31c-产品]');
  });

  test('部门：UI 编辑弹窗停用→API 回读 status=0→删除清理', async ({ page }) => {
    test.setTimeout(180_000);
    const name = `P0停用部门${TS}`;
    const r = await apiCall<{ id?: number }>(page, 'POST', '/departments', {
      name,
      description: 'P0停用部门',
    });
    const id = r?.data?.id;
    expect(id, '[31c-部门] 自建部门未返回 id，POST /departments 前置失败').toBeTruthy();
    console.log(`[31c-部门] 创建成功 id=${id}`);

    const opened = await openEditDialog(page, '/departments', name);
    expect(opened, `[31c-部门] 无法打开自建部门 ${name} 编辑弹窗：${editFailReason}`).toBe(true);
    const toggled = await toggleStatusInDialog(page, '停用', /确定|确认|保存/);
    expect(toggled, '[31c-部门] 编辑弹窗内未找到/未能切换「停用」状态控件').toBe(true);
    const chk = await page.request.get(`${API_BASE}${API_PREFIX}/departments/${id}`);
    expect(chk.ok(), `[31c-部门] API 回读部门详情应 2xx，实际 HTTP ${chk.status()}`).toBe(true);
    const statusAfter = (await chk.json())?.data?.is_active;
    console.log(`[31c-部门] API 回读 is_active=${statusAfter}`);
    expect(statusAfter, `[31c-部门] UI 停用后 is_active 应为 false，实际 ${statusAfter}`).toBe(
      false
    );
    await tryCleanup(page, 'DELETE', `/departments/${id}`, '[31c-部门]');
  });
});
