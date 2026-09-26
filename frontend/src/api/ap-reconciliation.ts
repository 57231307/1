// ap-reconciliation.ts - 应付对账 API 兼容层
// 实际定义在 ap.ts 中
export {
  getAPReconciliationList,
  getAPReconciliation,
  autoReconcileAllAP,
  generateAPReconciliation,
  confirmAPReconciliation,
  disputeAPReconciliation,
  type APReconciliation,
} from './ap';
