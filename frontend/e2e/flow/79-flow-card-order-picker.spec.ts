import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCallRaw,
  genCode,
  BASE_URL,
  tryCleanup,
  trackPageHealth,
  assertPageHealthy,
} from './helpers';
import {
  openDialog,
  submitDialog,
  pickSelectIn,
  getFieldValue,
  formItemByExactLabel,
  waitForToast,
} from './ui-helpers';

// 锁定新建流转卡对话框把裸的生产订单 ID 输入换成可搜索下拉后的三条行为契约。
// 一，打开对话框后经生产订单列表端点加载候选，输入单号部分字符可筛出目标订单。
// 二，选中该订单后自动带出并回填产品名称与产品 ID 两个只读派生字段。
// 三，提交后新卡确实挂在该生产订单上，派生字段随单落库。
// 另附一条负向，未选择生产订单直接提交应被必填校验拦截且不产生新卡。
// 造数全部经 API 真实建产品与建生产订单，不依赖种子行、不硬编 id，读值走列表富化路径。

interface CreatedProduct {
  id: number;
  name: string;
}
interface CreatedOrder {
  id: number;
  order_no: string;
}

const CLEANUP: Array<{ path: string; label: string }> = [];

test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) {
    await tryCleanup(page, 'DELETE', c.path, c.label);
  }
  CLEANUP.length = 0;
});

