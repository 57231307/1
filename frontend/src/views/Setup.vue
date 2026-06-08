<template>
  <div class="setup-container">
    <div class="setup-card">
      <div class="setup-header">
        <h1>面料管理系统</h1>
        <p class="subtitle">系统初始化向导</p>
      </div>

      <el-steps :active="currentStep" finish-status="success" class="setup-steps">
        <el-step title="环境检查" />
        <el-step title="数据库配置" />
        <el-step title="创建管理员" />
        <el-step title="完成安装" />
      </el-steps>

      <!-- 步骤 1: 环境检查 -->
      <div v-if="currentStep === 0" class="step-content">
        <h3>环境检查</h3>
        <div class="check-list">
          <div v-for="item in envChecks" :key="item.name" class="check-item">
            <el-icon :class="item.status ? 'success' : 'error'">
              <CircleCheckFilled v-if="item.status" />
              <CircleCloseFilled v-else />
            </el-icon>
            <span>{{ item.name }}</span>
            <span class="check-status">{{ item.status ? '通过' : '失败' }}</span>
          </div>
        </div>
        <div class="step-actions">
          <el-button type="primary" :loading="checking" @click="checkEnvironment">
            {{ checking ? '检查中...' : '重新检查' }}
          </el-button>
          <el-button type="primary" :disabled="!allChecksPassed" @click="nextStep">
            下一步
          </el-button>
        </div>
      </div>

      <!-- 步骤 2: 数据库配置 -->
      <div v-if="currentStep === 1" class="step-content">
        <h3>数据库配置</h3>
        <el-form
          ref="dbFormRef"
          :model="dbConfig"
          :rules="dbRules"
          label-width="120px"
          class="config-form"
        >
          <el-form-item label="数据库主机" prop="host">
            <el-input v-model="dbConfig.host" placeholder="localhost" />
          </el-form-item>
          <el-form-item label="数据库端口" prop="port">
            <el-input v-model="dbConfig.port" placeholder="5432" />
          </el-form-item>
          <el-form-item label="数据库名称" prop="name">
            <el-input v-model="dbConfig.name" placeholder="bingxi" />
          </el-form-item>
          <el-form-item label="数据库用户" prop="username">
            <el-input v-model="dbConfig.username" placeholder="bingxi" />
          </el-form-item>
          <el-form-item label="数据库密码" prop="password">
            <el-input v-model="dbConfig.password" type="password" placeholder="请输入密码" />
          </el-form-item>
        </el-form>
        <div class="step-actions">
          <el-button @click="prevStep">上一步</el-button>
          <el-button type="primary" :loading="testing" @click="testConnection">
            {{ testing ? '测试中...' : '测试连接' }}
          </el-button>
          <el-button type="primary" :disabled="!dbConnected" @click="nextStep"> 下一步 </el-button>
        </div>
      </div>

      <!-- 步骤 3: 创建管理员 -->
      <div v-if="currentStep === 2" class="step-content">
        <h3>创建管理员账号</h3>
        <el-form
          ref="adminFormRef"
          :model="adminConfig"
          :rules="adminRules"
          label-width="120px"
          class="config-form"
        >
          <el-form-item label="管理员用户名" prop="username">
            <el-input v-model="adminConfig.username" placeholder="admin" />
          </el-form-item>
          <el-form-item label="管理员密码" prop="password">
            <el-input v-model="adminConfig.password" type="password" placeholder="请输入密码" />
          </el-form-item>
          <el-form-item label="确认密码" prop="confirmPassword">
            <el-input
              v-model="adminConfig.confirmPassword"
              type="password"
              placeholder="请再次输入密码"
            />
          </el-form-item>
          <el-form-item label="邮箱" prop="email">
            <el-input v-model="adminConfig.email" placeholder="admin@example.com" />
          </el-form-item>
        </el-form>
        <div class="step-actions">
          <el-button @click="prevStep">上一步</el-button>
          <el-button type="primary" :disabled="!isAdminValid" @click="nextStep"> 下一步 </el-button>
        </div>
      </div>

      <!-- 步骤 4: 完成安装 -->
      <div v-if="currentStep === 3" class="step-content">
        <h3>完成安装</h3>
        <div class="install-summary">
          <p>系统将执行以下操作：</p>
          <ul>
            <li>创建数据库表结构</li>
            <li>初始化系统数据</li>
            <li>创建管理员账号: {{ adminConfig.username }}</li>
          </ul>
        </div>
        <div class="step-actions">
          <el-button @click="prevStep">上一步</el-button>
          <el-button type="primary" :loading="installing" :disabled="installed" @click="install">
            {{ installing ? '安装中...' : installed ? '安装完成' : '开始安装' }}
          </el-button>
        </div>
      </div>

      <!-- 安装完成 -->
      <div v-if="currentStep === 4" class="step-content">
        <div class="success-icon">
          <el-icon><CircleCheckFilled /></el-icon>
        </div>
        <h3>安装成功！</h3>
        <p>系统已成功安装，您现在可以登录使用。</p>
        <div class="step-actions">
          <el-button type="primary" @click="goToLogin"> 进入登录页面 </el-button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { CircleCheckFilled, CircleCloseFilled } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'

