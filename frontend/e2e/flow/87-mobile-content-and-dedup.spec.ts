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
 * 四 明细行与面板，采购新建弹窗的明细行在竖屏必须逐项堆叠，数据权限页的角色栏必须改为上下
 *   堆叠，销售统计卡片必须两列排布，这三处此前都用固定列宽把内容压出屏幕或压到不可读。
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

  test('采购新建弹窗的明细行在竖屏逐项堆叠且不横向溢出', async ({ page }) => {
    await gotoRoute(page, '/purchase');
    await page.getByRole('button', { name: '新建采购单' }).click();
    const dialog = page.locator('.el-dialog').first();
    await expect(dialog).toBeVisible();

    const body = dialog.locator('.el-dialog__body');
    await expect(body).toBeVisible();
    const bodyBox = await body.boundingBox();
    if (!bodyBox) throw new Error('采购弹窗未渲染出可测量内容区');

    const bodyOverflow = await body.evaluate(el => el.scrollWidth - el.clientWidth);
    expect(bodyOverflow, `采购弹窗内容区横向溢出 ${bodyOverflow} 像素`).toBeLessThanOrEqual(2);

    await expect(dialog.locator('.items-header'), '竖屏应隐藏固定列宽的明细表头').toBeHidden();

    const rows = dialog.locator('.items-row');
    const rowCount = await rows.count();
    expect(rowCount, '采购明细应至少渲染一行').toBeGreaterThan(0);
    for (let index = 0; index < rowCount; index += 1) {
      const row = rows.nth(index);
      const rowOverflow = await row.evaluate(el => el.scrollWidth - el.clientWidth);
      expect(
        rowOverflow,
        `第 ${index + 1} 行采购明细横向溢出 ${rowOverflow} 像素`
      ).toBeLessThanOrEqual(2);

      const children = await row.evaluate(el =>
        Array.from(el.children).map(child => {
          const rect = child.getBoundingClientRect();
          return { left: rect.left, right: rect.right };
        })
      );
      expect(children.length, `第 ${index + 1} 行明细控件不应为空`).toBeGreaterThan(0);
      for (const child of children) {
        expect(child.left, `第 ${index + 1} 行明细控件左缘越出内容区`).toBeGreaterThanOrEqual(
          bodyBox.x - 1
        );
        expect(child.right, `第 ${index + 1} 行明细控件右缘越出内容区`).toBeLessThanOrEqual(
          bodyBox.x + bodyBox.width + 1
        );
      }
    }
  });

  test('数据权限页角色栏与权限面板在竖屏上下堆叠', async ({ page }) => {
    await gotoRoute(page, '/data-permission');
    const layout = page.locator('.data-permission .layout');
    await expect(layout).toBeVisible();

    const direction = await layout.evaluate(el => window.getComputedStyle(el).flexDirection);
    expect(direction, '角色栏与权限面板应改为纵向堆叠，而非固定 240 像素并排').toBe('column');

    const panel = page.locator('.data-permission .role-panel');
    const panelBox = await panel.boundingBox();
    const layoutBox = await layout.boundingBox();
    if (!panelBox || !layoutBox) throw new Error('数据权限布局未渲染出可测量区域');
    expect(panelBox.width, '角色栏应展开为整行宽').toBeGreaterThanOrEqual(layoutBox.width - 4);
    expect(panelBox.x + panelBox.width, '角色栏右缘不应越出布局区').toBeLessThanOrEqual(
      layoutBox.x + layoutBox.width + 1
    );
  });

  test('销售统计卡片在竖屏按两列排布而非四列挤压', async ({ page }) => {
    await gotoRoute(page, '/sales');
    const cols = page.locator('.stats-cards .el-col');
    await expect(cols.first()).toBeVisible();

    const count = await cols.count();
    expect(count, '销售订单统计应有四张卡片').toBeGreaterThanOrEqual(4);
    const first = await cols.nth(0).boundingBox();
    const second = await cols.nth(1).boundingBox();
    const third = await cols.nth(2).boundingBox();
    if (!first || !second || !third) throw new Error('销售统计卡片未渲染出可测量区域');

    expect(Math.abs(first.y - second.y), '前两张卡片应并排在同一行').toBeLessThanOrEqual(2);
    expect(third.y, '第三张卡片应换到下一行').toBeGreaterThanOrEqual(first.y + first.height - 2);
    expect(first.width, '每张统计卡宽度应接近半屏，不得挤成四列').toBeGreaterThanOrEqual(140);
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

  test('客户新建连点两次只发出一条建档请求', async ({ page }) => {
    await gotoRoute(page, '/customer');

    let postCount = 0;
    const createdPayloads: Array<Record<string, unknown>> = [];
    await page.route('**/api/v1/erp/crm/customers', async route => {
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
        body: JSON.stringify({
          code: 200,
          message: 'ok',
          data: { id: 987650001, customer_name: 'e2e 客户防重' },
        }),
      });
    });

    await page.getByRole('button', { name: '新建客户' }).click();
    const dialog = page.locator('.el-dialog:visible').first();
    await expect(dialog).toBeVisible();
    await dialog.getByPlaceholder('请输入客户名称').fill('e2e 客户防重');
    await dialog.getByPlaceholder('请输入联系人').fill('测试联系人');
    await dialog.getByPlaceholder('请输入联系电话').fill('13800138000');

    const confirm = dialog.locator('.el-dialog__footer .el-button--primary');
    await confirm.evaluate((el: HTMLElement) => {
      el.click();
      el.click();
    });

    await expect(confirm, '提交在途时按钮应进入加载态').toHaveClass(/is-loading/, {
      timeout: 3000,
    });
    await expect(dialog, '提交成功后弹窗应关闭').toBeHidden({ timeout: 10000 });

    expect(postCount, '连点两次只允许发出一张建档请求').toBe(1);
    expect(createdPayloads).toHaveLength(1);
    expect(createdPayloads[0].customer_name).toBe('e2e 客户防重');
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
