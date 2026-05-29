import { ElMessage } from 'element-plus'

export interface ExportColumn {
  key: string
  title: string
  formatter?: (value: any, row: any) => string
}

export interface ExportOptions {
  filename: string
  columns: ExportColumn[]
  data: any[]
  format?: 'csv' | 'excel'
}

function escapeCSV(value: string): string {
  if (value === null || value === undefined) return ''
  const str = String(value)
  if (str.includes(',') || str.includes('"') || str.includes('\n')) {
    return `"${str.replace(/"/g, '""')}"`
  }
  return str
}

function generateCSV(columns: ExportColumn[], data: any[]): string {
  const headers = columns.map((col) => escapeCSV(col.title)).join(',')
  const rows = data.map((row) =>
    columns
      .map((col) => {
        const value = row[col.key]
        const formatted = col.formatter ? col.formatter(value, row) : value
        return escapeCSV(formatted ?? '')
      })
      .join(',')
  )
  return [headers, ...rows].join('\n')
}

function generateExcelHTML(columns: ExportColumn[], data: any[]): string {
  const headers = columns.map((col) => `<th>${col.title}</th>`).join('')
  const rows = data
    .map((row) => {
      const cells = columns
        .map((col) => {
          const value = row[col.key]
          const formatted = col.formatter ? col.formatter(value, row) : value
          return `<td>${formatted ?? ''}</td>`
        })
        .join('')
      return `<tr>${cells}</tr>`
    })
    .join('')
  return `
    <html xmlns:o="urn:schemas-microsoft-com:office:office"
          xmlns:x="urn:schemas-microsoft-com:office:excel"
          xmlns="http://www.w3.org/TR/REC-html40">
    <head>
      <meta charset="utf-8">
      <!--[if gte mso 9]>
      <xml>
        <x:ExcelWorkbook>
          <x:ExcelWorksheets>
            <x:ExcelWorksheet>
              <x:Name>Sheet1</x:Name>
              <x:WorksheetOptions>
                <x:DisplayGridlines/>
              </x:WorksheetOptions>
            </x:ExcelWorksheet>
          </x:ExcelWorksheets>
        </x:ExcelWorkbook>
      </xml>
      <![endif]-->
    </head>
    <body>
      <table>
        <thead><tr>${headers}</tr></thead>
        <tbody>${rows}</tbody>
      </table>
    </body>
    </html>
  `
}

function downloadFile(content: string, filename: string, mimeType: string) {
  const BOM = '\uFEFF'
  const blob = new Blob([BOM + content], { type: mimeType })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = filename
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  URL.revokeObjectURL(link.href)
}

export function exportToCSV(options: ExportOptions) {
  const { filename, columns, data } = options
  if (!data || data.length === 0) {
    ElMessage.warning('没有可导出的数据')
    return
  }
  const csvContent = generateCSV(columns, data)
  const date = new Date().toISOString().split('T')[0]
  downloadFile(csvContent, `${filename}_${date}.csv`, 'text/csv;charset=utf-8;')
  ElMessage.success('导出成功')
}

export function exportToExcel(options: ExportOptions) {
  const { filename, columns, data } = options
  if (!data || data.length === 0) {
    ElMessage.warning('没有可导出的数据')
    return
  }
  const htmlContent = generateExcelHTML(columns, data)
  const date = new Date().toISOString().split('T')[0]
  downloadFile(htmlContent, `${filename}_${date}.xls`, 'application/vnd.ms-excel;charset=utf-8;')
  ElMessage.success('导出成功')
}

export function exportData(options: ExportOptions) {
  const format = options.format || 'csv'
  if (format === 'excel') {
    exportToExcel(options)
  } else {
    exportToCSV(options)
  }
}
