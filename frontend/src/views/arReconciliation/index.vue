<script setup lang="ts">
import { ref } from 'vue'
import { ElTable, ElTableColumn, ElButton, ElDialog, ElForm, ElFormItem, ElInput, ElSelect, ElDatePicker, ElMessageBox, ElMessage, ElRow, ElCol, ElDescriptions } from 'element-plus'
import { Plus, Edit, Delete, View, Check } from '@element-plus/icons-vue'
import { listArReconciliations, getArReconciliation, createArReconciliation, updateArReconciliation, deleteArReconciliation, confirmReconciliation, getReconciliationDetails, type ArReconciliationEntity, type ReconciliationDetail } from '@/api/ar-reconciliation'
import { request } from '@/api/request'

const tableData = ref<ArReconciliationEntity[]>([])
const total = ref(0)
const loading = ref(false)
const searchForm = ref({
  customer_name: '',
  status: '',
  start_date: '',
  end_date: ''
})
const pagination = ref({
  page: 1,
  pageSize: 20
})

const dialogVisible = ref(false)
const dialogTitle = ref('新增对账')
const form = ref<Partial<ArReconciliationEntity>>({
  customer_id: 0,
  customer_name: '',
  start_date: '',
  end_date: '',
  status: 'draft'
})

const viewDialogVisible = ref(false)
const viewData = ref<ArReconciliationEntity | null>(null)
const detailData = ref<ReconciliationDetail[]>([])

const customerOptions = ref<{ label: string; value: number }[]>([])

const statusOptions = [
  { label: '全部', value: '' },
  { label: '草稿', value: 'draft' },
  { label: '已确认', value: 'confirmed' }
]

const getStatusLabel = (value: string) => {
  return statusOptions.find(s => s.value === value)?.label || value
}

const getStatusClass = (value: string) => {
  return value === 'draft' ? 'status-draft' : 'status-confirmed'
}

const loadData = async () => {
  loading.value = true
  try {
    const res: any = await listArReconciliations({
      page: pagination.value.page,
      pageSize: pagination.value.pageSize,
      ...searchForm.value
    })
    tableData.value = res.data!.list
    total.value = res.data!.total
  } catch (error) {
    ElMessage.error('加载失败')
  } finally {
    loading.value = false
  }
}

const loadCustomers = async () => {
  try {
    const res: any = await request.get('/api/v1/customers/select')
    customerOptions.value = res.data!
  } catch (error) {
    console.warn('加载客户失败')
  }
}

const handleSearch = () => {
  pagination.value.page = 1
  loadData()
}

