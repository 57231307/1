// MRP 计算 E2E 测试
// 创建时间: 2026-08-19
// 覆盖范围：MRP 计算执行 → 结果查看 → 建议采购 → BOM 多级展开净需求数值正确性
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, apiCallRaw, ensureTestEntities, getCtx, tryCleanup } from '../flow/helpers';
import { pickSelectIn, formItemByExactLabel } from '../flow/ui-helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

test.describe('MRP 计算', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('进入 MRP 计算页面', async ({ page }) => {
    await page.goto('/mrp');
    await expect(page.getByRole('heading', { name: /MRP/ })).toBeVisible({ timeout: 30000 });
  });

  test('MRP 计算可执行', async ({ page }) => {
    // 真实造数：补前置实体，确保 MRP 产品下拉有可参与运算的产品。
    // 原实现 `if (await calcBtn.isVisible())` + 断言 `计算完成|计算中`：
    // 表单校验未过时根本不发请求，文案也永不匹配；可见性分支一旦不成立即零断言假绿。
    await ensureTestEntities(page);

    const products = await apiCallRaw<unknown>(page, 'GET', '/production/mrp/products?keyword=E2E');
    if (!Array.isArray(products)) {
      throw new Error(
        `/production/mrp/products 响应 data 不是数组，实际：${JSON.stringify(products).slice(0, 200)}`
      );
    }
    expect(products.length, '造数后 MRP 产品下拉仍为空，属数据/端点缺陷').toBeGreaterThan(0);

    await page.goto('/mrp');

    // 硬断言（Tier A）：MRP 页必然提供"开始计算"按钮
    const calcBtn = page.getByRole('button', { name: /开始计算/ });
    await expect(calcBtn, 'MRP 页面应渲染"开始计算"按钮').toBeVisible({ timeout: 30000 });

    // 需求日期为 el-date-picker：先于产品多选填写。产品是 multiple remote el-select，
    // 其 teleport 到 body 的下拉面板会覆盖同表单下一行的日期输入框；若在选完产品后再点
    // date input，Playwright click 的 hit-target 校验被面板拦截 → 30s 超时（本用例红根因）。
    // 先填日期（此时无面板遮挡）再驱动产品多选，交互顺序与页面真实操作一致。
    const calcForm = page.getByLabel('MRP 计算参数表单');
    const demandDate = formItemByExactLabel(calcForm, '需求日期').locator('input').first();
    await demandDate.click();
    await demandDate.fill('2026-09-30');
    await page.keyboard.press('Enter');

    // 产品为 remote el-select（filterable remote，:remote-method=searchProducts）：旧写法
    // getByRole('combobox').click() 命中只读内层 input，被 placeholder「请输入产品名称搜索」拦
    // pointer events → click 超时；即便点开，pressSequentially 也未点外层 wrapper 稳定触发 remote。
    // 改用冻结 helper：以计算参数表单容器（aria-label=MRP 计算参数表单）为 root + 精确 label
    // 「产品选择」锚定，点外层 .el-select__wrapper → keyboard.type('E2E') 触发远程搜索 → 选首项。
    await pickSelectIn(calcForm, page, '产品选择', { query: 'E2E', index: 0, multiple: true });

    // 点计算前先收起产品多选面板：filterable remote 多选的 el-select 面板 teleport 到 body，
    // 选完项后面板在 CI 下仍可能展开，其层叠区域盖住同表单下一行的「开始计算」按钮 →
    // Playwright hit-target 校验失败 → calcBtn.click() 30s 超时（按钮已 resolved 却点不下）。
    // pickSelectIn(multiple) 内虽已按一次 Escape，但不足以在慢环境下稳定收起远程搜索面板；此处
    // 补一次 Escape 并等待 teleport 下拉面板确不可见（toHaveCount 自动重试，非放宽断言），
    // 交互顺序与真实用户「选完产品→收起下拉→点开始计算」一致。
    await page.keyboard.press('Escape');
    await expect(page.locator('.el-select-dropdown:visible')).toHaveCount(0);

    await calcBtn.click();
    await expect(page.getByText(/计算成功/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByText(/物料需求列表/)).toBeVisible({ timeout: 30000 });
  });

  test('MRP 历史页面可正常加载', async ({ page }) => {
    await page.goto('/mrp/history');
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });

  test('BOM 多级展开 → MRP 净需求数值正确性', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    // 取至少 3 个产品：父产品 + 2 个子物料
    expect(
      ctx.productIds.length,
      'MRP BOM 展开测试需要至少 3 个产品（1 父 + 2 子）'
    ).toBeGreaterThanOrEqual(3);
    const parentProductId = ctx.productIds[0];
    const childMaterialId1 = ctx.productIds[1];
    const childMaterialId2 = ctx.productIds[2];

    // 创建 BOM：父产品含 2 个物料，数量/损耗率明确可推算
    // material1: quantity=3, scrap_rate=10 → 有效用量 = 3 * (1 + 10/100) = 3.3
    // material2: quantity=5, scrap_rate=0  → 有效用量 = 5
    // 出参泛型如实按后端响应定型：quantity/scrap_rate 是 rust_decimal Decimal，
    // 序列化为 JSON 字符串（"3.0000"/"10.0000"），非 number（Cargo.toml:60 未启 serde-float）。
    // 入参体同理以字符串提交，rust_decimal 反序列化接受 number/string 两形态。
    const bomResult = await apiCall<{
      bom?: { id: number; product_id: number };
      items?: Array<{ material_id: number; quantity: string; scrap_rate: string | null }>;
    }>(page, 'POST', '/boms', {
      product_id: parentProductId,
      version: 1,
      is_default: true,
      items: [
        { material_id: childMaterialId1, quantity: '3.0', unit: '千克', scrap_rate: '10.0' },
        { material_id: childMaterialId2, quantity: '5.0', unit: '米', scrap_rate: '0' },
      ],
    });
    const bomId = bomResult.data?.bom?.id;
    expect(bomId, `BOM 创建应返回 id: ${JSON.stringify(bomResult).slice(0, 200)}`).toBeTruthy();
    CLEANUP.push({ path: `/boms/${bomId}`, label: 'bom_mrptest' });

    // 触发 MRP 计算：父产品需求 100 单位
    const mrpResult = await apiCallRaw<{
      calculation_no: string;
      requirements: Array<{
        product_id: number;
        required_quantity: string | number;
        bom_level: number;
        shortage_quantity: string | number;
        on_hand_quantity: string | number;
      }>;
    }>(page, 'POST', '/production/mrp/calculate', {
      items: [
        {
          product_id: parentProductId,
          required_quantity: '100',
          required_date: '2026-12-31',
        },
      ],
      consider_safety_stock: false,
      consider_in_transit: false,
    });
    expect(mrpResult.requirements, 'MRP 计算应返回 requirements 数组').toBeDefined();
    expect(
      mrpResult.requirements.length,
      'BOM 展开后 requirements 至少应包含 2 行子物料需求'
    ).toBeGreaterThanOrEqual(2);

    // 找子物料需求行（bom_level >= 1）
    const reqChild1 = mrpResult.requirements.find(
      r => r.product_id === childMaterialId1 && r.bom_level >= 1
    );
    const reqChild2 = mrpResult.requirements.find(
      r => r.product_id === childMaterialId2 && r.bom_level >= 1
    );
    expect(
      reqChild1,
      `requirements 应包含 childMaterial1(id=${childMaterialId1}) 的需求行`
    ).toBeDefined();
    expect(
      reqChild2,
      `requirements 应包含 childMaterial2(id=${childMaterialId2}) 的需求行`
    ).toBeDefined();

    // 数值断言：
    // child1 净需求 = 100 * 3 * (1 + 10/100) = 330
    // child2 净需求 = 100 * 5 = 500
    const q1 = Number(reqChild1!.required_quantity);
    const q2 = Number(reqChild2!.required_quantity);
    expect(q1, `child1 需求应为 100*3*1.1=330（含10%损耗），实际: ${q1}`).toBeCloseTo(330, 1);
    expect(q2, `child2 需求应为 100*5=500，实际: ${q2}`).toBeCloseTo(500, 1);

    // bom_level 应为 1（一级 BOM 展开）
    expect(reqChild1!.bom_level, 'child1 的 bom_level 应为 1').toBe(1);
    expect(reqChild2!.bom_level, 'child2 的 bom_level 应为 1').toBe(1);
  });
});
