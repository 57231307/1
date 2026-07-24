<template>
  <div class="setup-container">
    <div class="setup-card">
      <div class="setup-header">
        <h1>{{ $t('setupPage.title') }}</h1>
        <p class="subtitle">{{ $t('setupPage.subtitle') }}</p>
      </div>

      <el-steps :active="currentStep" finish-status="success" class="setup-steps">
        <el-step :title="$t('setupPage.steps.environment')" />
        <el-step :title="$t('setupPage.steps.database')" />
        <el-step :title="$t('setupPage.steps.admin')" />
        <el-step :title="$t('setupPage.steps.complete')" />
      </el-steps>

      <!-- 步骤 1: 环境检查 -->
      <div v-if="currentStep === 0" class="step-content">
        <h3>{{ $t('setupPage.steps.environment') }}</h3>
        <div class="check-list">
          <div v-for="item in envChecks" :key="item.name" class="check-item">
            <el-icon :class="item.status ? 'success' : 'error'">
              <CircleCheckFilled v-if="item.status" />
              <CircleCloseFilled v-else />
            </el-icon>
            <span>{{ item.name }}</span>
            <span class="check-status">{{ item.status ? $t('setupPage.envCheck.pass') : $t('setupPage.envCheck.fail') }}</span>
          </div>
        </div>
        <div class="step-actions">
          <el-button type="primary" :loading="checking" @click="checkEnvironment">
            {{ checking ? $t('setupPage.envCheck.checking') : $t('setupPage.envCheck.recheck') }}
          </el-button>
          <el-button type="primary" :disabled="!allChecksPassed" @click="nextStep">
            {{ $t('setupPage.envCheck.next') }}
          </el-button>
        </div>
      </div>

      <!-- 步骤 2: 数据库配置 -->
      <div v-if="currentStep === 1" class="step-content">
        <h3>{{ $t('setupPage.db.title') }}</h3>
        <el-form
          ref="dbFormRef"
          :model="dbConfig"
          :rules="dbRules"
          label-width="120px"
          class="config-form"
          :aria-label="$t('setupPage.aria.dbForm')"
        >
          <el-form-item :label="$t('setupPage.db.host')" prop="host">
            <el-input v-model="dbConfig.host" placeholder="localhost" />
          </el-form-item>
          <el-form-item :label="$t('setupPage.db.port')" prop="port">
            <el-input v-model="dbConfig.port" placeholder="5432" />
          </el-form-item>
          <el-form-item :label="$t('setupPage.db.name')" prop="name">
            <el-input v-model="dbConfig.name" placeholder="bingxi" />
          </el-form-item>
          <el-form-item :label="$t('setupPage.db.username')" prop="username">
            <el-input v-model="dbConfig.username" placeholder="bingxi" />
          </el-form-item>
          <el-form-item :label="$t('setupPage.db.password')" prop="password">
            <el-input v-model="dbConfig.password" type="password" :placeholder="$t('setupPage.db.passwordPlaceholder')" />
          </el-form-item>
          <el-form-item :label="$t('setupPage.db.initToken')" prop="init_token">
            <el-input
              v-model="dbConfig.init_token"
              type="password"
              :placeholder="$t('setupPage.db.initTokenPlaceholder')"
            />
          </el-form-item>
        </el-form>
        <div class="step-actions">
          <el-button @click="prevStep">{{ $t('setupPage.envCheck.prev') }}</el-button>
          <el-button type="primary" :loading="testing" @click="testConnection">
            {{ testing ? $t('setupPage.db.testing') : $t('setupPage.db.testConnection') }}
          </el-button>
          <el-button type="primary" :disabled="!dbConnected" @click="nextStep">{{ $t('setupPage.envCheck.next') }}</el-button>
        </div>
      </div>

      <!-- 步骤 3: 创建管理员 -->
      <div v-if="currentStep === 2" class="step-content">
        <h3>{{ $t('setupPage.admin.title') }}</h3>
        <el-form
          ref="adminFormRef"
          :model="adminConfig"
          :rules="adminRules"
          label-width="120px"
          class="config-form"
          :aria-label="$t('setupPage.aria.adminForm')"
        >
          <el-form-item :label="$t('setupPage.admin.username')" prop="username">
            <el-input v-model="adminConfig.username" placeholder="admin" />
          </el-form-item>
          <el-form-item :label="$t('setupPage.admin.password')" prop="password">
            <el-input v-model="adminConfig.password" type="password" :placeholder="$t('setupPage.db.passwordPlaceholder')" />
          </el-form-item>
          <el-form-item :label="$t('setupPage.admin.confirmPassword')" prop="confirmPassword">
            <el-input
              v-model="adminConfig.confirmPassword"
              type="password"
              :placeholder="$t('setupPage.admin.confirmPasswordPlaceholder')"
            />
          </el-form-item>
          <el-form-item :label="$t('setupPage.admin.email')" prop="email">
            <el-input v-model="adminConfig.email" placeholder="admin@example.com" />
          </el-form-item>
        </el-form>
        <div class="step-actions">
          <el-button @click="prevStep">{{ $t('setupPage.envCheck.prev') }}</el-button>
          <el-button type="primary" :disabled="!isAdminValid" @click="nextStep">{{ $t('setupPage.envCheck.next') }}</el-button>
        </div>
      </div>

      <!-- 步骤 4: 完成安装 -->
      <div v-if="currentStep === 3" class="step-content">
        <h3>{{ $t('setupPage.complete.title') }}</h3>
        <div class="install-summary">
          <p>{{ $t('setupPage.complete.willExecute') }}</p>
          <ul>
            <li>{{ $t('setupPage.complete.createSchema') }}</li>
            <li>{{ $t('setupPage.complete.initData') }}</li>
            <li>{{ $t('setupPage.complete.createAdminWithName', { name: adminConfig.username }) }}</li>
          </ul>
        </div>
        <div class="step-actions">
          <el-button @click="prevStep">{{ $t('setupPage.envCheck.prev') }}</el-button>
          <el-button type="primary" :loading="installing" :disabled="installed" @click="install">
            {{ installing ? $t('setupPage.complete.installing') : installed ? $t('setupPage.complete.installed') : $t('setupPage.complete.startInstall') }}
          </el-button>
        </div>
      </div>

      <!-- 安装完成 -->
      <div v-if="currentStep === 4" class="step-content">
        <div class="success-icon">
          <el-icon><CircleCheckFilled /></el-icon>
        </div>
        <h3>{{ $t('setupPage.complete.successTitle') }}</h3>
        <p>{{ $t('setupPage.complete.successDesc') }}</p>
        <div class="step-actions">
          <el-button type="primary" @click="goToLogin">{{ $t('setupPage.complete.goToLogin') }}</el-button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { CircleCheckFilled, CircleCloseFilled } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import type { FormItemRule } from 'element-plus'
