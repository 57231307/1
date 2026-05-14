<script setup lang="ts">
import { ref } from 'vue'
import { ElTable, ElTableColumn, ElButton, ElDialog, ElForm, ElFormItem, ElInput, ElSelect, ElDatePicker, ElMessageBox, ElMessage, ElRow, ElCol } from 'element-plus'
import { Plus, Edit, Trash2, Eye, Refresh, Calendar } from '@element-plus/icons-vue'
import { listAccountingPeriods, getAccountingPeriod, createAccountingPeriod, updateAccountingPeriod, deleteAccountingPeriod, closePeriod, reopenPeriod, getCurrentPeriod, type AccountingPeriodEntity } from '@/api/accountingPeriod'

const tableData = ref<AccountingPeriodEntity[]>([])
const total = ref(0)
const loading = ref(false)
const searchForm = ref({
  year: '',
  month: '',
  status: ''
})
const pagination = ref({
  page: 1,
  pageSize: 20
})

const dialogVisible = ref(false)
const dialogTitle = ref('新增会计期间')
const form = ref<Partial<AccountingPeriodEntity>>({
  name: '',
  year: new Date().getFullYear(),
  month: 1,
  start_date: '',
  end_date: '',
  status: 'open'
})

const viewDialogVisible = ref(false)
const viewData = ref<AccountingPeriodEntity | null>(null)

const currentPeriod = ref<AccountingPeriodEntity | null>(null)

const statusOptions = [
  { label: '全部', value: '' },
  { label: '已打开', value: 'open' },
  { label: '已关闭', value: 'closed' }
]

const getStatusLabel = (value: string) => {
  return statusOptions.find(s => s.value === value)?.label || value
}

const getStatusClass = (value: string) => {
  return value === 'open' ? 'status-open' : 'status-closed'
}

const months = Array.from({ length: 12 }, (_, i) => ({
  label: `${i + 1}月`,
  value: i + 1
}))

const years = Array.from({ length: 10 }, (_, i) => ({
  label: `${new Date().getFullYear() - 5 + i}年`,
  value: new Date().getFullYear() - 5 + i
}))

const generatePeriodDates = () => {
  if (form.value.year && form.value.month) {
    const year = form.value.year as number
    const month = form.value.month as number
    const startDate = new Date(year, month - 1, 1)
    const endDate = new Date(year, month, 0)
    form.value.start_date = startDate.toISOString().split('T')[0]
    form.value.end_date = endDate.toISOString().split('T')[0]
    form.value.name = `${year}年${month}月`
  }
}

const loadData = async () => {
  loading.value = true
  try {
    const res = await listAccountingPeriods({
      page: pagination.value.page,
      pageSize: pagination.value.pageSize,
      year: searchForm.value.year ? Number(searchForm.value.year) : undefined,
      month: searchForm.value.month ? Number(searchForm.value.month) : undefined,
      status: searchForm.value.status || undefined
    })
    tableData.value = res.data.list
    total.value = res.data.total
  } catch (error) {
    ElMessage.error('加载失败')
  } finally {
    loading.value = false
  }
}

const loadCurrentPeriod = async () => {
  try {
    const res = await getCurrentPeriod()
    currentPeriod.value = res.data
  } catch (error) {
    console.log('获取当前期间失败')
  }
}

const handleSearch = () => {
  pagination.value.page = 1
  loadData()
}

const handleReset = () => {
  searchForm.value = {
    year: '',
    month: '',
    status: ''
  }
  handleSearch()
}

const handlePageChange = (page: number) => {
  pagination.value.page = page
  loadData()
}

const handlePageSizeChange = (pageSize: number) => {
  pagination.value.pageSize = pageSize
  loadData()
}

const openAddDialog = () => {
  dialogTitle.value = '新增会计期间'
  const now = new Date()
  form.value = {
    name: '',
    year: now.getFullYear(),
    month: now.getMonth() + 1,
    start_date: '',
    end_date: '',
    status: 'open'
  }
  generatePeriodDates()
  dialogVisible.value = true
}

const openEditDialog = (row: AccountingPeriodEntity) => {
  dialogTitle.value = '编辑会计期间'
  form.value = { ...row }
  dialogVisible.value = true
}

const openViewDialog = async (row: AccountingPeriodEntity) => {
  try {
    const res = await getAccountingPeriod(row.id!)
    viewData.value = res.data
    viewDialogVisible.value = true
  } catch (error) {
    ElMessage.error('获取详情失败')
  }
}

