import { chromium, request, expect } from '@playwright/test';
import { writeFileSync, mkdirSync } from 'fs';

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';
const FRONTEND_BASE = process.env.FRONTEND_BASE || 'http://localhost:3000';
// 分片专属账号：每个 CI runner（matrix.shard）独享，根除跨分片并发登录的 CSRF 互踢。
// 分片账号通过真实 UI（用户管理页面）创建，属于测试前置数据准备（ensureTestEntities 同级，
// 不属于测试验证手段），UI 测试本身仍全部走真实用户操作。
const BASE_USERNAME = process.env.TEST_USERNAME || 'e2e_admin';
const BASE_PASSWORD = process.env.TEST_PASSWORD || 'E2e@TestPassword2026!';
const SHARD_INDEX = process.env.E2E_SHARD_INDEX ?? '';
const SHARD_USERNAME = SHARD_INDEX !== '' ? `e2e_admin_s${SHARD_INDEX}` : BASE_USERNAME;
const SHARD_PASSWORD = BASE_PASSWORD;
const STORAGE_STATE_PATH = 'e2e/.auth/storage-state.json';

export default async function globalSetup() {
  // ---- 1. 分片账号不存在时，通过真实 UI 创建（e2e_admin 登录 → 用户管理页 → 新建用户）----
  if (SHARD_USERNAME !== BASE_USERNAME) {
    await ensureShardUserViaUI();
  }

  // ---- 2. 分片账号登录（API），保存 storageState 供全部 spec 复用 ----
  const ctx = await request.newContext({
    baseURL: API_BASE,
    extraHTTPHeaders: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
    },
  });

  const resp = await ctx.post(`${API_PREFIX}/auth/login`, {
    data: { username: SHARD_USERNAME, password: SHARD_PASSWORD },
  });

  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(`globalSetup 登录失败 (user=${SHARD_USERNAME}): HTTP ${resp.status()} ${body}`);
  }

  const cookies = await ctx.storageState();
  const accessCookie = cookies.cookies.find(c => c.name === 'access_token');
  if (!accessCookie) {
    throw new Error('globalSetup 登录后未获得 access_token cookie');
  }

  mkdirSync('e2e/.auth', { recursive: true });
  writeFileSync(STORAGE_STATE_PATH, JSON.stringify(cookies, null, 2));
  await ctx.dispose();
}

/**
 * 通过真实 UI 创建分片专属账号（不使用 API 直接创建）：
 * e2e_admin 登录 → /system 用户管理 → 新建用户 → 填用户名/密码/姓名/角色(admin) → 提交
 * 账号已存在（唯一约束冲突）时视为成功跳过。
 */
