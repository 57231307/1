import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 面料销售订单（/sales/fabric-orders）审核拒绝 API 契约层。
 *
 * 后端事实（routes/sales.rs 注册，勿改后端）：
 *   POST /api/v1/erp/sales/fabric-orders/{id}/reject
 *     → sales_fabric_order_handler::reject_fabric_order
 *     请求体 { "reason": "<必填非空>" }；成功返回掩码后的订单行，
 *     状态写 rejected、理由落 rejected_reason；非 pending 行 → 400 BUSINESS_ERROR。
 *
 * 说明：
 *   - 操作人身份由后端按服务端会话（AuthContext.user_id）派生，请求体不承载操作人；
 *     与 approve/dye-recipe reject 同口径，多传键会被 check-api-request 判失配。
 *   - 响应为「掩码后的 fabric_order 行」，后端当前无对应生成类型/OpenAPI 源可对齐，
 *     且本资源在前端暂无消费页面（见交付说明），故不手抄字段以免漂移，
 *     类型仅表达「返回一个 JSON 对象行」。NOT NULL 语义由后端契约保证，前端不臆造列名。
 */
export function rejectFabricOrder(
  id: number,
  reason: string
): Promise<ApiResponse<Record<string, unknown>>> {
  return request.post(`/sales/fabric-orders/${id}/reject`, { reason });
}
