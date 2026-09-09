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
            <span class="check-status">{{
              item.status ? $t('setupPage.envCheck.pass') : $t('setupPage.envCheck.fail')
            }}</span>
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
            <el-input
              v-model="dbConfig.password"
              type="password"
              :placeholder="$t('setupPage.db.passwordPlaceholder')"
            />
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
          <el-button type="primary" :disabled="!dbConnected" @click="nextStep">{{
            $t('setupPage.envCheck.next')
          }}</el-button>
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
            <el-input
              v-model="adminConfig.password"
              type="password"
              :placeholder="$t('setupPage.db.passwordPlaceholder')"
            />
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
          <el-button type="primary" :disabled="!isAdminValid" @click="nextStep">{{
            $t('setupPage.envCheck.next')
          }}</el-button>
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
            <li>
              {{ $t('setupPage.complete.createAdminWithName', { name: adminConfig.username }) }}
            </li>
          </ul>
        </div>
        <div class="step-actions">
          <el-button @click="prevStep">{{ $t('setupPage.envCheck.prev') }}</el-button>
          <el-button type="primary" :loading="installing" :disabled="installed" @click="install">
            {{
              installing
                ? $t('setupPage.complete.installing')
                : installed
                  ? $t('setupPage.complete.installed')
                  : $t('setupPage.complete.startInstall')
            }}
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
          <el-button type="primary" :loading="goToLoginLoading" @click="goToLogin">{{
            $t('setupPage.complete.goToLogin')
          }}</el-button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { useRouter } from 'vue-router';
import { resetInitStatus } from '@/router';
import { useI18n } from 'vue-i18n';
import { CircleCheckFilled, CircleCloseFilled } from '@element-plus/icons-vue';
import { ElMessage } from 'element-plus';
import type { FormItemRule } from 'element-plus';
import { logger } from '@/utils/logger';

const router = useRouter();
const { t } = useI18n({ useScope: 'global' });

const currentStep = ref(0);
const checking = ref(false);
const testing = ref(false);
const installing = ref(false);
const installed = ref(false);
const dbConnected = ref(false);

// 环境检查
const envChecks = ref([
  {
    name: t('setupPage.envChecks.backendApi'),
    status: false,
    detail: t('setupPage.envChecks.detail.backendApi'),
  },
  {
    name: t('setupPage.envChecks.disk'),
    status: false,
    detail: t('setupPage.envChecks.detail.disk'),
  },
  {
    name: t('setupPage.envChecks.memory'),
    status: false,
    detail: t('setupPage.envChecks.detail.memory'),
  },
]);

const allChecksPassed = computed(() => envChecks.value.every(item => item.status));

// 数据库配置
const dbFormRef = ref();
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
});
const dbRules = {
  host: [{ required: true, message: t('setupPage.validation.hostRequired'), trigger: 'blur' }],
  port: [{ required: true, message: t('setupPage.validation.portRequired'), trigger: 'blur' }],
  name: [{ required: true, message: t('setupPage.validation.nameRequired'), trigger: 'blur' }],
  username: [
    { required: true, message: t('setupPage.validation.usernameRequired'), trigger: 'blur' },
  ],
  init_token: [
    { required: true, message: t('setupPage.validation.initTokenRequired'), trigger: 'blur' },
  ],
};

