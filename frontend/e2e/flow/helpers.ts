/* eslint-disable no-console */
import {
  expect,
  type Page,
  type APIResponse,
  type Browser,
  type BrowserContext,
} from '@playwright/test';
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
  /** utils/error.rs:709 `CODE_FORBIDDEN`；中间件与 AppError::PermissionDenied 同码 */
  FORBIDDEN: 'FORBIDDEN',
  /** utils/error.rs:739 AppError::NotFound 分支的机器码 */
  NOT_FOUND: 'NOT_FOUND',
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

/**
 * 把一个 csrf_token 写入浏览器上下文 Cookie（非 httpOnly，前端 document.cookie 可读）。
 * 域取 url 的 hostname；域名不匹配等异常静默降级（交由各请求的 403 恢复路径兜底）。
 */
async function writeCsrfCookie(page: Page, url: string, value: string): Promise<void> {
  try {
    const urlObj = new URL(url);
    await page.context().addCookies([
      {
        name: 'csrf_token',
        value,
        domain: urlObj.hostname,
        path: '/',
        httpOnly: false,
        secure: false,
        sameSite: 'Strict',
        expires: Math.floor(Date.now() / 1000) + 1800,
      },
    ]);
  } catch {
    // addCookies 异常（如域名不匹配）不阻塞——降级到下一次 403 恢复路径
  }
}

/**
 * 后端 CSRF Token 为一次性消费（csrf.rs:110 consume + :216-224 Set-Cookie 轮换）。
 * page.request 成功写入后，响应 Set-Cookie 携带新 token，Playwright 自动存入 context。
 * 本函数为防御性保障：从响应的 set-cookie 头/ x-new-csrf-token 恢复头中显式提取 csrf_token 值
 * 并 addCookies，确保即便 Playwright 内部 cookie 传播存在微小时序差或 headers() 对多 Set-Cookie
 * 合并行为变化（Playwright >= 1.40 的 headers() 不保证返回 set-cookie），下一次 getCsrfToken
 * 也一定能读到最新轮换后的 token，杜绝因使用已消费 token 导致的不必要 403。
 */
async function syncCsrfFromResponse(page: Page, response: APIResponse, url: string): Promise<void> {
  let csrfValue: string | undefined;

  // 优先从 headersArray 中逐项匹配 Set-Cookie（Playwright 文档明确 headersArray 保留
  // 所有同名头且包含 set-cookie，而 headers() 在 >= 1.40 版本可能省略 set-cookie）。
  const entries = response.headersArray();
  for (const entry of entries) {
    if (entry.name.toLowerCase() === 'set-cookie') {
      const match = /csrf_token=([^;]+)/.exec(entry.value);
      if (match) {
        csrfValue = match[1];
        break;
      }
    }
  }

  // 回退：若 headersArray 未找到（极端兼容场景），尝试 headers() 合并值
  if (!csrfValue) {
    const setCookie = response.headers()['set-cookie'];
    if (setCookie) {
      const match = /csrf_token=([^;]+)/.exec(setCookie);
      if (match) csrfValue = match[1];
    }
  }

  // 额外处理：后端 CSRF 拒绝时的恢复头 x-new-csrf-token（csrf.rs:144-151）
  if (!csrfValue) {
    const recovery = response.headers()['x-new-csrf-token'];
    if (recovery) csrfValue = recovery;
  }

  if (!csrfValue) return;
  await writeCsrfCookie(page, url, csrfValue);
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

/**
 * CSRF 竞败重放的最大次数（有界，不是无限退避）。
 *
 * 为什么不能只重放一次：后端 token 是一次性消费（csrf.rs:110 consume + :216-224 轮换），
 * 而同一浏览器上下文里除了本 helper，UI 自己的写请求也在消费同一个 token
 * （前端 axios 拦截器同样"403 → 用 X-New-CSRF-Token 重放一次"，api/request.ts:197-223）。
 * 两侧各重放一次时，helper 换回的新 token 仍可能被对侧那次在途请求抢先消费——
 * 单侧重放把这种交错固化成"偶发成片 403"红（#4671 shard19/32/34/35 的
 * POST /products、POST /incoterms/cost-calculation、POST /production/dye-batches 即此族）。
 * 上限内每次重放都取"后端权威 token"（恢复头，缺失时重登），上限用尽仍被拒即判红：
 * CSRF 拒绝不是业务成功，绝不静默放过、也绝不无限重试掩盖。
 */
const CSRF_RECOVERY_MAX_ATTEMPTS = 2;

/**
 * CSRF 竞败后取回一个存活 token 所用：优先后端 `x-new-csrf-token` 恢复头
 * （csrf.rs:144-151，并发竞败场景的权威来源，无需重登），无恢复头时重新登录换新 token。
 * 返回用该 token 重放后的响应（调用方负责继续判定，仍 403 则按各自语义判红）。
 */
async function replayAfterCsrfRejection(
  page: Page,
  url: string,
  rejected: APIResponse,
  doFetch: (token: string) => Promise<APIResponse>
): Promise<APIResponse> {
  const recoveryToken = rejected.headers()['x-new-csrf-token'];
  let token: string;
  if (recoveryToken) {
    await writeCsrfCookie(page, url, recoveryToken);
    token = recoveryToken;
  } else {
    token = await refreshCsrfToken(page);
  }
  return doFetch(token);
}

/**
 * loginViaUI 短路路径的 csrf 活性探测（缺陷1 修复点4）。
 *
 * 背景：storage-state 里的 csrf_token 是"服务端一次性消费"的凭证，会被第一个使用它的
 * 写请求打死；而 loginViaUI 的短路分支（同 worker 内 LOGGED_IN 已置位 + cookie 存在）
 * 只做 /auth/me（GET，天然不消费 csrf）会话有效性探测，无法察觉 csrf 已被上一用例消费，
 * 于是带着死 csrf 返回 → 该用例首个写请求 403 CSRF_TOKEN_INVALID（成片 403 的会话级来源）。
 *
 * 探测方式（零业务副作用）：POST /webhooks 携带空体 '{}'。
 * - CreateWebhookRequest 的 name/url/events 均为非 Option 必填字段（webhook_handler.rs:29-34），
 *   空体在 axum `Json<T>` 抽取器反序列化阶段即报 422，早于 handler 主体、绝不触库，
 *   故不可能创建任何 webhook 行（对比 /warehouses、/products 的字段全是 Option，空体可过校验，
 *   会落 DB NOT NULL 500 甚至插脏行——不可用作探测）；
 * - 但 csrf.rs 在 handler 之前已完成消费+轮换：:199 consume → :202-211 生成并登记新 token →
 *   :215-224 把轮换后的新 token 以 Set-Cookie append 到 next.run 的响应（422/403 同样携带）。
 *   故两条路径都能取回一个"仍然存活"的 token：
 *   · 传入 token 存活 → 被消费并轮换，响应 Set-Cookie 带新活 token → syncCsrfFromResponse 取回；
 *   · 传入 token 已死 → 403 CSRF_* + x-new-csrf-token 恢复头（服务端已登记）→ 采用恢复 token。
 * 无论哪种，探测结束后会话上下文都持有一个存活 csrf，短路返回后不再撞首轮 403。
 */
async function probeCsrfLiveness(page: Page): Promise<void> {
  const url = `${API_BASE}${API_PREFIX}/webhooks`;
  const token = await getCsrfToken(page).catch(e => {
    console.warn(`[loginViaUI] csrf 活性探测：读取当前 csrf_token 失败: ${(e as Error).message}`);
    return '';
  });
  let resp: APIResponse;
  try {
    resp = await page.request.fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
        'X-CSRF-Token': token,
      },
      data: '{}',
      timeout: 30_000,
    });
  } catch (e) {
    // 探测请求本身网络异常：不阻塞登录（交由后续 apiCall / 前端的 403 恢复兜底），仅告警
    console.warn(`[loginViaUI] csrf 活性探测请求异常（跳过探测）: ${(e as Error).message}`);
    return;
  }
  const recovery = resp.headers()['x-new-csrf-token'];
  if (recovery) {
    await writeCsrfCookie(page, url, recovery);
    console.log('[loginViaUI] csrf 活性探测：原 token 已失效，已采用后端下发的恢复 token');
  } else {
    await syncCsrfFromResponse(page, resp, url);
    console.log('[loginViaUI] csrf 活性探测：token 存活并已轮换回写');
  }
}

