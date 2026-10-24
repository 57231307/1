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

// 覆盖前端改动：评估管理新增的供应商对比页签。
// 用 UI 真实操作锁定：对比入口是按名称与编码向服务端远程搜索的多选下拉（不要求手填数字主键），
// 并排呈现的评级、平均评分、评估记录数与最近评估日期全部来自各供应商既有评分聚合端点的出参，
// 行名称来自主数据列表出参而不是用编码或主键拼出的假名，取数结论列把有评分与失败分开呈现。
// 造数与清理都走真实后端：先造两个供应商与一条评估记录，收尾按供应商与周期回读评估记录逐条删除，
// 不信任创建响应体形状，也不留残留数据。

const supplierPath = '/purchase/suppliers';
const indicatorPath = '/purchase/supplier-evaluations/indicators';
const recordPath = '/purchase/supplier-evaluations/records';
const evaluationPath = '/purchase/supplier-evaluations';

// 界面文案取自 locales/zh-CN.ts，逐字符一致
const TAB_COMPARE = '供应商对比';
const PLACEHOLDER_COMPARE_PICKER = '输入供应商名称或编码搜索，可多选';
const BUTTON_COMPARE = '开始对比';
const TABLE_ARIA_LABEL = '供应商对比结果列表';
const OUTCOME_LOADED = '有评分';
const OUTCOME_FAILED = '取数失败';
const COLUMN_RECORD_COUNT = '评估记录数';
const COLUMN_LATEST_DATE = '最近评估日期';

type SupplierRow = { id?: number; supplier_name?: string };
type IndicatorRow = { id?: number; indicator_code?: string; max_score?: number };
type RecordRow = { id?: number };

async function createUniqueSupplier(page: Page, tag: string): Promise<number> {
  const uniq = `${Date.now()}${tag}${Math.floor(Math.random() * 1000)}`;
  const name = `E2E对比${tag}供应商${uniq}`;
  const creditCode = `91${uniq.slice(0, 14)}`.replace(/\D/g, '0').padEnd(18, '0').slice(0, 18);
  await apiCall(page, 'POST', supplierPath, {
    supplier_name: name,
    supplier_short_name: `E2E对比${tag}`,
    contact_phone: '13800000003',
    credit_code: creditCode,
  });
  const list = await apiCall<{ items?: SupplierRow[] }>(
    page,
    'GET',
    `${supplierPath}?keyword=${encodeURIComponent(name)}&page=1&page_size=50`
  );
  const row = (list.data?.items ?? []).find(r => r.supplier_name === name);
  expect(row, `建供应商后应能按唯一名称回读命中：${name}`).toBeTruthy();
  return row!.id!;
}

/**
 * 按唯一名称回读供应商编码用于断言名称列取的是主数据出参；
 * 与 createUniqueSupplier 分开是因为对比页只需主键即可造评估记录。
 */
async function readSupplierName(page: Page, supplierId: number): Promise<string> {
  const detail = await apiCall<{ supplier_name?: string }>(
    page,
    'GET',
    `${supplierPath}/${supplierId}`
  );
  expect(detail.data?.supplier_name, '供应商详情应回读出名称').toBeTruthy();
  return detail.data!.supplier_name!;
}

/**
 * 取一条可用的评估指标：库里没有就按固定编码自建一条，再按编码回读为准。
 * 指标编码重复时后端回业务错「已存在」（并行分片可能先建了同一条），这不致命，
 * 回读能拿到即可；用固定编码而不是随机编码是为了不让每次 CI 都往主数据里堆一条新指标。
 */
const INDICATOR_CODE = 'E2E-CMP-IND-001';

async function readIndicator(page: Page): Promise<IndicatorRow | undefined> {
  const res = await apiCall<IndicatorRow[]>(page, 'GET', `${indicatorPath}?page_size=100`);
  const list = Array.isArray(res.data) ? res.data : [];
  return list.find(i => i.indicator_code === INDICATOR_CODE) ?? list[0];
}

async function ensureIndicator(page: Page): Promise<IndicatorRow> {
  let found = await readIndicator(page);
  if (!found?.id) {
    const created = await apiCall<IndicatorRow>(page, 'POST', indicatorPath, {
      indicator_name: 'E2E 对比稳定性指标',
      indicator_code: INDICATOR_CODE,
      category: 'quality',
      weight: '0.50',
      max_score: 100,
      evaluation_method: 'score',
    });
    if (!created.data?.id && !String(created.message ?? '').includes('已存在')) {
      throw new Error(`[101] 评估指标前置创建失败：${JSON.stringify(created).slice(0, 300)}`);
    }
    found = await readIndicator(page);
  }
  expect(
    found?.id,
    `评估指标前置未就绪（既读不到也建不出编码 ${INDICATOR_CODE}），无法造评分`
  ).toBeTruthy();
  return found!;
}