import { logger } from '@/utils/logger'

const router = useRouter()
const { t } = useI18n({ useScope: 'global' })

const currentStep = ref(0)
const checking = ref(false)
const testing = ref(false)
const installing = ref(false)
const installed = ref(false)
const dbConnected = ref(false)

// 环境检查
const envChecks = ref([
  { name: t('setupPage.envChecks.backendApi'), status: false, detail: '检查后端API是否正常响应' },
  { name: t('setupPage.envChecks.disk'), status: false, detail: '检查是否有足够的磁盘空间（至少1GB）' },
  { name: t('setupPage.envChecks.memory'), status: false, detail: '检查系统内存是否充足（至少512MB）' },
])

const allChecksPassed = computed(() => envChecks.value.every(item => item.status))

// 数据库配置
const dbFormRef = ref()
const dbConfig = ref({
  host: 'localhost',
  port: '5432',
  name: 'bingxi',
  username: 'bingxi',
  password: '',
  // 批次 24 v6 P0-3 修复：添加 init_token 字段。
  // 后端 init_token_middleware 强制要求 X-Init-Token 头匹配环境变量 INIT_TOKEN，
  // 否则返回 401（fail-secure）。原 Setup.vue 未传此头导致首次部署初始化必然失败。
  init_token: '',
})
const dbRules = {
  host: [{ required: true, message: t('setupPage.validation.hostRequired'), trigger: 'blur' }],
  port: [{ required: true, message: t('setupPage.validation.portRequired'), trigger: 'blur' }],
  name: [{ required: true, message: t('setupPage.validation.nameRequired'), trigger: 'blur' }],
  username: [{ required: true, message: t('setupPage.validation.usernameRequired'), trigger: 'blur' }],
  init_token: [{ required: true, message: t('setupPage.validation.initTokenRequired'), trigger: 'blur' }],
}

// 管理员配置
const adminFormRef = ref()
const adminConfig = ref({
  username: 'admin',
  password: '',
  confirmPassword: '',
  email: 'admin@example.com',
})
const adminRules = {
  username: [{ required: true, message: t('setupPage.validation.adminUsernameRequired'), trigger: 'blur' }],
  password: [
    { required: true, message: t('setupPage.validation.passwordRequired'), trigger: 'blur' },
    { min: 6, message: t('setupPage.validation.passwordMinLength'), trigger: 'blur' },
  ],
  confirmPassword: [
    { required: true, message: t('setupPage.validation.confirmPasswordRequired'), trigger: 'blur' },
    {
      validator: ((_rule: unknown, value: string, callback: (error?: Error) => void) => {
        if (value !== adminConfig.value.password) {
          callback(new Error(t('setupPage.validation.passwordMismatch')))
        } else {
          callback()
        }
      }) as FormItemRule['validator'],
      trigger: 'blur',
    },
  ],
  email: [{ type: 'email', message: t('setupPage.validation.emailInvalid'), trigger: 'blur' }],
}

