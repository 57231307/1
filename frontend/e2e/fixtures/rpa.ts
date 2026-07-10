/**
 * RPA / 表单自动化 / 数据提取工具集
 *
 * 批次 262：针对项目 E2E 测试增强，提供以下能力：
 * - 表单自动填充：按 label/placeholder 定位字段并填充
 * - 按钮自动点击：按文本定位按钮并点击
 * - 表格数据提取：爬虫类批量收集表格数据供断言
 * - 下拉选择自动化：el-select 组件自动选择
 * - 等待元素稳定：防抖/加载完成后操作
 *
 * 设计原则：
 * - 所有定位器优先使用 Element Plus 组件结构（el-form/el-select/el-table-v2）
 * - 容错处理：元素不存在时返回 null 而非抛出（便于断言判断）
 * - 与项目现有 smoke 测试的组件选择器保持一致（.el-table-v2 / .el-select 等）
 */
import type { Locator, Page } from '@playwright/test'

/**
 * 表单字段定义
 */
export interface FormField {
  /** 字段类型 */
  type: 'text' | 'select' | 'textarea' | 'number' | 'date'
  /** 定位器（CSS 选择器或 placeholder 文本） */
  selector?: string
  /** placeholder 文本（用于 input[placeholder*="xxx"] 定位） */
  placeholder?: string
  /** label 文本（用于 label:has-text("xxx") 定位） */
  label?: string
  /** 要填入的值 */
  value: string
}

/**
 * 按 label 文本定位字段输入框
 */
function locateByLabel(page: Page, labelText: string): Locator {
  // Element Plus 表单：.el-form-item__label + .el-form-item__content > input
  return page
    .locator('.el-form-item')
    .filter({ has: page.locator('.el-form-item__label', { hasText: labelText }) })
    .locator('input, textarea')
    .first()
}

/**
 * 按 placeholder 定位字段输入框
 */
function locateByPlaceholder(page: Page, placeholder: string): Locator {
  return page.locator(`input[placeholder*="${placeholder}"]`).first()
}

/**
 * 自动填充单个表单字段
 *
 * 使用场景：
 * - RPA 类批量表单填充
 * - 减少测试中重复的 fill/click 代码
 */
export async function autoFillField(page: Page, field: FormField): Promise<void> {
  let locator: Locator

  if (field.selector) {
    locator = page.locator(field.selector).first()
  } else if (field.placeholder) {
    locator = locateByPlaceholder(page, field.placeholder)
  } else if (field.label) {
    locator = locateByLabel(page, field.label)
  } else {
    throw new Error('FormField 必须提供 selector / placeholder / label 之一')
  }

  await locator.waitFor({ state: 'visible', timeout: 5_000 })

  switch (field.type) {
    case 'select':
      // Element Plus el-select：点击触发下拉 → 选择匹配项
      await locator.click()
      await page.waitForSelector('.el-select-dropdown__item', { state: 'visible', timeout: 3_000 })
      // 选择包含目标文本的选项
      await page
        .locator('.el-select-dropdown__item')
        .filter({ hasText: field.value })
        .first()
        .click()
      break
    case 'textarea':
      await locator.fill(field.value)
      break
    case 'number':
      await locator.fill(field.value)
      break
    case 'date':
      // Element Plus el-date-editor：点击触发面板 → 输入日期
      await locator.click()
      await page.waitForSelector('.el-date-picker', { state: 'visible', timeout: 3_000 })
      await locator.fill(field.value)
      await page.keyboard.press('Enter')
      break
    case 'text':
    default:
      await locator.fill(field.value)
      break
  }
}

/**
 * 批量自动填充表单
 *
 * 使用场景：
 * - RPA 类批量表单填充
 * - 减少测试中重复的 fill/click 代码
 *
 * @example
 * await autoFillForm(page, [
 *   { type: 'text', placeholder: 'PANTONE', value: 'E2E-001' },
 *   { type: 'select', label: '色卡类型', value: '标准色卡' },
 * ])
 */
export async function autoFillForm(page: Page, fields: FormField[]): Promise<void> {
  for (const field of fields) {
    await autoFillField(page, field)
  }
}

/**
 * 按文本自动点击按钮
 *
 * 使用场景：
 * - RPA 类按钮点击（保存/提交/取消/导出等）
 * - 减少测试中重复的 button:has-text 代码
 *
 * @example
 * await autoClickButton(page, '保存')
 */
export async function autoClickButton(
  page: Page,
  text: string,
  options: { timeout?: number } = {}
): Promise<void> {
  const btn = page.locator(`button:has-text("${text}")`).first()
  await btn.waitFor({ state: 'visible', timeout: options.timeout ?? 5_000 })
  await btn.click()
}

