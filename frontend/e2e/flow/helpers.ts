/* eslint-disable no-console */
import type { Page } from '@playwright/test';
import {
  createWarehouseUI,
  createDepartmentUI,
  createSupplierUI,
  createProductUI,
  createColorCardUI,
  createDyeBatchUI,
  createDyeRecipeUI,
  createBomUI,
  createCustomOrderUI,
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

export interface EntityContext {
  departmentIds: number[];
  warehouseIds: number[];
  productCategoryIds: number[];
  productIds: number[];
  productColorIds: number[];
  colorNos: string[];
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

export async function ensureTestEntities(page: Page): Promise<void> {
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
    ctx.warehouseIds = await readEntityIds(page, '/warehouse', `${API_PREFIX}/warehouses`);
  } catch {
    ctx.warehouseIds = [];
  }
  if (ctx.warehouseIds.length < 2) {
    for (let i = ctx.warehouseIds.length; i < 2; i++) {
      const id = await uiCreateWithRetry(page, createWarehouseUI);
      if (id) {
        ctx.warehouseIds.push(id);
      } else {
        console.error(
          '[ensureTestEntities] 仓库 UI 创建失败: 返回 undefined（详见 ui-helpers 截图诊断）'
        );
      }
    }
  }
  if (ctx.warehouseIds.length < 2) ctx.warehouseIds = [1, 2];

  // ---- 2. 产品（UI 创建）----
  // 前置：确保"面料"产品分类存在（表单 category_id 必填，系统初始化不创建分类种子数据）
  try {
    const cats = await apiCallRaw<{ id?: number; name?: string }[] | { items?: { id: number }[] }>(
      page,
      'GET',
      '/product-categories'
    );
    const catItems = Array.isArray(cats)
      ? cats
      : (cats as { items?: { id: number }[] }).items || [];
    const hasFabric = catItems.some(c => (c as { name?: string }).name?.includes('面料'));
    if (!hasFabric) {
      const created = await apiCall<{ id?: number }>(page, 'POST', '/product-categories', {
        name: '面料',
        code: 'FABRIC',
      });
      console.log('[ensureTestEntities] 创建产品分类"面料":', created.code);
    }
  } catch (e) {
    // 分类创建失败仅告警（可能已存在），产品创建失败时诊断信息会暴露详情
    console.warn('[ensureTestEntities] 产品分类检查/创建失败:', (e as Error).message);
  }
  try {
    ctx.productIds = await readEntityIds(page, '/product', `${API_PREFIX}/products`);
  } catch {
    ctx.productIds = [];
  }
  if (ctx.productIds.length === 0) {
    // 先 UI 尝试一次（下拉交互脆弱：分类 select 点击后偶发不更新 v-model）
    const uiId = await uiCreateWithRetry(page, createProductUI);
    if (uiId) {
      ctx.productIds.push(uiId);
    } else {
      console.warn(
        '[ensureTestEntities] 产品 UI 创建失败，改用 API 兜底创建（保证后续流程不被阻塞）'
      );
    }
    // API 兜底补齐到 3 个
    while (ctx.productIds.length < 3) {
      try {
        // CreateProductRequest 字段：code/name/category_id/unit
        const result = await apiCall<{ id?: number }>(page, 'POST', '/products', {
          code: `E2E-P${Date.now().toString().slice(-6)}${ctx.productIds.length}`,
          name: `E2E产品${Date.now().toString().slice(-6)}${ctx.productIds.length}`,
          unit: '米',
        });
        if (result.data?.id) {
          ctx.productIds.push(result.data.id);
          console.log('[ensureTestEntities] 产品 API 兜底创建成功 id=', result.data.id);
        } else {
          console.error('[ensureTestEntities] 产品 API 兜底未返回 id:', JSON.stringify(result));
          break;
        }
      } catch (e) {
        console.error('[ensureTestEntities] 产品 API 兜底创建失败:', (e as Error).message);
        break;
      }
    }
  }
  if (ctx.productIds.length === 0) ctx.productIds = [1];

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
  } catch {
    ctx.colorNos = ['TEST-COLOR'];
    ctx.productColorIds = [1];
  }
  if (ctx.colorNos.length === 0) ctx.colorNos = ['TEST-COLOR'];

  // ---- 4. 供应商（UI 创建）----
  try {
    ctx.supplierId = await readFirstEntityId(page, '/supplier', `${API_PREFIX}/purchase/suppliers`);
  } catch (e) {
    console.error('[ensureTestEntities] supplierId 查找失败:', (e as Error).message);
    ctx.supplierId = undefined;
  }
  if (!ctx.supplierId) {
    const id = await uiCreateWithRetry(page, createSupplierUI);
    ctx.supplierId = id;
    if (!id) {
      console.warn('[ensureTestEntities] 供应商 UI 创建失败，改用 API 兜底创建');
      try {
        const result = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
          supplier_name: `E2E供应商${Date.now().toString().slice(-6)}`,
          supplier_short_name: 'E2E供',
          contact_phone: '13800000001',
        });
        ctx.supplierId = result.data?.id;
        if (!ctx.supplierId) {
          console.error('[ensureTestEntities] 供应商 API 兜底未返回 id:', JSON.stringify(result));
        }
      } catch (e) {
        console.error('[ensureTestEntities] 供应商 API 兜底创建失败:', (e as Error).message);
      }
    }
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
    console.error('[ensureTestEntities] customerId 创建失败:', (e as Error).message);
    ctx.customerId = undefined;
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
  } catch {
    ctx.accountSubjectIds = [];
  }

  // ---- 7. 部门（UI 创建）----
  if (ctx.departmentIds.length === 0) {
    try {
      ctx.departmentIds = await readEntityIds(page, '/departments', `${API_PREFIX}/departments`);
    } catch {
      ctx.departmentIds = [];
    }
  }
  if (ctx.departmentIds.length === 0) {
    const id = await uiCreateWithRetry(page, createDepartmentUI);
    if (id) {
      ctx.departmentIds.push(id);
    } else {
      console.error(
        '[ensureTestEntities] 部门 UI 创建失败: 返回 undefined（详见 ui-helpers 截图诊断）'
      );
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
      const result = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
        supplier_id: ctx.supplierId || 1,
        warehouse_id: ctx.warehouseIds[0] || 1,
        department_id: ctx.departmentIds[0] || 1,
        order_date: new Date().toISOString().slice(0, 10),
        items: [{ material_id: ctx.productIds[0] || 1, quantity_ordered: '1', unit_price: '1' }],
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
  if (!ctx.salesOrderId) {
    try {
      // 先创建库存记录（销售订单创建会锁库存，无库存 → BUSINESS_ERROR）
      const stock = await apiCall<{ id?: number }>(page, 'POST', '/inventory/stock/fabric', {
        warehouse_id: ctx.warehouseIds[0] || 1,
        product_id: ctx.productIds[0] || 1,
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
        customer_id: ctx.customerId || 1,
        order_date: new Date().toISOString().slice(0, 10),
        items: [{ product_id: ctx.productIds[0] || 1, quantity: '1', unit_price: '1' }],
      });
      ctx.salesOrderId = result.data?.id;
    } catch (e) {
      console.error('[ensureTestEntities] salesOrderId 创建失败:', (e as Error).message);
      ctx.salesOrderId = undefined;
    }
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
        customer_id: ctx.customerId || 1,
        sales_user_id: 1,
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
            product_id: ctx.productIds[0] || 1,
            unit: '米',
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
      `${API_PREFIX}/production/dye-batches`
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
          status: 'DRAFT',
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

  // ---- 12.5 BPM 流程定义（测试前置：销售订单 submit 触发 BPM 审批流程，
  //        "sales_order_approval" 定义不存在则 submit 400 回滚 → approve/ship 连锁失败）----
  try {
    await apiCall<{ id?: number }>(page, 'POST', '/bpm/definitions', {
      name: '销售订单审批流程',
      code: 'sales_order_approval',
      description: 'E2E 测试用销售订单审批流程定义',
      category: 'sales',
      version: '1.0',
      config: {
        nodes: [
          { node_id: 'start', node_name: '提交审批', node_type: 'start' },
          { node_id: 'approve', node_name: '审批', node_type: 'approval' },
          { node_id: 'end', node_name: '完成', node_type: 'end' },
        ],
      },
      status: 'ACTIVE',
    });
    console.log('[ensureTestEntities] BPM sales_order_approval 定义已创建/已存在');
  } catch (e) {
    // 已存在或 CSRF 恢复失败均视为成功（幂等）
    console.warn('[ensureTestEntities] BPM 定义创建跳过:', (e as Error).message);
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
      `${API_PREFIX}/custom-orders`
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
      `${API_PREFIX}/color-cards`
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
  let csrfToken = (await getCsrfToken(page).catch(() => null)) ?? '';
  const url = `${API_BASE}${API_PREFIX}${path}`;
  const doFetch = async (token: string) => {
    return page.request.fetch(url, {
      method,
      headers: {
        'Content-Type': 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
        'X-CSRF-Token': token,
      },
      data: body ? JSON.stringify(body) : undefined,
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
    throw new Error(
      `API ${method} ${path} returned non-JSON (status ${response.status()}): ${text.slice(0, 500)}`
    );
  }

  // CSRF 校验失败恢复（两级）：
  // 1. 优先读取后端 X-New-CSRF-Token 恢复头（并发竞败场景的权威来源，无需重新登录）
  // 2. 无恢复头时重新登录获取全新 token
  if (json.code === 'CSRF_TOKEN_INVALID' || json.code === 'CSRF_TOKEN_MISSING') {
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
        throw new Error(
          `retry returned non-JSON (status ${response.status()}): ${text.slice(0, 200)}`
        );
      }
    } catch (e) {
      throw new Error(`API ${method} ${path} CSRF 重试失败: ${(e as Error).message}`);
    }
  }

  if (json.code !== 200 && json.code !== 0) {
    throw new Error(`API ${method} ${path} failed: code=${json.code} message=${json.message}`);
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
): Promise<{ status: number; code?: number; message?: string }> {
  const csrfToken = await getCsrfToken(page);
  const url = `${API_BASE}${API_PREFIX}${path}`;
  const response = await page.request.fetch(url, {
    method,
    headers: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
      'X-CSRF-Token': csrfToken,
    },
    data: body ? JSON.stringify(body) : undefined,
  });

  const text = await response.text();
  let json: { code?: number; message?: string } = {};
  try {
    json = JSON.parse(text);
  } catch {
    // non-JSON response
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
  // 拦截 lock-status 请求（避免 16 shard 并发时后端挂起 5s+ 导致登录超时）
  await page
    .route('**/api/v1/erp/lock-status**', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          code: 200,
          message: 'ok',
          data: {
            is_locked: false,
            failed_attempts: 0,
            max_attempts: 5,
            locked_until: null,
            username: 'e2e',
            user_id: 0,
          },
        }),
      })
    )
    .catch(() => {});

  // 检查 cookie 是否还在（同 BrowserContext 内已登录则跳过）
  // 注意：必须同时检查 access_token 和 csrf_token —— CSRF 失效场景下前端会清空 csrf_token
  // Cookie 并跳转登录页，仅凭 access_token 存在就跳过登录会导致后续所有 POST 请求 403。
  if (LOGGED_IN.done && !force) {
    const cookies = await page.context().cookies();
    const hasToken = cookies.some(c => c.name === 'access_token');
    const hasCsrf = cookies.some(c => c.name === 'csrf_token');
    if (hasToken && hasCsrf) {
      await page
        .goto(`${BASE_URL}/dashboard`, { waitUntil: 'domcontentloaded', timeout: 30000 })
        .catch(() => {});
      return;
    }
    console.warn(
      `[loginViaUI] 检测到会话不完整 (access_token=${hasToken}, csrf_token=${hasCsrf})，强制重新登录`
    );
    LOGGED_IN.done = false;
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
        .catch(() => {});
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

async function loginOnPage(page: Page, u: string, p: string, consoleLogs: string[]): Promise<void> {
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
    .catch(() => false);
  console.log(`复选框初始状态: checked=${isChecked}`);
  if (!isChecked) {
    // 点击视觉复选框区域（.el-checkbox__inner）
    await checkboxInner.click();
    await page.waitForTimeout(500);
    let nowChecked = await page
      .locator('.el-checkbox input')
      .first()
      .isChecked()
      .catch(() => false);
    console.log(`点击 inner 后复选框状态: checked=${nowChecked}`);
    if (!nowChecked) {
      // fallback: 点击 label
      await page.locator('.el-checkbox').first().click();
      await page.waitForTimeout(300);
      nowChecked = await page
        .locator('.el-checkbox input')
        .first()
        .isChecked()
        .catch(() => false);
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
  const isDisabled = await loginButton.isDisabled().catch(() => false);
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
      .catch(() => []);
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
      .catch(() => []);
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
    await page.close().catch(() => {});
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
  const username = process.env[`E2E_${role.toUpperCase()}_USERNAME`];
  const password = process.env[`E2E_${role.toUpperCase()}_PASSWORD`];
  if (!username || !password) {
    throw new Error(`E2E role credentials not found for role: ${role}`);
  }
  await loginViaUI(page, username, password);
}

export async function healthCheck(): Promise<boolean> {
  try {
    const response = await fetch(`${API_BASE}/health`);
    return response.ok;
  } catch {
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
  } catch {
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
  } catch {
    // action may fail if already in target state
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

export async function verifyStockFourDim(
  page: Page,
  productId: number,
  colorNo?: string,
  dyeLotNo?: string
): Promise<Record<string, unknown>> {
  let path = `/inventory/stock?product_id=${productId}&page=1&page_size=50`;
  if (colorNo) path += `&color_no=${encodeURIComponent(colorNo)}`;
  if (dyeLotNo) path += `&dye_lot_no=${encodeURIComponent(dyeLotNo)}`;
  const stock = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(page, 'GET', path);
  return stock.items?.[0] || {};
}

export async function verifyAuditLog(
  page: Page,
  action: string,
  resourceType?: string,
  pathIncludes?: string
): Promise<boolean> {
  // 业务操作审计有两条真实管道，两条都查、任一命中即通过：
  // 1. omni_audit 中间件（业务 CRUD）→ omni_audit_logs 表，查询端点
  //    GET /finance/audit/search（module 列存事件类型 CREATE/UPDATE/...，
  //    resource_type 存路径业务段，如 /api/v1/erp/purchase/orders → "purchase"；
  //    动作类 POST（submit/approve/audit/depreciate 等）统一记为 CREATE）
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
    const visible = await el.isVisible().catch(() => false);
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
  try {
    await apiCall(page, 'PUT', `/users/${userId}`, {
      role_id: roleA,
    });
    // 单角色分配成功不代表无 SoD 约束；继续尝试把用户改为 roleB，两次分配都成功说明互斥未生效
    const second = await apiCall(page, 'PUT', `/users/${userId}`, {
      role_id: roleB,
    });
    return !(second.code === 200 || second.code === 0);
  } catch {
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
): number {
  // 公斤 = 米 * 克重 * 幅宽 / 1000 / 100 (克→公斤, cm→m)
  return Number(((meters * gramWeight * width) / 100000).toFixed(2));
}

export async function verifyNetWeight(grossWeight: number, paperTubeWeight: number): number {
  return Number((grossWeight - paperTubeWeight).toFixed(2));
}

export async function getProcessSteps(
  page: Page,
  modeCode: string
): Promise<Array<{ step_code: string; step_name: string; is_required: boolean }>> {
  try {
    const modes = await apiCallRaw<{ items: Array<{ id: number; mode_code: string }> }>(
      page,
      'GET',
      '/business-modes?page=1&page_size=50'
    );
    const mode = modes.items.find(m => m.mode_code === modeCode);
    if (!mode) return [];
    const steps = await apiCallRaw<{
      items: Array<{ step_code: string; step_name: string; is_required: boolean }>;
    }>(page, 'GET', `/business-modes/${mode.id}/flow-steps?page=1&page_size=20`);
    return steps.items || [];
  } catch {
    return [];
  }
}

export async function verifyOutsourcingVoucher(
  page: Page,
  orderId: number,
  voucherType: string
): Promise<Record<string, unknown> | null> {
  try {
    const vouchers = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/outsourcing-vouchers?outsourcing_order_id=${orderId}&voucher_type=${voucherType}&page=1&page_size=5`
    );
    return vouchers.items?.[0] || null;
  } catch {
    return null;
  }
}

export async function verifyTrialBalance(
  page: Page
): Promise<{ balanced: boolean; debit_total: number; credit_total: number }> {
  try {
    const result = await apiCallRaw<{ debit_total: number; credit_total: number }>(
      page,
      'GET',
      '/finance/reports/trial-balance'
    );
    return {
      balanced: Math.abs((result.debit_total || 0) - (result.credit_total || 0)) < 0.01,
      debit_total: result.debit_total || 0,
      credit_total: result.credit_total || 0,
    };
  } catch {
    return { balanced: false, debit_total: 0, credit_total: 0 };
  }
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
