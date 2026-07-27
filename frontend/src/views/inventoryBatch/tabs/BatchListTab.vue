<!--
  BatchListTab.vue - 批次列表 Tab
  来源：原 inventoryBatch/index.vue 中 列表/过滤内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="batch-list">
    <el-card shadow="hover">
      <template #header>
        <div class="card-header">
          <span>{{ t('inventoryBatch.batchListTab.cardTitle') }}</span>
        </div>
      </template>

      <div class="toolbar">
        <el-form inline :aria-label="t('inventoryBatch.batchListTab.ariaFilterForm')">
          <el-form-item :label="t('inventoryBatch.batchListTab.filterBatchNo')">
            <el-input
              v-model="queryParams.batchNo"
              :placeholder="t('inventoryBatch.batchListTab.placeholderInput')"
              clearable
              style="width: 180px"
            />
          </el-form-item>
          <el-form-item :label="t('inventoryBatch.batchListTab.filterColorNo')">
            <el-input
              v-model="queryParams.colorNo"
              :placeholder="t('inventoryBatch.batchListTab.placeholderInput')"
              clearable
              style="width: 180px"
            />
          </el-form-item>
          <el-form-item :label="t('inventoryBatch.batchListTab.filterGrade')">
            <el-select
              v-model="queryParams.grade"
              :placeholder="t('inventoryBatch.batchListTab.placeholderSelect')"
              clearable
              style="width: 120px"
            >
              <el-option
                :label="t('inventoryBatch.batchListTab.optionGradeFirst')"
                :value="t('inventoryBatch.batchListTab.optionGradeFirst')"
              />
              <el-option
                :label="t('inventoryBatch.batchListTab.optionGradeSecond')"
                :value="t('inventoryBatch.batchListTab.optionGradeSecond')"
              />
              <el-option
                :label="t('inventoryBatch.batchListTab.optionGradeThird')"
                :value="t('inventoryBatch.batchListTab.optionGradeThird')"
              />
            </el-select>
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="handleQuery">{{
              t('inventoryBatch.batchListTab.buttonQuery')
            }}</el-button>
            <el-button @click="handleReset">{{
              t('inventoryBatch.batchListTab.buttonReset')
            }}</el-button>
          </el-form-item>
        </el-form>
        <div class="actions">
          <!-- P2-10 修复（批次 82 v1 复审）：补齐 v-permission 按钮权限 -->
          <el-button v-permission="'inventory:create'" type="primary" @click="handleCreate">{{
            t('inventoryBatch.batchListTab.buttonCreate')
          }}</el-button>
        </div>
      </div>

      <el-table
        v-loading="loading"
        :data="batchList"
        border
        stripe
        :aria-label="t('inventoryBatch.batchListTab.ariaTable')"
      >
        <el-table-column
          prop="batchNo"
          :label="t('inventoryBatch.batchListTab.colBatchNo')"
          width="140"
        />
        <el-table-column
          prop="productName"
          :label="t('inventoryBatch.batchListTab.colProductName')"
        />
        <el-table-column
          prop="colorNo"
          :label="t('inventoryBatch.batchListTab.colColorNo')"
          width="100"
        />
        <el-table-column
          prop="dyeLotNo"
          :label="t('inventoryBatch.batchListTab.colDyeLotNo')"
          width="100"
        />
        <el-table-column
          prop="grade"
          :label="t('inventoryBatch.batchListTab.colGrade')"
          width="100"
        >
          <template #default="{ row }">
            <el-tag
              v-if="row.grade === t('inventoryBatch.batchListTab.optionGradeFirst')"
              type="success"
              >{{ row.grade }}</el-tag
            >
            <el-tag
              v-else-if="row.grade === t('inventoryBatch.batchListTab.optionGradeSecond')"
              type="warning"
              >{{ row.grade }}</el-tag
            >
            <el-tag v-else type="danger">{{ row.grade }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="quantityMeters"
          :label="t('inventoryBatch.batchListTab.colQuantityMeters')"
          width="120"
        />
        <el-table-column
          prop="quantityKg"
          :label="t('inventoryBatch.batchListTab.colQuantityKg')"
          width="100"
        />
        <el-table-column
          prop="gramWeight"
          :label="t('inventoryBatch.batchListTab.colGramWeight')"
          width="100"
        />
        <el-table-column
          prop="width"
          :label="t('inventoryBatch.batchListTab.colWidth')"
          width="100"
        />
        <el-table-column
          prop="warehouseName"
          :label="t('inventoryBatch.batchListTab.colWarehouse')"
        />
        <el-table-column
          prop="stockStatus"
          :label="t('inventoryBatch.batchListTab.colStockStatus')"
          width="100"
        >
          <template #default="{ row }">
            <el-tag
              v-if="row.stockStatus === t('inventoryBatch.batchListTab.statusNormal')"
              type="success"
              >{{ row.stockStatus }}</el-tag
            >
            <el-tag v-else type="warning">{{ row.stockStatus }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="qualityStatus"
          :label="t('inventoryBatch.batchListTab.colQualityStatus')"
          width="100"
        >
          <template #default="{ row }">
            <el-tag
              v-if="row.qualityStatus === t('inventoryBatch.batchListTab.statusQualified')"
              type="success"
              >{{ row.qualityStatus }}</el-tag
            >
            <el-tag v-else type="danger">{{ row.qualityStatus }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="productionDate"
          :label="t('inventoryBatch.batchListTab.colProductionDate')"
          width="120"
        />
        <el-table-column
          :label="t('inventoryBatch.batchListTab.colOperation')"
          fixed="right"
          width="220"
        >
          <template #default="{ row }">
            <el-button link type="primary" @click="handleView(row)">{{
              t('inventoryBatch.batchListTab.buttonView')
            }}</el-button>
            <el-button
              v-permission="'inventory:update'"
              link
              type="primary"
              @click="handleEdit(row)"
              >{{ t('inventoryBatch.batchListTab.buttonEdit') }}</el-button
            >
            <el-button
              v-permission="'inventory:transfer'"
              link
              type="primary"
              @click="handleTransfer(row)"
              >{{ t('inventoryBatch.batchListTab.buttonTransfer') }}</el-button
            >
            <el-button
              v-permission="'inventory:delete'"
              link
              type="danger"
              @click="handleDelete(row)"
              >{{ t('inventoryBatch.batchListTab.buttonDelete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        layout="total, prev, pager, next, jumper"
        :aria-label="t('inventoryBatch.batchListTab.ariaPagination')"
        @current-change="handlePageChange"
      />
    </el-card>

    <!-- 批次 157b P1-1 修复：批次调拨对话框 -->
    <el-dialog
      v-model="transferDialogVisible"
      :title="t('inventoryBatch.batchListTab.titleTransfer')"
      width="480px"
      :aria-label="t('inventoryBatch.batchListTab.ariaTransferDialog')"
    >
      <el-form
        ref="transferFormRef"
        :model="transferForm"
        :rules="transferRules"
        label-width="100px"
        :aria-label="t('inventoryBatch.batchListTab.ariaTransferForm')"
      >
        <el-form-item :label="t('inventoryBatch.batchListTab.labelFromWarehouse')">
          <el-input :model-value="transferForm.fromWarehouseName" disabled />
        </el-form-item>
        <el-form-item
          :label="t('inventoryBatch.batchListTab.labelToWarehouse')"
          prop="toWarehouseId"
        >
          <el-select
            v-model="transferForm.toWarehouseId"
            :placeholder="t('inventoryBatch.batchListTab.placeholderSelectToWarehouse')"
            style="width: 100%"
          >
            <el-option
              v-for="w in warehouseOptions"
              :key="w.id"
              :label="w.warehouse_name"
              :value="w.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item
          :label="t('inventoryBatch.batchListTab.labelTransferQuantityMeters')"
          prop="quantityMeters"
        >
          <el-input-number v-model="transferForm.quantityMeters" :min="0" style="width: 100%" />
        </el-form-item>
        <el-form-item
          :label="t('inventoryBatch.batchListTab.labelTransferQuantityKg')"
          prop="quantityKg"
        >
          <el-input-number v-model="transferForm.quantityKg" :min="0" style="width: 100%" />
        </el-form-item>
        <el-form-item :label="t('inventoryBatch.batchListTab.labelRemarks')">
          <el-input
            v-model="transferForm.remarks"
            type="textarea"
            :placeholder="t('inventoryBatch.batchListTab.placeholderRemarks')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="transferDialogVisible = false">{{
          t('inventoryBatch.batchListTab.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="transferSubmitting" @click="onSubmitTransfer">{{
          t('inventoryBatch.batchListTab.buttonConfirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import {
  deleteBatch,
  transferBatch,
  type InventoryBatch,
  type TransferBatchRequest,
} from '@/api/inventoryBatch';
import { getWarehouseList, type Warehouse } from '@/api/warehouse';
import { useTableApi } from '@/composables/useTableApi';

const { t } = useI18n({ useScope: 'global' });

const emit = defineEmits<{ openForm: [row: InventoryBatch | null] }>();

const queryParams = reactive({
  batchNo: '',
  colorNo: '',
  grade: '',
});

// 批次 276：接入 useTableApi，消除手写 batchList/loading/pagination/fetchBatches 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: batchList,
  loading,
  page,
  pageSize,
  total,
  refresh: fetchBatches,
  setQueryParam,
} = useTableApi<InventoryBatch>({
  url: '/inventory/batches',
  onError: (err: unknown) =>
    ElMessage.error(
      (err instanceof Error ? err.message : String(err)) ||
        t('inventoryBatch.batchListTab.messageFetchFailed')
    ),
});

// 批次 276：同步筛选条件到 useTableApi.queryParams 并刷新
const syncQueryParams = () => {
  setQueryParam('batchNo', queryParams.batchNo || undefined);
  setQueryParam('colorNo', queryParams.colorNo || undefined);
  setQueryParam('grade', queryParams.grade || undefined);
};

const handleQuery = () => {
  syncQueryParams();
  page.value = 1;
  fetchBatches();
};

const handleReset = () => {
  queryParams.batchNo = '';
  queryParams.colorNo = '';
  queryParams.grade = '';
  syncQueryParams();
  page.value = 1;
  fetchBatches();
};

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p;
};

const handleCreate = () => emit('openForm', null);
const handleView = (row: InventoryBatch) => emit('openForm', row);
const handleEdit = (row: InventoryBatch) => emit('openForm', row);

// 批次 157b P1-1 修复：批次调拨接入 transferBatch API
const transferDialogVisible = ref(false);
const transferSubmitting = ref(false);
const transferFormRef = ref<FormInstance>();
const transferCurrentRow = ref<InventoryBatch | null>(null);
const warehouseOptions = ref<Warehouse[]>([]);
const transferForm = reactive<{
  fromWarehouseName: string;
  toWarehouseId: number | null;
  quantityMeters: number;
  quantityKg: number;
  remarks: string;
}>({
  fromWarehouseName: '',
  toWarehouseId: null,
  quantityMeters: 0,
  quantityKg: 0,
  remarks: '',
});
const transferRules: FormRules = {
  toWarehouseId: [
    {
      required: true,
      message: t('inventoryBatch.batchListTab.ruleToWarehouseRequired'),
      trigger: 'change',
    },
  ],
  quantityMeters: [
    {
      required: true,
      message: t('inventoryBatch.batchListTab.ruleTransferQuantityMetersRequired'),
      trigger: 'blur',
    },
  ],
  quantityKg: [
    {
      required: true,
      message: t('inventoryBatch.batchListTab.ruleTransferQuantityKgRequired'),
      trigger: 'blur',
    },
  ],
};

const fetchWarehouseOptions = async () => {
  try {
    const res = (await getWarehouseList({ page: 1, page_size: 1000 })) as unknown as {
      data?: { list?: Warehouse[] };
    };
    warehouseOptions.value = res.data?.list || [];
  } catch {
    warehouseOptions.value = [];
  }
};

const handleTransfer = async (row: InventoryBatch) => {
  transferCurrentRow.value = row;
  transferForm.fromWarehouseName = row.warehouseName || '-';
  transferForm.toWarehouseId = null;
  transferForm.quantityMeters = row.quantityMeters || 0;
  transferForm.quantityKg = row.quantityKg || 0;
  transferForm.remarks = '';
  if (warehouseOptions.value.length === 0) {
    await fetchWarehouseOptions();
  }
  transferDialogVisible.value = true;
};

const onSubmitTransfer = async () => {
  if (!transferFormRef.value || !transferCurrentRow.value) return;
  const row = transferCurrentRow.value;
  await transferFormRef.value.validate(async valid => {
    if (!valid) return;
    if (!row.warehouseId || !transferForm.toWarehouseId) {
      ElMessage.warning(t('inventoryBatch.batchListTab.messageWarehouseInfoIncomplete'));
      return;
    }
    transferSubmitting.value = true;
    try {
      const payload: TransferBatchRequest = {
        fromWarehouseId: row.warehouseId,
        toWarehouseId: transferForm.toWarehouseId,
        quantityMeters: transferForm.quantityMeters,
        quantityKg: transferForm.quantityKg,
        remarks: transferForm.remarks || undefined,
      };
      await transferBatch(row.id as number, payload);
      ElMessage.success(t('inventoryBatch.batchListTab.messageTransferSuccess'));
      transferDialogVisible.value = false;
      fetchBatches();
    } catch (error) {
      ElMessage.error(
        (error as Error).message || t('inventoryBatch.batchListTab.messageTransferFailed')
      );
    } finally {
      transferSubmitting.value = false;
    }
  });
};

const handleDelete = async (row: InventoryBatch) => {
  try {
    await ElMessageBox.confirm(
      t('inventoryBatch.batchListTab.messageConfirmDelete', { batchNo: row.batchNo }),
      t('inventoryBatch.batchListTab.titleDeleteConfirm'),
      { type: 'warning' }
    );
    await deleteBatch(row.id as number);
    ElMessage.success(t('inventoryBatch.batchListTab.messageDeleteSuccess'));
    fetchBatches();
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(
        (error as Error).message || t('inventoryBatch.batchListTab.messageDeleteFailed')
      );
    }
  }
};

defineExpose({ fetchBatches });
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 600;
}
.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 16px;
}
.actions {
  display: flex;
  gap: 8px;
}
</style>
