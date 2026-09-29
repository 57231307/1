<!--
  BatchFormDialogTab.vue - 批次编辑对话框
  来源：原 inventoryBatch/index.vue 中 批次编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="
      formData.id
        ? t('inventoryBatch.batchFormDialog.titleEdit')
        : t('inventoryBatch.batchFormDialog.titleCreate')
    "
    width="600px"
    :aria-label="
      formData.id
        ? t('inventoryBatch.batchFormDialog.ariaLabelEdit')
        : t('inventoryBatch.batchFormDialog.ariaLabelCreate')
    "
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      :rules="formRules"
      label-width="100px"
      :aria-label="t('inventoryBatch.batchFormDialog.ariaLabelForm')"
    >
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelBatchNo')" prop="batch_no">
        <el-input v-model="formData.batch_no" :disabled="!!formData.id" />
      </el-form-item>
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelProductName')" prop="product_id">
        <el-select v-model="formData.product_id" filterable style="width: 100%">
          <el-option
            v-for="p in productOptions"
            :key="p.id"
            :label="p.product_name"
            :value="p.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelWarehouse')" prop="warehouse_id">
        <el-select v-model="formData.warehouse_id" filterable style="width: 100%">
          <el-option
            v-for="w in warehouseOptions"
            :key="w.id"
            :label="w.warehouse_name"
            :value="w.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelColorNo')" prop="color_no">
        <el-input v-model="formData.color_no" />
      </el-form-item>
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelDyeLotNo')" prop="dye_lot_no">
        <el-input v-model="formData.dye_lot_no" />
      </el-form-item>
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelGrade')" prop="grade">
        <el-select v-model="formData.grade" style="width: 100%">
          <el-option
            :label="t('inventoryBatch.batchFormDialog.optionGradeFirst')"
            :value="STOCK_GRADE.first"
          />
          <el-option
            :label="t('inventoryBatch.batchFormDialog.optionGradeSecond')"
            :value="STOCK_GRADE.second"
          />
          <el-option
            :label="t('inventoryBatch.batchFormDialog.optionGradeThird')"
            :value="STOCK_GRADE.offGrade"
          />
        </el-select>
      </el-form-item>
      <el-form-item
        :label="t('inventoryBatch.batchFormDialog.labelQuantityMeters')"
        prop="quantity_meters"
      >
        <el-input-number v-model="formData.quantity_meters" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelQuantityKg')" prop="quantity_kg">
        <el-input-number
          v-model="formData.quantity_kg"
          :min="0"
          :precision="2"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelGramWeight')" prop="gram_weight">
        <el-input-number v-model="formData.gram_weight" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelWidth')" prop="width">
        <el-input-number v-model="formData.width" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item
        :label="t('inventoryBatch.batchFormDialog.labelProductionDate')"
        prop="production_date"
      >
        <el-date-picker
          v-model="formData.production_date"
          type="date"
          value-format="YYYY-MM-DD"
          style="width: 100%"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('inventoryBatch.batchFormDialog.buttonCancel')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('inventoryBatch.batchFormDialog.buttonSave')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import {
  createBatch,
  updateBatch,
  type CreateBatchRequest,
  type UpdateBatchRequest,
  type InventoryBatch,
} from '@/api/inventory-batch';
import { getProductList, type Product } from '@/api/product';
import { getWarehouseList, type Warehouse } from '@/api/warehouse';
import { logAuxLoadFailure, logger } from '@/utils/logger';
import { STOCK_GRADE } from '@/constants/stock-grade';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  currentRow: InventoryBatch | null;
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
  (e: 'submitted'): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const formRef = ref<FormInstance>();
const submitLoading = ref(false);

// 与 CreateBatchRequest/UpdateBatchRequest 逐字段对齐的表单模型。
// quantity_meters/quantity_kg/gram_weight/width 是后端 f64 入参（非出参 Decimal），保持 number。
const formData = reactive({
  id: 0,
  batch_no: '',
  product_id: 0,
  warehouse_id: 0,
  color_no: '',
  dye_lot_no: '',
  grade: STOCK_GRADE.first as string,
  quantity_meters: 0,
  quantity_kg: 0,
  gram_weight: 0,
  width: 0,
  production_date: '',
});

// product_id / warehouse_id 必须来自真实主数据选择器，禁止硬编码 id 或 mock 数组：
// 打开弹窗时按需拉取产品/仓库列表（单次，失败降级为空列表并记录日志）。
const productOptions = ref<Product[]>([]);
const warehouseOptions = ref<Warehouse[]>([]);
const optionsLoaded = ref(false);

