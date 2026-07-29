<script setup lang="ts">
import { ref, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
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
} from 'element-plus';
import { Search, View, Refresh, Key } from '@element-plus/icons-vue';
import {
  getStatsByFiveDimensionId,
  parseFiveDimensionId,
  searchFiveDimension,
  type FiveDimensionStatsResponse,
  type FiveDimensionItem,
} from '@/api/five-dimension';
import { useTableApi } from '@/composables/useTableApi';

const { t } = useI18n({ useScope: 'global' });

const searchForm = ref({
  product_id: '',
  batch_no: '',
  color_no: '',
  grade: '',
});

// 批次 273：接入 useTableApi，消除手写 tableData/total/loading/pagination/loadData 重复
// 修复 0-based 分页 bug：原 page-1 传 0 被后端 max(1) 修正为 1，page=2 时传 1 offset=0，分页错乱
// useTableApi 使用 1-based 分页，与后端 page.unwrap_or(1).max(1) + (page-1)*page_size 一致
const {
  data: tableData,
  loading,
  page,
  pageSize,
  total,
  refresh: loadData,
  setQueryParam,
} = useTableApi<FiveDimensionStatsResponse>({
  url: '/crm/five-dimension/stats',
  listKey: 'items',
  onError: () => ElMessage.error(t('fiveDimension.index.messageLoadFailed')),
});

const viewDialogVisible = ref(false);
const viewData = ref<FiveDimensionStatsResponse | null>(null);

const parseInput = ref('');
const parseResult = ref<FiveDimensionItem | null>(null);
const parseError = ref('');

const searchDialogVisible = ref(false);
const searchKeyword = ref('');
const searchType = ref('product');
const searchResults = ref<FiveDimensionItem[]>([]);
const searchFormRef = reactive({ keyword: '' });

const gradeOptions = [
  { label: t('fiveDimension.index.gradeFirst'), value: t('fiveDimension.index.gradeFirst') },
  { label: t('fiveDimension.index.gradeSecond'), value: t('fiveDimension.index.gradeSecond') },
  { label: t('fiveDimension.index.gradeThird'), value: t('fiveDimension.index.gradeThird') },
  {
    label: t('fiveDimension.index.gradeDefective'),
    value: t('fiveDimension.index.gradeDefective'),
  },
];

const searchTypeOptions = [
  { label: t('fiveDimension.index.searchTypeProduct'), value: 'product' },
  { label: t('fiveDimension.index.searchTypeBatch'), value: 'batch' },
  { label: t('fiveDimension.index.searchTypeColor'), value: 'color' },
  { label: t('fiveDimension.index.searchTypeDyeLot'), value: 'dye_lot' },
  { label: t('fiveDimension.index.searchTypeGrade'), value: 'grade' },
];

// 批次 273：同步筛选条件到 useTableApi.queryParams 并刷新
// useTableApi 自动 watch page/pageSize 变化触发重载，无需手动 loadData
const syncQueryParams = () => {
  setQueryParam(
    'product_id',
    searchForm.value.product_id ? Number(searchForm.value.product_id) : undefined
  );
  setQueryParam('batch_no', searchForm.value.batch_no || undefined);
  setQueryParam('color_no', searchForm.value.color_no || undefined);
  setQueryParam('grade', searchForm.value.grade || undefined);
};

const handleSearch = () => {
  syncQueryParams();
  page.value = 1;
  loadData();
};

const handleReset = () => {
  searchForm.value = {
    product_id: '',
    batch_no: '',
    color_no: '',
    grade: '',
  };
  syncQueryParams();
  page.value = 1;
  loadData();
};

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p;
};

const handlePageSizeChange = (s: number) => {
  pageSize.value = s;
  page.value = 1;
};

const openViewDialog = async (item: FiveDimensionStatsResponse) => {
  try {
    // v11 批次 179 P2-1 修复：res: any 改为具体类型
    const res = (await getStatsByFiveDimensionId(item.dimension.five_dimension_id!)) as {
      data?: FiveDimensionStatsResponse;
    };
    viewData.value = res.data || null;
    viewDialogVisible.value = true;
  } catch (error) {
    ElMessage.error(t('fiveDimension.index.messageFetchDetailFailed'));
  }
};

