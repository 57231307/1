import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 委外订单出参（GET 列表/详情/状态机迁移响应 data 载荷）——键名/可空性逐字段对齐后端
 * backend/src/models/outsourcing_order.rs Model（DeriveEntityModel+Serialize，无 rename，
 * 出参即 snake_case）。NOT NULL 列不标可选；rust_decimal 列出参为字符串（如 "100.0000"），
 * 参与运算/回显数值输入前必须 Number() 归一；Date 列出参为 "YYYY-MM-DD" 字符串。
 * 状态词值域见 utils/outsourcing-status.ts（权威：models/status/wage_energy_chemical_business.rs::outsourcing_order_status）。
 */
export interface OutsourcingOrder {
  id: number;
  order_no: string;
  order_type: string;
  supplier_id: number;
  production_order_id: number | null;
  dye_batch_id: number | null;
  color_no: string | null;
  dye_lot_no: string | null;
  issue_date: string;
  expected_return_date: string | null;
  actual_return_date: string | null;
  issue_quantity: string;
  issue_unit: string;
  return_quantity: string;
  loss_quantity: string;
  loss_type: string | null;
  loss_rate: string | null;
  standard_loss_rate: string | null;
  material_cost: string;
  processing_fee: string;
  freight_fee: string;
  tax_amount: string;
  abnormal_loss_amount: string;
  total_cost: string;
  unit_cost: string;
  status: string;
  voucher_no_issue: string | null;
  voucher_no_fee: string | null;
  voucher_no_receipt: string | null;
  remarks: string | null;
  is_deleted: boolean;
  created_by: number | null;
  created_at: string;
  updated_at: string;
}

export interface CreateOutsourcingOrderPayload {
  order_no: string;
  order_type: string;
  supplier_id: number;
  production_order_id?: number;
  dye_batch_id?: number;
  color_no?: string;
  dye_lot_no?: string;
  issue_date: string;
  expected_return_date?: string;
  issue_quantity: number;
  issue_unit?: string;
  // 后端 CreateOutsourcingOrderRequest.material_cost 为 rust_decimal::Decimal 必填字段，
  // 与本文件其它金额字段(issue_quantity/unit_cost 等)一致，统一以 JSON number 传参序列化。
  material_cost: number;
  /**
   * 加工费/运费/进项税额：outsourcing_order 表 NOT NULL DECIMAL(14,4) 真实列
   * （backend migration/src/domain/v15/mod.rs:3247-3249）。后端 Create DTO 类型层
   * 非 Option（NOT NULL 列不标可选，显式 null 被 serde 类型校验拒绝）；
   * 建单缺省键=0 起步（serde(default)，与后端建单初始化同值），真实值须随单提交或
   * draft 期经 updateOutsourcingOrder 补录——结算 FEE 凭证金额=加工费+运费，
   * 不录入则成本链恒 0。
   */
  processing_fee?: number;
  freight_fee?: number;
  tax_amount?: number;
}

/**
 * 列表端点出参逐端点定型（禁止调用侧再写 data.list / items / 裸数组 双形状探测）：
 * GET /production/outsourcing-orders → ApiResponse<PaginatedResponse>
 * （handlers/outsourcing_handler.rs:113-137 返回类型直书，分页唯一形状 {items,total,page,page_size}，
 * utils/response.rs:38-43）。
 */
export function getOutsourcingOrderList(
  params?: Record<string, unknown>
): Promise<ApiResponse<PaginatedResponse<OutsourcingOrder>>> {
  return request.get<ApiResponse<PaginatedResponse<OutsourcingOrder>>>(
    '/production/outsourcing-orders',
    {
      params,
    }
  );
}

export function createOutsourcingOrder(
  data: CreateOutsourcingOrderPayload
): Promise<ApiResponse<OutsourcingOrder>> {
  return request.post<ApiResponse<OutsourcingOrder>>('/production/outsourcing-orders', data);
}

