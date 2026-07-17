import { ElMessage } from 'element-plus'
// V15 P0-S12 修复（Batch 474）：从同级 ./request 导入（与 src/api/* 一致）
import { request } from './request'

/** 导出列定义，使用泛型支持类型安全的字段访问 */
export interface ExportColumn<T extends Record<string, unknown> = Record<string, unknown>> {
  key: keyof T & string
  title: string
  formatter?: (value: unknown, row: T) => string
}

/** 导出选项，使用泛型约束数据类型 */
export interface ExportOptions<T extends Record<string, unknown> = Record<string, unknown>> {
  filename: string
  columns: ExportColumn<T>[]
  data: T[]
  /** 导出格式，默认 excel（规则 3：禁止 CSV 作为最终交付） */
  format?: 'excel'
}

function generateExcelHTML<T extends Record<string, unknown>>(
  columns: ExportColumn<T>[],
  data: T[]
): string {
  const headers = columns.map(col => `<th>${col.title}</th>`).join('')
  const rows = data
    .map(row => {
      const cells = columns
        .map(col => {
          const value = row[col.key]
          const formatted = col.formatter ? col.formatter(value, row) : String(value ?? '')
          return `<td>${formatted}</td>`
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

/**
 * V15 P0-S12 修复（Batch 474）：本地 HTML 导出保留为兼容方案（资源尚未接入后端 export 时降级使用）
 *
 * 历史背景：原 exportToExcel 生成 .xls HTML 格式，无水印、无审计、无合规保障，
 * 已被 P0-S12 列为 P0 阻塞级问题。新接入后端的资源应改用 exportFromBackend。
 */
export function exportToExcel<T extends Record<string, unknown>>(options: ExportOptions<T>) {
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

export function exportData<T extends Record<string, unknown>>(options: ExportOptions<T>) {
  exportToExcel(options)
}

/**
 * V15 P0-S12 + P0-S15 修复（Batch 474）：从后端下载带水印的 xlsx 文件
 *
 * 设计要点：
 * - 调用后端 GET API（如 `/customers/export`），返回 Blob 流（application/vnd.openxmlformats-officedocument.spreadsheetml.sheet）
 * - 后端已注入水印（操作员/IP/时间戳），前端无需重复添加
 * - 自动从 Content-Disposition 提取文件名；失败时回退到传入的 filename + 时间戳
 * - 保留本地 exportToExcel 作为兼容方案（资源尚未接入后端 export 时降级使用）
 *
 * @param apiPath 后端导出 API 路径（如 `/customers/export`）
 * @param params 查询参数（与 list 接口共用）
 * @param filename 下载文件名前缀（不含扩展名，后端会附加 .xlsx）
 */
export async function exportFromBackend<TParams extends Record<string, unknown>>(
  apiPath: string,
  params: TParams,
  filename: string
): Promise<void> {
  try {
    const response = await request.get<Blob>(apiPath, {
      params,
      responseType: 'blob',
    })
    // V15 P0-S12：从 Content-Disposition 提取文件名（后端返回 filename="customers_export_xxx.xlsx"）
    const disposition = response.headers?.['content-disposition'] || ''
    const matched = /filename="?([^";]+)"?/.exec(disposition)
    const downloadName = matched?.[1] || `${filename}_${new Date().toISOString().replace(/[:.]/g, '')}.xlsx`

    const blob = response.data as unknown as Blob
    const link = document.createElement('a')
    link.href = URL.createObjectURL(blob)
    link.download = downloadName
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    URL.revokeObjectURL(link.href)
    ElMessage.success('导出成功')
  } catch (err) {
    // V15 P0-S12：错误用 ElMessage 表达（与 exportToExcel 行为一致）
    const msg = err instanceof Error ? err.message : '导出失败'
    ElMessage.error(`导出失败：${msg}`)
    throw err
  }
}
