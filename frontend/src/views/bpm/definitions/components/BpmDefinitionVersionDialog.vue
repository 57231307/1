<!--
  BpmDefinitionVersionDialog.vue - BPM 流程定义版本管理对话框
  拆分自 bpm/definitions.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('bpm.definitions.versionDialog.title', { name: definition ? definition.process_name : '' })"
    :aria-label="$t('bpm.definitions.versionDialog.ariaLabel')"
    width="700px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <div class="version-actions">
      <el-button type="primary" @click="emit('create-version')">
        <el-icon><Plus /></el-icon>
        {{ $t('bpm.definitions.versionDialog.createVersion') }}
      </el-button>
    </div>
    <el-table v-loading="loading" :data="data" border :aria-label="$t('bpm.definitions.versionDialog.tableAriaLabel')">
      <el-table-column prop="version" :label="$t('bpm.definitions.versionDialog.version')" width="100" align="center" />
      <el-table-column prop="status" :label="$t('bpm.definitions.versionDialog.status')" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="row.status === 'active' ? 'success' : 'info'">
            {{ getVersionStatusText(row.status, t) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="change_log" :label="$t('bpm.definitions.versionDialog.changeLog')" min-width="180" show-overflow-tooltip />
      <el-table-column prop="created_at" :label="$t('bpm.definitions.versionDialog.createdAt')" width="160" />
      <el-table-column prop="created_by" :label="$t('bpm.definitions.versionDialog.createdBy')" width="100" />
      <el-table-column :label="$t('bpm.definitions.versionDialog.operation')" width="120" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status !== 'active'"
            type="primary"
            link
            size="small"
            @click="emit('activate', row)"
            >{{ $t('bpm.definitions.versionDialog.activate') }}</el-button
          >
        </template>
      </el-table-column>
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Plus } from '@element-plus/icons-vue'
import { getVersionStatusText } from '../composables/bpmDfFmts'
import type { ProcessDefinition, ProcessVersion } from '@/api/bpm-enhanced'

const { t } = useI18n({ useScope: 'global' })

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