/**
 * 提取表格数据（爬虫类：批量收集表格行数据）
 *
 * 使用场景：
 * - 验证表格渲染的数据是否正确
 * - 爬虫类批量收集列表数据供后续断言
 * - 多角色协作中验证数据一致性
 *
 * 注意：项目使用 Element Plus el-table-v2（虚拟滚动），
 * 此函数仅提取当前可视区的行数据。
 *
 * @example
 * const rows = await extractTableData(page, '.el-table-v2')
 * expect(rows.length).toBeGreaterThan(0)
 * expect(rows[0]).toContain('E2E-001')
 */
export async function extractTableData(
  page: Page,
  tableSelector = '.el-table-v2'
): Promise<string[][]> {
  const table = page.locator(tableSelector).first()
  await table.waitFor({ state: 'attached', timeout: 10_000 })

  // 提取所有行的单元格文本
  const rows = await table.locator('.el-table-v2__row').all()
  const result: string[][] = []

  for (const row of rows) {
    const cells = await row.locator('.el-table-v2__cell, td').all()
    const rowData: string[] = []
    for (const cell of cells) {
      const text = await cell.textContent()
      rowData.push((text ?? '').trim())
    }
    if (rowData.length > 0) {
      result.push(rowData)
    }
  }

  return result
}

/**
 * 提取表格列数据（爬虫类：批量收集某一列的所有值）
 *
 * 使用场景：
 * - 验证某一列的数据是否正确（如订单号列、状态列）
 * - 爬虫类收集特定字段的所有值
 *
 * @example
 * // 提取第一列（通常是编号列）的所有值
 * const orderIds = await extractColumnData(page, '.el-table-v2', 0)
 * expect(orderIds.length).toBeGreaterThan(0)
 */
export async function extractColumnData(
  page: Page,
  tableSelector: string,
  columnIndex: number
): Promise<string[]> {
  const rows = await extractTableData(page, tableSelector)
  return rows.map((row) => row[columnIndex] ?? '').filter((v) => v.length > 0)
}

/**
 * 等待表格数据加载完成（至少有一行数据或空状态提示）
 *
 * 使用场景：
 * - 翻页/搜索后等待表格刷新
 * - 避免在表格加载中途断言
 *
 * @example
 * await page.click('.el-pagination .btn-next')
 * await waitForTableLoaded(page)
 */
export async function waitForTableLoaded(
  page: Page,
  tableSelector = '.el-table-v2',
  timeout = 10_000
): Promise<void> {
  const table = page.locator(tableSelector).first()
  await table.waitFor({ state: 'attached', timeout })
  // 等待至少一行数据或空状态提示出现
  await page
    .locator(`${tableSelector} .el-table-v2__row, .el-empty, .el-table__empty-text`)
    .first()
    .waitFor({ state: 'attached', timeout: 5_000 })
    .catch(() => {
      // 超时不阻塞（可能是虚拟滚动未渲染）
    })
}

/**
 * 等待 Element Plus 消息提示出现（成功/错误/警告）
 *
 * 使用场景：
 * - 表单提交后等待成功/错误提示
 * - 验证业务操作的反馈消息
 *
 * @example
 * await autoClickButton(page, '保存')
 * await waitForElMessage(page, 'success')
 */
export async function waitForElMessage(
  page: Page,
  type: 'success' | 'warning' | 'info' | 'error' = 'success',
  timeout = 5_000
): Promise<Locator> {
  const message = page.locator(`.el-message--${type}`).first()
  await message.waitFor({ state: 'visible', timeout })
  return message
}

/**
 * RPA 流程录制：记录一系列操作的执行时间戳（用于性能分析）
 *
 * 使用场景：
 * - RPA 类流程的性能基准测试
 * - 验证业务流程在时间约束内完成
 *
 * @example
 * const recorder = createRpaRecorder()
 * await page.goto('/sales')
 * recorder.mark('页面加载完成')
 * await autoClickButton(page, '新建')
 * recorder.mark('点击新建按钮')
 * const metrics = recorder.report()
 */
export function createRpaRecorder() {
  const marks: Array<{ label: string; timestamp: number }> = []
  const startTime = Date.now()

  return {
    mark(label: string): void {
      marks.push({ label, timestamp: Date.now() - startTime })
    },
    report(): Array<{ label: string; elapsed: number }> {
      return marks.map((m, i) => ({
        label: m.label,
        elapsed: i === 0 ? m.timestamp : m.timestamp - marks[i - 1].timestamp,
      }))
    },
    total(): number {
      return Date.now() - startTime
    },
  }
}
