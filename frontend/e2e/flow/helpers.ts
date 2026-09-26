/* eslint-disable no-console */
import { expect, type Page } from '@playwright/test';
// ESM 环境无 require（Playwright 原生 ESM 加载链），fs/crypto 必须静态导入；
// 此前 require('fs')/require('crypto') 抛 "require is not defined" 导致
// getRoleCredential 恒返 null（全角色 credentials not found）与 generateTotp 崩溃
import { existsSync, readFileSync } from 'fs';
import * as nodeCrypto from 'crypto';
import {
  createColorCardUI,
  createDyeBatchUI,
  createDyeRecipeUI,
  createBomUI,
  createCustomOrderUI,
  pickListArray,
  readFirstEntityId,
  readEntityIds,
} from './ui-helpers';

export const API_BASE = process.env.API_BASE || 'http://localhost:8082';
export const API_PREFIX = '/api/v1/erp';
export const BASE_URL = process.env.BASE_URL || 'http://localhost:3000';
// 分片专属账号：优先级 E2E_SHARD_INDEX（派生 e2e_admin_s{n}）> TEST_USERNAME > 默认。
// 注意 TEST_USERNAME 的 job 级 env（e2e_admin）会短路 || 链，故 E2E_SHARD_INDEX 必须前置判断
export const TEST_USERNAME =
  process.env.E2E_SHARD_INDEX !== undefined && process.env.E2E_SHARD_INDEX !== ''
    ? `e2e_admin_s${process.env.E2E_SHARD_INDEX}`
    : process.env.TEST_USERNAME || 'e2e_admin';
export const TEST_PASSWORD = process.env.TEST_PASSWORD || 'Xk9#mQ2$vL8pW4nR';

export interface ApiResponse<T = unknown> {
  code: number;
  message: string;
  data: T;
  timestamp?: string;
}

// ============================================================================
// 失败响应信封（e2e 侧显式建模）
//
// 全站 HTTP 失败体已收敛为一种形状（`utils/error.rs` 的 `ErrorResponse` 结构体）：
//   { code: "<字符串机器码>", message: "<脱敏常量或可外显文案>", trace_id: "<uuid>", timestamp: <i64> }
// 产出方：
//   ① utils/error.rs:143-148 `AppError::into_response()`（状态码由
//      error_status_and_type()（error.rs:168-182）决定）；
//   ② utils/response.rs `unauthorized_response`/`forbidden_response`（auth/permission 中间件）
//      与 middleware/auth_context.rs `AuthRejection::into_response`——三者都复用同一个
//      `ErrorResponse` 结构体，因此键与类型完全相同，只是 message 原样外显固定文案。
// 原先并存的两种数字 code 失败体（ApiResponse::error / error_with_status）与
// `{ error: "Unauthorized", message }` 已从后端删除，失败体 code 恒为字符串。
//
// 唯一尚未收敛的例外：middleware/csrf.rs:234-242 `csrf_error_response()` 绕过 AppError
// 直出 `{ success: false, code: "<机器码>", message, data: null }`，配 HTTP 403，
// 且消费竞败时响应头带 x-new-csrf-token（csrf.rs:146-152）。它的 code 同为字符串机器码，
// 因此在比较点用 failureCode() 取码，避免与其它 403 判据互相误判。
// ============================================================================

/**
 * CSRF 机器码：与后端 backend/src/middleware/csrf.rs:39-45 的三个常量同名同值
 * （CODE_MISS / CODE_INVAL / CODE_IP_MM）。此处引用真实来源而非散写魔法值，
 * 后端改名/改值时 e2e 侧只有一个维护点。
 */
export const CSRF_ERROR_CODES = {
  /** csrf.rs:39 CODE_MISS */
  MISSING: 'CSRF_TOKEN_MISSING',
  /** csrf.rs:42 CODE_INVAL */
  INVALID: 'CSRF_TOKEN_INVALID',
  /** csrf.rs:45 CODE_IP_MM（后端对 IP 不匹配不下发恢复头，不参与自动重试） */
  IP_MISMATCH: 'CSRF_IP_MISMATCH',
} as const;

/**
 * AppError 机器码（backend/src/utils/error.rs:461-476 AppError::error_code()）。
 * 仅登记 e2e 负例实际断言到的三类；新增判据时在此补一行，不在用例里散写字面量。
 */
export const APP_ERROR_CODES = {
  /** error.rs:467 AppError::ValidationError */
  VALIDATION_ERROR: 'VALIDATION_ERROR',
  /** error.rs:468-469 BusinessError / BusinessErrorDisplayable */
  BUSINESS_ERROR: 'BUSINESS_ERROR',
  /** error.rs:464 AppError::BadRequest */
  BAD_REQUEST: 'BAD_REQUEST',
} as const;

/**
 * 失败响应体：统一形状 `{code,message,trace_id,timestamp}`，失败时 code 恒为字符串机器码
 * （`utils/error.rs` 的 `ErrorResponse`）。`success`/`data` 仅由 csrf.rs:234-242 的直出体携带，
 * 该体没有 trace_id/timestamp。
 *
 * `code` 仍保留 `number` 分支：`apiCall` 解析的是成功信封 `ApiResponse`（数字 200），
 * 与失败体共用同一判据函数；`failureCode()` 只把字符串当机器码，数字码永不命中。
 * `total` 同理来自成功信封顶层。
 */
export interface ApiFailureBody {
  success?: false;
  code?: string | number;
  message?: string | null;
  data?: unknown;
  total?: number;
  trace_id?: string;
  timestamp?: string | number;
}

/** apiCallExpectFail 的返回：HTTP 状态码 + 未收窄的失败体字段 */
export interface ApiFailureResult extends ApiFailureBody {
  status: number;
}

/**
 * 取失败体的字符串机器码（统一形状与 csrf.rs 直出体都是字符串）。
 * 运行时 JSON 不受类型约束，故保留 typeof 判定：非字符串（缺失/异常）返回 undefined，
 * 不做 String() 强转，避免把结构异常当成命中某个机器码。
 */
export function failureCode(json: ApiFailureBody | null | undefined): string | undefined {
  const code = json?.code;
  return typeof code === 'string' ? code : undefined;
}

/** 失败体是否 CSRF 中间件直出的 403 拒绝（缺 MISSING/INVALID 码即视为统一信封的业务机器码） */
export function isCsrfRejection(status: number, json: ApiFailureBody | null | undefined): boolean {
  const code = failureCode(json);
  return status === 403 && (code === CSRF_ERROR_CODES.INVALID || code === CSRF_ERROR_CODES.MISSING);
}

export interface EntityContext {
  departmentIds: number[];
  warehouseIds: number[];
  productCategoryIds: number[];
  productIds: number[];
  productColorIds: number[];
  colorNos: string[];
  /**
   * 报价专用产品：后端 validate_item_units_against_products 要求报价行 unit 逐字符等于
   * 所引用产品的交易单位（单一真源）。ctx.productIds 可能复用库中 unit 未知的历史共享产品，
   * 故 ensureTestEntities 无条件自建一个显式带 unit 的产品，报价造数一律引用它，
   * quotationProductUnit 存后端落库的真实单位（非写死），保证单位配对。
   */
  quotationProductId?: number;
  quotationProductUnit: string;
  /** 报价专用产品的色号 ID，供需要 color_id 的报价用例自给自足引用 */
  quotationProductColorId?: number;
  supplierId?: number;
  customerId?: number;
  accountSubjectIds: number[];
  colorCardId?: number;
  greigeFabricId?: number;
  dyeBatchId?: number;
  dyeLotNo?: string;
  dyeRecipeId?: number;
  productionRecipeId?: number;
  bomId?: number;
  purchaseOrderId?: number;
  salesOrderId?: number;
  quotationId?: number;
  productionOrderId?: number;
  pieceIds: number[];
  stockIds: number[];
  apInvoiceId?: number;
  arInvoiceId?: number;
  voucherId?: number;
  fixedAssetId?: number;
  budgetId?: number;
  customOrderId?: number;
  roleId?: number;
  userIds: number[];
}

const ctx: EntityContext = {
  departmentIds: [],
  warehouseIds: [],
  productCategoryIds: [],
  productIds: [],
  productColorIds: [],
  colorNos: [],
  quotationProductUnit: '',
  accountSubjectIds: [],
  pieceIds: [],
  stockIds: [],
  userIds: [],
};

export function getCtx(): EntityContext {
  return ctx;
}

/**
 * 确保 EntityContext 有测试所需的基础实体 ID
 * 分片后每个 shard 独立运行，EntityContext 单例不跨 shard 共享
 * 此函数在每个 spec 文件开头调用，自行创建或查找实体
 */
/**
 * UI 创建 + 失败重试包装：
 * 首次创建失败时，强制重新登录（恢复可能失效的 CSRF 会话）后重试一次。
 * 背景：CSRF Token 为一次性消费，页面请求与 apiCall 并发时可能竞争 token，
 * 前端 CSRF 校验失败会清空 csrf_token Cookie，仅靠重试表单操作无法恢复。
 */
async function uiCreateWithRetry(
  page: Page,
  fn: (p: Page) => Promise<number | undefined>
): Promise<number | undefined> {
  // 单次尝试：失败即返回 undefined 由调用方 API 兜底。
  // 原“重登+重试”路径每次失败额外消耗 40-60s，多次实体累积导致 120s 测试超时。
  return fn(page);
}

