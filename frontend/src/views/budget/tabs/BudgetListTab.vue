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
      <el-form :inline="true" :model="queryForm" aria-label="预算筛选表单">
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
      <el-table v-loading="loading" :data="budgetList" stripe aria-label="预算列表">
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
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          aria-label="预算列表分页"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="form.id ? '编辑预算' : '新建预算'" width="500px" aria-label="预算编辑对话框">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px" aria-label="预算表单">
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
import { ref, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Download } from '@element-plus/icons-vue'
import {
  createBudget,
  updateBudget,
  deleteBudget as deleteBudgetApi,
  approveBudget as approveBudgetApi,
  type Budget,
} from '@/api/budget'
import { logger } from '@/utils/logger'
import { exportFromBackend } from '@/utils/export'
// 批次 278：迁移到 useTableApi composable，自动管理分页与 loading
import { useTableApi } from '@/composables/useTableApi'

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' })

const submitLoading = ref(false)
const dialogVisible = ref(false)
const formRef = ref<FormInstance>()

// 批次 278：筛选条件（仅保留业务字段，page/page_size 由 useTableApi 管理）
const queryForm = reactive({
  budget_no: '',
  name: '',
  status: '' as '' | Budget['status'],
})

// 批次 278：使用 useTableApi 管理预算列表分页
const {
  data: budgetList,
  total,
  loading,
  page,
  pageSize,
  queryParams,
  setQueryParam,
  refresh: fetchBudgets,
} = useTableApi<Budget>({
  url: '/budgets',
  defaultPageSize: 20,
  onError: (err: unknown) => {
    if (err instanceof Error) {
      ElMessage.error(err.message || '获取预算列表失败')
    } else {
      ElMessage.error('获取预算列表失败')
    }
  },
})

// 批次 278：将筛选字段同步到 queryParams
const syncQueryParams = () => {
  setQueryParam('budget_no', queryForm.budget_no)
  setQueryParam('name', queryForm.name)
  setQueryParam('status', queryForm.status)
}

// 批次 278：分页变化处理函数
const handlePageChange = (_p: number) => {
  // useTableApi 内部 watch page 自动触发刷新
}
const handleSizeChange = (_s: number) => {
  // useTableApi 内部 watch pageSize 自动触发刷新
  page.value = 1
}

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

const handleSearch = () => {
  // 批次 278：同步筛选条件并重置到第一页
  syncQueryParams()
  page.value = 1
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

// V15 P0-S12 修复（Batch 475e）：迁移到后端导出，注入水印 + 审计日志
const handleExport = async () => {
  const params: Record<string, unknown> = {
    item_type: queryParams.value.item_type as string | undefined,
    status: queryParams.value.status as string | undefined,
  }
  await exportFromBackend('/budgets/export', params, 'budget_items_export')
  logger.info('预算列表已导出')
}
</script>
