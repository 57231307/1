<!--
  BpmDefinitionVersionDialog.vue - BPM 流程定义版本管理对话框
  拆分自 bpm/definitions.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="`版本管理 - ${definition ? definition.process_name : ''}`"
    aria-label="流程版本对话框"
    width="700px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <div class="version-actions">
      <el-button type="primary" @click="emit('create-version')">
        <el-icon><Plus /></el-icon>
        创建新版本
      </el-button>
    </div>
    <el-table v-loading="loading" :data="data" border aria-label="流程版本列表">
      <el-table-column prop="version" label="版本号" width="100" align="center" />
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="row.status === 'active' ? 'success' : 'info'">
            {{ getVersionStatusText(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="change_log" label="变更说明" min-width="180" show-overflow-tooltip />
      <el-table-column prop="created_at" label="创建时间" width="160" />
      <el-table-column prop="created_by" label="创建人" width="100" />
      <el-table-column label="操作" width="120" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status !== 'active'"
            type="primary"
            link
            size="small"
            @click="emit('activate', row)"
            >激活</el-button
          >
        </template>
      </el-table-column>
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { Plus } from '@element-plus/icons-vue'
import { getVersionStatusText } from '../composables/bpmDfFmts'
import type { ProcessDefinition, ProcessVersion } from '@/api/bpm-enhanced'

/**
 * 版本管理对话框
 */
defineProps<{
  // 可见性
  visible: boolean
  // 当前流程定义
  definition: ProcessDefinition | null
  // 版本数据
  data: ProcessVersion[]
  // 加载状态
  loading: boolean
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  // 创建新版本
  'create-version': []
  // 激活版本
  activate: [version: ProcessVersion]
}>()
</script>

<style scoped>
.version-actions {
  margin-bottom: 12px;
}
</style>
