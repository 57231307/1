import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  expectStateGateRejection,
  genCode,
  tryCleanup,
} from './helpers';
// 产品类型单源常量（与后端 product_type 词表同源）：建品夹具取值必须经成员引用。
import { PRODUCT_TYPE } from '../../src/constants/product-type';

/**
 * 99 采购单据族首个真契约覆盖（两取号口 明细删口带总额重算 对照表对象口 质检统计口 黑名单清单口）
 *
 * 与既有覆盖的分工。既有全局种子与 20 号等片对采购族做过建单与收货链，
 * 但两枚取号口、明细删口、对照表对象读口、质检统计口、黑名单清单口在判定器
 * 口径下均为零覆盖。本 spec 把五口钉成一条自建自流转的真业务链。
 *
 * 契约真相（逐段读后端源，非推测）：
 * 取号口（handlers/purchase_order_handler 与 handlers/purchase_receipt_handler）：
 * orders 取号出参两键 prefix 固定 PO 与 order_no；receipts 取号出参单键 receipt_no，
 * 前缀 PR；两号均为日期段加流水，不入库，只读探针可重复调。
 * 建单门（services/po 建单校验链）：warehouse_id 强校验非空，供应商与明细必填，
 * 明细行 material_id、quantity_ordered、unit_price 逐行提交；建单初态大写词表 DRAFT。
 * 明细删口（services/po/receipt.rs 的 delete_order_item）：事务内删行并重算总额，
 * 门为订单必须 DRAFT 且操作人为创建人；非 DRAFT 拒绝走 business 族 HTTP 400。
 * 质检统计口（handlers/purchase_inspection_handler 与 models/purchase_inspection）：
 * 出参四键 total、pending、passed、failed，支持 supplier_id 过滤，与列表口同源同筛选。
 * 对照表对象读口（handlers/sku_mapping_handler）：GET 单行按 id，出参含
 * product_id 与 supplier_id 两枚外键列；数据依赖全局对照表种子，读空即判红不兜底。
 * 黑名单（handlers/supplier_blacklist_handler 与 services/supplier_blacklist_service）：
 * 建请求必填 supplier_id、blacklist_reason、detail_description、approver_id、is_permanent；
 * 清单口支持 supplier_id 过滤，出参分页四键；release 口为 POST 集合段动作。
 * 金额比较：rust_decimal 出参为字符串，Number 归一后对表。
 *
 * 假绿防线。统计口用自建全新供应商做零值域断言（四键全零且与列表口 total 互证），
 * 不接受"跑通即可"的空转；删明细后必须同时回读行数与总额两个口径；
 * 负例必须 expectStateGateRejection 钉 400 与状态门机器码族并零漂移；
 * 对照表种子缺失时显式判红，不静默跳过该端点。
 */

type Row = Record<string, unknown>;

const today = (): string => new Date().toISOString().slice(0, 10);
const toNum = (v: unknown): number => Number(String(v));

/**
 * 99 单据族链。单文件串行推进，前置供应商、仓库、部门、产品全部真实反查或自建。
 */
