import { test, expect, type Page } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  trackPageHealth,
  assertPageHealthy,
  BROWSER_NETWORK_NOISE,
  tryCleanup,
} from './helpers';
import { safeGoto } from './ui-helpers';

// 覆盖前端改动：供应商 360 页头由要求手填供应商数字主键改为按名称与编码向服务端远程搜索的选择器。
// 用 UI 真实操作锁定三件事：页头不再出现数字主键输入项；关键字真的触发服务端搜索并把候选带回；
// 换一个关键字后候选按当前输入独立持有（不是本地过滤、也不是上一批累积）。
// 选定后点查询，派生信息区以只读文本回显编码、名称与由选择带出的主键。
// 造数走真实后端，回读按唯一名称命中后端落库的主键与自动取号的编码，不信任创建响应体形状。

const supplierPath = '/purchase/suppliers';
const enhancedPath = '/supplier-enhanced';

// 页头选择器的可见名称（取自 locales/zh-CN.ts supplier.enhanced.picker.placeholder，逐字符一致）
const PLACEHOLDER_PICKER = '输入供应商名称或编码搜索';
// 派生信息区三项标签与查询前提示（同 locales supplier.enhanced 段）
const LABEL_ID = '供应商 ID';
const LABEL_CODE = '供应商编码';
const LABEL_NAME = '供应商名称';
const TEXT_QUERY = '查询';
const TOAST_SELECT_FIRST = '请先选择供应商';

type SupplierRow = {
  id?: number;
  supplier_name?: string;
  supplier_code?: string;
};

async function createUniqueSupplier(
  page: Page,
  tag: string
): Promise<{ id: number; code: string; name: string }> {
  const uniq = `${Date.now()}${tag}${Math.floor(Math.random() * 1000)}`;
  const name = `E2E360${tag}供应商${uniq}`;
  const creditCode = `91${uniq.slice(0, 14)}`.replace(/\D/g, '0').padEnd(18, '0').slice(0, 18);
  await apiCall(page, 'POST', supplierPath, {
    supplier_name: name,
    supplier_short_name: `E2E360${tag}`,
    contact_phone: '13800000002',
    credit_code: creditCode,
  });
  const list = await apiCall<{ items?: SupplierRow[] }>(
    page,
    'GET',
    `${supplierPath}?keyword=${encodeURIComponent(name)}&page=1&page_size=50`
  );
  const row = (list.data?.items ?? []).find(r => r.supplier_name === name);
  expect(row, `建供应商后应能按唯一名称回读命中：${name}`).toBeTruthy();
  expect(row!.id, '回读供应商行应含主键 id').toBeTruthy();
  return { id: row!.id!, code: row!.supplier_code ?? '', name };
}

/**
 * 页头选择器外壳。Element Plus 2.14 的 el-select 把占位文案渲染成 `.el-select__placeholder`
 * 独立文本节点，不再写到内部 input 的 placeholder 属性上，
 * 因此 getByPlaceholder 恒取不到（CI 上就是这个原因判红），改按页头操作区里的下拉定位，
 * 并用占位文案节点确认它确实是供应商选择器而不是别的下拉。
 */
function pickerShell(page: Page) {
  return page.locator('.header-actions .el-select').first();
}

/** 选择器的可输入节点：filterable 与 remote 都挂在它身上 */
function pickerInput(page: Page) {
  return pickerShell(page).locator('input').first();
}

/** 占位文案锁：写到 input 属性或以独立文本节点呈现都算，两种 Element Plus 形态都能过 */
async function assertPickerPlaceholder(page: Page): Promise<void> {
  const shell = pickerShell(page);
  const viaAttr = await shell.locator(`input[placeholder="${PLACEHOLDER_PICKER}"]`).count();
  const viaNode = await shell
    .locator('.el-select__placeholder', { hasText: PLACEHOLDER_PICKER })
    .count();
  expect(
    viaAttr + viaNode,
    `页头下拉应显示供应商搜索占位文案 ${PLACEHOLDER_PICKER}`
  ).toBeGreaterThan(0);
}

async function openAndSearch(page: Page, keyword: string): Promise<void> {
  const input = pickerInput(page);
  await input.click();
  // fill 会整体替换输入内容，配合 remote 属性即真实触发一次服务端搜索，而不是在前端筛
  await input.fill(keyword);
}

async function visibleDropdown(page: Page) {
  const dropdown = page.locator('.el-select-dropdown:visible').last();
  await expect(dropdown, '关键字搜索后候选面板应可见').toBeVisible();
  return dropdown;
}