const handleParse = async () => {
  if (!parseInput.value.trim()) {
    ElMessage.warning(t('fiveDimension.index.messageInputFiveDimensionId'));
    return;
  }
  try {
    // v11 批次 179 P2-1 修复：res: any 改为具体类型
    const res = (await parseFiveDimensionId(parseInput.value)) as {
      data?: { success?: boolean; dimension?: FiveDimensionItem; error?: string };
    };
    if (res.data?.success) {
      parseResult.value = res.data.dimension || null;
      parseError.value = '';
    } else {
      parseResult.value = null;
      parseError.value = res.data?.error || t('fiveDimension.index.messageParseFailed');
    }
  } catch (error) {
    parseError.value = t('fiveDimension.index.messageParseFailed');
    parseResult.value = null;
  }
};

const handleQuickSearch = async () => {
  if (!searchKeyword.value.trim()) {
    ElMessage.warning(t('fiveDimension.index.messageInputSearchKeyword'));
    return;
  }
  try {
    // v11 批次 179 P2-1 修复：res: any 改为具体类型
    const res = (await searchFiveDimension({
      keyword: searchKeyword.value,
      search_type: searchType.value,
      page: 0,
      page_size: 50,
    })) as { data?: { items?: FiveDimensionItem[] } };
    searchResults.value = res.data?.items || [];
  } catch (error) {
    ElMessage.error(t('fiveDimension.index.messageSearchFailed'));
  }
};

const selectFromSearch = (item: FiveDimensionItem) => {
  searchForm.value.batch_no = item.batch_no || '';
  searchForm.value.color_no = item.color_no || '';
  searchForm.value.grade = item.grade || '';
  searchDialogVisible.value = false;
  handleSearch();
};

// 批次 273：useTableApi 构造时自动初始加载，无需 setup 顶层调用 loadData
</script>

