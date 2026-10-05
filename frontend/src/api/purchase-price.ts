import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 列表出参 = 后端 PurchasePriceView（services/purchase_price_service.rs get_prices_list），
// 详情/历史出参 = purchase_price::Model（get_price/get_price_history，handlers/purchase_price_handler.rs）。
// 审批理由两列（approval_reason/rejected_reason，models/purchase_price.rs:26-28，m0079 加列）本接口
// 刻意不声明：当前无消费方展示，且列表读模型 PurchasePriceView 不携带这两列
// （services/purchase_price_service.rs:20-40），在行型上补键只会造出恒空的假列。
// status 词表权威 = backend/src/models/status/sales.rs:175-190 price_approval（全小写）；采购侧四值均有写入方：
// 建单 pending（purchase_price_service.rs create_price）、审批 approved（::approve_price）、
// 拒绝 rejected（::reject_price）、update_price 按 price_approval::ALL 校验后透传（::update_price）；
// DB CHECK chk_purchase_price_status 与 {pending,approved,rejected,inactive} 恰等
// （migration/src/domain/price_vocab_check 施加窄集 + 后继
// migration/src/domain/price_vocab_extend/m0080_extend_price_status_check_rejected.rs 扩 rejected）；
// 列表 status 筛选白名单同源 price_approval::ALL（handlers/purchase_price_handler.rs:51-59）。
// 旧注释误指向 general.rs 的 master_data（值恰同、指向错），本次纠正。表内无产品/供应商名称列，需后端 JOIN。
// 读接口 Decimal 线格式：rust_decimal 在 Cargo.toml:60 仅启用 serde、未启 serde-float
// ⇒ 序列化为 JSON 字符串；模型证据 backend/src/models/purchase_price.rs:14 price / :17 min_order_qty
// 均为 Decimal 且非 Option ⇒ 线格式恒在；建表列 NOT NULL：m0009_add_purchase_extensions.rs:242/:245。
// 前端如实声明为 string，数值解析与定点格式化统一走 composables/ppFmts.ts 的 formatCurrency。
// 注：刻意不 export —— api/index.ts 对 sales-price/purchase-price 两侧 `export *`，
// 与销售侧同名导出会触发 TS2308 重复导出（vue-tsc 实测），类型别名模块私有即可满足使用。
type DecimalString = string;

export interface PurchasePrice {
  id: number;
  product_id: number;
  supplier_id: number;
  // DB price DECIMAL(18,6) NOT NULL（m0009_add_purchase_extensions.rs:242）
  price: DecimalString;
  currency: string;
  unit: string;
  // DB min_order_qty DECIMAL(12,2) NOT NULL（同上 :245）⇒ NOT NULL 列前端接口不写 `?`
  min_order_qty: DecimalString;
  price_type: string;
  effective_date: string;
  expiry_date: string | null;
  // 取值域 = 采购侧写入方全集 = price_approval 四值（见文件头注），与 chk_purchase_price_status 恰等
  status: 'pending' | 'approved' | 'rejected' | 'inactive';
  approved_by: number | null;
  approved_at: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
  /** 需要后端 JOIN：purchase_prices.product_id -> products.product_name（表内无名列） */
  product_name?: string | null;
  /** 需要后端 JOIN：purchase_prices.product_id -> products.product_code（表内无编码列） */
  product_code?: string | null;
  /** 需要后端 JOIN：purchase_prices.supplier_id -> suppliers.supplier_name（表内无名列） */
  supplier_name?: string | null;
}

/**
 * GET /purchase/purchase-prices 查询参数。对应后端 purchase_price_handler::PurchasePriceQuery
 * （backend/src/handlers/purchase_price_handler.rs:16），无 rename_all → snake_case，全 Option → 可选。
 * 注意：后端不读 keyword/product_name/supplier_name（原 QueryParams/视图里的这些键被 Axum 静默丢弃）。
 */
export interface PurchasePriceQueryParams {
  product_id?: number;
  supplier_id?: number;
  status?: string;
  page?: number;
  page_size?: number;
}

/**
 * 创建采购价格请求（严格对齐 backend CreatePurchasePriceInput，
 * services/purchase_price_service.rs:59-70）。
 * unit/price_type 必填：后端列 NOT NULL 无 DB 默认（m0009），缺失即被 400 校验拒绝。
 * Decimal 入参线格式：rust_decimal 的反序列化对 JSON 字符串与整数都接受，**浮点字面量同样被接受**
 * （rust_decimal 在本仓构建图启用的 base serde feature 下走 visit_f64），不会因此 400；
 * 但浮点只有约 17 位有效数字，DECIMAL(18,6) 的末位会在静默失真中丢失，所以 string 才是
 * 精确且全域的写线格式。这里暂以 `string | number` 放宽，是因为既有调用点 usePp.ts 的 formData
 * 以 number 初始化 price/min_order_qty（:70/:73），该文件不在本轮派工范围；收敛为 string 的
 * 收口项与销售侧同形（销售侧已完成，见 api/sales-price.ts 的 CreateInput/UpdateInput）。
 */