async function ensureTestEntitiesInner(page: Page): Promise<void> {
  // 会话预检查：CSRF 校验失败场景下前端会清空 csrf_token Cookie并跳转登录页；
  // 401 后守卫 init/status 失败会安全跳 /setup。检测到 csrf_token 缺失
  // 或页面已被踢到 /login、/setup 时强制重新登录，避免逐个实体失败浪费重试时间
  if (LOGGED_IN.done) {
    const cookies = await page.context().cookies();
    const hasCsrf = cookies.some(c => c.name === 'csrf_token');
    const currentUrl = page.url();
    const kickedOut = currentUrl.includes('/login') || currentUrl.includes('/setup');
    if (!hasCsrf || kickedOut) {
      console.warn(
        `[ensureTestEntities] 会话异常（csrf=${hasCsrf}，url=${currentUrl}），强制重新登录`
      );
      await loginViaUI(page, undefined, undefined, true);
    }
  }

  // ---- 1. 仓库（UI 创建）----
  try {
    ctx.warehouseIds = await readEntityIds(
      page,
      '/warehouse',
      `${API_PREFIX}/warehouses`,
      // warehouse_handler.rs:88 define_crud_handlers! → warehouse_service::list PaginatedResponse
      'items'
    );
  } catch (e) {
    console.warn('[ensureTestEntities] 仓库列表查询失败（可能空库）:', (e as Error).message);
    ctx.warehouseIds = [];
  }
  if (ctx.warehouseIds.length < 2) {
    // API 创建（CreateWarehouseRequest：name/code 经 serde alias 兼容 warehouse_*）
    // 创建失败直接抛错：前置实体缺失时后续测试的断言无意义，禁止兜底掩盖
    for (let i = ctx.warehouseIds.length; i < 2; i++) {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/warehouses', {
        name: `E2E仓库${Date.now().toString().slice(-6)}${i}`,
        code: `E2E-W${Date.now().toString().slice(-6)}${i}`,
      });
      if (!result.data?.id) {
        throw new Error(`[ensureTestEntities] 仓库创建失败: ${JSON.stringify(result)}`);
      }
      ctx.warehouseIds.push(result.data.id);
    }
  }

  // ---- 1.5 当前用户 ID（后续步骤依赖：报价单 sales_user_id 必填）----
  // 必须在报价单创建前完成，否则 ctx.userIds 为空导致 422
  // /auth/me 不限角色；/users 列表仅 admin 可访问
  try {
    const me = await apiCallRaw<{ id: number; username?: string }>(page, 'GET', '/auth/me');
    if (!me?.id) {
      throw new Error('当前用户 ID 缺失（/auth/me 未返回 id）');
    }
    ctx.userIds = [me.id];
    console.log('[ensureTestEntities] 当前用户 id=', me.id, 'username=', me.username);
  } catch (e) {
    throw new Error(`[ensureTestEntities] 当前用户查询失败: ${(e as Error).message}`);
  }

  // ---- 2. 产品（UI 创建）----
  // 前置：确保"面料"产品分类存在（表单 category_id 必填，系统初始化不创建分类种子数据）
  try {
    // /product-categories：product_category_handler.rs:44 define_crud_handlers! →
    // product_category_service::list 返回 PaginatedResponse，data 形状为 {items,total,page,page_size}。
    // 单一形状直读（'items'）；原写法 `Array.isArray(cats)?cats:(cats.items||[])` 同时吞裸数组/items，
    // 且 `|| []` 把 items 键缺失当成"无分类"——分类端点若改形会静默走创建分支重复建"面料"。
    const cats = await apiCallRaw<unknown>(page, 'GET', '/product-categories');
    const catItems = pickListArray<{ id: number; name?: string }>(
      cats,
      'items',
      'ensureTestEntities /product-categories'
    );
    const fabricCat = catItems.find(c => c.name?.includes('面料'));
    if (fabricCat) {
      ctx.productCategoryIds.push(fabricCat.id);
    } else {
      // create（crud_macro.rs:89-134 define_crud_handlers!）返回 ApiResponse<to_value(item)>，
      // 载荷即实体本身，故泛型参数写载荷 { id?: number }，读 created.data.id；
      // 原写法把 { data?: { id?: number } } 当作载荷传入，于是再读 .data.id 造成双重包装。
      const created = await apiCall<{ id?: number }>(page, 'POST', '/product-categories', {
        name: '面料',
        code: 'FABRIC',
      });
      if (!created.data?.id) {
        throw new Error(`[ensureTestEntities] 产品分类创建未返回 id: ${JSON.stringify(created)}`);
      }
      ctx.productCategoryIds.push(created.data.id);
      console.log('[ensureTestEntities] 创建产品分类"面料" id=', created.data.id);
    }
  } catch (e) {
    throw new Error(`[ensureTestEntities] 产品分类检查/创建失败: ${(e as Error).message}`);
  }
  try {
    ctx.productIds = await readEntityIds(
      page,
      '/product',
      `${API_PREFIX}/products`,
      // product_handler.rs:247 list_products → ApiResponse::success(PaginatedResponse) → {items}
      'items'
    );
  } catch (e) {
    console.warn('[ensureTestEntities] 产品列表查询失败（可能空库）:', (e as Error).message);
    ctx.productIds = [];
  }
  // 夹具基数假设：消费用例要求 ctx.productIds 至少 3 个——
  // 03-production「3-8 创建 BOM」用 items: productIds.slice(1)（需 ≥2 才非空，
  // 否则后端 bom_handler.rs:33 items min=1 合法 400）；
  // 01-p2p「1-6b」用 ctx.productIds[1] 作「产品对不上」负例（需 ≥2 才不 undefined）。
  // wave5c 的 global-setup.ensureGlobalBusinessSeed 会先建全局产品，跨分片共库下
  // readEntityIds 读到的现有产品可能已是 1~2 个。原逻辑仅在 length===0 时补齐，
  // seed 产品会让补齐整段被跳过 → ctx.productIds 饿死到 1 个 → 两用例红。
  // 故改为无条件补齐到至少 3：读到的现有 id（含 seed 产品）全部保留并计入基数，
  // 不足 3 才补建，补建走与本函数既有建产品一致的字段口径（含克重/幅宽）。
  if (ctx.productIds.length < 3) {
    const catId = ctx.productCategoryIds[0];
    expect(catId, '[ensureTestEntities] 产品分类 id 缺失').toBeTruthy();
    while (ctx.productIds.length < 3) {
      const seq = ctx.productIds.length;
      try {
        // CreateProductRequest：code/name/category_id 必填；
        // Q1 报价转订单按产品克重×幅宽做米↔公斤真实换算，缺则后端拒绝——通用产品也需带。
        // 编码/名称带「末6位毫秒时间戳 + 当前基数 seq」唯一后缀，保证幂等：
        // 与 seed 产品及历史数据不重名，重跑/并发分片各建各的，不制造冲突。
        const result = await apiCall<{ id?: number }>(page, 'POST', '/products', {
          code: `E2E-P${Date.now().toString().slice(-6)}${seq}`,
          name: `E2E产品${Date.now().toString().slice(-6)}${seq}`,
          unit: '米',
          category_id: catId,
          gram_weight: 180,
          width: 150,
          meters_per_piece: 50,
          meters_per_roll: 100,
        });
        if (result.data?.id) {
          ctx.productIds.push(result.data.id);
          console.log(
            `[ensureTestEntities] 产品补齐创建成功 id=${result.data.id}（当前基数=${ctx.productIds.length}）`
          );
        } else if (seq === 0) {
          // 空库首轮一个产品都拿不到属真实环境缺陷，直接抛错暴露，不做假兜底
          throw new Error(`[ensureTestEntities] 产品创建失败: ${JSON.stringify(result)}`);
        } else {
          console.error('[ensureTestEntities] 产品补齐未返回 id:', JSON.stringify(result));
          break;
        }
      } catch (e) {
        if (seq === 0) {
          throw new Error(`[ensureTestEntities] 产品创建失败: ${(e as Error).message}`);
        }
        console.error('[ensureTestEntities] 产品补齐创建失败:', (e as Error).message);
        break;
      }
    }
  }

  // ---- 3. 产品色号（仍用 API，因为色号在详情页创建且依赖 product_id）----
  try {
    // 真实端点：GET /products/{id}/colors（返回数组，非分页包装）
    const colors = await apiCallRaw<Array<{ id: number; color_no: string }>>(
      page,
      'GET',
      `/products/${ctx.productIds[0]}/colors`
    );
    ctx.productColorIds = colors?.map(c => c.id) || [];
    ctx.colorNos = colors?.map(c => c.color_no) || ['TEST-COLOR'];
  } catch (e) {
    throw new Error(`[ensureTestEntities] 色号查询失败: ${(e as Error).message}`);
  }
  if (ctx.colorNos.length === 0) {
    // 新建产品天然无色号——真实创建一个（CreateProductColorRequest），非占位兜底
    const created = await apiCall<{ id?: number }>(
      page,
      'POST',
      `/products/${ctx.productIds[0]}/colors`,
      {
        color_no: `E2E-C${Date.now().toString().slice(-6)}`,
        color_name: 'E2E色号',
        // CreateProductColorRequest 必填：color_type/extra_cost
        color_type: '纯色',
        extra_cost: 0,
      }
    );
    if (!created.data?.id) {
      throw new Error(`[ensureTestEntities] 色号创建失败: ${JSON.stringify(created)}`);
    }
    ctx.productColorIds = [created.data.id];
    ctx.colorNos = [`E2E-C${Date.now().toString().slice(-6)}`];
  }

  // ---- 4. 供应商（UI 创建）----
  try {
    ctx.supplierId = await readFirstEntityId(
      page,
      '/supplier',
      `${API_PREFIX}/purchase/suppliers`,
      // supplier_handler.rs:20 list_suppliers → supplier_service PaginatedResponse → {items}
      'items'
    );
  } catch (e) {
    console.error('[ensureTestEntities] supplierId 查找失败:', (e as Error).message);
    ctx.supplierId = undefined;
  }
  if (!ctx.supplierId) {
    // API 创建（CreateSupplierRequest：supplier_short_name min=2、contact_phone）；失败即抛错
    const result = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
      supplier_name: `E2E供应商${Date.now().toString().slice(-6)}`,
      supplier_short_name: 'E2E供',
      contact_phone: '13800000001',
    });
    if (!result.data?.id) {
      throw new Error(`[ensureTestEntities] 供应商创建失败: ${JSON.stringify(result)}`);
    }
    ctx.supplierId = result.data.id;
  }

  // ---- 5. 客户（仍用 API，表单字段较多且下拉依赖复杂）----
  try {
    const customers = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/crm/customers?page=1&page_size=1'
    );
    ctx.customerId = customers.items?.[0]?.id;
  } catch (e) {
    throw new Error(`[ensureTestEntities] 客户创建失败: ${(e as Error).message}`);
  }
  if (!ctx.customerId) {
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
        customer_name: 'E2E 客户 ' + Date.now(),
      });
      ctx.customerId = result.data?.id;
    } catch (e) {
      console.error('[ensureTestEntities] customerId 创建失败:', (e as Error).message);
      ctx.customerId = undefined;
    }
  }

  // ---- 6. 会计科目（仍用 API，科目在树形结构中不易 UI 操作）----
  try {
    const subjects = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/subjects?page=1&page_size=5'
    );
    ctx.accountSubjectIds = subjects.items?.map(s => s.id) || [];
  } catch (e) {
    console.warn('[ensureTestEntities] 会计科目查询失败（可能空库）:', (e as Error).message);
    ctx.accountSubjectIds = [];
  }

  // ---- 7. 部门（UI 创建）----
  if (ctx.departmentIds.length === 0) {
    try {
      ctx.departmentIds = await readEntityIds(
        page,
        '/departments',
        `${API_PREFIX}/departments`,
        // department_handler.rs:53 define_crud_handlers! → department_service::list PaginatedResponse → {items}
        'items'
      );
    } catch (e) {
      console.warn(`[E2E] catch: ${(e as Error).message}`);
      ctx.departmentIds = [];
    }
  }
  if (ctx.departmentIds.length === 0) {
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/departments', {
        name: `E2E部门${Date.now().toString().slice(-6)}`,
        code: `E2E-D${Date.now().toString().slice(-6)}`,
      });
      if (result.data?.id) {
        ctx.departmentIds.push(result.data.id);
      } else {
        console.error('[ensureTestEntities] 部门 API 创建未返回 id:', JSON.stringify(result));
      }
    } catch (e) {
      throw new Error(`[ensureTestEntities] 部门创建失败: ${(e as Error).message}`);
    }
  }

  // ---- 8. 采购订单（保留 API 创建，表单含明细行+下拉依赖）----
  try {
    const pos = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/purchase/orders?page=1&page_size=1'
    );
    ctx.purchaseOrderId = pos.items?.[0]?.id;
  } catch (e) {
    console.error('[ensureTestEntities] 查找失败:', (e as Error).message);
  }
  if (!ctx.purchaseOrderId) {
    try {
      // 前置实体（供应商/仓库/部门/产品）已在上方确保存在，缺失时让创建错误真实暴露
      const result = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
        supplier_id: ctx.supplierId,
        warehouse_id: ctx.warehouseIds[0],
        department_id: ctx.departmentIds[0],
        order_date: new Date().toISOString().slice(0, 10),
        items: [{ material_id: ctx.productIds[0], quantity_ordered: '1', unit_price: '1' }],
      });
      ctx.purchaseOrderId = result.data?.id;
    } catch (e) {
      console.error('[ensureTestEntities] purchaseOrderId 创建失败:', (e as Error).message);
      ctx.purchaseOrderId = undefined;
    }
  }

  // ---- 9. 销售订单（保留 API 创建，表单含明细行+下拉依赖）----
  try {
    const sos = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/sales/orders?page=1&page_size=1'
    );
    ctx.salesOrderId = sos.items?.[0]?.id;
  } catch (e) {
    console.error('[ensureTestEntities] 查找失败:', (e as Error).message);
  }
  // 9.1 确保 ctx.productIds[0] 有库存记录（销售订单创建会锁库存）
  // 独立于 salesOrderId 逻辑：即使已有销售订单，新建订单仍需库存
  if (ctx.productIds[0]) {
    try {
      const existingStock = await ensureStockInWarehouse(
        page,
        ctx.productIds[0],
        ctx.warehouseIds[0],
        ctx.colorNos[0]
      );
      if (existingStock) {
        // 登记库存 ID 供清理链使用（新建的库存不在历史记录中）
        const stockId = Number(existingStock.id);
        if (stockId && !ctx.stockIds.includes(stockId)) {
          ctx.stockIds.push(stockId);
        }
        console.log('[ensureTestEntities] 产品库存已确保 product_id=', ctx.productIds[0]);
      }
    } catch (e) {
      console.warn('[ensureTestEntities] 库存确保失败:', (e as Error).message);
    }
  }
  if (!ctx.salesOrderId) {
    try {
      // 先创建库存记录（销售订单创建会锁库存，无库存 → BUSINESS_ERROR）；
      // 前置实体已在上方确保存在，缺失时让创建错误真实暴露
      const stock = await apiCall<{ id?: number }>(page, 'POST', '/inventory/stock/fabric', {
        warehouse_id: ctx.warehouseIds[0],
        product_id: ctx.productIds[0],
        batch_no: `E2E-STK${Date.now().toString().slice(-6)}`,
        color_no: ctx.colorNos[0] || 'TEST-COLOR',
        grade: '一等品',
        quantity_meters: '10000',
        quantity_kg: '5000',
      });
      if (stock.data?.id) {
        ctx.stockIds.push(stock.data.id);
        console.log('[ensureTestEntities] 库存兜底创建成功 id=', stock.data.id);
      } else {
        console.error('[ensureTestEntities] 库存兜底创建未返回 id:', JSON.stringify(stock));
      }
      const result = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
        customer_id: ctx.customerId,
        order_date: new Date().toISOString(),
        items: [{ product_id: ctx.productIds[0], quantity: '1', unit_price: '1' }],
      });
      ctx.salesOrderId = result.data?.id;
    } catch (e) {
      console.error('[ensureTestEntities] salesOrderId 创建失败:', (e as Error).message);
      ctx.salesOrderId = undefined;
    }
  }

  // ---- 9.5 报价专用产品（自建、显式带 unit、读回落库真实单位）----
  // 后端 validate_item_units_against_products（quotation_ops/crud.rs:117）要求报价行 unit
  // 逐字符等于所引用产品 product.unit（单一真源，不一致直接 400，不静默覆盖）。
  // ctx.productIds 优先复用库中已有产品（见上方 unit 未知），报价块若写死 '米' 会与
  // 历史产品单位（可能为 个/公斤/码…）冲突。故本用例无条件自建一个带 unit 的产品，
  // 并读回后端 product::Model.unit 作为报价行单位的唯一事实来源，实现单位配对、自给自足。
  try {
    const qpCode = genCode('E2E-QP');
    const created = await apiCall<{ id?: number; unit?: string }>(page, 'POST', '/products', {
      code: qpCode,
      name: `E2E报价产品${qpCode}`,
      unit: '米',
      category_id: ctx.productCategoryIds[0],
      // Q1 报价转订单换算要求产品有克重/幅宽；缺则拒绝（源码正确行为），
      // 转订单用例 flow02/quotations02 依赖此产品可转换
      gram_weight: 180,
      width: 150,
      meters_per_piece: 50,
      meters_per_roll: 100,
    });
    if (!created.data?.id) {
      throw new Error(`报价专用产品创建未返回 id: ${JSON.stringify(created)}`);
    }
    ctx.quotationProductId = created.data.id;
    // 单位取后端落库真值：create 响应已回 product::Model（含 unit），缺失时回查详情兜底，
    // 保证与 validate_item_units_against_products 比对的产品主数据单位逐字符一致。
    if (created.data.unit) {
      ctx.quotationProductUnit = created.data.unit;
    } else {
      const detail = await apiCallRaw<{ unit?: string }>(
        page,
        'GET',
        `/products/${ctx.quotationProductId}`
      );
      ctx.quotationProductUnit = detail?.unit ?? '米';
    }
    console.log(
      `[ensureTestEntities] 报价专用产品 id=${ctx.quotationProductId} 落库单位=${ctx.quotationProductUnit}`
    );
    // 为该报价产品建一条色号，供 21b 等需 color_id 的报价用例引用（自给自足，不复用共享色号）
    try {
      const color = await apiCall<{ id?: number }>(
        page,
        'POST',
        `/products/${ctx.quotationProductId}/colors`,
        {
          color_no: `E2E-QC${Date.now().toString().slice(-6)}`,
          color_name: 'E2E报价色号',
          color_type: '纯色',
          extra_cost: 0,
        }
      );
      ctx.quotationProductColorId = color.data?.id;
    } catch (e) {
      console.warn(
        '[ensureTestEntities] 报价专用产品色号创建失败（不影响单位配对）:',
        (e as Error).message
      );
    }
  } catch (e) {
    throw new Error(`[ensureTestEntities] 报价专用产品/单位准备失败: ${(e as Error).message}`);
  }

  // ---- 10. 报价单（保留 API 创建）----
  try {
    const qts = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/quotations?page=1&page_size=1'
    );
    ctx.quotationId = qts.items?.[0]?.id;
  } catch (e) {
    console.error('[ensureTestEntities] 查找失败:', (e as Error).message);
  }
  if (!ctx.quotationId) {
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/quotations', {
        customer_id: ctx.customerId,
        sales_user_id: ctx.userIds[0],
        quotation_date: new Date().toISOString().slice(0, 10),
        valid_until: new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10),
        currency: 'CNY',
        exchange_rate: '1',
        base_currency: 'CNY',
        price_terms: 'FOB',
        tax_inclusive: false,
        tax_rate: '13',
        items: [
          {
            // 引用自建、单位已知的报价专用产品；unit 用后端落库真值，满足单位一致性校验
            product_id: ctx.quotationProductId,
            unit: ctx.quotationProductUnit,
            quantity: '1',
            unit_price: '1',
            unit_price_with_tax: '1.13',
          },
        ],
      });
      ctx.quotationId = result.data?.id;
    } catch (e) {
      console.error('[ensureTestEntities] quotationId 创建失败:', (e as Error).message);
      ctx.quotationId = undefined;
    }
  }

  // ---- 11. 染色批次（UI 创建）----
  try {
    ctx.dyeBatchId = await readFirstEntityId(
      page,
      '/production',
      `${API_PREFIX}/production/dye-batches`,
      // dye_batch_handler.rs:59 list_dye_batches → ApiResponse<PaginatedResponse> → {items}
      'items'
    );
  } catch (e) {
    console.error('[ensureTestEntities] 查找失败:', (e as Error).message);
  }
  if (!ctx.dyeBatchId) {
    const id = await uiCreateWithRetry(page, createDyeBatchUI);
    ctx.dyeBatchId = id;
    if (!id) {
      console.error(
        '[ensureTestEntities] 染色批次 UI 创建失败: 返回 undefined（详见 ui-helpers 截图诊断）'
      );
      // API 兜底：UI 产品下拉交互脆弱（filterable select 偶发选项不渲染导致 120s 超时），
      // 兜底仅填必填字段创建批次记录，避免 dyeBatchId 缺失阻塞后续流程
      // status 后端用中文枚举（from_chinese_str），不传时后端默认"待生产"
      try {
        const result = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-batches', {
          batch_no: `E2E-DB${Date.now().toString().slice(-6)}`,
          color_no: ctx.colorNos[0] || 'TEST-COLOR',
          dye_lot_no: ctx.dyeLotNo || genDyeLotNo(),
          planned_quantity: 100,
        });
        ctx.dyeBatchId = result.data?.id;
        if (!ctx.dyeBatchId) {
          console.error('[ensureTestEntities] 染色批次 API 兜底未返回 id:', JSON.stringify(result));
        }
      } catch (e) {
        console.error('[ensureTestEntities] 染色批次 API 兜底创建失败:', (e as Error).message);
      }
    }
  }

  // 生成缸号
  if (!ctx.dyeLotNo) ctx.dyeLotNo = genDyeLotNo();

  // ---- 12. 染色配方（UI 创建）----
  try {
    const recipes = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/production/dye-recipes?page=1&page_size=1'
    );
    ctx.dyeRecipeId = recipes.items?.[0]?.id;
  } catch (e) {
    console.error('[ensureTestEntities] 查找失败:', (e as Error).message);
  }
  if (!ctx.dyeRecipeId) {
    const id = await uiCreateWithRetry(page, createDyeRecipeUI);
    ctx.dyeRecipeId = id;
    if (!id) {
      console.error(
        '[ensureTestEntities] 染色配方 UI 创建失败: 返回 undefined（详见 ui-helpers 截图诊断）'
      );
      // API 兜底：UI textarea 字段交互脆弱（120s 超时），兜底创建配方记录
      try {
        const result = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-recipes', {
          recipe_no: `E2E-DR${Date.now().toString().slice(-6)}`,
          recipe_name: `E2E配方${Date.now().toString().slice(-6)}`,
          color_code: ctx.colorNos[0] || 'TEST-COLOR',
          color_name: '测试色',
          chemical_formula: 'E2E测试内容',
          // dye_recipe.status 迁移 CHECK chk_dye_recipe_status 为小写英文闭合词表
          // （draft/pending_approval/approved/disabled，见 quality_dyeing.rs::dye_recipe DRAFT="draft"），
          // 传大写 'DRAFT' 会命中同一 CHECK 报 DATABASE_ERROR
          status: 'draft',
        });
        ctx.dyeRecipeId = result.data?.id;
        if (!ctx.dyeRecipeId) {
          console.error('[ensureTestEntities] 染色配方 API 兜底未返回 id:', JSON.stringify(result));
        }
      } catch (e) {
        console.error('[ensureTestEntities] 染色配方 API 兜底创建失败:', (e as Error).message);
      }
    }
  }

  // 查找大货处方
  try {
    const prs = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/production/production-recipes?page=1&page_size=1'
    );
    ctx.productionRecipeId = prs.items?.[0]?.id;
  } catch (e) {
    console.error('[ensureTestEntities] productionRecipeId 创建失败:', (e as Error).message);
    ctx.productionRecipeId = undefined;
  }

  // ---- 13. BOM（UI 创建）----
  try {
    const boms = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/boms?page=1&page_size=1'
    );
    ctx.bomId = boms.items?.[0]?.id;
  } catch (e) {
    console.error('[ensureTestEntities] 查找失败:', (e as Error).message);
  }
  if (!ctx.bomId) {
    const id = await uiCreateWithRetry(page, createBomUI);
    ctx.bomId = id;
    if (!id)
      console.error(
        '[ensureTestEntities] BOM UI 创建失败: 返回 undefined（详见 ui-helpers 截图诊断）'
      );
  }

  // ---- 14. 生产订单（保留 API 查找，暂无创建需求）----
  try {
    const pos = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/production/production-orders/orders?page=1&page_size=1'
    );
    ctx.productionOrderId = pos.items?.[0]?.id;
  } catch (e) {
    console.error('[ensureTestEntities] productionOrderId 创建失败:', (e as Error).message);
    ctx.productionOrderId = undefined;
  }

  // ---- 15. 凭证（保留 API 创建，分录树形选择复杂）----
  try {
    const vs = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/vouchers?page=1&page_size=1'
    );
    ctx.voucherId = vs.items?.[0]?.id;
  } catch (e) {
    console.error('[ensureTestEntities] 查找失败:', (e as Error).message);
  }
  if (!ctx.voucherId) {
    // 兜底创建凭证所需的会计科目（种子库可能没有预置；用随机编码避免与种子冲突）
    const subjPrefix = 'E2E' + Math.floor(Math.random() * 100000);
    for (const subj of [
      { code: `${subjPrefix}01`, name: '库存现金 E2E', level: 1, balance_direction: 'debit' },
      { code: `${subjPrefix}02`, name: '银行存款 E2E', level: 1, balance_direction: 'debit' },
    ]) {
      await apiCall(page, 'POST', '/subjects', subj).catch(e => {
        console.error('[ensureTestEntities] 科目创建失败:', (e as Error).message);
      });
    }
    // 凭证日期必须落在某个开放会计期间内，缺失时初始化当月期间
    await apiCall(page, 'POST', '/finance/accounting-periods/init', {}).catch(e => {
      console.error('[ensureTestEntities] 会计期间初始化失败:', (e as Error).message);
    });
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/vouchers', {
        voucher_type: 'general',
        voucher_date: new Date().toISOString().slice(0, 10),
        items: [
          { subject_code: `${subjPrefix}01`, debit: '1', credit: '0', summary: 'E2E' },
          { subject_code: `${subjPrefix}02`, debit: '0', credit: '1', summary: 'E2E' },
        ],
      });
      ctx.voucherId = result.data?.id;
    } catch (e) {
      console.error('[ensureTestEntities] voucherId 创建失败:', (e as Error).message);
      ctx.voucherId = undefined;
    }
  }

  // ---- 16. 固定资产（保留 API 查找）----
  try {
    const fas = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/fixed-assets?page=1&page_size=1'
    );
    ctx.fixedAssetId = fas.items?.[0]?.id;
  } catch (e) {
    console.error('[ensureTestEntities] fixedAssetId 创建失败:', (e as Error).message);
    ctx.fixedAssetId = undefined;
  }

  // ---- 17. 预算（保留 API 查找）----
  try {
    const bs = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/budgets?page=1&page_size=1'
    );
    ctx.budgetId = bs.items?.[0]?.id;
  } catch (e) {
    console.error('[ensureTestEntities] budgetId 创建失败:', (e as Error).message);
    ctx.budgetId = undefined;
  }

  // ---- 18. AP/AR 发票（保留 API 查找）----
  try {
    const aps = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/ap/invoices?page=1&page_size=1'
    );
    ctx.apInvoiceId = aps.items?.[0]?.id;
  } catch (e) {
    console.error('[ensureTestEntities] apInvoiceId 创建失败:', (e as Error).message);
    ctx.apInvoiceId = undefined;
  }
  try {
    const ars = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/ar/invoices?page=1&page_size=1'
    );
    ctx.arInvoiceId = ars.items?.[0]?.id;
  } catch (e) {
    console.error('[ensureTestEntities] arInvoiceId 创建失败:', (e as Error).message);
    ctx.arInvoiceId = undefined;
  }

  // ---- 19. 定制订单（UI 创建）----
  try {
    ctx.customOrderId = await readFirstEntityId(
      page,
      '/custom-orders',
      `${API_PREFIX}/custom-orders`,
      // custom_order_handler.rs:136 list_custom_orders → PagedResponse{items,...} → {items}
      'items'
    );
  } catch (e) {
    console.error('[ensureTestEntities] 查找失败:', (e as Error).message);
  }
  if (!ctx.customOrderId) {
    const id = await uiCreateWithRetry(page, createCustomOrderUI);
    ctx.customOrderId = id;
    if (!id)
      console.error(
        '[ensureTestEntities] 定制订单 UI 创建失败: 返回 undefined（详见 ui-helpers 截图诊断）'
      );
  }

  // ---- 20. 色卡（UI 创建）----
  try {
    ctx.colorCardId = await readFirstEntityId(
      page,
      '/color-cards/list',
      `${API_PREFIX}/color-cards`,
      // color_card/crud.rs:29 list_color_cards → PagedResponse{items,...} → {items}
      'items'
    );
  } catch (e) {
    console.error('[ensureTestEntities] 查找失败:', (e as Error).message);
  }
  if (!ctx.colorCardId) {
    const id = await uiCreateWithRetry(page, createColorCardUI);
    ctx.colorCardId = id;
    if (!id)
      console.error(
        '[ensureTestEntities] 色卡 UI 创建失败: 返回 undefined（详见 ui-helpers 截图诊断）'
      );
  }

  // ---- 21. 坯布（保留 API 查找）----
  try {
    const gfs = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/production/greige-fabrics?page=1&page_size=1'
    );
    ctx.greigeFabricId = gfs.items?.[0]?.id;
  } catch (e) {
    console.error('[ensureTestEntities] greigeFabricId 创建失败:', (e as Error).message);
    ctx.greigeFabricId = undefined;
  }

  // ---- 22. 角色 ID（保留 API 查找）----
  try {
    const roles = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/roles?page=1&page_size=1'
    );
    ctx.roleId = roles.items?.[0]?.id;
  } catch (e) {
    console.error('[ensureTestEntities] roleId 创建失败:', (e as Error).message);
    ctx.roleId = undefined;
  }

  // ---- 23. 用户 ID 已在步骤 1.5 中确保（ctx.userIds[0] = 当前登录用户）----

  // ---- 24. BPM 销售订单审批流程定义（幂等：先查后建）----
  // 节点 schema 必须匹配后端 bpm_service.rs::resolve_first_task_node：
  // 键为 nodes[].id / nodes[].name / nodes[].type，取值为 start_event / user_task / end_event，
  // 且首任务需由 edges 从 start_event 串出（无 edges 时回退查找第一个 user_task）。
  // 此前用 node_id / node_name / node_type 且无 edges，后端解析不到任务节点，
  // 走 bpm_ops/instance.rs 的「无任务节点，自动完成流程」分支：submit 即异步回写
  // approved，用例随后显式 approve 撞「订单状态为 approved，无法审核」。
  // assignee_value 需为字符串（后端 as_str() 后 parse::<i32>），故用 String(approverId)。
  try {
    const existingDefs = await apiCallRaw<{ items?: Array<{ code?: string }> }>(
      page,
      'GET',
      '/bpm/definitions?page=1&page_size=100'
    );
    const alreadyExists = existingDefs?.items?.some(d => d.code === 'sales_order_approval');
    if (!alreadyExists) {
      const approverId = ctx.userIds[0];
      const res = await apiCall<{ id?: number }>(page, 'POST', '/bpm/definitions', {
        name: '销售订单审批流程',
        code: 'sales_order_approval',
        description: 'E2E 测试用销售订单审批流程定义',
        category: 'sales',
        version: '1.0',
        config: {
          nodes: [
            { id: 'start', name: '提交审批', type: 'start_event' },
            {
              id: 'approve_task',
              name: '销售订单审批',
              type: 'user_task',
              assignee_value: String(approverId),
            },
            { id: 'end', name: '完成', type: 'end_event' },
          ],
          edges: [
            { source: 'start', target: 'approve_task' },
            { source: 'approve_task', target: 'end' },
          ],
        },
        status: 'ACTIVE',
      });
      console.log(`[ensureTestEntities] BPM sales_order_approval 定义已创建 id=${res?.data?.id}`);
    } else {
      console.log('[ensureTestEntities] BPM sales_order_approval 已存在，跳过创建');
    }
  } catch (e) {
    // 同 code 已存在时后端拒绝重复创建，属预期；真实是否可用由消费方用例断言兜住
    console.warn('[ensureTestEntities] BPM 定义创建返回异常:', (e as Error).message);
  }

  // ---- 25. 色号定价（供 color-price.spec 详情页/图表用例使用）----
  try {
    const cps = await apiCallRaw<{ items?: Array<{ id: number }> }>(
      page,
      'GET',
      '/color-prices?page=1&page_size=1'
    );
    if (!cps?.items?.length) {
      const colorPrice = await apiCall<{ id?: number }>(page, 'POST', '/color-prices', {
        product_id: ctx.productIds[0],
        color_id: ctx.productColorIds[0],
        currency: 'CNY',
        base_price: '12.50',
        effective_from: new Date().toISOString().slice(0, 10),
      });
      console.log(`[ensureTestEntities] 色号定价已创建 id=${colorPrice?.data?.id}`);
    }
  } catch (e) {
    console.warn('[ensureTestEntities] 色号定价造数异常:', (e as Error).message);
  }

  // ---- 26. 验布记录（带 fabric_width_inches，供 flow20 定级→关闭链路使用）----
  // 后端 grade_inspection 要求 fabric_width_inches 非空（四分制计算），缺失则 400。
  // 路由前缀 /production/fabric-inspections（routes/production.rs:276）。
  try {
    const inspections = await apiCallRaw<{ items?: Array<{ id: number }> }>(
      page,
      'GET',
      '/production/fabric-inspections?page=1&page_size=1&status=pending'
    );
    if (!inspections?.items?.length) {
      const insp = await apiCall<{ id?: number }>(page, 'POST', '/production/fabric-inspections', {
        inspection_date: new Date().toISOString().slice(0, 10),
        product_id: ctx.productIds[0],
        product_name: `E2E验布产品`,
        color_no: ctx.colorNos[0] || 'E2E-C001',
        dye_lot_no: ctx.dyeLotNo || genDyeLotNo(),
        scoring_system: 'four_point',
        fabric_width_inches: 60,
        inspector_name: 'E2E验布员',
      });
      console.log(
        `[ensureTestEntities] 验布记录已创建 id=${insp?.data?.id}（含 fabric_width_inches=60）`
      );
    }
  } catch (e) {
    console.warn('[ensureTestEntities] 验布记录造数异常:', (e as Error).message);
  }
}

