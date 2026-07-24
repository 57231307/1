<!--
  登录页
  - 支持登录失败后展示账号锁定信息（红色 alert + 倒计时）
  - 用户名输入框失焦时可预检查账号锁定状态
  - 调 GET /security/lock-status?username=xxx
-->
<template>
  <div class="login-container">
    <div class="login-card">
      <!-- 批次 23 v5 P0-1：标题接入 i18n -->
      <h2 class="login-title">{{ $t('login.subtitle') }}</h2>

      <!-- 账号锁定提示（红色 alert + 倒计时） -->
      <el-alert
        v-if="lockInfo.isLocked"
        :title="$t('login.lockedAlert', { minutes: lockInfo.remainingMinutes })"
        type="error"
        :closable="false"
        show-icon
        class="lock-alert"
      >
        <template #default>
          <div class="lock-content">
            <div>{{ $t('login.failedAttempts', { count: lockInfo.failedAttempts }) }}</div>
            <div v-if="lockInfo.remainingMinutes > 0" class="lock-countdown">
              {{ $t('login.remainingTime', { minutes: lockInfo.remainingMinutes, seconds: lockInfo.remainingSeconds }) }}
            </div>
          </div>
        </template>
      </el-alert>

      <!-- 批次 23 v5 P0-2：表单可访问性 - 添加 role="form" 与 aria-label，供屏幕阅读器识别表单用途 -->
      <el-form
        ref="formRef"
        :model="loginForm"
        :rules="rules"
        role="form"
        :aria-label="$t('login.formLabel')"
        @submit.prevent="handleLogin"
      >
        <!-- P0-2：el-form-item 与 el-input 均补 aria-label，确保无 label 文本时屏幕阅读器仍可朗读字段用途 -->
        <el-form-item prop="username" :aria-label="$t('login.username')">
          <el-input
            v-model="loginForm.username"
            :placeholder="$t('login.username')"
            :aria-label="$t('login.username')"
            prefix-icon="User"
            size="large"
            @blur="handleUsernameBlur"
          />
        </el-form-item>
        <el-form-item prop="password" :aria-label="$t('login.password')">
          <el-input
            v-model="loginForm.password"
            type="password"
            :placeholder="$t('login.password')"
            :aria-label="$t('login.password')"
            prefix-icon="Lock"
            size="large"
            show-password
            @keyup.enter="handleLogin"
          />
        </el-form-item>
        <el-form-item>
          <el-button
            type="primary"
            size="large"
            style="width: 100%"
            :loading="loading"
            :disabled="lockInfo.isLocked"
            @click="handleLogin"
          >
            {{ $t('login.submit') }}
          </el-button>
        </el-form-item>
      </el-form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { useUserStore } from '@/store/user'
import { checkLockStatus } from '@/api/security'
import { logger } from '@/utils/logger'

const router = useRouter()
const route = useRoute()
const userStore = useUserStore()
/* 批次 23 v5 P0-1：useScope: 'global' 确保读取根 i18n messages（资源在 i18n/index.ts 全局注册） */
const { t } = useI18n({ useScope: 'global' })

const formRef = ref<FormInstance>()
const loading = ref(false)

const loginForm = reactive({
  username: '',
  password: '',
})

const rules: FormRules = {
  /* 批次 23 v5 P0-1：校验提示走 i18n。注意 rules 在 setup 初始化时一次性求值，切换语言后不会自动刷新；
     如需动态响应语言切换，可将 message 改为函数形式，留作后续迭代优化。 */
  username: [{ required: true, message: t('login.usernameRequired'), trigger: 'blur' }],
  password: [{ required: true, message: t('login.passwordRequired'), trigger: 'blur' }],
}

/** 账号锁定信息 */
const lockInfo = reactive({
  isLocked: false,
  failedAttempts: 0,
  remainingMinutes: 0,
  remainingSeconds: 0,
  maxAttempts: 5,
  lockEndAt: 0, // 时间戳
})

/** 倒计时定时器 */
let countdownTimer: number | null = null

/**
 * 处理账号锁定状态：设置 isLocked + 启动倒计时
 */
const applyLockStatus = (status: {
  is_locked: boolean
  failed_attempts: number
  locked_until: string | null
  max_attempts: number
}) => {
  if (status.is_locked && status.locked_until) {
    const endTime = new Date(status.locked_until).getTime()
    const now = Date.now()
    if (endTime > now) {
      lockInfo.isLocked = true
      lockInfo.failedAttempts = status.failed_attempts
      lockInfo.maxAttempts = status.max_attempts
      lockInfo.lockEndAt = endTime
      startCountdown()
    } else {
      // 已过期，清空锁定状态
      clearLockInfo()
    }
  } else {
    clearLockInfo()
  }
}

/** 清空锁定信息 */
const clearLockInfo = () => {
  lockInfo.isLocked = false
  lockInfo.failedAttempts = 0
  lockInfo.remainingMinutes = 0
  lockInfo.remainingSeconds = 0
  lockInfo.lockEndAt = 0
  if (countdownTimer !== null) {
    window.clearInterval(countdownTimer)
    countdownTimer = null
  }
}

