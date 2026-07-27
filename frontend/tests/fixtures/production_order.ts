/**
 * 生产订单域测试 mock 数据夹具（V15 批次 06 P1-6 修复）
 * 规则 6：测试 mock 数据禁止硬编码在测试用例中，统一抽取到 fixtures。
 * 覆盖生产订单（含 5 种状态：draft/planned/in_production/completed/cancelled）。
 */
import type { ProductionOrder } from '@/api/production'

/** 创建生产订单 mock（草稿状态，可通过 overrides 覆盖） */
export function createProductionOrderMock(
  overrides: Partial<ProductionOrder> = {},
): ProductionOrder {
  const now = new Date().toISOString()
  return {
    id: 1,
    order_no: 'MO20260101001',
    sales_order_id: 1,
    product_id: 1,
    product_name: '测试面料',
    planned_quantity: 1000,
    actual_quantity: 0,
    scheduled_start_date: now.slice(0, 10),
    scheduled_end_date: now.slice(0, 10),
    actual_start_date: undefined,
    actual_end_date: undefined,
    status: 'draft',
    priority: 3,
    work_center_id: 1,
    remark: '测试生产订单',
    created_at: now,
    updated_at: now,
    ...overrides,
  }
}

/** 创建已计划生产订单 mock */
export function createPlannedProductionOrderMock(
  overrides: Partial<ProductionOrder> = {},
): ProductionOrder {
  return createProductionOrderMock({ status: 'planned', ...overrides })
}

/** 创建生产中订单 mock（含实际开工日期与部分产量） */
export function createInProductionOrderMock(
  overrides: Partial<ProductionOrder> = {},
): ProductionOrder {
  const today = new Date().toISOString().slice(0, 10)
  return createProductionOrderMock({
    status: 'in_production',
    actual_start_date: today,
    actual_quantity: 500,
    ...overrides,
  })
}

/** 创建已完成订单 mock（含完工日期与全部产量） */
export function createCompletedProductionOrderMock(
  overrides: Partial<ProductionOrder> = {},
): ProductionOrder {
  const today = new Date().toISOString().slice(0, 10)
  return createProductionOrderMock({
    status: 'completed',
    actual_start_date: today,
    actual_end_date: today,
    actual_quantity: 1000,
    ...overrides,
  })
}

/** 创建已取消订单 mock */
export function createCancelledProductionOrderMock(
  overrides: Partial<ProductionOrder> = {},
): ProductionOrder {
  return createProductionOrderMock({
    status: 'cancelled',
    remark: '客户取消',
    ...overrides,
  })
}

/** 创建生产订单列表 mock（默认覆盖 5 种状态） */
export function createProductionOrderListMock(count = 5): ProductionOrder[] {
  const factories = [
    createProductionOrderMock,
    createPlannedProductionOrderMock,
    createInProductionOrderMock,
    createCompletedProductionOrderMock,
    createCancelledProductionOrderMock,
  ]
  return Array.from({ length: count }, (_, i) => {
    const factory = factories[i % factories.length] ?? createProductionOrderMock
    return factory({
      id: i + 1,
      order_no: `MO20260101${String(i + 1).padStart(3, '0')}`,
      product_id: i + 1,
      product_name: `面料${i + 1}`,
    })
  })
}

/** 创建生产进度上报请求 mock（用于 POST /production-orders/:id/progress） */
export function createProgressReportMock(
  overrides: { completed_quantity?: number; defect_quantity?: number; remark?: string } = {},
) {
  return {
    completed_quantity: 100,
    defect_quantity: 2,
    remark: '进度上报',
    ...overrides,
  }
}

/** 创建审核请求 mock（用于 POST /production-orders/:id/approve） */
export function createApprovalRequestMock(
  overrides: { approved?: boolean; remark?: string } = {},
) {
  return {
    approved: true,
    remark: '审核通过',
    ...overrides,
  }
}
