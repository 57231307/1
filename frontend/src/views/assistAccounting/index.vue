<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  ElTable,
  ElTableColumn,
  ElButton,
  ElDialog,
  ElInput,
  ElSelect,
  ElOption,
  ElDatePicker,
  ElMessage,
  ElRow,
  ElCol,
  ElDescriptions,
  ElDescriptionsItem,
  ElTabs,
  ElTabPane,
  ElPagination,
} from 'element-plus'
import { View, Refresh } from '@element-plus/icons-vue'
import {
  listAssistDimensions,
  getAssistSummary,
  type AssistDimensionResponse,
  type AssistRecordResponse,
  type AssistSummaryResponse,
} from '@/api/assist-accounting'
import { useTableApi } from '@/composables/useTableApi'
import { logger } from '@/utils/logger'

const activeTab = ref('records')
const dimensions = ref<AssistDimensionResponse[]>([])

// 批次 390：records tab 接入 useTableApi，修复 0-based 分页 bug
// 原代码第 98 行 page: pagination.value.page - 1 为 0-based 分页，
// 与后端 page.unwrap_or(1).clamp(1,1000) + page.saturating_sub(1)*page_size 约定不一致。
// useTableApi 使用 1-based 分页，直接传 page 给后端，无需 -1 转换。
// 后端返回 { data: { records: [], total: 0 } }，需配置 listKey: 'records'。
const {
  data: tableData,
  total,
  loading: recordsLoading,
  page,
  pageSize,
  queryParams,
  refresh: loadRecords,
} = useTableApi<AssistRecordResponse>({
  url: '/assist-accounting/records',
  listKey: 'records',
  defaultPageSize: 20,
  defaultParams: {
    accounting_period: '',
    dimension_code: '',
    business_type: '',
    warehouse_id: '',
  },
  onError: (err: unknown) => {
    logger.error('获取辅助核算记录失败', err)
    ElMessage.error('获取辅助核算记录失败')
  },
})

// summary tab 保持独立 loading 和数据管理（不分页，不接入 useTableApi）
const summaryLoading = ref(false)
const summaryData = ref<AssistSummaryResponse[]>([])