// 管理员配置
const adminFormRef = ref();
const adminConfig = ref({
  username: 'admin',
  password: '',
  confirmPassword: '',
  email: 'admin@example.com',
});
const adminRules = {
  username: [
    { required: true, message: t('setupPage.validation.adminUsernameRequired'), trigger: 'blur' },
  ],
  password: [
    { required: true, message: t('setupPage.validation.passwordRequired'), trigger: 'blur' },
    { min: 8, message: t('setupPage.validation.passwordMinLength'), trigger: 'blur' },
    {
      // 与后端 PasswordPolicy::default() 对齐：大写+小写+数字+特殊字符
      validator: ((_rule: unknown, value: string, callback: (error?: Error) => void) => {
        if (!/[A-Z]/.test(value) || !/[a-z]/.test(value) || !/[0-9]/.test(value)) {
          callback(new Error(t('setupPage.validation.passwordComplexity')));
        } else if (!/[^A-Za-z0-9]/.test(value)) {
          callback(new Error(t('setupPage.validation.passwordComplexity')));
        } else {
          callback();
        }
      }) as FormItemRule['validator'],
      trigger: 'blur',
    },
  ],
  confirmPassword: [
    { required: true, message: t('setupPage.validation.confirmPasswordRequired'), trigger: 'blur' },
    {
      validator: ((_rule: unknown, value: string, callback: (error?: Error) => void) => {
        if (value !== adminConfig.value.password) {
          callback(new Error(t('setupPage.validation.passwordMismatch')));
        } else {
          callback();
        }
      }) as FormItemRule['validator'],
      trigger: 'blur',
    },
  ],
  email: [{ type: 'email', message: t('setupPage.validation.emailInvalid'), trigger: 'blur' }],
};

// 管理员密码规则与后端 PasswordPolicy::default() 对齐（utils/password_validator.rs）：
// ≥8 位 + 大写 + 小写 + 数字 + 特殊字符。前后端不一致会导致前端放行、
// 后端 initialize() 拒绝（ValidationError），用户在最后一步被卡死
const adminPasswordValid = computed(() => {
  const pwd = adminConfig.value.password;
  return (
    pwd.length >= 8 &&
    /[A-Z]/.test(pwd) &&
    /[a-z]/.test(pwd) &&
    /[0-9]/.test(pwd) &&
    /[^A-Za-z0-9]/.test(pwd)
  );
});

const isAdminValid = computed(() => {
  return (
    adminConfig.value.username &&
    adminPasswordValid.value &&
    adminConfig.value.password === adminConfig.value.confirmPassword
  );
});

// 检查环境
async function checkEnvironment() {
  checking.value = true;
  try {
    // Setup 模式（首次部署，数据库未连接）下后端仅暴露 /init/* 路由，
    // /api/v1/erp/health 返回 404，res.json() 解析抛错 → 三项检查全挂 →
    // 「下一步」永远禁用，引导流程卡死在步骤 1（部署包 v2026.9.7.1357 后排查发现）。
    // 修复：以后端可达性为主判定——/init/status 在 Setup 模式与完整模式都存在；
    // 磁盘/内存仅在 /health 可达（完整模式）时按 checks 校验，不可达时跳过不阻塞。
    let backendOk = false;
    try {
      const statusRes = await fetch('/api/v1/erp/init/status');
      if (statusRes.ok) {
        await statusRes.json();
        backendOk = true;
      }
    } catch (error) {
      logger.error(t('setupPage.message.envCheckFailed'), error);
    }
    envChecks.value[0].status = backendOk;

    // 磁盘/内存：仅完整模式 /health 提供 checks 字段；Setup 模式跳过（默认通过）
    envChecks.value[1].status = true;
    envChecks.value[2].status = true;
    if (backendOk) {
      try {
        const healthRes = await fetch('/api/v1/erp/health');
        if (healthRes.ok) {
          const healthData = await healthRes.json();
          if (healthData.checks && healthData.checks.disk) {
            envChecks.value[1].status = healthData.checks.disk.status === 'healthy';
          }
          if (healthData.checks && healthData.checks.memory) {
            envChecks.value[2].status = healthData.checks.memory.status === 'healthy';
          }
        }
      } catch {
        // /health 不可达（Setup 模式）：磁盘/内存维持跳过状态，不阻塞引导
      }
    }
  } finally {
    checking.value = false;
  }
}

