export * from './auth'
export * from './dashboard'
export * from './fabric'
export * from './inventory'
export * from './customer'
export * from './sales'
export * from './supplier'
export * from './purchase'
export * from './product'
export * from './warehouse'
export * from './notification'
export * from './bpm'
export * from './asset'
export * from './ap'
export * from './ar'
export * from './quality'
export * from './production'
export * from './customer-credit'
export * from './financial-analysis'
export * from './five-dimension'
export * from './assist-accounting'
export * from './business-trace'
export * from './sales-return'
export * from './purchase-return'
export * from './sales-price'
export * from './purchase-price'
export * from './finance'
export * from './user'
export * from './role'
export * from './department'
export * from './crm'
// D14 Batch 5b：crm-enhanced 与 customer 存在同名导出（createCustomer/deleteCustomer/getCustomerList/updateCustomer），
// 且无 caller 从 '@/api' 聚合入口导入（均直接从 '@/api/crm-enhanced' 导入），故不在此 re-export
// export * from './crm-enhanced'
export * from './data-permission'
export * from './fund'
export * from './currency'
export * from './cost'
export * from './bom'
export * from './scheduling'
// D14 Batch 5b：purchaseReceipt 与 purchase 存在同名导出（createPurchaseReceipt/getPurchaseReceiptList），
// 且无 caller 从 '@/api' 聚合入口导入，故不在此 re-export
// export * from './purchaseReceipt'
export * from './inventoryCount'
export * from './inventoryAdjustment'
// D14 Batch 5b：inventoryTransfer 与 inventory 存在同名导出（approveInventoryTransfer/createInventoryTransfer/getInventoryTransferList），
// 且无 caller 从 '@/api' 聚合入口导入，故不在此 re-export
// export * from './inventoryTransfer'
export * from './inventoryBatch'
export * from './accounting-period'
export * from './dye-recipe'
export * from './dye-batch'
export * from './greige-fabric'
export * from './mrp'
// 修复：financeReport 中的 ReportData 与 financial-analysis 中的 ReportData 同名冲突
// 业务代码均通过 '@/api/financeReport' 直接导入，不依赖此处的重新导出
export type {
  BalanceSheetItem,
  ProfitStatementItem,
  CashFlowItem,
  TrialBalanceItem,
  GeneralLedgerItem,
  SubsidiaryLedgerItem,
  ReportItem,
  FinanceReportQueryParams,
  GeneralLedgerQueryParams,
  SubsidiaryLedgerQueryParams,
} from './financeReport'
export {
  getBalanceSheet,
  getProfitStatement,
  getCashFlowStatement,
  getTrialBalance,
  getGeneralLedger,
  getSubsidiaryLedger,
} from './financeReport'
export * from './sales-analysis'
export * from './supplier-evaluation'
export * from './security'
export * from './capacity'
export * from './barcode-scanner'
export * from './ar-reconciliation-enhanced'
export * from './report-enhanced'
export * from './material-shortage'
// D14 Batch 5b：bpm-enhanced 与 bpm 存在同名导出（getBpmPendingTaskList），
// 且无 caller 从 '@/api' 聚合入口导入，故不在此 re-export
// export * from './bpm-enhanced'
export * from './omniAudit'
export {
  forecastSales,
  optimizeInventory,
  detectAnomalies,
  getRecommendations,
  executeReport,
} from './advanced'
export * from './quotation'
export * from './custom-order'
export * from './trading'
// P2-4 AI 分析深化（工艺优化 + 质量预测）16 端点客户端
export * from './ai-extend'
