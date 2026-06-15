<!--
  RecordTab.vue - 检验记录 Tab
  来源：原 quality/index.vue 中 检验记录 tab 内容
  拆分日期：2026-06-15 B3-4
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
      <el-table v-loading="loading" :data="records" stripe>
        <el-table-column prop="record_no" label="记录编号" width="140" />
        <el-table-column prop="inspection_type" label="检验类型" width="120" />
        <el-table-column prop="product_name" label="产品" width="150" />
        <el-table-column prop="batch_no" label="批次号" width="140" />
        <el-table-column prop="inspection_date" label="检验日期" width="120" />
        <el-table-column prop="inspector" label="检验员" width="100" />
        <el-table-column prop="result" label="检验结果" width="100" align="center">
          <template #default="{ row }">
            <el-tag
              :type="
                row.result === 'pass' ? 'success' : row.result === 'fail' ? 'danger' : 'warning'
              "
              size="small"
            >
              {{ row.result === 'pass' ? '合格' : row.result === 'fail' ? '不合格' : '待检' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="120" fixed="right">
          <template #default>
            <el-button type="primary" link size="small" @click="handleView">查看</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, inject } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus, Download, Printer } from '@element-plus/icons-vue'
import { type QualityRecord } from '@/api/quality'
import { logger } from '@/utils/logger'

const actions = inject<{
  openRecordDialog: (row: QualityRecord | null) => void
}>('qualityActions')

const records = ref<QualityRecord[]>([])
const loading = ref(false)

const fetchRecords = async () => {
  loading.value = true
  try {
    const { listQualityRecords } = await import('@/api/quality')
    const res = await listQualityRecords()
    records.value = (res.data as QualityRecord[] | undefined) || []
  } catch (error) {
    const err = error as Error
    logger.error('获取检验记录失败', err.message)
  } finally {
    loading.value = false
  }
}

const openCreate = () => {
  actions?.openRecordDialog(null)
}

const handleView = () => {
  ElMessage.info('查看检验记录')
}

const resultMap: Record<string, string> = { pass: '合格', fail: '不合格', pending: '待检' }

const handleExport = () => {
  const headers = ['记录编号,检验类型,产品,批次号,检验日期,检验员,结果']
  const rows = records.value.map(item =>
    [
      item.record_no,
      item.inspection_type,
      item.product_name,
      item.batch_no,
      item.inspection_date,
      item.inspector,
      resultMap[item.result] || item.result,
    ].join(',')
  )
  const csv = [...headers, ...rows].join('\n')
  const blob = new Blob(['\uFEFF' + csv], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `检验记录_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
  logger.info('检验记录已导出')
}

const handlePrint = () => {
  const printWindow = window.open('', '_blank')
  if (!printWindow) {
    ElMessage.error('无法打开打印窗口')
    return
  }
  const rows = records.value
    .map(
      item => `
    <tr>
      <td>${item.record_no}</td><td>${item.inspection_type}</td>
      <td>${item.product_name}</td><td>${item.batch_no}</td>
      <td>${item.inspection_date}</td><td>${item.inspector}</td>
      <td>${resultMap[item.result] || item.result}</td>
    </tr>
  `
    )
    .join('')
  printWindow.document.write(`<html><head><meta charset="utf-8"><title>检验记录</title>
    <style>@media print{@page{size:landscape;}}body{font-family:"Microsoft YaHei",sans-serif;font-size:12px;}h1{text-align:center;}table{width:100%;border-collapse:collapse;margin-top:12px;}th,td{border:1px solid #333;padding:6px 8px;}th{background:#f5f5f5;}.meta{text-align:center;color:#666;font-size:11px;}</style></head><body>
    <h1>质量检验记录</h1><div class="meta">打印日期: ${new Date().toISOString().split('T')[0]} | 共 ${records.value.length} 条</div>
    <table><thead><tr><th>记录编号</th><th>检验类型</th><th>产品</th><th>批次号</th><th>检验日期</th><th>检验员</th><th>结果</th></tr></thead><tbody>${rows}</tbody></table></body></html>`)
  printWindow.document.close()
  printWindow.onload = () => printWindow.print()
  logger.info('检验记录打印任务已生成')
}

onMounted(() => {
  fetchRecords()
})

defineExpose({ fetchRecords })
</script>