const router = useRouter()

const currentStep = ref(0)
const checking = ref(false)
const testing = ref(false)
const installing = ref(false)
const installed = ref(false)
const dbConnected = ref(false)

// 环境检查
const envChecks = ref([
  { name: '后端API服务', status: false, detail: '检查后端API是否正常响应' },
  { name: '磁盘空间', status: false, detail: '检查是否有足够的磁盘空间（至少1GB）' },
  { name: '系统内存', status: false, detail: '检查系统内存是否充足（至少512MB）' },
])

const allChecksPassed = computed(() => envChecks.value.every((item) => item.status))

// 数据库配置
const dbFormRef = ref()
const dbConfig = ref({
  host: 'localhost',
  port: '5432',
  name: 'bingxi',
  username: 'bingxi',
  password: '',
})
const dbRules = {
  host: [{ required: true, message: '请输入数据库主机', trigger: 'blur' }],
  port: [{ required: true, message: '请输入数据库端口', trigger: 'blur' }],
  name: [{ required: true, message: '请输入数据库名称', trigger: 'blur' }],
  username: [{ required: true, message: '请输入数据库用户', trigger: 'blur' }],
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
  username: [{ required: true, message: '请输入管理员用户名', trigger: 'blur' }],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 6, message: '密码至少6位', trigger: 'blur' },
  ],
  confirmPassword: [
    { required: true, message: '请确认密码', trigger: 'blur' },
    {
      validator: (_: any, value: string, callback: any) => {
        if (value !== adminConfig.value.password) {
          callback(new Error('两次输入的密码不一致'))
        } else {
          callback()
        }
      },
      trigger: 'blur',
    },
  ],
  email: [{ type: 'email', message: '请输入正确的邮箱地址', trigger: 'blur' }],
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
    const healthRes = await fetch('/api/v1/erp/health')
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
    console.error('环境检查失败:', error)
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
      ElMessage.success('数据库连接成功')
    } else {
      dbConnected.value = false
      ElMessage.error(data.data?.message || data.message || '数据库连接失败')
    }
  } catch (error) {
    dbConnected.value = false
    ElMessage.error('数据库连接失败')
  } finally {
    testing.value = false
  }
}

// 安装系统
async function install() {
  installing.value = true
  try {
    const res = await fetch('/api/v1/erp/init/initialize-with-db', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
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
      ElMessage.success('系统安装成功')
      currentStep.value = 4
    } else {
      ElMessage.error(data.message || '安装失败')
    }
  } catch (error) {
    ElMessage.error('安装失败')
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
