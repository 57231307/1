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
          <el-button :loading="exporting" @click="handleExport">
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
              {{ t('product.productListTab.buttonColors') || '色号' }}
            </el-button>
            <el-button type="danger" link size="small" @click="handleDelete(row)">{{
              t('product.productListTab.buttonDelete')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-button v-if="selectedIds.length" type="danger" size="small" @click="handleBatchDelete">
          {{ t('product.productListTab.buttonBatchDelete') || '批量删除' }}（{{
            selectedIds.length
          }}）
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
  </div>
</template>

<script setup lang="ts">
import { formatCurrency } from '@/utils';
import { logger } from '@/utils/logger';
// 批次 277：迁移到 useTableApi composable，移除手写分页逻辑
import { ref, reactive, watch, onMounted, defineEmits, defineExpose } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
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
  type Product,
  type ProductCategory,
  type ProductColor,
} from '@/api/product';
import { exportFabrics } from '@/api/fabric';
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
  stats.avgPrice =
    products.value.length > 0
      ? products.value.reduce((sum, p) => sum + (p.price || 0), 0) / products.value.length
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
    categories.value = (flatRes.data as ProductCategory[] | undefined) || [];
    categoryTree.value = (treeRes.data as ProductCategory[] | undefined) || [];
    stats.totalCategories = countNodes(categoryTree.value);
  } catch (error) {
    logger.error(t('product.productListTab.messageFetchCategoriesFailed'), error);
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

const exporting = ref(false);
// 产品列表导出：调 /products/export 下载 blob（与 dye-recipe 等页面统一模式）
const handleExport = async () => {
  if (exporting.value) return;
  exporting.value = true;
  try {
    const res = await exportFabrics(queryParams.value as never);
    const url = window.URL.createObjectURL(new Blob([res]));
    const link = document.createElement('a');
    link.href = url;
    link.setAttribute('download', `products-${Date.now()}.xlsx`);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    window.URL.revokeObjectURL(url);
    ElMessage.success(t('message.exportSuccess'));
  } catch (error) {
    ElMessage.error(t('message.exportFailed'));
    console.error('[ProductListTab] 导出失败:', error);
  } finally {
    exporting.value = false;
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
    if (error !== 'cancel') {
      ElMessage.error((error as Error).message || t('product.productListTab.messageDeleteFailed'));
    }
  }
};

// 产品色号管理（createProductColor/updateProductColor）弹窗
const colorDialogVisible = ref(false);
const colorProduct = ref<Product | null>(null);
const colorRows = ref<ProductColor[]>([]);
const colorForm = reactive({ color_no: '', color_name: '' });
const colorEditingId = ref<number | null>(null);
const colorSubmitting = ref(false);

const openColorDialog = async (row: Product) => {
  colorProduct.value = row;
  colorEditingId.value = null;
  colorForm.color_no = '';
  colorForm.color_name = '';
  colorDialogVisible.value = true;
  try {
    const { getProductColorList } = await import('@/api/product');
    const res = await getProductColorList(row.id);
    colorRows.value = (res.data as unknown as ProductColor[]) || [];
  } catch (error) {
    logger.error(t('product.productListTab.messageFetchColorsFailed'), error);
    colorRows.value = [];
  }
};

const submitColor = async () => {
  if (!colorProduct.value) return;
  if (!colorForm.color_no.trim()) {
    ElMessage.warning(t('product.productListTab.messageColorNoRequired') || '请输入色号');
    return;
  }
  colorSubmitting.value = true;
  try {
    if (colorEditingId.value) {
      await updateProductColor(colorProduct.value.id, colorEditingId.value, colorForm);
    } else {
      await createProductColor(colorProduct.value.id, colorForm);
    }
    ElMessage.success(t('common.success'));
    colorEditingId.value = null;
    colorForm.color_no = '';
    colorForm.color_name = '';
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
  } catch {
    return;
  }
  try {
    await deleteProductColor(colorProduct.value.id, row.id);
    ElMessage.success(t('common.success'));
    const { getProductColorList } = await import('@/api/product');
    const res = await getProductColorList(colorProduct.value.id);
    colorRows.value = (res.data as unknown as ProductColor[]) || [];
  } catch (e) {
    if (e !== 'cancel') {
      ElMessage.error((e as { message?: string }).message || t('common.failed'));
    }
  }
};

// 批量添加色号（batchCreateProductColors：JSON 数组）
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
    await batchCreateProductColors(colorProduct.value.id, colors as Partial<ProductColor>[]);
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
    ElMessage.warning(t('product.productListTab.messageSelectFirst') || '请先勾选要删除的产品');
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
    if (error !== 'cancel') {
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
</style>
