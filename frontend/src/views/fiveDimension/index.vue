<script setup lang="ts">
import { ref, reactive } from 'vue'
import {
  ElTable,
  ElTableColumn,
  ElButton,
  ElDialog,
  ElForm,
  ElFormItem,
  ElInput,
  ElSelect,
  ElMessage,
  ElRow,
  ElCol,
  ElDescriptions,
  ElCard,
  ElDivider,
} from 'element-plus'
import { Search, View, Refresh, Key } from '@element-plus/icons-vue'
import {
  listFiveDimensionStats,
  getStatsByFiveDimensionId,
  parseFiveDimensionId,
  searchFiveDimension,
  type FiveDimensionStatsResponse,
  type FiveDimensionItem,
} from '@/api/five-dimension'

const tableData = ref<FiveDimensionStatsResponse[]>([])
const total = ref(0)
const loading = ref(false)
const searchForm = ref({
  product_id: '',
  batch_no: '',
  color_no: '',
  grade: '',
})
const pagination = ref({
  page: 1,
  pageSize: 20,
})

const viewDialogVisible = ref(false)
const viewData = ref<FiveDimensionStatsResponse | null>(null)

const parseInput = ref('')
const parseResult = ref<FiveDimensionItem | null>(null)
const parseError = ref('')

const searchDialogVisible = ref(false)
const searchKeyword = ref('')
const searchType = ref('product')
const searchResults = ref<FiveDimensionItem[]>([])
const searchFormRef = reactive({ keyword: '' })

const gradeOptions = [
  { label: '一等品', value: '一等品' },
  { label: '二等品', value: '二等品' },
  { label: '三等品', value: '三等品' },
  { label: '次品', value: '次品' },
]

const searchTypeOptions = [
  { label: '产品', value: 'product' },
  { label: '批次', value: 'batch' },
  { label: '色号', value: 'color' },
  { label: '染缸号', value: 'dye_lot' },
  { label: '等级', value: 'grade' },
]

const loadData = async () => {
  loading.value = true
  try {
    const res: any = await listFiveDimensionStats({
      page: pagination.value.page - 1,
      page_size: pagination.value.pageSize,
      product_id: searchForm.value.product_id ? Number(searchForm.value.product_id) : undefined,
      batch_no: searchForm.value.batch_no || undefined,
      color_no: searchForm.value.color_no || undefined,
      grade: searchForm.value.grade || undefined,
    })
    tableData.value = res.data!.items
    total.value = res.data!.total
  } catch (error) {
    ElMessage.error('加载失败')
  } finally {
    loading.value = false
  }
}

const handleSearch = () => {
  pagination.value.page = 1
  loadData()
}

