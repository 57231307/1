<!--
  BudgetListTab.vue - 预算管理 Tab
  来源：原 budget/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="budget-list-tab">
    <div class="page-header">
      <h2 class="page-title">预算管理</h2>
      <div>
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>新建预算
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>导出
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryForm">
        <el-form-item label="预算编号">
          <el-input v-model="queryForm.budget_no" placeholder="编号" clearable />
        </el-form-item>
        <el-form-item label="预算名称">
          <el-input v-model="queryForm.name" placeholder="名称" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryForm.status" placeholder="选择状态" clearable>
            <el-option label="草稿" value="draft" />
            <el-option label="待审核" value="pending" />
            <el-option label="已批准" value="approved" />
            <el-option label="已拒绝" value="rejected" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="budgetList" stripe>
        <el-table-column prop="budget_no" label="预算编号" width="140" />
        <el-table-column prop="name" label="预算名称" min-width="180" />
        <el-table-column prop="period" label="期间" width="120" />
        <el-table-column prop="department_name" label="部门" width="120" />
        <el-table-column label="预算总额" width="140" align="right">
          <template #default="{ row }">¥{{ row.total_amount.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" min-width="150" show-overflow-tooltip />
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button v-permission="'budget:update'" type="primary" link size="small" @click="openDialog(row)">编辑</el-button>
            <el-button
              v-permission="'budget:approve'"
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="approveBudget(row)"
              >审核</el-button
            >
            <el-button v-permission="'budget:delete'" type="danger" link size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="queryForm.page"
          v-model:page-size="queryForm.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleSearch"
          @current-change="handleSearch"
        />
      </div>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="form.id ? '编辑预算' : '新建预算'" width="500px">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-form-item label="预算编号" prop="budget_no">
          <el-input v-model="form.budget_no" :disabled="!!form.id" />
        </el-form-item>
        <el-form-item label="预算名称" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item label="期间" prop="period">
          <el-input v-model="form.period" placeholder="如 2024-01" />
        </el-form-item>
        <el-form-item label="预算总额" prop="total_amount">
          <el-input-number
            v-model="form.total_amount"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="部门ID">
          <el-input-number v-model="form.department_id" :min="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="form.remark" type="textarea" :rows="3" />
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
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Download } from '@element-plus/icons-vue'
import {
  listBudgets,
  createBudget,
  updateBudget,
  deleteBudget as deleteBudgetApi,
  approveBudget as approveBudgetApi,
  type Budget,
} from '@/api/budget'
import { logger } from '@/utils/logger'

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' })

const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const budgetList = ref<Budget[]>([])
const total = ref(0)
const formRef = ref<FormInstance>()

const queryForm = reactive({
  budget_no: '',
  name: '',
  status: '' as '' | Budget['status'],
  page: 1,
  page_size: 20,
})

const form = reactive<Partial<Budget>>({
  id: undefined,
  budget_no: '',
  name: '',
  period: new Date().toISOString().slice(0, 7),
  department_id: 0,
  total_amount: 0,
  status: 'draft',
  remark: '',
})

const rules: FormRules = {
  budget_no: [{ required: true, message: t('budget.validation.budgetNoRequired'), trigger: 'blur' }],
  name: [{ required: true, message: t('budget.validation.nameRequired'), trigger: 'blur' }],
  period: [{ required: true, message: t('budget.validation.periodRequired'), trigger: 'blur' }],
  total_amount: [{ required: true, message: t('budget.validation.totalAmountRequired'), trigger: 'blur' }],
}

const getStatusLabel = (status: Budget['status']) => {
  const map: Record<Budget['status'], string> = {
    draft: '草稿',
    pending: '待审核',
    approved: '已批准',
    rejected: '已拒绝',
  }
  return map[status] || status
}

const getStatusType = (status: Budget['status']) => {
  const map: Record<Budget['status'], string> = {
    draft: 'info',
    pending: 'warning',
    approved: 'success',
    rejected: 'danger',
  }
  return map[status] || 'info'
}

const fetchBudgets = async () => {
  loading.value = true
  try {
    const res = await listBudgets(queryForm)
    const payload = res.data as { list?: Budget[]; total?: number } | Budget[]
    if (Array.isArray(payload)) {
      budgetList.value = payload
      total.value = payload.length
    } else {
      budgetList.value = payload?.list || []
      total.value = payload?.total || 0
    }
  } catch (e) {
    const err = e as Error
    ElMessage.error(err.message || '获取预算列表失败')
  } finally {
    loading.value = false
  }
}

const handleSearch = () => {
  queryForm.page = 1
  fetchBudgets()
}

const handleReset = () => {
  queryForm.budget_no = ''
  queryForm.name = ''
  queryForm.status = ''
  handleSearch()
}

const openDialog = (row?: Budget) => {
  formRef.value?.resetFields()
  if (row) {
    Object.assign(form, row)
  } else {
    form.id = undefined
    form.budget_no = ''
    form.name = ''
    form.period = new Date().toISOString().slice(0, 7)
    form.department_id = 0
    form.total_amount = 0
    form.status = 'draft'
    form.remark = ''
  }
  dialogVisible.value = true
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      if (form.id) {
        await updateBudget(form.id, form)
        ElMessage.success(t('message.updateSuccess'))
      } else {
        await createBudget(form)
        ElMessage.success(t('message.createSuccess'))
      }
      dialogVisible.value = false
      fetchBudgets()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const approveBudget = async (row: Budget) => {
  try {
    await ElMessageBox.confirm(t('budget.confirmAudit', { name: row.name }), t('message.auditConfirmTitle'), { type: 'info' })
    await approveBudgetApi(row.id)
    ElMessage.success(t('budget.auditSuccess'))
    fetchBudgets()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || '审核失败')
    }
  }
}

const handleDelete = async (row: Budget) => {
  try {
    await ElMessageBox.confirm(`确定删除预算 "${row.name}" 吗？`, t('message.deleteConfirmTitle'), { type: 'warning' })
    await deleteBudgetApi(row.id)
    ElMessage.success(t('message.deleteSuccess'))
    fetchBudgets()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || '删除失败')
    }
  }
}

const handleExport = () => {
  const csvContent = [
    ['预算编号', '预算名称', '期间', '部门', '预算总额', '状态', '备注'],
    ...budgetList.value.map(b => [
      b.budget_no,
      b.name,
      b.period,
      b.department_name || '-',
      b.total_amount.toFixed(2),
      getStatusLabel(b.status),
      b.remark || '-',
    ]),
  ]
    .map(row => row.map(cell => `"${cell}"`).join(','))
    .join('\n')
  const blob = new Blob(['\uFEFF' + csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `预算列表_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success(t('message.exportSuccess'))
  logger.info('预算列表已导出')
}

onMounted(() => {
  fetchBudgets()
})
</script>
