<template>
  <div class="security-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">登录安全</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>系统管理</el-breadcrumb-item>
          <el-breadcrumb-item>登录安全</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出日志
        </el-button>
      </div>
    </div>

    <!-- 统计卡片 -->
    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon login-icon">
              <el-icon><User /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">今日登录次数</div>
              <div class="stat-value">{{ stats.todayLogins }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon fail-icon">
              <el-icon><Warning /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">今日失败次数</div>
              <div class="stat-value">{{ stats.todayFailures }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon lock-icon">
              <el-icon><Lock /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">锁定账户数</div>
              <div class="stat-value">{{ stats.lockedAccounts }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon alert-icon">
              <el-icon><Bell /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">安全告警</div>
              <div class="stat-value">{{ stats.securityAlerts }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 登录日志 -->
    <el-card shadow="hover" class="table-card">
      <template #header>
        <div class="card-header">
          <span>登录日志</span>
          <el-form :inline="true" :model="queryParams" class="filter-form">
            <el-form-item label="用户名">
              <el-input v-model="queryParams.username" placeholder="请输入用户名" clearable @clear="handleQuery" />
            </el-form-item>
            <el-form-item label="登录状态">
              <el-select v-model="queryParams.status" placeholder="选择状态" clearable @change="handleQuery">
                <el-option label="成功" value="SUCCESS" />
                <el-option label="失败" value="FAILED" />
              </el-select>
            </el-form-item>
            <el-form-item label="登录时间">
              <el-date-picker v-model="queryParams.date_range" type="daterange" range-separator="至" start-placeholder="开始日期" end-placeholder="结束日期" @change="handleQuery" />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="handleQuery">
                <el-icon><Search /></el-icon>
                查询
              </el-button>
            </el-form-item>
          </el-form>
        </div>
      </template>

      <el-table v-loading="loading" :data="loginLogs" border stripe>
        <el-table-column type="index" label="序号" width="60" align="center" />
        <el-table-column prop="username" label="用户名" width="120" show-overflow-tooltip />
        <el-table-column prop="login_type" label="登录类型" width="100" align="center">
          <template #default="{ row }">
            <el-tag>{{ getTypeLabel(row.login_type) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="ip_address" label="IP地址" width="150" show-overflow-tooltip />
        <el-table-column prop="user_agent" label="浏览器" min-width="200" show-overflow-tooltip />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="fail_reason" label="失败原因" width="150" show-overflow-tooltip />
        <el-table-column prop="login_time" label="登录时间" width="180" align="center" />
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="queryParams.page"
          v-model:page-size="queryParams.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleSizeChange"
          @current-change="handleCurrentChange"
        />
      </div>
    </el-card>

    <!-- 锁定账户管理 -->
    <el-card shadow="hover" class="table-card">
      <template #header>
        <div class="card-header">
          <span>锁定账户管理</span>
        </div>
      </template>

      <el-table v-loading="lockLoading" :data="lockedAccounts" border stripe>
        <el-table-column type="index" label="序号" width="60" align="center" />
        <el-table-column prop="username" label="用户名" width="120" show-overflow-tooltip />
        <el-table-column prop="lock_reason" label="锁定原因" min-width="200" show-overflow-tooltip />
        <el-table-column prop="locked_at" label="锁定时间" width="180" align="center" />
        <el-table-column prop="unlock_at" label="解锁时间" width="180" align="center" />
        <el-table-column label="操作" width="120" align="center">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleUnlock(row)">解锁</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 安全告警 -->
    <el-card shadow="hover" class="table-card">
      <template #header>
        <div class="card-header">
          <span>安全告警</span>
        </div>
      </template>

      <el-table v-loading="alertLoading" :data="securityAlerts" border stripe>
        <el-table-column type="index" label="序号" width="60" align="center" />
        <el-table-column prop="alert_type" label="告警类型" width="120" align="center">
          <template #default="{ row }">
            <el-tag :type="getAlertType(row.alert_type)">{{ getAlertLabel(row.alert_type) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="username" label="用户名" width="120" show-overflow-tooltip />
        <el-table-column prop="ip_address" label="IP地址" width="150" show-overflow-tooltip />
        <el-table-column prop="description" label="告警描述" min-width="200" show-overflow-tooltip />
        <el-table-column prop="created_at" label="告警时间" width="180" align="center" />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getAlertStatusType(row.status)">{{ getAlertStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Download, Search, Warning, Lock, Bell } from '@element-plus/icons-vue'

// 统计数据
const stats = reactive({
  todayLogins: 0,
  todayFailures: 0,
  lockedAccounts: 0,
  securityAlerts: 0
})

// 查询参数
const queryParams = reactive({
  page: 1,
  page_size: 20,
  username: '',
  status: '',
  date_range: []
})

// 登录日志
const loading = ref(false)
const loginLogs = ref([])
const total = ref(0)

// 锁定账户
const lockLoading = ref(false)
const lockedAccounts = ref([])

// 安全告警
const alertLoading = ref(false)
const securityAlerts = ref([])

// 获取统计数据
const getStats = async () => {
  try {
    // TODO: 调用API获取统计数据
    stats.todayLogins = 156
    stats.todayFailures = 12
    stats.lockedAccounts = 3
    stats.securityAlerts = 5
  } catch (error) {
    console.error('获取统计数据失败:', error)
  }
}

// 获取登录日志
const getLoginLogs = async () => {
  loading.value = true
  try {
    // TODO: 调用API获取登录日志
    loginLogs.value = []
    total.value = 0
  } catch (error) {
    console.error('获取登录日志失败:', error)
  } finally {
    loading.value = false
  }
}

// 获取锁定账户
const getLockedAccounts = async () => {
  lockLoading.value = true
  try {
    // TODO: 调用API获取锁定账户
    lockedAccounts.value = []
  } catch (error) {
    console.error('获取锁定账户失败:', error)
  } finally {
    lockLoading.value = false
  }
}

// 获取安全告警
const getSecurityAlerts = async () => {
  alertLoading.value = true
  try {
    // TODO: 调用API获取安全告警
    securityAlerts.value = []
  } catch (error) {
    console.error('获取安全告警失败:', error)
  } finally {
    alertLoading.value = false
  }
}

// 查询
const handleQuery = () => {
  queryParams.page = 1
  getLoginLogs()
}

// 解锁账户
const handleUnlock = async (row: any) => {
  try {
    await ElMessageBox.confirm(`确认解锁账户 ${row.username}？`, '提示', { type: 'warning' })
    ElMessage.success('解锁成功')
    getLockedAccounts()
    getStats()
  } catch (error) {
    console.error('解锁失败:', error)
  }
}

// 导出日志
const handleExport = () => {
  ElMessage.success('导出成功')
}

// 分页
const handleSizeChange = (val: number) => {
  queryParams.page_size = val
  getLoginLogs()
}

const handleCurrentChange = (val: number) => {
  queryParams.page = val
  getLoginLogs()
}

// 获取类型标签
const getTypeLabel = (type: string) => {
  const map: Record<string, string> = {
    LOGIN: '登录',
    LOGOUT: '登出'
  }
  return map[type] || type
}

// 获取状态类型
const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    SUCCESS: 'success',
    FAILED: 'danger'
  }
  return map[status] || 'info'
}

// 获取状态标签
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    SUCCESS: '成功',
    FAILED: '失败'
  }
  return map[status] || status
}

// 获取告警类型
const getAlertType = (type: string) => {
  const map: Record<string, string> = {
    BRUTE_FORCE: 'danger',
    SUSPICIOUS_IP: 'warning',
    MULTIPLE_FAILURES: 'warning',
    UNUSUAL_LOCATION: 'info'
  }
  return map[type] || 'info'
}

// 获取告警标签
const getAlertLabel = (type: string) => {
  const map: Record<string, string> = {
    BRUTE_FORCE: '暴力破解',
    SUSPICIOUS_IP: '可疑IP',
    MULTIPLE_FAILURES: '多次失败',
    UNUSUAL_LOCATION: '异常地点'
  }
  return map[type] || type
}

// 获取告警状态类型
const getAlertStatusType = (status: string) => {
  const map: Record<string, string> = {
    PENDING: 'warning',
    PROCESSING: 'primary',
    RESOLVED: 'success',
    IGNORED: 'info'
  }
  return map[status] || 'info'
}

// 获取告警状态标签
const getAlertStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    PENDING: '待处理',
    PROCESSING: '处理中',
    RESOLVED: '已解决',
    IGNORED: '已忽略'
  }
  return map[status] || status
}

onMounted(() => {
  getStats()
  getLoginLogs()
  getLockedAccounts()
  getSecurityAlerts()
})
</script>

<style scoped>
.security-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.page-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.stats-row {
  margin-bottom: 20px;
}

.stat-card {
  height: 100%;
}

.stat-content {
  display: flex;
  align-items: center;
  gap: 15px;
}

.stat-icon {
  width: 50px;
  height: 50px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
}

.login-icon {
  background: #e6f7ff;
  color: #1890ff;
}

.fail-icon {
  background: #fff2e8;
  color: #fa541c;
}

.lock-icon {
  background: #fff7e6;
  color: #fa8c16;
}

.alert-icon {
  background: #f6ffed;
  color: #52c41a;
}

.stat-info {
  flex: 1;
}

.stat-label {
  font-size: 14px;
  color: #666;
  margin-bottom: 5px;
}

.stat-value {
  font-size: 24px;
  font-weight: 600;
  color: #333;
}

.table-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>