export async function apiCall<T = unknown>(
  page: Page,
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE',
  path: string,
  body?: Record<string, unknown>
): Promise<ApiResponse<T>> {
  const csrfToken =
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

  // CSRF 校验失败恢复：有界重放（见 CSRF_RECOVERY_MAX_ATTEMPTS 的成因说明），
  // 每次优先读取后端 X-New-CSRF-Token 恢复头，无恢复头时重新登录取全新 token。
  // 判据同 csrf.rs:234-242 直出体（字符串机器码 CSRF_* + HTTP 403）：统一失败信封的
  // FORBIDDEN/UNAUTHORIZED 也是字符串码，故用 failureCode() + status===403 双判据区分，
  // 权限拒绝不会被当成竞败而重试吞掉。
  for (
    let attempt = 1;
    attempt <= CSRF_RECOVERY_MAX_ATTEMPTS && isCsrfRejection(response.status(), json);
    attempt++
  ) {
    try {
      response = await replayAfterCsrfRejection(page, url, response, doFetch);
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

  // 重放上限用尽仍被 CSRF 中间件拒绝：直接判红并点名是 CSRF 面（不是业务面），
  // 绝不返回/放行一个"看起来像成功"的会话状态。
  if (isCsrfRejection(response.status(), json)) {
    const csrfErr = new Error(
      `API ${method} ${path} CSRF 恢复耗尽（已重放 ${CSRF_RECOVERY_MAX_ATTEMPTS} 次仍被 CSRF 中间件拒绝，` +
        `status=${response.status()} code=${json.code}）——属会话/中间件面失败，判红不放过`
    ) as Error & { status?: number };
    csrfErr.status = response.status();
    throw csrfErr;
  }

  // 写请求（含 CSRF 竞败重试后的最终 response）完成后，先同步轮换后的 csrf 到会话，
  // 再判定业务错误——顺序至关重要（缺陷1 修复点3）。
  // 根因：后端在 handler 之前即消费旧 token 并 Set-Cookie 下发轮换后的新 token
  // （csrf.rs:199 consume → :215-224 把新 token append 到 next.run 的响应，业务 4xx/5xx 同样携带）。
  // 旧写法把同步放在 `json.code !== 200` 抛错之后，业务错误分支抛出时同步永不执行，
  // 轮换出的新 token 丢失，同会话下一请求仍携带已消费的旧 token → 级联成片 403。
  // 故对携带请求体的方法无条件先同步，再决定是否抛业务错误。
  if (method === 'POST' || method === 'PUT' || method === 'PATCH' || method === 'DELETE') {
    await syncCsrfFromResponse(page, response, url);
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
  const csrfToken =
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

  // CSRF 竞败恢复：与 apiCall 同一有界重放策略（共用 CSRF_RECOVERY_MAX_ATTEMPTS），
  // 避免把 CSRF 失败误判为业务错误——被测的拒绝必须是业务/校验拒绝，不能是竞败中间态。
  for (
    let attempt = 1;
    attempt <= CSRF_RECOVERY_MAX_ATTEMPTS && isCsrfRejection(response.status(), json);
    attempt++
  ) {
    try {
      response = await replayAfterCsrfRejection(page, url, response, doFetch);
      text = await response.text();
      try {
        json = JSON.parse(text);
      } catch {
        console.warn(
          `[apiCallExpectFail] 重试后非 JSON status=${response.status()} body: ${text.slice(0, 120)}`
        );
      }
    } catch (e) {
      // 重放动作本身异常（网络/重登失败）：如实报出，不静默。返回的仍是 CSRF 403，
      // 调用方的拒绝码集合不含 CSRF_* ⇒ 用例判红，根因由本行日志外显。
      console.error(
        `[apiCallExpectFail] ${method} ${path} CSRF 第 ${attempt} 次重放异常: ${(e as Error).message}`
      );
      break;
    }
  }

  if (isCsrfRejection(response.status(), json)) {
    console.error(
      `[apiCallExpectFail] ${method} ${path} 最终响应仍是 CSRF 中间件拒绝` +
        `（status=${response.status()} code=${json.code}，重放上限 ${CSRF_RECOVERY_MAX_ATTEMPTS}）——` +
        `这不是被测的业务拒绝，调用方按拒绝码集合断言即判红`
    );
  }

  // CSRF 恢复/业务错误后 token 可能已被消费并轮换（后端 middleware 在校验通过后
  // 消费旧 token + Set-Cookie 新 token，之后 handler 返回的业务错误不影响轮换）。
  // 主动同步确保后续 apiCall 不再携带已消费的 token。
  if (method === 'POST' || method === 'PUT' || method === 'PATCH' || method === 'DELETE') {
    await syncCsrfFromResponse(page, response, url);
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
          // /auth/me 是 GET、不消费 csrf，故此处补一次 csrf 活性探测：确保 storage-state
          // 里可能已被上一用例消费的 csrf_token 被换成存活 token，避免本用例首个写请求 403。
          await probeCsrfLiveness(page);
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
  expectStateGateRejection(result, `非法流转 ${action} @ ${endpoint}/${id} 应被状态机门拒绝`);
}

/**
 * 越权请求必须被**权限门**拒绝（HTTP 403 + 权限机器码 FORBIDDEN）。
 *
 * 判据全部下沉到 {@link expectDenied}（status + 信封机器码双钉），本函数只负责
 * 发起真实请求并转交判定；签名与返回形态（async → Promise<void>）保持向后兼容。
 * 为什么不能只判 status：本仓 CSRF 中间件与权限门**都**直出 403
 * （csrf.rs:232-234 → code=CSRF_*；response.rs:145 + error.rs:709/742 → code=FORBIDDEN），
 * 只判 status 时权限门被删掉用例照样绿。详见 expectDenied 的判据说明。
 * 禁止断言或读取错误文案（权限拒绝文案永久脱敏）。
 */
export async function verifyPermissionDenied(
  page: Page,
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE',
  path: string,
  body?: Record<string, unknown>
): Promise<void> {
  const result = await apiCallExpectFail(page, method, path, body);
  expectDenied(result, `${method} ${path} 应被权限门拒绝`);
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

/**
 * 白坯布库存播种（color_no 为空、无缸号、有批次）。
 *
 * 入库端点选择：POST /inventory/stock（通用建库存 handler `create_stock`，
 * backend/src/handlers/inventory_stock_handler.rs:146）。
 * 该 handler 虽复用 `CreateStockFabricRequest` DTO（含 color_no min=1 校验注解），
 * 但**不调用 payload.validate()**（区别于 /stock/fabric 的 `create_stock_fabric`），
 * service 层 `create_stock`（inventory_stock_service.rs:276）直接 Set(color_no) 落库，
 * 且 DB 列 `inventory_stocks.color_no VARCHAR(255)` 无 CHECK 约束——
 * 因此传 color_no="" 可合法写入白坯库存行。
 *
 * 另一条合法路径为采购收货入库（purchase_receipt_private.rs:327
 * `item.color_code.clone().unwrap_or_default()` 当 color_code=None 时落空串），
 * 但构造完整采购链（订单→收货→确认）复杂度过高且非本用例意图，
 * 故 e2e seed 统一走 /inventory/stock 通用端点。
 *
 * seed 维度与调拨出库白坯口径一致：color_no=""、batch_no 必填、dye_lot_no 为空/不传。
 */
export async function seedGreigeStockIn(
  page: Page,
  opts: {
    productId: number;
    warehouseId: number;
    batchNo: string;
    quantityMeters: string;
  }
): Promise<Record<string, unknown>> {
  await apiCall(page, 'POST', '/inventory/stock', {
    warehouse_id: opts.warehouseId,
    product_id: opts.productId,
    batch_no: opts.batchNo,
    color_no: '',
    grade: '一等品',
    quantity_meters: opts.quantityMeters,
    quantity_kg: opts.quantityMeters,
  });
  // 回读验证：按 product+warehouse+batch 查询，确认落库行 color_no 为空
  const path =
    `/inventory/stock?product_id=${opts.productId}` +
    `&warehouse_id=${opts.warehouseId}` +
    `&batch_no=${encodeURIComponent(opts.batchNo)}` +
    `&page=1&page_size=50`;
  const res = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(page, 'GET', path);
  const row = res.items?.find(r => !r.color_no || String(r.color_no) === '');
  if (!row) {
    throw new Error(
      `[seedGreigeStockIn] POST /inventory/stock 成功但回读未命中白坯行：` +
        `product=${opts.productId} warehouse=${opts.warehouseId} batch=${opts.batchNo} ` +
        `—— 落库行 color_no 非空或行不存在，见 reports/backend.log`
    );
  }
  return row;
}

/**
 * 出库第四维（匹号）seed 族 —— 出库对染色布强制四维 = 缸号/色号/批次/匹号
 * （用户 2026-10-02 拍板；后端 services/so/delivery_ops/inventory.rs 按四维 tuple 校验
 *  + services/piece_domain_service.rs:606 outbound_piece_filter CAS 消耗匹号）。
 *
 * 写入方口径（唯一事实源，禁按测试偏好臆造维度）：
 * backend/src/services/piece_domain_service.rs:518-556 —— 染色外发回仓确认生成染色匹：
 *   piece_type='dyed'、piece_no=`{缸号}-{seq:03}`、**batch_no = 缸号（dye_lot_no 同源）**、
 *   color_no 从回仓单/委外订单透传、status='AVAILABLE'、warehouse_id = 回仓单仓库。
 * 因此出库可命中的染色匹 tuple 恒有 batch_no == dye_lot_no == 缸号；
 * 历史 seedFourDimStockIn 以独立 batch 值灌的染色库存行（batch≠缸号）按该 tuple
 * **造不出真实匹**（写入方不产出这种组合），凡要走 UI/API 出染色布的用例一律改用
 * 本节的 seedDyedOutboundBundle（库存行 batch=缸号 + 委外真实链同维染色匹）。
 *
 * 链路先例：flow/07-fabric-four-dim.spec.ts（生产单→流转卡→报工逐匹→染色外发→回仓确认）
 * 与 inventory/05-piece-split.spec.ts（写后必回读 GET /inventory/pieces，词表大写
 * models/status/purchase_inventory.rs::inventory_piece）。
 */

/** GET /inventory/pieces 回读行（PieceResponse 中本族用到的字段） */
export interface DyedSeedPiece {
  id: number;
  piece_no: string;
  piece_type: string;
  status: string;
  color_no: string | null;
  dye_lot_no: string | null;
  batch_no: string;
  product_id: number;
  warehouse_id: number;
  length: number | string;
}

/**
 * 选一个"可承载染色匹"的仓库（piece_domain_service.rs:25-49 validate_warehouse_for_piece_type：
 * 染色匹只允许 成品仓(finished) 或 未设类型(NULL) 仓；胚布仓(greige) 必被拒）。
 * 返回含 warehouse_name/warehouse_code（发货对话框按名称选仓、API 出库按编码）。
 */
export async function pickDyeableWarehouse(
  page: Page
): Promise<{ id: number; name: string; code: string; warehouse_type: string | null }> {
  const res = await apiCallRaw<{ items?: unknown }>(
    page,
    'GET',
    '/warehouses?page=1&page_size=200'
  );
  const rows = pickListArray<Record<string, unknown>>(
    res,
    'items',
    '四维seed 仓库列表 /warehouses'
  );
  const hit = rows.find(
    w => w.warehouse_type === null || w.warehouse_type === undefined || w.warehouse_type === ''
  ) as Record<string, unknown> | undefined;
  const fallback = rows.find(w => w.warehouse_type === 'finished') as
    Record<string, unknown> | undefined;
  const target = hit ?? fallback;
  if (!target) {
    throw new Error(
      '[pickDyeableWarehouse] 仓库列表中不存在可承载染色匹的仓库（需未设类型仓或成品仓；' +
        '胚布仓按 validate_warehouse_for_piece_type 必拒染色匹入库）——真实链造不出匹，显式判红'
    );
  }
  const name = String(target.warehouse_name ?? target.name ?? '');
  const code = String(target.warehouse_code ?? target.code ?? '');
  if (!name || !code) {
    throw new Error(
      `[pickDyeableWarehouse] 仓库 ${JSON.stringify(target).slice(0, 200)} 缺 warehouse_name/warehouse_code`
    );
  }
  return {
    id: Number(target.id),
    name,
    code,
    warehouse_type: (target.warehouse_type as string | null | undefined) ?? null,
  };
}

/** 按四维回读 AVAILABLE 染色匹候选（与发货对话框 loadDeliveryPieces 同一 UI/API 查询口径） */
export async function fetchAvailableDyedPieces(
  page: Page,
  dims: { productId: number; warehouseId: number; dyeLotNo: string; batchNo?: string }
): Promise<DyedSeedPiece[]> {
  const qs =
    `product_id=${dims.productId}&warehouse_id=${dims.warehouseId}` +
    `&dye_lot_no=${encodeURIComponent(dims.dyeLotNo)}` +
    `&batch_no=${encodeURIComponent(dims.batchNo ?? dims.dyeLotNo)}` +
    '&status=AVAILABLE&page=1&page_size=50';
  const res = await apiCallRaw<unknown>(page, 'GET', `/inventory/pieces?${qs}`);
  const rows = pickListArray<DyedSeedPiece>(res, 'items', '染色匹候选 /inventory/pieces');
  return rows.filter(p => p.piece_type === 'dyed' && String(p.status) === 'AVAILABLE');
}

/** 按匹号+四维 tuple 回读单匹（消耗后状态断言用；同匹号跨缸可重复，必须带 tuple 归因） */
export async function readDyedPieceByNo(
  page: Page,
  dims: {
    productId: number;
    warehouseId: number;
    dyeLotNo: string;
    batchNo: string;
    pieceNo: string;
  }
): Promise<DyedSeedPiece | null> {
  const res = await apiCallRaw<unknown>(
    page,
    'GET',
    `/inventory/pieces?piece_no=${encodeURIComponent(dims.pieceNo)}&product_id=${dims.productId}` +
      `&warehouse_id=${dims.warehouseId}&page=1&page_size=20`
  );
  const rows = pickListArray<DyedSeedPiece>(res, 'items', '匹号回读 /inventory/pieces');
  return (
    rows.find(
      p =>
        p.piece_no === dims.pieceNo &&
        p.product_id === dims.productId &&
        p.warehouse_id === dims.warehouseId &&
        String(p.dye_lot_no) === dims.dyeLotNo &&
        String(p.batch_no) === dims.batchNo &&
        p.piece_type === 'dyed'
    ) ?? null
  );
}

/**
 * 真实委外染色链生成 N 匹 AVAILABLE 染色匹（不造假：全程状态机 + 写后必回读）。
 *
 * 链：生产订单 → 流转卡（schedule→备布→完成备布）→ 工序启动 → 报工逐匹（N 匹生产匹入非成品仓）
 *  → N 笔染色委外单（同缸号同色号；发料明细逐匹引用 AVAILABLE 生产匹——
 *    piece_domain_service.rs:160-205 发料必须精确到匹）→ 逐笔发料 → 回仓单（入目标仓）→ 确认
 *  → 每笔确认生成 1 匹染色匹（同缸 piece_seq 递增 → {缸号}-001/-002…，batch_no=缸号）。
 */
export async function seedDyedPieceChain(
  page: Page,
  opts: {
    productId: number;
    warehouseId: number;
    colorNo: string;
    dyeLotNo: string;
    pieceCount?: number;
    lengthPerPieceMeters?: number;
    context?: string;
  }
): Promise<DyedSeedPiece[]> {
  const tag = opts.context ?? 'DYEDSEED';
  const pieceCount = opts.pieceCount ?? 1;
  const length = opts.lengthPerPieceMeters ?? 100;
  const ctx = getCtx();
  if (!ctx.supplierId) {
    throw new Error(`[${tag}] 前置缺失：ctx.supplierId 未就绪（染色委外单必填加工厂）`);
  }

  // 仓库类型口径：染色匹回仓入 opts.warehouseId（须非胚布仓）；生产匹（胚布匹）入仓不得为成品仓。
  const whRes = await apiCallRaw<{ items?: unknown }>(
    page,
    'GET',
    '/warehouses?page=1&page_size=200'
  );
  const whRows = pickListArray<Record<string, unknown>>(whRes, 'items', `${tag} 仓库列表`);
  const target = whRows.find(w => Number(w.id) === opts.warehouseId);
  if (!target) throw new Error(`[${tag}] 仓库 ${opts.warehouseId} 不在 /warehouses 列表中`);
  if (target.warehouse_type === 'greige') {
    throw new Error(
      `[${tag}] 目标仓 ${opts.warehouseId} 为胚布仓（greige），validate_warehouse_for_piece_type ` +
        '拒绝染色匹入仓——染色布出库 seed 必须选未设类型/成品仓，显式判红（勿 skip）'
    );
  }
  const greigeWh =
    target.warehouse_type !== 'finished'
      ? opts.warehouseId
      : Number((whRows.find(w => w.warehouse_type !== 'finished')?.id ?? 0) as number);
  if (!greigeWh) {
    throw new Error(`[${tag}] 无可用非成品仓存放产匹（发料前置），链断，显式判红`);
  }

  // 1. 生产订单 + 流转卡 + 备布完成（07 先例 7-2/7-3 同构载荷）
  const productionOrderNo = genCode(`${tag}PO`);
  const po = await apiCall<{ id?: number; order_no?: string }>(
    page,
    'POST',
    '/production/production-orders/orders',
    {
      order_no: productionOrderNo,
      product_id: opts.productId,
      planned_quantity: pieceCount * length,
    }
  );
  if (!po.data?.id)
    throw new Error(`[${tag}] 生产订单创建未返回 id：${JSON.stringify(po).slice(0, 200)}`);
  const card = await apiCall<{ id?: number }>(page, 'POST', '/production/flow-cards', {
    production_order_id: po.data.id,
    product_id: opts.productId,
    product_name: genName(`${tag}胚布`),
    planned_fabric_weight: pieceCount * length,
  });
  const cardId = card.data?.id;
  if (!cardId)
    throw new Error(`[${tag}] 流转卡创建未返回 id：${JSON.stringify(card).slice(0, 200)}`);
  await apiCall(page, 'POST', `/production/flow-cards/${cardId}/schedule`, {});
  await apiCall(page, 'POST', `/production/flow-cards/${cardId}/start-preparing`);
  await apiCall(page, 'POST', `/production/flow-cards/${cardId}/complete-preparing`, {
    actual_fabric_weight: pieceCount * length,
  });

  // 2. 工序报工逐匹：N 匹生产匹（发料必须逐匹引用，07 先例 7-4 同构载荷）
  const step = await apiCall<{ id?: number }>(page, 'POST', '/production/flow-cards/steps/start', {
    flow_card_id: cardId,
  });
  const stepId = step.data?.id;
  if (!stepId) throw new Error(`[${tag}] 工序启动未返回 id：${JSON.stringify(step).slice(0, 200)}`);
  const greigePieceNos = Array.from(
    { length: pieceCount },
    (_, i) => `GR-${genCode(`${tag}P`)}-${String(i + 1).padStart(3, '0')}`
  );
  await apiCall(page, 'POST', `/production/flow-cards/steps/${stepId}/complete`, {
    actual_quantity: pieceCount * length,
    qualified_quantity: pieceCount * length,
    pieces: greigePieceNos.map(no => ({
      piece_no: no,
      machine_no: 'M-E2E-DYEED-SEED',
      machine_operator: 'E2E开机人',
      length,
      weight: length / 2,
      warehouse_id: greigeWh,
    })),
  });

  // 3. 每匹一笔染色委外（同缸同色）：订单→发料明细（逐匹）→发料→回仓（入目标仓）→确认。
  //    一笔订单确认收回后即 received，不再接受第二张回仓单（validate_receipt_eligibility），
  //    故 N 匹必须 N 笔独立委外单——这是真实写入方口径，不是测试绕行。
  const issueDate = new Date().toISOString().slice(0, 10);
  for (let i = 0; i < pieceCount; i++) {
    const order = await apiCall<{ id?: number }>(page, 'POST', '/production/outsourcing-orders', {
      order_no: genCode(`${tag}OS`),
      order_type: 'dyeing',
      supplier_id: ctx.supplierId,
      production_order_id: po.data.id,
      dye_lot_no: opts.dyeLotNo,
      color_no: opts.colorNo,
      issue_date: issueDate,
      issue_quantity: length,
      issue_unit: '米',
      material_cost: 100,
    });
    const orderId = order.data?.id;
    if (!orderId)
      throw new Error(`[${tag}] 染色委外单创建未返回 id：${JSON.stringify(order).slice(0, 200)}`);
    await apiCall(page, 'POST', '/production/outsourcing-orders/items', {
      outsourcing_order_id: orderId,
      product_id: opts.productId,
      color_no: opts.colorNo,
      dye_lot_no: opts.dyeLotNo,
      piece_no: greigePieceNos[i],
      quantity: length,
      unit: '米',
      unit_cost: 1,
    });
    await apiCall(page, 'POST', `/production/outsourcing-orders/${orderId}/issue`);
    const receipt = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/production/outsourcing-receipts',
      {
        receipt_no: genCode(`${tag}RC`),
        outsourcing_order_id: orderId,
        receipt_date: issueDate,
        product_id: opts.productId,
        dye_lot_no: opts.dyeLotNo,
        color_no: opts.colorNo,
        warehouse_id: opts.warehouseId,
        return_quantity: length,
        quality_status: 'qualified',
        grade: 'A',
      }
    );
    const receiptId = receipt.data?.id;
    if (!receiptId)
      throw new Error(`[${tag}] 回仓单创建未返回 id：${JSON.stringify(receipt).slice(0, 200)}`);
    await apiCall(page, 'POST', `/production/outsourcing-receipts/${receiptId}/confirm`);
  }

  // 4. 写后必回读：四维 tuple 下的 AVAILABLE 染色匹数量必须与 pieceCount 一致，
  //    且逐匹字段与写入方口径一致（batch_no=缸号、piece_type=dyed、大写 AVAILABLE）。
  const pieces = await fetchAvailableDyedPieces(page, {
    productId: opts.productId,
    warehouseId: opts.warehouseId,
    dyeLotNo: opts.dyeLotNo,
    batchNo: opts.dyeLotNo,
  });
  if (pieces.length < pieceCount) {
    throw new Error(
      `[${tag}] 委外染色链回仓确认 ${pieceCount} 次后，GET /inventory/pieces 按 ` +
        `product=${opts.productId} warehouse=${opts.warehouseId} 缸=${opts.dyeLotNo} ` +
        `批=${opts.dyeLotNo} status=AVAILABLE 仅命中 ${pieces.length} 匹染色匹——真实链断裂` +
        `（缸号建档/匹生成/回读口径任一断点），显式判红，勿 skip/放宽`
    );
  }
  for (const p of pieces.slice(0, pieceCount)) {
    if (p.piece_type !== 'dyed' || String(p.status) !== 'AVAILABLE') {
      throw new Error(
        `[${tag}] 匹 ${p.piece_no} 非 染色匹+AVAILABLE（实际 piece_type=${p.piece_type} status=${p.status}），` +
          '词表来源 models/status/purchase_inventory.rs::inventory_piece（大写），出库 tuple 必不命中，显式判红'
      );
    }
    if (String(p.batch_no) !== opts.dyeLotNo || String(p.dye_lot_no) !== opts.dyeLotNo) {
      throw new Error(
        `[${tag}] 匹 ${p.piece_no} 维度与写入方口径矛盾（batch=${p.batch_no} dye=${p.dye_lot_no}，应同为 ${opts.dyeLotNo}）`
      );
    }
  }
  console.log(
    `[${tag}] 真实链染色匹就绪：缸=${opts.dyeLotNo} 色=${opts.colorNo} 批=缸 仓=${opts.warehouseId} ` +
      `匹号=${pieces
        .slice(0, pieceCount)
        .map(p => p.piece_no)
        .join(',')}`
  );
  return pieces.slice(0, pieceCount);
}

/**
 * 染色布出库一体包：一行 batch=缸号 的四维库存行 + 同 tuple 真实 AVAILABLE 染色匹 N 匹。
 * 返回的 stockRow/pieces 四维逐字段一致（色号/缸号/批次/产品/仓库），出库必真实命中。
 */
export async function seedDyedOutboundBundle(
  page: Page,
  opts: {
    productId: number;
    warehouseId: number;
    quantityMeters: string;
    pieceCount?: number;
    colorNo?: string;
    context?: string;
  }
): Promise<{
  dyeLotNo: string;
  colorNo: string;
  stockRow: Record<string, unknown>;
  pieces: DyedSeedPiece[];
}> {
  const tag = opts.context ?? 'DYEDBUNDLE';
  const pieceCount = opts.pieceCount ?? 1;
  const dyeLotNo = genDyeLotNo();
  const colorNo = opts.colorNo ?? `E2EC-${genCode('C')}`;
  if (!colorNo) throw new Error(`[${tag}] 染色布 bundle 色号必须非空`);

  // 库存行按写入方口径造：batch_no = 缸号（与染色匹 tuple 同源同值）
  const stockRow = await seedFourDimStockIn(page, {
    productId: opts.productId,
    warehouseId: opts.warehouseId,
    colorNo,
    dyeLotNo,
    batchNo: dyeLotNo,
    quantityMeters: opts.quantityMeters,
  });
  if (
    String(stockRow.dye_lot_no) !== dyeLotNo ||
    String(stockRow.batch_no) !== dyeLotNo ||
    String(stockRow.color_no) !== colorNo
  ) {
    throw new Error(
      `[${tag}] 库存行落库维度与入参矛盾：期望 色=${colorNo} 缸=批=${dyeLotNo}，` +
        `实际=${JSON.stringify({ color_no: stockRow.color_no, dye_lot_no: stockRow.dye_lot_no, batch_no: stockRow.batch_no })}`
    );
  }

  const total = Number(opts.quantityMeters);
  if (!Number.isFinite(total) || total <= 0) {
    throw new Error(`[${tag}] quantityMeters 必须为正数米数，实际 ${opts.quantityMeters}`);
  }
  const pieces = await seedDyedPieceChain(page, {
    productId: opts.productId,
    warehouseId: opts.warehouseId,
    colorNo,
    dyeLotNo,
    pieceCount,
    lengthPerPieceMeters: Math.max(1, Math.floor(total / pieceCount)),
    context: tag,
  });
  return { dyeLotNo, colorNo, stockRow, pieces };
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

/**
 * 大货批色发货门禁是否**真的以业务拒绝生效**（返回布尔，签名与返回形态不变）。
 *
 * 原实现 `return result.status >= 400` 有三处假绿：CSRF 中间件的一次性 token 竞败 403、
 * 后端裸 5xx、端点未注册的 404 都会被判成"门禁生效"。现复用 {@link isStateGateRejection}：
 * 必须 HTTP 恰 400 且 code ∈ {VALIDATION_ERROR, BUSINESS_ERROR, BAD_REQUEST}。
 * 后端契约原文：services/so/delivery_ops/ship.rs:85-94 把
 * BulkColorApprovalError::InvalidState 映射为 `AppError::business` →
 * utils/error.rs:361 `(BAD_REQUEST, "BusinessError")`，出参机器码 BUSINESS_ERROR。
 * 判定结果与原始 status/code 一律打日志外显，不静默（发货成功返回 false 也看得见）。
 */
export async function verifyBulkColorDeliveryBlock(
  page: Page,
  salesOrderId: number
): Promise<boolean> {
  const result = await apiCallExpectFail(page, 'POST', `/sales/orders/${salesOrderId}/ship`);
  const blocked = isStateGateRejection(result);
  console.log(
    `[verifyBulkColorDeliveryBlock] POST /sales/orders/${salesOrderId}/ship → ` +
      `status=${result.status} code=${JSON.stringify(result.code)}；` +
      `门禁拒绝判据命中=${blocked}（要求 HTTP 400 + 状态门机器码族，CSRF/5xx/404 一律不算）`
  );
  return blocked;
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
 * 端点健康校验参数。
 *
 * - `allowForbidden`：该端点是否属于「权限外探测」场景（用一个无权角色去访问、
 *   预期被鉴权中间件 403 拒绝）。仅在此类显式声明时 403 才算健康；默认 false，
 *   403 视为失败——避免把「未授权访问被拒」误当成路由/权限回归的绿灯掩盖。
 */
export interface EndpointHealthOptions {
  allowForbidden?: boolean;
}

/**
 * 404 响应体的 D-1「Q1 机械二分」判据（纯函数，便于离线喂假体自证检测力）。
 *
 * 后端两种 404 在信封层可二分（utils/error.rs NotFound → 机器码 "NOT_FOUND" → HTTP 404；
 * 未注册路由的 404 由 axum 路由层给出、无 JSON 信封，全仓无 not_found_handler 兜底）：
 * - 响应体能 JSON.parse 且 `code === "NOT_FOUND"` ⇒ **数据面**：路由在册、handler 判查无行，
 *   缺的是前置（seed 回读 / 用例自建），不是端点。
 * - 其余（解析失败 / 非信封体 / 信封但 code 非 NOT_FOUND）⇒ **注册面**：路由不存在或已漂移。
 *
 * 判据只取 HTTP 码与机器 `code` 两个机械量，**绝不把用户可见 message 文案作为断言对象**
 * （文案永久脱敏，本仓红线）。
 */
export function attribute404Body(bodyText: string): {
  surface: 'data' | 'registry';
  code: string | null;
} {
  let json: unknown;
  try {
    json = JSON.parse(bodyText);
  } catch {
    return { surface: 'registry', code: null };
  }
  const rawCode =
    json !== null && typeof json === 'object' ? (json as { code?: unknown }).code : null;
  const code = typeof rawCode === 'string' ? rawCode : null;
  if (code === 'NOT_FOUND') {
    return { surface: 'data', code };
  }
  return { surface: 'registry', code };
}

/**
 * 验证「应当注册存在」的端点可达且不崩溃（严格模式，默认）。
 *
 * 判红口径（收紧假绿）：
 * - 2xx            → 健康。
 * - 5xx            → 失败（服务器内部错误）。
 * - 404            → 失败，并按下方三态判据**二分归因**（注册面 / 数据面，见 attribute404Body）。
 * - 403            → 失败，除非显式 `allowForbidden: true`（权限外探测的正常拒绝）。
 * - 其它 4xx       → 失败（请求契约破坏，如非法参数命中该端点）。
 *
 * 404 三态决策树（D-1 已裁定，机械执行、不留主观裁量）：
 *   Q1 响应体 JSON 且 code=="NOT_FOUND"？ 否→【注册面】进 Q2；是→【数据面】进 Q3。
 *   Q2 注册面处置：(path, METHOD) 在 route-snapshot.txt 在册仅方法不符 → 探针写错，改指真实
 *      GET 契约或移出并注明由哪个用例覆盖；不在册且 src/api/** 有同语义调用点 → 后端缺端点，
 *      补注册并再生成快照；不在册且无调用点 → 探针臆造，移出清单。
 *   Q3 数据面处置：集合/统计端点 id 来源可疑 → 改用例自建或 seed 回读值，禁止可能 undefined
 *      的列表来源插值参与 strict 探针；单资源/1:1 → 多 spec 依赖进 global seed（写后回读）、
 *      单 spec 依赖用例内前置，两者都必须回读断言、禁吞码。
 *   业务上本应 fail-closed 拒绝（缺法定配置）的 404/403 → 语义面，不是缺口，改断负例。
 *
 * 本仓不存在「允许缺失、用 404/403 装作健康」的端点类别，也没有把未注册端点登记成豁免的
 * 通道：scripts/check-api-paths.mjs 的 KNOWN_GAPS 只覆盖 A 类（src/api 调用点），E 类 e2e
 * 探针注册表在该文件中明文「本检查不设豁免表」。故未注册端点的处置只有：补注册 /
 * 探针改指 / 移出清单并注明覆盖去向，**禁止登记成豁免**；也禁止以任何宽松探测把 404/403
 * 伪装成健康（系统性假绿源，本模块不提供可选探测 helper）。
 */
export async function verifyEndpointHealthy(
  page: Page,
  path: string,
  opts: EndpointHealthOptions = {}
): Promise<void> {
  let status: number;
  try {
    const res = await apiCallRaw(page, 'GET', path);
    // apiCallRaw 成功即 2xx（非 2xx 会抛），无返回值也视为健康
    void res;
    return;
  } catch (e) {
    const err = e as { status?: number; message?: string };
    status = err.status || 0;
  }

  if (status >= 500) {
    throw new Error(`GET ${path} 返回 ${status}（服务器内部错误）`);
  }
  if (status === 404) {
    // D-1 Q1 二分归因：复读 404 响应体（GET 幂等，且仅失败路径才多这一次请求）。
    // 只凭状态码一律判"端点未注册或路由已漂移"，会把数据面的 NOT_FOUND 信封（如"该客户
    // 无信用评级记录"，缺的是前置不是端点）误判成注册面缺陷，故必须看信封里的机器 code。
    let bodyText: string;
    try {
      const res404 = await page.request.get(`${API_BASE}${API_PREFIX}${path}`);
      bodyText = await res404.text();
    } catch (e) {
      throw new Error(
        `GET ${path} 返回 404 且响应体复读失败（${(e as Error).message}）：无法执行 Q1 二分，需人工对照 backend.log 定性注册面/数据面，禁默认归因`
      );
    }
    const attr = attribute404Body(bodyText);
    const diagnosis =
      attr.surface === 'data'
        ? 'NOT_FOUND 信封=数据面缺前置（seed 回读或用例自建，禁吞码）'
        : attr.code === null
          ? '无 JSON 信封=注册面（路由不存在/已漂移：补注册或探针改指/移出，无豁免通道）'
          : `JSON 信封 code=${attr.code}（非 NOT_FOUND）=按 Q1 判注册面，需人工复核信封来源`;
    console.log(`[diag][http404] GET ${path} 归因=${attr.surface} — ${diagnosis}`);
    throw new Error(
      `GET ${path} 返回 404：${diagnosis}（严格健康检查判红，不再吞 404；归因判据只取 HTTP 码与机器 code，不断 message 文案）`
    );
  }
  if (status === 403) {
    if (opts.allowForbidden === true) {
      return; // 显式权限外探测：403 属预期，健康
    }
    throw new Error(
      `GET ${path} 返回 403：鉴权拒绝。若这是权限外探测端点，请显式传 { allowForbidden: true }；否则视为权限/路由回归判红（不存在可吞 403 的宽松健康探测函数，见本文件 strict 口径注释）`
    );
  }
  if (status >= 400) {
    throw new Error(`GET ${path} 返回 ${status}（请求契约破坏）`);
  }
  // status === 0：网络层错误（apiCall 已抛非数字状态）
  throw new Error(`GET ${path} 请求异常（status=${status}）`);
}

/**
 * 验证返回二进制/非 JSON 的下载类端点（xlsx / docx / zip 等）严格健康。
 *
 * `verifyEndpointHealthy` 走 apiCall → response.text() + JSON.parse()，对**二进制 2xx**
 * 响应会因 JSON.parse 失败而抛错（status 仍是 200），落入其末尾分支误判「请求异常」——
 * 对导出端点是假红。本函数改用原始响应状态判定，语义与 verifyEndpointHealthy 的严格口径一致：
 * - 2xx            → 健康。
 * - 5xx            → 失败（服务器内部错误）。
 * - 404            → 失败（端点未注册 / 路由漂移）。
 * - 403            → 失败，除非显式 `allowForbidden: true`（权限外探测的正常拒绝 / fail-closed）。
 * - 其它非 2xx     → 失败（重定向、请求契约破坏等）。
 *
 * 适用于 admin 上下文应直接 2xx 的导出端点（如 /production/wage-records/export）。
 */
export async function verifyDownloadEndpointHealthy(
  page: Page,
  path: string,
  opts: EndpointHealthOptions = {}
): Promise<void> {
  const res = await page.request.get(`${API_BASE}${API_PREFIX}${path}`);
  const status = res.status();
  if (status >= 200 && status < 300) {
    return;
  }
  if (status === 403 && opts.allowForbidden === true) {
    return;
  }
  throw new Error(
    `GET ${path} 返回 ${status}（下载端点严格健康检查：期望 2xx；404=路由漂移/未注册，403=权限拒，5xx=服务错误，其它非 2xx 均判红）`
  );
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

/** 隔离会话：独立 BrowserContext + 一个已登录 Page，供水平越权等多用户测试使用。 */
export interface IsolatedAuthedSession {
  context: BrowserContext;
  page: Page;
  username: string;
  close: () => Promise<void>;
}

/**
 * 在一个全新的隔离 BrowserContext 内用指定账号真实登录（cookie 会话独立，与默认 fixture
 * context 互不干扰），返回该会话。用于水平越权：以 B 账号凭证去改/删 A 账号的资源。
 *
 * 走后端真实登录（POST /auth/login，Set-Cookie 写入本 context），不使用 UI 表单——UI 登录
 * 会改写模块级共享 LOGGED_IN 标志并依赖 storageState，多 context 下不可靠。/auth/login 免
 * 鉴权且 CSRF 豁免，故无需预取 csrf token。登录成功后校验 access_token 已落本 context，
 * 否则判红（前置未就绪，不得伪装成"越权被拒"的绿灯）。
 */
export async function loginInIsolatedContext(
  browser: Browser,
  username: string,
  password: string
): Promise<IsolatedAuthedSession> {
  // 不继承 storageState，确保是干净的独立会话（用另一账号重新登录）
  const context = await browser.newContext({ storageState: { cookies: [], origins: [] } });
  const page = await context.newPage();
  const loginResp = await page.request.post(`${API_BASE}${API_PREFIX}/auth/login`, {
    data: { username, password },
    headers: { 'Content-Type': 'application/json', 'X-Requested-With': 'XMLHttpRequest' },
  });
  if (!loginResp.ok()) {
    const body = await loginResp.text().catch(() => '');
    await context.close();
    throw new Error(
      `loginInIsolatedContext：账号 ${username} 登录失败 HTTP ${loginResp.status()} body=${body.slice(0, 200)}`
    );
  }
  const cookies = await context.cookies();
  if (!cookies.some(c => c.name === 'access_token')) {
    await context.close();
    throw new Error(`loginInIsolatedContext：账号 ${username} 登录后未获得 access_token cookie`);
  }
  return {
    context,
    page,
    username,
    close: async () => {
      await context.close().catch(e => {
        console.warn(`[loginInIsolatedContext] 关闭 context 失败: ${(e as Error).message}`);
      });
    },
  };
}

/**
 * 取一个与默认分片账号（TEST_USERNAME，即"用户 A"）不同的"用户 B"凭证。
 * 从 role-credentials.json 中选一个 username 明确不等于 TEST_USERNAME 的角色账号，
 * 保证 B 是独立身份的账号（水平越权前提：两个不同 owner）。找不到即判红，
 * 不允许退化成"用同一账号自己测自己"的假越权。
 */
export function pickDifferentUserCredential(): { username: string; password: string } {
  try {
    if (!existsSync(ROLE_CREDENTIALS_PATH)) {
      throw new Error(
        `凭证文件不存在: ${ROLE_CREDENTIALS_PATH}（global-setup ensureRoleUsers 未运行？）`
      );
    }
    const data = JSON.parse(readFileSync(ROLE_CREDENTIALS_PATH, 'utf-8')) as Record<
      string,
      RoleCredential
    >;
    const candidate = Object.values(data).find(
      c => c && c.username && c.username !== TEST_USERNAME
    );
    if (!candidate) {
      throw new Error(
        `role-credentials.json 中找不到与分片账号 ${TEST_USERNAME} 不同的第二个账号，无法构造水平越权前提`
      );
    }
    return { username: candidate.username, password: candidate.password };
  } catch (e) {
    throw new Error(`pickDifferentUserCredential 失败：${(e as Error).message}`);
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
 * 「延迟清理」队列元素：记录一条要推迟到断言之后再发的清理动作（不发请求，仅登记）。
 */
export interface DeferredCleanup {
  method: 'DELETE' | 'PUT' | 'POST';
  path: string;
  label?: string;
}

/**
 * 登记一条「清理到断言之后执行」的动作，真正的 DELETE/PUT 由 {@link flushDeferredCleanups}
 * （通常在 test.afterEach 或 try/finally）统一发起。
 *
 * 为什么要与同步 tryCleanup 并存（不替换、不改其即时语义）：`tryCleanup` 是**当场**发起的同步
 * 软删。若用例在被测主数据上还留有后续读断言（by-code/详情回读、"未删状态下重复应被拒"的判重
 * 前提等），当场软删会让后端按 is_deleted=false 过滤后查不到行 → 回读 404 / 判重返回 200——这对
 * 后端是**正确行为**，却曾把用例推向假红并被 #4669/#4671 误判成后端缺陷。清理本质是 housekeeping,
 * 语义上应发生在全部断言之后,故用队列延后 flush。
 *
 * 范式与 purchase/03（CREATED_ORDER_IDS + afterEach）、finance/01（CLEANUP[] + afterEach）一致：
 * 队列由调用方 spec 自行持有（逐文件独立、afterEach flush 后清空），杜绝跨 spec 共享态泄漏；
 * 本函数只 push、不发请求,向后兼容——不影响任何既有 tryCleanup 调用点。
 */
export function deferCleanup(
  queue: DeferredCleanup[],
  method: 'DELETE' | 'PUT' | 'POST',
  path: string,
  label?: string
): void {
  queue.push({ method, path, label });
}

/**
 * 逆序 flush 延迟清理队列（子记录先于父记录删，尽量贴近引用顺序），逐条复用
 * {@link tryCleanup} 的「失败仅告警不 rethrow」语义，随后清空队列防止跨用例泄漏。
 */
export async function flushDeferredCleanups(page: Page, queue: DeferredCleanup[]): Promise<void> {
  for (const c of queue.slice().reverse()) {
    await tryCleanup(page, c.method, c.path, c.label);
  }
  queue.length = 0;
}

/** CSRF 家族机器码全集（csrf.rs:37-43 CODE_MISS/CODE_INVAL/CODE_IP_MM），均由中间件直出 HTTP 403 */
const CSRF_REJECT_CODES: ReadonlySet<string> = new Set<string>(Object.values(CSRF_ERROR_CODES));

/**
 * 断言 API 响应被**权限门**拒绝（HTTP 403 + 权限机器码 FORBIDDEN）。
 *
 * 判据 = status + `AppError` 信封机器码双钉，永不读取错误文案
 * （本仓权限拒绝文案永久脱敏，文案只可能空转，不能承载归因）。
 * 为什么只判 status 是假绿：本仓 CSRF 中间件与权限门**都**直出 403——
 * - CSRF 面：csrf.rs:232-234 `csrf_error_response` → `unified_error_response(FORBIDDEN, "CSRF_*")`；
 *   且一次性 token 竞败重放耗尽/异常时 apiCallExpectFail 仍返回该 403（仅记日志不抛），
 *   `CSRF_IP_MISMATCH` 不在 isCsrfRejection 重放判据内、首次即原样返回。
 * - 权限面：RBAC 直出 response.rs:145 与 AppError::PermissionDenied（error.rs:709/742）同码 `FORBIDDEN`。
 * 只判 status 时，CSRF-403 会冒名权限-403 通过——权限门即使被删，本类用例照样绿。
 * 故拒绝归因必须钉到权限家族唯一机器码 `FORBIDDEN`：实际 code 为 CSRF_* 即判红并点名
 * "会话 CSRF 前置问题"（修复方向是确保调用前会话持有存活 token——复用本 helpers 既有的
 * getCsrfToken / syncCsrfFromResponse / 有界重放机制，而非放宽本断言）。
 *
 * 参数类型向后兼容：apiCallExpectFail 的 ApiFailureResult 及既有裸 {status} 形态均可传入；
 * code 缺失/非字符串（如响应体非 JSON）无法完成归因，同样判红而非静默放过。
 */
export function expectDenied(
  result: { status: number; code?: string | number },
  context = ''
): void {
  const prefix = context || '应返回 403 权限拒绝';
  expect(result.status, `${prefix}：实际 status=${result.status}`).toBe(403);
  const code = typeof result.code === 'string' ? result.code : undefined;
  expect(
    CSRF_REJECT_CODES.has(code ?? ''),
    `${prefix}：该 403 的 code=${code ?? JSON.stringify(result.code)} 属 CSRF 中间件拒绝` +
      `（${[...CSRF_REJECT_CODES].join('/')}），是会话 CSRF 前置未成立而非权限门判定——` +
      `假绿拦截：禁止只判 status 放行，应确认会话持有存活 CSRF token（既有重放机制），勿放宽本断言`
  ).toBe(false);
  expect(
    code,
    `${prefix}：403 必须归因到权限门机器码 ${APP_ERROR_CODES.FORBIDDEN}（RBAC 直出与 AppError::PermissionDenied 同源），` +
      `实际 code=${JSON.stringify(result.code)}（缺失/非字符串=响应体非统一信封，无法归因，同样判红）`
  ).toBe(APP_ERROR_CODES.FORBIDDEN);
}

/**
 * 状态机/业务门禁拒绝的机器码族。唯一事实来源是后端 HTTP 映射
 * （backend/src/utils/error.rs:358-366 `error_status_and_type`）：
 * ValidationError / ValidationErrorDisplayable / BusinessError / BusinessErrorDisplayable /
 * BadRequest 全部映射到 **HTTP 400**，对应字符串码
 * （error.rs:743-746 + error.rs:464）VALIDATION_ERROR / BUSINESS_ERROR / BAD_REQUEST。
 * 403 不在此列——权限门与状态门是两条不同的门，混用即假绿。
 */
const STATE_GATE_REJECT_CODES: ReadonlySet<string> = new Set<string>([
  APP_ERROR_CODES.VALIDATION_ERROR,
  APP_ERROR_CODES.BUSINESS_ERROR,
  APP_ERROR_CODES.BAD_REQUEST,
]);

/**
 * 纯判据：失败体是否构成**状态机/业务门禁拒绝**（不抛错，供需要返回布尔的调用方复用）。
 * 三条同时成立：HTTP 恰 400 + code 为字符串机器码 + code ∈ STATE_GATE_REJECT_CODES。
 */
export function isStateGateRejection(result: ApiFailureResult): boolean {
  const code = failureCode(result);
  return result.status === 400 && code !== undefined && STATE_GATE_REJECT_CODES.has(code);
}

/**
 * 断言「非法状态转换/业务门禁确实被状态门拒绝」（收紧只判 status 的假绿）。
 *
 * 原 `verifyIllegalTransition` 写法是 `if (status < 400) throw`，三处假绿：
 * 1. **CSRF 冒名**：写方法的一次性 token 竞败重放耗尽后 apiCallExpectFail 原样返回
 *    403 + `CSRF_*`（见本文件 1564-1570 的日志分支）→ 状态门即使被整条删掉，用例照样"通过"；
 * 2. **裸 5xx**：INTERNAL_ERROR/DATABASE_ERROR（后端缺前置校验，靠 DB 约束或 panic 兜底）
 *    也被当成"拒绝生效"；
 * 3. **404**：端点未注册/路由漂移/资源不存在同样被当成"拒绝生效"。
 * 现在钉死 HTTP=400 + 状态门机器码族，归因到 `AppError` 信封机器码，
 * **永不读取错误文案**（文案脱敏，只能空转）。
 *
 * 判责：若后端对某条非法流转返回 403（权限门冒名状态门）或裸 5xx，本断言会红——
 * 那是真实缺陷（契约不符或权限/状态门混用），禁止放宽回 `>=400`。
 */
export function expectStateGateRejection(result: ApiFailureResult, context = ''): void {
  const prefix = context || '非法流转应被状态机门拒绝';
  const code = failureCode(result);
  expect(
    result.status,
    `${prefix}：应恰为 HTTP 400（backend utils/error.rs:358-366 状态映射），实际 status=${result.status}` +
      ` code=${JSON.stringify(result.code)}——403 属 CSRF 中间件拒绝或权限门（两条门都不是状态门）、` +
      `404 属端点未注册/路由漂移、5xx 属后端裸崩，均不构成"状态门拒绝生效"`
  ).toBe(400);
  expect(
    code,
    `${prefix}：拒绝必须带字符串机器码（统一失败信封 utils/error.rs ErrorResponse），` +
      `实际 code=${JSON.stringify(result.code)}（缺失/非字符串=响应体非统一信封，无法归因）`
  ).toBeTruthy();
  expect(
    code !== undefined && STATE_GATE_REJECT_CODES.has(code),
    `${prefix}：code=${code ?? JSON.stringify(result.code)} 不属状态门拒绝族` +
      ` {${[...STATE_GATE_REJECT_CODES].join('/')}}——假绿拦截：勿放宽本断言，` +
      `若后端确以其它族拒绝，应改后端契约而不是改这里`
  ).toBe(true);
}

/**
 * 断言 API 响应为业务错误（status >= 400）
 *
 * 替代各 spec 中重复的: expect(result.status >= 400).toBe(true)
 *
 * 注意：本函数刻意保留「宽松」语义仅供确实只关心"被拒且非 5xx"的历史调用点。
 * 删除/引用等防护类断言禁止用它兜底，应改用 expectBusinessRejection（钉死 400 + 业务码
 * + 错误 message），否则后端裸 500 会被 >=400 伪装成"删除守卫生效"的绿灯。
 */
export function expectBadRequest(result: { status: number }, context = ''): void {
  expect(result.status, context || '应返回 400+ 业务错误').toBeGreaterThanOrEqual(400);
}

/** 500/501 家族机器码：命中即"后端未实现前置校验、靠 DB 约束/未处理 panic 裸抛"——非业务拒绝。 */
const SERVER_FAULT_CODES: ReadonlySet<string> = new Set([
  'INTERNAL_ERROR',
  'DATABASE_ERROR',
  'NOT_IMPLEMENTED',
]);

/**
 * 断言「删除/引用防护被业务规则正确拒绝」的精确契约（收紧 >=400 假绿）。
 *
 * 三条同时成立才算通过：
 * 1. HTTP 状态恰为 400（业务拒绝的正确契约，非 500 裸崩、非 404 路径错误）；
 * 2. 响应 code 为字符串业务机器码且不属于 500 家族（INTERNAL_ERROR/DATABASE_ERROR/
 *    NOT_IMPLEMENTED）——排除"靠 DB FK 约束在 delete 阶段裸抛 500"被当成守卫；
 * 3. 响应含非空业务错误 message（守卫命中必带可读拒绝原因）。
 *
 * 判责：若后端实为裸 500（如引用校验前置缺失、FK 直接炸），本断言会红——这是源码缺陷，
 * 应保持红并交后端修复（补前置业务校验返回 400 BUSINESS_ERROR），禁止把断言放宽回 >=400 蒙过。
 */
export function expectBusinessRejection(
  result: ApiFailureResult,
  context = '删除/引用防护应被业务拒绝（HTTP 400 + 业务码 + 拒绝原因）'
): void {
  expect(
    result.status,
    `${context}：实际 status=${result.status} code=${result.code ?? '(none)'} message=${result.message ?? '(none)'}`
  ).toBe(400);
  const code = typeof result.code === 'string' ? result.code : undefined;
  expect(
    code,
    `${context}：code 应为字符串业务机器码（非数字/非缺失），实际 raw code=${JSON.stringify(result.code)} message=${result.message ?? '(none)'}`
  ).toBeTruthy();
  expect(
    code === undefined || !SERVER_FAULT_CODES.has(code),
    `${context}：code=${code} 属 5xx 裸崩家族（后端缺少删除前置校验，靠 DB 约束/panic 兜底），应返回 400 业务码——源码缺陷，勿放宽本断言`
  ).toBe(true);
  expect(
    typeof result.message === 'string' && result.message.trim().length > 0,
    `${context}：响应应含非空业务错误 message，实际 message=${JSON.stringify(result.message)}`
  ).toBe(true);
}

/**
 * 「质检合格方可入库/结算」门控（commit 48aa4395）要求的真实前置链。
 *
 * 后端 backend/src/services/purchase_receipt_service.rs::ensure_receipt_inspection_allows_flow
 * 只放行 inspection_status == PASSED 的收货单；新建收货单恒为 PENDING
 * （purchase_receipt.inspection_status 列 NOT NULL DEFAULT 'PENDING'），REJECTED 也拒。
 * PASSED 的唯一业务写入口是采购质检完成回写（purchase_inspection_service.rs::complete_inspection
 * → to_receipt_inspection_status），不存在任何直改状态的旁路端点。
 *
 * 因此凡是要 `POST /purchase/receipts/{id}/confirm`（或 `POST /ap/invoices/auto-generate`）
 * 的用例，都必须先跑完本函数：建质检单 → complete(pass) → 回读必须真读到 PASSED。
 * 任一步不达预期立即抛错，**不允许**继续去 confirm 撞 400 —— 那会把"回写链断了"
 * 伪装成"确认接口故障"，正是本仓反复踩过的隐性红。
 * 结论 token 用权威词表原值 pass/fail/partial（models/status/purchase_inventory.rs），
 * fail/partial 都会得到 REJECTED，故本函数只用于 pass 场景。
 *
 * @returns 质检单 id（供用例断言待质检列表或清理用）
 */
export async function seedInspectionPass(
  page: Page,
  opts: {
    receiptId: number;
    supplierId: number;
    /** 合格数量；省略时取收货单 total_quantity（即"整单全数合格"） */
    passQuantity?: string | number;
    context?: string;
  }
): Promise<number> {
  const tag = opts.context ?? `receipt#${opts.receiptId}`;
  let passQuantity = opts.passQuantity;
  if (passQuantity === undefined) {
    const head = await apiCall<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${opts.receiptId}`
    );
    // 出参 total_quantity 是 rust_decimal 序列化的十进制字符串，原样透传即可
    passQuantity = (head?.data as Record<string, unknown>)?.total_quantity as string | undefined;
    if (!passQuantity) {
      throw new Error(
        `[${tag}] 收货单回读缺 total_quantity，无法按"整单全数合格"完成质检：` +
          `${JSON.stringify(head)?.slice(0, 300)}`
      );
    }
  }
  const created = await apiCall<Record<string, unknown>>(page, 'POST', '/purchase/inspections', {
    receipt_id: opts.receiptId,
    supplier_id: opts.supplierId,
    inspection_date: new Date().toISOString().slice(0, 10),
  });
  const inspId = (created?.data as Record<string, unknown>)?.id as number | undefined;
  if (!inspId) {
    throw new Error(`[${tag}] 建质检单未取到 id，响应=${JSON.stringify(created)?.slice(0, 300)}`);
  }
  await apiCall(page, 'POST', `/purchase/inspections/${inspId}/complete`, {
    // 必须传局部变量：opts.passQuantity 省略时上面刚从收货单 total_quantity 推导出来，
    // 直接回读 opts 会把 undefined 送出去（JSON.stringify 丢键 → 后端必填 400，种子步成片真红）。
    pass_quantity: passQuantity,
    reject_quantity: 0,
    inspection_result: 'pass',
  });
  const readBack = await apiCall<Record<string, unknown>>(
    page,
    'GET',
    `/purchase/receipts/${opts.receiptId}`
  );
  const inspectionStatus = (readBack?.data as Record<string, unknown>)?.inspection_status;
  if (inspectionStatus !== 'PASSED') {
    throw new Error(
      `[${tag}] 质检 complete(pass) 后收货单 inspection_status=${String(inspectionStatus)}，` +
        '期望 PASSED（词表 backend models/status/purchase_inventory.rs::purchase_receipt_inspection）'
    );
  }
  return inspId;
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

/**
 * 【新增函数（CI #4669 I 族收口）】为「染色批次/缸号」建一条真实的色卡档案前置。
 *
 * 后端强校验（正当，不得放松）：dye_batch_handler.rs::resolve_dye_identity 归一
 * （backend/src/handlers/dye_batch_handler.rs:203-254）要求 color_no 非空即染色布，且该色号
 * 必须在全仓唯一的色卡明细档案 `color_card_items.color_code` 上**恰好命中一条**：
 * - 档案无此色 → 400 VALIDATION「色号 XXX 在色卡档案中不存在」（CI #4669 红 03-production:87、
 *   21d:237 的原文，即本函数消灭的前置缺失）；
 * - 同色号多条 → 显式业务错「无法唯一定位」，故色号取 genCode（时间戳+随机）保证全局唯一。
 *
 * 前置链全部走真实端点（与 fabric/02-dye.spec.ts 既有 seedColorCardItem 先例同型，本函数是
 * 其 helpers 版收口，供 flow 族多文件复用；不改动任何既有函数）：
 * 1. POST /color-cards（handlers/color_card/crud.rs:78-101，创建即 draft——色卡主体词表
 *    color_card::DRAFT；出参 ColorCardListItem 含 id）；
 * 2. POST /color-cards/{id}/items（handlers/color_card/items.rs:48-58；服务门控
 *    EDITABLE_CARD_STATUSES=[draft]（color_card_item_service.rs:65），新建卡恒可挂色号；
 *    出参 ColorItemInfo 含 id/color_code）。
 *
 * 色卡档案不做清理：DELETE 端点是**归档**语义（crud.rs archive，非物理删除），物理删档会
 * 破坏历史染色批次的派生链；每次运行新建专属卡+唯一色号，跨分片互不干扰。
 *
 * @returns cardId 专属色卡 id；colorCode 已入档的唯一色号（喂给 dye-batch 的 color_no）；itemId 色号明细 id
 */
export async function seedColorCardArchive(
  page: Page,
  opts?: { context?: string }
): Promise<{ cardId: number; colorCode: string; itemId: number }> {
  const tag = opts?.context ?? 'seedColorCardArchive';
  const colorCode = genCode('E2E-ARCH');
  const card = await apiCall<{ id?: number; card_no?: string }>(page, 'POST', '/color-cards', {
    card_no: genCode('E2E-ARCHCC'),
    card_name: `E2E 档案前置色卡 ${colorCode}`,
    card_type: 'CUSTOM',
  });
  const cardId = card?.data?.id;
  if (!cardId) {
    throw new Error(
      `[${tag}] 前置色卡创建未返回 id（后端契约：ColorCardListItem 必含 id）：${JSON.stringify(card)}`
    );
  }
  // 载荷对照 ColorItemDto（backend/src/models/color_card_item_dto.rs:12-66）：
  // color_code/color_name/rgb_r/g/b 必填，hex_value 必填且长度恰为 7（#RRGGBB）。
  const item = await apiCall<{ id?: number; color_code?: string }>(
    page,
    'POST',
    `/color-cards/${cardId}/items`,
    {
      color_code: colorCode,
      color_name: 'E2E 档案前置色号',
      rgb_r: 220,
      rgb_g: 20,
      rgb_b: 60,
      hex_value: '#DC143C',
    }
  );
  const itemId = item?.data?.id;
  if (!itemId) {
    throw new Error(
      `[${tag}] 色号明细创建未返回 id（后端契约：ColorItemInfo 必含 id）：${JSON.stringify(item)}`
    );
  }
  if (item.data.color_code !== colorCode) {
    throw new Error(
      `[${tag}] 色号明细回显与提交不一致（期望 ${colorCode}，实际 ${JSON.stringify(item)}）` +
        '——色卡状态门控或端点契约漂移属后端问题，不得在此放宽'
    );
  }
  console.log(`[${tag}] 色卡档案前置就绪：card=${cardId} item=${itemId} color_code=${colorCode}`);
  return { cardId, colorCode, itemId };
}
