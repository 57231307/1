/**
 * 染整域测试 mock 数据夹具（V15 批次 06 P1-6 修复）
 * 规则 6：测试 mock 数据禁止硬编码在测试用例中，统一抽取到 fixtures。
 * 覆盖染缸批次（4 种状态：pending/in_progress/completed/cancelled）。
 */
import type { DyeBatch } from '@/api/dye-batch'

/** 创建染缸批次 mock（待生产状态，可通过 overrides 覆盖） */
export function createDyeBatchMock(overrides: Partial<DyeBatch> = {}): DyeBatch {
  const now = new Date().toISOString()
  const today = now.slice(0, 10)
  return {
    id: 1,
    batch_no: 'DB20260101-01',
    color_code: '18-1664',
    color_name: '番茄红',
    greige_fabric_id: 1,
    greige_fabric_name: '胚布A',
    planned_quantity: 500,
    actual_quantity: 0,
    unit: '米',
    recipe_id: 1,
    recipe_name: '测试配方',
    status: 'pending',
    start_date: today,
    end_date: today,
    machine_code: 'MACHINE-01',
    operator: '张三',
    remark: '测试染缸批次',
    created_by: 1,
    created_by_name: 'admin',
    created_at: now,
    updated_at: now,
    ...overrides,
  }
}

/** 创建生产中染缸批次 mock（含部分实际产量） */
export function createInProgressDyeBatchMock(
  overrides: Partial<DyeBatch> = {},
): DyeBatch {
  return createDyeBatchMock({
    status: 'in_progress',
    actual_quantity: 250,
    ...overrides,
  })
}

/** 创建已完成染缸批次 mock（实际产量等于计划产量） */
export function createCompletedDyeBatchMock(
  overrides: Partial<DyeBatch> = {},
): DyeBatch {
  return createDyeBatchMock({
    status: 'completed',
    actual_quantity: 500,
    ...overrides,
  })
}

/** 创建已取消染缸批次 mock */
export function createCancelledDyeBatchMock(
  overrides: Partial<DyeBatch> = {},
): DyeBatch {
  return createDyeBatchMock({
    status: 'cancelled',
    remark: '计划变更取消',
    ...overrides,
  })
}

/** 创建染缸批次列表 mock（默认覆盖 4 种状态） */
export function createDyeBatchListMock(count = 4): DyeBatch[] {
  const factories = [
    createDyeBatchMock,
    createInProgressDyeBatchMock,
    createCompletedDyeBatchMock,
    createCancelledDyeBatchMock,
  ]
  return Array.from({ length: count }, (_, i) => {
    const factory = factories[i % factories.length] ?? createDyeBatchMock
    return factory({
      id: i + 1,
      batch_no: `DB20260101-${String(i + 1).padStart(2, '0')}`,
      color_code: `18-166${i}`,
      color_name: `颜色${i + 1}`,
      greige_fabric_id: i + 1,
      greige_fabric_name: `胚布${i + 1}`,
    })
  })
}

/** 创建染缸批次创建请求 mock（用于 POST /dye-batches） */
export function createDyeBatchCreateRequestMock(
  overrides: Partial<DyeBatch> = {},
): Partial<DyeBatch> {
  return {
    batch_no: 'DB20260102-01',
    color_code: '19-4052',
    color_name: '经典蓝',
    greige_fabric_id: 2,
    greige_fabric_name: '胚布B',
    planned_quantity: 800,
    unit: '米',
    recipe_id: 2,
    recipe_name: '新配方',
    start_date: new Date().toISOString().slice(0, 10),
    end_date: new Date().toISOString().slice(0, 10),
    machine_code: 'MACHINE-02',
    operator: '李四',
    remark: '新建染缸',
    ...overrides,
  }
}
