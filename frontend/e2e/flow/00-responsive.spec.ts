import { test, expect, devices, type Page } from '@playwright/test';
import { loginViaUI } from './helpers';

/**
 * 响应式适配测试：安卓（国产品牌为主）+ 苹果全尺寸 + 桌面
 *
 * 项目要求：窗口自动适配屏幕尺寸，手机端（安卓/苹果各机型）与桌面端均可运行。
 *
 * 机型矩阵设计说明：
 * - Playwright devices 库无国内品牌机型，国产品牌用自定义描述符（真实 CSS
 *   视口规格 + Android UA + 触摸模拟）。响应式行为由 CSS 视口宽度决定，
 *   按断点带选取代表机型即全带宽覆盖：
 *   · 375-399 主流带：iPhone SE 375（Playwright 最小 iPhone）/ 华为 P50 Pro 388 /
 *     小米14 393 / vivo X90 Pro 396
 *   · 400-430 大屏带：华为 Mate60 Pro 408 / 荣耀 Magic5 408 / OPPO Find X6 Pro 408 /
 *     一加11 412 / iPhone 14 390（中间值）/ iPhone 14 Pro Max 430（上界）
 *   · 768-833 平板：iPad Mini 768 / 华为 MatePad 11 768
 *   · 834-991 平板大：OPPO Pad 2 800 / 三星 Tab S9 800 / iPad Pro 11 834 / 小米 Pad 6 860
 *   · >=992 桌面带：1366 / 1920 + 横屏（iPhone 14 844 仍在抽屉带 / 小米 Pad 6 1357 切桌面）
 * - 断点契约（useBreakpoint）：<992 移动端（抽屉） / >=992 桌面（固定栏）
 * - theme.css：<768 对话框 92vw；触控目标 >=44px
 */

type CnDevice = {
  name: string;
  viewport: { width: number; height: number };
  deviceScaleFactor: number;
  isMobile: boolean;
  hasTouch: boolean;
  userAgent: string;
};

/** 安卓国产品牌 UA 模板（HarmonyOS 设备 UA 与 Android 一致） */
const cnUa = (model: string, v: string) =>
  `Mozilla/5.0 (Linux; Android ${v}; ${model}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Mobile Safari/537.36`;
const cnUaPad = (model: string, v: string) =>
  `Mozilla/5.0 (Linux; Android ${v}; ${model}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36`;

/** 国内品牌手机（竖屏） */
const CN_PHONES: CnDevice[] = [
  {
    name: '华为 Mate 60 Pro',
    viewport: { width: 408, height: 886 },
    deviceScaleFactor: 3,
    isMobile: true,
    hasTouch: true,
    userAgent: cnUa('ALN-AL00', '14'),
  },
  {
    name: '华为 P50 Pro',
    viewport: { width: 388, height: 844 },
    deviceScaleFactor: 3,
    isMobile: true,
    hasTouch: true,
    userAgent: cnUa('JAD-AL50', '12'),
  },
  {
    name: '荣耀 Magic5',
    viewport: { width: 408, height: 886 },
    deviceScaleFactor: 3,
    isMobile: true,
    hasTouch: true,
    userAgent: cnUa('PGT-AN10', '13'),
  },
  {
    name: '小米14',
    viewport: { width: 393, height: 851 },
    deviceScaleFactor: 3,
    isMobile: true,
    hasTouch: true,
    userAgent: cnUa('23127PN0CC', '14'),
  },
  {
    name: 'vivo X90 Pro',
    viewport: { width: 396, height: 880 },
    deviceScaleFactor: 3,
    isMobile: true,
    hasTouch: true,
    userAgent: cnUa('V2241A', '13'),
  },
  {
    name: 'OPPO Find X6 Pro',
    viewport: { width: 408, height: 892 },
    deviceScaleFactor: 3,
    isMobile: true,
    hasTouch: true,
    userAgent: cnUa('PGEM10', '13'),
  },
  {
    name: '一加 11',
    viewport: { width: 412, height: 915 },
    deviceScaleFactor: 3,
    isMobile: true,
    hasTouch: true,
    userAgent: cnUa('PHB110', '13'),
  },
];