export function getOutsourcingOrderByNo(no: string): Promise<ApiResponse<OutsourcingOrder>> {
  return request.get<ApiResponse<OutsourcingOrder>>(`/production/outsourcing-orders/by-no/${no}`);
}

export function getOutsourcingOrderDetail(id: number): Promise<ApiResponse<OutsourcingOrder>> {
  return request.get<ApiResponse<OutsourcingOrder>>(`/production/outsourcing-orders/${id}`);
}

/**
 * 更新委外订单载荷 —— 逐字段对齐后端 UpdateOutsourcingOrderRequest
 * （services/outsourcing_ops/types.rs，三态语义 RFC 7386 JSON Merge Patch）：
 * 键缺席=保持原值、显式 null=清空为 NULL（仅下方声明 `| null` 的 DB 可空列）、有值=覆盖。
 * 可空列依据 v15 outsourcing_order DDL（production_order_id/dye_batch_id/color_no/dye_lot_no/
 * expected_return_date/standard_loss_rate/remarks）。
 * NOT NULL 列（order_type/supplier_id/issue_date/issue_quantity/issue_unit/material_cost/
 * processing_fee/freight_fee/tax_amount）不声明 null——显式 null 会被后端
 * business_displayable 拒绝（"XX不能清空：该字段为必填项"）。
 * 委外单号 order_no 建单生成后不可改，后端更新结构无此字段，不得提交。
 * material_cost/issue_quantity/standard_loss_rate/三费 为 rust_decimal 入参，以 JSON number 提交。
 */
export interface UpdateOutsourcingOrderPayload {
  order_type?: string;
  supplier_id?: number;
  production_order_id?: number | null;
  dye_batch_id?: number | null;
  color_no?: string | null;
  dye_lot_no?: string | null;
  issue_date?: string;
  expected_return_date?: string | null;
  issue_quantity?: number;
  issue_unit?: string;
  material_cost?: number;
  /** 加工费（NOT NULL 列，后端 v15:3247）：有值覆盖/键缺席保持，禁显式 null */
  processing_fee?: number;
  /** 运费（NOT NULL 列，后端 v15:3248）：同上 */
  freight_fee?: number;
  /** 进项税额（NOT NULL 列，后端 v15:3249）：同上，结算 FEE 凭证 tax_amount 来源 */
  tax_amount?: number;
  standard_loss_rate?: number | null;
  remarks?: string | null;
}

export function updateOutsourcingOrder(
  id: number,
  data: UpdateOutsourcingOrderPayload
): Promise<ApiResponse<OutsourcingOrder>> {
  return request.put<ApiResponse<OutsourcingOrder>>(`/production/outsourcing-orders/${id}`, data);
}

export function deleteOutsourcingOrder(id: number): Promise<ApiResponse<null>> {
  return request.delete<ApiResponse<null>>(`/production/outsourcing-orders/${id}`);
}

// 后端 issue/processing/settle/close/cancel handler 仅 Path(id) + State，无 Json<T> 提取器（状态机迁移，
// 载荷不参与业务），此前前端发送 `data ?? {}` 空体属多余请求体（被 axum 丢弃但契约占位）——
// 状态词表小写 received/settled/processing（models/status/wage_energy_chemical_business.rs:262-277），
// 前端比较值须逐字符一致（门控值与标签一律引 utils/outsourcing-status.ts，同源于本词表）。
// 三端点均不带请求体调用；出参 data = 迁移后的 outsourcing_order 行（outsourcing_handler.rs:185-226）。
export function issueOutsourcingOrder(id: number): Promise<ApiResponse<OutsourcingOrder>> {
  return request.post<ApiResponse<OutsourcingOrder>>(`/production/outsourcing-orders/${id}/issue`);
}

export function processOutsourcingOrder(id: number): Promise<ApiResponse<OutsourcingOrder>> {
  return request.post<ApiResponse<OutsourcingOrder>>(
    `/production/outsourcing-orders/${id}/processing`
  );
}

