// TwoFactorSetup 主业务 composable
// 拆分自 security/TwoFactorSetup.vue（P14 批 2 I-3 第 6 批）
// 业务领域：双因素认证（currentStep + qrCodeDataUrl + secretText + recoveryCodes + isEnabled/username）
// 注：Step 3 的 verifyForm/verifyFormRef/verifyError 已下沉到 TfaStep3 子组件，
//     父组件通过 ref 调用其 validate() 方法
// 行为完全保持一致（仅结构重构）
import { computed, reactive, ref } from 'vue'
import { useUserStore } from '@/store/user'

/** TwoFactorSetup 主业务 composable（返回 reactive 包装的字段，父组件可直接 .字段 解包） */
export const useTfa = () => {
  const userStore = useUserStore()

  // 当前步骤：0 启动 / 1 扫码 / 2 验证 / 3 完成
  const currentStep = ref<number>(0)

  // Step 1 加载状态
  const setupLoading = ref(false)

  // Step 2 数据
  const qrCodeDataUrl = ref<string>('')
  const secretText = ref<string>('')

  // Step 3 加载状态（enableLoading 仍由父组件管，因为点击验证按钮的 loading 状态在父组件 UI）
  const enableLoading = ref(false)

  // Step 4 数据：恢复码（v11 批次 141 已接入后端 POST /auth/totp/recovery-codes）
  const recoveryCodes = ref<string[]>([])

  // 当前用户是否已启用 2FA
  const isEnabled = computed<boolean>(() => {
    return !!userStore.userInfo?.is_totp_enabled
  })

  // 当前用户名（用于显示在 2FA 账户名处）
  const username = computed<string>(() => {
    return userStore.userInfo?.username || ''
  })

  return reactive({
    currentStep,
    setupLoading,
    qrCodeDataUrl,
    secretText,
    enableLoading,
    recoveryCodes,
    isEnabled,
    username,
  })
}