/** 启动倒计时 */
const startCountdown = () => {
  if (countdownTimer !== null) {
    window.clearInterval(countdownTimer)
  }
  const update = () => {
    const remainMs = lockInfo.lockEndAt - Date.now()
    if (remainMs <= 0) {
      clearLockInfo()
      ElMessage.info(t('login.unlocked'))
      return
    }
    lockInfo.remainingMinutes = Math.floor(remainMs / 60000)
    lockInfo.remainingSeconds = Math.floor((remainMs % 60000) / 1000)
  }
  update()
  countdownTimer = window.setInterval(update, 1000)
}

/**
 * 用户名输入框失焦：预检查账号是否已被锁定
 */
const handleUsernameBlur = async () => {
  if (!loginForm.username || loginForm.username.length < 3) return
  try {
    const res = await checkLockStatus(loginForm.username)
    if (res.data) {
      applyLockStatus(res.data)
    }
  } catch (error) {
    // 检查失败不影响主流程
    logger.warn('预检查账号锁定状态失败:', error)
  }
}

/**
 * 登录失败时尝试检查锁定状态（响应 401 后由错误处理自动调用）
 */
const refreshLockStatus = async () => {
  if (!loginForm.username) return
  try {
    const res = await checkLockStatus(loginForm.username)
    if (res.data) {
      applyLockStatus(res.data)
    }
  } catch (error) {
    logger.warn('刷新账号锁定状态失败:', error)
  }
}

/**
 * 批次 22 v5 P0-2 修复：安全重定向白名单校验
 *
 * 防止 Open Redirect 漏洞：登录跳转参数 redirect 若被攻击者构造为
 * 外部站点（如 //evil.com）或 javascript: 协议，可在登录后被引导至恶意站点。
 * 本函数仅允许以单个 / 开头的相对路径，拒绝绝对 URL、协议相对 URL、反斜杠路径。
 *
 * @param raw 原始 redirect 参数（通常来自 route.query.redirect）
 * @returns 安全的内部跳转路径
 */
function safeRedirect(raw: unknown): string {
  if (typeof raw !== 'string' || !raw) return '/'
  if (/^(https?:)?\/\//i.test(raw)) return '/'
  if (/^\\\\/i.test(raw)) return '/'
  if (!raw.startsWith('/')) return '/'
  if (raw.startsWith('//') || raw.startsWith('/\\')) return '/'
  return raw
}

async function handleLogin() {
  const form = formRef.value
  if (!form) return

  await form.validate(async valid => {
    if (!valid) return
    if (lockInfo.isLocked) {
      ElMessage.warning(t('login.lockedAlert', { minutes: lockInfo.remainingMinutes }))
      return
    }

    loading.value = true
    try {
      const loginResult = await userStore.login(loginForm)

      // FE-P-2 修复（2026-06-26 第二次审计第二优先级）：
      // permissions 已在 userStore.login() 中合并到 userInfo，
      // v-permission 指令直接从 userStore.userInfo.permissions 读取字符串数组判断。
      // 原 permissionStore 写入路径是死代码（无读取方），已移除。

      // 登录成功清空锁定提示
      clearLockInfo()
      ElMessage.success(t('login.success'))

      // 批次 198 P0-2：密码过期引导改密（后端 password_expired=true 表示超过 90 天未修改）
      // 批次 389 FE-P2-3：密码过期文案改用 i18n（passwordExpired* 系列 key）
      if (loginResult?.password_expired) {
        ElMessageBox.confirm(
          t('login.passwordExpiredMessage'),
          t('login.passwordExpiredTitle'),
          {
            confirmButtonText: t('login.passwordExpiredConfirm'),
            cancelButtonText: t('login.passwordExpiredLater'),
            type: 'warning',
          },
        )
          .then(() => {
            router.push({ path: '/system/security' })
          })
          .catch(() => {
            // 用户选择稍后提醒，仍允许进入系统（不阻塞业务），但日志已记录
            const redirect = safeRedirect(route.query.redirect)
            router.push(redirect)
          })
        return
      }

      // 批次 22 v5 P0-2：使用 safeRedirect 校验跳转目标，防止 Open Redirect
      const redirect = safeRedirect(route.query.redirect)
      router.push(redirect)
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      const message = error instanceof Error ? error.message : String(error)
      ElMessage.error(message || t('login.failedFallback'))
      // 登录失败后异步检查账号是否被锁定
      refreshLockStatus()
    } finally {
      loading.value = false
    }
  })
}

/** 组件卸载时清理定时器 */
onUnmounted(() => {
  if (countdownTimer !== null) {
    window.clearInterval(countdownTimer)
    countdownTimer = null
  }
})
</script>

<style scoped>
.login-container {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
.login-card {
  width: 400px;
  padding: 40px;
  background: #fff;
  border-radius: 8px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
}
.login-title {
  text-align: center;
  margin-bottom: 30px;
  color: #303133;
}
.lock-alert {
  margin-bottom: 16px;
}
.lock-content {
  font-size: 13px;
  line-height: 1.6;
}
.lock-countdown {
  margin-top: 4px;
  color: #f56c6c;
  font-weight: 500;
}
</style>
