<!--
  登录页
  - 支持登录失败后展示账号锁定信息（红色 alert + 倒计时）
  - 用户名输入框失焦时可预检查账号锁定状态
  - 调 GET /security/lock-status?username=xxx
-->
<template>
  <div class="login-container">
    <div class="login-card">
      <h2 class="login-title">面料管理系统</h2>

      <!-- 账号锁定提示（红色 alert + 倒计时） -->
      <el-alert
        v-if="lockInfo.isLocked"
        :title="`账号已被锁定，请 ${lockInfo.remainingMinutes} 分钟后再试`"
        type="error"
        :closable="false"
        show-icon
        class="lock-alert"
      >
        <template #default>
          <div class="lock-content">
            <div>连续登录失败 {{ lockInfo.failedAttempts }} 次，账号已锁定</div>
            <div v-if="lockInfo.remainingMinutes > 0" class="lock-countdown">
              剩余等待时间：{{ lockInfo.remainingMinutes }} 分 {{ lockInfo.remainingSeconds }} 秒
            </div>
          </div>
        </template>
      </el-alert>

      <el-form ref="formRef" :model="loginForm" :rules="rules" @submit.prevent="handleLogin">
        <el-form-item prop="username">
          <el-input
            v-model="loginForm.username"
            placeholder="用户名"
            prefix-icon="User"
            size="large"
            @blur="handleUsernameBlur"
          />
        </el-form-item>
        <el-form-item prop="password">
          <el-input
            v-model="loginForm.password"
            type="password"
            placeholder="密码"
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
            登录
          </el-button>
        </el-form-item>
      </el-form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { useUserStore } from '@/store/user'
import { usePermissionStore } from '@/store/permission'
import { securityApi } from '@/api/security'
import { logger } from '@/utils/logger'

const router = useRouter()
const route = useRoute()
const userStore = useUserStore()
const permissionStore = usePermissionStore()

const formRef = ref<FormInstance>()
const loading = ref(false)

const loginForm = reactive({
  username: '',
  password: '',
})

const rules: FormRules = {
  username: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
  password: [{ required: true, message: '请输入密码', trigger: 'blur' }],
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
      ElMessage.info('账号已解除锁定，请重新登录')
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
    const res = await securityApi.checkLockStatus(loginForm.username)
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
    const res = await securityApi.checkLockStatus(loginForm.username)
    if (res.data) {
      applyLockStatus(res.data)
    }
  } catch (error) {
    logger.warn('刷新账号锁定状态失败:', error)
  }
}

async function handleLogin() {
  const form = formRef.value
  if (!form) return

  await form.validate(async valid => {
    if (!valid) return
    if (lockInfo.isLocked) {
      ElMessage.warning(`账号已被锁定，请 ${lockInfo.remainingMinutes} 分钟后再试`)
      return
    }

    loading.value = true
    try {
      await userStore.login(loginForm)

      if (userStore.userInfo?.permissions) {
        const permList = userStore.userInfo.permissions.map((code: string) => ({
          id: 0,
          name: code,
          code: code,
          type: 'menu',
          resource: code.split(':')[0] || code,
          action: code.split(':')[1] || '*',
        }))
        permissionStore.setPermissions(permList)
      }

      // 登录成功清空锁定提示
      clearLockInfo()
      ElMessage.success('登录成功')

      const redirect = (route.query.redirect as string) || '/'
      router.push(redirect)
    } catch (error: any) {
      ElMessage.error(error.message || '登录失败')
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
