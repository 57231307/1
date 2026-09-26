<template>
  <div class="mrp-history-container">
    <el-card class="header-card">
      <div class="header-content">
        <h2>{{ t('mrp.history.title') }}</h2>
        <p>{{ t('mrp.history.subtitle') }}</p>
      </div>
    </el-card>

    <!-- 筛选：仅暴露后端 MrpResultQuery 真正支持的三个条件 -->
    <el-card class="filter-card" :aria-label="t('mrp.history.filterAriaLabel')">
      <el-form :inline="true" @submit.prevent>
        <el-form-item :label="t('mrp.history.calculationNo')">
          <el-input
            v-model="filterForm.calculation_no"
            :placeholder="t('mrp.history.filterCalculationNoPlaceholder')"
            clearable
            style="width: 200px"
            @keyup.enter="applyFilters"
          />
        </el-form-item>
        <el-form-item :label="t('mrp.history.product')">
          <el-select
            v-model="filterForm.product_id"
            :placeholder="t('mrp.history.filterProductPlaceholder')"
            clearable
            filterable
            style="width: 220px"
          >
            <el-option
              v-for="p in products"
              :key="p.id"
              :label="`${p.code} - ${p.name}`"
              :value="p.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('mrp.history.status')">
          <el-select
            v-model="filterForm.status"
            :placeholder="t('mrp.history.filterStatusPlaceholder')"
            clearable
            style="width: 160px"
          >
            <el-option :label="t('mrp.history.statusPlanned')" value="PLANNED" />
            <el-option :label="t('mrp.history.statusConfirmed')" value="CONFIRMED" />
            <el-option :label="t('mrp.history.statusReleased')" value="RELEASED" />
            <el-option :label="t('mrp.history.statusCancelled')" value="CANCELLED" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="applyFilters">{{ t('mrp.history.search') }}</el-button>
          <el-button @click="resetFilters">{{ t('mrp.history.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 结果行列表（GET /mrp/results 真实分页，行级即 calculation_no 级） -->
    <el-card class="table-card">
      <el-table
        v-loading="loading"
        :data="rows"
        row-key="id"
        stripe
        border
        :aria-label="t('mrp.history.listAriaLabel')"
      >
        <el-table-column
          prop="calculation_no"
          :label="t('mrp.history.calculationNo')"
          width="180"
        />
        <el-table-column :label="t('mrp.history.product')" min-width="200">
          <template #default="{ row }">{{ productLabel(row.product_id) }}</template>
        </el-table-column>
        <el-table-column :label="t('mrp.history.demandQuantity')" width="120" align="right">
          <template #default="{ row }">{{ fmtQty(row.required_quantity) }}</template>
        </el-table-column>
        <el-table-column prop="required_date" :label="t('mrp.history.demandDate')" width="130" />
        <el-table-column :label="t('mrp.history.plannedOrderQuantity')" width="130" align="right">
          <template #default="{ row }">{{ fmtQty(row.planned_order_quantity) }}</template>
        </el-table-column>
        <el-table-column
          prop="planned_order_date"
          :label="t('mrp.history.plannedOrderDate')"
          width="130"
        />
        <el-table-column :label="t('mrp.history.status')" width="120">
          <template #default="{ row }">
            <el-tag :type="statusTag(row.status)">{{ statusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="source_type" :label="t('mrp.history.source')" width="120" />
        <el-table-column prop="created_at" :label="t('mrp.history.createdAt')" width="200" />
        <el-table-column :label="t('mrp.history.operation')" width="220" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewResult(row)">
              {{ t('mrp.history.viewResult') }}
            </el-button>
            <el-button
              v-if="row.status !== 'CANCELLED'"
              type="warning"
              link
              size="small"
              @click="handleCancel(row)"
            >
              {{ t('mrp.history.cancelCalculation') }}
            </el-button>
            <el-button type="success" link size="small" @click="handleExport(row)">
              {{ t('mrp.history.exportResult') }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('mrp.history.paginationAriaLabel')"
          @size-change="handleSizeChange"
          @current-change="handleCurrentChange"
        />
      </div>
    </el-card>

    <!-- 结果详情：行本身字段 + 该行实时库存分解（get_material_detail） -->
    <el-dialog
      v-model="detailVisible"
      :title="t('mrp.history.resultDialogTitle')"
      width="880px"
      :aria-label="t('mrp.history.resultDialogAriaLabel')"
    >
      <el-descriptions v-if="currentRow" :column="2" border class="result-header">
        <el-descriptions-item :label="t('mrp.history.calculationNo')">{{
          currentRow.calculation_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('mrp.history.product')">{{
          productLabel(currentRow.product_id)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('mrp.history.demandQuantity')">{{
          fmtQty(currentRow.required_quantity)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('mrp.history.demandDate')">{{
          currentRow.required_date
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('mrp.history.plannedOrderQuantity')">{{
          fmtQty(currentRow.planned_order_quantity)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('mrp.history.plannedOrderDate')">{{
          currentRow.planned_order_date
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('mrp.history.status')">
          <el-tag :type="statusTag(currentRow.status)">{{ statusLabel(currentRow.status) }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('mrp.history.source')">{{
          currentRow.source_type
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('mrp.history.createdAt')">{{
          currentRow.created_at
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('mrp.history.updatedAt')">{{
          currentRow.updated_at
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('mrp.history.remarks')" :span="2">
          {{ currentRow.remarks }}
        </el-descriptions-item>
      </el-descriptions>

      <el-divider content-position="left">{{ t('mrp.history.stockDivider') }}</el-divider>
      <el-descriptions
        v-if="detail"
        :column="2"
        border
        size="small"
        :aria-label="t('mrp.history.detailAriaLabel')"
      >
        <el-descriptions-item :label="t('mrp.history.onHandQuantity')">{{
          fmtQty(detail.on_hand_quantity)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('mrp.history.availableStock')">{{
          fmtQty(detail.available_quantity)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('mrp.history.inTransitQuantity')">{{
          fmtQty(detail.in_transit_quantity)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('mrp.history.safetyStock')">{{
          fmtQty(detail.safety_stock)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('mrp.history.shortageQuantity')">
          <span :class="{ 'highlight-quantity': Number(detail.shortage_quantity) > 0 }">
            {{ fmtQty(detail.shortage_quantity) }}
          </span>
        </el-descriptions-item>
      </el-descriptions>
      <el-skeleton v-else-if="detailLoading" :rows="2" animated />
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { useI18n } from 'vue-i18n';
import {
  cancelMrpCalculation,
  exportMrpResult,
  getMaterialRequirementDetail,
  getProductsForMrp,
  type MrpMaterialDetail,
  type MrpProductOption,
  type MrpResultResponse,
  type MrpStatus,
} from '../../api/mrp';
import { useTableApi } from '@/composables/useTableApi';

const { t } = useI18n({ useScope: 'global' });

type TagType = 'primary' | 'success' | 'info' | 'warning' | 'danger';

// 词表与后端唯一来源 models/status/production.rs 的 `pub mod mrp` 四个常量同源；
// Record<MrpStatus, ..> 让"新增状态没配颜色"变成编译错误，而不是运行时静默显示默认色。
const STATUS_TAG: Record<MrpStatus, TagType> = {
  PLANNED: 'info',
  CONFIRMED: 'success',
  RELEASED: 'warning',
  CANCELLED: 'danger',
};
const statusTag = (s: MrpStatus): TagType => STATUS_TAG[s];

// 显式 t('...') 分支而非动态拼接，保证 check-i18n 能校验每个 key 存在
const statusLabel = (s: MrpStatus): string => {
  switch (s) {
    case 'PLANNED':
      return t('mrp.history.statusPlanned');
    case 'CONFIRMED':
      return t('mrp.history.statusConfirmed');
    case 'RELEASED':
      return t('mrp.history.statusReleased');
    case 'CANCELLED':
      return t('mrp.history.statusCancelled');
    default: {
      // 兜底分支不可达：出现即说明后端词表已扩而前端未跟，必须编译期暴露而不是显示原文
      const unhandled: never = s;
      return unhandled;
    }
  }
};

// 后端 Decimal 字段运行时可能为字符串，统一数值化显示；null 显示空
const fmtQty = (v: number | null): string => (v == null ? '' : String(Number(v)));

const products = ref<MrpProductOption[]>([]);
const productById = computed(() => new Map(products.value.map(p => [p.id, p])));
// 产品主数据缺失（如已删除）时退回显示 id，属真实可得的标识，非空壳占位
const productLabel = (id: number): string => {
  const p = productById.value.get(id);
  return p ? `${p.code} - ${p.name}` : `#${id}`;
};

const loadProducts = async () => {
  try {
    const res = await getProductsForMrp();
    products.value = res.data;
  } catch (e: unknown) {
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('mrp.calc.fetchProductsError')
    );
  }
};

const filterForm = reactive({
  calculation_no: '',
  product_id: undefined as number | undefined,
  status: '',
});

const {
  data: rows,
  loading,
  page,
  pageSize,
  total,
  queryParams,
  refresh,
} = useTableApi<MrpResultResponse>({
  url: '/production/mrp/results',
  listKey: 'items',
  onError: (e: unknown) => {
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('mrp.history.fetchListError')
    );
  },
});

// 空串转 undefined，让 axios 丢弃未填条件（后端据此视为不过滤）
const applyFilters = () => {
  queryParams.value = {
    calculation_no: filterForm.calculation_no.trim() || undefined,
    product_id: filterForm.product_id,
    status: filterForm.status || undefined,
  };
  if (page.value !== 1) page.value = 1;
  else void refresh();
};

const resetFilters = () => {
  filterForm.calculation_no = '';
  filterForm.product_id = undefined;
  filterForm.status = '';
  applyFilters();
};

const handleSizeChange = (s: number) => {
  pageSize.value = s;
  page.value = 1;
};

const handleCurrentChange = (p: number) => {
  page.value = p;
};

const detailVisible = ref(false);
const detailLoading = ref(false);
const currentRow = ref<MrpResultResponse | null>(null);
const detail = ref<MrpMaterialDetail | null>(null);

const viewResult = async (row: MrpResultResponse) => {
  currentRow.value = row;
  detail.value = null;
  detailVisible.value = true;
  detailLoading.value = true;
  try {
    const res = await getMaterialRequirementDetail(row.id, row.product_id);
    detail.value = res.data;
  } catch (e: unknown) {
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('mrp.history.fetchResultError')
    );
  } finally {
    detailLoading.value = false;
  }
};

const handleCancel = async (row: MrpResultResponse) => {
  try {
    await ElMessageBox.confirm(
      t('mrp.history.cancelConfirmMessage', { no: row.calculation_no }),
      t('common.confirmTitle'),
      { type: 'warning' }
    );
  } catch {
    return;
  }
  try {
    await cancelMrpCalculation(row.id);
    ElMessage.success(t('common.success'));
    await refresh();
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : String(e));
  }
};

const handleExport = async (row: MrpResultResponse) => {
  try {
    const blob = await exportMrpResult(row.id);
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `mrp_result_${row.calculation_no}.xlsx`;
    a.click();
    URL.revokeObjectURL(url);
    ElMessage.success(t('common.success'));
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : String(e));
  }
};

onMounted(loadProducts);
</script>

<style scoped>
.mrp-history-container {
  padding: 20px;
}

.header-card {
  margin-bottom: 20px;
}

.header-content h2 {
  margin: 0 0 8px 0;
  color: #303133;
}

.header-content p {
  margin: 0;
  color: #909399;
}

.filter-card {
  margin-bottom: 20px;
}

.table-card {
  margin-bottom: 20px;
}

.pagination-container {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}

.result-header {
  margin-bottom: 16px;
}

.highlight-quantity {
  color: #e6a23c;
  font-weight: bold;
}
</style>
