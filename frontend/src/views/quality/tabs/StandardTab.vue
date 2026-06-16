<!--
  StandardTab.vue - 质量标准 Tab
  来源：原 quality/index.vue 中 质量标准 tab 内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="standard-tab">
    <div class="page-header">
      <h2 class="page-title">质量标准管理</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openCreate">
          <el-icon><Plus /></el-icon>
          新建标准
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
      <el-table v-loading="loading" :data="standards" stripe>
        <el-table-column prop="standard_code" label="标准编号" width="140" />
        <el-table-column prop="standard_name" label="标准名称" width="180" />
        <el-table-column prop="type" label="类型" width="100">
          <template #default="{ row }">
            {{ row.type === 'product' ? '产品标准' : '工艺标准' }}
          </template>
        </el-table-column>
        <el-table-column prop="version" label="版本" width="80" />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_by_name" label="创建人" width="100" />
        <el-table-column prop="approved_by_name" label="审批人" width="100">
          <template #default="{ row }">
            {{ row.approved_by_name || '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="approved_at" label="审批时间" width="160">
          <template #default="{ row }">
            {{ row.approved_at || '-' }}
          </template>
        </el-table-column>
        <el-table-column label="操作" width="300" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">查看</el-button>
            <el-button
              v-if="row.status !== 'draft'"
              type="primary"
              link
              size="small"
              @click="emit('openHistory', row)"
              >版本历史</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="primary"
              link
              size="small"
              @click="openEdit(row)"
              >编辑</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="success"
              link
              size="small"
              @click="emit('openApprove', row)"
              >审批</el-button
            >
            <el-button
              v-if="row.status === 'approved'"
              type="warning"
              link
              size="small"
              @click="handlePublish(row)"
              >发布</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, defineEmits, inject } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Download, Printer } from '@element-plus/icons-vue'
import { getQualityStandard, publishQualityStandard, type QualityStandard } from '@/api/quality'
import { logger } from '@/utils/logger'

const emit = defineEmits<{
  openApprove: [row: QualityStandard]
  openHistory: [row: QualityStandard]
}>()

const standards = ref<QualityStandard[]>([])
const loading = ref(false)

const actions = inject<{
  openStandardDialog: (row: QualityStandard | null) => void
}>('qualityActions')

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    approved: '已审批',
    published: '已发布',
    rejected: '已驳回',
  }
  return map[status] || status
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    approved: 'warning',
    published: 'success',
    rejected: 'danger',
  }
  return map[status] || 'info'
}

const fetchStandards = async () => {
  loading.value = true
  try {
    const { listQualityStandards } = await import('@/api/quality')
    const res = await listQualityStandards()
    standards.value = (res.data as QualityStandard[] | undefined) || []
  } catch (error) {
    const err = error as Error
    logger.error('获取质量标准失败', err.message)
  } finally {
    loading.value = false
  }
}

const openCreate = () => {
  actions?.openStandardDialog(null)
}

const openEdit = (row: QualityStandard) => {
  actions?.openStandardDialog(row)
}

const handleView = async (row: QualityStandard) => {
  try {
    const res = await getQualityStandard(row.id)
    actions?.openStandardDialog((res.data as QualityStandard | undefined) || null)
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '获取标准详情失败')
  }
}

const handlePublish = async (row: QualityStandard) => {
  try {
    await ElMessageBox.confirm('确定发布此标准吗？发布后将无法编辑。', '确认发布', {
      type: 'warning',
    })
    await publishQualityStandard(row.id)
    ElMessage.success('发布成功')
    fetchStandards()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const handleExport = () => {
  const headers = ['标准编号,标准名称,类型,版本,状态,创建人,审批人']
  const rows = standards.value.map(item =>
    [
      item.standard_code,
      item.standard_name,
      item.type === 'product' ? '产品标准' : '工艺标准',
      item.version,
      getStatusLabel(item.status),
      item.created_by_name || '-',
      item.approved_by_name || '-',
    ].join(',')
  )
  const csv = [...headers, ...rows].join('\n')
  const blob = new Blob(['\uFEFF' + csv], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `质量标准_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
  logger.info('质量标准列表已导出')
}

const handlePrint = () => {
  const printWindow = window.open('', '_blank')
  if (!printWindow) {
    ElMessage.error('无法打开打印窗口')
    return
  }
  const rows = standards.value
    .map(
      item => `
    <tr>
      <td>${item.standard_code}</td><td>${item.standard_name}</td>
      <td>${item.type === 'product' ? '产品标准' : '工艺标准'}</td>
      <td>${item.version}</td><td>${getStatusLabel(item.status)}</td>
      <td>${item.created_by_name || '-'}</td>
    </tr>
  `
    )
    .join('')
  printWindow.document.write(`<html><head><meta charset="utf-8"><title>质量标准</title>
    <style>@media print{@page{size:landscape;}}body{font-family:"Microsoft YaHei",sans-serif;font-size:12px;}h1{text-align:center;}table{width:100%;border-collapse:collapse;margin-top:12px;}th,td{border:1px solid #333;padding:6px 8px;}th{background:#f5f5f5;}.meta{text-align:center;color:#666;font-size:11px;}</style></head><body>
    <h1>质量标准列表</h1><div class="meta">打印日期: ${new Date().toISOString().split('T')[0]} | 共 ${standards.value.length} 条</div>
    <table><thead><tr><th>标准编号</th><th>标准名称</th><th>类型</th><th>版本</th><th>状态</th><th>创建人</th></tr></thead><tbody>${rows}</tbody></table></body></html>`)
  printWindow.document.close()
  printWindow.onload = () => printWindow.print()
  logger.info('质量标准打印任务已生成')
}

onMounted(() => {
  fetchStandards()
})

defineExpose({ fetchStandards })
</script>