/**
 * 结算：received → settled。后端两条硬拒（均 400，调用前界面须同口径门控，不可点了才吃 400）：
 * 状态门 order.rs:594-599（仅 received）；费用门 order.rs:607-611（processing_fee + freight_fee <= 0 拒）。
 */
export function settleOutsourcingOrder(id: number): Promise<ApiResponse<OutsourcingOrder>> {
  return request.post<ApiResponse<OutsourcingOrder>>(`/production/outsourcing-orders/${id}/settle`);
}

export function closeOutsourcingOrder(id: number): Promise<ApiResponse<OutsourcingOrder>> {
  return request.post<ApiResponse<OutsourcingOrder>>(`/production/outsourcing-orders/${id}/close`);
}

export function cancelOutsourcingOrder(id: number): Promise<ApiResponse<OutsourcingOrder>> {
  return request.post<ApiResponse<OutsourcingOrder>>(`/production/outsourcing-orders/${id}/cancel`);
}

/**
 * 委外发料明细出参 —— 键对齐 backend/src/models/outsourcing_order_item.rs Model；
 * GET .../items/by-order/{id} 的 data 是**裸数组**（outsourcing_handler.rs:234-240
 * ApiResponse<Vec<Model>>），不是 {items}，读取处直接取 res.data。
 */
export interface OutsourcingOrderItem {
  id: number;
  outsourcing_order_id: number;
  product_id: number;
  color_no: string | null;
  dye_lot_no: string | null;
  batch_no: string | null;
  warehouse_id: number | null;
  quantity: string;
  unit: string;
  unit_cost: string;
  total_cost: string;
  processing_fee: string;
  freight_fee: string;
  inventory_transaction_id: number | null;
  greige_fabric_id: number | null;
  piece_no: string | null;
  remarks: string | null;
  created_at: string;
  updated_at: string;
}

export function getOutsourcingItems(orderId: number): Promise<ApiResponse<OutsourcingOrderItem[]>> {
  return request.get<ApiResponse<OutsourcingOrderItem[]>>(
    `/production/outsourcing-orders/items/by-order/${orderId}`
  );
}

export function createOutsourcingItem(orderId: number, data: Record<string, unknown>) {
  return request.post('/production/outsourcing-orders/items', {
    outsourcing_order_id: orderId,
    ...data,
  });
}

/**
 * 收回单出参 —— 键对齐 backend/src/models/outsourcing_receipt.rs Model（snake_case）。
 * 状态词值域 draft/confirmed/cancelled（权威 outsourcing_receipt_status，
 * wage_energy_chemical_business.rs:288-295；utils/outsourcing-status.ts 目前**只收录订单态**，
 * 收回态映射缺失已交回编排者，见 报告）。
 */
export interface OutsourcingReceipt {
  id: number;
  receipt_no: string;
  outsourcing_order_id: number;
  receipt_date: string;
  product_id: number;
  color_no: string | null;
  dye_lot_no: string | null;
  batch_no: string | null;
  warehouse_id: number | null;
  return_quantity: string;
  loss_quantity: string;
  loss_type: string | null;
  loss_rate: string | null;
  is_loss_normal: boolean;
  unit_cost: string;
  total_cost: string;
  abnormal_loss_amount: string;
  quality_status: string | null;
  grade: string | null;
  // 打卷实测值三列（m0075 DB 可空列，CHECK >0 或 NULL）：键逐字符对齐
  // backend/src/models/outsourcing_receipt.rs:72,75,78（Option<Decimal>）。
  // rust_decimal 序列化为 JSON **字符串**（如 "12.5000"）→ string 而非 number；
  // null = 未补录。参与数值运算/回显数值输入控件前须 Number() 归一，
  // 直接把出参当 number 用（.toFixed 等）属类型谎言（运行期崩过）。
  weight: string | null;
  width: string | null;
  gram_weight: string | null;
  inventory_transaction_id: number | null;
  inspection_id: number | null;
  status: string;
  remarks: string | null;
  is_deleted: boolean;
  created_by: number | null;
  created_at: string;
  updated_at: string;
}

