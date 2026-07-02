/**
 * 库存台账 POC - 测试数据生成器 + 逻辑层单元测试
 *
 * 注意: 真实的浏览器性能测试(FPS、内存)需在本地通过 node scripts/poc-perf-test.cjs 复现,
 * 沙箱环境无 GUI 浏览器,无法采集真实指标。本测试只覆盖"代码层逻辑正确性"。
 */
import { describe, expect, it } from 'vitest'
import { generateStocks, loadOrGenerateStocks, persistStocks } from '../fixtures/inventoryTestData'

describe('POC - testData.ts 数据生成器', () => {
  it('默认生成 10000 行', () => {
    const data = generateStocks()
    expect(data.length).toBe(10000)
  })

  it('生成自定义行数', () => {
    const data = generateStocks(100)
    expect(data.length).toBe(100)
  })

  it('字段完整性', () => {
    const data = generateStocks(50)
    const required = [
      'id',
      'product_code',
      'product_name',
      'warehouse_name',
      'batch_no',
      'color_code',
      'location',
      'quantity',
      'unit',
      'gram_weight',
      'width',
      'status',
      'min_quantity',
      'created_at',
    ]
    for (const row of data) {
      for (const key of required) {
        expect(row, `缺少字段: ${key}`).toHaveProperty(key)
      }
    }
  })

  it('product_code 唯一且递增', () => {
    const data = generateStocks(1000)
    const codes = new Set(data.map((r) => r.product_code))
    expect(codes.size).toBe(1000)
    expect(data[0].product_code).toBe('P000001')
    expect(data[999].product_code).toBe('P001000')
  })

  it('status 只能是 normal / warning / frozen', () => {
    const data = generateStocks(500)
    const valid = new Set(['normal', 'warning', 'frozen'])
    for (const r of data) {
      expect(valid.has(r.status), `非法 status: ${r.status}`).toBe(true)
    }
  })

  it('同一种子结果一致(可重复)', () => {
    const a = generateStocks(100, 7)
    const b = generateStocks(100, 7)
    expect(JSON.stringify(a)).toBe(JSON.stringify(b))
  })

  it('不同种子结果不一致', () => {
    const a = generateStocks(100, 1)
    const b = generateStocks(100, 2)
    expect(JSON.stringify(a)).not.toBe(JSON.stringify(b))
  })

  it('1 万行生成耗时 < 500ms', () => {
    const t0 = performance.now()
    generateStocks(10000)
    const t1 = performance.now()
    expect(t1 - t0).toBeLessThan(500)
  })

  it('persistStocks + loadOrGenerateStocks 往返', () => {
    const stored = persistStocks(200, 11)
    const loaded = loadOrGenerateStocks(200)
    expect(loaded.length).toBe(200)
    expect(loaded[0].product_code).toBe(stored[0].product_code)
  })
})

describe('POC - 排序与筛选行为(业务逻辑层)', () => {
  it('筛选:关键词命中 product_code', () => {
    const data = generateStocks(5000, 99)
    const kw = 'p000100'
    const result = data.filter((r) => r.product_code.toLowerCase().includes(kw))
    expect(result.length).toBeGreaterThan(0)
    for (const r of result) {
      expect(r.product_code.toLowerCase()).toContain(kw)
    }
  })

  it('筛选:按状态聚合', () => {
    const data = generateStocks(5000, 99)
    const warningCount = data.filter((r) => r.status === 'warning').length
    const normalCount = data.filter((r) => r.status === 'normal').length
    const frozenCount = data.filter((r) => r.status === 'frozen').length
    expect(warningCount + normalCount + frozenCount).toBe(5000)
  })

  it('排序:按 quantity 升序', () => {
    const data = generateStocks(200, 5)
    const sorted = [...data].sort((a, b) => a.quantity - b.quantity)
    for (let i = 1; i < sorted.length; i++) {
      expect(sorted[i].quantity).toBeGreaterThanOrEqual(sorted[i - 1].quantity)
    }
  })

  it('排序:按 quantity 降序', () => {
    const data = generateStocks(200, 5)
    const sorted = [...data].sort((a, b) => b.quantity - a.quantity)
    for (let i = 1; i < sorted.length; i++) {
      expect(sorted[i].quantity).toBeLessThanOrEqual(sorted[i - 1].quantity)
    }
  })

  it('排序:按字符串字段(product_name) 升序', () => {
    const data = generateStocks(200, 5)
    const sorted = [...data].sort((a, b) => a.product_name.localeCompare(b.product_name))
    for (let i = 1; i < sorted.length; i++) {
      expect(sorted[i].product_name >= sorted[i - 1].product_name).toBe(true)
    }
  })
})

describe('POC - 风险与覆盖检查', () => {
  it('quantity 数值范围 [0, 499]', () => {
    const data = generateStocks(1000, 13)
    for (const r of data) {
      expect(r.quantity).toBeGreaterThanOrEqual(0)
      expect(r.quantity).toBeLessThan(500)
    }
  })

  it('min_quantity 数值范围 [10, 109]', () => {
    const data = generateStocks(1000, 13)
    for (const r of data) {
      expect(r.min_quantity).toBeGreaterThanOrEqual(10)
      expect(r.min_quantity).toBeLessThan(110)
    }
  })

  it('低库存条目与预警状态一致', () => {
    const data = generateStocks(1000, 13)
    for (const r of data) {
      if (r.quantity < r.min_quantity) {
        expect(r.status).toBe('warning')
      }
    }
  })
})
