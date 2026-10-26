<!--
  ProductListTab.vue - 产品列表 Tab
  来源：原 product/index.vue 中 列表/统计/过滤内容
  拆分日期：2026-06-15 B3-4
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <div class="product-list">
    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon total-icon">
              <el-icon><Goods /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ t('product.productListTab.statTotalProducts') }}</div>
              <div class="stat-value">{{ stats.totalProducts }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon active-icon">
              <el-icon><CircleCheck /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ t('product.productListTab.statActiveProducts') }}</div>
              <div class="stat-value">{{ stats.activeProducts }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card
          shadow="hover"
          class="stat-card warning"
          style="cursor: pointer"
          @click="emit('openCategory')"
        >
          <div class="stat-content">
            <div class="stat-icon category-icon">
              <el-icon><Collection /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ t('product.productListTab.statTotalCategories') }}</div>
              <div class="stat-value">{{ stats.totalCategories }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon price-icon">
              <el-icon><Money /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ t('product.productListTab.statAvgPrice') }}</div>
              <div class="stat-value">{{ formatCurrency(stats.avgPrice) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="queryParams"
        class="filter-form"
        :aria-label="t('product.productListTab.filterAriaLabel')"
      >
        <el-form-item :label="t('product.productListTab.labelKeyword')">
          <el-input
            v-model="queryParams.keyword"
            :placeholder="t('product.productListTab.placeholderKeyword')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('product.productListTab.labelCategory')">
          <el-cascader
            v-model="queryParams.category_id"
            :options="categoryTree"
            :props="{ checkStrictly: true, emitPath: false }"
            :placeholder="t('product.productListTab.placeholderCategory')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('product.productListTab.labelStatus')">
          <el-select
            v-model="queryParams.status"
            :placeholder="t('product.productListTab.placeholderStatus')"
            clearable
          >
            <el-option :label="t('product.productListTab.statusActive')" value="active" />
            <el-option :label="t('product.productListTab.statusInactive')" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">{{
            t('product.productListTab.buttonSearch')
          }}</el-button>
          <el-button @click="handleReset">{{ t('product.productListTab.buttonReset') }}</el-button>
          <!-- P2-10 修复（批次 82 v1 复审）：补齐 v-permission 按钮权限 -->
          <el-button
            v-permission="'products:create'"
            type="primary"
            @click="emit('openForm', 'create', null)"
          >
            <el-icon><Plus /></el-icon>{{ t('product.productListTab.buttonCreate') }}
          </el-button>
          <el-button @click="emit('openImport')">
            <el-icon><Upload /></el-icon>{{ t('product.productListTab.buttonImport') }}
          </el-button>
          <el-button @click="batchProductVisible = true">
            {{ t('product.productListTab.batchProductTitle') }}
          </el-button>
          <!-- 产品属敏感导出，只能发起审批申请；无申请权限的角色不渲染该入口 -->
          <el-button v-permission="'export-approvals:create'" @click="openExportDialog">
            <el-icon><Download /></el-icon>{{ t('common.export') }}
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table
        v-loading="loading"
        :data="products"
        stripe
        :aria-label="t('product.productListTab.tableAriaLabel')"
        @selection-change="handleSelectionChange"
      >
        <el-table-column type="selection" width="45" align="center" />
        <el-table-column
          prop="product_code"
          :label="t('product.productListTab.colProductCode')"
          width="140"
          fixed
        />
        <el-table-column
          prop="product_name"
          :label="t('product.productListTab.colProductName')"
          min-width="180"
          fixed
        />
        <el-table-column
          prop="category_name"
          :label="t('product.productListTab.colCategory')"
          width="120"
        >
          <template #default="{ row }">
            <el-tag v-if="row.category_name" type="info" size="small">{{
              row.category_name
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="specification"
          :label="t('product.productListTab.colSpecification')"
          width="120"
          show-overflow-tooltip
        />
        <el-table-column prop="unit" :label="t('product.productListTab.colUnit')" width="80" />
        <el-table-column
          prop="price"
          :label="t('product.productListTab.colPrice')"
          width="100"
          align="right"
        >
          <template #default="{ row }">
            <span v-if="row.price">{{ formatCurrency(row.price) }}</span>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column
          prop="cost_price"
          :label="t('product.productListTab.colCostPrice')"
          width="100"
          align="right"
        >
          <template #default="{ row }">
            <span v-if="row.cost_price">{{ formatCurrency(row.cost_price) }}</span>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column
          prop="barcode"
          :label="t('product.productListTab.colBarcode')"
          width="140"
        />
        <el-table-column prop="is_active" :label="t('product.productListTab.colStatus')" width="80">
          <template #default="{ row }">
            <el-tag :type="row.is_active ? 'success' : 'info'" size="small">
              {{
                row.is_active
                  ? t('product.productListTab.statusActive')
                  : t('product.productListTab.statusInactive')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('product.productListTab.colActions')" width="260" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="emit('openForm', 'view', row)">{{
              t('product.productListTab.buttonDetail')
            }}</el-button>
            <el-button type="primary" link size="small" @click="emit('openForm', 'edit', row)">{{
              t('product.productListTab.buttonEdit')
            }}</el-button>
            <el-button type="warning" link size="small" @click="openColorDialog(row)">
              {{ t('product.productListTab.buttonColors') }}
            </el-button>
            <el-button type="danger" link size="small" @click="handleDelete(row)">{{
              t('product.productListTab.buttonDelete')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-button v-if="selectedIds.length" type="danger" size="small" @click="handleBatchDelete">
          {{ t('product.productListTab.buttonBatchDelete') }}（{{ selectedIds.length }}）
        </el-button>
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('product.productListTab.paginationAriaLabel')"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <!-- 产品色号管理对话框（createProductColor/updateProductColor/deleteProductColor/batchCreateProductColors） -->
    <el-dialog
      v-model="colorDialogVisible"
      :title="`${t('product.productListTab.buttonColors')} - ${colorProduct?.product_name || ''}`"
      width="640"
    >
      <el-form :model="colorForm" :inline="true" class="color-form">
        <el-form-item :label="t('product.productListTab.colColorNo')">
          <el-input v-model="colorForm.color_no" style="width: 140px" />
        </el-form-item>
        <el-form-item :label="t('product.productListTab.colColorName')">
          <el-input v-model="colorForm.color_name" style="width: 160px" />
        </el-form-item>
        <el-form-item :label="t('product.productListTab.colColorExtraCost')">
          <!-- 数值录入用纯文本输入框（全项目禁加减步进控件），范围与精度由提交前校验点名拒绝 -->
          <el-input v-model="colorForm.extra_cost" style="width: 140px" />
        </el-form-item>
        <el-form-item :label="t('product.productListTab.colColorFastness')">
          <!-- 色牢度等级词表与后端 color_fastness 同源（GB 18401 九档，含半级），留空即未考核 -->
          <el-select
            v-model="colorForm.color_fastness_grade"
            clearable
            :placeholder="t('product.productListTab.placeholderColorFastness')"
            style="width: 140px"
          >
            <el-option
              v-for="grade in COLOR_FASTNESS_GRADE_VALUES"
              :key="grade"
              :label="grade"
              :value="grade"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="colorSubmitting" @click="submitColor">
            {{
              colorEditingId
                ? t('product.productListTab.buttonSave')
                : t('product.productListTab.buttonAdd')
            }}
          </el-button>
          <el-button v-if="colorEditingId" @click="colorEditingId = null">
            {{ t('common.cancel') }}
          </el-button>
        </el-form-item>
      </el-form>

      <el-table :data="colorRows" border size="small" max-height="260">
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column
          prop="color_no"
          :label="t('product.productListTab.colColorNo')"
          width="120"
        />
        <el-table-column
          prop="color_name"
          :label="t('product.productListTab.colColorName')"
          min-width="120"
        />
        <el-table-column
          prop="color_fastness_grade"
          :label="t('product.productListTab.colColorFastness')"
          width="110"
        />
        <el-table-column
          :label="t('product.productListTab.colOperation')"
          width="140"
          fixed="right"
        >
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="editColor(row)">{{
              t('product.productListTab.buttonEdit')
            }}</el-button>
            <el-button link type="danger" size="small" @click="handleDeleteColor(row)">{{
              t('product.productListTab.buttonDelete')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="batch-color-bar">
        <el-input
          v-model="batchColorsText"
          type="textarea"
          :rows="3"
          :placeholder="t('product.productListTab.batchColorsPlaceholder')"
        />
        <el-button type="primary" plain :loading="batchColorSaving" @click="handleBatchColors">
          {{ t('product.productListTab.buttonBatchColors') }}
        </el-button>
      </div>
    </el-dialog>

    <!-- 批量维护产品（batchCreateProducts / batchUpdateProducts） -->
    <el-dialog
      v-model="batchProductVisible"
      :title="t('product.productListTab.batchProductTitle')"
      width="620"
    >
      <el-radio-group v-model="batchProductMode" style="margin-bottom: 8px">
        <el-radio value="create">{{ t('product.productListTab.batchProductCreate') }}</el-radio>
        <el-radio value="update">{{ t('product.productListTab.batchProductUpdate') }}</el-radio>
      </el-radio-group>
      <el-input
        v-model="batchProductText"
        type="textarea"
        :rows="8"
        :placeholder="t('product.productListTab.batchProductPlaceholder')"
      />
      <template #footer>
        <el-button @click="batchProductVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="batchProductSaving" @click="handleBatchProduct">
          {{ t('common.confirm') }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 产品导出申请（敏感导出：先申请审批，通过后凭一次性令牌下载） -->
    <el-dialog
      v-model="exportDialogVisible"
      :title="t('product.productListTab.exportRequestTitle')"
      width="720"
    >
      <el-alert
        :closable="false"
        type="info"
        :title="t('product.productListTab.exportSensitiveNotice')"
      />
      <el-form
        class="export-request-form"
        label-width="110px"
        :aria-label="t('product.productListTab.exportFormAriaLabel')"
      >
        <el-form-item :label="t('product.productListTab.exportFormatLabel')">
          <el-radio-group v-model="exportForm.file_format">
            <el-radio value="xlsx">{{ t('product.productListTab.exportFormatXlsx') }}</el-radio>
            <el-radio value="pdf">{{ t('product.productListTab.exportFormatPdf') }}</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item :label="t('product.productListTab.exportEstimatedRowsLabel')">
          <span class="export-estimated-rows">{{ total }}</span>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="exportSubmitting" @click="handleSubmitExportRequest">
            {{ t('product.productListTab.buttonSubmitRequest') }}
          </el-button>
        </el-form-item>
      </el-form>

      <el-divider />

      <div class="export-requests-head">
        <span class="export-requests-title">
          {{ t('product.productListTab.exportMyRequestsTitle') }}
        </span>
        <el-button text size="small" :loading="exportRequestsLoading" @click="loadMyExportRequests">
          {{ t('common.refresh') }}
        </el-button>
      </div>
      <el-table
        v-loading="exportRequestsLoading"
        :data="myExportRequests"
        size="small"
        border
        max-height="280"
        :aria-label="t('product.productListTab.exportRequestsTableAriaLabel')"
      >
        <el-table-column prop="created_at" :label="t('exportApprovals.colCreatedAt')" width="170" />
        <el-table-column
          prop="file_format"
          :label="t('product.productListTab.exportFormatLabel')"
          width="90"
        />
        <el-table-column :label="t('exportApprovals.colStatus')" width="110">
          <template #default="{ row }">
            <el-tag :type="exportApprovalStatusTagType(row.status)" size="small">
              {{ t(exportApprovalStatusLabelKey(row.status)) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('common.action')" min-width="180">
          <template #default="{ row }">
            <el-button
              v-if="canDownloadRow(row)"
              type="primary"
              size="small"
              :loading="downloadingApprovalId === row.id"
              @click="handleDownloadWithToken(row)"
            >
              {{ t('product.productListTab.buttonDownload') }}
            </el-button>
            <span v-else-if="row.status === 'approved'" class="export-token-hint">
              {{ t('product.productListTab.exportTokenConsumed') }}
            </span>
            <span
              v-else-if="row.status === 'pending' || row.status === 'pending_l2'"
              class="export-token-hint"
            >
              {{ t('product.productListTab.exportDownloadAfterApproved') }}
            </span>
            <span v-else class="export-token-hint">
              {{ t('product.productListTab.exportApplyAgainHint') }}
            </span>
          </template>
        </el-table-column>
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { formatCurrency } from '@/utils';
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { msg } from '@/utils/message';
// 批次 277：迁移到 useTableApi composable，移除手写分页逻辑
import { ref, reactive, watch, onMounted, defineEmits, defineExpose } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { isDialogDismissal, rethrowNonDismissal } from '@/utils/monitor';
import {
  Plus,
  Upload,
  Download,
  Goods,
  CircleCheck,
  Collection,
  Money,
} from '@element-plus/icons-vue';
import {
  getProductCategoryList,
  getProductCategoryTree,
  deleteProduct,
  batchDeleteProducts,
  batchCreateProducts,
  batchUpdateProducts,
  createProductColor,
  updateProductColor,
  deleteProductColor,
  batchCreateProductColors,
  exportProducts,
  type ExportProductsQuery,
  type Product,
  type ProductCategory,
  type ProductColor,
  type CreateProductColorPayload,
} from '@/api/product';
import { createApproval, listApprovals, type ExportApprovalRequest } from '@/api/export-approvals';
import { COLOR_FASTNESS_GRADE_VALUES } from '@/constants/product-spec-vocab';
import {
  exportApprovalStatusLabelKey,
  exportApprovalStatusTagType,
} from '@/constants/export-approval';
import { useUserStore } from '@/store/user';
import { useTableApi } from '@/composables/useTableApi';

const { t } = useI18n({ useScope: 'global' });

const emit = defineEmits<{
  openForm: [mode: 'create' | 'edit' | 'view', row: Product | null];
  openImport: [];
  openCategory: [];
}>();

// 批次 277：使用 useTableApi 管理列表分页/筛选/loading/total 状态，自动 watch 分页变化并初始加载
const {
  data: products,
  total,
  loading,
  page,
  pageSize,
  queryParams,
  refresh: fetchData,
  setQueryParam,
} = useTableApi<Product>({
  url: '/products',
  defaultParams: {
    keyword: '',
    category_id: undefined as number | undefined,
    status: undefined as 'active' | 'inactive' | undefined,
  },
  onError: (err: unknown) => {
    // 批次 277：类型守卫处理错误，避免直接 as Error 强转
    const message =
      err instanceof Error ? err.message : t('product.productListTab.messageFetchFailed');
    ElMessage.error(message);
  },
});

// 分类树与列表（fetchCategories 仍手写，与列表分页无关）
const categories = ref<ProductCategory[]>([]);
const categoryTree = ref<ProductCategory[]>([]);

const stats = reactive({
  totalProducts: 0,
  activeProducts: 0,
  totalCategories: 0,
  avgPrice: 0,
});

// 批次 277：watch data 自动更新统计指标（原 fetchData 内联逻辑迁移至此）
watch(products, () => {
  stats.totalProducts = total.value;
  stats.activeProducts = products.value.filter(p => p.is_active).length;
  // 出参 price 是 DECIMAL 字符串（NULL→null），聚合前先 Number 归一再求和；
  // 缺值与非数字归 0，与原来 price||0 的口径一致，避免字符串混进加法把均价算成 NaN
  stats.avgPrice =
    products.value.length > 0
      ? products.value.reduce((sum, p) => sum + (Number(p.price) || 0), 0) / products.value.length
      : 0;
});

// countNodes：递归统计树节点数（用于统计指标）
const countNodes = (nodes: ProductCategory[]): number =>
  nodes.reduce((sum, n) => sum + 1 + countNodes(n.children || []), 0);

const fetchCategories = async () => {
  try {
    const [flatRes, treeRes] = await Promise.all([
      getProductCategoryList(),
      getProductCategoryTree(),
    ]);
    // 后端 list 是 {items,total} 信封；此前把该对象当数组用，本 tab 的分类下拉恒空
    categories.value = flatRes.data.items;
    categoryTree.value = (treeRes.data as ProductCategory[] | undefined) || [];
    stats.totalCategories = countNodes(categoryTree.value);
  } catch (error) {
    logAuxLoadFailure(t('product.productListTab.messageFetchCategoriesFailed'), error);
  }
};

// 批次 277：将 queryParams 筛选字段同步到 setQueryParam，确保请求参数生效
const syncQueryParams = () => {
  setQueryParam('search', queryParams.value.keyword);
  setQueryParam('category_id', queryParams.value.category_id);
  setQueryParam('status', queryParams.value.status);
};

// 批次 277：分页页码变化处理（由 useTableApi watch 自动触发重载）
const handlePageChange = (p: number) => {
  page.value = p;
};

// 批次 277：分页每页条数变化处理（由 useTableApi watch 自动触发重载）
const handleSizeChange = (s: number) => {
  pageSize.value = s;
};

const handleQuery = () => {
  // 批次 277：同步筛选参数并回到首页重载
  syncQueryParams();
  page.value = 1;
  fetchData();
};
const handleReset = () => {
  queryParams.value.keyword = '';
  queryParams.value.category_id = undefined;
  queryParams.value.status = undefined;
  handleQuery();
};

// 产品清单属敏感导出：后端 products/export 强制 download_token（fail-closed），
// 直连导出必 403。这里改为「申请 至 审批 至 凭一次性令牌下载」的申请人侧流程，
// 申请走 api/export-approvals 的 createApproval，下载走 exportProducts 携带令牌。
const exportDialogVisible = ref(false);
const exportSubmitting = ref(false);
const exportRequestsLoading = ref(false);
const downloadingApprovalId = ref<number | null>(null);
const myExportRequests = ref<ExportApprovalRequest[]>([]);
// 后端 create 校验 file_format 仅接受 xlsx/pdf（export_approval_service validate 分支）
const exportForm = reactive({ file_format: 'xlsx' as 'xlsx' | 'pdf' });

const userStore = useUserStore();

const openExportDialog = () => {
  exportDialogVisible.value = true;
  void loadMyExportRequests();
};

// 当前筛选映射为后端 ExportProductsQuery 键名（keyword 对应 search，空值不上送）；
// 同一形状同时作为审批申请的 export_params 落库，下载时按申请时刻的参数复原。
// queryParams 是 Record<string, unknown>，逐键 typeof 收窄，不整体强转
const currentExportQuery = (): ExportProductsQuery => {
  const raw = queryParams.value;
  const q: ExportProductsQuery = {};
  if (typeof raw.category_id === 'number') {
    q.category_id = raw.category_id;
  }
  if (typeof raw.status === 'string' && raw.status !== '') {
    q.status = raw.status;
  }
  if (typeof raw.keyword === 'string' && raw.keyword.trim() !== '') {
    q.search = raw.keyword;
  }
  return q;
};

const handleSubmitExportRequest = async () => {
  if (exportSubmitting.value) return;
  exportSubmitting.value = true;
  try {
    const res = await createApproval({
      resource_type: 'product',
      export_params: { ...currentExportQuery() },
      estimated_rows: total.value,
      file_format: exportForm.file_format,
    });
    const created = res.data;
    logger.info(t('product.productListTab.exportRequestCreatedLog', { id: created.id }));
    msg.success('exportRequestSubmitted', {
      status: t(exportApprovalStatusLabelKey(created.status)),
    });
    await loadMyExportRequests();
  } catch (error) {
    // 失败文案由 request 拦截器统一按后端原始 message 外显（不吞不兜底），此处留显式日志
    logger.error(t('product.productListTab.exportRequestFailedLog'), error);
  } finally {
    exportSubmitting.value = false;
  }
};

// 我发起的产品导出申请：listApprovals 传本人 applicant_user_id（非 admin 后端本就强制
// 只看自己，admin 传该参数同样收敛到本人视角）
const loadMyExportRequests = async () => {
  exportRequestsLoading.value = true;
  try {
    const res = await listApprovals({
      page: 1,
      page_size: 20,
      resource_type: 'product',
      applicant_user_id: userStore.userInfo?.id,
    });
    myExportRequests.value = res.data.items;
  } catch (error) {
    logAuxLoadFailure(t('product.productListTab.exportRequestsLoadFailed'), error);
  } finally {
    exportRequestsLoading.value = false;
  }
};

// 令牌一次性：max_downloads 为 1，download_count 达上限后不可再用（是否过期由后端
// verify 权威判定，前端不自行比对时间避免时钟偏差造成的误判放行）
const canDownloadRow = (row: ExportApprovalRequest): boolean =>
  row.status === 'approved' &&
  row.download_token !== null &&
  row.download_count < row.max_downloads;

// 申请落库的 export_params 即 ExportProductsQuery 形状；逐键显式读取，不透传未知键。
// 入参可空对应后端 Model.export_params 的 Option 列：null 表示该申请未携带任何
// 筛选参数，下载即按无筛选执行（与申请时刻的参数集合一致，非兜底默认值）。
const toExportProductsQuery = (raw: Record<string, unknown> | null): ExportProductsQuery => {
  const q: ExportProductsQuery = {};
  if (!raw) return q;
  if (typeof raw.category_id === 'number') {
    q.category_id = raw.category_id;
  }
  if (typeof raw.status === 'string' && raw.status.trim() !== '') {
    q.status = raw.status;
  }
  if (typeof raw.search === 'string' && raw.search.trim() !== '') {
    q.search = raw.search;
  }
  return q;
};

// blob 请求失败时后端错误体也是 Blob（request 拦截器无法从中取 message），
// 这里解析出 message 原文外显，不做静默兜底
const extractBlobErrorMessage = async (error: unknown): Promise<string | null> => {
  const data = (error as { response?: { data?: unknown } } | undefined)?.response?.data;
  if (!(data instanceof Blob)) return null;
  try {
    const parsed = JSON.parse(await data.text()) as { message?: unknown };
    return typeof parsed.message === 'string' && parsed.message.trim() !== ''
      ? parsed.message
      : null;
  } catch (parseError) {
    logger.warn(t('product.productListTab.exportErrorBodyUnparsable'), parseError);
    return null;
  }
};

const handleDownloadWithToken = async (row: ExportApprovalRequest) => {
  const token = row.download_token;
  // 模板已按 canDownloadRow 门控，此处收窄类型并双保险防御直达调用
  if (!token) return;
  downloadingApprovalId.value = row.id;
  try {
    const res = await exportProducts({
      ...toExportProductsQuery(row.export_params),
      download_token: token,
    });
    // request 响应拦截器对 blob 响应返回完整 AxiosResponse（真 Blob 在其 data 上），
    // 归一化两种形态（同 api/ap.ts printAPPaymentDocx 的既定写法）
    const payload = res as unknown as Blob | { data: Blob };
    const blob = payload instanceof Blob ? payload : payload.data;
    // file_format 后端契约仅 xlsx/pdf 二值，扩展名按申请时刻的格式
    const ext = row.file_format === 'pdf' ? 'pdf' : 'xlsx';
    const url = window.URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.setAttribute('download', `products-${Date.now()}.${ext}`);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    window.URL.revokeObjectURL(url);
    logger.info(t('product.productListTab.exportDownloadedLog', { id: row.id }));
    msg.exportOk();
    // 令牌用后即失效：回源列表把该行刷新为已使用态，供再次申请
    await loadMyExportRequests();
  } catch (error) {
    logger.error(t('product.productListTab.exportDownloadFailedLog', { id: row.id }), error);
    const backendMessage = await extractBlobErrorMessage(error);
    if (backendMessage) {
      ElMessage.error(backendMessage);
    }
    // 失败同样回源刷新令牌真实状态（已用尽或已过期），绝不静默重试掩盖 403
    await loadMyExportRequests();
  } finally {
    downloadingApprovalId.value = null;
  }
};

const handleDelete = async (row: Product) => {
  try {
    await ElMessageBox.confirm(
      t('product.productListTab.messageDeleteConfirm', { productName: row.product_name }),
      t('product.productListTab.messageDeleteTitle'),
      { type: 'warning' }
    );
    await deleteProduct(row.id);
    ElMessage.success(t('product.productListTab.messageDeleteSuccess'));
    fetchData();
  } catch (error) {
    if (!isDialogDismissal(error)) {
      ElMessage.error((error as Error).message || t('product.productListTab.messageDeleteFailed'));
    }
  }
};

// 产品色号管理（createProductColor/updateProductColor）弹窗
const colorDialogVisible = ref(false);
const colorProduct = ref<Product | null>(null);
const colorRows = ref<ProductColor[]>([]);
// 后端 CreateProductColorRequest 必填 color_no/color_name/color_type/extra_cost，
// 建单请求此前缺 color_type/extra_cost 必 422（缺非 Option 必填）。
// color_type 全仓唯一确证取值为 DB 列默认 'STANDARD'（无后端枚举/迁移种子给出其余取值，
// 完整取值域已列待决策），建/改固定提交 'STANDARD'；extra_cost 表单以输入框文本采集（默认 '0'），
// 提交前经校验器点名拒绝非数字与超位小数，再归一为 Number 发给后端 f64 入参。
const COLOR_TYPE_STANDARD = 'STANDARD';
const colorForm = reactive({ color_no: '', color_name: '', extra_cost: '0', color_fastness_grade: '' });
// 加价文本判据：非负十进制、最多两位小数（与后端 extra_cost DECIMAL 列精度一致），负号与非法字符直接判不合格
const COLOR_EXTRA_COST_PATTERN = /^\d+(\.\d{1,2})?$/;
const colorEditingId = ref<number | null>(null);
const colorSubmitting = ref(false);

const openColorDialog = async (row: Product) => {
  colorProduct.value = row;
  colorEditingId.value = null;
  colorForm.color_no = '';
  colorForm.color_name = '';
  colorForm.extra_cost = '0';
  colorForm.color_fastness_grade = '';
  colorDialogVisible.value = true;
  try {
    const { getProductColorList } = await import('@/api/product');
    const res = await getProductColorList(row.id);
    colorRows.value = (res.data as unknown as ProductColor[]) || [];
  } catch (error) {
    logAuxLoadFailure(t('product.productListTab.messageFetchColorsFailed'), error);
    colorRows.value = [];
  }
};

const submitColor = async () => {
  if (!colorProduct.value) return;
  if (!colorForm.color_no.trim()) {
    ElMessage.warning(t('product.productListTab.messageColorNoRequired'));
    return;
  }
  const extraCostText = colorForm.extra_cost.trim();
  if (extraCostText === '' || !COLOR_EXTRA_COST_PATTERN.test(extraCostText)) {
    ElMessage.warning(t('product.productListTab.messageColorExtraCostInvalid'));
    return;
  }
  const extraCost = Number(extraCostText);
  colorSubmitting.value = true;
  try {
    if (colorEditingId.value) {
      // 更新契约（UpdateProductColorRequest）无 color_no，且 color_type 非用户可编辑项——
      // 省略即不改，避免把已有色号的非标准 color_type 覆盖成 STANDARD
      // color_fastness_grade 始终随发：后端把去空白后的空串当作清空该列，有效档位覆盖
      await updateProductColor(colorProduct.value.id, colorEditingId.value, {
        color_name: colorForm.color_name,
        extra_cost: extraCost,
        color_fastness_grade: colorForm.color_fastness_grade,
      });
    } else {
      // 新建时色牢度留空即摘除该键（后端单层 Option 缺键落 NULL），不发空串以免撞 DB CHECK
      await createProductColor(colorProduct.value.id, {
        color_no: colorForm.color_no,
        color_name: colorForm.color_name,
        color_type: COLOR_TYPE_STANDARD,
        extra_cost: extraCost,
        color_fastness_grade: colorForm.color_fastness_grade.trim() || undefined,
      });
    }
    ElMessage.success(t('common.success'));
    colorEditingId.value = null;
    colorForm.color_no = '';
    colorForm.color_name = '';
    colorForm.extra_cost = '0';
    colorForm.color_fastness_grade = '';
    const { getProductColorList } = await import('@/api/product');
    const res = await getProductColorList(colorProduct.value.id);
    colorRows.value = (res.data as unknown as ProductColor[]) || [];
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    colorSubmitting.value = false;
  }
};

const editColor = (row: ProductColor) => {
  colorEditingId.value = row.id;
  colorForm.color_no = row.color_no || '';
  colorForm.color_name = row.color_name || '';
  // extra_cost 为 rust_decimal 出参字符串（"0.00"），原样转文本回填输入框，不做数值往返
  colorForm.extra_cost = String(row.extra_cost ?? '0');
  // 色牢度等级出参为字符串或 null，回填 null 归一空串（下拉回到占位符态）
  colorForm.color_fastness_grade = row.color_fastness_grade ?? '';
};

// 删除色号（deleteProductColor）
const handleDeleteColor = async (row: ProductColor) => {
  if (!colorProduct.value) return;
  try {
    await ElMessageBox.confirm(
      t('product.productListTab.colorDeleteConfirm', { no: row.color_no }),
      t('product.productListTab.colorDeleteTitle'),
      { type: 'warning' }
    );
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('product.handleDeleteColor', error);
  }
  try {
    await deleteProductColor(colorProduct.value.id, row.id);
    ElMessage.success(t('common.success'));
    const { getProductColorList } = await import('@/api/product');
    const res = await getProductColorList(colorProduct.value.id);
    colorRows.value = (res.data as unknown as ProductColor[]) || [];
  } catch (e) {
    if (!isDialogDismissal(e)) {
      ElMessage.error((e as { message?: string }).message || t('common.failed'));
    }
  }
};

// 批量添加色号（batchCreateProductColors：入参为色号数组，api 层按后端
// BatchCreateColorsRequest 包成 { colors: [...] } 提交，调用点不裸发数组）
const batchColorsText = ref('');
const batchColorSaving = ref(false);

const handleBatchColors = async () => {
  if (!colorProduct.value) return;
  let colors: unknown;
  try {
    colors = JSON.parse(batchColorsText.value || '[]');
  } catch {
    ElMessage.warning(t('product.productListTab.batchColorsInvalid'));
    return;
  }
  if (!Array.isArray(colors) || colors.length === 0) {
    ElMessage.warning(t('product.productListTab.batchColorsInvalid'));
    return;
  }
  batchColorSaving.value = true;
  try {
    await batchCreateProductColors(colorProduct.value.id, colors as CreateProductColorPayload[]);
    ElMessage.success(t('common.success'));
    batchColorsText.value = '';
    const { getProductColorList } = await import('@/api/product');
    const res = await getProductColorList(colorProduct.value.id);
    colorRows.value = (res.data as unknown as ProductColor[]) || [];
  } catch (e) {
    ElMessage.error((e as { message?: string }).message || t('common.failed'));
  } finally {
    batchColorSaving.value = false;
  }
};

// 批量删除（勾选行 batchDeleteProducts）
const selectedIds = ref<number[]>([]);
const handleSelectionChange = (rows: Product[]) => {
  selectedIds.value = rows.map(r => r.id);
};
// ===== 批量维护产品（batchCreateProducts / batchUpdateProducts，JSON 数组） =====
const batchProductVisible = ref(false);
const batchProductSaving = ref(false);
const batchProductMode = ref<'create' | 'update'>('create');
const batchProductText = ref('');

const handleBatchProduct = async () => {
  let payload: unknown;
  try {
    payload = JSON.parse(batchProductText.value || '[]');
  } catch {
    ElMessage.warning(t('product.productListTab.batchColorsInvalid'));
    return;
  }
  if (!Array.isArray(payload) || payload.length === 0) {
    ElMessage.warning(t('product.productListTab.batchColorsInvalid'));
    return;
  }
  batchProductSaving.value = true;
  try {
    if (batchProductMode.value === 'create') {
      await batchCreateProducts(payload as Partial<Product>[]);
    } else {
      await batchUpdateProducts(payload as Partial<Product>[]);
    }
    ElMessage.success(t('common.success'));
    batchProductVisible.value = false;
    batchProductText.value = '';
    fetchData();
  } catch (e) {
    ElMessage.error((e as { message?: string }).message || t('common.failed'));
  } finally {
    batchProductSaving.value = false;
  }
};

const handleBatchDelete = async () => {
  if (!selectedIds.value.length) {
    ElMessage.warning(t('product.productListTab.messageSelectFirst'));
    return;
  }
  try {
    await ElMessageBox.confirm(
      t('product.productListTab.messageBatchDeleteConfirm', { count: selectedIds.value.length }) ||
        `确认删除选中的 ${selectedIds.value.length} 个产品？`,
      t('product.productListTab.messageDeleteTitle'),
      { type: 'warning' }
    );
    await batchDeleteProducts(selectedIds.value);
    ElMessage.success(t('product.productListTab.messageDeleteSuccess'));
    selectedIds.value = [];
    fetchData();
  } catch (error) {
    if (!isDialogDismissal(error)) {
      ElMessage.error((error as Error).message || t('product.productListTab.messageDeleteFailed'));
    }
  }
};

defineExpose({ fetchData, fetchCategories });
// 批次 277：useTableApi 自动初始加载列表，onMounted 仅调用 fetchCategories 获取分类树
onMounted(() => {
  fetchCategories();
});
</script>

<style scoped>
.stats-row {
  margin-bottom: 20px;
}
.stat-card {
  border-radius: 12px;
  transition: all 0.3s;
}
.stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}
.stat-card.highlight {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
.stat-card.highlight :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.warning {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}
.stat-card.warning :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.highlight :deep(.stat-label),
.stat-card.highlight :deep(.stat-value),
.stat-card.warning :deep(.stat-label),
.stat-card.warning :deep(.stat-value) {
  color: white;
}
:deep(.stat-content) {
  display: flex;
  align-items: center;
  gap: 16px;
}
:deep(.stat-icon) {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  color: white;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
:deep(.stat-icon.total-icon) {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
}
:deep(.stat-icon.active-icon),
:deep(.stat-icon.category-icon) {
  background: rgba(255, 255, 255, 0.2);
}
:deep(.stat-icon.price-icon) {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}
:deep(.stat-info) {
  flex: 1;
}
:deep(.stat-label) {
  font-size: 14px;
  color: #909399;
  margin-bottom: 4px;
}
:deep(.stat-value) {
  font-size: 28px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}
.filter-card {
  margin-bottom: 20px;
}
.table-card {
  margin-bottom: 20px;
}
.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
.batch-color-bar {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 12px;
}
.export-requests-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.export-requests-title,
.export-estimated-rows {
  font-weight: 600;
}
.export-token-hint {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>