const isAdminValid = computed(() => {
  return (
    adminConfig.value.username &&
    adminConfig.value.password &&
    adminConfig.value.password === adminConfig.value.confirmPassword &&
    adminConfig.value.password.length >= 6
  )
})

// 检查环境
async function checkEnvironment() {
  checking.value = true
  try {
    // 检查后端API服务
    const healthRes = await fetch('/health')
    const healthData = await healthRes.json()
    envChecks.value[0].status = healthRes.ok && healthData.status === 'healthy'

    // 检查磁盘空间（通过健康检查接口的checks）
    if (healthData.checks && healthData.checks.disk) {
      envChecks.value[1].status = healthData.checks.disk.status === 'healthy'
    } else {
      envChecks.value[1].status = true
    }

    // 检查系统内存（通过健康检查接口的checks）
    if (healthData.checks && healthData.checks.memory) {
      envChecks.value[2].status = healthData.checks.memory.status === 'healthy'
    } else {
      envChecks.value[2].status = true
    }
  } catch (error) {
    logger.error(t('setupPage.message.envCheckFailed'), error)
  } finally {
    checking.value = false
  }
}

// 测试数据库连接
async function testConnection() {
  testing.value = true
  try {
    const res = await fetch('/api/v1/erp/init/test-database', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(dbConfig.value),
    })
    const data = await res.json()
    if (data.code === 200 && data.data?.success) {
      dbConnected.value = true
      ElMessage.success(t('setupPage.message.dbConnectSuccess'))
    } else {
      dbConnected.value = false
      ElMessage.error(data.data?.message || data.message || t('setupPage.message.dbConnectFailed'))
    }
  } catch (error) {
    dbConnected.value = false
    ElMessage.error(t('setupPage.message.dbConnectFailed'))
  } finally {
    testing.value = false
  }
}

// 安装系统
async function install() {
  installing.value = true
  try {
    // 批次 24 v6 P0-3 修复：添加 X-Init-Token 请求头。
    // 后端 init_token_middleware 强制校验此头与 INIT_TOKEN 环境变量匹配，
    // 缺失或不匹配时返回 401，防止未授权的初始化操作。
    const res = await fetch('/api/v1/erp/init/initialize-with-db', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Init-Token': dbConfig.value.init_token,
      },
      body: JSON.stringify({
        db_config: dbConfig.value,
        admin_username: adminConfig.value.username,
        admin_password: adminConfig.value.password,
        admin_email: adminConfig.value.email,
      }),
    })
    const data = await res.json()
    if (data.success) {
      installed.value = true
      ElMessage.success(t('setupPage.message.installSuccess'))
      currentStep.value = 4
    } else {
      ElMessage.error(data.message || t('setupPage.message.installFailed'))
    }
  } catch (error) {
    ElMessage.error(t('setupPage.message.installFailed'))
  } finally {
    installing.value = false
  }
}

function nextStep() {
  currentStep.value++
}

function prevStep() {
  currentStep.value--
}

function goToLogin() {
  router.push('/login')
}

// 初始化检查
checkEnvironment()
</script>

<style scoped>
.setup-container {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  padding: 20px;
}

.setup-card {
  background: white;
  border-radius: 12px;
  padding: 40px;
  width: 100%;
  max-width: 700px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
}

.setup-header {
  text-align: center;
  margin-bottom: 30px;
}

.setup-header h1 {
  margin: 0;
  color: #303133;
  font-size: 28px;
}

.subtitle {
  color: #909399;
  margin-top: 10px;
}

.setup-steps {
  margin-bottom: 30px;
}

.step-content {
  min-height: 300px;
}

.step-content h3 {
  margin: 0 0 20px 0;
  color: #303133;
  font-size: 20px;
}

.check-list {
  margin-bottom: 20px;
}

.check-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px;
  border-bottom: 1px solid #ebeef5;
}

.check-item:last-child {
  border-bottom: none;
}

.check-item .success {
  color: #67c23a;
  font-size: 20px;
}

.check-item .error {
  color: #f56c6c;
  font-size: 20px;
}

.check-status {
  margin-left: auto;
  color: #909399;
}

.config-form {
  max-width: 500px;
}

.step-actions {
  display: flex;
  gap: 10px;
  margin-top: 30px;
  justify-content: flex-end;
}

.install-summary {
  background: #f5f7fa;
  padding: 20px;
  border-radius: 8px;
  margin-bottom: 20px;
}

.install-summary ul {
  margin: 10px 0 0 20px;
  color: #606266;
}

.install-summary li {
  margin-bottom: 8px;
}

.success-icon {
  text-align: center;
  margin-bottom: 20px;
}

.success-icon .el-icon {
  font-size: 80px;
  color: #67c23a;
}
</style>
