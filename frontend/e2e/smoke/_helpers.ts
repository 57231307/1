// P2-3 V2Table 冒烟测试公共工具
// 批次 190 规则 6 修复：mock 数据已抽取到 e2e/fixtures/auth.ts，本文件仅做再导出
// 保持 smoke spec 的 import 路径不变（向后兼容）

export {
  generateFakeJwt,
  injectAuthToken,
  mockAuthMe,
  mockInitStatus,
  mockBusinessApi,
  applyAuthMocks,
  waitForPageReady,
} from '../fixtures/auth'