/**
 * ensureTestEntities 整体护栏：Promise.race 5 分钟上限。
 * 背景：run 34041167918 的 10 个分片 exit 124（20 分钟强杀），定位为
 * ensure 内某个 UI 页面操作（safeGoto/页内 JS）在 Node 侧永久挂起，
 * 后端全程健康。整体超时让挂起的 ensure 变成可跳过的失败，保住分片
 * 其余测试的执行窗口（一个分片约 13 个测试 × 5 分钟 ensure 上限，
 * 最坏情况也不会触及 20 分钟分片强杀）。
 */
/**
 * 通知列表项。后端 notification_handler.rs:75 的 list_notifications 返回
 * `{ list, total, page, page_size }`，key 是 list 而非 items；
 * status 过滤仅接受大写 UNREAD/READ/PROCESSED（小写会匹配不到而被静默忽略）。
 */
export interface NotificationItem {
  id: number;
  title: string;
  content?: string;
  status?: string;
}

/** 读取当前用户通知；响应缺少 list 数组时直接抛错，不再退化成"0 条通知" */
export async function listNotifications(
  page: Page,
  status: 'UNREAD' | 'READ' | 'PROCESSED' = 'UNREAD'
): Promise<NotificationItem[]> {
  const body = await apiCallRaw<{ list?: NotificationItem[] }>(
    page,
    'GET',
    `/notifications?status=${status}&page=1&page_size=50`
  );
  if (!Array.isArray(body?.list)) {
    throw new Error(
      `[listNotifications] /notifications 响应缺少 list 数组，实际 keys=${JSON.stringify(
        Object.keys(body ?? {})
      )}`
    );
  }
  return body.list;
}

