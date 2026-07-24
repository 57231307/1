<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  ElTable,
  ElTableColumn,
  ElButton,
  ElDialog,
  ElForm,
  ElFormItem,
  ElInput,
  ElSelect,
  ElDatePicker,
  ElMessageBox,
  ElMessage,
  ElRow,
  ElCol,
  ElDescriptions,
} from 'element-plus'
import { Plus, Edit, Delete, View, Check } from '@element-plus/icons-vue'
import {
  getArReconciliation,
  createArReconciliation,
  updateArReconciliation,
  deleteArReconciliation,
  confirmReconciliation,
  getReconciliationDetails,
  type ArReconciliationEntity,
  type ReconciliationDetail,
} from '@/api/ar-reconciliation'
import { getCustomerSelectList } from '@/api/customer'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'

const { t } = useI18n({ useScope: 'global' })

const searchForm = ref({
  customer_name: '',
  status: '',
  start_date: '',
  end_date: '',
})

// 批次 272：接入 useTableApi，消除手写 pagination/tableData/total/loading + loadData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: tableData,
  loading,
  page,
  pageSize,
  total,
  refresh: loadData,
  setQueryParam,
} = useTableApi<ArReconciliationEntity>({
  url: '/ar-reconciliations',
  onError: (e: unknown) => {
    ElMessage.error(t('arReconciliationModule.index.loadFailed'))
    logger.warn(t('arReconciliationModule.index.loadListFailed'), String(e))
  },
})

const dialogVisible = ref(false)
const form = ref<Partial<ArReconciliationEntity>>({
  customer_id: 0,
  customer_name: '',
  start_date: '',
  end_date: '',
  status: 'draft',
})
// 对话框标题：依据 form.id 自动切换「新增 / 编辑」，computed 保证语言切换即时生效
const dialogTitle = computed(() =>
  form.value.id
    ? t('arReconciliationModule.index.editReconciliation')
    : t('arReconciliationModule.index.addReconciliation'),
)

const viewDialogVisible = ref(false)
const viewData = ref<ArReconciliationEntity | null>(null)
const detailData = ref<ReconciliationDetail[]>([])

const customerOptions = ref<{ label: string; value: number }[]>([])

// 状态下拉选项（label 走 i18n，computed 保证语言切换即时生效）
const statusOptions = computed(() => [
  { label: t('arReconciliationModule.index.statusAll'), value: '' },
  { label: t('arReconciliationModule.index.statusDraft'), value: 'draft' },
  { label: t('arReconciliationModule.index.statusConfirmed'), value: 'confirmed' },
])

const getStatusLabel = (value: string) => {
  return statusOptions.value.find(s => s.value === value)?.label || value
}

const getStatusClass = (value: string) => {
  return value === 'draft' ? 'status-draft' : 'status-confirmed'
}

// 批次 272：同步筛选条件到 useTableApi.queryParams 并刷新
// useTableApi 自动 watch page/pageSize 变化触发重载，无需手动 loadData
const syncQueryParams = () => {
  setQueryParam('customer_name', searchForm.value.customer_name || undefined)
  setQueryParam('status', searchForm.value.status || undefined)
  setQueryParam('start_date', searchForm.value.start_date || undefined)
  setQueryParam('end_date', searchForm.value.end_date || undefined)
}

const loadCustomers = async () => {
  try {
    // v11 批次 146 P1-4 修复：改用 customer.ts 统一封装的 getCustomerSelectList，
    // 避免绕过 API 层直接调用 request.get，并正确处理 PaginatedResponse → {label, value}[] 映射
    customerOptions.value = await getCustomerSelectList()
  } catch (error) {
    logger.warn(t('arReconciliationModule.index.loadCustomersFailed'))
  }
}

const handleSearch = () => {
  syncQueryParams()
  page.value = 1
  loadData()
}

