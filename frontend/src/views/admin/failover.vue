<!--
  主备隔离监控页面
  - 状态卡片（数据库 / 缓存）
  - 切换历史列表
  - 手动切换按钮（仅管理员）
  - 实时刷新
-->

<template>
  <div class="failover-monitor">
    <el-card class="page-header">
      <template #header>
        <div class="header-row">
          <span class="title">主备隔离监控（P0-2）</span>
          <el-button :icon="Refresh" @click="loadData" :loading="loading">刷新</el-button>
        </div>
      </template>
      <p class="description">
        监控秉羲 ERP 核心功能（数据库 / 缓存）的主备状态，自动切换与回切由
        <code>FailoverCall</code> trait + 熔断器保障。
      </p>
    </el-card>

    <!-- 状态卡片 -->
    <el-row :gutter="20" class="status-row">
      <el-col v-for="status in statuses" :key="status.function_name" :span="12">
        <FailoverStatusCard
          :status="status"
          @switch="handleSwitch"
        />
      </el-col>
    </el-row>

    <!-- 健康检查 -->
    <el-card class="health-card">
      <template #header>
        <span class="title">健康检查</span>
      </template>
      <FailoverMetrics :health="health" />
    </el-card>

    <!-- 切换历史 -->
    <el-card class="event-card">
      <template #header>
        <span class="title">切换历史（最近 20 条）</span>
      </template>
      <FailoverEventList :events="events" />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh } from '@element-plus/icons-vue'
import {
  getFailoverStatus,
  triggerSwitch,
  getFailoverHealth,
  type FailoverStatusDto,
  type FailoverEventDto,
} from '@/api/failover'
import FailoverStatusCard from './components/FailoverStatusCard.vue'
import FailoverEventList from './components/FailoverEventList.vue'
import FailoverMetrics from './components/FailoverMetrics.vue'

const loading = ref(false)
const statuses = ref<FailoverStatusDto[]>([])
const events = ref<FailoverEventDto[]>([])
const health = ref<{ database: string; cache: string }>({ database: 'unknown', cache: 'unknown' })

let timer: ReturnType<typeof setInterval> | null = null

/** 加载数据 */
async function loadData() {
  loading.value = true
  try {
    const [statusRes, healthRes] = await Promise.all([
      getFailoverStatus(),
      getFailoverHealth().catch(() => ({ data: { database: 'error', cache: 'error' } })),
    ])
    statuses.value = statusRes.statuses || []
    events.value = statusRes.events || []
    health.value = healthRes.data || { database: 'unknown', cache: 'unknown' }
  } catch (err: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (err: any) 改为 unknown + 类型守卫
    ElMessage.error(`加载失败: ${err instanceof Error ? err.message : String(err)}`)
  } finally {
    loading.value = false
  }
}

/** 手动切换 */
async function handleSwitch(functionName: string) {
  try {
    await ElMessageBox.confirm(
      `确认要手动触发 ${functionName} 切换至备用？`,
      '切换确认',
      {
        confirmButtonText: '确认切换',
        cancelButtonText: '取消',
        type: 'warning',
      }
    )
    const res = await triggerSwitch(functionName)
    ElMessage.success(res.data || '切换成功')
    await loadData()
  } catch (err: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (err: any) 改为 unknown + 类型守卫
    if (err !== 'cancel') {
      ElMessage.error(`切换失败: ${err instanceof Error ? err.message : String(err)}`)
    }
  }
}

onMounted(() => {
  loadData()
  // 每 10 秒自动刷新
  timer = setInterval(loadData, 10000)
})

onUnmounted(() => {
  if (timer) {
    clearInterval(timer)
  }
})
</script>

<style scoped>
.failover-monitor {
  padding: 20px;
}

.page-header {
  margin-bottom: 20px;
}

.header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.title {
  font-size: 18px;
  font-weight: 600;
}

.description {
  color: #606266;
  line-height: 1.6;
}

.description code {
  background: #f5f7fa;
  padding: 2px 6px;
  border-radius: 3px;
  font-family: 'Courier New', monospace;
}

.status-row {
  margin-bottom: 20px;
}

.health-card,
.event-card {
  margin-bottom: 20px;
}
</style>
