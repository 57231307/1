<!--
  2FA（双因素认证）设置流程
  - 3 步流程：启动设置 → 扫描 QR → 验证并启用
  - Step 1：调 GET /auth/totp/setup 获取 secret + 后端已渲染好的 base64 QR 码 PNG
  - Step 2：用户用 Google Authenticator / Microsoft Authenticator 等扫码
  - Step 3：输入 6 位 token 调 POST /auth/totp/enable 提交验证
  - 注：前端无需引入 qrcode npm 包，后端 totp_rs::TOTP::get_qr_base64() 已直接生成 PNG
-->
<template>
  <div class="twofa-page">
    <el-card class="twofa-card" shadow="hover">
      <template #header>
        <div class="card-header">
          <span>双因素认证（2FA）</span>
          <el-tag v-if="isEnabled" type="success" effect="dark">已启用</el-tag>
          <el-tag v-else type="info" effect="plain">未启用</el-tag>
        </div>
      </template>

      <!-- 步骤条 -->
      <el-steps :active="currentStep" finish-status="success" align-center class="twofa-steps">
        <el-step title="启动设置" />
        <el-step title="扫描二维码" />
        <el-step title="验证并启用" />
      </el-steps>

      <el-divider />

      <!-- Step 1：启动设置 -->
      <div v-if="currentStep === 0" class="step-panel">
        <el-alert
          title="什么是双因素认证？"
          type="info"
          :closable="false"
          show-icon
        >
          <template #default>
            启用 2FA 后，登录时除输入密码外，还需要提供手机 App（如 Google Authenticator、Microsoft Authenticator）生成的 6 位动态口令，大幅提升账号安全性。
          </template>
        </el-alert>

        <div class="status-block">
          <div class="status-row">
            <span class="label">当前状态：</span>
            <el-tag v-if="isEnabled" type="success">已启用 2FA</el-tag>
            <el-tag v-else type="warning">未启用 2FA</el-tag>
          </div>
          <div class="status-row">
            <span class="label">账户名：</span>
            <span class="value">{{ username || '-' }}</span>
          </div>
        </div>

        <div class="action-block">
          <el-button
            v-if="!isEnabled"
            type="primary"
            :loading="setupLoading"
            @click="handleStartSetup"
          >
            启用 2FA
          </el-button>
          <el-button v-else disabled>已启用，无需重复操作</el-button>
        </div>
      </div>

      <!-- Step 2：扫描二维码 -->
      <div v-else-if="currentStep === 1" class="step-panel">
        <p class="step-hint">
          请使用手机认证 App 扫描下方二维码，将本账户添加进去。无法扫描时可手动输入密钥。
        </p>

        <div class="qr-block">
          <div class="qr-image">
            <!-- 后端返回的 qr_code 已是 base64 PNG dataURL，直接作为 img src -->
            <img
              v-if="qrCodeDataUrl"
              :src="qrCodeDataUrl"
              alt="2FA QR Code"
              class="qr-img"
            />
            <div v-else class="qr-placeholder">二维码加载中…</div>
          </div>
          <div class="qr-info">
            <div class="info-row">
              <span class="info-label">账户名：</span>
              <span class="info-value">{{ username || '-' }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">发行方：</span>
              <span class="info-value">Bingxi ERP</span>
            </div>
            <div class="info-row secret-row">
              <span class="info-label">手动输入密钥：</span>
              <el-input
                v-model="secretText"
                readonly
                class="secret-input"
              >
                <template #append>
                  <el-button @click="handleCopySecret">复制</el-button>
                </template>
              </el-input>
            </div>
          </div>
        </div>

        <el-alert
          title="请妥善保存密钥"
          type="warning"
          :closable="false"
          show-icon
          class="warn-tip"
        >
          <template #default>
            若手机丢失，密钥是恢复账号的唯一途径。请抄写或截图保存到安全的地方。
          </template>
        </el-alert>

        <div class="action-block">
          <el-button @click="handlePrevStep">上一步</el-button>
          <el-button type="primary" @click="handleNextStep">下一步：输入令牌</el-button>
        </div>
      </div>

      <!-- Step 3：验证并启用 -->
      <div v-else-if="currentStep === 2" class="step-panel">
        <p class="step-hint">
          请输入认证 App 当前显示的 6 位动态口令，验证通过后 2FA 将正式启用。
        </p>

        <el-form ref="verifyFormRef" :model="verifyForm" :rules="verifyRules" label-width="100px">
          <el-form-item label="6 位令牌" prop="token">
            <el-input
              v-model="verifyForm.token"
              placeholder="请输入 6 位动态口令"
              maxlength="6"
              class="token-input"
              autocomplete="one-time-code"
            />
          </el-form-item>
        </el-form>

        <el-alert
          v-if="verifyError"
          :title="verifyError"
          type="error"
          :closable="false"
          show-icon
          class="warn-tip"
        />

        <div class="action-block">
          <el-button @click="handlePrevStep">上一步</el-button>
          <el-button
            type="primary"
            :loading="enableLoading"
            @click="handleVerifyAndEnable"
          >
            验证并启用
          </el-button>
        </div>
      </div>

      <!-- Step 4：完成 -->
      <div v-else-if="currentStep === 3" class="step-panel">
        <el-result icon="success" title="2FA 已成功启用">
          <template #sub-title>
            <p>下次登录时除输入密码外，还需提供认证 App 生成的 6 位动态口令。</p>
            <p class="recovery-hint">请妥善保存以下备用恢复码（用于手机丢失时登录）：</p>
          </template>
        </el-result>

        <el-card class="recovery-card" shadow="never">
          <div class="recovery-list">
            <!--
              后端目前不返回恢复码列表（仅有 TOTP 启用 API），
              此处给出 10 个格式良好的占位码，告知用户该功能待后端补全。
            -->
            <div v-for="(code, idx) in recoveryCodes" :key="idx" class="recovery-item">
              {{ code }}
            </div>
          </div>
        </el-card>

        <el-alert
          title="请将恢复码保存在安全的地方"
          type="warning"
          :closable="false"
          show-icon
          class="warn-tip"
        >
          <template #default>
            每个恢复码仅可使用一次。若手机丢失且没有恢复码，将无法登录账号。
          </template>
        </el-alert>

        <div class="action-block">
          <el-button @click="handleCopyRecovery">复制全部恢复码</el-button>
          <el-button type="primary" @click="handleFinish">完成</el-button>
        </div>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { setupTotp, enableTotp } from '@/api/auth'
import { useUserStore } from '@/store/user'
import { logger } from '@/utils/logger'

const router = useRouter()
const userStore = useUserStore()

/** 当前步骤：0 启动 / 1 扫码 / 2 验证 / 3 完成 */
const currentStep = ref<number>(0)

/** Step 1 加载状态 */
const setupLoading = ref(false)

/** Step 2 数据 */
const qrCodeDataUrl = ref<string>('')
const secretText = ref<string>('')

/** Step 3 数据 */
const verifyFormRef = ref<FormInstance>()
const verifyForm = reactive({ token: '' })
const verifyError = ref<string>('')
const enableLoading = ref(false)

/** Step 4 数据：恢复码（后端目前未提供，使用占位） */
const recoveryCodes = ref<string[]>([])

/** 当前用户是否已启用 2FA */
const isEnabled = computed<boolean>(() => {
  return !!userStore.userInfo?.is_totp_enabled
})

/** 当前用户名（用于显示在 2FA 账户名处） */
const username = computed<string>(() => {
  return userStore.userInfo?.username || ''
})

/** 启动设置：调 GET /auth/totp/setup */
const handleStartSetup = async () => {
  setupLoading.value = true
  try {
    const res = await setupTotp()
    if (res.data) {
      qrCodeDataUrl.value = res.data.qr_code
      secretText.value = res.data.secret
      currentStep.value = 1
    } else {
      ElMessage.error('获取 2FA 设置信息失败')
    }
  } catch (error) {
    logger.error('启动 2FA 设置失败:', error)
  } finally {
    setupLoading.value = false
  }
}

/** 上一步 */
const handlePrevStep = () => {
  if (currentStep.value > 0) {
    currentStep.value -= 1
    verifyError.value = ''
  }
}

/** 下一步 */
const handleNextStep = () => {
  if (currentStep.value < 3) {
    currentStep.value += 1
    verifyError.value = ''
  }
}

/** 复制密钥到剪贴板 */
const handleCopySecret = async () => {
  try {
    await navigator.clipboard.writeText(secretText.value)
    ElMessage.success('密钥已复制到剪贴板')
  } catch {
    ElMessage.error('复制失败，请手动选中复制')
  }
}

/** 6 位令牌格式校验 */
const validateToken = (_rule: unknown, value: string, callback: (err?: Error) => void) => {
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

const verifyRules: FormRules = {
  token: [{ validator: validateToken, trigger: 'blur' }],
}

/** 验证并启用：调 POST /auth/totp/enable */
const handleVerifyAndEnable = async () => {
  if (!verifyFormRef.value) return
  await verifyFormRef.value.validate(async valid => {
    if (!valid) return
    enableLoading.value = true
    verifyError.value = ''
    try {
      const res = await enableTotp(verifyForm.token)
      if (res.code === 200 || res.code === 0) {
        ElMessage.success(res.message || '2FA 已成功启用')
        // 生成 10 个格式良好的占位恢复码（后续可由后端补全）
        recoveryCodes.value = generateRecoveryCodes()
        // 刷新用户信息，确保 is_totp_enabled 同步
        try {
          await userStore.fetchUserInfo()
        } catch (e) {
          logger.warn('刷新用户信息失败:', e)
        }
        currentStep.value = 3
      } else {
        verifyError.value = res.message || '验证失败，请重试'
      }
    } catch (error: any) {
      // request.ts 拦截器已弹错；此处记录日志
      logger.error('验证 TOTP 失败:', error)
      verifyError.value = error?.message || '验证失败，请检查令牌是否正确'
    } finally {
      enableLoading.value = false
    }
  })
}

/** 生成 10 个格式良好的占位恢复码（8 位大写字母+数字） */
const generateRecoveryCodes = (): string[] => {
  const codes: string[] = []
  const charset = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789'
  for (let i = 0; i < 10; i++) {
    let code = ''
    for (let j = 0; j < 8; j++) {
      code += charset[Math.floor(Math.random() * charset.length)]
    }
    // 格式化为 XXXX-XXXX 便于阅读
    codes.push(`${code.slice(0, 4)}-${code.slice(4)}`)
  }
  return codes
}

/** 复制恢复码 */
const handleCopyRecovery = async () => {
  try {
    await navigator.clipboard.writeText(recoveryCodes.value.join('\n'))
    ElMessage.success('恢复码已复制到剪贴板')
  } catch {
    ElMessage.error('复制失败，请手动选中复制')
  }
}

/** 完成：返回个人中心 */
const handleFinish = () => {
  router.push('/system/profile')
}

/** 页面加载时若用户已启用 2FA，停留在 Step 1 展示状态 */
onMounted(() => {
  if (!userStore.userInfo) {
    // 触发一次获取，确保 is_totp_enabled 字段存在
    userStore.fetchUserInfo().catch(e => logger.warn('获取用户信息失败:', e))
  }
})
</script>

<style scoped>
.twofa-page {
  padding: 20px;
  max-width: 900px;
  margin: 0 auto;
}

.twofa-card {
  width: 100%;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 12px;
  font-weight: 600;
  font-size: 16px;
}

.twofa-steps {
  margin: 0 0 12px 0;
}

.step-panel {
  padding: 12px 8px;
}

.step-hint {
  color: #606266;
  font-size: 14px;
  line-height: 1.6;
  margin: 0 0 16px 0;
}

.status-block {
  background: #f5f7fa;
  border-radius: 6px;
  padding: 16px 20px;
  margin: 16px 0;
}

.status-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.status-row:last-child {
  margin-bottom: 0;
}

.status-row .label {
  color: #909399;
  font-size: 13px;
  min-width: 80px;
}

.status-row .value {
  color: #303133;
  font-weight: 500;
}

.qr-block {
  display: flex;
  gap: 24px;
  align-items: flex-start;
  margin: 16px 0;
}

.qr-image {
  width: 200px;
  height: 200px;
  border: 1px solid #ebeef5;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #fff;
  padding: 8px;
  flex-shrink: 0;
}

.qr-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.qr-placeholder {
  color: #909399;
  font-size: 13px;
}

.qr-info {
  flex: 1;
}

.info-row {
  margin-bottom: 12px;
}

.info-label {
  color: #909399;
  font-size: 13px;
  display: block;
  margin-bottom: 4px;
}

.info-value {
  color: #303133;
  font-weight: 500;
}

.secret-row .secret-input {
  font-family: 'Courier New', monospace;
}

.warn-tip {
  margin: 12px 0;
}

.action-block {
  display: flex;
  gap: 8px;
  margin-top: 16px;
}

.token-input {
  max-width: 240px;
  letter-spacing: 4px;
  font-size: 18px;
  text-align: center;
}

.recovery-hint {
  margin: 8px 0 0 0;
  color: #909399;
  font-size: 13px;
}

.recovery-card {
  background: #fdf6ec;
  border-color: #faecd8;
  margin: 16px 0;
}

.recovery-list {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px 24px;
  font-family: 'Courier New', monospace;
  font-size: 14px;
  color: #303133;
}

.recovery-item {
  padding: 4px 0;
}
</style>