const handleReset = () => {
  searchForm.value = {
    customer_name: '',
    status: '',
    start_date: '',
    end_date: '',
  }
  syncQueryParams()
  page.value = 1
  loadData()
}

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (val: number) => {
  page.value = val
}

const handlePageSizeChange = (val: number) => {
  pageSize.value = val
  page.value = 1
}

const openAddDialog = () => {
  form.value = {
    customer_id: 0,
    customer_name: '',
    start_date: '',
    end_date: '',
    status: 'draft',
  }
  dialogVisible.value = true
}

const openEditDialog = (row: ArReconciliationEntity) => {
  form.value = { ...row }
  dialogVisible.value = true
}

const openViewDialog = async (row: ArReconciliationEntity) => {
  try {
    // v11 批次 175 P2-1 修复：res: any 改为具体类型
    const res = (await getArReconciliation(row.id!)) as { data?: ArReconciliationEntity }
    // 安全检查：防止后端返回 data 为 null 时崩溃
    if (res.data) viewData.value = res.data
    const detailRes = (await getReconciliationDetails(row.id!)) as {
      data?: ReconciliationDetail[]
    }
    detailData.value = detailRes.data || []
    viewDialogVisible.value = true
  } catch (error) {
    ElMessage.error(t('arReconciliationModule.index.fetchDetailFailed'))
  }
}

const handleSubmit = async () => {
  if (!form.value.customer_id || !form.value.start_date || !form.value.end_date) {
    ElMessage.warning(t('arReconciliationModule.index.requiredFieldsMissing'))
    return
  }
  try {
    if (form.value.id) {
      await updateArReconciliation(form.value.id, form.value)
      ElMessage.success(t('common.message.updateSuccess'))
    } else {
      await createArReconciliation(form.value)
      ElMessage.success(t('common.message.createSuccess'))
    }
    dialogVisible.value = false
    loadData()
  } catch (error) {
    ElMessage.error(t('common.message.operationFailed'))
  }
}

const handleDelete = async (row: ArReconciliationEntity) => {
  if (row.status === 'confirmed') {
    ElMessage.warning(t('arReconciliationModule.index.cannotDeleteConfirmed'))
    return
  }
  try {
    await ElMessageBox.confirm(
      t('arReconciliationModule.index.deleteConfirm'),
      t('common.message.confirmTitle'),
      { type: 'warning' },
    )
    await deleteArReconciliation(row.id!)
    ElMessage.success(t('common.message.deleteSuccess'))
    loadData()
  } catch (error) {
    ElMessage.info(t('arReconciliationModule.index.deleteCancelled'))
  }
}

const handleConfirm = async (row: ArReconciliationEntity) => {
  try {
    await ElMessageBox.confirm(
      t('arReconciliationModule.index.confirmReconciliationConfirm'),
      t('common.message.confirmTitle'),
      { type: 'warning' },
    )
    await confirmReconciliation(row.id!)
    ElMessage.success(t('arReconciliationModule.index.confirmSuccess'))
    loadData()
  } catch (error) {
    ElMessage.info(t('arReconciliationModule.index.confirmCancelled'))
  }
}

// 批次 272：useTableApi 构造时自动初始加载，无需 setup 顶层调用 loadData
loadCustomers()
</script>

