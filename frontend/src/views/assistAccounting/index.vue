<script setup lang="ts">
import { ref, computed } from 'vue'
import { ElTable, ElTableColumn, ElButton, ElDialog, ElInput, ElSelect, ElDatePicker, ElMessage, ElRow, ElCol, ElDescriptions, ElTabs, ElTabPane } from 'element-plus'
import { View } from '@element-plus/icons-vue'
import { listAssistDimensions, queryAssistRecords, getAssistSummary, type AssistDimensionResponse, type AssistRecordResponse, type AssistSummaryResponse } from '@/api/assist-accounting'

const activeTab = ref('records')
const dimensions = ref<AssistDimensionResponse[]>([])
const tableData = ref<AssistRecordResponse[]>([])
const summaryData = ref<AssistSummaryResponse[]>([])
const total = ref(0)
const loading = ref(false)

const searchForm = ref({
  accounting_period: '',
  dimension_code: '',
  business_type: '',
  warehouse_id: ''
})

const pagination = ref({
  page: 1,
  pageSize: 20
})

const viewDialogVisible = ref(false)
const viewData = ref<AssistRecordResponse | null>(null)

const dimensionOptions = computed(() => {
  return dimensions.value.map(d => ({ label: d.dimension_name, value: d.dimension_code }))
})

const businessTypeOptions = [
  { label: '全部', value: '' },
  { label: '采购入库', value: 'PURCHASE_RECEIPT' },
  { label: '销售出库', value: 'SALES_DELIVERY' },
  { label: '库存调整', value: 'INVENTORY_ADJUSTMENT' },
  { label: '生产投入', value: 'PRODUCTION_INPUT' },
  { label: '生产产出', value: 'PRODUCTION_OUTPUT' }
]

const getBusinessTypeLabel = (value: string) => {
  return businessTypeOptions.find(b => b.value === value)?.label || value
}

const loadDimensions = async () => {
  try {
    const res = await listAssistDimensions()
    dimensions.value = res.data.data
  } catch (error) {
    ElMessage.error('加载维度失败')
  }
}

const loadRecords = async () => {
  loading.value = true
  try {
    const res = await queryAssistRecords({
      accounting_period: searchForm.value.accounting_period || undefined,
      dimension_code: searchForm.value.dimension_code || undefined,
      business_type: searchForm.value.business_type || undefined,
      warehouse_id: searchForm.value.warehouse_id ? Number(searchForm.value.warehouse_id) : undefined,
      page: pagination.value.page - 1,
      page_size: pagination.value.pageSize
    })
    tableData.value = res.data.records
    total.value = res.data.total
  } catch (error) {
    ElMessage.error('加载记录失败')
  } finally {
    loading.value = false
  }
}

const loadSummary = async () => {
  loading.value = true
  try {
    const period = searchForm.value.accounting_period || new Date().toISOString().slice(0, 7)
    const res = await getAssistSummary({
      accounting_period: period,
      dimension_code: searchForm.value.dimension_code || undefined
    })
    summaryData.value = res.data.data
  } catch (error) {
    ElMessage.error('加载汇总失败')
  } finally {
    loading.value = false
  }
}

const handleSearch = () => {
  pagination.value.page = 1
  if (activeTab.value === 'records') {
    loadRecords()
  } else {
    loadSummary()
  }
}

const handleReset = () => {
  searchForm.value = {
    accounting_period: '',
    dimension_code: '',
    business_type: '',
    warehouse_id: ''
  }
  handleSearch()
}

const handlePageChange = (page: number) => {
  pagination.value.page = page
  loadRecords()
}

const handlePageSizeChange = (pageSize: number) => {
  pagination.value.pageSize = pageSize
  loadRecords()
}

const openViewDialog = (row: AssistRecordResponse) => {
  viewData.value = row
  viewDialogVisible.value = true
}

const handleTabChange = () => {
  handleSearch()
}

loadDimensions()
loadRecords()
</script>

