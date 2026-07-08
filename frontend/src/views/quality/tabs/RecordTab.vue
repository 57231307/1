<!--
  RecordTab.vue - 检验记录 Tab（V2Table 迁移版）
  ----------------------------------------------------------------
  迁移说明（2026-06-16 P2-1 PR-5）：
  - 替换 el-table 为 V2Table 组件（基于 el-table-v2 的虚拟滚动通用组件）
  - 使用 useTableApi composable 接管分页/loading/重试
  - 保留原交互：page-header / 8 列表 / 结果 el-tag / 查看按钮 /
                    inject('qualityActions') openRecordDialog / openCreate /
                    handleExport (CSV) / handlePrint (新窗口) /
                    defineExpose({ fetchRecords }) / logger
  - 路径：/production/quality-inspection/records
-->
<template>
  <div class="record-tab">
    <div class="page-header">
      <h2 class="page-title">质量检验记录</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openCreate">
          <el-icon><Plus /></el-icon>
          新建检验
        </el-button>
        <el-button @click="handlePrint">
          <el-icon><Printer /></el-icon>
          打印
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
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
 *           handleExport (CSV) / handlePrint (新窗口) /
 *           defineExpose({ fetchRecords }) / logger
 */
import { h, onMounted, inject } from 'vue'
import { ElMessage, ElTag, ElButton } from 'element-plus'
import { Plus, Download, Printer } from '@element-plus/icons-vue'
import { useTableApi } from '@/composables/useTableApi'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'
import { type QualityRecord } from '@/api/quality'
import { logger } from '@/utils/logger'
import { escapeHtml } from '@/utils/print'
import { exportToExcel } from '@/utils/export'

// 父组件注入：openRecordDialog(row | null)
const actions = inject<{
  openRecordDialog: (row: QualityRecord | null) => void
}>('qualityActions')

// 检验记录列表（由 useTableApi 接管分页/loading/重试）
const { data, loading, page, pageSize, total, refresh } =
  useTableApi<QualityRecord>('/production/quality-inspection/records')

// 结果映射表（用于导出/打印）
const resultMap: Record<string, string> = { pass: '合格', fail: '不合格', pending: '待检' }

/**
 * 列定义
 * - 结果列：使用 el-tag 三色映射（pass→success, fail→danger, 其他→warning）
 * - 操作列：查看按钮（fixed right）
 */
const columns: ColumnDef<QualityRecord>[] = [
  { key: 'record_no', title: '记录编号', width: 140, fixed: 'left' },
  { key: 'inspection_type', title: '检验类型', width: 120 },
  { key: 'product_name', title: '产品', width: 150 },
  { key: 'batch_no', title: '批次号', width: 140 },
  { key: 'inspection_date', title: '检验日期', width: 120 },
  { key: 'inspector', title: '检验员', width: 100 },
  {
    key: 'result',
    title: '检验结果',
    width: 100,
    align: 'center',
    renderCell: (row: QualityRecord) => {
      const type = row.result === 'pass' ? 'success' : row.result === 'fail' ? 'danger' : 'warning'
      const text = row.result === 'pass' ? '合格' : row.result === 'fail' ? '不合格' : '待检'
      return h(ElTag, { type, size: 'small' }, () => text)
    },
  },
  {
    key: '__actions__',
    title: '操作',
    width: 120,
    fixed: 'right',
    renderCell: (row: QualityRecord) =>
      h(
        ElButton,
        { type: 'primary', link: true, size: 'small', onClick: () => handleView(row) },
        () => '查看'
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

// 导出 Excel（规则 3：禁止 CSV 作为最终交付格式）
const handleExport = () => {
  exportToExcel({
    filename: '检验记录',
    format: 'excel',
    data: data.value.map((item): Record<string, unknown> => ({ ...item })),
    columns: [
      { key: 'record_no', title: '记录编号' },
      { key: 'inspection_type', title: '检验类型' },
      { key: 'product_name', title: '产品' },
      { key: 'batch_no', title: '批次号' },
      { key: 'inspection_date', title: '检验日期' },
      { key: 'inspector', title: '检验员' },
      {
        key: 'result',
        title: '结果',
        formatter: (value: unknown) =>
          resultMap[value as string] || String(value),
      },
    ],
  })
  logger.info('检验记录已导出')
}

// 打印
const handlePrint = () => {
  const printWindow = window.open('', '_blank')
  if (!printWindow) {
    ElMessage.error('无法打开打印窗口')
    return
  }
  const rows = data.value
    .map(
      item => `
    <tr>
      <td>${escapeHtml(item.record_no)}</td><td>${escapeHtml(item.inspection_type)}</td>
      <td>${escapeHtml(item.product_name)}</td><td>${escapeHtml(item.batch_no)}</td>
      <td>${escapeHtml(item.inspection_date)}</td><td>${escapeHtml(item.inspector)}</td>
      <td>${escapeHtml(resultMap[item.result] || item.result)}</td>
    </tr>
  `
    )
    .join('')
  printWindow.document.write(`<html><head><meta charset="utf-8"><title>检验记录</title>
    <style>@media print{@page{size:landscape;}}body{font-family:"Microsoft YaHei",sans-serif;font-size:12px;}h1{text-align:center;}table{width:100%;border-collapse:collapse;margin-top:12px;}th,td{border:1px solid #333;padding:6px 8px;}th{background:#f5f5f5;}.meta{text-align:center;color:#666;font-size:11px;}</style></head><body>
    <h1>质量检验记录</h1><div class="meta">打印日期: ${new Date().toISOString().split('T')[0]} | 共 ${data.value.length} 条</div>
    <table><thead><tr><th>记录编号</th><th>检验类型</th><th>产品</th><th>批次号</th><th>检验日期</th><th>检验员</th><th>结果</th></tr></thead><tbody>${rows}</tbody></table></body></html>`)
  printWindow.document.close()
  printWindow.onload = () => printWindow.print()
  logger.info('检验记录打印任务已生成')
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
