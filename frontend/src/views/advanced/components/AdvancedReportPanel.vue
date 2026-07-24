<script setup lang="ts">
/**
 * AdvancedReportPanel - 报表引擎 tab 视图组件（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 第 2 个 tab）
 * 包含：报表模板列表、执行报表、导出报表、报表结果展示弹窗
 * 数据与函数全部由父组件通过 props 传入
 */
// v11 批次 180 P2-1 修复：从 useRpt 导入具体类型替代 any
import type {
  ReportTemplate,
  ReportColumn,
} from '../composables/useRpt'

interface Props {
  reportTemplates: ReportTemplate[]
  reportLoading: boolean
  reportResultVisible: boolean
  reportData: unknown[]
  reportColumns: ReportColumn[]
  executeReport: (row: ReportTemplate) => Promise<void>
  exportReport: (row: ReportTemplate, format: string) => Promise<void>
}

defineProps<Props>()

/**
 * 关闭报表结果弹窗（通过 emit 通知父组件）
 */
const emit = defineEmits<{
  (e: 'update:report-result-visible', value: boolean): void
}>()
const closeReportResult = () => emit('update:report-result-visible', false)
</script>

<template>
  <div class="page-header">
    <h2 class="page-title">{{ $t('advancedModule.report.title') }}</h2>
  </div>

  <el-card shadow="hover">
    <el-table v-loading="reportLoading" :data="reportTemplates" stripe aria-label="报表模板列表">
      <el-table-column prop="template_name" :label="$t('advancedModule.report.colName')" width="180" />
      <el-table-column prop="template_code" :label="$t('advancedModule.report.colCode')" width="120" />
      <el-table-column prop="category" :label="$t('advancedModule.report.colCategory')" width="120" />
      <el-table-column prop="description" :label="$t('advancedModule.report.colDesc')" min-width="200" />
      <el-table-column prop="created_at" :label="$t('advancedModule.report.colCreatedAt')" width="160" />
      <el-table-column :label="$t('advancedModule.report.colAction')" width="200" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="executeReport(row as ReportTemplate)"
            >{{ $t('advancedModule.report.execute') }}</el-button
          >
          <el-button type="success" link size="small" @click="exportReport(row, 'excel')"
            >{{ $t('advancedModule.report.exportExcel') }}</el-button
          >
          <el-button type="warning" link size="small" @click="exportReport(row, 'pdf')"
            >{{ $t('advancedModule.report.exportPdf') }}</el-button
          >
        </template>
      </el-table-column>
    </el-table>
  </el-card>

  <el-dialog
    :model-value="reportResultVisible"
    :title="$t('advancedModule.report.resultTitle')"
    width="80%"
    aria-label="报表结果对话框"
    @update:model-value="(v: boolean) => emit('update:report-result-visible', v)"
  >
    <div class="report-result">
      <el-empty v-if="!reportData" :description="$t('advancedModule.report.empty')" />
      <el-table v-else :data="reportData" border stripe aria-label="报表结果数据表">
        <el-table-column
          v-for="col in reportColumns"
          :key="col.key"
          :prop="col.key"
          :label="col.label"
        />
      </el-table>
    </div>
    <template #footer>
      <el-button @click="closeReportResult">{{ $t('advancedModule.report.close') }}</el-button>
    </template>
  </el-dialog>
</template>