test.describe('流转卡新建，生产订单可搜索下拉', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // 经 API 造一个可用产品与一条挂在该产品上的生产订单，返回两者主数据。
  // 产品名用带随机后缀的唯一串，便于在下拉选项与派生字段回读中做严格相等断言。
  const seedProductAndOrder = async (page: import('@playwright/test').Page): Promise<{
    product: CreatedProduct;
    order: CreatedOrder;
  }> => {
    const cats = await apiCallRaw<{ items?: Array<{ id?: number }> }>(
      page,
      'GET',
      '/product-categories?page=1&page_size=5'
    );
    const categoryId = Number(cats?.items?.[0]?.id);
    expect(categoryId, '须有已种入的产品分类作建产品必填来源，为空即暴露缺种子判红').toBeGreaterThan(
      0
    );

    const productName = genCode('E2EFCPROD');
    const product = await apiCallRaw<{ id?: number }>(page, 'POST', '/products', {
      name: productName,
      category_id: categoryId,
      unit: '米',
    });
    const productId = Number(product?.id);
    expect(productId, `建产品应返回主键，响应 ${JSON.stringify(product).slice(0, 160)}`).toBeGreaterThan(
      0
    );
    CLEANUP.push({ path: `/products/${productId}`, label: '[flow79] 产品' });

    // 建单载荷对齐后端 CreateProductionOrderPayload，product_id 与 planned_quantity 必填，
    // 单号由服务端取号，前端不提交 order_no 键。
    const order = await apiCallRaw<{ id?: number; order_no?: string }>(
      page,
      'POST',
      '/production/production-orders/orders',
      {
        product_id: productId,
        planned_quantity: 100,
        priority: 1,
      }
    );
    const orderId = Number(order?.id);
    const orderNo = order?.order_no ?? '';
    expect(orderId, `建生产订单应返回主键，响应 ${JSON.stringify(order).slice(0, 160)}`).toBeGreaterThan(
      0
    );
    expect(orderNo.length, `生产订单服务端应回填单号，实际 ${JSON.stringify(order).slice(0, 160)}`).toBeGreaterThanOrEqual(
      2
    );
    CLEANUP.push({
      path: `/production/production-orders/orders/${orderId}`,
      label: '[flow79] 生产订单',
    });

    return { product: { id: productId, name: productName }, order: { id: orderId, order_no: orderNo } };
  };

  test('输入单号筛出订单，选中自动带出产品并挂单落库', async ({ page }) => {
    const { product, order } = await seedProductAndOrder(page);

    const collector = trackPageHealth(page);
    await page.goto(`${BASE_URL}/flow-cards`, { waitUntil: 'domcontentloaded' });
    await page
      .locator('.el-table, .el-empty, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    await expect(
      page.getByRole('button', { name: '新建流转卡' }).first(),
      '流转卡页应渲染新建按钮，健康页判据之一'
    ).toBeVisible();
    await assertPageHealthy(page, collector);

    const dialog = await openDialog(page, '新建流转卡');

    // 在生产订单下拉输入该订单号的部分字符，触发 filter 生效，并点选含唯一产品名的选项。
    // query 取订单号尾若干字符以证部分匹配命中，optionText 取唯一产品名以稳锚目标行。
    const partialOrderNo = order.order_no.slice(Math.max(0, order.order_no.length - 4));
    await pickSelectIn(dialog, page, '生产订单 ID', {
      query: partialOrderNo,
      optionText: product.name,
      timeout: 20_000,
    });

    // 选中后断言两个只读派生字段被自动回填，值须与该产品真实数据严格相等。
    const filledName = await getFieldValue(dialog, '产品名称');
    expect(filledName, '产品名称派生字段应等于所选订单的产品名').toBe(product.name);
    const filledId = await getFieldValue(dialog, '产品 ID');
    expect(
      filledId,
      `产品 ID 派生字段应等于该产品主键 ${product.id}，实际 ${filledId}`
    ).toBe(String(product.id));

    await submitDialog(dialog, /确定/);

    const toast = await waitForToast(page, 10_000);
    expect(toast, `提交后应出现成功提示，实际 toast ${JSON.stringify(toast)}`).toContain('创建成功');
    await dialog.waitFor({ state: 'hidden', timeout: 10_000 });

    // 后端复核，新流转卡确实挂在所建生产订单上且派生字段随单持久。
    const list = await apiCallRaw<{ items?: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/production/flow-cards?production_order_id=${order.id}&page=1&page_size=20`
    );
    const cards = list?.items ?? [];
    const created = cards.find(c => Number(c.production_order_id) === order.id);
    expect(created, `后端应查得挂在订单 ${order.id} 上的新卡`).toBeTruthy();
    CLEANUP.push({
      path: `/production/flow-cards/${Number(created?.id)}`,
      label: '[flow79] 流转卡',
    });
    expect(
      String(created?.product_name ?? ''),
      '落库流转卡的产品名应与所选一致'
    ).toBe(product.name);
    expect(
      String(created?.card_no ?? ''),
      '落库流转卡应有服务端生成的卡号'
    ).toBeTruthy();
  });

  test('未选择生产订单直接提交被必填校验拦截且不建单', async ({ page }) => {
    // 造一条可用订单，让下拉确有候选，以证拦截来自必填而非空候选。
    const { order } = await seedProductAndOrder(page);

    await page.goto(`${BASE_URL}/flow-cards`, { waitUntil: 'domcontentloaded' });
    await page
      .locator('.el-table, .el-empty, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    const dialog = await openDialog(page, '新建流转卡');

    // 未选择订单时派生字段应为空，与产品名自动带出行为互证。
    const nameBeforeSelect = await getFieldValue(dialog, '产品名称');
    expect(nameBeforeSelect, '未选择订单时产品名称派生字段应为空').toBe('');

    await submitDialog(dialog, /确定/);

    const errLocator = formItemByExactLabel(dialog, '生产订单 ID').locator('.el-form-item__error');
    await errLocator.waitFor({ state: 'visible', timeout: 5_000 });
    const errText = (await errLocator.textContent()) ?? '';
    expect(errText, '空订单提交应触发生产订单必填校验文案').toContain('请选择生产订单');

    // 拦截后对话框不应关闭，且不应有任何成功 toast。
    await expect(dialog).toBeVisible();
    await expect(page.locator('.el-message--success')).toHaveCount(0);

    // 后端复核，该订单名下未因本次被拦截的提交产生任何新卡。
    const list = await apiCallRaw<{ items?: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/production/flow-cards?production_order_id=${order.id}&page=1&page_size=20`
    );
    expect(
      (list?.items ?? []).length,
      '被必填拦截的提交不应在后端落任何卡'
    ).toBe(0);
  });
});