async function ensureShardUserViaUI(): Promise<void> {
  const browser = await chromium.launch();
  const context = await browser.newContext({
    baseURL: FRONTEND_BASE,
    locale: 'zh-CN',
  });
  const page = await context.newPage();

  try {
    // UI 登录 e2e_admin（登录页含"用户协议"勾选必填项，漏勾会阻断提交）
    await page.goto(`${FRONTEND_BASE}/login`, { waitUntil: 'domcontentloaded' });
    const usernameInput = page
      .locator('input[placeholder*="用户名"], input[aria-label*="用户名"]')
      .first();
    await usernameInput.waitFor({ state: 'visible', timeout: 30_000 });
    await usernameInput.fill(BASE_USERNAME);
    // 密码框：aria-label 定位（show-password 包裹多层 input）
    const loginPwdInput = page.locator('input[type="password"], input[aria-label*="密码"]').first();
    await loginPwdInput.waitFor({ state: 'visible', timeout: 30_000 });
    await loginPwdInput.click().catch(() => {});
    await loginPwdInput.fill(BASE_PASSWORD);
    const pwdValue = await loginPwdInput.inputValue().catch(() => '');
    console.log(`[globalSetup] 密码已填: ${pwdValue.length > 0}`);
    // 勾选用户协议：el-checkbox 原生 input 隐藏，必须点 .el-checkbox__inner（真实点击区域）
    const termsInner = page.locator('.el-checkbox__inner').first();
    await termsInner.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});
    const isCheckedBefore = await page
      .locator('.el-checkbox input[type="checkbox"]')
      .first()
      .isChecked()
      .catch(() => false);
    if (!isCheckedBefore) {
      await termsInner.click({ force: true }).catch(() => {});
      await page.waitForTimeout(300);
    }
    const isCheckedAfter = await page
      .locator('.el-checkbox input[type="checkbox"]')
      .first()
      .isChecked()
      .catch(() => 'unknown');
    console.log(`[globalSetup] 协议勾选状态: ${isCheckedBefore} → ${isCheckedAfter}`);
    if (isCheckedAfter !== true) {
      // 兜底：JS 直接置值并派发 change（Playwright 点击无效时的可靠途径）
      await page.evaluate(() => {
        const box = document.querySelector(
          '.el-checkbox input[type="checkbox"]'
        ) as HTMLInputElement | null;
        if (box && !box.checked) {
          box.click();
        }
      });
      await page.waitForTimeout(300);
      console.log(
        `[globalSetup] JS 兜底后协议状态: ${await page
          .locator('.el-checkbox input[type="checkbox"]')
          .first()
          .isChecked()
          .catch(() => 'unknown')}`
      );
    }
    // 表单校验错误提示（协议未勾等）
    const formErrors = await page
      .locator('.el-form-item__error')
      .allTextContents()
      .catch(() => []);
    if (formErrors.length > 0) {
      console.log(`[globalSetup] 提交前表单错误: ${JSON.stringify(formErrors)}`);
    }
    const loginBtn = page.getByRole('button', { name: /登录|登 录/ }).first();
    console.log(`[globalSetup] 登录按钮可见: ${await loginBtn.isVisible().catch(() => false)}`);
    // force click：登录页可能有 loading 遮罩或 lockInfo 状态导致 actionability 检查失败，
    // force 绕过遮挡检测直接点击（按钮本身 disabled=false 已确认可见）
    await loginBtn.click({ force: true }).catch(() => {});
    await page.waitForTimeout(500);
    // 如果首次 force click 没跳转，尝试 JS 直接触发 form submit
    const currentUrl1 = page.url();
    if (currentUrl1.includes('/login')) {
      await page.evaluate(() => {
        const btn = document.querySelector(
          'button[type="primary"], .el-button--primary'
        ) as HTMLButtonElement | null;
        if (btn) btn.click();
        const form = document.querySelector('form') as HTMLFormElement | null;
        if (form) form.dispatchEvent(new Event('submit', { cancelable: true, bubbles: true }));
      });
      await page.waitForTimeout(500);
    }
    // 等待跳转或捕获登录后错误提示
    await page
      .waitForURL(url => !url.pathname.includes('/login'), { timeout: 60_000 })
      .catch(async () => {
        const afterErrors = await page
          .locator('.el-form-item__error, .el-message__content')
          .allTextContents()
          .catch(() => []);
        const currentUrl = page.url();
        console.error(
          `[globalSetup] UI 登录未跳转: url=${currentUrl}, 提示=${JSON.stringify(afterErrors)}`
        );
        await page.screenshot({ path: 'e2e/.auth/globalsetup-login-fail.png', fullPage: true });
        throw new Error(
          `globalSetup UI 登录未跳转（60s）：url=${currentUrl}，提示=${JSON.stringify(afterErrors)}`
        );
      });

    // 打开系统管理页（用户管理 Tab）
    await page.goto(`${FRONTEND_BASE}/system`, { waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(2000);

    // 打开新建用户对话框
    const createBtn = page.getByRole('button', { name: /新建用户/ }).first();
    await createBtn.waitFor({ state: 'visible', timeout: 60_000 });
    await createBtn.click();
    const dialog = page.locator('.el-dialog:visible').last();
    await dialog.waitFor({ state: 'visible', timeout: 30_000 });
    await page.waitForTimeout(300);

    // 填用户名/密码/姓名（表单字段 label：用户名/密码/姓名/角色）
    const userInput = dialog
      .locator('.el-form-item')
      .filter({ hasText: '用户名' })
      .locator('input')
      .first();
    await userInput.waitFor({ state: 'visible', timeout: 20_000 });
    await userInput.fill(SHARD_USERNAME);

    const pwdInput = dialog
      .locator('.el-form-item')
      .filter({ hasText: '密码' })
      .locator('input')
      .first();
    await pwdInput.fill(SHARD_PASSWORD);

    const nameInput = dialog
      .locator('.el-form-item')
      .filter({ hasText: '姓名' })
      .locator('input')
      .first();
    await nameInput.fill(`E2E分片${SHARD_INDEX}`);

    // 角色下拉：选 admin
    const roleItem = dialog.locator('.el-form-item').filter({ hasText: '角色' }).first();
    const roleSelect = roleItem.locator('.el-select__wrapper, .el-select').first();
    if ((await roleSelect.count()) > 0) {
      await roleSelect.click();
      const dropdown = page.locator('.el-select-dropdown:visible').last();
      await dropdown.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});
      const adminOption = dropdown
        .locator('.el-select-dropdown__item')
        .filter({ hasText: /admin/i })
        .first();
      if ((await adminOption.count()) > 0) {
        await adminOption.click();
      } else {
        const first = dropdown.locator('.el-select-dropdown__item').first();
        if ((await first.count()) > 0) await first.click();
      }
      await page.waitForTimeout(300);
    }

    // 提交（按钮文本：确定/保存/提交）
    const submitBtn = dialog.getByRole('button', { name: /确定|保存|提交/ }).last();
    await submitBtn.waitFor({ state: 'visible', timeout: 20_000 });
    await submitBtn.click();

    // 等待创建结果：成功（列表刷新/成功提示）或"已存在"
    const outcome = await page
      .waitForResponse(r => r.url().includes('/users') && r.request().method() === 'POST', {
        timeout: 30_000,
      })
      .then(r => r.status())
      .catch(() => 0);
    if (outcome === 200 || outcome === 201) {
      console.log(`[globalSetup] 分片账号 ${SHARD_USERNAME} UI 创建成功 (HTTP ${outcome})`);
    } else if (outcome === 0) {
      // 无 POST 响应：可能前端表单校验失败或账号已存在导致 UI 阻止提交，
      // 校验账号是否已可登录（已存在 → 合法跳过）
      console.warn(`[globalSetup] 未捕获 POST /users 响应 (status=${outcome})，验证账号是否已存在`);
    } else {
      console.warn(`[globalSetup] 分片账号创建 HTTP ${outcome}（可能已存在）`);
    }

    // 终验：分片账号必须可登录（创建成功或已存在均通过）
    const checkCtx = await request.newContext({ baseURL: API_BASE });
    const loginCheck = await checkCtx.post(`${API_PREFIX}/auth/login`, {
      data: { username: SHARD_USERNAME, password: SHARD_PASSWORD },
    });
    await checkCtx.dispose();
    if (!loginCheck.ok()) {
      const body = await loginCheck.text();
      throw new Error(
        `分片账号 ${SHARD_USERNAME} 终验失败（UI 创建与已存在均未通过）: HTTP ${loginCheck.status()} ${body}`
      );
    }
    console.log(`[globalSetup] 分片账号 ${SHARD_USERNAME} 就绪（登录验证通过）`);
    void expect; // 保持 import 一致性（断言在终验逻辑中体现）
  } finally {
    await context.close().catch(() => {});
    await browser.close().catch(() => {});
  }
}
