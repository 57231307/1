<!--
  AnalysisListTab.vue - 财务分析 Tab
  来源：原 financial-analysis/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="analysis-list-tab">
    <div class="page-header">
      <h2 class="page-title">财务分析</h2>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryForm" aria-label="财务分析筛选表单">
        <el-form-item label="报表类型">
          <el-select v-model="queryForm.reportType" placeholder="选择报表类型" style="width: 180px">
            <el-option label="盈利能力" value="profitability" />
            <el-option label="偿债能力" value="solvency" />
            <el-option label="运营能力" value="operation" />
            <el-option label="发展能力" value="development" />
          </el-select>
        </el-form-item>
        <el-form-item label="会计期间">
          <el-date-picker
            v-model="queryForm.period"
            type="month"
            placeholder="选择月份"
            value-format="YYYY-MM"
            style="width: 160px"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleAnalyze">开始分析</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <template #header>
        <div class="card-header">
          <span>分析报表</span>
          <el-button type="primary" size="small" @click="openCreateDialog">
            <el-icon><Plus /></el-icon>新建报表
          </el-button>
        </div>
      </template>
      <el-table v-loading="loading" :data="reports" stripe aria-label="财务分析报表列表">
        <el-table-column prop="reportName" label="报表名称" min-width="180" />
        <el-table-column prop="reportType" label="类型" width="120">
          <template #default="{ row }">
            <el-tag size="small">{{ getReportTypeLabel(row.reportType) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="period" label="会计期间" width="120" />
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="executedAt" label="执行时间" width="180" />
        <el-table-column label="操作" width="240" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="executeReport(row)">执行</el-button>
            <el-button type="success" link size="small" @click="viewReport(row)">查看</el-button>
            <el-button v-permission="'financial_report:update'" type="warning" link size="small" @click="editReport(row)">编辑</el-button>
            <el-button v-permission="'financial_report:delete'" type="danger" link size="small" @click="deleteReport(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="form.id ? '编辑报表' : '新建报表'" width="500px" :aria-label="form.id ? '编辑报表对话框' : '新建报表对话框'">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px" aria-label="财务分析报表表单">
        <el-form-item label="报表名称" prop="reportName">
          <el-input v-model="form.reportName" placeholder="请输入报表名称" />
        </el-form-item>
        <el-form-item label="报表类型" prop="reportType">
          <el-select v-model="form.reportType" placeholder="选择类型" style="width: 100%">
            <el-option label="盈利能力" value="profitability" />
            <el-option label="偿债能力" value="solvency" />
            <el-option label="运营能力" value="operation" />
            <el-option label="发展能力" value="development" />
          </el-select>
        </el-form-item>
        <el-form-item label="会计期间">
          <el-date-picker
            v-model="form.period"
            type="month"
            placeholder="选择月份"
            value-format="YYYY-MM"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  listReports,
  createReport,
  updateReport,
  deleteReport as deleteReportApi,
  executeFinancialReport,
  type FinancialReport,
} from '@/api/financial-analysis'
import { logger } from '@/utils/logger'

const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const reports = ref<FinancialReport[]>([])
const formRef = ref<FormInstance>()

const queryForm = reactive({
  reportType: '',
  period: new Date().toISOString().slice(0, 7),
})

const form = reactive<Partial<FinancialReport>>({
  id: undefined,
  reportName: '',
  reportType: 'profitability',
  period: new Date().toISOString().slice(0, 7),
})

const rules: FormRules = {
  reportName: [{ required: true, message: '请输入报表名称', trigger: 'blur' }],
  reportType: [{ required: true, message: '请选择报表类型', trigger: 'change' }],
}

const getReportTypeLabel = (type?: string) => {
  const map: Record<string, string> = {
    profitability: '盈利能力',
    solvency: '偿债能力',
    operation: '运营能力',
    development: '发展能力',
  }
  return map[type || ''] || type || '-'
}

const getStatusLabel = (status?: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    executed: '已执行',
    failed: '执行失败',
  }
  return map[status || ''] || status || '-'
}

const getStatusType = (status?: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    executed: 'success',
    failed: 'danger',
  }
  return map[status || ''] || 'info'
}

const fetchReports = async () => {
  loading.value = true
  try {
    const res = await listReports(queryForm)
    const d = (res as { data?: unknown }).data as
      | {
          list?: FinancialReport[]
          items?: FinancialReport[]
          data?: FinancialReport[]
          total?: number
        }
      | FinancialReport[]
    if (Array.isArray(d)) {
      reports.value = d
    } else {
      reports.value = d?.list || d?.items || []
    }
  } catch (e) {
    const err = e as Error
    logger.error('获取财务分析报表失败', err)
    ElMessage.error(err.message || '获取报表失败')
  } finally {
    loading.value = false
  }
}

const handleAnalyze = () => {
  fetchReports()
}

const openCreateDialog = () => {
  form.id = undefined
  form.reportName = ''
  form.reportType = 'profitability'
  form.period = new Date().toISOString().slice(0, 7)
  dialogVisible.value = true
}

const editReport = (row: FinancialReport) => {
  Object.assign(form, row)
  dialogVisible.value = true
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      if (form.id) {
        await updateReport(form.id, form)
        ElMessage.success('更新成功')
      } else {
        await createReport(form)
        ElMessage.success('创建成功')
      }
      dialogVisible.value = false
      fetchReports()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const executeReport = async (row: FinancialReport) => {
  if (row.id === undefined) return
  try {
    await executeFinancialReport(row.id)
    ElMessage.success('执行成功')
    fetchReports()
  } catch (e) {
    const err = e as Error
    ElMessage.error(err.message || '执行失败')
  }
}

// 批次 157b P1-1 修复：展示报表详情（无独立 getReport API，使用行数据展示）
const viewReport = async (row: FinancialReport) => {
  const lines = [
    `报表名称：${row.reportName || '-'}`,
    `报表类型：${getReportTypeLabel(row.reportType)}`,
    `会计期间：${row.period || '-'}`,
    `状态：${getStatusLabel(row.status)}`,
    `执行时间：${row.executedAt || '-'}`,
    `创建时间：${row.createdAt || '-'}`,
    `更新时间：${row.updatedAt || '-'}`,
  ]
  await ElMessageBox.alert(lines.join('\n'), '报表详情', { confirmButtonText: '关闭' })
}

const deleteReport = async (row: FinancialReport) => {
  if (row.id === undefined) return
  try {
    await ElMessageBox.confirm(`确定删除报表 "${row.reportName}" 吗？`, '删除确认', {
      type: 'warning',
    })
    await deleteReportApi(row.id)
    ElMessage.success('删除成功')
    fetchReports()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || '删除失败')
    }
  }
}

onMounted(() => {
  fetchReports()
})
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
