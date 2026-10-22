import { test, expect, type Locator, type Page } from '../diagnose-fixture';
import { loginViaUI, BASE_URL, API_PREFIX, apiCallRaw, genCode, tryCleanup } from './helpers';

/**
 * 移动端内容层适配与重复提交防重
 *
 * 功能，补两类桌面视口或只测正向流程测不出的缺陷。
 * 一 内容层适配，移动端弹窗若沿用 640 像素固定宽会超出手机屏，表单标签仍横排会把输入框压到
 * 无法输入，mobile-content.css 负责在窄屏下把弹窗收进视口、标签置顶、输入框占满内容宽。
 * 二 重复提交，提交锁必须在异步校验之前落下，否则连点两下会并发建出两条记录。
 * 三 重复入口，旧会计期间页已由新页取代，旧路径只保留重定向，侧栏不得再出现第二条入口。
 * 四 明细行与面板，采购新建弹窗的明细行在竖屏必须逐项堆叠，数据权限页的角色栏必须改为上下
 *   堆叠，销售统计卡片与审计统计卡都必须在竖屏改为两列，这些此前都用固定列宽把内容压出屏幕
 *   或压到不可读。
 *
 * 调用方，frontend e2e flow 分片。入参，页面与登录态由 fixture 提供。
 * 传给谁，断言结果只进 Playwright 报告。
 * 存什么，防重两条用例各向真实接口建一行数据（打印目标编码与客户名都带时间戳加随机位，跑完按
 *   回读到的主键删除）。路由拦截只延迟放行并记录请求体，响应全部来自真实后端，
 *   断言落在接口回读与页面真实渲染上。
 */

const MOBILE_PORTRAIT = { width: 360, height: 780 };

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

interface CreatePostProbe {
  /** 路由拦截器每放行一条建单 POST 调用一次（登记发出数与并发在途峰值） */
  recordIssue(): void;
  /** 浏览器真实发出的建单 POST 总条数（含 CSRF 竞败后的串行补发） */
  issued(): number;
  /** 同时 in-flight 的建单 POST 峰值：提交锁生效时恒为 1 */
  maxInflight(): number;
  /** 已收到响应的建单 POST 状态码（全部来自真实后端） */
  statuses(): number[];
}

/**
 * 建单请求探针：只读地统计浏览器真实发出的建单 POST 数、并发在途峰值与响应状态码。
 *
 * 为什么要分开发/收两面计数：CSRF 令牌一次性消费，前端拦截器在同一条逻辑提交上
 * 会串行补发一次（首次 403 → 换新令牌重放 → 200），只数"发出条数"会把这种补发
 * 误判成连点重复；而并发峰值与"被后端接受的条数"才真正对应被测的提交锁。
 * 入参：page 与建单端点 pathname。传给谁：本文件两条防重用例。存什么：只在内存计数，不落库。
 */
function createCreatePostProbe(page: Page, pathname: string): CreatePostProbe {
  let issued = 0;
  let inflight = 0;
  let peak = 0;
  const codes: number[] = [];
  page.on('response', res => {
    if (res.request().method() !== 'POST') return;
    if (new URL(res.url()).pathname !== pathname) return;
    codes.push(res.status());
    inflight = Math.max(0, inflight - 1);
  });
  return {
    recordIssue(): void {
      issued += 1;
      inflight += 1;
      peak = Math.max(peak, inflight);
    },
    issued: () => issued,
    maxInflight: () => peak,
    statuses: () => [...codes],
  };
}

/**
 * 把探针收到的状态码拆两档：被后端真实接受（2xx，已进 handler 并可能落库）与
 * CSRF 竞败（403，中间件即回，一次性令牌未进 handler、不会落库）。
 */
