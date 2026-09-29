// 库存管理 E2E 套件 — 02 库存调整（盘盈/盘亏）
// 覆盖范围：库存调整对话框（increase/decrease）、表单填写、提交；
//          调整单 item 级 IDOR 回归钉（非 owner 经 item_id 直访 403，owner 本人 2xx + 回读）
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  ensureTestEntities,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  seedFourDimStockIn,
  getCtx,
  getRoleCredential,
  loginInIsolatedContext,
  tryCleanup,
} from '../flow/helpers';
import { fillFieldByLabel, formItemByExactLabel, pickSelectIn } from '../flow/ui-helpers';

test.describe('库存管理 - 02 库存调整', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  // 调整类型 radio 的可点区是外层 <label class="el-radio">：EP 把真 <input type=radio>
  // 隐藏（.el-radio__original），getByRole('radio') 命中的是该隐藏 input，被 .el-radio__inner
  // 拦 pointer events → click 超时（既往假红根因）。改点其可见文本（label 内 span），
  // 事件冒泡到 label 触发选中。
  const pickAdjustType = async (dialog: import('@playwright/test').Locator, label: string) => {
    await dialog.getByText(label, { exact: true }).click();
  };

  // 后端库存调整以「既有库存行 stock_id」为调整对象（inventory_adjustment_service
  // ::create_adjustment_items 按 stock_id 读取在库量作为调整前量），工具栏入口只选「产品+仓库」，
  // 提交前须保证该组合确有库存行，否则调整对象不存在、后端拒绝。
  // 这里以对话框两个下拉实际渲染的口径（GET /warehouses、/products，page_size=1000）读取首项
  // ——与对话框 fetchWarehouses/fetchProducts 完全相同的请求，index 0 命中同一行——再为其造
  // 一行带全四维的库存，确保「选中的首行」确有库存可调整。属补齐真实前置，不弱化成功断言。
  const seedAdjustmentTarget = async (page: Page): Promise<void> => {
    await ensureTestEntities(page);
    const wh = await apiCallRaw<{ items: { id: number }[] }>(
      page,
      'GET',
      '/warehouses?page=1&page_size=1000'
    );
    const pr = await apiCallRaw<{ items: { id: number }[] }>(
      page,
      'GET',
      '/products?page=1&page_size=1000'
    );
    const warehouseId = wh.items?.[0]?.id;
    const productId = pr.items?.[0]?.id;
    expect(warehouseId, '前置：库存调整需至少一个仓库（对话框首项）').toBeTruthy();
    expect(productId, '前置：库存调整需至少一个产品（对话框首项）').toBeTruthy();
    const tag = Date.now().toString().slice(-6);
    await seedFourDimStockIn(page, {
      productId: productId!,
      warehouseId: warehouseId!,
      colorNo: `E2E-ADJ-C${tag}`,
      dyeLotNo: `E2E-ADJ-D${tag}`,
      batchNo: `E2E-ADJ-B${tag}`,
      quantityMeters: '5000',
    });
  };

  test('库存调整 - 盘盈', async ({ page }) => {
    await seedAdjustmentTarget(page);
    await page.goto('/inventory');
    await page.getByRole('button', { name: '库存调整' }).click();
    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 30000 });
    // 缺陷A 修复后：工具栏入口渲染可选 el-select（仓库/产品）
    await expect(formItemByExactLabel(dialog, '仓库')).toBeVisible({ timeout: 10000 });
    await pickSelectIn(dialog, page, '仓库', { index: 0 });
    await pickSelectIn(dialog, page, '产品', { index: 0 });
    // 调整类型是 radio（AdjustmentDialog.vue），标签为「增加」/「减少」（i18n typeIncrease/typeDecrease）
    await expect(dialog.getByText('增加', { exact: true })).toBeVisible();
    await pickAdjustType(dialog, '增加');
    // el-input-number fill 后按 Tab 失焦提交 v-model，否则 modelValue 不更新 → 提交被本地校验拦下
    await fillFieldByLabel(dialog, page, '调整数量', '50');
    await page.keyboard.press('Tab');
    // 调整原因为 el-input type=textarea（真 <textarea>，非 <input>），fillFieldByLabel 只命中
    // input，故按精确 label 锚定其 textarea 元素填写。
    await formItemByExactLabel(dialog, '调整原因')
      .locator('textarea')
      .first()
      .fill('E2E 测试盘盈调整');
    await dialog.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('库存调整成功')).toBeVisible({ timeout: 30000 });
  });

  test('库存调整 - 盘亏', async ({ page }) => {
    await seedAdjustmentTarget(page);
    await page.goto('/inventory');
    await page.getByRole('button', { name: '库存调整' }).click();
    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 30000 });
    // 缺陷A 修复后：工具栏入口渲染可选 el-select（仓库/产品）
    await expect(formItemByExactLabel(dialog, '仓库')).toBeVisible({ timeout: 10000 });
    await pickSelectIn(dialog, page, '仓库', { index: 0 });
    await pickSelectIn(dialog, page, '产品', { index: 0 });
    await expect(dialog.getByText('减少', { exact: true })).toBeVisible();
    await pickAdjustType(dialog, '减少');
    await fillFieldByLabel(dialog, page, '调整数量', '30');
    await page.keyboard.press('Tab');
    // 调整原因同为 el-input type=textarea
    await formItemByExactLabel(dialog, '调整原因')
      .locator('textarea')
      .first()
      .fill('E2E 测试盘亏调整');
    await dialog.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('库存调整成功')).toBeVisible({ timeout: 30000 });
  });

  // ============================================================
  // 调整单 item 级 IDOR 回归钉（越权防护：handlers/inventory_adjustment_handler.rs
  // update_item/delete_item/list_items 先反查父调整单再走 get_adjustment(Some(&data_scope_ctx))，
  // 归属校验 utils/data_scope.rs::check_resource_owner 拒 → AppError::permission_denied → HTTP 403
  // code=FORBIDDEN，message 透传「无权访问调整单 {id}（数据范围限制）」）。
  //
  // 第二用户（B）如何取得——不新建共享 helper，复用既有 global-setup 账号体系：
  //   getRoleCredential('inventory_manager') 读 role-credentials.json 中由
  //   ensureRoleUsers 生成的 e2e_inventory_manager（与默认 owner 账号 e2e_admin(_sN) 不同用户），
  //   再用既有 loginInIsolatedContext 真实登录出独立 cookie 会话（参照 flow/34 范式）。
  // 选 inventory_manager 而非任意角色是刻意的：该角色在后端 init 权限种子中带
  //   ("adjustments","*")（init_service_ops/permission.rs:433），能通过 RBAC 中间件、
  //   请求真实到达 handler 内的数据范围归属校验——这样 403 才钉的是本波修复的
  //   item 级 IDOR 守卫本身，而非"根本没权限碰这个资源"的 RBAC 拦截（那是另一种 403，
  //   message=「权限不足，无法访问该资源」，middleware/permission.rs:130）。
  //   故断言精确钉：status===403 且 message 含「数据范围限制」——若哪天 403 变成
  //   RBAC 来源（角色权限漂移）或用例路径写错，本断言会红并给出真实 message，不放宽。
  //   inventory_manager 角色 data_scope=dept（init role.rs:168-171），非本人创建的
  //   调整行必然过不了 check_resource_owner（资源无 department 列，dept 对 None 拒绝）。
  // ============================================================
  test('调整单 item 级 IDOR：非 owner 经 item_id 直访 GET/PUT/DELETE 精确 403，owner 2xx+回读', async ({
    page,
    browser,
  }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    expect(productId, '前置：需至少一个产品').toBeTruthy();
    expect(warehouseId, '前置：需至少一个仓库').toBeTruthy();

    // 真实 seed 四维库存行（调整对象 = 既有库存行 stock_id，口径同文件头注释）
    const tag = Date.now().toString().slice(-6);
    const stockRow = await seedFourDimStockIn(page, {
      productId: productId!,
      warehouseId: warehouseId!,
      colorNo: `E2E-IDOR-C${tag}`,
      dyeLotNo: `E2E-IDOR-D${tag}`,
      batchNo: `E2E-IDOR-B${tag}`,
      quantityMeters: '500',
    });
    const stockId = Number(stockRow.id);
    expect(stockId, '前置：seed 库存行应返回数字 id（调整明细外键）').toBeGreaterThan(0);

    // owner（默认登录账号，调整单 created_by = 本人 user_id）建 pending 调整单 + 1 明细
    const adj = await apiCall<{ id?: number; status?: string; items?: { id: number }[] }>(
      page,
      'POST',
      '/inventory/adjustments',
      {
        warehouse_id: warehouseId,
        adjustment_date: new Date().toISOString(),
        adjustment_type: 'increase',
        reason_type: '盘盈',
        reason_description: `E2E item 级 IDOR 回归钉 tag=${tag}`,
        notes: 'E2E IDOR pin',
        items: [{ stock_id: stockId, quantity: '10' }],
      }
    );
    const adjId = adj.data?.id;
    const itemId = adj.data?.items?.[0]?.id;
    expect(
      adjId,
      `前置：调整单建单应返回 data.id，实际：${JSON.stringify(adj).slice(0, 200)}`
    ).toBeTruthy();
    expect(
      itemId,
      `前置：建单响应应回显明细 items[0].id（后续经 item_id 直访），实际：${JSON.stringify(adj).slice(0, 200)}`
    ).toBeTruthy();
    console.warn(`[调整单IDOR] owner 建单成功 adjId=${adjId} itemId=${itemId} stockId=${stockId}`);

    // owner 基线：GET 明细（父单归属校验通过）应 2xx 且命中本用例明细
    const ownerItems = await apiCallRaw<unknown>(
      page,
      'GET',
      `/inventory/adjustments/${adjId}/items`
    );
    expect(
      Array.isArray(ownerItems),
      `owner GET items 应返回数组，实际：${JSON.stringify(ownerItems).slice(0, 200)}`
    ).toBe(true);
    const ownerItem = (ownerItems as Array<Record<string, unknown>>).find(
      it => Number(it.id) === itemId
    );
    expect(ownerItem, 'owner 回读：明细应存在').toBeTruthy();
    expect(Number(ownerItem!.quantity), 'owner 回读：初始明细数量应为 10').toBeCloseTo(10, 2);

    // 第二用户 B：global-setup 既有角色账号，独立 context 真实登录（凭证缺失=setup 缺陷判红，不 skip）
    const credB = getRoleCredential('inventory_manager');
    if (!credB) {
      throw new Error(
        'inventory_manager 角色凭证不存在——global-setup ensureRoleUsers 未正确执行，属 setup 缺陷判红（禁止 skip 掩盖越权面）'
      );
    }
    const sessionB = await loginInIsolatedContext(browser, credB.username, credB.password);
    try {
      // B 经父单 id 读他人调整单明细 → 403 且钉"数据范围限制"来源（handler list_items 的父单归属校验）
      const bGet = await apiCallExpectFail(
        sessionB.page,
        'GET',
        `/inventory/adjustments/${adjId}/items`
      );
      console.warn(
        `[调整单IDOR] B GET items: status=${bGet.status} code=${bGet.code} message=${bGet.message}`
      );
      expect(bGet.status, `B(${credB.username}) GET 他人调整单 ${adjId} items 应精确 403`).toBe(
        403
      );
      expect(
        String(bGet.message ?? ''),
        `B GET items 的 403 必须来自 handler 内数据范围归属校验（证明 RBAC 已过、IDOR 守卫生效），而非 RBAC 拦截；实际 message=${bGet.message}`
      ).toContain('数据范围限制');

      // B 经 item_id 直改（PUT /adjustments/items/{item_id}）→ 403（本波 8eac10bb 修复点：
      // 先 get_adjustment_id_by_item 反查父单再校验归属）
      const bPut = await apiCallExpectFail(
        sessionB.page,
        'PUT',
        `/inventory/adjustments/items/${itemId}`,
        { stock_id: stockId, quantity: '999' }
      );
      console.warn(
        `[调整单IDOR] B PUT item: status=${bPut.status} code=${bPut.code} message=${bPut.message}`
      );
      expect(bPut.status, `B PUT 他人调整明细 ${itemId} 应精确 403`).toBe(403);
      expect(
        String(bPut.message ?? ''),
        `B PUT item 的 403 应来自反查父单后的数据范围归属校验；实际 message=${bPut.message}`
      ).toContain('数据范围限制');

      // B 经 item_id 直删（DELETE /adjustments/items/{item_id}）→ 403
      const bDel = await apiCallExpectFail(
        sessionB.page,
        'DELETE',
        `/inventory/adjustments/items/${itemId}`
      );
      console.warn(
        `[调整单IDOR] B DELETE item: status=${bDel.status} code=${bDel.code} message=${bDel.message}`
      );
      expect(bDel.status, `B DELETE 他人调整明细 ${itemId} 应精确 403`).toBe(403);
      expect(
        String(bDel.message ?? ''),
        `B DELETE item 的 403 应来自反查父单后的数据范围归属校验；实际 message=${bDel.message}`
      ).toContain('数据范围限制');

      // B 经 item_id 篡改父单直访（GET /adjustments/{id} 详情）同样 403——覆盖"绕过 items 子路由"变体
      const bGetAdj = await apiCallExpectFail(
        sessionB.page,
        'GET',
        `/inventory/adjustments/${adjId}`
      );
      expect(bGetAdj.status, `B GET 他人调整单详情应精确 403`).toBe(403);
      expect(String(bGetAdj.message ?? ''), 'B GET 详情的 403 应同源数据范围校验').toContain(
        '数据范围限制'
      );
    } finally {
      await sessionB.close();
    }

    // 越权失败写后回读：明细必须原封不动（quantity 仍 10、仅 1 行）——攻击未改变资源
    const afterAttack = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      `/inventory/adjustments/${adjId}/items`
    );
    expect(afterAttack.length, '三次越权写被拒后明细行不应增减').toBe(1);
    expect(
      Number(afterAttack[0].quantity),
      `三次越权写被拒后明细数量应仍为 10（实际 ${afterAttack[0].quantity}）`
    ).toBeCloseTo(10, 2);

    // owner 正向对照（同路径同方法 2xx，证明上述 403 由身份而非路径/载荷决定）：
    // PUT 修改明细 10→12，回读钉新值
    await apiCall(page, 'PUT', `/inventory/adjustments/items/${itemId}`, {
      stock_id: stockId,
      quantity: '12',
    });
    const afterOwnerPut = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      `/inventory/adjustments/${adjId}/items`
    );
    const ownerUpdated = afterOwnerPut.find(it => Number(it.id) === itemId);
    expect(ownerUpdated, 'owner PUT 后回读：明细仍存在').toBeTruthy();
    expect(
      Number(ownerUpdated!.quantity),
      `owner 修改后明细数量应回读为 12（实际 ${ownerUpdated!.quantity}）`
    ).toBeCloseTo(12, 2);

    // owner DELETE 明细，回读命中 0 行
    await apiCall(page, 'DELETE', `/inventory/adjustments/items/${itemId}`);
    const afterOwnerDel = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      `/inventory/adjustments/${adjId}/items`
    );
    expect(afterOwnerDel.length, 'owner DELETE 后该明细应消失').toBe(0);

    // 清理本用例调整单（pending 可删；失败不静默，tryCleanup 自带 warn 日志）
    await tryCleanup(page, 'DELETE', `/inventory/adjustments/${adjId}`, 'inventory_adjustment');
  });
});