const loadOptions = async () => {
  if (optionsLoaded.value) return;
  try {
    const res = await getProductList({ page: 1, page_size: 1000 });
    productOptions.value = res.data.items;
  } catch (error) {
    logAuxLoadFailure(t('inventoryBatch.batchFormDialog.messageLoadProductsFailed'), error);
    productOptions.value = [];
  }
  try {
    const res = await getWarehouseList({ page: 1, page_size: 1000 });
    warehouseOptions.value = res.data.items;
  } catch (error) {
    logAuxLoadFailure(t('inventoryBatch.batchListTab.messageLoadWarehousesFailed'), error);
    warehouseOptions.value = [];
  }
  optionsLoaded.value = true;
};

/** 校验规则：computed 确保语言切换后规则消息响应式更新 */
const formRules = computed<FormRules>(() => ({
  batch_no: [
    {
      required: true,
      message: t('inventoryBatch.batchFormDialog.ruleBatchNoRequired'),
      trigger: 'blur',
    },
  ],
  product_id: [
    {
      required: true,
      validator: (_rule, value, callback) => {
        if (!value || Number(value) <= 0) {
          callback(new Error(t('inventoryBatch.batchFormDialog.ruleProductIdRequired')));
        } else {
          callback();
        }
      },
      trigger: 'change',
    },
  ],
  warehouse_id: [
    {
      required: true,
      validator: (_rule, value, callback) => {
        if (!value || Number(value) <= 0) {
          callback(new Error(t('inventoryBatch.batchFormDialog.ruleWarehouseRequired')));
        } else {
          callback();
        }
      },
      trigger: 'change',
    },
  ],
}));

const resetForm = () => {
  formData.id = 0;
  formData.batch_no = '';
  formData.product_id = 0;
  formData.warehouse_id = 0;
  formData.color_no = '';
  formData.dye_lot_no = '';
  formData.grade = STOCK_GRADE.first;
  formData.quantity_meters = 0;
  formData.quantity_kg = 0;
  formData.gram_weight = 0;
  formData.width = 0;
  formData.production_date = '';
};

watch(
  () => props.modelValue,
  val => {
    if (!val) return;
    loadOptions();
    if (props.currentRow) {
      const row = props.currentRow;
      formData.id = row.id;
      formData.batch_no = row.batch_no;
      formData.product_id = row.product_id;
      formData.warehouse_id = row.warehouse_id;
      formData.color_no = row.color_no;
      formData.dye_lot_no = row.dye_lot_no ?? '';
      formData.grade = row.grade;
      // 出参 quantity/gram/width 是 Decimal 字符串，回填到 number 表单前在边界显式转数
      formData.quantity_meters = Number(row.quantity_meters) || 0;
      formData.quantity_kg = Number(row.quantity_kg) || 0;
      formData.gram_weight = Number(row.gram_weight) || 0;
      formData.width = Number(row.width) || 0;
      formData.production_date = row.production_date ? row.production_date.slice(0, 10) : '';
    } else {
      resetForm();
    }
  }
);

const handleSubmit = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    submitLoading.value = true;
    try {
      if (formData.id) {
        const payload: UpdateBatchRequest = {
          color_no: formData.color_no || undefined,
          dye_lot_no: formData.dye_lot_no || undefined,
          grade: formData.grade,
          gram_weight: formData.gram_weight,
          width: formData.width,
        };
        await updateBatch(formData.id, payload);
      } else {
        const payload: CreateBatchRequest = {
          batch_no: formData.batch_no,
          product_id: formData.product_id,
          warehouse_id: formData.warehouse_id,
          color_no: formData.color_no,
          grade: formData.grade,
          quantity_meters: formData.quantity_meters,
          quantity_kg: formData.quantity_kg,
          gram_weight: formData.gram_weight,
          width: formData.width,
          dye_lot_no: formData.dye_lot_no || undefined,
          // 后端 production_date 是 DateTime<Utc>，date-picker 的 YYYY-MM-DD 须在边界转 RFC3339
          production_date: formData.production_date
            ? new Date(`${formData.production_date}T00:00:00Z`).toISOString()
            : undefined,
        };
        await createBatch(payload);
      }
      ElMessage.success(t('inventoryBatch.batchFormDialog.messageSuccess'));
      emit('update:modelValue', false);
      emit('submitted');
    } catch (error) {
      ElMessage.error(
        (error as Error).message || t('inventoryBatch.batchFormDialog.messageFailed')
      );
      logger.error(t('inventoryBatch.batchFormDialog.messageSaveFailed'), (error as Error).message);
    } finally {
      submitLoading.value = false;
    }
  });
};
</script>
