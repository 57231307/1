/**
 * 库存域测试 mock 数据夹具（V15 P2 B06-P2-3 修复）
 * 规则 6：测试 mock 数据禁止硬编码在测试用例中，统一抽取到 fixtures。
 * 覆盖库存列表项、库存告警、库存调整等核心库存域实体。
 */
import type { InventoryStock, StockAlert, StockAdjustmentData } from '@/api/inventory';

/** 创建库存项 mock（默认 active 状态，可通过 overrides 覆盖） */
export function createInventoryStockMock(
  overrides: Partial<InventoryStock> = {}
): InventoryStock {
  const now = new Date().toISOString();
  return {
    id: 1,
    product_id: 1,
    product_name: '面料A',
    product_code: 'FAB-001',
    warehouse_id: 1,
    warehouse_name: '主仓库',
    quantity: 100,
    unit: '米',
    status: 'active',
    created_at: now,
    updated_at: now,
    ...overrides,
  };
}

/** 创建库存列表 mock（默认 2 个不同面料） */
export function createInventoryStockListMock(
  count = 2
): InventoryStock[] {
  return Array.from({ length: count }, (_, i) =>
    createInventoryStockMock({
      id: i + 1,
      product_name: `面料${String.fromCharCode(65 + i)}`,
      quantity: (i + 1) * 100,
    })
  );
}

/** 创建库存告警 mock（默认低库存告警） */
export function createStockAlertMock(overrides: Partial<StockAlert> = {}): StockAlert {
  return {
    id: 1,
    product_id: 1,
    product_name: '面料A',
    product_code: 'FAB-001',
    warehouse_id: 1,
    warehouse_name: '主仓库',
    current_quantity: 5,
    min_quantity: 20,
    unit: '米',
    alert_level: 'warning',
    ...overrides,
  };
}

/** 创建库存告警列表 mock（默认 1 个低库存告警） */
export function createStockAlertListMock(
  count = 1
): StockAlert[] {
  return Array.from({ length: count }, (_, i) =>
    createStockAlertMock({
      id: i + 1,
      alert_level: i % 2 === 0 ? 'warning' : 'danger',
    })
  );
}

/** 创建库存调整数据 mock（默认增加调整） */
export function createStockAdjustmentDataMock(
  overrides: Partial<StockAdjustmentData> = {}
): StockAdjustmentData {
  return {
    warehouse_id: 1,
    product_id: 1,
    adjustment_quantity: 10,
    adjustment_type: 'increase',
    reason: '测试调整',
    ...overrides,
  };
}
