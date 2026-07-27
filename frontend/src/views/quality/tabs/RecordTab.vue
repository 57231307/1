<!--
  RecordTab.vue - 检验记录 Tab（V2Table 迁移版）
  ----------------------------------------------------------------
  迁移说明（2026-06-16 P2-1 PR-5）：
  - 替换 el-table 为 V2Table 组件（基于 el-table-v2 的虚拟滚动通用组件）
  - 使用 useTableApi composable 接管分页/loading/重试
  - 保留原交互：page-header / 8 列表 / 结果 el-tag / 查看按钮 /
                    inject('qualityActions') openRecordDialog / openCreate /
                    handleExport (Batch 475d：改用后端 xlsx 导出) / handlePrint (新窗口) /
                    defineExpose({ fetchRecords }) / logger
  - 路径：/production/quality-inspection/records
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <div class="record-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('quality.recordTab.pageTitle') }}</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openCreate">
          <el-icon><Plus /></el-icon>
          {{ t('quality.recordTab.createButton') }}
        </el-button>
        <el-button v-permission="'quality.record.print'" @click="handlePrint">
          <el-icon><Printer /></el-icon>
          {{ t('quality.recordTab.printButton') }}
        </el-button>
        <el-button v-permission="'quality.record.export'" @click="handleExport">
          <el-icon><Download /></el-icon>
          {{ t('quality.recordTab.exportButton') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <V2Table
        :columns="columns"
        :data="data"
        :loading="loading"
        :page="page"
        :page-size="pageSize"
        :total="total"
        :height="600"
        @page-change="handlePageChange"
        @size-change="handleSizeChange"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
/**
 * 检验记录 Tab（V2Table 迁移版）
 * - V2Table：基于 el-table-v2 的虚拟滚动通用组件
 * - useTableApi：通用数据 composable（分页/loading/重试）
 * 保留原交互：page-header / 8 列表 / 结果 el-tag / 查看按钮 /
 *           inject('qualityActions') openRecordDialog / openCreate /
 *           handleExport (Batch 475d：改用后端 xlsx 导出) / handlePrint (新窗口) /
 *           defineExpose({ fetchRecords }) / logger
 */
import { h, onMounted, inject } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElTag, ElButton } from 'element-plus'
import { Plus, Download, Printer } from '@element-plus/icons-vue'
import { useTableApi } from '@/composables/useTableApi'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'
import { type QualityRecord } from '@/api/quality'
import { logger } from '@/utils/logger'
import { escapeHtml } from '@/utils/print'
// V15 P0-S12 修复（Batch 475d）：导出改用后端带水印 xlsx 接口
// 后端 GET /production/quality-inspection/records/export 已就绪（含异步审计日志 + 水印）
import { exportFromBackend } from '@/utils/export'

const { t } = useI18n({ useScope: 'global' })

// 父组件注入：openRecordDialog(row | null)
const actions = inject<{
  openRecordDialog: (row: QualityRecord | null) => void
}>('qualityActions')

// 检验记录列表（由 useTableApi 接管分页/loading/重试）
const { data, loading, page, pageSize, total, refresh } = useTableApi<QualityRecord>(
  '/production/quality-inspection/records'
)

// 结果标签映射函数（用于表格 el-tag 与导出/打印）
const getResultLabel = (result: string): string => {
  const map: Record<string, string> = {
    pass: t('quality.recordTab.resultPass'),
    fail: t('quality.recordTab.resultFail'),
    pending: t('quality.recordTab.resultPending'),
  }
  return map[result] || result
}

// 结果颜色映射
const getResultType = (result: string): 'success' | 'danger' | 'warning' => {
  if (result === 'pass') return 'success'
  if (result === 'fail') return 'danger'
  return 'warning'
}

/**
 * 列定义
 * - 结果列：使用 el-tag 三色映射（pass→success, fail→danger, 其他→warning）
 * - 操作列：查看按钮（fixed right）
 */
const columns: ColumnDef<QualityRecord>[] = [
  { key: 'record_no', title: t('quality.recordTab.colRecordNo'), width: 140, fixed: 'left' },
  { key: 'inspection_type', title: t('quality.recordTab.colInspectionType'), width: 120 },
  { key: 'product_name', title: t('quality.recordTab.colProduct'), width: 150 },
  { key: 'batch_no', title: t('quality.recordTab.colBatchNo'), width: 140 },
  { key: 'inspection_date', title: t('quality.recordTab.colInspectionDate'), width: 120 },
  { key: 'inspector', title: t('quality.recordTab.colInspector'), width: 100 },
  {
    key: 'result',
    title: t('quality.recordTab.colResult'),
    width: 100,
    align: 'center',
    renderCell: (row: QualityRecord) => {
      const type = getResultType(row.result)
      const text = getResultLabel(row.result)
      return h(ElTag, { type, size: 'small' }, () => text)
    },
  },
  {
    key: '__actions__',
    title: t('quality.recordTab.colActions'),
    width: 120,
    fixed: 'right',
    renderCell: (row: QualityRecord) =>
      h(
        ElButton,
        { type: 'primary', link: true, size: 'small', onClick: () => handleView(row) },
        () => t('quality.recordTab.buttonView')
      ),
  },
]

// 分页变化
const handlePageChange = (newPage: number) => {
  page.value = newPage
}

const handleSizeChange = (newSize: number) => {
  pageSize.value = newSize
}

// 打开新建对话框
const openCreate = () => {
  actions?.openRecordDialog(null)
}

// 查看检验记录（v11 批次 159 P1-1 修复：接入 openRecordDialog 显示详情，替代占位 ElMessage.info）
const handleView = (row: QualityRecord) => {
  actions?.openRecordDialog(row)
}

// 导出 Excel（V15 P0-S12 修复 Batch 475d）
// 规则 3：导出统一使用 xlsx 格式（禁止 CSV 作为最终交付格式）
// 改为调用后端 GET /production/quality-inspection/records/export，后端注入水印 + 异步审计日志
// 当前页面无筛选条件（前端未暴露 queryParams），导出全量数据
const handleExport = async () => {
  await exportFromBackend(
    '/production/quality-inspection/records/export',
    {},
    'quality_inspection_records_export'
  )
  logger.info(t('quality.recordTab.messageExported'))
}

// 构造打印表格行 HTML
const buildPrintRows = (): string => {
  return data.value
    .map(
      item => `
    <tr>
      <td>${escapeHtml(item.record_no)}</td><td>${escapeHtml(item.inspection_type)}</td>
      <td>${escapeHtml(item.product_name)}</td><td>${escapeHtml(item.batch_no)}</td>
      <td>${escapeHtml(item.inspection_date)}</td><td>${escapeHtml(item.inspector)}</td>
      <td>${escapeHtml(getResultLabel(item.result) || item.result)}</td>
    </tr>
  `
    )
    .join('')
}

// 打印
const handlePrint = () => {
  const printWindow = window.open('', '_blank')
  if (!printWindow) {
    ElMessage.error(t('quality.recordTab.messageCannotOpenPrintWindow'))
    return
  }
  const rows = buildPrintRows()
  const printDate = new Date().toISOString().split('T')[0]
  const totalCount = data.value.length
  printWindow.document
    .write(`<html><head><meta charset="utf-8"><title>${t('quality.recordTab.print.title')}</title>
    <style>@media print{@page{size:landscape;}}body{font-family:"Microsoft YaHei",sans-serif;font-size:12px;}h1{text-align:center;}table{width:100%;border-collapse:collapse;margin-top:12px;}th,td{border:1px solid #333;padding:6px 8px;}th{background:#f5f5f5;}.meta{text-align:center;color:#666;font-size:11px;}</style></head><body>
    <h1>${t('quality.recordTab.print.headerTitle')}</h1><div class="meta">${t('quality.recordTab.print.dateLabel')}: ${printDate} | ${t('quality.recordTab.print.totalLabel')} ${totalCount} ${t('quality.recordTab.print.totalUnit')}</div>
    <table><thead><tr><th>${t('quality.recordTab.print.colRecordNo')}</th><th>${t('quality.recordTab.print.colInspectionType')}</th><th>${t('quality.recordTab.print.colProduct')}</th><th>${t('quality.recordTab.print.colBatchNo')}</th><th>${t('quality.recordTab.print.colInspectionDate')}</th><th>${t('quality.recordTab.print.colInspector')}</th><th>${t('quality.recordTab.print.colResult')}</th></tr></thead><tbody>${rows}</tbody></table></body></html>`)
  printWindow.document.close()
  printWindow.onload = () => printWindow.print()
  logger.info(t('quality.recordTab.messagePrintGenerated'))
}

// 组件挂载时获取数据
onMounted(() => {
  refresh()
})

// 暴露给父组件调用（兼容外部刷新接口）
defineExpose({ fetchRecords: refresh })
</script>

<style scoped>
.record-tab {
  padding: 0;
}
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}
.page-title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}
.header-actions {
  display: flex;
  gap: 8px;
}
</style>