type DecimalWire = string | number;

export interface CreatePurchasePricePayload {
  product_id: number;
  supplier_id: number;
  price: DecimalWire;
  unit: string;
  price_type: string;
  currency?: string;
  min_order_qty?: DecimalWire;
  effective_date?: string;
  expiry_date?: string;
}

/**
 * 更新采购价格请求（严格对齐 backend UpdatePriceRequest，
 * handlers/purchase_price_handler.rs:36-40）。
 * 后端 price 是 String（非数值），必须以字符串形式提交（否则 Serde 反序列化失败）。
 * status 透传前由后端按 price_approval::ALL 白名单校验（purchase_price_service.rs::update_price），
 * 前端如实收敛为采购侧写入方全集四值。
 */
export interface UpdatePurchasePricePayload {
  price: string;
  expiry_date?: string;
  status?: 'pending' | 'approved' | 'rejected' | 'inactive';
}

export function getPurchasePriceList(
  params?: PurchasePriceQueryParams
  // 后端 purchase_price_handler::list_prices 返回 ApiResponse<Vec<Model>> ⇒ 裸数组
): Promise<ApiResponse<PurchasePrice[]>> {
  return request.get('/purchase/purchase-prices', { params });
}

export function getPurchasePrice(id: number): Promise<ApiResponse<PurchasePrice>> {
  return request.get(`/purchase/purchase-prices/${id}`);
}

export function createPurchasePrice(
  data: CreatePurchasePricePayload
): Promise<ApiResponse<PurchasePrice>> {
  return request.post('/purchase/purchase-prices', data);
}

export function updatePurchasePrice(
  id: number,
  data: UpdatePurchasePricePayload
): Promise<ApiResponse<null>> {
  return request.put(`/purchase/purchase-prices/${id}`, data);
}

/**
 * 审批「通过」请求体（严格对齐 backend ApprovePriceRequest，handlers/purchase_price_handler.rs:28-35，
 * serde 无 rename ⇒ snake_case 同名）。approved 为 `bool`（非 Option）⇒ 线上必带该键；
 * handler 的 approve_price 只受理批准，approved=false 直接被拒并指向真实存在的拒绝端点，
 * 故类型收敛为字面量 true，杜绝误发 false 走成必然失败的调用。
 * approval_reason 后端为 Option<String> 只为让"缺键"落到统一 AppError 校验信封（而非 axum 解码层
 * 裸 400），必填语义在 handler 收口：缺失/空串/纯空白一律 400，非空值真实落
 * purchase_prices.approval_reason 列 ⇒ 前端必带且必采（采集见 composables/useActionPrompts.ts）。
 * 旧键 remark 已从后端 DTO 删除 ⇒ 不再声明。
 */
export interface ApprovePurchasePriceRequest {
  approved: true;
  approval_reason: string;
}

/**
 * 批准采购价格（pending → approved）。
 * 路径/方法逐字对齐后端：POST /api/v1/erp/purchase/purchase-prices/{id}/approve
 * （routes/purchase.rs nest + `/purchase-prices/{id}/approve` → purchase_price_handler::approve_price；
 * 服务层 approve_price 事务 + lock_exclusive + 仅 pending 可批的状态门 + 审计）。
 * 成功出参 ApiResponse<()> ⇒ data 为 null，与 updatePurchasePrice 同形。
 */
export function approvePurchasePrice(
  id: number,
  data: ApprovePurchasePriceRequest
): Promise<ApiResponse<null>> {
  return request.post(`/purchase/purchase-prices/${id}/approve`, data);
}

/**
 * 审批「拒绝」请求体（对齐后端 RejectPriceRequest，handlers/purchase_price_handler.rs:37-41，
 * reason: String 非 Option 且抽取器为强类型 `Json<T>` ⇒ 必带体）。
 * handler 的 reject_price 对 trim 后空串回 400；非空值真实落 purchase_prices.rejected_reason 列，
 * 状态门仅 pending 起拒（purchase_price_service.rs::reject_price）。
 */
export interface RejectPurchasePriceRequest {
  reason: string;
}

/**
 * 拒绝采购价格（pending → rejected 终态）。
 * 路径逐字对齐后端：POST /api/v1/erp/purchase/purchase-prices/{id}/reject
 * （routes/purchase.rs `/purchase-prices/{id}/reject` → purchase_price_handler::reject_price）。
 */
export function rejectPurchasePrice(
  id: number,
  data: RejectPurchasePriceRequest
): Promise<ApiResponse<null>> {
  return request.post(`/purchase/purchase-prices/${id}/reject`, data);
}

export function deletePurchasePrice(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchase/purchase-prices/${id}`);
}

export function getPurchasePriceHistory(productId: number): Promise<ApiResponse<PurchasePrice[]>> {
  return request.get(`/purchase/purchase-prices/history/${productId}`);
}
