<template>
  <div class="financial-analysis-container">
    <el-card class="header-card">
      <div class="header-content">
        <h2>财务分析</h2>
        <p>管理和执行财务分析报告，查看财务趋势数据</p>
      </div>
    </el-card>

    <el-card class="table-card">
      <template #header>
        <div class="card-header">
          <span>财务分析报告</span>
          <el-button type="primary" @click="openDialog('create')">
            <el-icon><Plus /></el-icon>新建报告
          </el-button>
        </div>
      </template>
      
      <el-table :data="reportList" v-loading="loading" stripe border>
        <el-table-column prop="reportName" label="报告名称" min-width="160" />
        <el-table-column prop="reportType" label="报告类型" width="140" />
        <el-table-column prop="period" label="期间" width="120" />
        <el-table-column prop="status" label="状态" width="120">
          <template #default="{ row }">
            <el-tag :type="row.status === 'completed' ? 'success' : 'warning'">
              {{ row.status === 'completed' ? '已完成' : '草稿' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="createdAt" label="创建时间" width="160" />
        <el-table-column label="操作" width="220" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewDetail(row)">查看</el-button>
            <el-button type="success" link size="small" @click="handleExecute(row)" v-if="row.status === 'draft'">执行</el-button>
            <el-button type="danger" link size="small" @click="handleDelete(row)" v-if="row.status === 'draft'">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="queryForm.page"
          v-model:page-size="queryForm.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="fetchReports"
          @current-change="fetchReports"
        />
      </div>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="dialogType === 'create' ? '新建财务报告' : '编辑财务报告'"
      width="600px"
      @close="resetForm"
    >
      <el-form :model="reportForm" :rules="reportRules" ref="reportFormRef" label-width="120px">
        <el-form-item label="报告名称" prop="reportName">
          <el-input v-model="reportForm.reportName" placeholder="请输入报告名称" />
        </el-form-item>
        <el-form-item label="报告类型" prop="reportType">
          <el-select v-model="reportForm.reportType" placeholder="请选择报告类型" style="width: 100%">
            <el-option label="收入分析" value="income" />
            <el-option label="支出分析" value="expense" />
            <el-option label="利润分析" value="profit" />
            <el-option label="现金流分析" value="cashflow" />
          </el-select>
        </el-form-item>
        <el-form-item label="期间" prop="period">
          <el-input v-model="reportForm.period" placeholder="例如：2024-01" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmitForm">确认</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" title="财务分析详情" width="700px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="报告名称">{{ currentReport?.reportName }}</el-descriptions-item>
        <el-descriptions-item label="报告类型">{{ currentReport?.reportType }}</el-descriptions-item>
        <el-descriptions-item label="期间">{{ currentReport?.period }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="currentReport?.status === 'completed' ? 'success' : 'warning'">
            {{ currentReport?.status === 'completed' ? '已完成' : '草稿' }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="创建时间">{{ currentReport?.createdAt }}</el-descriptions-item>
        <el-descriptions-item label="更新时间">{{ currentReport?.updatedAt }}</el-descriptions-item>
      </el-descriptions>
      <el-divider />
      <el-alert
        title="分析结果预览"
        type="info"
        :closable="false"
        style="margin-bottom: 20px"
      />
      <el-empty description="分析数据将在此处显示" v-if="true" />
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
  deleteReport,
  executeFinancialReport,
  type FinancialReport,
} from '@/api/financial-analysis'

const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const detailVisible = ref(false)
const dialogType = ref<'create' | 'edit'>('create')
const reportList = ref<FinancialReport[]>([])
const currentReport = ref<FinancialReport | null>(null)
const reportFormRef = ref<FormInstance>()
const total = ref(0)

const queryForm = reactive({
  page: 1,
  page_size: 20,
})

const reportForm = reactive<Partial<FinancialReport>>({
  reportName: '',
  reportType: '',
  period: '',
  status: 'draft',
})

const reportRules: FormRules = {
  reportName: [{ required: true, message: '请输入报告名称', trigger: 'blur' }],
  reportType: [{ required: true, message: '请选择报告类型', trigger: 'change' }],
  period: [{ required: true, message: '请输入期间', trigger: 'blur' }],
}

const fetchReports = async () => {
  loading.value = true
  try {
    const res = await listReports(queryForm)
    reportList.value = res.data?.list || []
    total.value = res.data?.total || 0
  } catch (e: any) {
    ElMessage.error(e.message || '获取报告列表失败')
  } finally {
    loading.value = false
  }
}

const openDialog = (type: 'create' | 'edit', row?: FinancialReport) => {
  dialogType.value = type
  resetForm()
  
  if (type === 'edit' && row) {
    Object.assign(reportForm, row)
  }
  
  dialogVisible.value = true
}

const resetForm = () => {
  Object.assign(reportForm, {
    id: undefined,
    reportName: '',
    reportType: '',
    period: '',
    status: 'draft',
  })
  reportFormRef.value?.clearValidate()
}

const handleSubmitForm = async () => {
  if (!reportFormRef.value) return
  
  await reportFormRef.value.validate(async (valid) => {
    if (!valid) return
    
    submitLoading.value = true
    try {
      if (dialogType.value === 'create') {
        await createReport(reportForm)
        ElMessage.success('创建成功')
      } else {
        if (reportForm.id) {
          await updateReport(reportForm.id, reportForm)
          ElMessage.success('更新成功')
        }
      }
      
      dialogVisible.value = false
      fetchReports()
    } catch (e: any) {
      ElMessage.error(e.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const viewDetail = (row: FinancialReport) => {
  currentReport.value = row
  detailVisible.value = true
}

const handleExecute = async (row: FinancialReport) => {
  try {
    await ElMessageBox.confirm(`确认执行财务报告 ${row.reportName} 吗？`, '确认', { type: 'info' })
    await executeFinancialReport(row.id)
    ElMessage.success('执行成功，报告已完成')
    fetchReports()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '执行失败')
  }
}

const handleDelete = async (row: FinancialReport) => {
  try {
    await ElMessageBox.confirm(`确认删除报告 ${row.reportName} 吗？`, '删除确认', {
      type: 'warning',
      confirmButtonText: '确定',
      cancelButtonText: '取消',
    })
    
    await deleteReport(row.id)
    ElMessage.success('删除成功')
    fetchReports()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '删除失败')
  }
}

onMounted(() => {
  fetchReports()
})
</script>

<style scoped>
.financial-analysis-container {
  padding: 20px;
}

.header-card {
  margin-bottom: 20px;
}

.header-content h2 {
  margin: 0 0 8px 0;
  color: #303133;
}

.header-content p {
  margin: 0;
  color: #909399;
}

.table-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.pagination-container {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
</style>