<template>
  <div class="app-container">
    <div class="filter-container">
      <ElCard :title="t('fiveDimension.index.cardTitleParse')" class="parse-card">
        <ElRow :gutter="20">
          <ElCol :span="12">
            <ElInput
              v-model="parseInput"
              :placeholder="t('fiveDimension.index.placeholderParseInput')"
              class="filter-item"
            />
          </ElCol>
          <ElCol :span="4">
            <ElButton type="primary" class="w-full" @click="handleParse">
              <Key /> {{ t('fiveDimension.index.buttonParse') }}
            </ElButton>
          </ElCol>
          <ElCol :span="4">
            <ElButton type="success" class="w-full" @click="searchDialogVisible = true">
              <Search /> {{ t('fiveDimension.index.buttonQuickSearch') }}
            </ElButton>
          </ElCol>
        </ElRow>
        <div v-if="parseResult" class="parse-result">
          <ElDivider />
          <ElDescriptions :column="5" border>
            <ElDescriptionsItem :label="t('fiveDimension.index.labelProductId')">{{
              parseResult.product_id
            }}</ElDescriptionsItem>
            <ElDescriptionsItem :label="t('fiveDimension.index.labelBatchNo')">{{
              parseResult.batch_no
            }}</ElDescriptionsItem>
            <ElDescriptionsItem :label="t('fiveDimension.index.labelColorNo')">{{
              parseResult.color_no
            }}</ElDescriptionsItem>
            <ElDescriptionsItem :label="t('fiveDimension.index.labelDyeLotNo')">{{
              parseResult.dye_lot_no || '-'
            }}</ElDescriptionsItem>
            <ElDescriptionsItem :label="t('fiveDimension.index.labelGrade')">{{
              parseResult.grade
            }}</ElDescriptionsItem>
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
            :placeholder="t('fiveDimension.index.placeholderProductId')"
            class="filter-item"
            @keyup.enter="handleSearch"
          />
        </ElCol>
        <ElCol :span="6">
          <ElInput
            v-model="searchForm.batch_no"
            :placeholder="t('fiveDimension.index.placeholderBatchNo')"
            class="filter-item"
            @keyup.enter="handleSearch"
          />
        </ElCol>
        <ElCol :span="6">
          <ElInput
            v-model="searchForm.color_no"
            :placeholder="t('fiveDimension.index.placeholderColorNo')"
            class="filter-item"
            @keyup.enter="handleSearch"
          />
        </ElCol>
        <ElCol :span="6">
          <ElSelect
            v-model="searchForm.grade"
            :placeholder="t('fiveDimension.index.placeholderGrade')"
            class="filter-item"
          >
            <ElOption :label="t('fiveDimension.index.optionAll')" value="" />
            <ElOption v-for="g in gradeOptions" :key="g.value" :label="g.label" :value="g.value" />
          </ElSelect>
        </ElCol>
      </ElRow>
      <div class="filter-actions">
        <ElButton type="primary" @click="handleSearch">{{
          t('fiveDimension.index.buttonQuery')
        }}</ElButton>
        <ElButton @click="handleReset">{{ t('fiveDimension.index.buttonReset') }}</ElButton>
        <ElButton @click="loadData">
          <Refresh /> {{ t('fiveDimension.index.buttonRefresh') }}
        </ElButton>
      </div>
    </div>

    <ElTable
      :data="tableData"
      :loading="loading"
      border
      fit
      highlight-current-row
      style="width: 100%"
      :aria-label="t('fiveDimension.index.ariaTable')"
    >
      <ElTableColumn
        prop="dimension.product_id"
        :label="t('fiveDimension.index.colProductId')"
        width="100"
      />
      <ElTableColumn
        prop="dimension.product_name"
        :label="t('fiveDimension.index.colProductName')"
        width="150"
      />
      <ElTableColumn
        prop="dimension.batch_no"
        :label="t('fiveDimension.index.colBatchNo')"
        width="120"
      />
      <ElTableColumn
        prop="dimension.color_no"
        :label="t('fiveDimension.index.colColorNo')"
        width="100"
      />
      <ElTableColumn
        prop="dimension.dye_lot_no"
        :label="t('fiveDimension.index.colDyeLotNo')"
        width="120"
      />
      <ElTableColumn
        prop="dimension.grade"
        :label="t('fiveDimension.index.colGrade')"
        width="100"
      />
      <ElTableColumn
        prop="total_meters"
        :label="t('fiveDimension.index.colTotalMeters')"
        width="120"
        align="right"
      >
        <template #default="scope">{{ scope.row.total_meters }}</template>
      </ElTableColumn>
      <ElTableColumn
        prop="total_kg"
        :label="t('fiveDimension.index.colTotalKg')"
        width="120"
        align="right"
      >
        <template #default="scope">{{ scope.row.total_kg }}</template>
      </ElTableColumn>
      <ElTableColumn
        prop="stock_count"
        :label="t('fiveDimension.index.colStockCount')"
        width="120"
        align="center"
      />
      <ElTableColumn
        prop="dimension.five_dimension_id"
        :label="t('fiveDimension.index.colFiveDimensionId')"
      />
      <ElTableColumn :label="t('fiveDimension.index.colOperation')" width="100" align="center">
        <template #default="scope">
          <ElButton size="small" @click="openViewDialog(scope.row as FiveDimensionStatsResponse)">
            <View />
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
        :aria-label="t('fiveDimension.index.ariaPagination')"
        @size-change="handlePageSizeChange"
        @current-change="handlePageChange"
      />
    </div>

    <ElDialog
      :title="t('fiveDimension.index.dialogTitleView')"
      :visible="viewDialogVisible"
      width="800px"
      :aria-label="t('fiveDimension.index.ariaViewDialog')"
      @close="viewDialogVisible = false"
    >
      <div v-if="viewData">
        <ElDescriptions :column="3" border>
          <ElDescriptionsItem :label="t('fiveDimension.index.labelProductId')">{{
            viewData.dimension.product_id
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('fiveDimension.index.colProductName')">{{
            viewData.dimension.product_name || '-'
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('fiveDimension.index.labelBatchNo')">{{
            viewData.dimension.batch_no
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('fiveDimension.index.labelColorNo')">{{
            viewData.dimension.color_no
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('fiveDimension.index.labelDyeLotNo')">{{
            viewData.dimension.dye_lot_no || '-'
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('fiveDimension.index.labelGrade')">{{
            viewData.dimension.grade
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('fiveDimension.index.colTotalMeters')">{{
            viewData.total_meters
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('fiveDimension.index.colTotalKg')">{{
            viewData.total_kg
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('fiveDimension.index.colStockCount')">{{
            viewData.stock_count
          }}</ElDescriptionsItem>
        </ElDescriptions>
        <div style="margin-top: 20px">
          <h4>{{ t('fiveDimension.index.titleWarehouseDistribution') }}</h4>
          <ElTable
            :data="viewData.warehouse_distribution"
            border
            style="width: 100%"
            :aria-label="t('fiveDimension.index.ariaWarehouseTable')"
          >
            <ElTableColumn
              prop="warehouse_id"
              :label="t('fiveDimension.index.colWarehouseId')"
              width="100"
            />
            <ElTableColumn
              prop="warehouse_name"
              :label="t('fiveDimension.index.colWarehouseName')"
              width="150"
            />
            <ElTableColumn
              prop="quantity_meters"
              :label="t('fiveDimension.index.colMeters')"
              width="120"
              align="right"
            />
            <ElTableColumn
              prop="quantity_kg"
              :label="t('fiveDimension.index.colKg')"
              width="120"
              align="right"
            />
          </ElTable>
        </div>
      </div>
    </ElDialog>

    <ElDialog
      :title="t('fiveDimension.index.dialogTitleQuickSearch')"
      :visible="searchDialogVisible"
      width="700px"
      :aria-label="t('fiveDimension.index.ariaQuickSearchDialog')"
      @close="searchDialogVisible = false"
    >
      <ElForm
        :model="searchFormRef"
        label-width="80px"
        :aria-label="t('fiveDimension.index.ariaQuickSearchForm')"
      >
        <ElFormItem :label="t('fiveDimension.index.labelSearchKeyword')">
          <ElInput
            v-model="searchFormRef.keyword"
            :placeholder="t('fiveDimension.index.placeholderSearchKeyword')"
          />
        </ElFormItem>
        <ElFormItem :label="t('fiveDimension.index.labelSearchType')">
          <ElSelect v-model="searchType">
            <ElOption
              v-for="opt in searchTypeOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </ElSelect>
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="searchDialogVisible = false">{{
          t('fiveDimension.index.buttonCancel')
        }}</ElButton>
        <ElButton type="primary" @click="handleQuickSearch">{{
          t('fiveDimension.index.buttonSearch')
        }}</ElButton>
      </template>
      <div v-if="searchResults.length > 0" style="margin-top: 10px">
        <ElDivider />
        <ElTable
          :data="searchResults"
          border
          style="width: 100%"
          size="small"
          :aria-label="t('fiveDimension.index.ariaSearchResults')"
        >
          <ElTableColumn
            prop="product_id"
            :label="t('fiveDimension.index.colProductId')"
            width="80"
          />
          <ElTableColumn
            prop="product_name"
            :label="t('fiveDimension.index.colProductName')"
            width="120"
          />
          <ElTableColumn prop="batch_no" :label="t('fiveDimension.index.colBatchNo')" width="120" />
          <ElTableColumn prop="color_no" :label="t('fiveDimension.index.colColorNo')" width="80" />
          <ElTableColumn
            prop="dye_lot_no"
            :label="t('fiveDimension.index.colDyeLotNo')"
            width="120"
          />
          <ElTableColumn prop="grade" :label="t('fiveDimension.index.colGrade')" width="80" />
          <ElTableColumn :label="t('fiveDimension.index.colOperation')" width="80">
            <template #default="scope">
              <ElButton
                size="small"
                type="primary"
                @click="selectFromSearch(scope.row as FiveDimensionItem)"
                >{{ t('fiveDimension.index.buttonSelect') }}</ElButton
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
