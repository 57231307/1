<!--
  定制订单工艺跟踪大屏
  - 5 阶段甘特图
  - 当前节点高亮
  - 时间线 + 操作日志
-->
<template>
  <div class="custom-order-tracking" v-loading="loading">
    <el-card v-if="timeline">
      <template #header>
        <div class="card-header">
          <div>
            <span class="title">工艺跟踪 - {{ timeline.order_no }}</span>
            <el-tag :type="STATUS_COLORS[timeline.current_status] || 'info'" style="margin-left: 12px">
              {{ STATUS_LABELS[timeline.current_status] || timeline.current_status }}
            </el-tag>
          </div>
          <el-button @click="$router.push(`/custom-orders/${orderId}`)">详情</el-button>
        </div>
      </template>

      <!-- 5 阶段甘特图 -->
      <div class="gantt">
        <div
          v-for="node in timeline.nodes || []"
          :key="node.id"
          class="gantt-row"
        >
          <div class="gantt-label">
            <div class="node-name">{{ node.node_name }}</div>
            <div class="node-type">{{ node.node_type }}</div>
          </div>
          <div class="gantt-bar-wrapper">
            <div
              class="gantt-bar"
              :class="`status-${node.status}`"
              :style="{
                width: getBarWidth(node),
                background: getBarColor(node.status),
              }"
            >
              <span class="bar-text">{{ getStatusText(node.status) }}</span>
            </div>
          </div>
          <div class="gantt-time">
            <div v-if="node.actual_start_date">
              实际：{{ formatDate(node.actual_start_date) }} → {{ formatDate(node.actual_end_date) || '进行中' }}
            </div>
            <div v-else-if="node.planned_start_date">
              计划：{{ formatDate(node.planned_start_date) }} → {{ formatDate(node.planned_end_date) || '?' }}
            </div>
            <div v-else>未开始</div>
          </div>
        </div>
      </div>

      <!-- 节点日志时间线 -->
      <el-divider>操作日志</el-divider>
      <el-timeline>
        <el-timeline-item
          v-for="log in allLogs"
          :key="log.id"
          :timestamp="formatDate(log.log_time)"
          :type="getLogColor(log.action)"
        >
          <el-card>
            <h4>{{ log.action }}</h4>
            <p v-if="log.log_content">{{ log.log_content }}</p>
            <p v-if="log.before_status && log.after_status">
              状态：{{ log.before_status }} → {{ log.after_status }}
            </p>
            <p style="font-size: 12px; color: #909399">
              操作人：{{ log.operator_id || '-' }}
            </p>
          </el-card>
        </el-timeline-item>
        <el-empty v-if="allLogs.length === 0" description="暂无操作日志" />
      </el-timeline>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import { getTimeline, CUSTOM_ORDER_STATUS as STATUS_LABELS, CUSTOM_ORDER_STATUS_COLORS as STATUS_COLORS } from '@/api/custom-order'

const route = useRoute()
const loading = ref(false)
const timeline = ref<any>(null)
const orderId = computed(() => Number(route.params.id))

const allLogs = computed(() => {
  if (!timeline.value?.nodes) return []
  return timeline.value.nodes
    .flatMap((n: any) => (n.logs || []).map((l: any) => ({ ...l, node_name: n.node_name })))
    .sort((a: any, b: any) => new Date(b.log_time).getTime() - new Date(a.log_time).getTime())
})

function formatDate(d: any) {
  if (!d) return ''
  return new Date(d).toLocaleString('zh-CN')
}

function getStatusText(s: string) {
  const map: Record<string, string> = {
    pending: '待开始',
    in_progress: '进行中',
    completed: '已完成',
    blocked: '阻塞',
  }
  return map[s] || s
}

function getBarColor(s: string) {
  const map: Record<string, string> = {
    pending: '#909399',
    in_progress: '#409eff',
    completed: '#67c23a',
    blocked: '#f56c6c',
  }
  return map[s] || '#909399'
}

function getLogColor(action: string) {
  if (action === 'complete') return 'success'
  if (action === 'block') return 'danger'
  if (action === 'start' || action === 'resume') return 'primary'
  return 'info'
}

function getBarWidth(node: any) {
  if (node.status === 'completed') return '100%'
  if (node.status === 'in_progress') return '60%'
  if (node.status === 'blocked') return '40%'
  return '0%'
}

async function loadData() {
  const id = orderId.value
  if (!id) return
  loading.value = true
  try {
    const res: any = await getTimeline(id)
    timeline.value = res.data || res
  } catch (e) {
    console.error('加载时间线失败', e)
    ElMessage.error('加载时间线失败')
  } finally {
    loading.value = false
  }
}

watch(() => route.params.id, loadData)
onMounted(loadData)
</script>

<style scoped>
.custom-order-tracking {
  padding: 16px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.title {
  font-size: 18px;
  font-weight: 600;
}
.gantt {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin: 16px 0;
}
.gantt-row {
  display: grid;
  grid-template-columns: 200px 1fr 280px;
  gap: 12px;
  align-items: center;
}
.gantt-label .node-name {
  font-weight: 600;
}
.gantt-label .node-type {
  font-size: 12px;
  color: #909399;
}
.gantt-bar-wrapper {
  background: #f5f7fa;
  border-radius: 4px;
  height: 32px;
  position: relative;
}
.gantt-bar {
  height: 32px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  padding: 0 12px;
  color: white;
  font-size: 12px;
  transition: width 0.3s;
}
.gantt-time {
  font-size: 12px;
  color: #606266;
}
</style>