function splitPostStatuses(codes: number[]): { accepted: number; csrfRejected: number } {
  return {
    accepted: codes.filter(status => status >= 200 && status < 300).length,
    csrfRejected: codes.filter(status => status === 403).length,
  };
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

    await expect(dialog.locator('.items-header'), '明细表头应存在且只有一个').toHaveCount(1);
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

  test('审计统计卡在竖屏按两列排布', async ({ page }) => {
    await gotoRoute(page, '/omni-audit');
    const grid = page.locator('.stats-grid');
    await expect(grid).toBeVisible();

    const columns = await grid.evaluate(el => {
      const template = window.getComputedStyle(el).gridTemplateColumns;
      return template.split(/\s+/).filter(part => part.length > 0).length;
    });
    expect(columns, `四张审计统计卡应排成两列，实测 ${columns} 列`).toBe(2);

    const first = await grid.locator('.stat-card').nth(0).boundingBox();
    const third = await grid.locator('.stat-card').nth(2).boundingBox();
    if (!first || !third) throw new Error('审计统计卡未渲染出可测量区域');
    expect(third.y, '第三张卡片应换到第二行').toBeGreaterThanOrEqual(first.y + first.height - 2);
  });
});

test.describe('重复提交与重复入口', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('连点两次提交只发出一条建单请求', async ({ page }) => {
    await gotoRoute(page, '/print-targets');

    // 编码带时间戳与随机位：真实回读按编码计数，才能证明"只建了一条"而不是把历史残留算进来。
    const targetCode = genCode('E2E-DEDUP');
    let createdId = 0;
    try {
      const createdPayloads: Array<Record<string, unknown>> = [];
      const probe = createCreatePostProbe(page, `${API_PREFIX}/print-targets`);
      // 拦截只做两件事：登记并发出请求体 + 延迟放行到真实后端，响应一律由后端给出。
      // 延迟让提交锁在请求在途期间真实生效，连点的第二下必须被挡在锁外。
      await page.route('**/api/v1/erp/print-targets', async route => {
        if (route.request().method() !== 'POST') {
          await route.continue();
          return;
        }
        probe.recordIssue();
        const raw = route.request().postData();
        if (raw) createdPayloads.push(JSON.parse(raw) as Record<string, unknown>);
        await new Promise(resolve => setTimeout(resolve, 800));
        await route.continue();
      });

      const dialog = await openCreateDialog(page);
      await dialog.getByPlaceholder('请输入目标编码').fill(targetCode);
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

      // 弹窗关闭即建单请求已拿到响应；再等一拍把 response 事件计入探针，避免与监听器竞态。
      await expect
        .poll(() => probe.statuses().length, {
          timeout: 5000,
          message: '建单请求没有拿到任何后端响应',
        })
        .toBeGreaterThanOrEqual(1);
      const settled = probe.statuses();
      const { accepted, csrfRejected } = splitPostStatuses(settled);
      expect(
        probe.issued() - csrfRejected,
        `连点两次只允许发出一张建单请求（CSRF 竞败的串行补发除外），实际状态码 ${settled.join(',')}`
      ).toBe(1);
      expect(probe.maxInflight(), '两条建单请求不得并发在途——提交锁必须在异步校验之前落下').toBe(1);
      expect(accepted, `真实后端只应接受一条建单请求，实际状态码 ${settled.join(',')}`).toBe(1);
      expect(createdPayloads.length, '每条放行的建单请求都应抓到请求体').toBe(probe.issued());
      for (const payload of createdPayloads) {
        expect(payload.target_code, '建单请求体应携带本用例的编码').toBe(targetCode);
      }

      // 真实接口回读：后端落库的恰是一行，且各列就是表单提交值——成功不是拦截器喂出来的。
      const list = await apiCallRaw<Array<Record<string, unknown>>>(page, 'GET', '/print-targets');
      const rows = list.filter(row => row.target_code === targetCode);
      expect(rows, `接口应读回恰一行 ${targetCode}，实得 ${rows.length} 行`).toHaveLength(1);
      createdId = Number(rows[0].id);
      expect(createdId, `回读行应带真实主键：${JSON.stringify(rows[0])}`).toBeGreaterThan(0);
      expect(rows[0].display_name, '落库显示名应等于提交值').toBe('e2e 防重');
      expect(rows[0].kind, '未改连接种类时缺省为 cups').toBe('cups');
      expect(rows[0].queue_name, '落库队列名应等于提交值').toBe('e2e-queue');

      // 真实页面渲染：建单成功后列表刷新，表格出现且仅出现该编码一行。
      await expect(
        page.locator('.el-table__row:visible').filter({ hasText: targetCode }),
        '打印目标列表应渲染出真实落库的那一行'
      ).toHaveCount(1);
    } finally {
      if (createdId) await tryCleanup(page, 'DELETE', `/print-targets/${createdId}`, '打印目标');
    }
  });

  test('客户新建连点两次只发出一条建档请求', async ({ page }) => {
    await gotoRoute(page, '/customer');

    // 客户名带时间戳与随机位：真实回读用 keyword（后端 customer_name LIKE 过滤）计数，
    // 只命中这一条建档才算证明。
    const customerName = genCode('e2e客户防重');
    let createdId = 0;
    try {
      const createdPayloads: Array<Record<string, unknown>> = [];
      const probe = createCreatePostProbe(page, `${API_PREFIX}/crm/customers`);
      // 拦截只做两件事：登记并发出请求体 + 延迟放行到真实后端，响应一律由后端给出。
      // 延迟让提交锁在请求在途期间真实生效，连点的第二下必须被挡在锁外。
      await page.route('**/api/v1/erp/crm/customers', async route => {
        if (route.request().method() !== 'POST') {
          await route.continue();
          return;
        }
        probe.recordIssue();
        const raw = route.request().postData();
        if (raw) createdPayloads.push(JSON.parse(raw) as Record<string, unknown>);
        await new Promise(resolve => setTimeout(resolve, 800));
        await route.continue();
      });

      await page.getByRole('button', { name: '新建客户' }).click();
      const dialog = page.locator('.el-dialog:visible').first();
      await expect(dialog).toBeVisible();
      await dialog.getByPlaceholder('请输入客户名称').fill(customerName);
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

      // 弹窗关闭即建档请求已拿到响应；再等一拍把 response 事件计入探针，避免与监听器竞态。
      await expect
        .poll(() => probe.statuses().length, {
          timeout: 5000,
          message: '建档请求没有拿到任何后端响应',
        })
        .toBeGreaterThanOrEqual(1);
      const settled = probe.statuses();
      const { accepted, csrfRejected } = splitPostStatuses(settled);
      expect(
        probe.issued() - csrfRejected,
        `连点两次只允许发出一张建档请求（CSRF 竞败的串行补发除外），实际状态码 ${settled.join(',')}`
      ).toBe(1);
      expect(probe.maxInflight(), '两条建档请求不得并发在途——提交锁必须在异步校验之前落下').toBe(1);
      expect(accepted, `真实后端只应接受一条建档请求，实际状态码 ${settled.join(',')}`).toBe(1);
      expect(createdPayloads.length, '每条放行的建档请求都应抓到请求体').toBe(probe.issued());
      for (const payload of createdPayloads) {
        expect(payload.customer_name, '建档请求体应携带本用例的客户名').toBe(customerName);
      }

      // 真实接口回读：后端只落一条同名客户，且列值就是提交值——成功不是拦截器喂出来的。
      const paged = await apiCallRaw<{ items: Array<Record<string, unknown>>; total: number }>(
        page,
        'GET',
        `/crm/customers?keyword=${encodeURIComponent(customerName)}&page=1&page_size=20`
      );
      expect(
        paged.items,
        `接口应读回恰一条 ${customerName}，实得 ${paged.items.length} 条`
      ).toHaveLength(1);
      createdId = Number(paged.items[0].id);
      expect(createdId, `回读行应带真实主键：${JSON.stringify(paged.items[0])}`).toBeGreaterThan(0);
      expect(paged.items[0].customer_name).toBe(customerName);
      expect(paged.items[0].contact_person, '落库联系人应等于提交值').toBe('测试联系人');

      // 真实页面渲染：建档成功后列表刷新（按 created_at 倒序），表格出现且仅出现该客户一行。
      await expect(
        page.locator('.el-table__row:visible').filter({ hasText: customerName }),
        '客户列表应渲染出真实建档的那一行'
      ).toHaveCount(1);
    } finally {
      if (createdId) await tryCleanup(page, 'DELETE', `/crm/customers/${createdId}`, '客户');
    }
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
