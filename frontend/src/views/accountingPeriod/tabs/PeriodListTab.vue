<!--
  PeriodListTab.vue - 会计期间 Tab
  来源：原 accountingPeriod/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="period-list-tab">
    <div class="page-header">
      <h2 class="page-title">会计期间</h2>
      <div>
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>新建期间
        </el-button>
        <el-button @click="handleInitYear">
          <el-icon><Refresh /></el-icon>初始化年度
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryForm">
        <el-form-item label="年度">
          <el-input-number v-model="queryForm.year" :min="2000" :max="2100" style="width: 140px" />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryForm.status" placeholder="选择状态" clearable>
            <el-option label="未启用" value="pending" />
            <el-option label="启用中" value="active" />
            <el-option label="已关闭" value="closed" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="periodList" stripe>
        <el-table-column prop="name" label="期间名称" width="160" />
        <el-table-column prop="year" label="年度" width="80" align="center" />
        <el-table-column prop="month" label="月份" width="80" align="center" />
        <el-table-column prop="start_date" label="开始日期" width="120" />
        <el-table-column prop="end_date" label="结束日期" width="120" />
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="closed_at" label="关闭时间" width="180" />
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openDialog(row)">编辑</el-button>
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="activatePeriod(row)"
              >启用</el-button
            >
            <el-button
              v-if="row.status === 'active'"
              type="warning"
              link
              size="small"
              @click="closePeriod(row)"
              >关闭</el-button
            >
            <el-button
              v-if="row.status === 'closed'"
              type="info"
              link
              size="small"
              @click="reopenPeriod(row)"
              >重新打开</el-button
            >
            <el-button
              v-if="row.status !== 'active'"
              type="danger"
              link
              size="small"
              @click="deletePeriod(row)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="form.id ? '编辑期间' : '新建期间'" width="500px">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-form-item label="年度" prop="year">
          <el-input-number v-model="form.year" :min="2000" :max="2100" style="width: 100%" />
        </el-form-item>
        <el-form-item label="月份" prop="month">
          <el-input-number v-model="form.month" :min="1" :max="12" style="width: 100%" />
        </el-form-item>
        <el-form-item label="期间名称">
          <el-input :value="`${form.year}-${String(form.month).padStart(2, '0')}`" disabled />
        </el-form-item>
        <el-form-item label="开始日期" prop="start_date">
          <el-date-picker
            v-model="form.start_date"
            type="date"
            placeholder="选择日期"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="结束日期" prop="end_date">
          <el-date-picker
            v-model="form.end_date"
            type="date"
            placeholder="选择日期"
            value-format="YYYY-MM-DD"
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
import { Plus, Refresh } from '@element-plus/icons-vue'
import {
  listAccountingPeriods,
  createAccountingPeriod,
  updateAccountingPeriod,
  deleteAccountingPeriod,
  closePeriod as closePeriodApi,
  reopenPeriod as reopenPeriodApi,
  type AccountingPeriodEntity,
} from '@/api/accounting-period'
import { logger } from '@/utils/logger'

const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const periodList = ref<AccountingPeriodEntity[]>([])
const formRef = ref<FormInstance>()

const queryForm = reactive({
  year: new Date().getFullYear(),
  status: '',
})

const form = reactive<Partial<AccountingPeriodEntity>>({
  id: undefined,
  name: '',
  year: new Date().getFullYear(),
  month: 1,
  start_date: '',
  end_date: '',
  status: 'pending',
})

const rules: FormRules = {
  year: [{ required: true, message: '请选择年度', trigger: 'blur' }],
  month: [{ required: true, message: '请选择月份', trigger: 'blur' }],
  start_date: [{ required: true, message: '请选择开始日期', trigger: 'change' }],
  end_date: [{ required: true, message: '请选择结束日期', trigger: 'change' }],
}

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: '未启用',
    active: '启用中',
    closed: '已关闭',
  }
  return map[status] || status
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    pending: 'info',
    active: 'success',
    closed: 'warning',
  }
  return map[status] || 'info'
}

