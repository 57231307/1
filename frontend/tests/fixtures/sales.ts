/**
 * 销售域测试 mock 数据夹具（V15 批次 06 P1-6 修复）
 * 规则 6：测试 mock 数据禁止硬编码在测试用例中，统一抽取到 fixtures。
 * 覆盖销售订单、销售订单明细、发货单、报价单等核心销售域实体。
 */
import type { SalesOrder, SalesOrderItem, SalesDelivery } from '@/api/sales'
import type {
  QuotationStatus,
  CurrencyCode,
  PriceTerms,
  TierPricingItem,
  CreateQuotationDto,
  CreateQuotationItemDto,
} from '@/api/quotation'

/** 创建销售订单明细 mock */
export function createSalesOrderItemMock(
  overrides: Partial<SalesOrderItem> = {},
): SalesOrderItem {
  return {
    id: 1,
    product_id: 1,
    product_name: '测试面料',
    product_code: 'FAB-001',
    quantity: 100,
    unit: '米',
    unit_price: 50,
    tax_rate: 0.13,
    tax_amount: 650,
    discount_rate: 0,
    discount_amount: 0,
    subtotal: 5000,
    delivered_quantity: 0,
    delivered_amount: 0,
    ...overrides,
  }
}

/** 创建销售订单 mock（草稿状态，可通过 overrides 覆盖） */
export function createSalesOrderMock(
  overrides: Partial<SalesOrder> = {},
): SalesOrder {
  const now = new Date().toISOString()
  return {
    id: 1,
    order_no: 'SO20260101001',
    customer_id: 1,
    customer_name: '测试客户',
    order_date: now.slice(0, 10),
    status: 'draft',
    total_amount: 5000,
    tax_amount: 650,
    discount_amount: 0,
    contact_person: '李四',
    contact_phone: '13800000000',
    delivery_address: '上海市浦东新区',
    remark: '测试订单',
    creator_name: 'admin',
    created_at: now,
    updated_at: now,
    items: [createSalesOrderItemMock()],
    ...overrides,
  }
}

/** 创建已确认销售订单 mock */
export function createConfirmedSalesOrderMock(
  overrides: Partial<SalesOrder> = {},
): SalesOrder {
  return createSalesOrderMock({ status: 'confirmed', ...overrides })
}

/** 创建已完成销售订单 mock（含已发货明细） */
export function createCompletedSalesOrderMock(
  overrides: Partial<SalesOrder> = {},
): SalesOrder {
  return createSalesOrderMock({
    status: 'completed',
    items: [
      createSalesOrderItemMock({
        delivered_quantity: 100,
        delivered_amount: 5000,
      }),
    ],
    ...overrides,
  })
}

/** 创建销售订单列表 mock（默认 3 个） */
export function createSalesOrderListMock(count = 3): SalesOrder[] {
  const statuses = ['draft', 'confirmed', 'completed']
  return Array.from({ length: count }, (_, i) =>
    createSalesOrderMock({
      id: i + 1,
      order_no: `SO20260101${String(i + 1).padStart(3, '0')}`,
      status: statuses[i % statuses.length] ?? 'draft',
      total_amount: (i + 1) * 1000,
    }),
  )
}

/** 创建销售发货单 mock */
export function createSalesDeliveryMock(
  overrides: Partial<SalesDelivery> = {},
): SalesDelivery {
  const now = new Date().toISOString()
  return {
    id: 1,
    delivery_no: 'DN20260101001',
    order_id: 1,
    order_no: 'SO20260101001',
    customer_id: 1,
    customer_name: '测试客户',
    delivery_date: now.slice(0, 10),
    warehouse_id: 1,
    ...overrides,
  } as SalesDelivery
}

/** 创建阶梯定价项 mock */
export function createTierPricingMock(
  overrides: Partial<TierPricingItem> = {},
): TierPricingItem {
  return {
    min_quantity: 100,
    max_quantity: 499,
    unit_price: 48,
    unit_price_with_tax: 54.24,
    ...overrides,
  }
}

/** 创建报价单明细 mock */
export function createQuotationItemMock(
  overrides: Partial<CreateQuotationItemDto> = {},
): CreateQuotationItemDto {
  return {
    product_id: 1,
    color_id: 1,
    specification: '21S/2',
    unit: '米',
    quantity: 1000,
    unit_price: 50,
    unit_price_with_tax: 56.5,
    tier_pricing: [createTierPricingMock()],
    discount_rate: 0,
    notes: '',
    ...overrides,
  }
}

/** 创建报价单 DTO mock（用于 POST /quotations） */
export function createQuotationDtoMock(
  overrides: Partial<CreateQuotationDto> = {},
): CreateQuotationDto {
  const today = new Date().toISOString().slice(0, 10)
  return {
    customer_id: 1,
    sales_user_id: 1,
    quotation_date: today,
    valid_until: today,
    currency: 'CNY' as CurrencyCode,
    exchange_rate: 1,
    base_currency: 'CNY',
    price_terms: 'FOB' as PriceTerms,
    tax_inclusive: true,
    tax_rate: 0.13,
    moq: 100,
    lead_time_days: 30,
    customer_level: 'NORMAL',
    notes: '测试报价单',
    items: [createQuotationItemMock()],
    ...overrides,
  }
}

/** 创建报价单状态枚举值（便于参数化测试） */
export const QUOTATION_STATUS_SAMPLES: QuotationStatus[] = [
  'draft',
  'pending_approval',
  'approved',
  'rejected',
  'expired',
  'converted',
  'cancelled',
]
