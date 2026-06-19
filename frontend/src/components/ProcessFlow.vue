<!--
  工艺流程图组件
  - 5 阶段节点展示
  - 状态颜色 + 操作按钮
-->
<template>
  <div class="process-flow">
    <el-empty v-if="!nodes || nodes.length === 0" description="暂无工艺节点" />
    <el-steps v-else :active="activeIndex" align-center finish-status="success">
      <el-step
        v-for="node in nodes"
        :key="node.id"
        :title="node.node_name"
        :description="getDescription(node)"
        :status="getStatus(node.status)"
      />
    </el-steps>

    <!-- 节点操作 -->
    <div v-if="nodes && nodes.length > 0" class="node-actions">
      <el-card v-for="node in nodes" :key="node.id" class="node-card" shadow="hover">
        <template #header>
          <div class="node-header">
            <span>{{ node.node_name }}</span>
            <el-tag :type="NODE_STATUS_COLORS[node.status] || 'info'" size="small">
              {{ NODE_STATUS[node.status] || node.status }}
            </el-tag>
          </div>
        </template>
        <div class="node-info">
          <div>计划开始：{{ node.planned_start_date || '未设置' }}</div>
          <div>实际开始：{{ node.actual_start_date || '未开始' }}</div>
          <div>实际结束：{{ node.actual_end_date || '进行中' }}</div>
          <div>操作人：{{ node.operator_id || '未分配' }}</div>
        </div>
        <div class="node-buttons">
          <el-button
            v-if="node.status === 'pending'"
            size="small"
            type="primary"
            @click="handleAction(node, 'start')"
          >
            开始
          </el-button>
          <el-button
            v-if="node.status === 'in_progress'"
            size="small"
            @click="handleAction(node, 'pause')"
          >
            暂停
          </el-button>
          <el-button
            v-if="node.status === 'in_progress'"
            size="small"
            type="success"
            @click="handleAction(node, 'complete')"
          >
            完成
          </el-button>
          <el-button
            v-if="node.status !== 'blocked' && node.status !== 'completed'"
            size="small"
            type="danger"
            @click="handleAction(node, 'block')"
          >
            阻塞
          </el-button>
        </div>
      </el-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  advanceProcessNode,
  NODE_STATUS,
  NODE_STATUS_COLORS,
} from '@/api/custom-order'

const props = defineProps<{
  nodes: any[]
  orderId?: number
}>()

const emit = defineEmits<{ (e: 'refresh'): void }>()

const activeIndex = computed(() => {
  if (!props.nodes) return 0
  // 找到第一个 in_progress 或最后一个未完成
  const idx = props.nodes.findIndex((n: any) => n.status === 'in_progress')
  if (idx >= 0) return idx
  const lastCompleted = props.nodes.map((n: any) => n.status).lastIndexOf('completed')
  return lastCompleted + 1
})

function getStatus(s: string): 'process' | 'finish' | 'error' | 'wait' {
  if (s === 'in_progress') return 'process'
  if (s === 'completed') return 'finish'
  if (s === 'blocked') return 'error'
  return 'wait'
}

function getDescription(node: any) {
  if (node.actual_end_date) return `完成于 ${new Date(node.actual_end_date).toLocaleDateString()}`
  if (node.actual_start_date) return `开始于 ${new Date(node.actual_start_date).toLocaleDateString()}`
  if (node.planned_start_date) return `计划 ${new Date(node.planned_start_date).toLocaleDateString()}`
  return '待开始'
}

async function handleAction(node: any, action: string) {
  try {
    if (action === 'block') {
      const { value: reason } = await ElMessageBox.prompt('请输入阻塞原因', '阻塞节点', {
        inputPattern: /\S+/,
        inputErrorMessage: '原因不能为空',
      })
      await advanceProcessNode(props.orderId || 0, node.id, {
        action,
        operator_id: 1,
        notes: reason,
      })
    } else {
      await advanceProcessNode(props.orderId || 0, node.id, {
        action,
        operator_id: 1,
      })
    }
    ElMessage.success('操作成功')
    emit('refresh')
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e?.message || '操作失败')
  }
}
</script>

<style scoped>
.process-flow {
  padding: 16px 0;
}
.node-actions {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 12px;
  margin-top: 16px;
}
.node-card {
  margin-bottom: 12px;
}
.node-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.node-info {
  font-size: 13px;
  line-height: 1.8;
  color: #606266;
  margin-bottom: 12px;
}
.node-buttons {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}
</style>