const fetchPeriods = async () => {
  loading.value = true
  try {
    const res = await listAccountingPeriods(queryForm)
    const d = (res as { data?: unknown }).data as
      | AccountingPeriodEntity[]
      | {
          items?: AccountingPeriodEntity[]
          data?: AccountingPeriodEntity[]
          list?: AccountingPeriodEntity[]
        }
    if (Array.isArray(d)) {
      periodList.value = d
    } else {
      periodList.value = d?.items || d?.data || d?.list || []
    }
  } catch (e) {
    const err = e as Error
    ElMessage.error(err.message || '获取会计期间失败')
  } finally {
    loading.value = false
  }
}

const handleSearch = () => {
  fetchPeriods()
}

const handleReset = () => {
  queryForm.year = new Date().getFullYear()
  queryForm.status = ''
  fetchPeriods()
}

const computeDateRange = (year: number, month: number) => {
  const start = new Date(year, month - 1, 1)
  const end = new Date(year, month, 0)
  return {
    start_date: start.toISOString().split('T')[0],
    end_date: end.toISOString().split('T')[0],
  }
}

const openDialog = (row?: AccountingPeriodEntity) => {
  formRef.value?.resetFields()
  if (row) {
    Object.assign(form, row)
  } else {
    const now = new Date()
    form.id = undefined
    form.name = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`
    form.year = now.getFullYear()
    form.month = now.getMonth() + 1
    const { start_date, end_date } = computeDateRange(form.year, form.month)
    form.start_date = start_date
    form.end_date = end_date
    form.status = 'pending'
  }
  dialogVisible.value = true
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      form.name = `${form.year}-${String(form.month).padStart(2, '0')}`
      if (form.id) {
        await updateAccountingPeriod(form.id, form)
        ElMessage.success('更新成功')
      } else {
        await createAccountingPeriod(form)
        ElMessage.success('创建成功')
      }
      dialogVisible.value = false
      fetchPeriods()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const activatePeriod = async (row: AccountingPeriodEntity) => {
  if (!row.id) return
  try {
    await ElMessageBox.confirm(`确定启用期间 "${row.name}" 吗？`, '启用确认', { type: 'info' })
    await updateAccountingPeriod(row.id, { status: 'active' })
    ElMessage.success('已启用')
    fetchPeriods()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || '启用失败')
    }
  }
}

const closePeriod = async (row: AccountingPeriodEntity) => {
  if (!row.id) return
  try {
    await ElMessageBox.confirm(
      `确定关闭期间 "${row.name}" 吗？关闭后无法再录入凭证。`,
      '关闭确认',
      { type: 'warning' }
    )
    await closePeriodApi(row.id)
    ElMessage.success('已关闭')
    fetchPeriods()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || '关闭失败')
    }
  }
}

const reopenPeriod = async (row: AccountingPeriodEntity) => {
  if (!row.id) return
  try {
    await ElMessageBox.confirm(`确定重新打开期间 "${row.name}" 吗？`, '重新打开确认', {
      type: 'info',
    })
    await reopenPeriodApi(row.id)
    ElMessage.success('已重新打开')
    fetchPeriods()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const deletePeriod = async (row: AccountingPeriodEntity) => {
  if (!row.id) return
  try {
    await ElMessageBox.confirm(`确定删除期间 "${row.name}" 吗？`, '删除确认', { type: 'warning' })
    await deleteAccountingPeriod(row.id)
    ElMessage.success('删除成功')
    fetchPeriods()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || '删除失败')
    }
  }
}

const handleInitYear = async () => {
  try {
    const year = queryForm.year
    await ElMessageBox.confirm(`确定初始化 ${year} 年的会计期间吗？`, '初始化确认', {
      type: 'info',
    })
    for (let month = 1; month <= 12; month++) {
      const { start_date, end_date } = computeDateRange(year, month)
      await createAccountingPeriod({
        name: `${year}-${String(month).padStart(2, '0')}`,
        year,
        month,
        start_date,
        end_date,
        status: 'pending',
      })
    }
    ElMessage.success('初始化成功')
    fetchPeriods()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      logger.error('初始化年度期间失败', err)
      ElMessage.error(err.message || '初始化失败')
    }
  }
}

onMounted(() => {
  fetchPeriods()
})
</script>
