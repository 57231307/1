// TwoFactorSetup 业务流程 composable
// 拆分自 security/TwoFactorSetup.vue（P14 批 2 I-3 第 6 批）
// 业务领域：双因素认证（启动设置 / 上下步 / 复制密钥 / 验证并启用 / 生成恢复码 / 复制恢复码 / 完成）
// 注：Step 3 验证流程中通过 TwoFactorAuthStep3 的 ref 调其 validate() 获取 token
// 行为完全保持一致（仅结构重构）
import { ElMessage } from 'element-plus'
import { useRouter } from 'vue-router'
import { setupTotp, enableTotp, generateRecoveryCodes } from '@/api/auth'
import { useUserStore } from '@/store/user'
import { logger } from '@/utils/logger'

/** TwoFactorAuthStep3 子组件暴露的接口（defineExpose 提供的 validate/setError 方法） */
export interface TwoFactorAuthStep3Instance {
  validate: () => Promise<{ valid: boolean; token: string }>
  setError: (msg: string) => void
  clearError: () => void
}

/** v11 批次 172 P2-1 修复：定义 TFA 主状态接口，替代 tfa: any */
export interface TfaState {
  currentStep: number
  setupLoading: boolean
  qrCodeDataUrl: string
  secretText: string
  enableLoading: boolean
  recoveryCodes: string[]
  isEnabled: boolean
  username: string
}

/** TwoFactorSetup 业务流程 composable */
export const useTfaProc = () => {
  const router = useRouter()
  const userStore = useUserStore()

  // 启动设置：调 GET /auth/totp/setup
  const handleStartSetup = async (tfa: TfaState) => {
    tfa.setupLoading = true
    try {
      const res = await setupTotp()
      if (res.data) {
        tfa.qrCodeDataUrl = res.data.qr_code
        tfa.secretText = res.data.secret
        tfa.currentStep = 1
      } else {
        ElMessage.error('获取 2FA 设置信息失败')
      }
    } catch (error) {
      logger.error('启动 2FA 设置失败:', error)
    } finally {
      tfa.setupLoading = false
    }
  }

  // 上一步
  const handlePrevStep = (tfa: TfaState) => {
    if (tfa.currentStep > 0) {
      tfa.currentStep -= 1
    }
  }

  // 下一步
  const handleNextStep = (tfa: TfaState) => {
    if (tfa.currentStep < 3) {
      tfa.currentStep += 1
    }
  }

  // 复制密钥到剪贴板
  const handleCopySecret = async (tfa: TfaState) => {
    try {
      await navigator.clipboard.writeText(tfa.secretText)
      ElMessage.success('密钥已复制到剪贴板')
    } catch {
      ElMessage.error('复制失败，请手动选中复制')
    }
  }

  // 验证并启用：调 POST /auth/totp/enable
  // 通过 tfaStep3Ref 调 TwoFactorAuthStep3 的 validate 方法获取 token
  const handleVerifyAndEnable = async (tfa: TfaState, tfaStep3Ref?: TwoFactorAuthStep3Instance | null) => {
    if (!tfaStep3Ref) {
      ElMessage.error('表单组件未就绪')
      return
    }
    const { valid, token } = await tfaStep3Ref.validate()
    if (!valid) return
    tfa.enableLoading = true
    try {
      const res = await enableTotp(token)
      if (res.code === 200 || res.code === 0) {
        ElMessage.success(res.message || '2FA 已成功启用')
        // 批次 94 P2-12 修复：原客户端生成占位恢复码（Math.random 非密码学安全且服务端无记录），
        // 改为调用服务端 API 获取恢复码（服务端使用密码学安全随机源生成并存储哈希）
        try {
          const codesRes = await generateRecoveryCodes()
          tfa.recoveryCodes = codesRes.data || []
        } catch (e) {
          logger.error('获取恢复码失败:', e)
          ElMessage.warning('2FA 已启用，但恢复码获取失败，请稍后在设置页重新生成')
          tfa.recoveryCodes = []
        }
        // 刷新用户信息，确保 is_totp_enabled 同步
        try {
          await userStore.fetchUserInfo()
        } catch (e) {
          logger.warn('刷新用户信息失败:', e)
        }
        tfa.currentStep = 3
      } else {
        // 将错误传给 TwoFactorAuthStep3（接口已扩展 setError，直接调用）
        if (tfaStep3Ref && typeof tfaStep3Ref.setError === 'function') {
          tfaStep3Ref.setError(res.message || '验证失败，请重试')
        } else {
          ElMessage.error(res.message || '验证失败，请重试')
        }
      }
    } catch (error: unknown) {
      // v11 批次 172 P2-1 修复：catch (error: any) 改为 unknown + 类型守卫
      logger.error('验证 TOTP 失败:', error)
      const errMsg = error instanceof Error ? error.message : String(error)
      if (tfaStep3Ref && typeof tfaStep3Ref.setError === 'function') {
        tfaStep3Ref.setError(errMsg || '验证失败，请检查令牌是否正确')
      } else {
        ElMessage.error(errMsg || '验证失败，请检查令牌是否正确')
      }
    } finally {
      tfa.enableLoading = false
    }
  }

  // 复制恢复码
  const handleCopyRecovery = async (tfa: TfaState) => {
    try {
      await navigator.clipboard.writeText(tfa.recoveryCodes.join('\n'))
      ElMessage.success('恢复码已复制到剪贴板')
    } catch {
      ElMessage.error('复制失败，请手动选中复制')
    }
  }

  // 完成：返回个人中心
  const handleFinish = () => {
    router.push('/system/profile')
  }

  // 页面加载时若用户已启用 2FA，停留在 Step 1 展示状态
  const initOnMount = () => {
    if (!userStore.userInfo) {
      // 触发一次获取，确保 is_totp_enabled 字段存在
      userStore.fetchUserInfo().catch(e => logger.warn('获取用户信息失败:', e))
    }
  }

  return {
    handleStartSetup,
    handlePrevStep,
    handleNextStep,
    handleCopySecret,
    handleVerifyAndEnable,
    handleCopyRecovery,
    handleFinish,
    initOnMount,
  }
}
