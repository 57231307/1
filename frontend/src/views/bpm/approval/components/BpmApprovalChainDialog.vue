<!--
  BpmApprovalChainDialog.vue - BPM 审批链对话框
  拆分自 bpm/approval.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :aria-label="$t('bpm.approval.chainDialog.ariaLabel')"
    :model-value="visible"
    :title="$t('bpm.approval.chainDialog.title')"
    width="700px"
    destroy-on-close
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <div v-if="chain.length > 0" class="approval-chain">
      <div v-for="(node, index) in chain" :key="index" class="chain-item">
        <div class="chain-node" :class="getNodeStatusClassFmt(node.status)">
          <div class="node-order">{{ node.order }}</div>
          <div class="node-content">
            <div class="node-name">{{ node.node_name }}</div>
            <div class="node-type">{{ getNodeTypeNameFmt(node.node_type) }}</div>
            <div v-if="node.approver_name" class="node-approver">
              {{ $t('bpm.approval.chainDialog.approver') }}：{{ node.approver_name }}
            </div>
            <div v-if="node.approved_at" class="node-time">{{ node.approved_at }}</div>
            <div v-if="node.comment" class="node-comment">{{ $t('bpm.approval.chainDialog.comment') }}：{{ node.comment }}</div>
            <div v-if="node.duration" class="node-duration">{{ $t('bpm.approval.chainDialog.durationText', { minutes: node.duration }) }}</div>
          </div>
        </div>
        <div v-if="index < chain.length - 1" class="chain-arrow">
          <el-icon><ArrowDown /></el-icon>
        </div>
      </div>
    </div>
    <el-empty v-else :description="$t('bpm.approval.chainDialog.empty')" />
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { ArrowDown } from '@element-plus/icons-vue'
import type { ApprovalChainNode } from '@/api/bpm-enhanced'
import { getNodeStatusClass } from '../composables/bpmApFmts'

const { t } = useI18n({ useScope: 'global' })

/**
 * 审批链对话框组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean
  // 审批链节点列表
  chain: ApprovalChainNode[]
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
}>()

// 透传格式化函数
const getNodeStatusClassFmt = getNodeStatusClass

// 节点类型名称（响应式求值，随语言切换更新）
const getNodeTypeNameFmt = (type: string) => {
  const map: Record<string, string> = {
    start: t('bpm.nodeType.start'),
    end: t('bpm.nodeType.end'),
    approval: t('bpm.nodeType.approval'),
    condition: t('bpm.nodeType.condition'),
    notify: t('bpm.nodeType.notify'),
  }
  return map[type] || type
}
</script>

<style scoped>
.approval-chain {
  max-height: 500px;
  overflow-y: auto;
  padding: 8px 0;
}
.chain-item {
  display: flex;
  flex-direction: column;
  align-items: center;
}
.chain-node {
  display: flex;
  gap: 16px;
  padding: 16px;
  border-radius: 8px;
  border: 2px solid #e4e7ed;
  background: #fff;
  width: 100%;
}
.chain-node.status-approved {
  border-color: #67c23a;
  background: #f0f9eb;
}
.chain-node.status-rejected {
  border-color: #f56c6c;
  background: #fef0f0;
}
.chain-node.status-skipped {
  border-color: #909399;
  background: #f4f4f5;
  opacity: 0.7;
}
.node-order {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: #409eff;
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  flex-shrink: 0;
}
.chain-node.status-approved .node-order {
  background: #67c23a;
}
.chain-node.status-rejected .node-order {
  background: #f56c6c;
}
.node-content {
  flex: 1;
}
.node-name {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}
.node-type {
  font-size: 12px;
  color: #909399;
  margin-top: 4px;
}
.node-approver {
  font-size: 14px;
  color: #606266;
  margin-top: 8px;
}
.node-time {
  font-size: 12px;
  color: #909399;
  margin-top: 4px;
}
.node-comment {
  font-size: 13px;
  color: #409eff;
  margin-top: 8px;
  padding: 8px;
  background: #ecf5ff;
  border-radius: 4px;
}
.node-duration {
  font-size: 12px;
  color: #e6a23c;
  margin-top: 4px;
}
.chain-arrow {
  color: #c0c4cc;
  padding: 4px 0;
}
</style>
