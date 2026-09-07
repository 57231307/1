import { test, expect, type Page } from '@playwright/test';
import { readFileSync } from 'fs';
import { execSync } from 'child_process';

/**
 * 引导页（/setup）初始化链路真实数据 E2E（禁止 mock，用户指令）
 *
 * 与真实环境的完整契约：
 * - 后端以真实二进制启动（CI: ci-build-rust artifact，端口 8083）
 * - 专用空库 bingxi_setup_test（e2e/scripts/setup-wizard-real-env.sh 重置），
 *   空库无表 → 后端启动即 Setup 模式（仅 /init/* 路由）
 * - 初始化通过 UI 真实点击走完：环境检查 → test-database → initialize-with-db
 *   （真实执行迁移 60+ 个 + 种子数据 + Argon2id 管理员哈希）→ 后端自退
 * - 后端重启（e2e/scripts/restart-backend-full.sh 等效 systemd 拉起）→
 *   完整模式 → 真实登录验证
 *
 * 前置（由 CI step 运行，本 spec 断言前置成立）：
 *   bash frontend/e2e/scripts/setup-wizard-real-env.sh
 * 凭据文件：/tmp/e2e-setup-logs/test-context.env
 *
 * 路径隔离：本 spec 位于 e2e/setup-wizard/（不在 e2e/flow/），
 * 主 CI 的 34 分片 `npx playwright test ... e2e/flow/` 不会拾取，
 * 仅由 ci-e2e-setup-wizard job 用 playwright.setup-wizard.config.ts 运行
 */

const API_BASE = process.env.SETUP_E2E_API_BASE || 'http://127.0.0.1:8083';

/** 读取环境准备脚本写入的凭据（真实 INIT_TOKEN/管理员账号） */
function loadContext(): {
  db: string;
  admin: string;
  pass: string;
  initToken: string;
} {
  const raw = readFileSync('/tmp/e2e-setup-logs/test-context.env', 'utf-8');
  const get = (k: string) => {
    const m = raw.match(new RegExp(`^${k}=(.*)$`, 'm'));
    if (!m) throw new Error(`test-context.env 缺少 ${k}（前置脚本未运行？）`);
    return m[1].trim();
  };
  return {
    db: get('SETUP_E2E_DB'),
    admin: get('SETUP_E2E_ADMIN'),
    pass: get('SETUP_E2E_PASS'),
    initToken: get('SETUP_E2E_INIT_TOKEN'),
  };
}

function btn(page: Page, texts: string[]) {
  const selector = texts.map(t => `button:has-text("${t}")`).join(', ');
  return page.locator(selector).first();
}