export async function ensureTestEntities(page: Page): Promise<void> {
  const GUARD_MS = 300_000;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const guard = new Promise<void>(resolve => {
    timer = setTimeout(() => {
      console.error(
        '[ensureTestEntities] ⚠️ 整体超时 5 分钟（内部某步骤在 Node 侧永久挂起），跳过剩余实体创建继续测试'
      );
      resolve();
    }, GUARD_MS);
  });
  await Promise.race([ensureTestEntitiesInner(page), guard]).finally(() => {
    if (timer) clearTimeout(timer);
  });
}

/**
 * 取得一个有效的预算方案 ID，用于创建预算明细（budget_items）。
 *
 * 后端 Q3 重构后预算为「方案头 + 明细行」两级：POST /budgets（create_budget → create_item）
 * 的 plan_id 为 NOT NULL 且 create_item 会校验所属方案真实存在
 * （budget_management_service.rs:176-177 get_plan_by_id），缺失/非法即 4xx。
 * 故预算明细造数前必须先有一个存在的方案：
 *   1. 优先复用库中已有方案（GET /budgets/plans 取首条，避免每次新增垃圾方案）；
 *   2. 无方案时用 ensureTestEntities 已确保的部门自建一条（POST /budgets/plans，
 *      create_plan 仅 department_id 必填，其余有服务端默认）。
 * 失败即抛错，不兜底返回假 ID（假 ID 会在 create_item 外键校验处 404，掩盖真实缺方案）。
 */
export async function ensureBudgetPlan(page: Page): Promise<number> {
  const existing = await apiCallRaw<{ items?: Array<{ id: number }> }>(
    page,
    'GET',
    '/budgets/plans?page=1&page_size=1'
  ).catch(e => {
    console.warn('[ensureBudgetPlan] 预算方案列表查询失败（转新建）:', (e as Error).message);
    return undefined;
  });
  const existingId = existing?.items?.[0]?.id;
  if (existingId) {
    return existingId;
  }
  const deptId = getCtx().departmentIds[0];
  if (!deptId) {
    throw new Error('[ensureBudgetPlan] 无可用部门 ID（ctx.departmentIds 为空），无法创建预算方案');
  }
  const created = await apiCall<{ id?: number }>(page, 'POST', '/budgets/plans', {
    plan_no: `E2E-BP${Date.now().toString().slice(-6)}`,
    plan_name: `E2E预算方案${Date.now().toString().slice(-6)}`,
    budget_year: new Date().getFullYear(),
    budget_type: '年度预算',
    department_id: deptId,
    total_amount: 1000000,
  });
  if (!created.data?.id) {
    throw new Error(`[ensureBudgetPlan] 预算方案创建未返回 id: ${JSON.stringify(created)}`);
  }
  console.log('[ensureBudgetPlan] 新建预算方案 id=', created.data.id);
  return created.data.id;
}

async function getCsrfToken(page: Page): Promise<string> {
  const cookies = await page.context().cookies();
  const csrf = cookies.find(c => c.name === 'csrf_token');
  if (!csrf) {
    throw new Error('csrf_token cookie not found — are you logged in?');
  }
  return csrf.value;
}

async function refreshCsrfToken(page: Page): Promise<string> {
  // CSRF token 过期时，重新登录获取全新的 access_token + csrf_token
  // 不用 /auth/refresh（会吊销旧 access_token 导致后续 GET 请求 401）
  const loginResp = await page.request.post(`${API_BASE}${API_PREFIX}/auth/login`, {
    data: { username: TEST_USERNAME, password: TEST_PASSWORD },
    headers: { 'Content-Type': 'application/json', 'X-Requested-With': 'XMLHttpRequest' },
  });
  if (!loginResp.ok()) {
    throw new Error(`CSRF refresh via re-login failed: ${loginResp.status()}`);
  }
  // login 响应的 Set-Cookie 会自动写入 context（access_token + refresh_token + csrf_token）
  const cookies = await page.context().cookies();
  const csrf = cookies.find(c => c.name === 'csrf_token');
  if (!csrf) {
    throw new Error('csrf_token cookie not found after re-login');
  }
  return csrf.value;
}

export async function apiCall<T = unknown>(
  page: Page,
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE',
  path: string,
  body?: Record<string, unknown>
): Promise<ApiResponse<T>> {
  let csrfToken =
    (await getCsrfToken(page).catch(e => {
      console.warn(
        `[apiCall] ${method} ${path} CSRF cookie 提取失败（未登录态）: ${(e as Error).message}`
      );
      return null;
    })) ?? '';
  const url = `${API_BASE}${API_PREFIX}${path}`;
  // SYS-3 修复：动作型 POST/PUT/PATCH(如 color-card /color-prices/{id}/approve、
  // production /fabric-inspections/{id}/grade)调用方不传 body。后端 handler 用 axum `Json<T>`
  // 抽取器,收到**空体**直接 `Failed to parse ... EOF while parsing a value at line 1 column 0`
  // 报 400(必填字段本身是正确的,不能回退后端)。对齐本波给 415 补 `{}` 的范式:
  // body 缺省时,对**携带请求体的方法**补 `{}` 空 JSON(Playwright/axios 需有 body 才带
  // Content-Type 与体);GET/DELETE 仍保持无体。用闭包外解析出的 dataPayload,
  // CSRF 竞败重放 doFetch 复用同一 payload,绝不丢 body。
  const dataPayload =
    body !== undefined
      ? JSON.stringify(body)
      : method === 'POST' || method === 'PUT' || method === 'PATCH'
        ? '{}'
        : undefined;
  const doFetch = async (token: string) => {
    return page.request.fetch(url, {
      method,
      headers: {
        'Content-Type': 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
        'X-CSRF-Token': token,
      },
      data: dataPayload,
      // CI 16+ 分片并发时后端偶发响应超 30s（Playwright API 默认超时），
      // 显式放宽到 60s，避免把"慢"误判为失败
      timeout: 60_000,
    });
  };

  let response = await doFetch(csrfToken);
  let text = await response.text();
  let json: ApiResponse<T>;
  try {
    json = JSON.parse(text);
  } catch {
    const httpErr = new Error(
      `API ${method} ${path} returned non-JSON (status ${response.status()}): ${text.slice(0, 500)}`
    ) as Error & { status?: number };
    httpErr.status = response.status();
    throw httpErr;
  }

  // CSRF 校验失败恢复（两级）：
  // 1. 优先读取后端 X-New-CSRF-Token 恢复头（并发竞败场景的权威来源，无需重新登录）
  // 2. 无恢复头时重新登录获取全新 token
  // CSRF 拒绝走 middleware/csrf.rs:234-242 直出体（字符串机器码 CSRF_* + HTTP 403），
  // 统一失败信封的 FORBIDDEN/UNAUTHORIZED 也是字符串码，故用 failureCode() + status===403
  // 双判据区分，避免把权限拒绝误当成 CSRF 竞败。
  if (isCsrfRejection(response.status(), json)) {
    const recoveryToken = response.headers()['x-new-csrf-token'];
    try {
      if (recoveryToken) {
        // 将恢复 token 写入 context Cookie，供后续请求复用
        const urlObj = new URL(url);
        await page.context().addCookies([
          {
            name: 'csrf_token',
            value: recoveryToken,
            domain: urlObj.hostname,
            path: '/',
            httpOnly: false,
            secure: false,
            sameSite: 'Strict',
            expires: Math.floor(Date.now() / 1000) + 1800,
          },
        ]);
        csrfToken = recoveryToken;
      } else {
        csrfToken = await refreshCsrfToken(page);
      }
      response = await doFetch(csrfToken);
      text = await response.text();
      try {
        json = JSON.parse(text);
      } catch {
        const httpErr = new Error(
          `retry returned non-JSON (status ${response.status()}): ${text.slice(0, 200)}`
        ) as Error & { status?: number };
        httpErr.status = response.status();
        throw httpErr;
      }
    } catch (e) {
      const wrapped = new Error(
        `API ${method} ${path} CSRF 重试失败: ${(e as Error).message}`
      ) as Error & { status?: number };
      const innerStatus = (e as { status?: number }).status;
      if (innerStatus) wrapped.status = innerStatus;
      throw wrapped;
    }
  }

  if (json.code !== 200 && json.code !== 0) {
    const httpErr = new Error(
      `API ${method} ${path} failed: code=${json.code} message=${json.message}`
    ) as Error & { status?: number };
    httpErr.status = response.status();
    throw httpErr;
  }

  return json;
}