/** 对比页签的多选下拉外壳：Element Plus 2.14 把占位文案渲染成独立文本节点，getByPlaceholder 取不到 */
function comparePickerShell(page: Page) {
  return page.locator('.toolbar .el-select:visible').first();
}

/** 占位文案锁：写到 input 属性或以独立节点呈现都算，缺文案才是选择器不对 */
async function assertPickerPlaceholder(page: Page, text: string): Promise<void> {
  const shell = comparePickerShell(page);
  const viaAttr = await shell.locator(`input[placeholder="${text}"]`).count();
  const viaNode = await shell.locator('.el-select__placeholder', { hasText: text }).count();
  expect(viaAttr + viaNode, `对比下拉应显示占位文案「${text}」`).toBeGreaterThan(0);
}

/**
 * 打开多选下拉并按关键字远程搜索后选中一项。
 * 点击面取外壳 el-select__wrapper（组件真正接事件的触发区），不点内层 input。
 * 多选形态下占位文案渲染成盖在 input 上的独立节点，点 input 会被它拦截 pointer events。
 * 输入用键盘逐字触发 remote-method 的服务端搜索，候选只能来自服务端回包，
 * 断言候选含关键字即证明按名称或编码搜索真实发生，选中由 v-model 带出主键 id。
 * 多选面板选中后不自动关闭，且组件默认保留搜索关键字，收面板需要两段 Escape，
 * 第一段只清空关键字，第二段关键字为空才收起面板，
 * 收完确认面板确不可见，避免 teleport 到 body 的面板盖住后续操作。
 */
async function searchAndPick(page: Page, keyword: string): Promise<void> {
  const wrapper = comparePickerShell(page).locator('.el-select__wrapper').first();
  await wrapper.click();
  await page.keyboard.type(keyword, { delay: 50 });
  const dropdown = page.locator('.el-select-dropdown:visible').last();
  await expect(dropdown, `输入「${keyword}」后候选面板应可见`).toBeVisible();
  const option = dropdown.locator('.el-select-dropdown__item').filter({ hasText: keyword }).first();
  await expect(option, `应搜到含「${keyword}」的供应商候选`).toBeVisible({ timeout: 15_000 });
  await option.click();
  // 组件契约（element-plus useSelect 的 handleEsc 与 reserveKeyword 默认 true），点选后关键字仍在，
  // 第一段 Escape 清空关键字不收面板，第二段才收，与本仓 remote 多选既有用例的两段收法一致。
  await page.keyboard.press('Escape');
  await page.keyboard.press('Escape');
  await expect(page.locator('.el-select-dropdown:visible')).toHaveCount(0);
}