/** 国内品牌平板（竖屏，均 <992 → 抽屉形态） */
const CN_TABLETS: CnDevice[] = [
  {
    name: '华为 MatePad 11',
    viewport: { width: 768, height: 1024 },
    deviceScaleFactor: 2,
    isMobile: true,
    hasTouch: true,
    userAgent: cnUaPad('DBY-W09', '12'),
  },
  {
    name: '小米 Pad 6',
    viewport: { width: 860, height: 1357 },
    deviceScaleFactor: 2.75,
    isMobile: true,
    hasTouch: true,
    userAgent: cnUaPad('M82', '13'),
  },
  {
    name: 'OPPO Pad 2',
    viewport: { width: 800, height: 1337 },
    deviceScaleFactor: 2.5,
    isMobile: true,
    hasTouch: true,
    userAgent: cnUaPad('OPD2203', '13'),
  },
  {
    name: '三星 Galaxy Tab S9',
    viewport: { width: 800, height: 1176 },
    deviceScaleFactor: 2.25,
    isMobile: true,
    hasTouch: true,
    userAgent: cnUaPad('SM-X710', '14'),
  },
];

/** 苹果手机（devices 库原生描述符，竖屏） */
const APPLE_PHONES = [
  { name: 'iPhone SE', device: devices['iPhone SE'] },
  { name: 'iPhone 14', device: devices['iPhone 14'] },
  { name: 'iPhone 14 Pro Max', device: devices['iPhone 14 Pro Max'] },
];

/** 苹果平板 */
const APPLE_TABLETS = [
  { name: 'iPad Mini', device: devices['iPad Mini'] },
  { name: 'iPad Pro 11', device: devices['iPad Pro 11'] },
];

/** 溢出容差（滚动条宽度/亚像素取整） */
const OVERFLOW_TOLERANCE = 2;

/** 登录页断言：未认证上下文 + 卡片宽度不超视口 */
async function assertLoginPageFits(page: Page, viewportWidth: number) {
  await page.goto('/login', { waitUntil: 'domcontentloaded' });
  // 全局 storageState 已清除（test.use），此处必然停留登录页
  await expect(page).toHaveURL(/\/login/);
  await expectNoHorizontalOverflow(page);
  const card = page.locator('.login-card').first();
  await expect(card).toBeVisible();
  const box = await card.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.width).toBeLessThanOrEqual(viewportWidth);
}

/** 登录页全机型（storageState 置空：清除全局登录态，避免路由守卫弹回 Dashboard） */
test.describe('登录页·全机型视口适配', () => {
  test.use({ storageState: undefined });

  for (const d of APPLE_PHONES) {
    test(`登录页不溢出 · ${d.name}`, async ({ page }) => {
      await page.setViewportSize(d.device.viewport);
      await assertLoginPageFits(page, d.device.viewport.width);
    });
  }
  for (const d of CN_PHONES) {
    test.describe(d.name, () => {
      // 只传标准 TestOptions 字段，name 留作标题
      test.use({
        viewport: d.viewport,
        deviceScaleFactor: d.deviceScaleFactor,
        isMobile: d.isMobile,
        hasTouch: d.hasTouch,
        userAgent: d.userAgent,
      });
      test('登录页不溢出', async ({ page }) => {
        await assertLoginPageFits(page, d.viewport.width);
      });
    });
  }
});

/** 页面横向无溢出断言：document scrollWidth 不超过视口宽度 */
async function expectNoHorizontalOverflow(page: Page) {
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth
  );
  expect(overflow).toBeLessThanOrEqual(OVERFLOW_TOLERANCE);
}