test.describe.serial('引导页初始化真实链路（真实后端 + 真实 PostgreSQL，零 mock）', () => {
  let ctx: ReturnType<typeof loadContext>;

  test.beforeAll(() => {
    ctx = loadContext();
  });

  test('前置断言：后端处于真实 Setup 模式（/init/status 返回 mode=setup 且 initialized=false）', async ({
    request,
  }) => {
    // 真实后端响应（无任何 route mock）：空库无表 → Setup 模式
    const resp = await request.get(`${API_BASE}/api/v1/erp/init/status`);
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.initialized).toBe(false);
    expect(body.mode).toBe('setup');
  });

  test('环境检查真实通过：/health 404（Setup 模式无该路由）+ /init/status 可达 → 检查全绿', async ({
    page,
  }) => {
    await page.goto('/setup', { waitUntil: 'domcontentloaded' });
    // 真实链路：后端 Setup 模式 /health 返回 404，环境检查按修复逻辑
    // 以 /init/status 判定后端可达（原实现 json() 抛错导致死门的回归防护）
    await expect(page.locator('.check-item .success').first()).toBeVisible({ timeout: 30_000 });
    await expect(page.locator('.check-item .success')).toHaveCount(3);
    await expect(btn(page, ['下一步', 'Next'])).toBeEnabled({ timeout: 10_000 });
  });

  test('数据库连接真实测试：填真实凭据 → test-database 200 → 下一步解禁', async ({ page }) => {
    await page.goto('/setup', { waitUntil: 'domcontentloaded' });
    await expect(page.locator('.check-item .success').first()).toBeVisible({ timeout: 30_000 });
    await btn(page, ['下一步', 'Next']).click();
    await expect(page.locator('input[placeholder="localhost"]')).toBeVisible({
      timeout: 10_000,
    });

    // 真实数据库凭据（专用空库 bingxi_setup_test）
    await page.locator('input[placeholder="localhost"]').fill('127.0.0.1');
    await page.locator('input[placeholder="5432"]').fill('5432');
    await page.locator('input[placeholder="bingxi"]').first().fill('bingxi');
    const dbPwd = page.locator('input[type="password"]');
    await dbPwd.nth(0).fill('bingxi_test');
    await dbPwd.nth(1).fill(ctx.initToken);

    await btn(page, ['测试', '连接', 'Test']).click();
    // 真实 SELECT 1 探测通过 → 成功提示 + 下一步解禁
    await expect(page.locator('.el-message--success').first()).toBeVisible({ timeout: 15_000 });
    await expect(btn(page, ['下一步', 'Next'])).toBeEnabled({ timeout: 10_000 });
  });

  test('真实初始化全流程：环境→数据库→管理员→安装（真实迁移+种子数据）→完成页', async ({
    page,
  }) => {
    await page.goto('/setup', { waitUntil: 'domcontentloaded' });
    await expect(page.locator('.check-item .success').first()).toBeVisible({ timeout: 30_000 });
    await btn(page, ['下一步', 'Next']).click();
    await expect(page.locator('input[placeholder="localhost"]')).toBeVisible({
      timeout: 10_000,
    });

    // 步骤 2：真实数据库配置 + 真实 INIT_TOKEN
    await page.locator('input[placeholder="localhost"]').fill('127.0.0.1');
    await page.locator('input[placeholder="5432"]').fill('5432');
    await page.locator('input[placeholder="bingxi"]').first().fill('bingxi');
    const dbPwd = page.locator('input[type="password"]');
    await dbPwd.nth(0).fill('bingxi_test');
    await dbPwd.nth(1).fill(ctx.initToken);
    await btn(page, ['测试', '连接', 'Test']).click();
    await expect(btn(page, ['下一步', 'Next'])).toBeEnabled({ timeout: 15_000 });
    await btn(page, ['下一步', 'Next']).click();

    // 步骤 3：真实管理员账号（Argon2id 哈希落库）
    await page.locator('input[placeholder="admin"]').fill(ctx.admin);
    const adminPwd = page.locator('input[type="password"]');
    await adminPwd.nth(0).fill(ctx.pass);
    await adminPwd.nth(1).fill(ctx.pass);
    await expect(btn(page, ['下一步', 'Next'])).toBeEnabled({ timeout: 10_000 });
    await btn(page, ['下一步', 'Next']).click();

    // 步骤 4：安装（真实执行 60+ 迁移 + 角色/部门/权限种子 + 管理员创建）
    // 真实迁移耗时显著长于 mock：超时放宽到 120s
    await expect(btn(page, ['安装'])).toBeVisible({ timeout: 10_000 });
    await btn(page, ['安装']).click();

    // 完成页真实渲染（install 响应 code=200 且 data.success=true）
    await expect(page.locator('.success-icon .el-icon')).toBeVisible({ timeout: 120_000 });
    await expect(btn(page, ['登录'])).toBeVisible({ timeout: 10_000 });
    await expect(page.locator('.el-message--success').first()).toBeVisible({ timeout: 10_000 });
  });

  test('初始化后数据库真实校验：users 表有管理员行（PSQL 直查，数据级断言）', async () => {
    // 数据级验证：初始化真实发生（不依赖 UI 状态）。
    // CI（postgres service）：psql 直连；本地（su postgres）：socket 直连
    const sql = `SELECT username || '|' || is_active FROM users WHERE username = '${ctx.admin}';`;
    let out = '';
    try {
      out = execSync(
        `PGPASSWORD=bingxi_test psql -h 127.0.0.1 -U bingxi -d ${ctx.db} -tAc "${sql}"`
      ).toString();
    } catch {
      out = execSync(`su postgres -c "psql -d ${ctx.db} -tAc \\"${sql}\\""`).toString();
    }
    expect(out.trim()).toBe(`${ctx.admin}|t`);
  });

  test('后端自退重启后完整模式就绪：/health 200 + /init/status 返回已初始化', async ({
    request,
  }) => {
    // 真实进程管理：后端 initialize 成功后自退（exit 0），本 step 等效
    // systemd Restart=always 拉起（同一套环境变量，同库已初始化 → 完整模式）。
    // 重启脚本自身带 60s 探活，非零退出即真实失败。
    // REPO_ROOT 由 CI step 注入（checkout 目录）；本地默认 /workspace
    execSync('bash frontend/e2e/scripts/restart-backend-full.sh', {
      cwd: process.env.REPO_ROOT || '/workspace',
      stdio: 'inherit',
      env: { ...process.env, BACKEND_BIN: process.env.BACKEND_BIN ?? '' },
    });

    // 完整模式 /health 返回 200 healthy（重启脚本已探活，此处断言契约形态）
    const health = await request.get(`${API_BASE}/health`);
    expect(health.ok()).toBeTruthy();
    const healthBody = await health.json();
    expect(healthBody.status).toBe('healthy');

    // 完整模式 /init/status：code=200 + data.initialized=true
    const resp = await request.get(`${API_BASE}/api/v1/erp/init/status`);
    const body = await resp.json();
    expect(body.code).toBe(200);
    expect(body.data.initialized).toBe(true);
  });

  test('初始化后真实登录：新管理员账号新建会话成功 + 拿到完整权限', async ({ request }) => {
    // 真实登录（无 mock）：Argon2id 校验 + access_token/refresh_token/csrf_token Cookie
    const resp = await request.post(`${API_BASE}/api/v1/erp/auth/login`, {
      data: { username: ctx.admin, password: ctx.pass },
      headers: { 'X-Requested-With': 'XMLHttpRequest' },
    });
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.code).toBe(200);
    expect(body.data.user.username).toBe(ctx.admin);
    // admin 角色注入 *:* 通配权限（fetch_role_permissions 真实 DB 查询）
    expect(body.data.permissions).toContain('*:*');

    // Cookie 真实下发（httpOnly Cookie 鉴权链路）
    const cookies = await (resp as unknown as { headersArray?(): Array<{ name: string }> })
      .headersArray?.()
      .then(arr => arr?.map(h => h.name))
      .catch(() => undefined);
    // Playwright APIRequestContext 不直接暴露 Set-Cookie 明细，用 storageState 校验
    const state = await request.storageState();
    const names = state.cookies.map(c => c.name);
    expect(names).toContain('access_token');
    expect(names).toContain('csrf_token');
    void cookies;
  });

  test('UI 真实登录跳转：向导页去登录 → 登录页渲染', async ({ page }) => {
    await page.goto('/setup', { waitUntil: 'domcontentloaded' });
    // 初始化完成后守卫查 /init/status：initialized=true → 不再强制 /setup
    // 完整模式下直接访问 /login 验证渲染
    await page.goto('/login', { waitUntil: 'domcontentloaded' });
    await expect(
      page.locator('input[placeholder="用户名"], input[placeholder="Username"]').first()
    ).toBeVisible({ timeout: 15_000 });
  });

  test('UI 真实登录进入主应用：向导创建的管理员可登录并看到 Dashboard', async ({ page }) => {
    await page.goto('/login', { waitUntil: 'domcontentloaded' });
    const usernameInput = page.locator(
      'input[placeholder="用户名"], input[placeholder="Username"]'
    );
    await usernameInput.first().waitFor({ state: 'visible', timeout: 30_000 });
    await usernameInput.first().fill(ctx.admin);
    const passwordInput = page.locator('input[placeholder="密码"], input[placeholder="Password"]');
    await passwordInput.first().waitFor({ state: 'visible', timeout: 30_000 });
    await passwordInput.first().fill(ctx.pass);

    const loginButton = page.locator('form button.el-button--primary').first();
    await loginButton.waitFor({ state: 'visible', timeout: 10_000 });
    await loginButton.click();

    // 真实登录成功 → 离开 /login（Cookie 会话建立）
    await page.waitForURL(url => !url.pathname.includes('/login'), { timeout: 40_000 });
    await expect(page.locator('.main-layout, .el-main, [class*="dashboard"]').first()).toBeVisible({
      timeout: 30_000,
    });
  });
});