const handleReset = () => {
  searchForm.value = {
    product_id: '',
    batch_no: '',
    color_no: '',
    grade: '',
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

const openViewDialog = async (item: FiveDimensionStatsResponse) => {
  try {
    const res: any = await getStatsByFiveDimensionId(item.dimension.five_dimension_id)
    viewData.value = res.data!
    viewDialogVisible.value = true
  } catch (error) {
    ElMessage.error('获取详情失败')
  }
}

const handleParse = async () => {
  if (!parseInput.value.trim()) {
    ElMessage.warning('请输入五维ID')
    return
  }
  try {
    const res: any = await parseFiveDimensionId(parseInput.value)
    if (res.data.success) {
      parseResult.value = res.data.dimension
      parseError.value = ''
    } else {
      parseResult.value = null
      parseError.value = res.data.error || '解析失败'
    }
  } catch (error) {
    parseError.value = '解析失败'
    parseResult.value = null
  }
}

const handleQuickSearch = async () => {
  if (!searchKeyword.value.trim()) {
    ElMessage.warning('请输入搜索关键词')
    return
  }
  try {
    const res: any = await searchFiveDimension({
      keyword: searchKeyword.value,
      search_type: searchType.value,
      page: 0,
      page_size: 50,
    })
    searchResults.value = res.data!.items
  } catch (error) {
    ElMessage.error('搜索失败')
  }
}

const selectFromSearch = (item: FiveDimensionItem) => {
  searchForm.value.batch_no = item.batch_no || ''
  searchForm.value.color_no = item.color_no || ''
  searchForm.value.grade = item.grade || ''
  searchDialogVisible.value = false
  handleSearch()
}

loadData()
</script>

<template>
  <div class="app-container">
    <div class="filter-container">
      <ElCard title="五维解析" class="parse-card">
        <ElRow :gutter="20">
          <ElCol :span="12">
            <ElInput
              v-model="parseInput"
              placeholder="输入五维ID进行解析（如：P1|B20240101|C001|D20240101001|G 一等品）"
              class="filter-item"
            />
          </ElCol>
          <ElCol :span="4">
            <ElButton type="primary" class="w-full" @click="handleParse"> <Key /> 解析 </ElButton>
          </ElCol>
          <ElCol :span="4">
            <ElButton type="success" class="w-full" @click="searchDialogVisible = true">
              <Search /> 快速搜索
            </ElButton>
          </ElCol>
        </ElRow>
        <div v-if="parseResult" class="parse-result">
          <ElDivider />
          <ElDescriptions :column="5" border>
            <ElDescriptionsItem label="产品ID">{{ parseResult.product_id }}</ElDescriptionsItem>
            <ElDescriptionsItem label="批次号">{{ parseResult.batch_no }}</ElDescriptionsItem>
            <ElDescriptionsItem label="色号">{{ parseResult.color_no }}</ElDescriptionsItem>
            <ElDescriptionsItem label="染缸号">{{
              parseResult.dye_lot_no || '-'
            }}</ElDescriptionsItem>
            <ElDescriptionsItem label="等级">{{ parseResult.grade }}</ElDescriptionsItem>
          </ElDescriptions>
        </div>
        <div v-if="parseError" class="parse-error">
          <ElDivider />
          <span class="error-text">{{ parseError }}</span>
        </div>
      </ElCard>

      <ElRow :gutter="20" style="margin-top: 20px">
        <ElCol :span="6">
          <ElInput
            v-model="searchForm.product_id"
            placeholder="产品ID"
            class="filter-item"
            @keyup.enter="handleSearch"
          />
        </ElCol>
        <ElCol :span="6">
          <ElInput
            v-model="searchForm.batch_no"
            placeholder="批次号"
            class="filter-item"
            @keyup.enter="handleSearch"
          />
        </ElCol>
        <ElCol :span="6">
          <ElInput
            v-model="searchForm.color_no"
            placeholder="色号"
            class="filter-item"
            @keyup.enter="handleSearch"
          />
        </ElCol>
        <ElCol :span="6">
          <ElSelect v-model="searchForm.grade" placeholder="等级" class="filter-item">
            <ElOption label="全部" value="" />
            <ElOption v-for="g in gradeOptions" :key="g.value" :label="g.label" :value="g.value" />
          </ElSelect>
        </ElCol>
      </ElRow>
      <div class="filter-actions">
        <ElButton type="primary" @click="handleSearch">查询</ElButton>
        <ElButton @click="handleReset">重置</ElButton>
        <ElButton @click="loadData"> <Refresh /> 刷新 </ElButton>
      </div>
    </div>

    <ElTable
    :data="tableData"
      :loading="loading"
      border
      fit
      highlight-current-row
      style="width: 100%"
    >
      <ElTableColumn prop="dimension.product_id" label="产品ID" width="100" />
      <ElTableColumn prop="dimension.product_name" label="产品名称" width="150" />
      <ElTableColumn prop="dimension.batch_no" label="批次号" width="120" />
      <ElTableColumn prop="dimension.color_no" label="色号" width="100" />
      <ElTableColumn prop="dimension.dye_lot_no" label="染缸号" width="120" />
      <ElTableColumn prop="dimension.grade" label="等级" width="100" />
      <ElTableColumn prop="total_meters" label="总米数" width="120" align="right">
        <template #default="scope">{{ scope.row.total_meters }}</template>
      </ElTableColumn>
      <ElTableColumn prop="total_kg" label="总公斤数" width="120" align="right">
        <template #default="scope">{{ scope.row.total_kg }}</template>
      </ElTableColumn>
      <ElTableColumn prop="stock_count" label="库存记录数" width="120" align="center" />
      <ElTableColumn prop="dimension.five_dimension_id" label="五维ID" />
      <ElTableColumn label="操作" width="100" align="center">
        <template #default="scope">
          <ElButton size="small" @click="openViewDialog(scope.row as any)">
            <View />
          </ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <div class="pagination-wrapper" style="margin-top: 16px; text-align: right;">
      <ElPagination
        v-model:current-page="pagination.page"
        v-model:page-size="pagination.pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="handlePageSizeChange"
        @current-change="handlePageChange"
      />
    </div>

    <ElDialog
      title="五维统计详情"
      :visible="viewDialogVisible"
      width="800px"
      @close="viewDialogVisible = false"
    >
      <div v-if="viewData">
        <ElDescriptions :column="3" border>
          <ElDescriptionsItem label="产品ID">{{
            viewData.dimension.product_id
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="产品名称">{{
            viewData.dimension.product_name || '-'
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="批次号">{{ viewData.dimension.batch_no }}</ElDescriptionsItem>
          <ElDescriptionsItem label="色号">{{ viewData.dimension.color_no }}</ElDescriptionsItem>
          <ElDescriptionsItem label="染缸号">{{
            viewData.dimension.dye_lot_no || '-'
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="等级">{{ viewData.dimension.grade }}</ElDescriptionsItem>
          <ElDescriptionsItem label="总米数">{{ viewData.total_meters }}</ElDescriptionsItem>
          <ElDescriptionsItem label="总公斤数">{{ viewData.total_kg }}</ElDescriptionsItem>
          <ElDescriptionsItem label="库存记录数">{{ viewData.stock_count }}</ElDescriptionsItem>
        </ElDescriptions>
        <div style="margin-top: 20px">
          <h4>仓库分布</h4>
          <ElTable :data="viewData.warehouse_distribution" border style="width: 100%">
            <ElTableColumn prop="warehouse_id" label="仓库ID" width="100" />
            <ElTableColumn prop="warehouse_name" label="仓库名称" width="150" />
            <ElTableColumn prop="quantity_meters" label="米数" width="120" align="right" />
            <ElTableColumn prop="quantity_kg" label="公斤数" width="120" align="right" />
          </ElTable>
        </div>
      </div>
    </ElDialog>

    <ElDialog
      title="快速搜索"
      :visible="searchDialogVisible"
      width="700px"
      @close="searchDialogVisible = false"
    >
      <ElForm :model="searchFormRef" label-width="80px">
        <ElFormItem label="搜索关键词">
          <ElInput v-model="searchFormRef.keyword" placeholder="请输入搜索关键词" />
        </ElFormItem>
        <ElFormItem label="搜索类型">
          <ElSelect v-model="searchType">
            <ElOption
              v-for="t in searchTypeOptions"
              :key="t.value"
              :label="t.label"
              :value="t.value"
            />
          </ElSelect>
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="searchDialogVisible = false">取消</ElButton>
        <ElButton type="primary" @click="handleQuickSearch">搜索</ElButton>
      </template>
      <div v-if="searchResults.length > 0" style="margin-top: 10px">
        <ElDivider />
        <ElTable :data="searchResults" border style="width: 100%" size="small">
          <ElTableColumn prop="product_id" label="产品ID" width="80" />
          <ElTableColumn prop="product_name" label="产品名称" width="120" />
          <ElTableColumn prop="batch_no" label="批次号" width="120" />
          <ElTableColumn prop="color_no" label="色号" width="80" />
          <ElTableColumn prop="dye_lot_no" label="染缸号" width="120" />
          <ElTableColumn prop="grade" label="等级" width="80" />
          <ElTableColumn label="操作" width="80">
            <template #default="scope">
              <ElButton size="small" type="primary" @click="selectFromSearch(scope.row as any)"
                >选择</ElButton
              >
            </template>
          </ElTableColumn>
        </ElTable>
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

.parse-card {
  margin-bottom: 20px;
}

.parse-result {
  margin-top: 15px;
}

.parse-error {
  margin-top: 15px;
}

.error-text {
  color: #f56c6c;
}

.w-full {
  width: 100%;
}
</style>
