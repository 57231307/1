import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCallRaw, genCode, tryCleanup } from './helpers';
// 产品类型单源常量（与后端 product_type 词表同源）：建品夹具取值必须经成员引用。
import { PRODUCT_TYPE } from '../../src/constants/product-type';

/**
 * 主数据读回持久锁 + 列表列值非空锁（客户 / 供应商 / 产品）
 *
 * 目的：高频主数据「保存显示成功、回读却是空/旧/缺键」这类假绿，用真实 API 链路逐字段钉死。
 * 与 56-read-after-write（UI reload 后断列表单元格渲染）分层互补：本组只走 API，
 * 不依赖任何 DOM 选择器，断言落在「后端落库值 / 列表出参值」本身。
 *
 * 端点与出参键（对着后端源码逐一核实，键名以实体/DTO 序列化为准，snake_case）：
 * - 客户 POST/GET/PUT/DELETE /crm/customers（handlers/customer_handler.rs +
 *   models/customer.rs）。create 返回裸 Model，get/list 出口经字段级权限与 PII 脱敏层；
 *   admin 会话不脱敏，故读回值断言只用非 PII 的稳定列：
 *   customer_name / customer_code / customer_type / status / credit_limit / payment_terms /
 *   city / province / main_products / customer_industry。credit_limit 为 Decimal，
 *   序列化为字符串且入库按列标度归一（提交 8888.88 可能读回 8888.88 或等价写法），
 *   一律 Number() 归一后按数值比较，避免标度差造成假红，同时仍能因「值为空/为旧/缺键」而红。
 *   建行 owner_id 即当前登录用户，更新/删除的跨 owner 写门按本人行放行。
 * - 供应商 POST/GET/PUT/DELETE /purchase/suppliers（handlers/supplier_handler.rs +
 *   services/supplier_service.rs）。create 入参必填 supplier_name，其余列在服务层给默认值；
 *   响应为裸 Model。建前服务端按 supplier_name 全库查重，故用唯一名造数。
 *   读回值断言用稳定列：supplier_name / supplier_short_name / supplier_type /
 *   registered_address / legal_representative / main_business / contact_phone / supplier_code。
 * - 产品 POST/GET/PUT/DELETE /products（handlers/product_handler.rs + models/product.rs）。
 *   关键事实：create/update 返回**裸 Model**（键 name / code / status / standard_price /
 *   unit / category_id），而 get_product / list_products 才会经 append_frontend_aliases 补出
 *   UI 契约别名 product_name / product_code / price / is_active，并经 attach_category_names
 *   把 category_id 关联出 category_name。读回断言同时钉「实体真名」与「UI 别名」两组键，
 *   任一组缺失/为空即红（这正是「列表 LEFT JOIN 富化名 vs 前端列恒空」缺陷族的回归锁）。
 *   standard_price 为 Decimal → 序列化字符串，同样 Number() 归一比较。
 *
 * 列表出参信封统一为 PaginatedResponse{items,total,page,page_size}（utils/response.rs），
 * 列表定位一律先断 items 为数组（缺键即红，绝不 `?? []` 吞掉），再按主键 id 命中自建行。
 *
 * 本批未覆盖（如实标注，避免为凑数写弱断言）：字段级非法值/词表拒绝锁（如 customer_type
 * 的 CHECK 值域）属另一类契约，需先把拒绝码/文案钉到契约层，本组不含该负向用例。
 */

type Row = Record<string, unknown>;

/** rust_decimal 序列化为 string|number 均可能，统一 Number(String(v)) 归一后按值比较 */
const toDec = (v: unknown): number => Number(String(v));

/** 断言列表出参含 items 数组并返回（缺键/形状漂移即红，禁止默认空数组吞掉） */
function requireItemsArray(env: Row | null | undefined, label: string): Row[] {
  expect(
    env !== null && env !== undefined && Array.isArray((env as Row).items),
    `${label} 列表出参必须含 items 数组（PaginatedResponse），实际 ${JSON.stringify(env).slice(0, 200)}`
  ).toBe(true);
  return (env as Row).items as Row[];
}