test.describe('供应商 360 名称选择器端到端', () => {
  let supplierA: { id: number; code: string; name: string } | undefined;
  let supplierB: { id: number; code: string; name: string } | undefined;

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test.afterEach(async ({ page }) => {
    if (supplierA) {
      await tryCleanup(page, 'DELETE', `${supplierPath}/${supplierA.id}`, '清理供应商A');
      supplierA = undefined;
    }
    if (supplierB) {
      await tryCleanup(page, 'DELETE', `${supplierPath}/${supplierB.id}`, '清理供应商B');
      supplierB = undefined;
    }
  });

  test('页头不再有手填数字主键的输入项，按名称搜索可命中并由选择带出主键', async ({ page }) => {
    const collector = trackPageHealth(page);
    supplierA = await createUniqueSupplier(page, '甲');

    await safeGoto(page, enhancedPath);
    await page.waitForTimeout(500);
    await assertPageHealthy(page, collector, {
      consoleNoisePatterns: BROWSER_NETWORK_NOISE,
      label: '供应商 360 页',
    });

    // 1) 页头不应再出现数字主键输入项（评分页签里的打分输入框不属页头，不参与该判据）
    await expect(
      page.locator('.header-actions .el-input-number'),
      '页头不应再有要求手填供应商数字主键的输入项'
    ).toHaveCount(0);
    await expect(pickerInput(page), '页头应存在按名称搜索的选择器').toBeVisible();
    await assertPickerPlaceholder(page);

    // 2) 输入唯一名称的部分字符，断言候选真的由服务端返回
    const partial = supplierA.name.slice(4);
    await openAndSearch(page, partial);
    const dropdown = await visibleDropdown(page);
    const matched = dropdown.locator('.el-select-dropdown__item').filter({ hasText: partial });
    await expect(matched.first(), `下拉应出现含部分名称「${partial}」的供应商选项`).toBeVisible({
      timeout: 15_000,
    });

    // 3) 点选后按名称与编码回显（编码由主数据出参带出，不是前端拼的）
    await matched.first().click();
    const expectedLabel = supplierA.code
      ? `${supplierA.name}（${supplierA.code}）`
      : supplierA.name;
    await expect(
      page.locator('.header-actions .el-select').first(),
      '选中项应回显为名称加编码的选项文案'
    ).toContainText(expectedLabel);

    // 4) 点查询，派生信息区只读回显编码、名称与由选择带出的主键
    await page.getByRole('button', { name: TEXT_QUERY, exact: true }).click();
    const descriptions = page.locator('.el-descriptions').first();
    await expect(descriptions, '查询后应出现供应商派生信息区').toBeVisible({ timeout: 15_000 });
    await expect(descriptions).toContainText(LABEL_ID);
    await expect(descriptions).toContainText(LABEL_CODE);
    await expect(descriptions).toContainText(LABEL_NAME);
    await expect(descriptions).toContainText(supplierA.name);
    if (supplierA.code) {
      await expect(descriptions).toContainText(supplierA.code);
    }
    await expect(descriptions).toContainText(String(supplierA.id));

    // 5) 派生字段是文本呈现，不存在可编辑控件绑定它们
    await expect(
      descriptions.locator('input, .el-input, .el-select, .el-input-number'),
      '派生信息区不应有可编辑控件'
    ).toHaveCount(0);
  });

  test('换一个关键字后候选按当前输入独立持有，不残留上一批也不做本地过滤', async ({ page }) => {
    const collector = trackPageHealth(page);
    supplierA = await createUniqueSupplier(page, '甲');
    supplierB = await createUniqueSupplier(page, '乙');

    await safeGoto(page, enhancedPath);
    await page.waitForTimeout(500);
    await assertPageHealthy(page, collector, {
      consoleNoisePatterns: BROWSER_NETWORK_NOISE,
      label: '供应商 360 页换关键字',
    });

    const partialA = supplierA.name.slice(6);
    await openAndSearch(page, partialA);
    const dropdownA = await visibleDropdown(page);
    await expect(
      dropdownA.locator('.el-select-dropdown__item').filter({ hasText: partialA }).first()
    ).toBeVisible({ timeout: 15_000 });

    // 换成 B 的关键字：整体替换输入，服务端按新关键字返回，旧关键字的候选不得残留
    const partialB = supplierB.name.slice(6);
    await openAndSearch(page, partialB);
    const dropdownB = await visibleDropdown(page);
    await expect(
      dropdownB.locator('.el-select-dropdown__item').filter({ hasText: partialB }).first(),
      `换关键字后应命中「${partialB}」`
    ).toBeVisible({ timeout: 15_000 });
    await expect(
      dropdownB.locator('.el-select-dropdown__item').filter({ hasText: partialA }),
      '换关键字后上一批候选不得残留，残留即候选不是按当前输入独立持有'
    ).toHaveCount(0);
  });

  test('未选择供应商就点查询给出选择提示，而不是要求手填主键', async ({ page }) => {
    const collector = trackPageHealth(page);

    await safeGoto(page, enhancedPath);
    await page.waitForTimeout(500);
    await assertPageHealthy(page, collector, {
      consoleNoisePatterns: BROWSER_NETWORK_NOISE,
      label: '供应商 360 页空选择',
    });

    await page.getByRole('button', { name: TEXT_QUERY, exact: true }).click();
    await expect(
      page.getByText(TOAST_SELECT_FIRST, { exact: true }),
      '未选择供应商应提示先选择，而不是提示输入主键'
    ).toBeVisible({ timeout: 10_000 });
    await expect(
      page.getByText('请输入供应商 ID', { exact: true }),
      '要求手填供应商 ID 的旧提示不得回流'
    ).toHaveCount(0);
  });
});
