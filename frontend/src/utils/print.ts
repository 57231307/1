import { ElMessage } from 'element-plus'

/**
 * 打印列定义接口
 * @template T - 数据行类型
 */
export interface PrintColumn<T extends Record<string, unknown> = Record<string, unknown>> {
  key: keyof T & string
  title: string
  width?: string
  align?: 'left' | 'center' | 'right'
  formatter?: (value: unknown, row: T) => string
}

/**
 * 打印选项接口
 * @template T - 数据行类型
 */
export interface PrintOptions<T extends Record<string, unknown> = Record<string, unknown>> {
  title: string
  columns: PrintColumn<T>[]
  data: T[]
  extraInfo?: { label: string; value: string }[]
  orientation?: 'portrait' | 'landscape'
}

/**
 * 生成打印 HTML 内容
 */
function generatePrintHTML<T extends Record<string, unknown>>(options: PrintOptions<T>): string {
  const { title, columns, data, extraInfo, orientation = 'portrait' } = options

  const headerCells = columns
    .map(
      (col) =>
        `<th style="padding: 8px 12px; border: 1px solid #333; background: #f5f5f5; text-align: ${col.align || 'left'}; ${col.width ? `width: ${col.width}` : ''}">${col.title}</th>`
    )
    .join('')

  const bodyRows = data
    .map((row) => {
      const cells = columns
        .map((col) => {
          const value = row[col.key]
          const formatted = col.formatter ? col.formatter(value, row) : (value ?? '')
          return `<td style="padding: 6px 12px; border: 1px solid #333; text-align: ${col.align || 'left'}">${formatted}</td>`
        })
        .join('')
      return `<tr>${cells}</tr>`
    })
    .join('')

  const infoSection = extraInfo
    ? `<div style="margin: 16px 0; display: flex; gap: 32px;">
        ${extraInfo.map((info) => `<span><strong>${info.label}:</strong> ${info.value}</span>`).join('')}
       </div>`
    : ''

  const now = new Date()
  const printDate = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')} ${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`

  return `
    <!DOCTYPE html>
    <html>
    <head>
      <meta charset="utf-8">
      <title>${title}</title>
      <style>
        @media print {
          @page { size: ${orientation}; margin: 15mm; }
          body { -webkit-print-color-adjust: exact; print-color-adjust: exact; }
        }
        body { font-family: "Microsoft YaHei", "SimSun", sans-serif; font-size: 12px; color: #333; }
        h1 { font-size: 20px; text-align: center; margin-bottom: 8px; }
        .print-meta { text-align: center; color: #666; font-size: 11px; margin-bottom: 16px; }
        table { width: 100%; border-collapse: collapse; margin-top: 8px; }
        th, td { border: 1px solid #333; }
        .footer { margin-top: 20px; display: flex; justify-content: space-between; font-size: 11px; color: #666; }
      </style>
    </head>
    <body>
      <h1>${title}</h1>
      <div class="print-meta">打印时间: ${printDate} | 共 ${data.length} 条记录</div>
      ${infoSection}
      <table>
        <thead><tr>${headerCells}</tr></thead>
        <tbody>${bodyRows}</tbody>
      </table>
      <div class="footer">
        <span>打印人: ___________</span>
        <span>审核人: ___________</span>
        <span>日期: ___________</span>
      </div>
    </body>
    </html>
  `
}

/**
 * 打印数据表格
 */
export function printData<T extends Record<string, unknown>>(options: PrintOptions<T>) {
  if (!options.data || options.data.length === 0) {
    ElMessage.warning('没有可打印的数据')
    return
  }

  const html = generatePrintHTML(options)
  const printWindow = window.open('', '_blank')
  if (!printWindow) {
    ElMessage.error('无法打开打印窗口，请检查浏览器弹窗设置')
    return
  }

  printWindow.document.write(html)
  printWindow.document.close()
  printWindow.onload = () => {
    printWindow.print()
  }
  ElMessage.success('打印窗口已打开')
}

/**
 * 打印单个单据（含表头信息、明细行、页脚）
 */
export function printSingleDocument<T extends Record<string, unknown>>(options: {
  title: string
  info: Record<string, string>
  items: T[]
  itemColumns: PrintColumn<T>[]
  footer?: Record<string, string>
}) {
  const { title, info, items, itemColumns, footer } = options

  const infoHTML = Object.entries(info)
    .map(
      ([key, value]) => `<span style="margin-right: 32px;"><strong>${key}:</strong> ${value}</span>`
    )
    .join('')

  const headerCells = itemColumns
    .map(
      (col) =>
        `<th style="padding: 8px 12px; border: 1px solid #333; background: #f5f5f5; text-align: ${col.align || 'left'}">${col.title}</th>`
    )
    .join('')

  const bodyRows = items
    .map((row) => {
      const cells = itemColumns
        .map((col) => {
          const value = row[col.key]
          const formatted = col.formatter ? col.formatter(value, row) : (value ?? '')
          return `<td style="padding: 6px 12px; border: 1px solid #333; text-align: ${col.align || 'left'}">${formatted}</td>`
        })
        .join('')
      return `<tr>${cells}</tr>`
    })
    .join('')

  const footerHTML = footer
    ? `<div style="margin-top: 20px; display: flex; justify-content: space-between;">
        ${Object.entries(footer)
          .map(([key, value]) => `<span><strong>${key}:</strong> ${value}</span>`)
          .join('')}
       </div>`
    : ''

  const now = new Date()
  const printDate = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`

  const html = `
    <!DOCTYPE html>
    <html>
    <head>
      <meta charset="utf-8">
      <title>${title}</title>
      <style>
        @media print {
          @page { size: A4; margin: 15mm; }
        }
        body { font-family: "Microsoft YaHei", "SimSun", sans-serif; font-size: 12px; color: #333; padding: 20px; }
        h1 { font-size: 20px; text-align: center; margin-bottom: 16px; }
        .info-section { margin: 16px 0; line-height: 1.8; }
        table { width: 100%; border-collapse: collapse; margin-top: 12px; }
        .sign-area { margin-top: 40px; display: flex; justify-content: space-between; }
        .sign-item { text-align: center; }
        .sign-line { width: 150px; border-bottom: 1px solid #333; margin-top: 40px; }
      </style>
    </head>
    <body>
      <h1>${title}</h1>
      <div class="info-section">${infoHTML}</div>
      <table>
        <thead><tr>${headerCells}</tr></thead>
        <tbody>${bodyRows}</tbody>
      </table>
      ${footerHTML}
      <div class="sign-area">
        <div class="sign-item"><div class="sign-line"></div>制单人</div>
        <div class="sign-item"><div class="sign-line"></div>审核人</div>
        <div class="sign-item"><div class="sign-line"></div>收货人</div>
      </div>
      <div style="text-align: right; margin-top: 16px; font-size: 11px; color: #999;">打印日期: ${printDate}</div>
    </body>
    </html>
  `

  const printWindow = window.open('', '_blank')
  if (!printWindow) {
    ElMessage.error('无法打开打印窗口')
    return
  }
  printWindow.document.write(html)
  printWindow.document.close()
  printWindow.onload = () => {
    printWindow.print()
  }
}
