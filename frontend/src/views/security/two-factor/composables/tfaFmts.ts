// TwoFactorSetup 验证规则
// 拆分自 security/TwoFactorSetup.vue（P14 批 2 I-3 第 6 批）
// 行为完全保持一致（仅结构重构）
import type { FormRules } from 'element-plus'

/** 6 位令牌格式校验 */
export const validateToken = (_rule: unknown, value: string, callback: (err?: Error) => void) => {
  if (!value) {
    callback(new Error('请输入 6 位动态口令'))
    return
  }
  if (!/^\d{6}$/.test(value)) {
    callback(new Error('令牌格式不正确（需 6 位数字）'))
    return
  }
  callback()
}

/** 表单验证规则 */
export const verifyRules: FormRules = {
  token: [{ validator: validateToken, trigger: 'blur' }],
}
