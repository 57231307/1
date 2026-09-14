/**
 * 质量标准 API（统一出口）
 *
 * 实现全部收敛至 quality.ts（本文件原先与其重复定义同一批端点，存在语义漂移风险）。
 * 此文件保留为兼容 shim：既有 import 路径不受影响。
 */
export {
  approveQualityStandard,
  archiveQualityStandard,
  createQualityStandard,
  deleteQualityStandard,
  getQualityStandard,
  getQualityStandardList,
  publishQualityStandard,
  updateQualityStandard,
  type QualityStandard,
} from './quality';