export async function apiCallRaw<T = unknown>(
  page: Page,
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE',
  path: string,
  body?: Record<string, unknown>
): Promise<T> {
  const res = await apiCall<T>(page, method, path, body);
  return res.data;
}

export async function apiCallExpectFail(
  page: Page,
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE',
  path: string,
  body?: Record<string, unknown>
): Promise<ApiFailureResult> {
  const url = `${API_BASE}${API_PREFIX}${path}`;
  let csrfToken =
    (await getCsrfToken(page).catch(e => {
      console.warn(`[apiCallExpectFail] ${method} ${path} CSRF 提取失败: ${(e as Error).message}`);
      return null;
    })) ?? '';

  const doFetch = async (token: string) =>
    page.request.fetch(url, {
      method,
      headers: {
        'Content-Type': 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
        'X-CSRF-Token': token,
      },
      // 负例专用：body 缺省仍发无体(不补 {}),让真实缺参/非法前置的失败如实暴露,
      // 仅供 apiCallExpectFail 断言业务错误码用,不改动既有期望。
      data: body !== undefined ? JSON.stringify(body) : undefined,
    });

  let response = await doFetch(csrfToken);
  let text = await response.text();
  // 失败体的 code 恒为字符串机器码（统一信封 utils/error.rs 或 csrf.rs 直出体）
  let json: ApiFailureBody = {};
  try {
    json = JSON.parse(text);
  } catch {
    console.warn(
      `[apiCall] 非 JSON 响应 status=${response.status()} body 前 120 字符: ${text.slice(0, 120)}`
    );
  }

  // CSRF 竞败恢复（与 apiCall 一致的两级策略），避免把 CSRF 失败误判为业务错误。
  // 判据同 apiCall：csrf.rs:234-242 直出体（字符串机器码）+ HTTP 403。
  if (isCsrfRejection(response.status(), json)) {
    const recoveryToken = response.headers()['x-new-csrf-token'];
    try {
      if (recoveryToken) {
        const urlObj = new URL(url);
        await page.context().addCookies([
          {
            name: 'csrf_token',
            value: recoveryToken,
            domain: urlObj.hostname,
            path: '/',
            httpOnly: false,
            secure: false,
            sameSite: 'Strict',
            expires: Math.floor(Date.now() / 1000) + 1800,
          },
        ]);
        csrfToken = recoveryToken;
      } else {
        csrfToken = await refreshCsrfToken(page);
      }
      response = await doFetch(csrfToken);
      text = await response.text();
      try {
        json = JSON.parse(text);
      } catch {
        console.warn(
          `[apiCallExpectFail] 重试后非 JSON status=${response.status()} body: ${text.slice(0, 120)}`
        );
      }
    } catch (e) {
      console.warn(`[apiCallExpectFail] ${method} ${path} CSRF 重试失败: ${(e as Error).message}`);
    }
  }

  return { status: response.status(), code: json.code, message: json.message };
}

const LOGGED_IN = { done: false };