<template>
  <div class="app-container">
    <div class="filter-container">
      <ElRow :gutter="20">
        <ElCol :span="6">
          <ElDatePicker
            v-model="searchForm.accounting_period"
            type="month"
            placeholder="会计期间"
            class="filter-item"
          />
        </ElCol>
        <ElCol :span="6">
          <ElSelect
            v-model="searchForm.dimension_code"
            placeholder="核算维度"
            class="filter-item"
          >
            <ElOption label="全部" value="" />
            <ElOption v-for="d in dimensionOptions" :key="d.value" :label="d.label" :value="d.value" />
          </ElSelect>
        </ElCol>
        <ElCol :span="6">
          <ElSelect
            v-model="searchForm.business_type"
            placeholder="业务类型"
            class="filter-item"
          >
            <ElOption v-for="b in businessTypeOptions" :key="b.value" :label="b.label" :value="b.value" />
          </ElSelect>
        </ElCol>
        <ElCol :span="6">
          <ElInput
            v-model="searchForm.warehouse_id"
            placeholder="仓库ID"
            class="filter-item"
            @keyup.enter="handleSearch"
          />
        </ElCol>
      </ElRow>
      <div class="filter-actions">
        <ElButton type="primary" @click="handleSearch">查询</ElButton>
        <ElButton @click="handleReset">重置</ElButton>
        <ElButton @click="activeTab === 'records' ? loadRecords() : loadSummary()">
          <Refresh /> 刷新
        </ElButton>
      </div>
    </div>

    <ElTabs v-model="activeTab" @tab-change="handleTabChange">
      <ElTabPane label="辅助核算记录" name="records">
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
          <ElTableColumn prop="id" label="ID" width="80" />
          <ElTableColumn prop="business_type" label="业务类型" width="120">
            <template #default="scope">{{ getBusinessTypeLabel(scope.row.business_type) }}</template>
          </ElTableColumn>
          <ElTableColumn prop="business_no" label="业务单号" width="150" />
          <ElTableColumn prop="batch_no" label="批次号" width="120" />
          <ElTableColumn prop="color_no" label="色号" width="100" />
          <ElTableColumn prop="grade" label="等级" width="100" />
          <ElTableColumn prop="warehouse_id" label="仓库ID" width="100" />
          <ElTableColumn prop="quantity_meters" label="米数" width="120" align="right" />
          <ElTableColumn prop="quantity_kg" label="公斤数" width="120" align="right" />
          <ElTableColumn prop="debit_amount" label="借方金额" width="120" align="right" />
          <ElTableColumn prop="credit_amount" label="贷方金额" width="120" align="right" />
          <ElTableColumn prop="created_at" label="创建时间" width="150" />
          <ElTableColumn label="操作" width="100" align="center">
            <template #default="scope">
              <ElButton size="small" @click="openViewDialog(scope.row)">
                <View />
              </ElButton>
            </template>
          </ElTableColumn>
        </ElTable>
      </ElTabPane>

      <ElTabPane label="辅助核算汇总" name="summary">
        <ElTable
          :data="summaryData"
          :loading="loading"
          border
          fit
          highlight-current-row
          style="width: 100%"
        >
          <ElTableColumn prop="id" label="ID" width="80" />
          <ElTableColumn prop="accounting_period" label="会计期间" width="120" />
          <ElTableColumn prop="dimension_code" label="维度编码" width="120" />
          <ElTableColumn prop="dimension_value_name" label="维度值" width="150" />
          <ElTableColumn prop="total_debit" label="借方合计" width="120" align="right" />
          <ElTableColumn prop="total_credit" label="贷方合计" width="120" align="right" />
          <ElTableColumn prop="total_quantity_meters" label="总米数" width="120" align="right" />
          <ElTableColumn prop="total_quantity_kg" label="总公斤数" width="120" align="right" />
          <ElTableColumn prop="record_count" label="记录数" width="100" align="center" />
        </ElTable>
      </ElTabPane>
    </ElTabs>

    <ElDialog title="辅助核算记录详情" :visible="viewDialogVisible" width="800px" @close="viewDialogVisible = false">
      <div v-if="viewData">
        <ElDescriptions :column="3" border>
          <ElDescriptionsItem label="ID">{{ viewData.id }}</ElDescriptionsItem>
          <ElDescriptionsItem label="业务类型">{{ getBusinessTypeLabel(viewData.business_type) }}</ElDescriptionsItem>
          <ElDescriptionsItem label="业务单号">{{ viewData.business_no }}</ElDescriptionsItem>
          <ElDescriptionsItem label="业务ID">{{ viewData.business_id }}</ElDescriptionsItem>
          <ElDescriptionsItem label="会计科目ID">{{ viewData.account_subject_id }}</ElDescriptionsItem>
          <ElDescriptionsItem label="五维ID">{{ viewData.five_dimension_id }}</ElDescriptionsItem>
          <ElDescriptionsItem label="产品ID">{{ viewData.product_id }}</ElDescriptionsItem>
          <ElDescriptionsItem label="批次号">{{ viewData.batch_no }}</ElDescriptionsItem>
          <ElDescriptionsItem label="色号">{{ viewData.color_no }}</ElDescriptionsItem>
          <ElDescriptionsItem label="染缸号">{{ viewData.dye_lot_no || '-' }}</ElDescriptionsItem>
          <ElDescriptionsItem label="等级">{{ viewData.grade }}</ElDescriptionsItem>
          <ElDescriptionsItem label="仓库ID">{{ viewData.warehouse_id }}</ElDescriptionsItem>
          <ElDescriptionsItem label="米数">{{ viewData.quantity_meters }}</ElDescriptionsItem>
          <ElDescriptionsItem label="公斤数">{{ viewData.quantity_kg }}</ElDescriptionsItem>
          <ElDescriptionsItem label="借方金额">{{ viewData.debit_amount }}</ElDescriptionsItem>
          <ElDescriptionsItem label="贷方金额">{{ viewData.credit_amount }}</ElDescriptionsItem>
          <ElDescriptionsItem label="车间ID">{{ viewData.workshop_id || '-' }}</ElDescriptionsItem>
          <ElDescriptionsItem label="客户ID">{{ viewData.customer_id || '-' }}</ElDescriptionsItem>
          <ElDescriptionsItem label="供应商ID">{{ viewData.supplier_id || '-' }}</ElDescriptionsItem>
          <ElDescriptionsItem label="备注">{{ viewData.remarks || '-' }}</ElDescriptionsItem>
          <ElDescriptionsItem label="创建时间">{{ viewData.created_at }}</ElDescriptionsItem>
        </ElDescriptions>
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
</style>