/**
 * print-js 类型声明扩展
 * v11 批次 181 P2-1 修复：print-js 1.6.0 自带类型定义的 PrintTypes 未包含 'table'，
 * 但库实际支持表格打印。此声明扩展 Configuration.type 支持 'table'。
 * 规则 0：对所有遇到的错误均进行统一修复，不使用 as 绕过类型检查。
 */
declare module 'print-js' {
  /// 扩展 print-js 的打印类型，包含 'table'（库实际支持但类型定义缺失）
  export interface Configuration {
    type?: 'pdf' | 'html' | 'image' | 'json' | 'raw-html' | 'table'
  }
}