test.describe.serial('99 采购单据族链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('T1 两取号口形状 建单两行 明细删口与总额重算 非草稿删行被拒', async ({ page }) => {
    const tag = genCode('PO99');

    // 1) 取号探针（A 档两枚）：形状与键集，不入库所以可反复取同段号，只断形态
    const poNo = await apiCallRaw<Row>(page, 'GET', '/purchase/orders/generate-no');
    expect(String(poNo.prefix), `orders 取号 prefix 必须 PO，实际 ${String(poNo.prefix)}`).toBe(
      'PO'
    );
    expect(
      /^PO\d+$/.test(String(poNo.order_no)),
      `orders 取号应为 PO 加数字段，实际 ${String(poNo.order_no)}`
    ).toBe(true);

    const rcvNo = await apiCallRaw<Row>(page, 'GET', '/purchase/receipts/generate-no');
    expect(
      /^PR\d+$/.test(String(rcvNo.receipt_no)),
      `receipts 取号应为 PR 加数字段，实际 ${String(rcvNo.receipt_no)}`
    ).toBe(true);

    // 2) 前置反查：供应商与产品自建，仓库与部门取全局种子真实行（建单门强校验仓库）
    const supplier = await apiCallRaw<Row>(page, 'POST', '/purchase/suppliers', {
      supplier_name: `99供应商${tag}`,
    });
    const supplierId = Number(supplier.id);
    expect(supplierId > 0, `供应商自建失败 ${JSON.stringify(supplier)}`).toBe(true);

    const category = await apiCallRaw<Row>(page, 'POST', '/categories', {
      name: `99类${tag}`,
      code: `99CAT${tag}`,
    });
    const categoryId = Number(category.id);
    expect(categoryId > 0, `分类自建失败 ${JSON.stringify(category)}`).toBe(true);

    const product = await apiCallRaw<Row>(page, 'POST', '/products', {
      name: `99坯${tag}`,
      code: `99P${tag}`,
      category_id: categoryId,
      unit: 'm',
      // 夹具名即「坯」：按坯布收货语义取单源坯布类型，面料三键随类型同全
      product_type: PRODUCT_TYPE.greige,
      width: 160,
      gram_weight: 210,
      fabric_composition: '100%棉',
    });
    const productId = Number(product.id);
    expect(productId > 0, `产品自建失败 ${JSON.stringify(product)}`).toBe(true);

    const warehouses = await apiCallRaw<{ items: Row[]; total: number }>(
      page,
      'GET',
      '/warehouses?page=1&page_size=1'
    );
    expect(
      warehouses.items.length,
      '仓库种子缺失（全局种子未建仓库属载体级失败，不兜底）'
    ).toBeGreaterThan(0);
    const warehouseId = Number(warehouses.items[0].id);

    const departments = await apiCallRaw<{ items: Row[]; total: number }>(
      page,
      'GET',
      '/departments?page=1&page_size=1'
    );
    expect(
      departments.items.length,
      '部门种子缺失（全局种子未建部门属载体级失败，不兜底）'
    ).toBeGreaterThan(0);
    const departmentId = Number(departments.items[0].id);

    // 3) 建单两行：10 x 12.50 加 5 x 20.00，总额 225.00
    const order = await apiCallRaw<Row>(page, 'POST', '/purchase/orders', {
      supplier_id: supplierId,
      order_date: today(),
      warehouse_id: warehouseId,
      department_id: departmentId,
      notes: `99建单${tag}`,
      items: [
        { material_id: productId, quantity_ordered: '10', unit_price: '12.50', tax_rate: 0 },
        { material_id: productId, quantity_ordered: '5', unit_price: '20.00', tax_rate: 0 },
      ],
    });
    const orderId = Number(order.id);
    expect(orderId > 0, `建单失败 ${JSON.stringify(order)}`).toBe(true);
    expect(
      String(order.order_status),
      `初态必须大写 DRAFT，实际 ${String(order.order_status)}`
    ).toBe('DRAFT');
    expect(
      toNum(order.total_amount),
      `两行建单总额应 225.00，实际 ${String(order.total_amount)}`
    ).toBe(225);

    // 4) 明细行反查两枚行 id（列表口先行，删口按真实行主键走）
    const items = await apiCallRaw<Row[]>(page, 'GET', `/purchase/orders/${orderId}/items`);
    expect(Array.isArray(items), '明细列表必须是数组').toBe(true);
    expect(items.length, '两行建单后明细应为 2 行').toBe(2);
    const victim = items[0];
    const survivor = items[1];
    // 明细列表口的视图对象把外键序列化成 material_id（提交键也叫 material_id），
    // 读 product_id 会恒 undefined——同键比同键才是「提交什么回读什么」这条锁的本意。
    expect(String(victim.material_id), `明细回读 material_id 必须等于提交物料 ${productId}`).toBe(
      String(productId)
    );

    // 5) 删除第一行（A 档删口），行数与总额双口径回读重算真实发生
    await apiCall(page, 'DELETE', `/purchase/orders/${orderId}/items/${Number(victim.id)}`);
    const itemsAfter = await apiCallRaw<Row[]>(page, 'GET', `/purchase/orders/${orderId}/items`);
    expect(itemsAfter.length, '删行后明细必须剩 1 行').toBe(1);
    expect(Number(itemsAfter[0].id), '剩余行必须是未删的那一行（防删错行）').toBe(
      Number(survivor.id)
    );
    const orderAfter = await apiCallRaw<Row>(page, 'GET', `/purchase/orders/${orderId}`);
    expect(
      toNum(orderAfter.total_amount),
      `删行后总额应重算为 100.00，实际 ${String(orderAfter.total_amount)}`
    ).toBe(100);

    // 6) 提交并审批后删行必须被状态门拒且零漂移（门为仅 DRAFT 可删）
    await apiCall(page, 'POST', `/purchase/orders/${orderId}/submit`, {});
    await apiCall(page, 'POST', `/purchase/orders/${orderId}/approve`, {});
    const rejectedDelete = await apiCallExpectFail(
      page,
      'DELETE',
      `/purchase/orders/${orderId}/items/${Number(survivor.id)}`
    );
    expectStateGateRejection(rejectedDelete, `非草稿订单 ${orderId} 删明细`);
    const itemsKept = await apiCallRaw<Row[]>(page, 'GET', `/purchase/orders/${orderId}/items`);
    expect(itemsKept.length, '被拒的删除不许少行').toBe(1);
    const orderKept = await apiCallRaw<Row>(page, 'GET', `/purchase/orders/${orderId}`);
    expect(toNum(orderKept.total_amount), '被拒的删除不许动总额').toBe(100);
    expect(
      String(orderKept.order_status),
      `审批后状态必须离开 DRAFT，实际 ${String(orderKept.order_status)}`
    ).not.toBe('DRAFT');

    // 7) 清理。已审批单不可删，供应商与产品与分类走延迟删除通道并如实告警
    await tryCleanup(page, 'DELETE', `/products/${productId}`, `[99] product ${productId}`);
    await tryCleanup(page, 'DELETE', `/categories/${categoryId}`, `[99] category ${categoryId}`);
    await tryCleanup(
      page,
      'DELETE',
      `/purchase/suppliers/${supplierId}`,
      `[99] supplier ${supplierId}`
    );
  });

  test('T2 对照表对象读口按种子真行回读', async ({ page }) => {
    // 对照表数据由全局种子建设（外键族太重不在此自建）；读空即判红，不静默跳过
    const list = await apiCallRaw<{ items: Row[]; total: number }>(
      page,
      'GET',
      '/purchase/sku-mappings?page=1&page_size=1'
    );
    expect(
      list.items.length,
      '对照表种子缺失（全局种子第 19 步未落行属载体级失败，不兜底）'
    ).toBeGreaterThan(0);
    const seedRow = list.items[0];
    const mappingId = Number(seedRow.id);

    // A 档对象读口：按真实主键取单行，与列表行逐键互证（同 id 同外键）
    const one = await apiCallRaw<Row>(page, 'GET', `/purchase/sku-mappings/${mappingId}`);
    expect(Number(one.id), '对象读口 id 必须等于请求主键').toBe(mappingId);
    expect(toNum(one.product_id), '对照行 product_id 必须为真实外键正数').toBeGreaterThan(0);
    expect(toNum(one.supplier_id), '对照行 supplier_id 必须为真实外键正数').toBeGreaterThan(0);
    expect(String(one.product_id), '对象读口与列表口同行同值（两通道取数一致）').toBe(
      String(seedRow.product_id)
    );
  });

  test('T3 质检统计口零值域与列表口互证 黑名单清单口随行可见', async ({ page }) => {
    const tag = genCode('BL99');

    // 全新供应商名域：质检必然零行，统计四键全零，且与列表口 total 互证
    const supplier = await apiCallRaw<Row>(page, 'POST', '/purchase/suppliers', {
      supplier_name: `99质检空白${tag}`,
    });
    const supplierId = Number(supplier.id);
    expect(supplierId > 0, `供应商自建失败 ${JSON.stringify(supplier)}`).toBe(true);

    // A 档统计口（零值域断言，同筛选键 supplier_id 与列表口同源）
    const stats = await apiCallRaw<Row>(
      page,
      'GET',
      `/purchase/inspections/stats?supplier_id=${supplierId}`
    );
    for (const key of ['total', 'pending', 'passed', 'failed']) {
      expect(stats[key], `空白供应商统计键 ${key} 必须存在且可归数`).toBeDefined();
      expect(toNum(stats[key]), `空白供应商统计键 ${key} 必须为 0`).toBe(0);
    }
    const inspList = await apiCallRaw<{ items: Row[]; total: number }>(
      page,
      'GET',
      `/purchase/inspections?supplier_id=${supplierId}&page=1&page_size=10`
    );
    expect(
      inspList.total,
      `统计 total 与列表 total 必须同筛选同值，实际 ${String(stats.total)} 对 ${inspList.total}`
    ).toBe(toNum(stats.total));

    // A 档清单口：拉黑自建供应商后按 supplier_id 过滤必须看到该行且原因回读一致
    const me = await apiCallRaw<Row>(page, 'GET', '/auth/me');
    const approverId = Number(me.id);
    expect(approverId > 0, '/auth/me 必须回真实审批人 id').toBe(true);

    const added = await apiCallRaw<Row>(page, 'POST', '/purchase/supplier-blacklists', {
      supplier_id: supplierId,
      blacklist_reason: `99拉黑原因${tag}`,
      detail_description: `99详细说明${tag}（连续交期违约）`,
      approver_id: approverId,
      blacklist_date: today(),
      approval_date: today(),
      is_permanent: false,
    });
    expect(added, '拉黑建口必须返回行体').not.toBeNull();

    const list = await apiCallRaw<{ items: Row[]; total: number }>(
      page,
      'GET',
      `/purchase/supplier-blacklists?supplier_id=${supplierId}&page=1&page_size=10`
    );
    expect(list.total, '按供应商过滤必须命中一行').toBe(1);
    const row = list.items[0];
    expect(Number(row.supplier_id), '清单行 supplier_id 必须等于自建供应商').toBe(supplierId);
    expect(
      String(row.blacklist_reason),
      `清单行拉黑原因必须回读提交原文，实际 ${String(row.blacklist_reason)}`
    ).toBe(`99拉黑原因${tag}`);
    expect(row.id, '清单行必须有真实主键供 release').toBeDefined();

    // release 收尾并回读过滤面（清单口二次真用，行仍可查但状态口径由 release 通道处理）
    await apiCall(page, 'POST', `/purchase/supplier-blacklists/${Number(row.id)}/release`, {});

    await tryCleanup(
      page,
      'DELETE',
      `/purchase/suppliers/${supplierId}`,
      `[99] supplier ${supplierId}`
    );
  });
});
