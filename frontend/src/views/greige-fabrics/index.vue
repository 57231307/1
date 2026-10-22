<template>
  <div class="greige-fabrics-page">
    <div class="header">
      <h2>{{ t('greigeFabrics.index.pageTitle') }}</h2>
      <el-button type="primary" @click="handleCreate">{{
        t('greigeFabrics.index.buttonCreate')
      }}</el-button>
    </div>

    <el-table
      v-loading="loading"
      :data="greigeList"
      border
      :aria-label="t('greigeFabrics.index.ariaTable')"
    >
      <el-table-column
        prop="fabric_no"
        :label="t('greigeFabrics.index.colFabricCode')"
        width="140"
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
      <el-table-column :label="t('greigeFabrics.index.colSupplier')" width="140">
        <template #default="{ row }">{{ supplierName(row.supplier_id) }}</template>
      </el-table-column>
      <el-table-column prop="width" :label="t('greigeFabrics.index.colWidth')" width="90" />
      <el-table-column
        prop="gram_weight"
        :label="t('greigeFabrics.index.colGramWeight')"
        width="100"
      />
      <el-table-column
        prop="composition"
        :label="t('greigeFabrics.index.colComposition')"
        width="120"
      />
      <el-table-column
        prop="weight_kg"
        :label="t('greigeFabrics.index.colWeightKg')"
        width="120"
        align="right"
      />
      <el-table-column
        prop="length_m"
        :label="t('greigeFabrics.index.colLengthM')"
        width="120"
        align="right"
      />
      <el-table-column prop="status" :label="t('greigeFabrics.index.colStatus')" width="100">
        <template #default="{ row }">
          <el-tag :type="row.status === GREIGE_STATUS.IN_STOCK ? 'success' : 'info'">
            {{ row.status }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('greigeFabrics.index.colOperation')" width="300" fixed="right">
        <template #default="{ row }">
          <el-button size="small" type="success" @click="handleStock('in', row)">{{
            t('greigeFabrics.index.buttonStockIn')
          }}</el-button>
          <el-button size="small" type="warning" @click="handleStock('out', row)">{{
            t('greigeFabrics.index.buttonStockOut')
          }}</el-button>
          <el-button size="small" @click="handleEdit(row)">{{
            t('greigeFabrics.index.buttonEdit')
          }}</el-button>
          <el-button size="small" type="danger" @click="handleDelete(row)">{{
            t('greigeFabrics.index.buttonDelete')
          }}</el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="
        dialogMode === 'create'
          ? t('greigeFabrics.index.titleCreate')
          : t('greigeFabrics.index.titleEdit')
      "
      width="600px"
      :aria-label="t('greigeFabrics.index.ariaEditDialog')"
      @close="handleDialogClose"
    >
      <el-form
        ref="formRef"
        :model="formData"
        :rules="formRules"
        label-width="110px"
        :aria-label="t('greigeFabrics.index.ariaForm')"
      >
        <el-form-item :label="t('greigeFabrics.index.colFabricCode')" prop="fabric_no">
          <el-input
            v-model="formData.fabric_no"
            disabled
            :placeholder="t('greigeFabrics.index.placeholderFabricNoAuto')"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.colFabricName')" prop="fabric_name">
          <el-input
            v-model="formData.fabric_name"
            :placeholder="t('greigeFabrics.index.placeholderFabricName')"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.colFabricType')" prop="fabric_type">
          <el-input
            v-model="formData.fabric_type"
            :placeholder="t('greigeFabrics.index.placeholderFabricType')"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.colSupplier')" prop="supplier_id">
          <el-select
            v-model="formData.supplier_id"
            :placeholder="t('greigeFabrics.index.placeholderSelectSupplier')"
            filterable
            style="width: 100%"
          >
            <el-option
              v-for="s in supplierList"
              :key="s.id"
              :label="s.supplier_name"
              :value="s.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.labelWidth')" prop="width">
          <el-input-number
            v-model="formData.width"
            :min="0"
            :precision="2"
            :disabled="dialogMode === 'edit'"
            :placeholder="t('greigeFabrics.index.placeholderWidth')"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.labelWeight')" prop="gram_weight">
          <el-input-number
            v-model="formData.gram_weight"
            :min="0"
            :precision="2"
            :disabled="dialogMode === 'edit'"
            :placeholder="t('greigeFabrics.index.placeholderWeight')"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.labelComposition')" prop="composition">
          <el-input
            v-model="formData.composition"
            :disabled="dialogMode === 'edit'"
            :placeholder="t('greigeFabrics.index.placeholderComposition')"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.colStatus')" prop="status">
          <el-select
            v-model="formData.status"
            :placeholder="t('greigeFabrics.index.placeholderSelectStatus')"
            style="width: 100%"
          >
            <el-option :label="GREIGE_STATUS.IN_STOCK" :value="GREIGE_STATUS.IN_STOCK" />
            <el-option :label="GREIGE_STATUS.OUT_OF_STOCK" :value="GREIGE_STATUS.OUT_OF_STOCK" />
          </el-select>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">{{
          t('greigeFabrics.index.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">
          {{ t('greigeFabrics.index.buttonConfirm') }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 坯布入库/出库：入库必填仓库与重量长度；出库重量长度至少一项正数（对齐后端请求体）。 -->
    <el-dialog
      v-model="stockDialogVisible"
      :title="
        stockType === 'in'
          ? t('greigeFabrics.index.stockInTitle')
          : t('greigeFabrics.index.stockOutTitle')
      "
      width="480px"
      :aria-label="
        stockType === 'in'
          ? t('greigeFabrics.index.stockInTitle')
          : t('greigeFabrics.index.stockOutTitle')
      "
    >
      <el-form ref="stockFormRef" :model="stockForm" :rules="stockRules" label-width="110px">
        <el-form-item
          v-if="stockType === 'in'"
          :label="t('greigeFabrics.index.labelWarehouse')"
          prop="warehouse_id"
        >
          <el-select
            v-model="stockForm.warehouse_id"
            :placeholder="t('greigeFabrics.index.placeholderWarehouse')"
            style="width: 100%"
          >
            <el-option
              v-for="w in warehouses"
              :key="w.id"
              :label="w.warehouse_name"
              :value="w.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.labelWeightKg')" prop="weight_kg">
          <el-input-number
            v-model="stockForm.weight_kg"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.labelLengthM')" prop="length_m">
          <el-input-number
            v-model="stockForm.length_m"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="stockDialogVisible = false">{{
          t('greigeFabrics.index.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="stockLoading" @click="submitStock">
          {{ t('greigeFabrics.index.buttonConfirm') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import {
  getGreigeFabricList,
  createGreigeFabric,
  updateGreigeFabric,
  deleteGreigeFabric,
  stockInGreigeFabric,
  stockOutGreigeFabric,
  GREIGE_STATUS,
  type GreigeFabric,
  type GreigeStatusValue,
  type CreateGreigeFabricPayload,
  type UpdateGreigeFabricPayload,
  type GreigeStockInPayload,
  type GreigeStockOutPayload,
} from '@/api/greige-fabric';
import { getSupplierList, type Supplier } from '@/api/supplier';
import { getWarehouseList, type Warehouse } from '@/api/warehouse';
import { decimalWireToNumber } from '@/utils/money';
import { logAuxLoadFailure } from '@/utils/logger';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';

const { t } = useI18n({ useScope: 'global' });

const loading = ref(false);
const submitLoading = ref(false);
const dialogVisible = ref(false);
const dialogMode = ref<'create' | 'edit'>('create');
const formRef = ref<FormInstance>();
const greigeList = ref<GreigeFabric[]>([]);
const supplierList = ref<Supplier[]>([]);

interface GreigeFabricForm {
  id?: number;
  fabric_no: string;
  fabric_name: string;
  fabric_type: string;
  supplier_id?: number;
  width: number;
  gram_weight: number;
  composition: string;
  status: GreigeStatusValue;
}

const formData = reactive<GreigeFabricForm>({
  fabric_no: '',
  fabric_name: '',
  fabric_type: '',
  supplier_id: undefined,
  width: 0,
  gram_weight: 0,
  composition: '',
  status: GREIGE_STATUS.IN_STOCK,
});

const formRules: FormRules = {
  fabric_type: [
    { required: true, message: t('greigeFabrics.index.ruleFabricTypeRequired'), trigger: 'blur' },
  ],
  status: [
    { required: true, message: t('greigeFabrics.index.ruleStatusRequired'), trigger: 'change' },
  ],
};

const supplierName = (supplierId: number | undefined) => {
  if (!supplierId) return '-';
  return supplierList.value.find(s => s.id === supplierId)?.supplier_name ?? '-';
};

const loadGreigeFabrics = async () => {
  loading.value = true;
  try {
    const res = await getGreigeFabricList();
    // 后端 list_greige_fabrics 返回 PaginatedResponse，data.items 为唯一形状
    greigeList.value = res.data.items;
  } catch (error) {
    logAuxLoadFailure(t('greigeFabrics.index.messageLoadListFailed'), error);
  } finally {
    loading.value = false;
  }
};

const loadSuppliers = async () => {
  try {
    const res = await getSupplierList();
    supplierList.value = res.data?.items || [];
  } catch (error) {
    logAuxLoadFailure(t('greigeFabrics.index.messageLoadSuppliersFailed'), error);
  }
};

const resetForm = () => {
  Object.assign(formData, {
    id: undefined,
    fabric_no: '',
    fabric_name: '',
    fabric_type: '',
    supplier_id: undefined,
    width: 0,
    gram_weight: 0,
    composition: '',
    status: GREIGE_STATUS.IN_STOCK,
  });
};

const handleCreate = () => {
  dialogMode.value = 'create';
  resetForm();
  dialogVisible.value = true;
};

const handleEdit = (row: GreigeFabric) => {
  dialogMode.value = 'edit';
  Object.assign(formData, {
    id: row.id,
    fabric_no: row.fabric_no ?? '',
    fabric_name: row.fabric_name ?? '',
    fabric_type: row.fabric_type,
    supplier_id: row.supplier_id,
    width: decimalWireToNumber(row.width),
    gram_weight: decimalWireToNumber(row.gram_weight),
    composition: row.composition ?? '',
    status: (row.status as GreigeStatusValue) ?? GREIGE_STATUS.IN_STOCK,
  });
  dialogVisible.value = true;
};

const handleDelete = async (row: GreigeFabric) => {
  if (!row.id) return;
  try {
    await deleteGreigeFabric(row.id);
    ElMessage.success(t('greigeFabrics.index.messageDeleteSuccess'));
    await loadGreigeFabrics();
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('greigeFabrics.index.messageDeleteFailed'));
  }
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  const valid = await formRef.value.validate().catch(() => false);
  if (!valid) return;

  submitLoading.value = true;
  try {
    if (dialogMode.value === 'create') {
      const payload: CreateGreigeFabricPayload = {
        fabric_name: formData.fabric_name || undefined,
        fabric_type: formData.fabric_type,
        supplier_id: formData.supplier_id,
        width: formData.width || undefined,
        gram_weight: formData.gram_weight || undefined,
        composition: formData.composition || undefined,
        status: formData.status,
      };
      await createGreigeFabric(payload);
      ElMessage.success(t('greigeFabrics.index.messageCreateSuccess'));
    } else {
      // UpdateGreigeFabricRequest 不含 width/gram_weight/composition（创建后不可改）
      const payload: UpdateGreigeFabricPayload = {
        fabric_name: formData.fabric_name,
        fabric_type: formData.fabric_type,
        supplier_id: formData.supplier_id,
        status: formData.status,
      };
      await updateGreigeFabric(formData.id!, payload);
      ElMessage.success(t('greigeFabrics.index.messageUpdateSuccess'));
    }
    dialogVisible.value = false;
    await loadGreigeFabrics();
  } catch (error) {
    const err = error as Error;
    ElMessage.error(
      err.message ||
        (dialogMode.value === 'create'
          ? t('greigeFabrics.index.messageCreateFailed')
          : t('greigeFabrics.index.messageUpdateFailed'))
    );
  } finally {
    submitLoading.value = false;
  }
};

const handleDialogClose = () => {
  formRef.value?.resetFields();
};

// ---- 坯布入库/出库 ----
const stockDialogVisible = ref(false);
const stockType = ref<'in' | 'out'>('in');
const stockRow = ref<GreigeFabric | null>(null);
const warehouses = ref<Warehouse[]>([]);
const stockLoading = ref(false);
const stockFormRef = ref<FormInstance>();
const stockForm = reactive({
  warehouse_id: undefined as number | undefined,
  weight_kg: undefined as number | undefined,
  length_m: undefined as number | undefined,
});

// 入库三项全必填（对齐 StockInRequest）；出库两项均可选，至少填一项正数由 submitStock 前置校验。
const stockRules = computed<FormRules>(() =>
  stockType.value === 'in'
    ? {
        warehouse_id: [
          {
            required: true,
            message: () => t('greigeFabrics.index.messageWarehouseRequired'),
            trigger: 'change',
          },
        ],
        weight_kg: [
          {
            required: true,
            message: () => t('greigeFabrics.index.messageWeightMustBePositive'),
            trigger: 'blur',
          },
        ],
        length_m: [
          {
            required: true,
            message: () => t('greigeFabrics.index.messageLengthMustBePositive'),
            trigger: 'blur',
          },
        ],
      }
    : {}
);

const fetchWarehouses = async () => {
  try {
    const res = await getWarehouseList();
    warehouses.value = res.data?.items || [];
  } catch (error) {
    logAuxLoadFailure(t('greigeFabrics.index.messageLoadWarehousesFailed'), error);
  }
};

const handleStock = async (type: 'in' | 'out', row: GreigeFabric) => {
  stockType.value = type;
  stockRow.value = row;
  stockForm.warehouse_id = undefined;
  stockForm.weight_kg = undefined;
  stockForm.length_m = undefined;
  stockFormRef.value?.clearValidate();
  if (type === 'in') fetchWarehouses();
  stockDialogVisible.value = true;
};

const positive = (v: number | undefined): v is number => typeof v === 'number' && v > 0;

const submitStock = async () => {
  if (!stockRow.value) return;
  if (
    stockType.value === 'out' &&
    !positive(stockForm.weight_kg) &&
    !positive(stockForm.length_m)
  ) {
    ElMessage.error(t('greigeFabrics.index.messageAtLeastOneOfWeightOrLength'));
    return;
  }
  const ok = await stockFormRef.value?.validate().catch(() => false);
  if (!ok) return;
  if (
    stockType.value === 'in' &&
    (!positive(stockForm.weight_kg) || !positive(stockForm.length_m))
  ) {
    ElMessage.error(t('greigeFabrics.index.messageWeightAndLengthMustBePositive'));
    return;
  }
  stockLoading.value = true;
  try {
    if (stockType.value === 'in') {
      const payload: GreigeStockInPayload = {
        warehouse_id: stockForm.warehouse_id as number,
        weight_kg: stockForm.weight_kg as number,
        length_m: stockForm.length_m as number,
      };
      await stockInGreigeFabric(stockRow.value.id, payload);
      ElMessage.success(t('greigeFabrics.index.messageStockInSuccess'));
    } else {
      const payload: GreigeStockOutPayload = {};
      if (positive(stockForm.weight_kg)) payload.weight_kg = stockForm.weight_kg;
      if (positive(stockForm.length_m)) payload.length_m = stockForm.length_m;
      await stockOutGreigeFabric(stockRow.value.id, payload);
      ElMessage.success(t('greigeFabrics.index.messageStockOutSuccess'));
    }
    stockDialogVisible.value = false;
    await loadGreigeFabrics();
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('greigeFabrics.index.messageStockFailed'));
  } finally {
    stockLoading.value = false;
  }
};

const hasLoaded = createLazyLoader();

onMounted(() => {
  loadGreigeFabrics();
  loadIfNot('suppliers', loadSuppliers, hasLoaded);
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
  margin-bottom: 20px;
}
</style>
