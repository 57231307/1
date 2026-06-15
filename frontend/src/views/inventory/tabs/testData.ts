/**
 * 库存台账测试数据生成(与 scripts/gen-test-data.ts 同步)
 *
 * 该模块与 scripts/gen-test-data.ts 保持完全一致,放在 src 目录下
 * 以便 vue 文件直接 import(避免 scripts 目录未纳入 tsconfig include 范围)。
 *
 * 生成 1 万行与原表格列结构一致的库存台账测试数据。
 */

export interface StockRow {
  id: number
  product_code: string
  product_name: string
  warehouse_name: string
  batch_no: string
  color_code: string
  location: string
  quantity: number
  unit: string
  gram_weight: number
  width: number
  status: 'normal' | 'warning' | 'frozen'
  min_quantity: number
  alert_level?: 'danger' | 'warning' | null
  created_at: string
}

/** 仓库候选列表(与原表格保持一致) */
const WAREHOUSES = [
  '主仓',
  '次仓',
  '坯布一库',
  '坯布二库',
  '成品一库',
  '成品二库',
  '染色仓',
  '退货仓',
  '电商仓',
  '中转仓',
]

/** 颜色候选(染色业务使用) */
const COLORS = ['BLK', 'WHT', 'RED', 'BLU', 'GRN', 'YLW', 'GRY', 'PNK', 'PUR', 'ORG']

/** 状态候选 */
const STATUSES: StockRow['status'][] = ['normal', 'warning', 'frozen']

/** 单位候选 */
const UNITS = ['米', '码', '公斤', '匹', '卷']

/** 库位候选 */
const LOCATIONS = [
  'A-01-01',
  'A-01-02',
  'A-02-01',
  'A-02-02',
  'B-01-01',
  'B-01-02',
  'B-02-01',
  'B-02-02',
  'C-01-01',
  'C-01-02',
  'C-02-01',
  'C-02-02',
  'D-01-01',
  'D-01-02',
  'D-02-01',
  'D-02-02',
]

/**
 * 简单确定性伪随机数生成器(种子化,保证多次运行结果一致,便于对比性能数据)
 * Mulberry32 算法
 */
function createRng(seed: number): () => number {
  let s = seed >>> 0
  return function rng() {
    s = (s + 0x6d2b79f5) >>> 0
    let t = s
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

function pick<T>(arr: T[], rng: () => number): T {
  return arr[Math.floor(rng() * arr.length)]
}

function pad(n: number, len: number): string {
  return n.toString().padStart(len, '0')
}

/**
 * 生成 N 行库存台账测试数据
 * @param count 数据量,默认 10000
 * @param seed 随机种子,默认 42
 */
export function generateStocks(count = 10000, seed = 42): StockRow[] {
  const rng = createRng(seed)
  const result: StockRow[] = []
  const startTime = performance.now()

  // 静默使用 STATUSES(避免 linter 报未使用)
  void STATUSES

  for (let i = 0; i < count; i++) {
    const minQuantity = Math.floor(rng() * 100) + 10
    const quantity = Math.floor(rng() * 500)
    const status: StockRow['status'] =
      quantity < minQuantity
        ? 'warning'
        : rng() < 0.1
          ? 'frozen'
          : 'normal'

    const productName = `测试面料${pad(i + 1, 5)}`

    result.push({
      id: i + 1,
      product_code: `P${pad(i + 1, 6)}`,
      product_name: productName,
      warehouse_name: pick(WAREHOUSES, rng),
      batch_no: `B${pad(Math.floor(rng() * 99999), 5)}`,
      color_code: pick(COLORS, rng),
      location: pick(LOCATIONS, rng),
      quantity,
      unit: pick(UNITS, rng),
      gram_weight: Math.floor(rng() * 500) + 80,
      width: Math.floor(rng() * 280) + 100,
      status,
      min_quantity: minQuantity,
      alert_level: status === 'warning' ? (rng() < 0.3 ? 'danger' : 'warning') : null,
      created_at: new Date(2026, 0, 1 + (i % 365)).toISOString(),
    })
  }

  const cost = performance.now() - startTime
  // eslint-disable-next-line no-console
  console.info(`[gen-test-data] 已生成 ${count} 行测试数据,耗时 ${cost.toFixed(1)}ms`)
  return result
}

/**
 * 将生成的数据写入 localStorage,供 POC 页面读取
 */
export function persistStocks(count = 10000, seed = 42): StockRow[] {
  const data = generateStocks(count, seed)
  try {
    localStorage.setItem('__poc_stock_test_data', JSON.stringify(data))
    // eslint-disable-next-line no-console
    console.info(`[gen-test-data] 已写入 localStorage.__poc_stock_test_data, ${data.length} 行`)
  } catch (e) {
    // eslint-disable-next-line no-console
    console.warn('[gen-test-data] localStorage 写入失败,改用 sessionStorage', e)
    sessionStorage.setItem('__poc_stock_test_data', JSON.stringify(data))
  }
  return data
}

/**
 * 从 localStorage 读取测试数据,失败则自动生成
 */
export function loadOrGenerateStocks(count = 10000): StockRow[] {
  try {
    const raw = localStorage.getItem('__poc_stock_test_data')
    if (raw) {
      const arr = JSON.parse(raw) as StockRow[]
      if (Array.isArray(arr) && arr.length >= count) {
        return arr
      }
    }
  } catch {
    // 忽略解析错误,降级到生成
  }
  return persistStocks(count)
}