// searchForm 用于表单双向绑定，搜索时同步到 useTableApi queryParams
const searchForm = ref({
  accounting_period: '',
  dimension_code: '',
  business_type: '',
  warehouse_id: '',
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
  { label: '生产产出', value: 'PRODUCTION_OUTPUT' },
]

const getBusinessTypeLabel = (value: string) => {
  return businessTypeOptions.find(b => b.value === value)?.label || value
}

const loadDimensions = async () => {
  try {
    // v11 批次 175 P2-1 修复：res: any 和 res.data as any 改为具体类型
    const res = (await listAssistDimensions()) as {
      data?: AssistDimensionResponse[] | { data?: AssistDimensionResponse[]; items?: AssistDimensionResponse[] }
    }
    const d = res.data
    if (Array.isArray(d)) {
      dimensions.value = d
    } else if (d && typeof d === 'object') {
      const obj = d as { data?: AssistDimensionResponse[]; items?: AssistDimensionResponse[] }
      dimensions.value = obj?.data || obj?.items || []
    } else {
      dimensions.value = []
    }
  } catch (error) {
    ElMessage.error('加载维度失败')
  }
}

const loadSummary = async () => {
  summaryLoading.value = true
  try {
    const period = searchForm.value.accounting_period || new Date().toISOString().slice(0, 7)
    // v11 批次 175 P2-1 修复：res: any 和 res.data as any 改为具体类型
    const res = (await getAssistSummary({
      accounting_period: period,
      dimension_code: searchForm.value.dimension_code || undefined,
    })) as {
      data?: AssistSummaryResponse[] | { data?: AssistSummaryResponse[]; items?: AssistSummaryResponse[] }
    }
    const d = res.data
    if (Array.isArray(d)) {
      summaryData.value = d
    } else if (d && typeof d === 'object') {
      const obj = d as { data?: AssistSummaryResponse[]; items?: AssistSummaryResponse[] }
      summaryData.value = obj?.data || obj?.items || []
    } else {
      summaryData.value = []
    }
  } catch (error) {
    ElMessage.error('加载汇总失败')
  } finally {
    summaryLoading.value = false
  }
}

// 批次 390：handleSearch 同步 searchForm 到 useTableApi queryParams
// useTableApi watch 只监听 page/pageSize 变化，不监听 queryParams，
// 所以修改 queryParams 后需要手动调 loadRecords 确保加载
const handleSearch = () => {
  if (activeTab.value === 'records') {
    // 同步搜索条件到 useTableApi queryParams
    queryParams.value = {
      accounting_period: searchForm.value.accounting_period || undefined,
      dimension_code: searchForm.value.dimension_code || undefined,
      business_type: searchForm.value.business_type || undefined,
      warehouse_id: searchForm.value.warehouse_id
        ? Number(searchForm.value.warehouse_id)
        : undefined,
    }
    page.value = 1
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
    warehouse_id: '',
  }
  handleSearch()
}

const openViewDialog = (row: AssistRecordResponse) => {
  viewData.value = row
  viewDialogVisible.value = true
}

const handleTabChange = () => {
  handleSearch()
}

// 批次 390：维度数据在 setup 阶段加载，records 由 useTableApi setup 自动加载
loadDimensions()
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
          <ElSelect v-model="searchForm.dimension_code" placeholder="核算维度" class="filter-item">
            <ElOption label="全部" value="" />
            <ElOption
              v-for="d in dimensionOptions"
              :key="d.value"
              :label="d.label"
              :value="d.value"
            />
          </ElSelect>
        </ElCol>
        <ElCol :span="6">
          <ElSelect v-model="searchForm.business_type" placeholder="业务类型" class="filter-item">
            <ElOption
              v-for="b in businessTypeOptions"
              :key="b.value"
              :label="b.label"
              :value="b.value"
            />
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
          :loading="recordsLoading"
          border
          fit
          highlight-current-row
          style="width: 100%"
          aria-label="辅助核算记录列表"
        >
          <ElTableColumn prop="id" label="ID" width="80" />
          <ElTableColumn prop="business_type" label="业务类型" width="120">
            <template #default="scope">{{
              getBusinessTypeLabel(scope.row.business_type)
            }}</template>
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
              <ElButton size="small" @click="openViewDialog(scope.row as AssistRecordResponse)">
                <View />
              </ElButton>
            </template>
          </ElTableColumn>
        </ElTable>

        <!-- 批次 390：分页由 useTableApi watch 自动加载，v-model 双向绑定 page/pageSize -->
        <div class="pagination-wrapper" style="margin-top: 16px; text-align: right">
          <ElPagination
            v-model:current-page="page"
            v-model:page-size="pageSize"
            :page-sizes="[10, 20, 50, 100]"
            :total="total"
            layout="total, sizes, prev, pager, next, jumper"
            aria-label="辅助核算记录列表分页"
          />
        </div>
      </ElTabPane>

      <ElTabPane label="辅助核算汇总" name="summary">
        <ElTable
          :data="summaryData"
          :loading="summaryLoading"
          border
          fit
          highlight-current-row
          style="width: 100%"
          aria-label="辅助核算汇总列表"
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

    <ElDialog
      title="辅助核算记录详情"
      :visible="viewDialogVisible"
      width="800px"
      aria-label="辅助核算记录详情"
      @close="viewDialogVisible = false"
    >
      <div v-if="viewData">
        <ElDescriptions :column="3" border>
          <ElDescriptionsItem label="ID">{{ viewData.id }}</ElDescriptionsItem>
          <ElDescriptionsItem label="业务类型">{{
            getBusinessTypeLabel(viewData.business_type)
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="业务单号">{{ viewData.business_no }}</ElDescriptionsItem>
          <ElDescriptionsItem label="业务ID">{{ viewData.business_id }}</ElDescriptionsItem>
          <ElDescriptionsItem label="会计科目ID">{{
            viewData.account_subject_id
          }}</ElDescriptionsItem>
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
          <ElDescriptionsItem label="供应商ID">{{
            viewData.supplier_id || '-'
          }}</ElDescriptionsItem>
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
