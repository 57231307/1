import { test, expect, type Locator, type Page } from "../diagnose-fixture";
import { loginViaUI, BASE_URL } from "./helpers";

/*
 * 移动端缺口带（769 到 991 视口）显示异常回归用例。
 *
 * 调用方：CI 的 e2e flow 分片与本地 playwright --list，视口由本文件自行设定。
 * 入参：page，由 diagnose-fixture 注入，带会话与诊断上下文。
 * 传给谁：断言对象是 frontend/src/styles/mobile-gap-zone.css 这一补丁层与
 *         frontend/src/composables/useBreakpoint.ts 的 992 断点判定是否同口径。
 * 存什么：三段可复现的显示事实，弹窗右缘不越视口、页面无文档级横向溢出、
 *         该宽度下侧栏确实走移动抽屉分支。
 * 存哪里：本文件，随 e2e/flow 一起被 87 号作业矩阵收集。
 *
 * 为什么单独一段视口。isMobile 判定是视口宽小于 992，而 theme.css 与 mobile-list.css 与
 * mobile-content.css 三处移动适配只写到 max-width 768px。769 到 991 这一段（手机横屏与窄平板）
 * 因此既走移动布局又没有移动宽度兜底，声明 width 800px 的弹窗直接顶破可视区右缘。
 * 本用例钉的就是这段宽度，改坏任一側都会在这里判红。
 */

const GAP_ZONE = { width: 820, height: 1180 };
const MOBILE_NARROW = { width: 375, height: 812 };

/** 打开线索页头部动作区里的建档弹窗，按钮文案取新建族词表，不写死单一条文案 */
async function openLeadCreateDialog(page: Page): Promise<Locator> {
  const trigger = page.locator(".header-actions").getByRole("button", { name: /新建|新增|添加|创建/ }).first();
  await expect(trigger, "线索页头部动作区应有建档入口，否则用例前置页面形态已变").toBeVisible();
  await trigger.click();
  const dialog = page.locator(".el-dialog").first();
  await expect(dialog, "点击建档入口后弹窗必须出现").toBeVisible();
  return dialog;
}

async function gotoRoute(page: Page, route: string): Promise<void> {
  await page.goto(`${BASE_URL}${route}`, { waitUntil: "domcontentloaded" });
  await page.waitForLoadState("networkidle", { timeout: 8000 }).catch(() => {});
}

test.describe("移动端缺口带显示回归", () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize(GAP_ZONE);
    await loginViaUI(page);
  });

  test("缺口带内侧栏按 isMobile 走抽屉分支，断点口径两侧一致", async ({ page }) => {
    await gotoRoute(page, "/crm/leads");

    const drawer = page.locator(".mobile-sidebar");
    await expect(
      drawer,
      `视口宽 ${GAP_ZONE.width} 小于 992，MainLayout 应把侧栏渲染成抽屉而非桌面侧栏`
    ).toBeVisible();
    await expect(
      page.locator(".aside"),
      "同一视口下不应同时存在桌面侧栏，说明 JS 与 CSS 断点判定又分叉了"
    ).toHaveCount(0);
  });

  test("缺口带内 800px 声明宽的建档弹窗收进视口且内部不横向滚动", async ({ page }) => {
    await gotoRoute(page, "/crm/leads");
    const dialog = await openLeadCreateDialog(page);

    const viewport = page.viewportSize();
    if (!viewport) throw new Error("视口未设置，无法测量弹窗宽度");

    const box = await dialog.boundingBox();
    if (!box) throw new Error("弹窗未渲染出可测量区域");

    expect(box.x, `弹窗左边缘 ${box.x} 不应移出视口`).toBeGreaterThanOrEqual(0);
    expect(
      box.x + box.width,
      `弹窗右边缘 ${box.x + box.width} 不应超出视口宽 ${viewport.width}`
    ).toBeLessThanOrEqual(viewport.width + 1);
    expect(
      box.width,
      "弹窗应收进可视区，但仍要保持可用宽度，不得被压成窄条"
    ).toBeGreaterThanOrEqual(viewport.width * 0.7);

    const innerOverflow = await dialog.evaluate(el => el.scrollWidth - el.clientWidth);
    expect(innerOverflow, "弹窗内部不应出现横向滚动").toBeLessThanOrEqual(2);
  });

  test("缺口带内列表页无文档级横向溢出", async ({ page }) => {
    await gotoRoute(page, "/crm/leads");

    const overflow = await page.evaluate(() => {
      const doc = document.documentElement;
      return { scrollWidth: doc.scrollWidth, clientWidth: doc.clientWidth };
    });

    expect(
      overflow.scrollWidth,
      `页面滚动宽 ${overflow.scrollWidth} 不应大于可视宽 ${overflow.clientWidth}，表格自身的横向滚动不在此列`
    ).toBeLessThanOrEqual(overflow.clientWidth + 1);
  });

  test("窄屏竖屏段的既有适配不因缺口带补丁回退", async ({ page }) => {
    await page.setViewportSize(MOBILE_NARROW);
    await gotoRoute(page, "/crm/leads");
    const dialog = await openLeadCreateDialog(page);

    const viewport = page.viewportSize();
    if (!viewport) throw new Error("视口未设置，无法测量弹窗宽度");

    const box = await dialog.boundingBox();
    if (!box) throw new Error("弹窗未渲染出可测量区域");

    expect(box.x + box.width, "竖屏段弹窗仍须收进视口").toBeLessThanOrEqual(viewport.width + 1);
    expect(
      box.width,
      `竖屏段弹窗应展开到接近满屏宽，实测 ${box.width} 说明 768px 那层被本补丁挤掉了`
    ).toBeGreaterThanOrEqual(viewport.width * 0.75);
  });
});