/** 手机/平板全机型：主布局形态 + Dashboard 溢出（登录态） */
test.describe('主应用·全机型视口适配', () => {
  const mobileCases = [
    ...APPLE_PHONES.map(a => ({ name: a.name, viewport: a.device.viewport })),
    ...CN_PHONES.map(c => ({ name: c.name, viewport: c.viewport })),
    ...APPLE_TABLETS.map(a => ({ name: a.name, viewport: a.device.viewport })),
    ...CN_TABLETS.map(c => ({ name: c.name, viewport: c.viewport })),
  ];

  for (const c of mobileCases) {
    test(`Dashboard 无溢出 + 抽屉化 · ${c.name}(${c.viewport.width}x${c.viewport.height})`, async ({
      page,
    }) => {
      await loginViaUI(page);
      await page.setViewportSize(c.viewport);
      await page.goto('/dashboard', { waitUntil: 'domcontentloaded' });
      await page
        .locator('.main-layout, .el-main, [class*="dashboard"]')
        .first()
        .waitFor({ state: 'visible', timeout: 30_000 });
      await page.waitForTimeout(800); // 图表渲染稳定
      await expectNoHorizontalOverflow(page);

      // <992 契约：固定侧边栏不存在、汉堡按钮存在
      await expect(page.locator('.aside')).toHaveCount(0);
      await expect(page.locator('.hamburger-btn').first()).toBeVisible();

      // 抽屉可开（direction=ltr 从左滑出，size 260px）
      await page.locator('.hamburger-btn').first().click();
      await expect(page.locator('.mobile-sidebar').first()).toBeVisible({ timeout: 5000 });
      // ESC 关闭（closeOnPressEscape: true），关闭后 DOM 保留但不可见（destroyOnClose: false）
      await page.keyboard.press('Escape');
      await expect(page.locator('.mobile-sidebar').first()).not.toBeVisible({ timeout: 5000 });
    });
  }
});

/** 断点边界契约：991/992 一像素之差形态必须切换 */
test.describe('断点边界与横屏', () => {
  test('991px 抽屉 / 992px 固定栏（断点精确契约）', async ({ page }) => {
    await loginViaUI(page);
    await page.setViewportSize({ width: 991, height: 800 });
    await page.goto('/dashboard', { waitUntil: 'domcontentloaded' });
    await expect(page.locator('.aside')).toHaveCount(0);
    await expect(page.locator('.hamburger-btn').first()).toBeVisible();

    await page.setViewportSize({ width: 992, height: 800 });
    await page.waitForTimeout(500);
    await expect(page.locator('.aside').first()).toBeVisible();
    await expect(page.locator('.hamburger-btn')).toHaveCount(0);
  });

  test('iPhone 14 横屏（844×390）：仍为抽屉形态且无溢出', async ({ page }) => {
    await loginViaUI(page);
    await page.setViewportSize({ width: 844, height: 390 });
    await page.goto('/dashboard', { waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(500);
    await expectNoHorizontalOverflow(page);
    await expect(page.locator('.aside')).toHaveCount(0);
    await expect(page.locator('.hamburger-btn').first()).toBeVisible();
  });

  test('小米 Pad 6 横屏（1357×860）：跨过 992 切桌面形态', async ({ page }) => {
    await loginViaUI(page);
    await page.setViewportSize({ width: 1357, height: 860 });
    await page.goto('/dashboard', { waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(500);
    await expectNoHorizontalOverflow(page);
    await expect(page.locator('.aside').first()).toBeVisible();
    await expect(page.locator('.hamburger-btn')).toHaveCount(0);
  });
});

/** 桌面端 */
test.describe('桌面端视口', () => {
  for (const vp of [
    { width: 1366, height: 768 },
    { width: 1920, height: 1080 },
  ]) {
    test(`Dashboard 正常 + 固定侧边栏 · ${vp.width}x${vp.height}`, async ({ page }) => {
      await loginViaUI(page);
      await page.setViewportSize(vp);
      await page.goto('/dashboard', { waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(500);
      await expectNoHorizontalOverflow(page);
      await expect(page.locator('.aside').first()).toBeVisible();
      await expect(page.locator('.hamburger-btn')).toHaveCount(0);
    });
  }

  test('视口动态切换热适配：桌面→手机→桌面实时切换', async ({ page }) => {
    await loginViaUI(page);
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto('/dashboard', { waitUntil: 'domcontentloaded' });
    await expect(page.locator('.aside').first()).toBeVisible();

    await page.setViewportSize({ width: 390, height: 844 });
    await page.waitForTimeout(500);
    await expect(page.locator('.aside')).toHaveCount(0);
    await expect(page.locator('.hamburger-btn')).toBeVisible();

    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.waitForTimeout(500);
    await expect(page.locator('.aside').first()).toBeVisible();
    await expect(page.locator('.hamburger-btn')).toHaveCount(0);
  });
});