const handleSubmit = async () => {
  if (!form.value.name || !form.value.start_date || !form.value.end_date) {
    ElMessage.warning('请填写必填字段')
    return
  }
  try {
    if (form.value.id) {
      await updateAccountingPeriod(form.value.id, form.value)
      ElMessage.success('更新成功')
    } else {
      await createAccountingPeriod(form.value)
      ElMessage.success('新增成功')
    }
    dialogVisible.value = false
    loadData()
    loadCurrentPeriod()
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const handleDelete = async (row: AccountingPeriodEntity) => {
  if (row.status === 'closed') {
    ElMessage.warning('已关闭的期间不能删除')
    return
  }
  try {
    await ElMessageBox.confirm('确定要删除这个会计期间吗？', '提示', {
      type: 'warning'
    })
    await deleteAccountingPeriod(row.id!)
    ElMessage.success('删除成功')
    loadData()
  } catch (error) {
    ElMessage.info('取消删除')
  }
}

const handleClose = async (row: AccountingPeriodEntity) => {
  try {
    await ElMessageBox.confirm('确定要关闭这个会计期间吗？关闭后将无法录入凭证。', '提示', {
      type: 'warning'
    })
    await closePeriod(row.id!)
    ElMessage.success('期间已关闭')
    loadData()
    loadCurrentPeriod()
  } catch (error) {
    ElMessage.info('取消操作')
  }
}

const handleReopen = async (row: AccountingPeriodEntity) => {
  try {
    await ElMessageBox.confirm('确定要重新打开这个会计期间吗？', '提示', {
      type: 'warning'
    })
    await reopenPeriod(row.id!)
    ElMessage.success('期间已重新打开')
    loadData()
    loadCurrentPeriod()
  } catch (error) {
    ElMessage.info('取消操作')
  }
}

loadData()
loadCurrentPeriod()
</script>

<template>
  <div class="app-container">
    <div class="current-period-card" v-if="currentPeriod">
      <div class="card-icon">
        <Calendar />
      </div>
      <div class="card-content">
        <div class="card-title">当前会计期间</div>
        <div class="card-value">{{ currentPeriod.name }}</div>
        <div class="card-status" :class="getStatusClass(currentPeriod.status)">
          {{ getStatusLabel(currentPeriod.status) }}
        </div>
      </div>
    </div>

    <div class="filter-container">
      <ElRow :gutter="20">
        <ElCol :span="6">
          <ElSelect
            v-model="searchForm.year"
            placeholder="选择年份"
            class="filter-item"
          >
            <ElOption label="全部" value="" />
            <ElOption v-for="y in years" :key="y.value" :label="y.label" :value="String(y.value)" />
          </ElSelect>
        </ElCol>
        <ElCol :span="6">
          <ElSelect
            v-model="searchForm.month"
            placeholder="选择月份"
            class="filter-item"
          >
            <ElOption label="全部" value="" />
            <ElOption v-for="m in months" :key="m.value" :label="m.label" :value="String(m.value)" />
          </ElSelect>
        </ElCol>
        <ElCol :span="6">
          <ElSelect
            v-model="searchForm.status"
            placeholder="状态"
            class="filter-item"
          >
            <ElOption v-for="s in statusOptions" :key="s.value" :label="s.label" :value="s.value" />
          </ElSelect>
        </ElCol>
      </ElRow>
      <div class="filter-actions">
        <ElButton type="primary" @click="handleSearch">查询</ElButton>
        <ElButton @click="handleReset">重置</ElButton>
        <ElButton type="success" @click="openAddDialog">
          <Plus /> 新增期间
        </ElButton>
      </div>
    </div>

    <ElTable
      :data="tableData"
      :total="total"
      :loading="loading"
      :page-size="pagination.pageSize"
      :current-page="pagination.page"
      @current-change="handlePageChange"
      @size-change="handlePageSizeChange"
      border
      fit
      highlight-current-row
      style="width: 100%"
    >
      <ElTableColumn prop="name" label="期间名称" width="150" />
      <ElTableColumn prop="year" label="年份" width="80" />
      <ElTableColumn prop="month" label="月份" width="80" />
      <ElTableColumn prop="start_date" label="开始日期" width="120" />
      <ElTableColumn prop="end_date" label="结束日期" width="120" />
      <ElTableColumn prop="status" label="状态" width="100">
        <template #default="scope">
          <span :class="['status-tag', getStatusClass(scope.row.status)]">
            {{ getStatusLabel(scope.row.status) }}
          </span>
        </template>
      </ElTableColumn>
      <ElTableColumn prop="closed_at" label="关闭时间" width="150" />
      <ElTableColumn label="操作" width="250" align="center">
        <template #default="scope">
          <ElButton size="small" @click="openViewDialog(scope.row)">
            <Eye />
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'open'"
            size="small"
            type="primary"
            @click="openEditDialog(scope.row)"
          >
            <Edit />
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'open'"
            size="small"
            type="warning"
            @click="handleClose(scope.row)"
          >
            结账
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'closed'"
            size="small"
            type="success"
            @click="handleReopen(scope.row)"
          >
            反结账
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'open'"
            size="small"
            type="danger"
            @click="handleDelete(scope.row)"
          >
            <Trash2 />
          </ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <ElDialog :title="dialogTitle" :visible="dialogVisible" width="500px" @close="dialogVisible = false">
      <ElForm :model="form" label-width="100px">
        <ElFormItem label="期间名称" prop="name">
          <ElInput v-model="form.name" placeholder="自动生成" readonly />
        </ElFormItem>
        <ElRow :gutter="20">
          <ElCol :span="12">
            <ElFormItem label="年份" prop="year">
              <ElSelect
                v-model="form.year"
                placeholder="选择年份"
                @change="generatePeriodDates"
              >
                <ElOption v-for="y in years" :key="y.value" :label="y.label" :value="y.value" />
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="月份" prop="month">
              <ElSelect
                v-model="form.month"
                placeholder="选择月份"
                @change="generatePeriodDates"
              >
                <ElOption v-for="m in months" :key="m.value" :label="m.label" :value="m.value" />
              </ElSelect>
            </ElFormItem>
          </ElCol>
        </ElRow>
        <ElRow :gutter="20">
          <ElCol :span="12">
            <ElFormItem label="开始日期" prop="start_date">
              <ElDatePicker v-model="form.start_date" type="date" />
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="结束日期" prop="end_date">
              <ElDatePicker v-model="form.end_date" type="date" />
            </ElFormItem>
          </ElCol>
        </ElRow>
      </ElForm>
      <template #footer>
        <ElButton @click="dialogVisible = false">取消</ElButton>
        <ElButton type="primary" @click="handleSubmit">确定</ElButton>
      </template>
    </ElDialog>

    <ElDialog title="期间详情" :visible="viewDialogVisible" width="500px" @close="viewDialogVisible = false">
      <ElDescriptions v-if="viewData" :column="2" border>
        <ElDescriptionsItem label="期间名称">{{ viewData.name }}</ElDescriptionsItem>
        <ElDescriptionsItem label="状态">{{ getStatusLabel(viewData.status) }}</ElDescriptionsItem>
        <ElDescriptionsItem label="年份">{{ viewData.year }}</ElDescriptionsItem>
        <ElDescriptionsItem label="月份">{{ viewData.month }}月</ElDescriptionsItem>
        <ElDescriptionsItem label="开始日期">{{ viewData.start_date }}</ElDescriptionsItem>
        <ElDescriptionsItem label="结束日期">{{ viewData.end_date }}</ElDescriptionsItem>
        <ElDescriptionsItem label="关闭时间">{{ viewData.closed_at || '-' }}</ElDescriptionsItem>
        <ElDescriptionsItem label="创建时间">{{ viewData.created_at }}</ElDescriptionsItem>
      </ElDescriptions>
    </ElDialog>
  </div>
</template>

<style scoped>
.app-container {
  padding: 20px;
}

.current-period-card {
  display: flex;
  align-items: center;
  padding: 20px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border-radius: 8px;
  margin-bottom: 20px;
  color: #fff;
}

.card-icon {
  width: 60px;
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.2);
  border-radius: 50%;
  font-size: 24px;
  margin-right: 20px;
}

.card-content {
  flex: 1;
}

.card-title {
  font-size: 14px;
  opacity: 0.8;
  margin-bottom: 5px;
}

.card-value {
  font-size: 24px;
  font-weight: bold;
  margin-bottom: 5px;
}

.card-status {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
}

.card-status.status-open {
  background: rgba(255, 255, 255, 0.2);
}

.card-status.status-closed {
  background: rgba(255, 255, 255, 0.3);
}

.filter-container {
  margin-bottom: 20px;
}

.filter-item {
  width: 100%;
}

.filter-actions {
  margin-top: 10px;
}

.status-tag {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
}

.status-open {
  background: #f0f9eb;
  color: #67c23a;
}

.status-closed {
  background: #fff7e6;
  color: #e6a23c;
}
</style>