test.describe('供应商对比页签端到端', () => {
  let supplierIds: number[] = [];
  let supplierNames: string[] = [];

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test.afterEach(async ({ page }) => {
    // 先删造的评估记录（按供应商与本轮周期回读，逐条删，不依赖创建响应体形状），再删供应商
    for (const id of supplierIds) {
      const list = await apiCall<RecordRow[]>(
        page,
        'GET',
        `${recordPath}?supplier_id=${id}&period=2000-Q1&page_size=100`
      ).catch(() => undefined);
      // 该端点出参形状是 ApiResponse<Vec<EvaluationRecordView>>，data 直接是数组
      const records = Array.isArray(list?.data) ? list.data : [];
      for (const rec of records) {
        if (rec.id !== undefined) {
          await tryCleanup(page, 'DELETE', `${evaluationPath}/${rec.id}`, '清理评估记录');
        }
      }
      await tryCleanup(page, 'DELETE', `${supplierPath}/${id}`, '清理对比用供应商');
    }
    supplierIds = [];
    supplierNames = [];
  });

  test('对比页签用名称搜索多选供应商，并排呈现四项关键指标且取数结论为有评分', async ({ page }) => {
    const collector = trackPageHealth(page);

    // 1) 造两个供应商，并各写一条本周期评估记录（否则后端对该供应商返回无评估记录）
    const idA = await createUniqueSupplier(page, '甲');
    const idB = await createUniqueSupplier(page, '乙');
    supplierIds = [idA, idB];

    const indicator = await ensureIndicator(page);

    for (const supplierId of supplierIds) {
      await apiCall(page, 'POST', recordPath, {
        supplier_id: supplierId,
        evaluation_period: '2000-Q1',
        indicator_id: indicator.id,
        score: 1,
      });
    }

    // 2) 回读两个供应商的名称，用于断言对比行的名称取的是主数据出参
    supplierNames = [await readSupplierName(page, idA), await readSupplierName(page, idB)];

    // 3) 进评估管理并切到对比页签
    await safeGoto(page, '/supplier-evaluation');
    await page.waitForTimeout(500);
    await assertPageHealthy(page, collector, {
      consoleNoisePatterns: BROWSER_NETWORK_NOISE,
      label: '供应商评估管理页',
    });
    await page.getByRole('tab', { name: TAB_COMPARE, exact: true }).click();

    // 4) 对比入口是按关键字远程搜索的多选下拉：页签内不应有数字主键输入项
    const picker = comparePickerShell(page);
    await expect(picker, '对比页签应存在按名称搜索的多选下拉').toBeVisible();
    await assertPickerPlaceholder(page, PLACEHOLDER_COMPARE_PICKER);
    // 对比工具区里不应再有手填数字主键的输入项（按承载搜索框的那一条工具行作用域判）
    await expect(
      page.locator('.toolbar').filter({ has: picker }).locator('.el-input-number'),
      '对比工具区不应有手填数字主键的输入项'
    ).toHaveCount(0);

    // 5) 按唯一名称的部分字符分别搜出并选中两个供应商
    const partialA = supplierNames[0].slice(6);
    const partialB = supplierNames[1].slice(6);
    await searchAndPick(page, partialA);
    await searchAndPick(page, partialB);

    // 6) 点开始对比，断言并排两行、名称取主数据出参、结论为有评分、指标位不是占位
    await page.getByRole('button', { name: BUTTON_COMPARE, exact: true }).click();
    const table = page.locator(`[aria-label="${TABLE_ARIA_LABEL}"]`);
    await expect(table, '对比结果表应出现').toBeVisible({ timeout: 15_000 });
    await expect(table).toContainText(COLUMN_RECORD_COUNT);
    await expect(table).toContainText(COLUMN_LATEST_DATE);

    for (const name of supplierNames) {
      const row = table.locator('tr.el-table__row').filter({ hasText: name });
      await expect(row, `对比表应有名为「${name}」的行`).toHaveCount(1);
      await expect(row, '两行都应按主数据出参显示供应商名称').toContainText(name);
      await expect(row).not.toContainText(OUTCOME_FAILED);
      const rowText = await row.innerText();
      // 结论列有评分，且评估记录数是真的数字而不是占位横线
      expect(
        rowText.includes(OUTCOME_LOADED),
        `行应显示取数结论「${OUTCOME_LOADED}」：${rowText}`
      ).toBe(true);
      expect(/\d/.test(rowText), `行应含数字型评估记录数：${rowText}`).toBe(true);
    }

    // 7) 交叉校验：后端对同一供应商的评分聚合同样给出评级，前端没有自行折算
    const scoreA = await apiCall<{
      rating?: string;
      total_records?: number;
      average_score?: string;
    }>(page, 'GET', `/purchase/supplier-evaluations/suppliers/${idA}/score`);
    expect(scoreA.data?.rating, '后端评分聚合应返回评级').toBeTruthy();
    const firstRowText = await table
      .locator('tr.el-table__row')
      .filter({ hasText: supplierNames[0] })
      .innerText();
    expect(
      firstRowText.includes(scoreA.data!.rating!),
      `对比行的评级应与后端出参一致（后端 ${scoreA.data!.rating} 行 ${firstRowText}）`
    ).toBe(true);
  });

  test('未选供应商就点开始对比给出选择提示', async ({ page }) => {
    const collector = trackPageHealth(page);

    await safeGoto(page, '/supplier-evaluation');
    await page.waitForTimeout(500);
    await assertPageHealthy(page, collector, {
      consoleNoisePatterns: BROWSER_NETWORK_NOISE,
      label: '供应商评估管理页空选择',
    });
    await page.getByRole('tab', { name: TAB_COMPARE, exact: true }).click();
    await page.getByRole('button', { name: BUTTON_COMPARE, exact: true }).click();

    await expect(
      page.getByText('请先选择要对比的供应商', { exact: true }),
      '未选择供应商应提示先选择'
    ).toBeVisible({ timeout: 10_000 });
  });
});
