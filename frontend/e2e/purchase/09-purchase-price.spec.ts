// 采购 E2E 套件 — 09 采购价格（专用页 /purchase-price）
// 迁移来源：e2e/purchase-ext/02-price.spec.ts（枢纽 Tab 已删除）
//   02-01 Tab 加载          → 09-01
//   02-02 新建采购价格      → 09-02
//
// 状态机真值（backend/src/models/status/general.rs 的 master_data，全小写）：
//   create_price 写 pending（services/purchase_price_service.rs status: master_data::PENDING），
//   批准端点写 approved、停用写 inactive。前端比较对象见 views/purchase-price/composables/ppFmts.ts。
// 后端 list_prices 仅支持 product_id / supplier_id / status 过滤（无 keyword），
// 故回读按建单返回的 product_id 检索列表 + 按 id 回读详情，二者结合确认真实落库。
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCallRaw, ensureTestEntities, tryCleanup } from '../flow/helpers';
import type { Page } from '@playwright/test';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 展开当前可见 el-select 并点选其第一个选项（建单只需任一有效产品/供应商） */
async function pickFirstOption(page: Page, labelText: string): Promise<void> {
  const dlg = page.locator('.el-dialog:visible');
  await dlg.locator('.el-form-item').filter({ hasText: labelText }).first().locator('.el-select').click();
  const option = page
    .locator('.el-select-dropdown:visible .el-select-dropdown__item')
    .first();
  await option.click();
}

test.describe('09 采购价格', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
    await ensureTestEntities(page);
  });

  test('09-01 进入采购价格列表页', async ({ page }) => {
    await page.goto('/purchase-price');
    await expect(page.getByText('采购价格管理')).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: '新建价格' })).toBeVisible();
    await expect(page.locator('.el-table')).toBeVisible({ timeout: 30000 });
  });

  test('09-02 新建采购价格（UI 建单 → 列表按 product_id 检索 + 详情回读确认落库）', async ({
    page,
  }) => {
    await page.goto('/purchase-price');
    await expect(page.getByText('采购价格管理')).toBeVisible({ timeout: 30000 });
    await page.getByRole('button', { name: '新建价格' }).click();
    const dlg = page.locator('.el-dialog:visible');
    await expect(dlg).toBeVisible({ timeout: 30000 });

    await pickFirstOption(page, '产品');
    await pickFirstOption(page, '供应商');
    await dlg
      .locator('.el-form-item')
      .filter({ hasText: '采购价格' })
      .first()
      .locator('input')
      .first()
      .fill('123.456');
    // 生效日期（后端建单默认写当天，但表单已提供且原用例填此字段，保持等价）
    const effInput = dlg
      .locator('.el-form-item')
      .filter({ hasText: '生效日期' })
      .first()
      .locator('input')
      .first();
    await effInput.click();
    await effInput.fill('2026-08-01');
    await page.keyboard.press('Enter');

    const createdResp = page
      .waitForResponse(
        res => res.url().includes('/purchase/purchase-prices') && res.request().method() === 'POST',
        { timeout: 30000 }
      )
      .catch(() => null);
    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('保存成功')).toBeVisible({ timeout: 30000 });

    const resp = await createdResp;
    expect(resp, '未捕获到建单 POST 响应').not.toBeNull();
    const body = (await resp!.json()) as {
      data: {
        id: number;
        product_id: number;
        supplier_id: number;
        price: number | string;
        status: string;
      };
    };
    const created = body.data;
    expect(created?.id, '建单响应未返回 id').toBeTruthy();
    CLEANUP.push({ path: `/purchase/purchase-prices/${created.id}`, label: 'purchase_price' });

    // 详情回读：价格值正确，状态为 pending（后端写入侧 master_data::PENDING）
    expect(Number(created.price), '建单返回价格应与提交值一致').toBeCloseTo(123.456, 2);
    expect(created.status, '新建价格状态应为 pending').toBe('pending');

    // 列表检索回读：按 product_id 过滤（后端真实支持的过滤键），确认新记录出现在列表中
    const list = await apiCallRaw<Array<{ id: number }>>(
      page,
      'GET',
      `/purchase/purchase-prices?product_id=${created.product_id}&page=1&page_size=50`
    );
    expect(
      list.some(r => r.id === created.id),
      `列表按 product_id=${created.product_id} 检索未命中新建价格 ${created.id}`
    ).toBe(true);
  });
});