/** 在 items 中按 id 精确命中自建行；命中不了即红（证明列表确实回读到了刚建的数据行） */
function findRowById(items: Row[], id: number, label: string): Row {
  const row = items.find(it => Number(it.id) === id);
  expect(
    row,
    `${label} 列表应能按 id=${id} 回读到自建行（回读不到=建行未真正落库或列表未包含它）`
  ).toBeTruthy();
  return row as Row;
}

test.describe('主数据读回持久锁 + 列表列值非空锁（客户/供应商/产品）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // ===================== 客户 =====================

  test('客户-新建回读：POST 逐字段提交值经 GET 回读全部持久', async ({ page }) => {
    const code = genCode('MD83C');
    const name = `主数据客户_${code}`;
    const created = await apiCallRaw<Row>(page, 'POST', '/crm/customers', {
      customer_code: code,
      customer_name: name,
      customer_type: 'wholesale',
      status: 'active',
      credit_limit: '8888.88',
      payment_terms: 45,
      city: '杭州',
      province: '浙江',
      main_products: '坯布',
      customer_industry: '纺织',
    });
    const id = Number(created?.id);
    expect(id, `客户建行应回主键 id：${JSON.stringify(created).slice(0, 200)}`).toBeGreaterThan(0);

    try {
      const detail = await apiCallRaw<Row>(page, 'GET', `/crm/customers/${id}`);
      expect(detail?.customer_code, `回读 customer_code 应等于提交值 ${code}`).toBe(code);
      expect(detail?.customer_name, `回读 customer_name 应等于提交值 ${name}`).toBe(name);
      expect(detail?.customer_type, `回读 customer_type 应为提交值 wholesale`).toBe('wholesale');
      expect(detail?.status, `回读 status 应为提交值 active`).toBe('active');
      expect(
        toDec(detail?.credit_limit),
        `回读 credit_limit 应等于提交值 8888.88，实际 ${JSON.stringify(detail?.credit_limit)}`
      ).toBeCloseTo(8888.88, 4);
      expect(
        Number(detail?.payment_terms),
        `回读 payment_terms 应等于提交值 45，实际 ${JSON.stringify(detail?.payment_terms)}`
      ).toBe(45);
      expect(detail?.city, `回读 city 应为提交值 杭州`).toBe('杭州');
      expect(detail?.province, `回读 province 应为提交值 浙江`).toBe('浙江');
      expect(detail?.main_products, `回读 main_products 应为提交值 坯布`).toBe('坯布');
      expect(detail?.customer_industry, `回读 customer_industry 应为提交值 纺织`).toBe('纺织');
    } finally {
      await tryCleanup(page, 'DELETE', `/crm/customers/${id}`, '客户建行回读-清理');
    }
  });

  test('客户-编辑回读：PUT 改名称与额度后回读新值持久、编码状态等其余字段不丢', async ({
    page,
  }) => {
    const code = genCode('MD83CE');
    const origName = `主数据客户原名_${code}`;
    const created = await apiCallRaw<Row>(page, 'POST', '/crm/customers', {
      customer_code: code,
      customer_name: origName,
      customer_type: 'retail',
      status: 'active',
      credit_limit: '1000.00',
      payment_terms: 30,
    });
    const id = Number(created?.id);
    expect(id, `客户建行应回主键 id：${JSON.stringify(created).slice(0, 200)}`).toBeGreaterThan(0);

    const newName = `主数据客户改名_${genCode('N')}`;
    try {
      await apiCallRaw<Row>(page, 'PUT', `/crm/customers/${id}`, {
        customer_name: newName,
        credit_limit: '2000.00',
      });

      const detail = await apiCallRaw<Row>(page, 'GET', `/crm/customers/${id}`);
      // 被改字段：读到新值
      expect(
        detail?.customer_name,
        `编辑后回读 customer_name 应为新值 ${newName}，实际 ${JSON.stringify(detail?.customer_name)}`
      ).toBe(newName);
      expect(detail?.customer_name, `编辑后回读 customer_name 不得仍是旧值 ${origName}`).not.toBe(
        origName
      );
      expect(
        toDec(detail?.credit_limit),
        `编辑后回读 credit_limit 应为新值 2000，实际 ${JSON.stringify(detail?.credit_limit)}`
      ).toBeCloseTo(2000, 4);
      // 未提交字段（编码不可改 + 状态/渠道/账期 PUT 未携带=不改）必须保持原值，证明编辑未误清其它列
      expect(detail?.customer_code, `编码不可经 PUT 改，回读应仍为 ${code}`).toBe(code);
      expect(detail?.customer_type, `未提交的 customer_type 应保持不变 retail`).toBe('retail');
      expect(detail?.status, `未提交的 status 应保持不变 active`).toBe('active');
      expect(
        Number(detail?.payment_terms),
        `未提交的 payment_terms 应保持 30，实际 ${JSON.stringify(detail?.payment_terms)}`
      ).toBe(30);
    } finally {
      await tryCleanup(page, 'DELETE', `/crm/customers/${id}`, '客户编辑回读-清理');
    }
  });

  test('客户-列表列值非空锁：keyword 命中自建行，名称/编码列等于造数真值', async ({ page }) => {
    const code = genCode('MD83CL');
    const name = `主数据客户列表_${code}`;
    const created = await apiCallRaw<Row>(page, 'POST', '/crm/customers', {
      customer_code: code,
      customer_name: name,
    });
    const id = Number(created?.id);
    expect(id, `客户建行应回主键 id：${JSON.stringify(created).slice(0, 200)}`).toBeGreaterThan(0);

    try {
      const env = await apiCallRaw<Row>(
        page,
        'GET',
        `/crm/customers?keyword=${encodeURIComponent(code)}&page=1&page_size=100`
      );
      const items = requireItemsArray(env, '客户列表');
      const row = findRowById(items, id, '客户列表');
      // 名称类列必须非空且等于造数真值（后端若漏投影/前端列绑错字段，这里即为空或不等→红）
      expect(
        typeof row.customer_name === 'string' && row.customer_name.length > 0,
        `列表行 customer_name 应为非空字符串，实际 ${JSON.stringify(row.customer_name)}`
      ).toBe(true);
      expect(row.customer_name, `列表 customer_name 应等于 ${name}`).toBe(name);
      expect(row.customer_code, `列表 customer_code 应等于 ${code}`).toBe(code);
    } finally {
      await tryCleanup(page, 'DELETE', `/crm/customers/${id}`, '客户列表列锁-清理');
    }
  });

  // ===================== 供应商 =====================

  test('供应商-新建回读：POST 逐字段提交值经 GET 回读全部持久', async ({ page }) => {
    const name = `主数据供应商_${genCode('MD83S')}`;
    const created = await apiCallRaw<Row>(page, 'POST', '/purchase/suppliers', {
      supplier_name: name,
      supplier_short_name: '主数供83',
      supplier_type: '面料供应商',
      registered_address: '杭州市西湖区83号',
      legal_representative: '张三83',
      main_business: '坯布织造',
      contact_phone: '13900008300',
    });
    const id = Number(created?.id);
    expect(id, `供应商建行应回主键 id：${JSON.stringify(created).slice(0, 200)}`).toBeGreaterThan(
      0
    );

    try {
      const detail = await apiCallRaw<Row>(page, 'GET', `/purchase/suppliers/${id}`);
      expect(detail?.supplier_name, `回读 supplier_name 应等于提交值 ${name}`).toBe(name);
      expect(detail?.supplier_short_name, `回读简称应为主数供83`).toBe('主数供83');
      expect(detail?.supplier_type, `回读 supplier_type 应为面料供应商`).toBe('面料供应商');
      expect(detail?.registered_address, `回读注册地址应为杭州市西湖区83号`).toBe(
        '杭州市西湖区83号'
      );
      expect(detail?.legal_representative, `回读法人应为张三83`).toBe('张三83');
      expect(detail?.main_business, `回读主营业务应为坯布织造`).toBe('坯布织造');
      expect(detail?.contact_phone, `回读联系电话应为 13900008300`).toBe('13900008300');
      // supplier_code 由服务端取号，必须非空（取号链路回归锁）
      expect(
        typeof detail?.supplier_code === 'string' && detail.supplier_code.length > 0,
        `回读 supplier_code 应为非空（服务端取号），实际 ${JSON.stringify(detail?.supplier_code)}`
      ).toBe(true);
    } finally {
      await tryCleanup(page, 'DELETE', `/purchase/suppliers/${id}`, '供应商建行回读-清理');
    }
  });

  test('供应商-编辑回读：PUT 改主营与名称后回读新值持久、地址类型等其余字段不丢', async ({
    page,
  }) => {
    const origName = `主数据供应商原名_${genCode('MD83SE')}`;
    const created = await apiCallRaw<Row>(page, 'POST', '/purchase/suppliers', {
      supplier_name: origName,
      supplier_short_name: '原名供83',
      supplier_type: '辅料供应商',
      registered_address: '绍兴市83号',
      main_business: '原名主营',
    });
    const id = Number(created?.id);
    expect(id, `供应商建行应回主键 id：${JSON.stringify(created).slice(0, 200)}`).toBeGreaterThan(
      0
    );

    const newName = `主数据供应商改名_${genCode('N')}`;
    const newMain = `改名后主营_${genCode('M')}`;
    try {
      await apiCallRaw<Row>(page, 'PUT', `/purchase/suppliers/${id}`, {
        supplier_name: newName,
        main_business: newMain,
      });

      const detail = await apiCallRaw<Row>(page, 'GET', `/purchase/suppliers/${id}`);
      expect(
        detail?.supplier_name,
        `编辑后回读 supplier_name 应为新值 ${newName}，实际 ${JSON.stringify(detail?.supplier_name)}`
      ).toBe(newName);
      expect(detail?.supplier_name, `编辑后回读 supplier_name 不得仍是旧值 ${origName}`).not.toBe(
        origName
      );
      expect(detail?.main_business, `编辑后回读主营应为新值 ${newMain}`).toBe(newMain);
      // 未提交字段保持不变（证明 PUT 未把其它列清掉）
      expect(detail?.supplier_type, `未提交的 supplier_type 应保持 辅料供应商`).toBe('辅料供应商');
      expect(detail?.registered_address, `未提交的注册地址应保持 绍兴市83号`).toBe('绍兴市83号');
      expect(detail?.supplier_short_name, `未提交的简称应保持 原名供83`).toBe('原名供83');
    } finally {
      await tryCleanup(page, 'DELETE', `/purchase/suppliers/${id}`, '供应商编辑回读-清理');
    }
  });

  test('供应商-列表列值非空锁：keyword 命中自建行，名称/编码列等于造数真值', async ({ page }) => {
    const name = `主数据供应商列表_${genCode('MD83SL')}`;
    const created = await apiCallRaw<Row>(page, 'POST', '/purchase/suppliers', {
      supplier_name: name,
      supplier_short_name: '列供83',
    });
    const id = Number(created?.id);
    expect(id, `供应商建行应回主键 id：${JSON.stringify(created).slice(0, 200)}`).toBeGreaterThan(
      0
    );

    try {
      const env = await apiCallRaw<Row>(
        page,
        'GET',
        `/purchase/suppliers?keyword=${encodeURIComponent(name)}&page=1&page_size=100`
      );
      const items = requireItemsArray(env, '供应商列表');
      const row = findRowById(items, id, '供应商列表');
      expect(
        typeof row.supplier_name === 'string' && row.supplier_name.length > 0,
        `列表行 supplier_name 应为非空字符串，实际 ${JSON.stringify(row.supplier_name)}`
      ).toBe(true);
      expect(row.supplier_name, `列表 supplier_name 应等于 ${name}`).toBe(name);
      expect(
        typeof row.supplier_code === 'string' && row.supplier_code.length > 0,
        `列表 supplier_code 应为非空，实际 ${JSON.stringify(row.supplier_code)}`
      ).toBe(true);
    } finally {
      await tryCleanup(page, 'DELETE', `/purchase/suppliers/${id}`, '供应商列表列锁-清理');
    }
  });

  // ===================== 产品 =====================

  // 产品建档必须先有一个真实分类：既满足建单来源，又能用唯一分类名去钉
  // 列表/详情出参的 category_name 富化（后端 attach_category_names 关联 product_categories）。
  async function seedCategory(
    page: import('@playwright/test').Page
  ): Promise<{ id: number; name: string }> {
    const name = `主数据分类_${genCode('MD83CAT')}`;
    const created = await apiCallRaw<Row>(page, 'POST', '/product-categories', { name });
    const id = Number(created?.id);
    expect(id, `产品分类建行应回主键 id：${JSON.stringify(created).slice(0, 200)}`).toBeGreaterThan(
      0
    );
    // 用唯一分类名作为富化对照基准；建分类即定名，读回断言直接拿这个真值
    return { id, name };
  }

  test('产品-新建回读：实体真名键与 UI 别名键同时持久，category_name 富化正确', async ({
    page,
  }) => {
    const cat = await seedCategory(page);
    const code = genCode('MD83P');
    const name = `主数据产品_${code}`;
    const created = await apiCallRaw<Row>(page, 'POST', '/products', {
      code,
      name,
      unit: '米',
      status: 'active',
      category_id: cat.id,
      standard_price: 66.6,
      specification: '133x72',
      // 新建回读列锁用例只比对名称/编码/富化分类列，取非面料类型免带面料三键
      product_type: PRODUCT_TYPE.auxiliary,
    });
    const id = Number(created?.id);
    expect(id, `产品建行应回主键 id：${JSON.stringify(created).slice(0, 200)}`).toBeGreaterThan(0);

    try {
      const detail = await apiCallRaw<Row>(page, 'GET', `/products/${id}`);
      // 实体真名键（models/product.rs 序列化口径）
      expect(detail?.name, `回读 name 应等于提交值 ${name}`).toBe(name);
      expect(detail?.code, `回读 code 应等于提交值 ${code}`).toBe(code);
      expect(detail?.status, `回读 status 应为提交值 active`).toBe('active');
      expect(detail?.unit, `回读 unit 应为提交值 米`).toBe('米');
      expect(Number(detail?.category_id), `回读 category_id 应为 ${cat.id}`).toBe(cat.id);
      expect(
        detail?.specification,
        `回读 specification 应为 133x72，实际 ${JSON.stringify(detail?.specification)}`
      ).toBe('133x72');
      expect(
        toDec(detail?.standard_price),
        `回读 standard_price 应等于提交值 66.6，实际 ${JSON.stringify(detail?.standard_price)}`
      ).toBeCloseTo(66.6, 4);
      // UI 契约别名键（append_frontend_aliases 补出，缺任一即「前端列恒空」缺陷族）
      expect(detail?.product_name, `详情别名 product_name 应等于 ${name}`).toBe(name);
      expect(detail?.product_code, `详情别名 product_code 应等于 ${code}`).toBe(code);
      expect(
        detail?.is_active,
        `详情别名 is_active 应由 status=active 派生为 true，实际 ${JSON.stringify(detail?.is_active)}`
      ).toBe(true);
      expect(
        toDec(detail?.price),
        `详情别名 price 应等于 standard_price 66.6，实际 ${JSON.stringify(detail?.price)}`
      ).toBeCloseTo(66.6, 4);
      // 关联富化列：category_name 必须等于自建分类名（LEFT JOIN 富化，非硬造）
      expect(
        detail?.category_name,
        `详情富化列 category_name 应等于分类真名 ${cat.name}，实际 ${JSON.stringify(detail?.category_name)}`
      ).toBe(cat.name);
    } finally {
      await tryCleanup(page, 'DELETE', `/products/${id}`, '产品建行回读-清理产品');
      await tryCleanup(page, 'DELETE', `/product-categories/${cat.id}`, '产品建行回读-清理分类');
    }
  });

  test('产品-编辑回读：PUT 改名与价格后回读新值持久、编码/单位/分类等不丢', async ({ page }) => {
    const cat = await seedCategory(page);
    const code = genCode('MD83PE');
    const origName = `主数据产品原名_${code}`;
    const created = await apiCallRaw<Row>(page, 'POST', '/products', {
      code,
      name: origName,
      unit: '米',
      status: 'active',
      category_id: cat.id,
      standard_price: 10.5,
      // 编辑回读列锁用例改名与价格不触碰类型列，取非面料类型免带面料三键
      product_type: PRODUCT_TYPE.auxiliary,
    });
    const id = Number(created?.id);
    expect(id, `产品建行应回主键 id：${JSON.stringify(created).slice(0, 200)}`).toBeGreaterThan(0);

    const newName = `主数据产品改名_${genCode('N')}`;
    try {
      await apiCallRaw<Row>(page, 'PUT', `/products/${id}`, {
        name: newName,
        standard_price: 20.25,
      });

      const detail = await apiCallRaw<Row>(page, 'GET', `/products/${id}`);
      expect(
        detail?.name,
        `编辑后回读 name 应为新值 ${newName}，实际 ${JSON.stringify(detail?.name)}`
      ).toBe(newName);
      expect(detail?.name, `编辑后回读 name 不得仍是旧值 ${origName}`).not.toBe(origName);
      expect(detail?.product_name, `编辑后别名 product_name 应同步为新值 ${newName}`).toBe(newName);
      expect(
        toDec(detail?.standard_price),
        `编辑后回读 standard_price 应为 20.25，实际 ${JSON.stringify(detail?.standard_price)}`
      ).toBeCloseTo(20.25, 4);
      expect(
        toDec(detail?.price),
        `编辑后别名 price 应同步为 20.25，实际 ${JSON.stringify(detail?.price)}`
      ).toBeCloseTo(20.25, 4);
      // 编码（UpdateProductRequest 无 code 列）与未提交字段必须保持，证明编辑未误清其它列
      expect(detail?.code, `编码应保持不变 ${code}`).toBe(code);
      expect(detail?.product_code, `别名编码应保持不变 ${code}`).toBe(code);
      expect(detail?.unit, `未提交的 unit 应保持 米`).toBe('米');
      expect(Number(detail?.category_id), `未提交的 category_id 应保持 ${cat.id}`).toBe(cat.id);
      expect(detail?.status, `未提交的 status 应保持 active`).toBe('active');
      expect(detail?.category_name, `富化分类名应保持 ${cat.name}`).toBe(cat.name);
    } finally {
      await tryCleanup(page, 'DELETE', `/products/${id}`, '产品编辑回读-清理产品');
      await tryCleanup(page, 'DELETE', `/product-categories/${cat.id}`, '产品编辑回读-清理分类');
    }
  });

  test('产品-列表列值非空锁：按 category_id 命中自建行，名称/别名/富化分类列等于真值', async ({
    page,
  }) => {
    const cat = await seedCategory(page);
    const code = genCode('MD83PL');
    const name = `主数据产品列表_${code}`;
    const created = await apiCallRaw<Row>(page, 'POST', '/products', {
      code,
      name,
      unit: '米',
      status: 'active',
      category_id: cat.id,
      // 列表列值非空锁不比对类型列，取非面料类型免带面料三键
      product_type: PRODUCT_TYPE.auxiliary,
    });
    const id = Number(created?.id);
    expect(id, `产品建行应回主键 id：${JSON.stringify(created).slice(0, 200)}`).toBeGreaterThan(0);

    try {
      // 用自建分类过滤：分类唯一，命中行必为本次自建产品，规避全局分页漏读
      const env = await apiCallRaw<Row>(
        page,
        'GET',
        `/products?category_id=${cat.id}&page=1&page_size=100`
      );
      const items = requireItemsArray(env, '产品列表');
      const row = findRowById(items, id, '产品列表');
      // 实体真名 + UI 别名 + 编码：列表三组都必须非空且等于真值（前端列表列绑错/缺富化即红）
      expect(
        typeof row.name === 'string' && row.name.length > 0,
        `列表行 name 应为非空字符串，实际 ${JSON.stringify(row.name)}`
      ).toBe(true);
      expect(row.name, `列表 name 应等于 ${name}`).toBe(name);
      expect(row.product_name, `列表别名 product_name 应等于 ${name}`).toBe(name);
      expect(row.code, `列表 code 应等于 ${code}`).toBe(code);
      expect(row.product_code, `列表别名 product_code 应等于 ${code}`).toBe(code);
      // 跨表富化列 category_name：列表必须带出且等于自建分类真名（attach_category_names 回归锁）
      expect(
        typeof row.category_name === 'string' && row.category_name.length > 0,
        `列表富化列 category_name 应为非空，实际 ${JSON.stringify(row.category_name)}`
      ).toBe(true);
      expect(row.category_name, `列表 category_name 应等于 ${cat.name}`).toBe(cat.name);
    } finally {
      await tryCleanup(page, 'DELETE', `/products/${id}`, '产品列表列锁-清理产品');
      await tryCleanup(page, 'DELETE', `/product-categories/${cat.id}`, '产品列表列锁-清理分类');
    }
  });
});