/**
 * GET /production/outsourcing-receipts → ApiResponse<PaginatedResponse>
 * （outsourcing_handler.rs:275-295），列表唯一读法 = res.data.items。
 */
export function getOutsourcingReceiptList(
  params?: Record<string, unknown>
): Promise<ApiResponse<PaginatedResponse<OutsourcingReceipt>>> {
  return request.get<ApiResponse<PaginatedResponse<OutsourcingReceipt>>>(
    '/production/outsourcing-receipts',
    {
      params,
    }
  );
}

export function createOutsourcingReceipt(data: Record<string, unknown>) {
  return request.post('/production/outsourcing-receipts', data);
}

/**
 * 更新收回单载荷 —— 逐字段对齐后端 UpdateOutsourcingReceiptRequest
 * （services/outsourcing_ops/types.rs:243-289，三态语义 RFC 7386）：
 * 键缺席=保持原值、显式 null=清空为 NULL（仅 DB 可空列）、有值=覆盖。
 * 仅 draft 状态可更新（receipt.rs:316-321）。
 * NOT NULL 列（receipt_date/product_id/return_quantity/loss_quantity）不声明 null——
 * 显式 null 被后端"不能清空"拒绝（receipt.rs:296-313）。
 * 打卷实测值三列 weight/width/gram_weight 为 DB 可空 DECIMAL（types.rs:282-288，
 * 值域 >0，validate_measured_values receipt.rs:76-86）：传 null=清空回"未补录"，
 * 清空仅退回 NULL，不存在回落主数据语义。rust_decimal 入参以 JSON number 提交。
 */
export interface UpdateOutsourcingReceiptPayload {
  /** 收回日期（NOT NULL 列）：有值覆盖/键缺席保持，禁显式 null */
  receipt_date?: string;
  /** 成品 ID（NOT NULL 列）：同上 */
  product_id?: number;
  /** 色号（DB 可空）：显式 null=清空 */
  color_no?: string | null;
  /** 缸号（DB 可空）：显式 null=清空 */
  dye_lot_no?: string | null;
  /** 匹号（DB 可空）：显式 null=清空 */
  batch_no?: string | null;
  /** 入库仓库 ID（DB 可空）：显式 null=清空 */
  warehouse_id?: number | null;
  /** 收回数量（NOT NULL 列，>0）：禁显式 null */
  return_quantity?: number;
  /** 损耗数量（NOT NULL 列）：禁显式 null */
  loss_quantity?: number;
  /** 质检结论（DB 可空，词表 outsourcing_receipt_quality_status）：显式 null=清空 */
  quality_status?: string | null;
  /** 等级（DB 可空）：显式 null=清空 */
  grade?: string | null;
  /** 备注（DB 可空）：显式 null=清空 */
  remarks?: string | null;
  /** 实测重量 kg（DB 可空，>0）：null=清空回"未补录" */
  weight?: number | null;
  /** 实测幅宽 cm（DB 可空，>0）：null=清空回"未补录" */
  width?: number | null;
  /** 实测克重 g/m²（DB 可空，>0）：null=清空回"未补录" */
  gram_weight?: number | null;
}

export function updateOutsourcingReceipt(
  id: number,
  data: UpdateOutsourcingReceiptPayload
): Promise<ApiResponse<OutsourcingReceipt>> {
  return request.put<ApiResponse<OutsourcingReceipt>>(
    `/production/outsourcing-receipts/${id}`,
    data
  );
}

/**
 * 确认收回单：draft → confirmed，并把关联委外订单推进 received（receipt.rs:510）。
 * 后端两条硬拒（400）：状态门 receipt.rs:326-331（仅 draft）；数量门 receipt.rs:338-342
 * （return_quantity <= 0 拒，存量 0 量草稿同样拦住）。
 */
export function confirmOutsourcingReceipt(id: number): Promise<ApiResponse<OutsourcingReceipt>> {
  return request.post<ApiResponse<OutsourcingReceipt>>(
    `/production/outsourcing-receipts/${id}/confirm`
  );
}