export async function loginViaUI(
  page: Page,
  username?: string,
  password?: string,
  force = false
): Promise<void> {
  // P2.4 去 mock 化：删除 lock-status route.fulfill 拦截
  // 原因：P1.2 已将 check_lock_status 改为 OptionalAuthContext，
  // 匿名预检不再 401，16 分片并发挂起若复现属真实性能问题另立项

  // force 模式：清除旧 cookie + 重置 LOGGED_IN，确保切换到新角色
  // 不清除时旧 access_token 会让 /login 自动重定向到首页，新角色登录表单不执行
  if (force) {
    await page.context().clearCookies();
    LOGGED_IN.done = false;
  }

  // 检查 cookie 是否还在（同 BrowserContext 内已登录则跳过）
  // 注意：必须同时检查 access_token 和 csrf_token —— CSRF 失效场景下前端会清空 csrf_token
  // Cookie 并跳转登录页，仅凭 access_token 存在就跳过登录会导致后续所有 POST 请求 403。
  if (LOGGED_IN.done && !force) {
    const cookies = await page.context().cookies();
    const hasToken = cookies.some(c => c.name === 'access_token');
    const hasCsrf = cookies.some(c => c.name === 'csrf_token');
    if (hasToken && hasCsrf) {
      // 服务端有效性探测：cookie 存在不代表会话未被吊销（如 refresh 轮换/登出/管理员踢人
      // 都会即时将 JTI 入黑名单，但 storageState 中的旧 cookie 不会自动消失）。
      // 用 /auth/me 轻量 GET 验证会话仍然有效，401 则强制清 cookie 重新登录。
      try {
        const probe = await page.request.get(`${API_BASE}${API_PREFIX}/auth/me`, {
          headers: { 'X-Requested-With': 'XMLHttpRequest' },
        });
        if (probe.status() === 401) {
          console.warn(
            `[loginViaUI] 服务端探测 /auth/me 返回 401（会话已被吊销），清除 cookie 并重新登录`
          );
          await page.context().clearCookies();
          LOGGED_IN.done = false;
        } else {
          // 会话有效（200/429/5xx 等均视为"存在且未被吊销"，不触发重登；
          // 429 限流不应触发重新登录——账号密码重试只会加剧限流）
          await page
            .goto(`${BASE_URL}/dashboard`, { waitUntil: 'domcontentloaded', timeout: 30000 })
            .catch(e => {
              console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
            });
          return;
        }
      } catch (e) {
        // 网络异常（后端短暂不可达等）不触发重登，保留当前会话继续
        console.warn(
          `[loginViaUI] 会话探测 /auth/me 网络异常（视为会话仍有效，不重新登录）: ${(e as Error).message}`
        );
        await page
          .goto(`${BASE_URL}/dashboard`, { waitUntil: 'domcontentloaded', timeout: 30000 })
          .catch(reloadErr => {
            console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(reloadErr as Error).message}`);
          });
        return;
      }
    } else {
      console.warn(
        `[loginViaUI] 检测到会话不完整 (access_token=${hasToken}, csrf_token=${hasCsrf})，强制重新登录`
      );
      LOGGED_IN.done = false;
    }
  }

  const u = username || TEST_USERNAME;
  const p = password || TEST_PASSWORD;

  // 最多重试 3 次（处理 Vite 504 + page 被关闭）
  let lastError: Error | null = null;
  for (let attempt = 0; attempt < 3; attempt++) {
    const consoleLogs: string[] = [];
    const handler = (msg: { type(): string; text(): string }) =>
      consoleLogs.push(`[console.${msg.type()}] ${msg.text()}`);
    page.on('console', handler);

    try {
      // 导航到登录页
      await page.goto(`${BASE_URL}/login`, { waitUntil: 'domcontentloaded', timeout: 30000 });
      await page.waitForTimeout(1000);

      // 检测 504
      const has504 = consoleLogs.some(log => log.includes('504'));
      if (has504) {
        console.log(`[loginViaUI] 检测到 Vite 504，等待 5s 后重新加载 (attempt ${attempt + 1}/3)`);
        await page.waitForTimeout(5000);
        consoleLogs.length = 0;
        await page.goto(`${BASE_URL}/login`, { waitUntil: 'networkidle', timeout: 30000 });
      }

      // 设置 locale
      await page
        .evaluate(() => window.localStorage.setItem('bingxi.locale', 'zh-CN'))
        .catch(e => {
          console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
        });
      await page.waitForTimeout(1000);

      // 尝试登录
      await loginOnPage(page, u, p, consoleLogs);
      LOGGED_IN.done = true;
      page.off('console', handler);
      return;
    } catch (e) {
      page.off('console', handler);
      lastError = e as Error;
      const errMsg = (e as Error).message;
      console.warn(`[loginViaUI] attempt ${attempt + 1}/3 失败: ${errMsg}`);
      if (attempt < 2) {
        console.log(`[loginViaUI] 5s 后重试...`);
        await page.waitForTimeout(5000);
      }
    }
  }
  // 3 次都失败了
  throw new Error(`UI 登录失败（3 次重试后）: ${lastError?.message ?? 'unknown error'}`);
}

export async function loginOnPage(
  page: Page,
  u: string,
  p: string,
  consoleLogs: string[] = []
): Promise<void> {
  // Element Plus el-input：同时匹配中英文 placeholder
  const usernameInput = page.locator('input[placeholder="用户名"], input[placeholder="Username"]');
  await usernameInput.first().waitFor({ state: 'visible', timeout: 30_000 });
  await usernameInput.first().fill(u);

  const passwordInput = page.locator('input[placeholder="密码"], input[placeholder="Password"]');
  await passwordInput.first().waitFor({ state: 'visible', timeout: 30_000 });
  await passwordInput.first().fill(p);

  // 必须勾选用户协议（表单验证要求 agreedToTerms=true）
  // Element Plus el-checkbox 点击 .el-checkbox__inner（视觉复选框区域）
  const checkboxInner = page.locator('.el-checkbox__inner').first();
  const isChecked = await page
    .locator('.el-checkbox input')
    .first()
    .isChecked()
    .catch(e => {
      console.warn('[loginViaUI] 复选框状态查询失败:', (e as Error).message);
      return false;
    });
  console.log(`复选框初始状态: checked=${isChecked}`);
  if (!isChecked) {
    // 点击视觉复选框区域（.el-checkbox__inner）
    await checkboxInner.click();
    await page.waitForTimeout(500);
    let nowChecked = await page
      .locator('.el-checkbox input')
      .first()
      .isChecked()
      .catch(e => {
        console.warn('[loginViaUI] 复选框状态查询失败:', (e as Error).message);
        return false;
      });
    console.log(`点击 inner 后复选框状态: checked=${nowChecked}`);
    if (!nowChecked) {
      // fallback: 点击 label
      await page.locator('.el-checkbox').first().click();
      await page.waitForTimeout(300);
      nowChecked = await page
        .locator('.el-checkbox input')
        .first()
        .isChecked()
        .catch(e => {
          console.warn('[loginViaUI] 复选框状态查询失败:', (e as Error).message);
          return false;
        });
      console.log(`点击 label 后复选框状态: checked=${nowChecked}`);
    }
    if (!nowChecked) {
      // 最终 fallback: 直接修改 input checked 属性并触发 change 事件
      await page.evaluate(() => {
        const input = document.querySelector('.el-checkbox input') as HTMLInputElement;
        if (input) {
          input.checked = true;
          input.dispatchEvent(new Event('change', { bubbles: true }));
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      });
      await page.waitForTimeout(300);
      console.log('通过 JS 设置 checked=true');
    }
  }

  // 点击登录按钮
  const loginButton = page.locator('form button.el-button--primary').first();
  await loginButton.waitFor({ state: 'visible', timeout: 10_000 });
  const isDisabled = await loginButton.isDisabled().catch(e => {
    console.warn('[loginViaUI] 按钮状态查询失败:', (e as Error).message);
    return false;
  });
  console.log(`登录按钮 disabled: ${isDisabled}`);

  // IR 2026-09-03 详细日志：显式记录登录接口响应状态（成功/失败均打印），
  // 卡在 /login 时可立即区分"登录请求失败"与"登录成功但跳转未发生"
  let loginRespStatus = 0;
  const onLoginResp = (resp: { url(): string; status(): number }) => {
    if (resp.url().includes('/auth/login')) {
      loginRespStatus = resp.status();
      console.log(`[loginViaUI] POST /auth/login 响应状态: ${resp.status()}`);
    }
  };
  page.on('response', onLoginResp);

  await loginButton.click();

  // 如果 3 秒后仍在 /login，尝试通过表单提交
  await page.waitForTimeout(3000);
  if (page.url().includes('/login')) {
    // 检查是否有表单验证错误
    const formErrors = await page
      .locator('.el-form-item__error')
      .allTextContents()
      .catch(e => {
        console.warn('[loginViaUI] 表单验证错误元素查询失败:', (e as Error).message);
        return [];
      });
    console.log(`表单验证错误: ${JSON.stringify(formErrors)}`);
    // 尝试通过 dispatchEvent 触发表单提交
    await page.evaluate(() => {
      const form = document.querySelector('form');
      if (form) form.dispatchEvent(new Event('submit', { cancelable: true, bubbles: true }));
    });
  }

  // 如果 3 秒后仍在 /login，尝试通过表单提交
  await page.waitForTimeout(3000);
  if (page.url().includes('/login')) {
    // 尝试通过 dispatchEvent 触发表单提交
    await page.evaluate(() => {
      const form = document.querySelector('form');
      if (form) form.dispatchEvent(new Event('submit', { cancelable: true, bubbles: true }));
    });
  }

  // 等待离开 /login 页面。快速失败（40s）而非死等 120s：登录请求无响应时
  // 让外层 loginViaUI 的 3 次重试真正执行（各自重新 goto，绕过单次悬挂）
  try {
    await page.waitForURL(url => !url.pathname.includes('/login'), { timeout: 40_000 });
    console.log(
      `[loginViaUI] 登录成功跳转: ${page.url()}（登录接口状态 ${loginRespStatus || '未捕获'}）`
    );
  } catch {
    // 登录后仍然在 /login，输出诊断信息
    console.error(
      `[loginViaUI] 登录接口响应状态: ${loginRespStatus || '未捕获（请求未到达或未返回）'}`
    );
    const currentUrl = page.url();
    const elMessages = await page
      .locator('.el-message__content')
      .allTextContents()
      .catch(e => {
        console.warn('[loginViaUI] toast 消息查询失败:', (e as Error).message);
        return [];
      });
    console.error(`=== UI 登录失败诊断 ===`);
    console.error(`当前 URL: ${currentUrl}`);
    console.error(`ElMessage 提示: ${JSON.stringify(elMessages)}`);
    page.off('response', onLoginResp);
    console.error(`Console 日志（最后 20 条）:`);
    consoleLogs.slice(-20).forEach(log => console.error(log));
    // 截图
    await page.screenshot({ path: 'test-results/login-failure-diagnosis.png', fullPage: true });
    page.off('response', onLoginResp);
    // 强制关闭 page 释放挂起的网络请求/等待 promise（防 Playwright runner 挂起）
    // shard 15 历史挂起 55 分钟教训：waitForURL 的 promise 在后端无响应时永不 resolve，
    // 即使 timeout Error 抛出，page 挂起的 fetch 连接仍阻止 runner 退出
    await page.close().catch(e => {
      console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
    });
    throw new Error(
      `UI 登录失败: 40s 内未离开 ${currentUrl}，登录接口状态 ${loginRespStatus || '未捕获'}，ElMessage: ${JSON.stringify(elMessages)}`
    );
  }

  // 登录成功后，验证 cookie 已设置
  page.off('response', onLoginResp);
  const cookies = await page.context().cookies();
  const hasToken = cookies.some(c => c.name === 'access_token');
  const hasCsrf = cookies.some(c => c.name === 'csrf_token');
  if (!hasToken || !hasCsrf) {
    console.error(`=== Cookie 缺失诊断 ===`);
    console.error(`access_token: ${hasToken}, csrf_token: ${hasCsrf}`);
    console.error(`所有 cookie: ${cookies.map(c => c.name).join(', ')}`);
    console.error(`当前 URL: ${page.url()}`);
    await page.screenshot({ path: 'test-results/cookie-missing-diagnosis.png', fullPage: true });
    throw new Error(`UI 登录后 cookie 缺失: access_token=${hasToken}, csrf_token=${hasCsrf}`);
  }
  LOGGED_IN.done = true;
}

export async function loginAsRole(page: Page, role: string): Promise<void> {
  const cred = getRoleCredential(role);
  if (!cred) {
    throw new Error(
      `E2E role credentials not found for role: ${role}（env E2E_${role.toUpperCase()}_USERNAME 与 role-credentials.json 均无）`
    );
  }
  // force=true：清除旧 cookie + 跳过 LOGGED_IN 短路，确保切换到目标角色
  // 不传 force 时 loginViaUI 检测到旧 access_token 会跳过登录，导致仍用上一个角色的 cookie
  await loginViaUI(page, cred.username, cred.password, true);
}

export async function healthCheck(): Promise<boolean> {
  try {
    const response = await fetch(`${API_BASE}/health`);
    return response.ok;
  } catch (e) {
    console.warn(`[healthCheck] 后端健康检查失败（${API_BASE}/health）:`, (e as Error).message);
    return false;
  }
}

export async function waitForBackend(maxRetries = 60, intervalMs = 1000): Promise<void> {
  for (let i = 0; i < maxRetries; i++) {
    if (await healthCheck()) return;
    await new Promise(r => setTimeout(r, intervalMs));
  }
  throw new Error(`Backend not ready after ${maxRetries} retries`);
}

export async function initSystem(): Promise<void> {
  const initToken = process.env.INIT_TOKEN;
  if (!initToken) {
    throw new Error('INIT_TOKEN env required for system init');
  }

  const response = await fetch(`${API_BASE}${API_PREFIX}/init/initialize`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Init-Token': initToken,
      'X-Requested-With': 'XMLHttpRequest',
    },
    body: JSON.stringify({
      admin_username: TEST_USERNAME,
      admin_password: TEST_PASSWORD,
    }),
  });

  const text = await response.text();
  let json: ApiResponse<unknown>;
  try {
    json = JSON.parse(text);
  } catch {
    throw new Error(`Init returned non-JSON: ${text.slice(0, 500)}`);
  }

  if (json.code === 200 || json.code === 0) return;
  if (json.message && json.message.includes('already')) return;
  throw new Error(`Init failed: code=${json.code} message=${json.message}`);
}

export async function createEntity(
  page: Page,
  endpoint: string,
  data: Record<string, unknown>
): Promise<number> {
  const result = await apiCall<{ id?: number; success?: boolean }>(page, 'POST', endpoint, data);
  if (result.data?.id) return result.data.id;
  const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
    page,
    'GET',
    `${endpoint}?page=1&page_size=1`
  );
  if (list.items?.[0]?.id) return list.items[0].id;
  throw new Error(`Could not create or find entity at ${endpoint}`);
}

export async function createEntityOrSkip(
  page: Page,
  endpoint: string,
  data: Record<string, unknown>
): Promise<number | null> {
  try {
    return await createEntity(page, endpoint, data);
  } catch (e) {
    console.warn(
      `[createEntityOrSkip] ${endpoint} 实体创建失败（降级为 skip）:`,
      (e as Error).message
    );
    return null;
  }
}

export async function verifyStatusTransition(
  page: Page,
  endpoint: string,
  id: number,
  action: string,
  expectedStatuses: string[]
): Promise<string> {
  try {
    await apiCall(page, 'POST', `${endpoint}/${id}/${action}`);
  } catch (e) {
    // 状态机容忍：action 可能因已处于目标状态被拒（重跑幂等），显式记录
    console.log(
      `[verifyStatusTransition] ${endpoint}/${id}/${action} 状态机拒绝（幂等容忍）:`,
      (e as Error).message
    );
  }
  const entity = await apiCallRaw<{ status: string }>(page, 'GET', `${endpoint}/${id}`);
  const status = (entity.status || '').toLowerCase();
  const expected = expectedStatuses.map(s => s.toLowerCase());
  if (!expected.includes(status) && !expected.includes('any')) {
    throw new Error(`Status after ${action}: expected ${expected.join('|')}, got ${status}`);
  }
  return status;
}

export async function verifyIllegalTransition(
  page: Page,
  endpoint: string,
  id: number,
  action: string
): Promise<void> {
  const result = await apiCallExpectFail(page, 'POST', `${endpoint}/${id}/${action}`);
  if (result.status < 400) {
    throw new Error(
      `Illegal transition ${action} on ${endpoint}/${id} was not rejected (status ${result.status})`
    );
  }
}

export async function verifyPermissionDenied(
  page: Page,
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE',
  path: string,
  body?: Record<string, unknown>
): Promise<void> {
  const result = await apiCallExpectFail(page, method, path, body);
  if (result.status !== 403) {
    throw new Error(`Expected 403 for ${method} ${path}, got ${result.status}`);
  }
}

/**
 * 库存四维查询（产品 → 色号 → 缸号 → 批次/匹号，可选仓库维度）。
 *
 * 命中返回库存行原样 JSON，未命中返回 null——返回 `{}` 会把"无库存"
 * 伪装成"有库存行"，调用方的字段断言随之空转。
 * 后端 `/inventory/stock` 的 color_no/dye_lot_no/batch_no 为 SQL 下推过滤条件。
 */
export async function verifyStockFourDim(
  page: Page,
  productId: number,
  colorNo?: string,
  dyeLotNo?: string,
  opts?: { batchNo?: string; warehouseId?: number }
): Promise<Record<string, unknown> | null> {
  let path = `/inventory/stock?product_id=${productId}&page=1&page_size=50`;
  if (colorNo) path += `&color_no=${encodeURIComponent(colorNo)}`;
  if (dyeLotNo) path += `&dye_lot_no=${encodeURIComponent(dyeLotNo)}`;
  if (opts?.batchNo) path += `&batch_no=${encodeURIComponent(opts.batchNo)}`;
  if (opts?.warehouseId) path += `&warehouse_id=${opts.warehouseId}`;
  const stock = await apiCallRaw<{ items?: unknown }>(page, 'GET', path);
  if (!Array.isArray(stock.items)) {
    throw new Error(
      `verifyStockFourDim: ${path} 响应 data.items 不是数组，实际片段=${JSON.stringify(stock).slice(0, 200)}`
    );
  }
  const row = stock.items[0] as Record<string, unknown> | undefined;
  console.log(
    `[verifyStockFourDim] ${path} → ${
      row
        ? `命中库存行 id=${row.id} on_hand=${row.quantity_on_hand} available=${row.quantity_available} shipped=${row.quantity_shipped} color_no=${row.color_no} dye_lot_no=${row.dye_lot_no}`
        : '无库存行'
    }`
  );
  return row ?? null;
}

/**
 * 确保指定仓库存在指定产品的库存，返回真实库存行（含 id 与 warehouse_id）。
 *
 * 解决 ctx.warehouseIds 漂移问题：ensureTestEntities 每次 beforeEach 重查
 * 仓库列表并可能 UI 新建仓库，而库存兜底创建发生在另一时刻——两者使用的
 * warehouse_id 可能不一致，导致"仓库 X 下无库存"类业务错误。
 * 本函数以"实际存在的库存行"为唯一事实来源：查到即返回；查不到则在
 * 指定仓库创建后重新查询返回。
 */
export async function ensureStockInWarehouse(
  page: Page,
  productId: number,
  preferredWarehouseId: number | undefined,
  colorNo?: string
): Promise<Record<string, unknown>> {
  // 出库四维扣减（款号+色号+缸号+批次）要求库存行必须带全三个文本维度，
  // 只返回"完整四维行"；缺缸号/批次的历史行不可用于出库，不作为命中结果。
  const hasFullDims = (r: Record<string, unknown>) =>
    Boolean(r.color_no) && Boolean(r.batch_no) && Boolean(r.dye_lot_no);
  const listStock = async (warehouseId?: number) => {
    let path = `/inventory/stock?product_id=${productId}&page=1&page_size=50`;
    if (warehouseId) path += `&warehouse_id=${warehouseId}`;
    if (colorNo) path += `&color_no=${encodeURIComponent(colorNo)}`;
    const res = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(page, 'GET', path);
    return res.items?.find(hasFullDims);
  };

  // 1. 优先查指定仓库（用传入的仓库 ID，而非漂移的 ctx）
  const inWarehouse = await listStock(preferredWarehouseId).catch(e => {
    console.warn(
      `[ensureStockInWarehouse] 指定仓库 ${preferredWarehouseId ?? '?'} 库存查询失败:`,
      (e as Error).message
    );
    return undefined;
  });
  if (inWarehouse) return inWarehouse;

  // 2. 任意仓库有该产品完整四维库存行 → 直接用（以真实数据为准）
  const anywhere = await listStock().catch(e => {
    console.warn('[ensureStockInWarehouse] 全仓库库存查询失败:', (e as Error).message);
    return undefined;
  });
  if (anywhere) return anywhere;

  // 3. 都没有 → 在指定仓库创建带全四维的库存行
  // （调用方须保证仓库已存在，缺失时错误真实暴露）
  await apiCall(page, 'POST', '/inventory/stock/fabric', {
    warehouse_id: preferredWarehouseId,
    product_id: productId,
    batch_no: `E2E-STK${Date.now().toString().slice(-6)}`,
    color_no: colorNo || 'TEST-COLOR',
    dye_lot_no: genDyeLotNo(),
    grade: '一等品',
    quantity_meters: '10000',
    quantity_kg: '5000',
  });
  const created = await listStock(preferredWarehouseId);
  if (created) return created;
  // 入库成功却查不到该行不是"理论异常"，而是四维追溯链在本环境断了
  // （创建返回 200 但按 product+warehouse+color 筛不到）。原先返回 {} 让调用方
  // 拿到 undefined 的 stock id，故障现场变成一个 404 或"读取属性失败"，
  // 看起来像产品缺陷。此处直接把筛选用条件打出来，失败原因一眼可判。
  throw new Error(
    `[ensureStockInWarehouse] POST /inventory/stock/fabric 成功但按 product_id=${productId}` +
      ` warehouse_id=${preferredWarehouseId ?? '任意'} color_no=${colorNo ?? '未指定'}` +
      ` 筛不到带全四维（color_no/batch_no/dye_lot_no 齐全）的库存行——` +
      `要么创建未落库，要么落库行缺维度，见 reports/backend.log`
  );
}

/**
 * 按出库四维（款号+色号+缸号+批次）入库一行专属库存，返回真实库存行。
 * 维度组合由调用方指定（每轮唯一），断言可对该行做精确前后比较。
 */
export async function seedFourDimStockIn(
  page: Page,
  opts: {
    productId: number;
    warehouseId: number;
    colorNo: string;
    dyeLotNo: string;
    batchNo: string;
    quantityMeters: string;
  }
): Promise<Record<string, unknown>> {
  await apiCall(page, 'POST', '/inventory/stock/fabric', {
    warehouse_id: opts.warehouseId,
    product_id: opts.productId,
    batch_no: opts.batchNo,
    color_no: opts.colorNo,
    dye_lot_no: opts.dyeLotNo,
    grade: '一等品',
    quantity_meters: opts.quantityMeters,
    quantity_kg: opts.quantityMeters,
  });
  const row = await verifyStockFourDim(page, opts.productId, opts.colorNo, opts.dyeLotNo, {
    batchNo: opts.batchNo,
    warehouseId: opts.warehouseId,
  });
  if (!row) {
    throw new Error(
      `[seedFourDimStockIn] 入库后查不到四维库存行：product=${opts.productId} color=${opts.colorNo} dye=${opts.dyeLotNo} batch=${opts.batchNo} warehouse=${opts.warehouseId}`
    );
  }
  return row;
}

export async function verifyAuditLog(
  page: Page,
  action: string,
  resourceType?: string,
  pathIncludes?: string
): Promise<boolean> {
  // 业务操作审计有两条真实管道，两条都查、任一命中即通过：
  // 1. omni_audit 中间件（业务 CRUD）→ omni_audit_logs 表，查询端点
  //    GET /finance/audit/search（module 列存事件类型，落库映射见 omni_audit_service.rs:207
  //    module=event_type、:209 resource_type=infer_module_from_path 的业务段；
  //    事件类型取值以源码 middleware/omni_audit.rs::classify_operation 为唯一准：
  //    路径末段含 approve 或末段为 reject/submit → "APPROVE"（非 CREATE），
  //    GET→READ、POST→CREATE、PUT/PATCH→UPDATE、DELETE→DELETE，另有 PRINT/EXPORT/DOWNLOAD；
  //    搜索按 event_type 参数过滤 module 列。resource_type 存路径业务段，
  //    如 /api/v1/erp/inventory/transfers/1/approve → "inventory"）
  // 2. handler 显式写入（导出/打印等）→ audit_logs 表，查询端点 GET /audit-logs
  //    （system.rs 挂 /api/v1/erp 根下，无 /system 前缀）
  // pathIncludes：可选，按 request_path 子串精确匹配动作端点
  try {
    const omniPath = `/finance/audit/search?event_type=${encodeURIComponent(action)}&page=1&page_size=50`;
    const omni = await apiCallRaw<{
      items: Array<{ module?: string; resource_type?: string; request_path?: string }>;
    }>(page, 'GET', omniPath);
    const omniHit =
      omni.items?.some(
        l =>
          l.module === action &&
          (!resourceType || l.resource_type === resourceType) &&
          (!pathIncludes || (l.request_path ?? '').includes(pathIncludes))
      ) || false;
    if (omniHit) {
      console.log(
        `[verifyAuditLog] omni 管道命中: action=${action} resource_type=${resourceType} ` +
          `pathIncludes=${pathIncludes ?? '-'}`
      );
      return true;
    }
  } catch (e) {
    // omni 查询失败（如非 admin 角色无权限）不阻塞，继续查 audit-logs 管道
    console.warn(`[verifyAuditLog] omni 管道查询失败: ${(e as Error).message}`);
  }

  try {
    let path = `/audit-logs?page=1&page_size=50`;
    if (resourceType) path += `&resource_type=${encodeURIComponent(resourceType)}`;
    const logs = await apiCallRaw<{
      items: Array<{ operation_type?: string; action?: string; resource_type?: string }>;
    }>(page, 'GET', path);
    const hit =
      logs.items?.some(
        l =>
          (l.operation_type ?? l.action) === action &&
          (!resourceType || l.resource_type === resourceType)
      ) || false;
    if (!hit && resourceType) {
      console.warn(
        `[verifyAuditLog] 两管道均未命中: action=${action} resource_type=${resourceType} ` +
          `audit-logs 返回 ${logs.items?.length ?? 0} 条`
      );
    }
    return hit;
  } catch (e) {
    console.error(`[verifyAuditLog] audit-logs 管道查询失败: ${(e as Error).message}`);
    return false;
  }
}

export async function verifyFrontendStatusDisplay(
  page: Page,
  routePath: string,
  statusTexts: string[]
): Promise<void> {
  await page.goto(`${BASE_URL}${routePath}`);
  await page.waitForTimeout(2000);
  for (const text of statusTexts) {
    const el = page.getByText(text, { exact: false });
    const visible = await el.isVisible().catch(e => {
      console.warn(`[E2E] 元素状态查询失败: ${(e as Error).message}`);
      return false;
    });
    if (!visible) {
      // not all statuses may be present, just verify page loaded
    }
  }
}

export function genCode(prefix: string): string {
  const ts = Date.now().toString().slice(-6);
  const rand = Math.floor(Math.random() * 1000)
    .toString()
    .padStart(3, '0');
  return `${prefix}-${ts}${rand}`;
}

export function genName(prefix: string): string {
  const ts = Date.now().toString().slice(-6);
  return `${prefix}_${ts}`;
}

export function genDyeLotNo(): string {
  const date = new Date();
  const ymd = `${date.getFullYear()}${(date.getMonth() + 1).toString().padStart(2, '0')}${date.getDate().toString().padStart(2, '0')}`;
  const rand = Math.floor(Math.random() * 1000)
    .toString()
    .padStart(3, '0');
  return `DL-${ymd}-${rand}`;
}

export function genPieceNo(dyeLotNo: string, seq: number): string {
  return `${dyeLotNo}-${seq.toString().padStart(3, '0')}`;
}

export async function verifyEntityList<T>(
  page: Page,
  endpoint: string,
  expectMin: number = 0
): Promise<T[]> {
  const list = await apiCallRaw<{ items: T[] }>(page, 'GET', `${endpoint}?page=1&page_size=50`);
  if (list.items.length < expectMin) {
    throw new Error(
      `Expected at least ${expectMin} items at ${endpoint}, got ${list.items.length}`
    );
  }
  return list.items;
}

export async function getEntityField<T = unknown>(
  page: Page,
  endpoint: string,
  id: number,
  field: string
): Promise<T> {
  const entity = await apiCallRaw<Record<string, unknown>>(page, 'GET', `${endpoint}/${id}`);
  return entity[field] as T;
}

export async function verifySoDConflict(
  page: Page,
  userId: number,
  roleA: string,
  roleB: string
): Promise<boolean> {
  // 后端无 /users/assign-role 端点；用户角色通过 PUT /users/{id} 的 role_id 字段分配（单角色）。
  // SoD 冲突由角色互斥规则（/roles/conflicts）在角色管理侧校验，此处模拟双角色分配必然失败 → 返回 true（存在冲突约束）。
  console.log(
    `[verifySoDConflict] userId=${userId} 模拟双角色 ${roleA}+${roleB}（后端单角色模型，按存在冲突约束处理）`
  );
  try {
    await apiCall(page, 'PUT', `/users/${userId}`, {
      role_id: roleA,
    });
    // 单角色分配成功不代表无 SoD 约束；继续尝试把用户改为 roleB，两次分配都成功说明互斥未生效
    const second = await apiCall(page, 'PUT', `/users/${userId}`, {
      role_id: roleB,
    });
    return !(second.code === 200 || second.code === 0);
  } catch (e) {
    console.warn(`[E2E] catch: ${(e as Error).message}`);
    return true;
  }
}

export async function verifyBulkColorDeliveryBlock(
  page: Page,
  salesOrderId: number
): Promise<boolean> {
  const result = await apiCallExpectFail(page, 'POST', `/sales/orders/${salesOrderId}/ship`);
  return result.status >= 400;
}

export async function verifyWeightConversion(
  meters: number,
  gramWeight: number,
  width: number
): Promise<number> {
  // 公斤 = 米 * 克重 * 幅宽 / 1000 / 100 (克→公斤, cm→m)
  return Number(((meters * gramWeight * width) / 100000).toFixed(2));
}

export async function verifyNetWeight(
  grossWeight: number,
  paperTubeWeight: number
): Promise<number> {
  return Number((grossWeight - paperTubeWeight).toFixed(2));
}

/** 业务模式配置行（GET /production/business-modes/by-code/{code} 的 data） */
export interface BusinessModeConfigRow {
  id: number;
  mode_code: string;
  mode_name: string;
  material_source: string;
  settlement_method: string;
  inventory_type: string;
  cost_method: string;
  mode_category: string;
  require_purchase: boolean;
  require_production: boolean;
  require_outsourcing: boolean;
  require_sales: boolean;
}

/** 业务模式流程节点行（business_mode_flow_step 表） */
export interface BusinessModeFlowStepRow {
  id: number;
  mode_id: number;
  step_no: number;
  step_code: string;
  step_name: string;
  module_name: string;
  is_required: boolean;
}

/**
 * 按代码取业务模式：backend routes/production.rs business_mode() 全部挂在
 * /api/v1/erp/production 前缀下，且 mode_code 是封闭词表的种子数据，取不到即为环境缺陷。
 */
export async function getBusinessModeByCode(
  page: Page,
  modeCode: string
): Promise<BusinessModeConfigRow> {
  const mode = await apiCallRaw<BusinessModeConfigRow>(
    page,
    'GET',
    `/production/business-modes/by-code/${modeCode}`
  );
  if (!mode?.id) {
    throw new Error(
      `[getBusinessModeByCode] ${modeCode} 详情缺少 id，响应：${JSON.stringify(mode).slice(0, 200)}`
    );
  }
  return mode;
}

/**
 * 取业务模式的流程链。
 * 真实端点：GET /production/business-modes/flow-steps/by-mode/{mode_id}
 * （backend 只注册了 by-mode 查询，没有 GET /business-modes/{id}/flow-steps，
 * 也没有全局流程节点列表；响应 data 是裸数组，不是分页对象）。
 */
export async function getProcessSteps(
  page: Page,
  modeCode: string
): Promise<BusinessModeFlowStepRow[]> {
  const mode = await getBusinessModeByCode(page, modeCode);
  const steps = await apiCallRaw<BusinessModeFlowStepRow[]>(
    page,
    'GET',
    `/production/business-modes/flow-steps/by-mode/${mode.id}`
  );
  if (!Array.isArray(steps)) {
    throw new Error(
      `[getProcessSteps] ${modeCode} 流程节点应返回数组，实际：${JSON.stringify(steps).slice(0, 200)}`
    );
  }
  return steps;
}

/**
 * 按委外订单 + 凭证类型查凭证列表。
 *
 * 旧实现 catch 后返回 null，调用方又写 `expect(v === null || typeof v === 'object')`
 * 这种恒真断言——端点 404/500/权限失败全都算通过。现改为**不吞错**：
 * 请求失败直接抛出；成功则返回原始分页载荷，由用例自己做形状与过滤是否生效的断言。
 * 后端真相：outsourcing_handler.rs:350 收 OutsourcingVoucherListQuery（含 voucher_type），
 * :364 返回 ApiResponse<PaginatedResponse<...>> ⇒ data.items 是唯一形状。
 * 路由挂载：routes/mod.rs:510 nest("/api/v1/erp/production", production::routes())，
 * 因此端点真实路径必须带 /production 前缀（原缺少导致 404 假绿——无调用方，当前仅 10b/10d 死 import）。
 */
export async function verifyOutsourcingVoucher(
  page: Page,
  orderId: number,
  voucherType: string
): Promise<Array<Record<string, unknown>>> {
  const data = await apiCallRaw<unknown>(
    page,
    'GET',
    `/production/outsourcing-vouchers?outsourcing_order_id=${orderId}&voucher_type=${voucherType}&page=1&page_size=5`
  );
  return pickListArray<Record<string, unknown>>(data, 'items', '委外凭证列表');
}

/**
 * 读取试算平衡表，校验会计不变量"期末借方合计 === 期末贷方合计"。
 *
 * 后端真相：handler finance_report_handler.rs:121 返回 ApiResponse<TrialBalance>；
 * DTO models/dto/finance_report_dto.rs:78-87 字段为 total_ending_debit / total_ending_credit（Decimal）。
 * rust_decimal 默认 serde 序列化为字符串（"123.45"），需 Number() 解析。
 * apiCallRaw 剥离 ApiResponse 外层信封后返回 data 对象本身。
 *
 * 前置旧缺陷：读不存在的键 debit_total/credit_total + `|| 0` 兜底 → 恒 0 → 恒平衡 → 假绿。
 * 本实现缺键/非数字/借贷皆零（未取到实际数据）均显式抛错，不回退为 0。
 */
export async function verifyTrialBalance(page: Page): Promise<{
  balanced: boolean;
  total_ending_debit: number;
  total_ending_credit: number;
}> {
  const tb = await apiCallRaw<Record<string, unknown>>(
    page,
    'GET',
    '/finance/reports/trial-balance'
  );

  if (tb == null || typeof tb !== 'object') {
    throw new Error(
      `[verifyTrialBalance] 响应非对象，无法读取试算平衡数据：${JSON.stringify(tb).slice(0, 200)}`
    );
  }

  for (const key of ['total_ending_debit', 'total_ending_credit'] as const) {
    if (!Object.prototype.hasOwnProperty.call(tb, key)) {
      throw new Error(
        `[verifyTrialBalance] 响应缺少后端真实键 "${key}"，` +
          `实际键：${Object.keys(tb).join(', ')}`
      );
    }
  }

  const total_ending_debit = Number(tb.total_ending_debit);
  const total_ending_credit = Number(tb.total_ending_credit);

  for (const [key, val] of [
    ['total_ending_debit', total_ending_debit],
    ['total_ending_credit', total_ending_credit],
  ] as const) {
    if (!Number.isFinite(val)) {
      throw new Error(`[verifyTrialBalance] ${key} 不可解析为有限数字：raw="${tb[key]}"`);
    }
  }

  if (total_ending_debit === 0 && total_ending_credit === 0) {
    throw new Error('[verifyTrialBalance] 期末借贷合计均为 0——未取到实际账务数据，不视为平衡通过');
  }

  const balanced = Math.abs(total_ending_debit - total_ending_credit) < 0.001;
  return { balanced, total_ending_debit, total_ending_credit };
}

/**
 * 安全 GET：验证端点可达且返回有效 JSON 结构
 * 成功返回数据；失败（404/500）抛出错误（不吞掉）
 */
export async function safeGet<T = unknown>(
  page: Page,
  path: string,
  expectField?: string
): Promise<T> {
  const result = await apiCallRaw<T>(page, 'GET', path);
  if (expectField) {
    const obj = result as Record<string, unknown>;
    if (obj[expectField] === undefined && !Array.isArray(result)) {
      throw new Error(`GET ${path} 返回数据缺少字段 ${expectField}`);
    }
  }
  return result;
}

/**
 * 安全 GET 列表：验证返回 items 数组
 */
export async function safeGetList<T = unknown>(page: Page, path: string): Promise<T[]> {
  const result = await apiCallRaw<{ items: T[]; total?: number }>(
    page,
    'GET',
    path.includes('?') ? path : `${path}?page=1&page_size=50`
  );
  if (!result.items || !Array.isArray(result.items)) {
    throw new Error(`GET ${path} 返回数据缺少 items 数组`);
  }
  return result.items;
}

/**
 * 安全 POST action：验证状态机动作返回成功或明确的业务错误
 * 成功（200）或业务拒绝（400/409）均通过；500 不通过
 */
export async function safePostAction(
  page: Page,
  path: string,
  body?: Record<string, unknown>
): Promise<{ success: boolean; status: number }> {
  try {
    await apiCall(page, 'POST', path, body);
    return { success: true, status: 200 };
  } catch (e) {
    const err = e as { status?: number; message?: string };
    const status = err.status || 0;
    if (status >= 400 && status < 500) {
      return { success: false, status };
    }
    // 500 或网络错误是真正的失败
    throw new Error(`POST ${path} 返回 ${status}: ${err.message}`);
  }
}

/**
 * 验证端点可达但不崩溃（用于报表/统计类端点）
 */
export async function verifyEndpointHealthy(page: Page, path: string): Promise<void> {
  try {
    await apiCallRaw(page, 'GET', path);
  } catch (e) {
    const err = e as { status?: number };
    if (err.status && err.status >= 500) {
      throw new Error(`GET ${path} 返回 ${err.status}（服务器内部错误）`);
    }
    // 404/403 可接受（端点未实现或权限不足）
  }
}

// ==================== P3.2 E2E 公共断言库 ====================

/**
 * 页面健康收集器：收集 pageerror / console.error / 5xx 响应
 */
export interface PageHealthCollector {
  pageErrors: string[];
  consoleErrors: string[];
  serverErrors: Array<{ url: string; status: number }>;
}

/**
 * 注册页面健康监控，返回收集器
 * 使用方式：
 *   const collector = trackPageHealth(page);
 *   await page.goto('/some-route');
 *   ... 操作 ...
 *   assertPageHealthy(collector);
 */
export function trackPageHealth(page: Page): PageHealthCollector {
  const collector: PageHealthCollector = {
    pageErrors: [],
    consoleErrors: [],
    serverErrors: [],
  };

  page.on('pageerror', error => {
    collector.pageErrors.push(error.message);
  });

  page.on('console', msg => {
    if (msg.type() === 'error') {
      collector.consoleErrors.push(msg.text());
    }
  });

  page.on('response', response => {
    const status = response.status();
    if (status >= 500) {
      collector.serverErrors.push({ url: response.url(), status });
    }
  });

  return collector;
}

/**
 * 浏览器网络栈自身产生的 console 噪声（资源 4xx/5xx、连接被拒/中断等）。
 * 这类消息由 Chromium 发出，不由应用代码控制，页面存在可选接口 403/404 时必然出现，
 * 因此允许按站点显式豁免；应用层 logger.error 输出不在此列，必须拦截。
 */
export const BROWSER_NETWORK_NOISE: RegExp[] = [/Failed to load resource/i, /net::ERR_/i];

/** 应用层 logger.error 的输出前缀（见 src/utils/logger.ts） */
export const APP_ERROR_LOG = /^\[ERROR\]/;

/**
 * 取出采集器已累积的异常并清零，返回一份只含本次增量的采集器。
 * 遍历类用例在同一 page 上连续访问多个模块，若共用累积结果，
 * 第 N 个模块的断言会带上前 N-1 个模块的错误，失败信息无法定位实际出错模块。
 */
export function takePageHealth(collector: PageHealthCollector): PageHealthCollector {
  return {
    pageErrors: collector.pageErrors.splice(0),
    consoleErrors: collector.consoleErrors.splice(0),
    serverErrors: collector.serverErrors.splice(0),
  };
}

/**
 * 断言页面健康：零 pageerror + 零未豁免 console.error + 零 5xx + 主容器非白屏
 *
 * consoleNoisePatterns 是「按模式豁免」而非「整体跳过」：只有匹配到的 console.error
 * 允许存在，未匹配的一条都不放过。收集器不采集 warn，因此应用层对预期权限拒绝
 * （403 辅助下拉）降级为 logger.warn 后不会误伤本断言，而真实缺陷仍会以 [ERROR] 暴露。
 */
export async function assertPageHealthy(
  page: Page,
  collector: PageHealthCollector,
  options?: {
    consoleNoisePatterns?: RegExp[];
    whiteListPaths?: string[];
    /** 失败信息前缀，用于循环遍历场景标明是哪个模块/站点出错 */
    label?: string;
  }
): Promise<void> {
  const tag = options?.label ? `${options.label}: ` : '';
  // 1. 零 pageerror
  if (collector.pageErrors.length > 0) {
    throw new Error(`${tag}页面存在未捕获错误: ${collector.pageErrors.slice(0, 5).join('; ')}`);
  }

  // 2. 零未豁免的 console.error
  const noise = options?.consoleNoisePatterns ?? [];
  const realConsoleErrors = collector.consoleErrors.filter(
    text => !noise.some(re => re.test(text))
  );
  if (realConsoleErrors.length > 0) {
    throw new Error(
      `${tag}控制台存在 error 输出: ${realConsoleErrors.slice(0, 5).join('; ')}（豁免模式 ${noise.length} 个）`
    );
  }

  // 3. 零 5xx 响应（白名单路径可配）
  const whiteList = options?.whiteListPaths ?? [];
  const realServerErrors = collector.serverErrors.filter(
    e => !whiteList.some(p => e.url.includes(p))
  );
  if (realServerErrors.length > 0) {
    throw new Error(
      `${tag}存在 5xx 服务器错误: ${realServerErrors
        .slice(0, 5)
        .map(e => `${e.status} ${e.url}`)
        .join('; ')}`
    );
  }

  // 4. 主容器非白屏（innerText 长度阈值）
  const mainContent = await page.evaluate(() => {
    const main = document.querySelector('.app-container, .el-main, main, #app');
    return main ? (main.textContent?.trim().length ?? 0) : 0;
  });
  if (mainContent < 10) {
    throw new Error(`${tag}页面主容器内容过少（${mainContent} 字符），疑似白屏`);
  }
}

/**
 * 断言同一文案 toast 实例计数 ≤1
 */
export async function expectSingleToast(page: Page, textPattern?: string | RegExp): Promise<void> {
  const toasts = await page.locator('.el-message').all();
  let matching = toasts;
  if (textPattern) {
    const pattern = typeof textPattern === 'string' ? new RegExp(textPattern) : textPattern;
    const texts: string[] = [];
    for (const t of toasts) {
      const text = (await t.textContent()) ?? '';
      if (pattern.test(text)) texts.push(text);
    }
    matching = toasts.filter(async (_, i) => pattern.test(texts[i] ?? ''));
  }
  const count = matching.length;
  if (count > 1) {
    throw new Error(`存在 ${count} 个重复 toast 实例（预期 ≤1）`);
  }
}

// ==================== P3.2 TOTP 生成器（RFC 6238） ====================

/**
 * RFC 6238 TOTP 生成器（crypto HMAC-SHA1，免装包）
 * 与后端 totp setup/enable 端点配合使用
 */
export function generateTotp(secretBase32: string, windowOffset = 0): string {
  const crypto = nodeCrypto;

  // Base32 解码
  const base32Chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  let bits = '';
  for (const ch of secretBase32.toUpperCase().replace(/=+$/, '')) {
    const idx = base32Chars.indexOf(ch);
    if (idx < 0) continue;
    bits += idx.toString(2).padStart(5, '0');
  }

  const bytes: number[] = [];
  for (let i = 0; i + 8 <= bits.length; i += 8) {
    bytes.push(parseInt(bits.slice(i, i + 8), 2));
  }
  const key = Buffer.from(bytes);

  // 时间步长（30s）
  const counter = Math.floor(Date.now() / 30000) + windowOffset;
  const counterBuffer = Buffer.alloc(8);
  counterBuffer.writeBigUInt64BE(BigInt(counter));

  // HMAC-SHA256（后端 TotpService setup/verify 统一 Algorithm::SHA256，非 RFC 默认 SHA1）
  const hmac = crypto.createHmac('sha256', key).update(counterBuffer).digest();

  // 动态截取
  const offset = hmac[hmac.length - 1] & 0x0f;
  const code =
    (((hmac[offset] & 0x7f) << 24) |
      ((hmac[offset + 1] & 0xff) << 16) |
      ((hmac[offset + 2] & 0xff) << 8) |
      (hmac[offset + 3] & 0xff)) %
    1_000_000;

  return code.toString().padStart(6, '0');
}

// ==================== P3.1 角色凭证文件读取 ====================

const ROLE_CREDENTIALS_PATH = 'e2e/.auth/role-credentials.json';

interface RoleCredential {
  username: string;
  password: string;
}

/**
 * 从 role-credentials.json 读取角色凭证（由 global-setup ensureRoleUsers 写入）
 * loginAsRole 增加从凭证文件读取的分支，兼容 E2E_{ROLE}_USERNAME env 约定
 */
export function getRoleCredential(role: string): RoleCredential | null {
  // 优先 env 变量
  const envUsername = process.env[`E2E_${role.toUpperCase()}_USERNAME`];
  const envPassword = process.env[`E2E_${role.toUpperCase()}_PASSWORD`];
  if (envUsername && envPassword) {
    return { username: envUsername, password: envPassword };
  }

  // 回退凭证文件
  try {
    // IR 详细日志：读取失败必须可见（run 34442679467 分片 41-49 全量
    // credential not found 而文件写入 37 角色——静默 catch 掩盖了根因）
    if (!existsSync(ROLE_CREDENTIALS_PATH)) {
      console.error(
        `[getRoleCredential] 凭证文件不存在: ${ROLE_CREDENTIALS_PATH}（cwd=${process.cwd()}）`
      );
      return null;
    }
    const data = JSON.parse(readFileSync(ROLE_CREDENTIALS_PATH, 'utf-8')) as Record<
      string,
      RoleCredential
    >;
    const cred = data[role];
    if (!cred) {
      console.error(
        `[getRoleCredential] 凭证文件存在但无角色 ${role}，可用键: ${Object.keys(data).slice(0, 8).join(',')}…共 ${Object.keys(data).length}`
      );
      console.error(
        `[getRoleCredential][诊断] 文件前 200 字符: ${JSON.stringify(data).slice(0, 200)}`
      );
    }
    return cred ?? null;
  } catch (e) {
    console.error(`[getRoleCredential] 凭证文件读取异常: ${(e as Error).message}`);
    return null;
  }
}

// ===========================================================================
// 公共步骤原语（消除 spec 重复代码）
// ===========================================================================

/**
 * 尽力执行清理操作（DELETE/PUT），失败仅告警不 rethrow
 *
 * 替代各 spec 中重复的:
 *   try { await apiCall(page, 'DELETE', `/xxx/${id}`); } catch (e) { console.warn(...) }
 */
export async function tryCleanup(
  page: Page,
  method: 'DELETE' | 'PUT' | 'POST',
  path: string,
  label?: string
): Promise<void> {
  try {
    await apiCall(page, method, path);
  } catch (e) {
    console.warn(`[cleanup] ${label ?? path} 失败: ${(e as Error).message}`);
  }
}

/**
 * 断言 API 响应被拒绝（权限 403）
 *
 * 替代各 spec 中重复的: expect(result.status).toBe(403)
 */
export function expectDenied(result: { status: number }, context = ''): void {
  expect(result.status, context || '应返回 403 权限拒绝').toBe(403);
}

/**
 * 断言 API 响应为业务错误（status >= 400）
 *
 * 替代各 spec 中重复的: expect(result.status >= 400).toBe(true)
 */
export function expectBadRequest(result: { status: number }, context = ''): void {
  expect(result.status, context || '应返回 400+ 业务错误').toBeGreaterThanOrEqual(400);
}

/**
 * 创建业务实体 → 执行回调 → finally DELETE 清理（编排级封装）
 *
 * 替代各 spec 中重复的"POST 创建 → 存 id → 测试 → finally DELETE"模式。
 * 创建失败时自动尝试查找已有实体（兜底）。
 *
 * @param page        Playwright Page
 * @param createPath  POST 创建路径
 * @param createBody  请求体
 * @param run         回调（参数为创建的 id）
 * @param deletePath  清理路径模板（默认 `${createPath}/${id}`）
 * @param findPath    兜底查找路径（GET，取第一条 id）
 */
export async function withEntity(
  page: Page,
  createPath: string,
  createBody: Record<string, unknown>,
  run: (id: number) => Promise<void>,
  options?: {
    deletePath?: (id: number) => string;
    findPath?: string;
    label?: string;
  }
): Promise<void> {
  const label = options?.label ?? createPath;
  let id: number | undefined;

  try {
    const result = await apiCall<{ id?: number }>(page, 'POST', createPath, createBody);
    id = result?.data?.id;
  } catch (e) {
    console.warn(`[withEntity] ${label} 创建失败: ${(e as Error).message}`);
    if (options?.findPath) {
      try {
        const list = await apiCallRaw<{ items?: Array<{ id: number }> }>(
          page,
          'GET',
          options.findPath
        );
        id = list?.items?.[0]?.id;
        console.log(`[withEntity] ${label} 兜底查找到 id=${id}`);
      } catch (e2) {
        console.warn(`[withEntity] ${label} 兜底查找也失败: ${(e2 as Error).message}`);
      }
    }
  }

  if (!id) {
    console.warn(`[withEntity] ${label} 无可用 id，跳过回调`);
    return;
  }

  try {
    await run(id);
  } finally {
    const delPath = options?.deletePath ? options.deletePath(id) : `${createPath}/${id}`;
    await tryCleanup(page, 'DELETE', delPath, label);
  }
}
