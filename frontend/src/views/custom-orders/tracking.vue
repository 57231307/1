<!--
  定制订单工艺跟踪大屏
  - 5 阶段甘特图
  - 当前节点高亮
  - 时间线 + 操作日志
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <div v-loading="loading" class="custom-order-tracking">
    <el-card v-if="timeline">
      <template #header>
        <div class="card-header">
          <div>
            <span class="title"
              >{{ t('customOrders.tracking.title') }} - {{ timeline.order_no }}</span
            >
            <el-tag
              :type="STATUS_COLORS[timeline.current_status] || 'info'"
              style="margin-left: 12px"
            >
              {{ getStatusLabel(timeline.current_status) }}
            </el-tag>
          </div>
          <el-button @click="$router.push(`/custom-orders/${orderId}`)">{{
            t('customOrders.tracking.buttonDetail')
          }}</el-button>
        </div>
      </template>

      <!-- 5 阶段甘特图 -->
      <div class="gantt">
        <div v-for="node in timeline.nodes || []" :key="node.id" class="gantt-row">
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
              <span class="bar-text">{{ getNodeStatusText(node.status) }}</span>
            </div>
          </div>
          <div class="gantt-time">
            <div v-if="node.actual_start_date">
              {{ t('customOrders.tracking.labelActual') }}：{{
                formatDate(node.actual_start_date)
              }}
              → {{ formatDate(node.actual_end_date) || t('customOrders.tracking.labelInProgress') }}
            </div>
            <div v-else-if="node.planned_start_date">
              {{ t('customOrders.tracking.labelPlan') }}：{{
                formatDate(node.planned_start_date)
              }}
              → {{ formatDate(node.planned_end_date) || t('customOrders.tracking.labelUnknown') }}
            </div>
            <div v-else>{{ t('customOrders.tracking.labelNotStarted') }}</div>
          </div>
        </div>
      </div>

      <!-- 节点日志时间线 -->
      <el-divider>{{ t('customOrders.tracking.dividerOperationLog') }}</el-divider>
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
              {{
                t('customOrders.tracking.labelStatusTransition', {
                  before: log.before_status,
                  after: log.after_status,
                })
              }}
            </p>
            <p style="font-size: 12px; color: #909399">
              {{ t('customOrders.tracking.labelOperator', { operator: log.operator_id || '-' }) }}
            </p>
          </el-card>
        </el-timeline-item>
        <el-empty v-if="allLogs.length === 0" :description="t('customOrders.tracking.emptyLogs')" />
      </el-timeline>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { getTimeline, CUSTOM_ORDER_STATUS_COLORS as STATUS_COLORS } from '@/api/custom-order'
import type {
  TimelineProcessNode,
  NodeLog,
  CustomOrderProcessNode,
  OrderTimeline,
} from '@/api/custom-order'
import logger from '@/utils/logger'

const route = useRoute()
const { t } = useI18n({ useScope: 'global' })
const loading = ref(false)
// 时间线响应数据（getTimeline 返回的 res.data 结构）
const timeline = ref<OrderTimeline | null>(null)
const orderId = computed(() => Number(route.params.id))

const allLogs = computed(() => {
  // 提取到局部变量以便 TypeScript 正确进行 null 收窄
  const tl = timeline.value
  if (!tl?.nodes) return []
  return tl.nodes
    .flatMap((n: TimelineProcessNode) =>
      (n.logs || []).map((l: NodeLog) => ({ ...l, node_name: n.node_name }))
    )
    .sort(
      (a: NodeLog, b: NodeLog) => new Date(b.log_time).getTime() - new Date(a.log_time).getTime()
    )
})

function formatDate(d: string | Date | null | undefined) {
  if (!d) return ''
  return new Date(d).toLocaleString('zh-CN')
}

// 订单状态标签映射函数（i18n）
const getStatusLabel = (status: string): string => {
  const map: Record<string, string> = {
    draft: t('customOrders.status.draft'),
    yarn_purchasing: t('customOrders.status.yarnPurchasing'),
    dyeing: t('customOrders.status.dyeing'),
    finishing: t('customOrders.status.finishing'),
    delivery: t('customOrders.status.delivery'),
    after_sales: t('customOrders.status.afterSales'),
    completed: t('customOrders.status.completed'),
    cancelled: t('customOrders.status.cancelled'),
  }
  return map[status] || status
}

// 节点状态标签映射函数（i18n）
const getNodeStatusText = (s: string): string => {
  const map: Record<string, string> = {
    pending: t('customOrders.tracking.nodeStatusPending'),
    in_progress: t('customOrders.tracking.nodeStatusInProgress'),
    completed: t('customOrders.tracking.nodeStatusCompleted'),
    blocked: t('customOrders.tracking.nodeStatusBlocked'),
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

function getBarWidth(node: CustomOrderProcessNode) {
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
    const res = await getTimeline(id)
    timeline.value = res.data || null
  } catch (e) {
    logger.error(t('customOrders.tracking.messageLoadFailed'), e)
    ElMessage.error(t('customOrders.tracking.messageLoadFailed'))
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