<template>
  <div class="app-container">
    <div class="filter-container">
      <ElRow :gutter="20">
        <ElCol :span="6">
          <ElInput
            v-model="searchForm.customer_name"
            :placeholder="$t('arReconciliationModule.index.customerNamePlaceholder')"
            class="filter-item"
            @keyup.enter="handleSearch"
          />
        </ElCol>
        <ElCol :span="6">
          <ElSelect
            v-model="searchForm.status"
            :placeholder="$t('arReconciliationModule.index.statusPlaceholder')"
            class="filter-item"
          >
            <ElOption v-for="s in statusOptions" :key="s.value" :label="s.label" :value="s.value" />
          </ElSelect>
        </ElCol>
        <ElCol :span="6">
          <ElDatePicker
            v-model="searchForm.start_date"
            type="date"
            :placeholder="$t('arReconciliationModule.index.startDatePlaceholder')"
            class="filter-item"
          />
        </ElCol>
        <ElCol :span="6">
          <ElDatePicker
            v-model="searchForm.end_date"
            type="date"
            :placeholder="$t('arReconciliationModule.index.endDatePlaceholder')"
            class="filter-item"
          />
        </ElCol>
      </ElRow>
      <div class="filter-actions">
        <ElButton type="primary" @click="handleSearch">{{ $t('arReconciliationModule.index.query') }}</ElButton>
        <ElButton @click="handleReset">{{ $t('common.reset') }}</ElButton>
        <ElButton type="success" @click="openAddDialog"> <Plus /> {{ $t('arReconciliationModule.index.addReconciliation') }} </ElButton>
      </div>
    </div>

    <ElTable
      :data="tableData"
      :loading="loading"
      border
      fit
      highlight-current-row
      style="width: 100%"
      :aria-label="$t('arReconciliationModule.index.listAria')"
    >
      <ElTableColumn prop="customer_code" :label="$t('arReconciliationModule.index.customerCode')" width="120" />
      <ElTableColumn prop="customer_name" :label="$t('arReconciliationModule.index.customerName')" width="150" />
      <ElTableColumn prop="start_date" :label="$t('arReconciliationModule.index.reconciliationStart')" width="120" />
      <ElTableColumn prop="end_date" :label="$t('arReconciliationModule.index.reconciliationEnd')" width="120" />
      <ElTableColumn prop="total_invoice" :label="$t('arReconciliationModule.index.invoiceAmount')" width="120" align="right">
        <template #default="scope">{{ scope.row.total_invoice.toFixed(2) }}</template>
      </ElTableColumn>
      <ElTableColumn prop="total_payment" :label="$t('arReconciliationModule.index.paymentAmount')" width="120" align="right">
        <template #default="scope">{{ scope.row.total_payment.toFixed(2) }}</template>
      </ElTableColumn>
      <ElTableColumn prop="balance" :label="$t('arReconciliationModule.index.balance')" width="120" align="right">
        <template #default="scope">{{ scope.row.balance.toFixed(2) }}</template>
      </ElTableColumn>
      <ElTableColumn prop="status" :label="$t('arReconciliationModule.index.status')" width="100">
        <template #default="scope">
          <span :class="['status-tag', getStatusClass(scope.row.status)]">
            {{ getStatusLabel(scope.row.status) }}
          </span>
        </template>
      </ElTableColumn>
      <ElTableColumn prop="created_by_name" :label="$t('arReconciliationModule.index.createdBy')" width="100" />
      <ElTableColumn prop="created_at" :label="$t('common.createTime')" width="150" />
      <ElTableColumn :label="$t('common.operation')" width="250" align="center">
        <template #default="scope">
          <ElButton size="small" @click="openViewDialog(scope.row as ArReconciliationEntity)">
            <View />
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'draft'"
            size="small"
            type="primary"
            @click="openEditDialog(scope.row as ArReconciliationEntity)"
          >
            <Edit />
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'draft'"
            size="small"
            type="warning"
            @click="handleConfirm(scope.row as ArReconciliationEntity)"
          >
            <Check /> {{ $t('arReconciliationModule.index.confirm') }}
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'draft'"
            size="small"
            type="danger"
            @click="handleDelete(scope.row as ArReconciliationEntity)"
          >
            <Delete />
          </ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <div class="pagination-wrapper" style="margin-top: 16px; text-align: right">
      <ElPagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="handlePageSizeChange"
        @current-change="handlePageChange"
        :aria-label="$t('arReconciliationModule.index.paginationAria')"
      />
    </div>

    <ElDialog
      :title="dialogTitle"
      :visible="dialogVisible"
      width="500px"
      :aria-label="dialogTitle"
      @close="dialogVisible = false"
    >
      <ElForm :model="form" label-width="100px" :aria-label="$t('arReconciliationModule.index.formAria')">
        <ElFormItem :label="$t('arReconciliationModule.index.customer')" prop="customer_id">
          <ElSelect v-model="form.customer_id" :placeholder="$t('arReconciliationModule.index.selectCustomerPlaceholder')">
            <ElOption
              v-for="c in customerOptions"
              :key="c.value"
              :label="c.label"
              :value="c.value"
            />
          </ElSelect>
        </ElFormItem>
        <ElRow :gutter="20">
          <ElCol :span="12">
            <ElFormItem :label="$t('arReconciliationModule.index.startDateLabel')" prop="start_date">
              <ElDatePicker v-model="form.start_date" type="date" />
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem :label="$t('arReconciliationModule.index.endDateLabel')" prop="end_date">
              <ElDatePicker v-model="form.end_date" type="date" />
            </ElFormItem>
          </ElCol>
        </ElRow>
      </ElForm>
      <template #footer>
        <ElButton @click="dialogVisible = false">{{ $t('common.cancel') }}</ElButton>
        <ElButton type="primary" @click="handleSubmit">{{ $t('common.confirm') }}</ElButton>
      </template>
    </ElDialog>

    <ElDialog
      :title="$t('arReconciliationModule.index.detailTitle')"
      :visible="viewDialogVisible"
      width="800px"
      :aria-label="$t('arReconciliationModule.index.detailTitle')"
      @close="viewDialogVisible = false"
    >
      <div v-if="viewData">
        <ElDescriptions :column="4" border>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.customerCode')">{{ viewData.customer_code }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.customerName')">{{ viewData.customer_name }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.reconciliationStart')">{{ viewData.start_date }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.reconciliationEnd')">{{ viewData.end_date }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.invoiceAmount')">{{
            viewData.total_invoice.toFixed(2)
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.paymentAmount')">{{
            viewData.total_payment.toFixed(2)
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.adjustmentAmount')">{{
            viewData.total_adjustment.toFixed(2)
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.balance')">{{ viewData.balance.toFixed(2) }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.status')">{{
            getStatusLabel(viewData.status)
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.createdBy')">{{ viewData.created_by_name }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('common.createTime')">{{ viewData.created_at }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.confirmedAt')">{{
            viewData.confirmed_at || '-'
          }}</ElDescriptionsItem>
        </ElDescriptions>

        <div style="margin-top: 20px">
          <h4>{{ $t('arReconciliationModule.index.detailItems') }}</h4>
          <ElTable :data="detailData" border style="width: 100%" :aria-label="$t('arReconciliationModule.index.detailItemsAria')">
            <ElTableColumn prop="type" :label="$t('arReconciliationModule.index.type')" width="100" />
            <ElTableColumn prop="source_no" :label="$t('arReconciliationModule.index.sourceNo')" width="150" />
            <ElTableColumn prop="source_date" :label="$t('arReconciliationModule.index.date')" width="120" />
            <ElTableColumn prop="amount" :label="$t('arReconciliationModule.index.amount')" width="120" align="right">
              <template #default="scope">{{ scope.row.amount.toFixed(2) }}</template>
            </ElTableColumn>
            <ElTableColumn prop="paid_amount" :label="$t('arReconciliationModule.index.paidAmount')" width="120" align="right">
              <template #default="scope">{{ scope.row.paid_amount.toFixed(2) }}</template>
            </ElTableColumn>
            <ElTableColumn prop="balance" :label="$t('arReconciliationModule.index.balance')" width="120" align="right">
              <template #default="scope">{{ scope.row.balance.toFixed(2) }}</template>
            </ElTableColumn>
            <ElTableColumn prop="remark" :label="$t('common.description')" />
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
