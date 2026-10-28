<template>
  <div class="greige-fabrics-page">
    <div class="header">
      <h2>{{ t('greigeFabrics.index.pageTitle') }}</h2>
      <el-button v-permission="'greige-fabrics:create'" type="primary" @click="handleCreate">
        {{ t('greigeFabrics.index.buttonCreate') }}
      </el-button>
    </div>

    <el-card shadow="never" class="filter-card">
      <el-form
        :inline="true"
        :model="filters"
        class="filter-form"
        :aria-label="t('greigeFabrics.filter.ariaForm')"
        @submit.prevent
      >
        <el-form-item :label="t('greigeFabrics.index.colFabricCode')">
          <el-input
            v-model="filters.fabric_no"
            clearable
            :placeholder="t('greigeFabrics.filter.placeholderFabricNo')"
            @clear="handleQuery"
            @keyup.enter="handleQuery"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.colFabricName')">
          <el-input
            v-model="filters.fabric_name"
            clearable
            :placeholder="t('greigeFabrics.filter.placeholderFabricName')"
            @clear="handleQuery"
            @keyup.enter="handleQuery"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.colFabricType')">
          <el-input
            v-model="filters.fabric_type"
            clearable
            :placeholder="t('greigeFabrics.filter.placeholderFabricType')"
            @clear="handleQuery"
            @keyup.enter="handleQuery"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.colSupplier')">
          <el-select
            v-model="filters.supplier_id"
            filterable
            remote
            clearable
            :remote-show-suffix="true"
            :loading="supplierFilterLoading"
            :remote-method="searchSupplierFilter"
            :placeholder="t('greigeFabrics.filter.placeholderRemoteSupplier')"
            :no-data-text="t('greigeFabrics.picker.noData')"
            style="width: 200px"
            @change="handleQuery"
          >
            <el-option
              v-for="item in supplierFilterOptions"
              :key="item.id"
              :label="item.label"
              :value="item.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.labelWarehouse')">
          <el-select
            v-model="filters.warehouse_id"
            filterable
            remote
            clearable
            :remote-show-suffix="true"
            :loading="warehouseFilterLoading"
            :remote-method="searchWarehouseFilter"
            :placeholder="t('greigeFabrics.filter.placeholderRemoteWarehouse')"
            :no-data-text="t('greigeFabrics.picker.noData')"
            style="width: 200px"
            @change="handleQuery"
          >
            <el-option
              v-for="item in warehouseFilterOptions"
              :key="item.id"
              :label="item.label"
              :value="item.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.colStatus')">
          <el-select
            v-model="filters.status"
            clearable
            :placeholder="t('greigeFabrics.filter.placeholderStatus')"
            style="width: 140px"
            @change="handleQuery"
          >
            <el-option :label="GREIGE_STATUS.IN_STOCK" :value="GREIGE_STATUS.IN_STOCK" />
            <el-option :label="GREIGE_STATUS.OUT_OF_STOCK" :value="GREIGE_STATUS.OUT_OF_STOCK" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.form.labelQualityGrade')">
          <el-select
            v-model="filters.quality_grade"
            clearable
            :placeholder="t('greigeFabrics.form.placeholderSelectQualityGrade')"
            style="width: 120px"
            @change="handleQuery"
          >
            <el-option v-for="g in QUALITY_GRADES" :key="g" :label="g" :value="g" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            {{ t('greigeFabrics.filter.buttonQuery') }}
          </el-button>
          <el-button @click="handleResetFilter">
            {{ t('greigeFabrics.filter.buttonReset') }}
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-table
      v-loading="loading"
      :data="rows"
      border
      :aria-label="t('greigeFabrics.index.ariaTable')"
    >
      <el-table-column
        prop="fabric_no"
        :label="t('greigeFabrics.index.colFabricCode')"
        width="140"
        fixed="left"
      />
      <el-table-column
        prop="fabric_name"
        :label="t('greigeFabrics.index.colFabricName')"
        min-width="150"
      />
      <el-table-column
        prop="fabric_type"
        :label="t('greigeFabrics.index.colFabricType')"
        width="100"
      />
      <el-table-column :label="t('greigeFabrics.index.colSupplier')" width="160">
        <template #default="{ row }">{{ row.supplier_id ?? '-' }}</template>
      </el-table-column>
      <el-table-column :label="t('greigeFabrics.index.colWidth')" width="90" align="right">
        <template #default="{ row }">{{ formatDecimal(row.width) }}</template>
      </el-table-column>
      <el-table-column :label="t('greigeFabrics.index.colGramWeight')" width="100" align="right">
        <template #default="{ row }">{{ formatDecimal(row.gram_weight) }}</template>
      </el-table-column>
      <el-table-column
        prop="composition"
        :label="t('greigeFabrics.index.colComposition')"
        width="120"
      >
        <template #default="{ row }">{{ row.composition ?? '-' }}</template>
      </el-table-column>
      <el-table-column :label="t('greigeFabrics.index.colWeightKg')" width="120" align="right">
        <template #default="{ row }">{{ formatDecimal(row.weight_kg) }}</template>
      </el-table-column>
      <el-table-column :label="t('greigeFabrics.index.colLengthM')" width="120" align="right">
        <template #default="{ row }">{{ formatDecimal(row.length_m) }}</template>
      </el-table-column>
      <el-table-column :label="t('greigeFabrics.index.colStatus')" width="100">
        <template #default="{ row }">
          <el-tag :type="row.status === GREIGE_STATUS.IN_STOCK ? 'success' : 'info'">
            {{ row.status ?? '-' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('greigeFabrics.form.labelColorNo')" width="100">
        <template #default="{ row }">{{ row.color_no ?? '-' }}</template>
      </el-table-column>
      <el-table-column :label="t('greigeFabrics.form.labelDyeLotNo')" width="120">
        <template #default="{ row }">{{ row.dye_lot_no ?? '-' }}</template>
      </el-table-column>
      <el-table-column :label="t('greigeFabrics.form.labelQualityGrade')" width="90">
        <template #default="{ row }">{{ row.quality_grade ?? '-' }}</template>
      </el-table-column>
      <el-table-column :label="t('greigeFabrics.form.labelSafetyStock')" width="110" align="right">
        <template #default="{ row }">{{ formatDecimal(row.safety_stock) }}</template>
      </el-table-column>
      <el-table-column :label="t('greigeFabrics.form.labelReorderPoint')" width="110" align="right">
        <template #default="{ row }">{{ formatDecimal(row.reorder_point) }}</template>
      </el-table-column>
      <el-table-column
        :label="t('greigeFabrics.form.labelMaxStockPoint')"
        width="110"
        align="right"
      >
        <template #default="{ row }">{{ formatDecimal(row.max_stock_point) }}</template>
      </el-table-column>
      <el-table-column
        :label="t('greigeFabrics.form.labelReorderQuantity')"
        width="110"
        align="right"
      >
        <template #default="{ row }">{{ formatDecimal(row.reorder_quantity) }}</template>
      </el-table-column>
      <el-table-column :label="t('greigeFabrics.index.colOperation')" width="340" fixed="right">
        <template #default="{ row }">
          <el-button
            v-permission-detail="{ resource: 'greige-fabrics', action: 'read' }"
            size="small"
            @click="handleView(row)"
          >
            {{ t('greigeFabrics.index.buttonView') }}
          </el-button>
          <el-button
            v-permission="'greige-fabrics:create'"
            size="small"
            type="success"
            @click="handleStock('in', row)"
          >
            {{ t('greigeFabrics.index.buttonStockIn') }}
          </el-button>
          <!-- 出库走 POST /greige-fabrics/{id}/stock-out，与入库同为 POST，后端权限中间件按
               方法派生动作得 create（stock-in/stock-out 不在路径动作关键字表内），故入库与
               出库两个入口的可达键都必须是 greige-fabrics:create，与后端逐字符一致，否则只持
               update 不含 create 的账号点了必 403，成了假可供按钮 -->
          <el-button
            v-permission="'greige-fabrics:create'"
            size="small"
            type="warning"
            @click="handleStock('out', row)"
          >
            {{ t('greigeFabrics.index.buttonStockOut') }}
          </el-button>
          <el-button v-permission="'greige-fabrics:update'" size="small" @click="handleEdit(row)">
            {{ t('greigeFabrics.index.buttonEdit') }}
          </el-button>
          <el-button
            v-permission="'greige-fabrics:delete'"
            size="small"
            type="danger"
            @click="handleDelete(row)"
          >
            {{ t('greigeFabrics.index.buttonDelete') }}
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
        :aria-label="t('greigeFabrics.filter.ariaPagination')"
      />
    </div>

    <GreigeFormDialog
      v-model:visible="formDialogVisible"
      :mode="formDialogMode"
      :row="editingRow"
      @saved="handleSaved"
    />
    <GreigeStockDialog
      v-model:visible="stockDialogVisible"
      :type="stockDialogType"
      :row="stockRow"
      @done="handleSaved"
    />
    <GreigeDetailDialog v-model:visible="detailDialogVisible" :row-id="detailRowId" />
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';

import { GREIGE_STATUS, deleteGreigeFabric, type GreigeFabric } from '@/api/greige-fabric';
import { getSupplierList } from '@/api/supplier';
import { getWarehouseList } from '@/api/warehouse';
import { useTableApi } from '@/composables/useTableApi';
import { decimalWireToNumber } from '@/utils/money';

import { useGreigePicker, PICKER_PAGE_SIZE, type PickerOption } from './useGreigePicker';
import GreigeFormDialog from './components/GreigeFormDialog.vue';
import GreigeStockDialog from './components/GreigeStockDialog.vue';
import GreigeDetailDialog from './components/GreigeDetailDialog.vue';

/**
 * 坯布管理主页面。
 * 功能：以真分页与后端参数筛选呈现坯布列表，档案面覆盖 36 列（库存策略四列只做录入与展示），
 * 行操作提供建档 编辑 出入库 详情 删除；供应商维度入口通过筛选卡的供应商选择器与
 * by-supplier 端点复用同一份真相源，不再另起一套前端过滤。
 * 调用方：路由 views/greige-fabrics 挂载点。
 * 入参：无 props；页面自身消费 useTableApi 向 GET /production/greige-fabrics 发真分页与筛选。
 * 传给谁：GreigeFormDialog 承担建档与编辑（POST 与 PUT 全字段面），GreigeStockDialog 承担
 *      出入库（POST stock-in 与 stock-out），GreigeDetailDialog 承担详情读回（GET /{id}）。
 * 存什么：列表行、总数、页码、每页条数、筛选值与两个远程选择器的候选。
 * 存哪里：组件内响应式状态与 useTableApi 的 ref，不落 store 与浏览器存储。
 */
const { t } = useI18n({ useScope: 'global' });

/** 与建档对话框同一取值域（建表列注释声明 A/B/C） */
const QUALITY_GRADES = ['A', 'B', 'C'] as const;

/**
 * 后端 GreigeFabricListQuery 声明的 8 个筛选字段全部落到本对象，
 * 通过 useTableApi.queryParams 一并作为查询串下推，不允许在前端对已返回的
 * items 做二次过滤（本仓已出过分页截断缺陷族，见调查记录）。
 * 空串与 undefined 由 serializeParams 与后端 normalize_empty_query_params 双侧剔除。
 */
interface GreigeFilterState {
  fabric_no: string;
  fabric_name: string;
  fabric_type: string;
  supplier_id: number | null;
  warehouse_id: number | null;
  status: string;
  quality_grade: string;
}

const emptyFilters = (): GreigeFilterState => ({
  fabric_no: '',
  fabric_name: '',
  fabric_type: '',
  supplier_id: null,
  warehouse_id: null,
  status: '',
  quality_grade: '',
});

const filters = reactive<GreigeFilterState>(emptyFilters());

const {
  data: rows,
  total,
  loading,
  page,
  pageSize,
  refresh,
  setQueryParam,
} = useTableApi<GreigeFabric>({
  url: '/production/greige-fabrics',
  listKey: 'items',
  defaultPageSize: 20,
});

/** Decimal 出参是字符串，展示前 Number 归一；NULL 与 undefined 一律显 '-'，不兜 0 */
const formatDecimal = (value: string | null | undefined): string =>
  value === null || value === undefined ? '-' : String(decimalWireToNumber(value));

/** 把当前 filter 状态整批下推到 useTableApi.queryParams，空值以 undefined 剔除 */
const syncQueryParams = (): void => {
  setQueryParam('fabric_no', filters.fabric_no.trim() || undefined);
  setQueryParam('fabric_name', filters.fabric_name.trim() || undefined);
  setQueryParam('fabric_type', filters.fabric_type.trim() || undefined);
  setQueryParam('supplier_id', filters.supplier_id ?? undefined);
  setQueryParam('warehouse_id', filters.warehouse_id ?? undefined);
  setQueryParam('status', filters.status || undefined);
  setQueryParam('quality_grade', filters.quality_grade || undefined);
};

const handleQuery = async (): Promise<void> => {
  syncQueryParams();
  page.value = 1;
  await refresh();
};

const handleResetFilter = async (): Promise<void> => {
  Object.assign(filters, emptyFilters());
  supplierFilterPicker.reset();
  warehouseFilterPicker.reset();
  await handleQuery();
};

// useTableApi 自带 [page, pageSize] 的 watch，页码变化会带上当前 queryParams 重取；
// 这里不再重复挂 watch，也不做前端过滤。

// ---- 筛选卡供应商与仓库远程选择器 ----
const supplierFilterPicker = useGreigePicker(
  async (keyword): Promise<PickerOption[]> => {
    const res = await getSupplierList({ keyword, page: 1, page_size: PICKER_PAGE_SIZE });
    return res.data.items.map(item => ({
      id: item.id,
      label: `${item.supplier_name}（${item.supplier_code}）`,
    }));
  },
  async (): Promise<PickerOption | null> => null
);
const {
  options: supplierFilterOptions,
  loading: supplierFilterLoading,
  search: searchSupplierFilter,
} = supplierFilterPicker;

const warehouseFilterPicker = useGreigePicker(
  async (keyword): Promise<PickerOption[]> => {
    const res = await getWarehouseList({
      search: keyword,
      page: 1,
      page_size: PICKER_PAGE_SIZE,
    });
    return res.data.items.map(item => ({
      id: item.id,
      label: `${item.warehouse_name}（${item.warehouse_code}）`,
    }));
  },
  async (): Promise<PickerOption | null> => null
);
const {
  options: warehouseFilterOptions,
  loading: warehouseFilterLoading,
  search: searchWarehouseFilter,
} = warehouseFilterPicker;

// ---- 建档与编辑对话框 ----
const formDialogVisible = ref(false);
const formDialogMode = ref<'create' | 'edit'>('create');
const editingRow = ref<GreigeFabric | null>(null);

const handleCreate = (): void => {
  formDialogMode.value = 'create';
  editingRow.value = null;
  formDialogVisible.value = true;
};

const handleEdit = (row: GreigeFabric): void => {
  formDialogMode.value = 'edit';
  editingRow.value = row;
  formDialogVisible.value = true;
};

// ---- 出入库对话框 ----
const stockDialogVisible = ref(false);
const stockDialogType = ref<'in' | 'out'>('in');
const stockRow = ref<GreigeFabric | null>(null);

const handleStock = (type: 'in' | 'out', row: GreigeFabric): void => {
  stockDialogType.value = type;
  stockRow.value = row;
  stockDialogVisible.value = true;
};

// ---- 详情对话框 ----
const detailDialogVisible = ref(false);
const detailRowId = ref<number | null>(null);

const handleView = (row: GreigeFabric): void => {
  detailRowId.value = row.id;
  detailDialogVisible.value = true;
};

// ---- 删除，二次确认；后端只按 status == 在库 拦（该列可空，历史 NULL 行直接放行属另一条待裁定存量面） ----
const handleDelete = async (row: GreigeFabric): Promise<void> => {
  try {
    await ElMessageBox.confirm(
      t('greigeFabrics.messages.confirmDeleteBody', {
        fabricNo: row.fabric_no ?? row.id,
        fabricName: row.fabric_name ?? '-',
      }),
      t('greigeFabrics.messages.confirmDeleteTitle'),
      {
        type: 'warning',
        confirmButtonText: t('greigeFabrics.index.buttonConfirm'),
        cancelButtonText: t('greigeFabrics.index.buttonCancel'),
      }
    );
  } catch {
    // ElMessageBox.confirm 用户取消走 reject，这里不动数据也不误报失败
    return;
  }
  try {
    await deleteGreigeFabric(row.id);
    ElMessage.success(t('greigeFabrics.index.messageDeleteSuccess'));
    await refresh();
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('greigeFabrics.index.messageDeleteFailed'));
  }
};

const handleSaved = async (): Promise<void> => {
  await refresh();
};

onMounted(() => {
  // useTableApi 已在 setup 阶段自动首屏取数，这里预取一次筛选卡供应商与仓库的默认候选，
  // 避免用户第一次点开下拉时看到空白（后端空关键字等价于不加筛选，返回首屏 50 条候选）。
  void searchSupplierFilter('');
  void searchWarehouseFilter('');
});
</script>

<style scoped>
.greige-fabrics-page {
  padding: 20px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.filter-card {
  margin-bottom: 16px;
}

.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 0;
}

.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