// 测试数据库连接
async function testConnection() {
  testing.value = true;
  try {
    // X-Init-Token 与 initialize-with-db 保持同一契约形态：部署包
    // v2026.9.7.1357 的缺陷是该请求漏带此头（401 认证报错），补上以对齐
    // /init/* 端点的统一请求形态，并为未来服务端强制校验此头留好兼容
    const res = await fetch('/api/v1/erp/init/test-database', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        // CSRF 中间件要求 AJAX 标识：完整模式（空库可连接时启动形态）的
        // 中间件栈含 CSRF 校验，缺此头 → 403 CSRF_TOKEN_MISSING（CI 七轮实测）
        'X-Requested-With': 'XMLHttpRequest',
        'X-Init-Token': dbConfig.value.init_token,
      },
      body: JSON.stringify(dbConfig.value),
    });
    const data = await res.json();
    if (data.code === 200 && data.data?.success) {
      dbConnected.value = true;
      ElMessage.success(t('setupPage.message.dbConnectSuccess'));
    } else {
      dbConnected.value = false;
      ElMessage.error(data.data?.message || data.message || t('setupPage.message.dbConnectFailed'));
    }
  } catch (error) {
    dbConnected.value = false;
    ElMessage.error(t('setupPage.message.dbConnectFailed'));
  } finally {
    testing.value = false;
  }
}

// 安装系统
async function install() {
  installing.value = true;
  try {
    // 批次 24 v6 P0-3 修复：添加 X-Init-Token 请求头。
    // 后端 init_token_middleware 强制校验此头与 INIT_TOKEN 环境变量匹配，
    // 缺失或不匹配时返回 401，防止未授权的初始化操作。
    const res = await fetch('/api/v1/erp/init/initialize-with-db', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        // 同 testConnection：完整模式 CSRF 中间件要求 AJAX 标识
        'X-Requested-With': 'XMLHttpRequest',
        'X-Init-Token': dbConfig.value.init_token,
      },
      body: JSON.stringify({
        db_config: dbConfig.value,
        admin_username: adminConfig.value.username,
        admin_password: adminConfig.value.password,
        admin_email: adminConfig.value.email,
      }),
    });
    const data = await res.json();
    // 后端 ApiResponse 顶层无 success 字段（{code,data,message}），成功判定走
    // code===200 且 data.success（InitializationResult.success）。原 data.success
    // 顶层判断恒 undefined → 引导成功被 UI 误报为安装失败。
    const installOk = data.code === 200 ? !!data.data?.success : !!data.success;
    if (installOk) {
      installed.value = true;
      // 重置路由守卫的初始化状态缓存：初始化完成后跳转登录页时，
      // 守卫不再因缓存 initialized=false 把用户拉回 /setup
      resetInitStatus(true);
      ElMessage.success(t('setupPage.message.installSuccess'));
      currentStep.value = 4;
    } else {
      ElMessage.error(data.data?.message || data.message || t('setupPage.message.installFailed'));
    }
  } catch (error) {
    ElMessage.error(t('setupPage.message.installFailed'));
  } finally {
    installing.value = false;
  }
}

function nextStep() {
  currentStep.value++;
}

function prevStep() {
  currentStep.value--;
}

// Setup 模式初始化成功后后端会自退进程（systemd 拉起切完整模式），
// 点击登录前先探活 /health，避免撞上重启窗口的 connection refused
const goToLoginLoading = ref(false);
async function goToLogin() {
  goToLoginLoading.value = true;
  try {
    for (let i = 0; i < 30; i++) {
      try {
        const res = await fetch('/api/v1/erp/health', { signal: AbortSignal.timeout(2000) });
        if (res.ok) {
          router.push('/login');
          return;
        }
      } catch {
        // 服务重启中，等待后重试
      }
      await new Promise(r => setTimeout(r, 1000));
    }
    // 30s 未就绪仍跳转（由登录页给出网络错误反馈）
    router.push('/login');
  } finally {
    goToLoginLoading.value = false;
  }
}

// 初始化检查
checkEnvironment();
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