const handleReset = () => {
  searchForm.value = {
    customer_name: '',
    status: '',
    start_date: '',
    end_date: ''
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
  dialogTitle.value = '新增对账'
  form.value = {
    customer_id: 0,
    customer_name: '',
    start_date: '',
    end_date: '',
    status: 'draft'
  }
  dialogVisible.value = true
}

const openEditDialog = (row: ArReconciliationEntity) => {
  dialogTitle.value = '编辑对账'
  form.value = { ...row }
  dialogVisible.value = true
}

const openViewDialog = async (row: ArReconciliationEntity) => {
  try {
    const res: any = await getArReconciliation(row.id!)
    viewData.value = res.data!
    const detailRes: any = await getReconciliationDetails(row.id!)
    detailData.value = detailRes.data
    viewDialogVisible.value = true
  } catch (error) {
    ElMessage.error('获取详情失败')
  }
}

const handleSubmit = async () => {
  if (!form.value.customer_id || !form.value.start_date || !form.value.end_date) {
    ElMessage.warning('请填写必填字段')
    return
  }
  try {
    if (form.value.id) {
      await updateArReconciliation(form.value.id, form.value)
      ElMessage.success('更新成功')
    } else {
      await createArReconciliation(form.value)
      ElMessage.success('新增成功')
    }
    dialogVisible.value = false
    loadData()
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const handleDelete = async (row: ArReconciliationEntity) => {
  if (row.status === 'confirmed') {
    ElMessage.warning('已确认的对账不能删除')
    return
  }
  try {
    await ElMessageBox.confirm('确定要删除这个对账吗？', '提示', {
      type: 'warning'
    })
    await deleteArReconciliation(row.id!)
    ElMessage.success('删除成功')
    loadData()
  } catch (error) {
    ElMessage.info('取消删除')
  }
}

const handleConfirm = async (row: ArReconciliationEntity) => {
  try {
    await ElMessageBox.confirm('确定要确认这个对账吗？', '提示', {
      type: 'warning'
    })
    await confirmReconciliation(row.id!)
    ElMessage.success('确认成功')
    loadData()
  } catch (error) {
    ElMessage.info('取消操作')
  }
}

loadData()
loadCustomers()
</script>

<template>
  <div class="app-container">
    <div class="filter-container">
      <ElRow :gutter="20">
        <ElCol :span="6">
          <ElInput
            v-model="searchForm.customer_name"
            placeholder="客户名称"
            class="filter-item"
            @keyup.enter="handleSearch"
          />
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
        <ElCol :span="6">
          <ElDatePicker
            v-model="searchForm.start_date"
            type="date"
            placeholder="开始日期"
            class="filter-item"
          />
        </ElCol>
        <ElCol :span="6">
          <ElDatePicker
            v-model="searchForm.end_date"
            type="date"
            placeholder="结束日期"
            class="filter-item"
          />
        </ElCol>
      </ElRow>
      <div class="filter-actions">
        <ElButton type="primary" @click="handleSearch">查询</ElButton>
        <ElButton @click="handleReset">重置</ElButton>
        <ElButton type="success" @click="openAddDialog">
          <Plus /> 新增对账
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
      <ElTableColumn prop="customer_code" label="客户编码" width="120" />
      <ElTableColumn prop="customer_name" label="客户名称" width="150" />
      <ElTableColumn prop="start_date" label="对账起始" width="120" />
      <ElTableColumn prop="end_date" label="对账截止" width="120" />
      <ElTableColumn prop="total_invoice" label="发票金额" width="120" align="right">
        <template #default="scope">{{ scope.row.total_invoice.toFixed(2) }}</template>
      </ElTableColumn>
      <ElTableColumn prop="total_payment" label="回款金额" width="120" align="right">
        <template #default="scope">{{ scope.row.total_payment.toFixed(2) }}</template>
      </ElTableColumn>
      <ElTableColumn prop="balance" label="余额" width="120" align="right">
        <template #default="scope">{{ scope.row.balance.toFixed(2) }}</template>
      </ElTableColumn>
      <ElTableColumn prop="status" label="状态" width="100">
        <template #default="scope">
          <span :class="['status-tag', getStatusClass(scope.row.status)]">
            {{ getStatusLabel(scope.row.status) }}
          </span>
        </template>
      </ElTableColumn>
      <ElTableColumn prop="created_by_name" label="创建人" width="100" />
      <ElTableColumn prop="created_at" label="创建时间" width="150" />
      <ElTableColumn label="操作" width="250" align="center">
        <template #default="scope">
          <ElButton size="small" @click="openViewDialog(scope.row)">
            <View />
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'draft'"
            size="small"
            type="primary"
            @click="openEditDialog(scope.row)"
          >
            <Edit />
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'draft'"
            size="small"
            type="warning"
            @click="handleConfirm(scope.row)"
          >
            <Check /> 确认
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'draft'"
            size="small"
            type="danger"
            @click="handleDelete(scope.row)"
          >
            <Delete />
          </ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <ElDialog :title="dialogTitle" :visible="dialogVisible" width="500px" @close="dialogVisible = false">
      <ElForm :model="form" label-width="100px">
        <ElFormItem label="客户" prop="customer_id">
          <ElSelect v-model="form.customer_id" placeholder="请选择客户">
            <ElOption v-for="c in customerOptions" :key="c.value" :label="c.label" :value="c.value" />
          </ElSelect>
        </ElFormItem>
        <ElRow :gutter="20">
          <ElCol :span="12">
            <ElFormItem label="对账起始日期" prop="start_date">
              <ElDatePicker v-model="form.start_date" type="date" />
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="对账截止日期" prop="end_date">
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

    <ElDialog title="对账详情" :visible="viewDialogVisible" width="800px" @close="viewDialogVisible = false">
      <div v-if="viewData">
        <ElDescriptions :column="4" border>
          <ElDescriptionsItem label="客户编码">{{ viewData.customer_code }}</ElDescriptionsItem>
          <ElDescriptionsItem label="客户名称">{{ viewData.customer_name }}</ElDescriptionsItem>
          <ElDescriptionsItem label="对账起始">{{ viewData.start_date }}</ElDescriptionsItem>
          <ElDescriptionsItem label="对账截止">{{ viewData.end_date }}</ElDescriptionsItem>
          <ElDescriptionsItem label="发票金额">{{ viewData.total_invoice.toFixed(2) }}</ElDescriptionsItem>
          <ElDescriptionsItem label="回款金额">{{ viewData.total_payment.toFixed(2) }}</ElDescriptionsItem>
          <ElDescriptionsItem label="调整金额">{{ viewData.total_adjustment.toFixed(2) }}</ElDescriptionsItem>
          <ElDescriptionsItem label="余额">{{ viewData.balance.toFixed(2) }}</ElDescriptionsItem>
          <ElDescriptionsItem label="状态">{{ getStatusLabel(viewData.status) }}</ElDescriptionsItem>
          <ElDescriptionsItem label="创建人">{{ viewData.created_by_name }}</ElDescriptionsItem>
          <ElDescriptionsItem label="创建时间">{{ viewData.created_at }}</ElDescriptionsItem>
          <ElDescriptionsItem label="确认时间">{{ viewData.confirmed_at || '-' }}</ElDescriptionsItem>
        </ElDescriptions>

        <div style="margin-top: 20px">
          <h4>对账明细</h4>
          <ElTable :data="detailData" border style="width: 100%">
            <ElTableColumn prop="type" label="类型" width="100" />
            <ElTableColumn prop="source_no" label="单据号" width="150" />
            <ElTableColumn prop="source_date" label="日期" width="120" />
            <ElTableColumn prop="amount" label="金额" width="120" align="right">
              <template #default="scope">{{ scope.row.amount.toFixed(2) }}</template>
            </ElTableColumn>
            <ElTableColumn prop="paid_amount" label="已付金额" width="120" align="right">
              <template #default="scope">{{ scope.row.paid_amount.toFixed(2) }}</template>
            </ElTableColumn>
            <ElTableColumn prop="balance" label="余额" width="120" align="right">
              <template #default="scope">{{ scope.row.balance.toFixed(2) }}</template>
            </ElTableColumn>
            <ElTableColumn prop="remark" label="备注" />
          </ElTable>
        </div>
      </div>
    </ElDialog>
  </div>
</template>

<style scoped>
.app-container {
  padding: 20px;
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

.status-draft {
  background: #f5f7fa;
  color: #909399;
}

.status-confirmed {
  background: #f0f9eb;
  color: #67c23a;
}
</style>