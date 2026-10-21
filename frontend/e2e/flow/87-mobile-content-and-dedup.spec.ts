import { test, expect, type Locator, type Page } from '../diagnose-fixture';
import { loginViaUI, BASE_URL } from './helpers';

/**
 * 移动端内容层适配与重复提交防重
 *
 * 功能，补两类桌面视口或只测正向流程测不出的缺陷。
 * 一 内容层适配，移动端弹窗若沿用 640 像素固定宽会超出手机屏，表单标签仍横排会把输入框压到
 * 无法输入，mobile-content.css 负责在窄屏下把弹窗收进视口、标签置顶、输入框占满内容宽。
 * 二 重复提交，提交锁必须在异步校验之前落下，否则连点两下会并发建出两条记录。
 * 三 重复入口，旧会计期间页已由新页取代，旧路径只保留重定向，侧栏不得再出现第二条入口。
 *
 * 调用方，frontend e2e flow 分片。入参，页面与登录态由 fixture 提供。
 * 传给谁，断言结果只进 Playwright 报告。
 * 存什么，不落库。重复提交用路由拦截伪造成功响应，不向真实接口写数据。
 */

const MOBILE_PORTRAIT = { width: 393, height: 852 };

async function gotoRoute(page: Page, route: string): Promise<void> {
  await page.goto(`${BASE_URL}${route}`, { waitUntil: 'domcontentloaded' });
  await page.waitForLoadState('networkidle', { timeout: 8000 }).catch(() => {});
}

async function openCreateDialog(page: Page): Promise<Locator> {
  await page.getByRole('button', { name: '新建目标' }).click();
  const dialog = page.locator('.el-dialog').first();
  await expect(dialog).toBeVisible();
  return dialog;
}

test.describe('移动端内容层适配', () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize(MOBILE_PORTRAIT);
    await loginViaUI(page);
  });

  test('弹窗收进视口且表单标签置顶、输入框占满内容宽', async ({ page }) => {
    await gotoRoute(page, '/print-targets');
    const dialog = await openCreateDialog(page);

    const viewport = page.viewportSize();
    if (!viewport) throw new Error('视口未设置，无法测量弹窗宽度');
    const box = await dialog.boundingBox();
    if (!box) throw new Error('弹窗未渲染出可测量区域');

    expect(box.width, `弹窗宽 ${box.width} 应不超过视口宽减 16`).toBeLessThanOrEqual(
      viewport.width - 16
    );
    expect(box.width, '弹窗应展开到接近满屏宽').toBeGreaterThanOrEqual(viewport.width * 0.75);
    expect(box.x, '弹窗左边缘不应移出视口').toBeGreaterThanOrEqual(0);

    const innerOverflow = await dialog.evaluate(el => el.scrollWidth - el.clientWidth);
    expect(innerOverflow, '弹窗内部不应出现横向滚动').toBeLessThanOrEqual(2);

    const item = dialog.locator('.el-form-item').first();
    const labelBox = await item.locator('.el-form-item__label').boundingBox();
    const contentBox = await item.locator('.el-form-item__content').boundingBox();
    if (!labelBox || !contentBox) throw new Error('表单项未渲染出标签或内容区');

    expect(
      labelBox.y + labelBox.height,
      '标签应整体位于输入区上方，而非占据左侧固定宽度'
    ).toBeLessThanOrEqual(contentBox.y + 2);

    const inputBox = await item.locator('.el-input').first().boundingBox();
    if (!inputBox) throw new Error('表单项未渲染出输入框');
    expect(inputBox.width, '输入框应占满内容区宽度').toBeGreaterThanOrEqual(contentBox.width - 4);
  });

  test('列表表格字号在竖屏下不小于 12.5 像素', async ({ page }) => {
    await gotoRoute(page, '/print-targets');
    const table = page.locator('.main-content .el-table').first();
    await expect(table).toBeVisible();

    const fontSize = await table.evaluate(el =>
      Number.parseFloat(window.getComputedStyle(el).fontSize)
    );
    expect(fontSize, `表格字号应为移动端档位 13 像素，实测 ${fontSize}`).toBeGreaterThanOrEqual(
      12.5
    );
    expect(fontSize, `表格字号不应退回桌面档位 14 像素，实测 ${fontSize}`).toBeLessThanOrEqual(
      13.5
    );
  });
});

test.describe('重复提交与重复入口', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('连点两次提交只发出一条建单请求', async ({ page }) => {
    await gotoRoute(page, '/print-targets');

    const createdPayloads: Array<Record<string, unknown>> = [];
    let postCount = 0;
    await page.route('**/api/v1/erp/print-targets', async route => {
      if (route.request().method() !== 'POST') {
        await route.continue();
        return;
      }
      postCount += 1;
      const raw = route.request().postData();
      if (raw) createdPayloads.push(JSON.parse(raw) as Record<string, unknown>);
      // 延迟放行，让提交锁在请求在途期间真实生效，连点的第二下必须被挡在锁外。
      await new Promise(resolve => setTimeout(resolve, 800));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ code: 200, message: 'ok', data: { id: 987654321 } }),
      });
    });

    const dialog = await openCreateDialog(page);
    await dialog.getByPlaceholder('请输入目标编码').fill('E2E-DEDUP-01');
    await dialog.getByPlaceholder('请输入显示名称').fill('e2e 防重');
    await dialog.getByPlaceholder('请输入 CUPS 队列名称').fill('e2e-queue');

    const confirm = dialog.locator('.el-dialog__footer .el-button--primary');
    // 同一个同步任务里连点两下，此时重渲染还没把按钮置灰，第二下会真实进入事件处理器，
    // 只有提交锁在异步校验之前落下才能挡住它。
    await confirm.evaluate((el: HTMLElement) => {
      el.click();
      el.click();
    });

    await expect(confirm, '提交在途时按钮应进入加载态').toHaveClass(/is-loading/, {
      timeout: 3000,
    });
    await expect(dialog, '提交成功后弹窗应关闭').toBeHidden({ timeout: 10000 });

    expect(postCount, '连点两次只允许发出一张建单请求').toBe(1);
    expect(createdPayloads).toHaveLength(1);
    expect(createdPayloads[0].target_code).toBe('E2E-DEDUP-01');
  });

  test('旧会计期间路径重定向且侧栏仅一条入口', async ({ page }) => {
    await page.goto(`${BASE_URL}/periods`, { waitUntil: 'domcontentloaded' });

    await expect(page, '旧路径应重定向到会计期间管理页').toHaveURL(/\/accounting-period$/, {
      timeout: 15000,
    });

    const periodEntries = page.locator('.aside .el-menu-item').filter({ hasText: '会计期间' });
    await expect(periodEntries, '会计期间只允许存在一个菜单入口').toHaveCount(1);
  });
});
