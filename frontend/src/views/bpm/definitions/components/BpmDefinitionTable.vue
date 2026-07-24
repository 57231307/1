<!--
  BpmDefinitionTable.vue - BPM 流程定义列表
  拆分自 bpm/definitions.vue（P14 批 2 I-3 第 5 批）
  批次 282：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
-->
<template>
  <el-card class="table-card">
    <el-table v-loading="loading" :data="data" border stripe :aria-label="$t('bpm.definitions.table.ariaLabel')">
      <el-table-column prop="process_key" :label="$t('bpm.definitions.table.processKey')" min-width="140" />
      <el-table-column prop="process_name" :label="$t('bpm.definitions.table.processName')" min-width="180" />
      <el-table-column prop="category" :label="$t('bpm.definitions.table.category')" width="100">
        <template #default="{ row }">
          {{ getCategoryText(row.category, t) }}
        </template>
      </el-table-column>
      <el-table-column prop="version" :label="$t('bpm.definitions.table.version')" width="100" align="center" />
      <el-table-column prop="status" :label="$t('bpm.definitions.table.status')" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">
            {{ getStatusText(row.status, t) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="description" :label="$t('bpm.definitions.table.description')" min-width="150" show-overflow-tooltip />
      <el-table-column prop="created_at" :label="$t('bpm.definitions.table.createdAt')" min-width="160" />
      <el-table-column :label="$t('bpm.definitions.table.operation')" width="320" fixed="right">
        <template #default="{ row }">
          <el-button v-permission="'bpm_definition:update'" size="small" @click="emit('edit', row)">{{ $t('bpm.definitions.table.edit') }}</el-button>
          <el-button size="small" type="primary" @click="emit('versions', row)">{{ $t('bpm.definitions.table.versions') }}</el-button>
          <el-button size="small" type="success" @click="emit('save-as-template', row)">
            {{ $t('bpm.definitions.table.saveAsTemplate') }}
          </el-button>
          <el-button v-permission="'bpm_definition:delete'" size="small" type="danger" @click="emit('delete', row)">{{ $t('bpm.definitions.table.delete') }}</el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-pagination
      :current-page="page"
      :page-size="pageSize"
      :total="total"
      :page-sizes="[10, 20, 50, 100]"
      layout="total, sizes, prev, pager, next, jumper"
      :aria-label="$t('bpm.definitions.table.paginationAriaLabel')"
      @update:current-page="(v: number) => emit('update:page', v)"
      @update:page-size="(v: number) => emit('update:page-size', v)"
    />
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { getCategoryText, getStatusType, getStatusText } from '../composables/bpmDfFmts'
import type { ProcessDefinition } from '@/api/bpm-enhanced'

const { t } = useI18n({ useScope: 'global' })

/** 列表组件（批次 282：page/pageSize props + v-model 绑定分页） */
defineProps<{
  // 列表数据
  data: ProcessDefinition[]
  // 总数
  total: number
  // 加载状态
  loading: boolean
  // 分页
  page: number
  pageSize: number
}>()

const emit = defineEmits<{
  edit: [row: ProcessDefinition]
  versions: [row: ProcessDefinition]
  'save-as-template': [row: ProcessDefinition]
  delete: [row: ProcessDefinition]
  'update:page': [v: number]
  'update:page-size': [v: number]
}>()
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
.el-pagination {
  margin-top: 20px;
  justify-content: flex-end;
}
</style>
