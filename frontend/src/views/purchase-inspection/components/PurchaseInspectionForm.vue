<!--
  PurchaseInspectionForm.vue - 采购验货新建/编辑表单对话框
  拆分自 purchase-inspection/index.vue（P14 批 2 I-3 第 5 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="
      isEdit ? t('purchaseInspection.form.title.edit') : t('purchaseInspection.form.title.create')
    "
    width="800px"
    :aria-label="
      isEdit
        ? t('purchaseInspection.form.ariaLabel.edit')
        : t('purchaseInspection.form.ariaLabel.create')
    "
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      ref="formRef"
      :model="localFormData"
      :rules="rules"
      label-width="100px"
      :aria-label="t('purchaseInspection.form.ariaLabel.form')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchaseInspection.form.label.receiptNo')" prop="receipt_id">
            <el-select
              :model-value="localFormData.receipt_id"
              :placeholder="t('purchaseInspection.form.placeholder.receiptNo')"
              filterable
              @update:model-value="(v: number) => emit('receipt-change', v)"
            >
              <el-option
                v-for="receipt in receipts"
                :key="receipt.id"
                :label="receipt.receipt_no"
                :value="receipt.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="t('purchaseInspection.form.label.inspectionDate')"
            prop="inspection_date"
          >
            <el-date-picker
              :model-value="localFormData.inspection_date"
              type="date"
              :placeholder="t('purchaseInspection.form.placeholder.inspectionDate')"
              value-format="YYYY-MM-DD"
              @update:model-value="(v: string) => (localFormData.inspection_date = v)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('purchaseInspection.form.label.remark')">
        <el-input
          :model-value="localFormData.remark"
          type="textarea"
          :rows="3"
          :placeholder="t('purchaseInspection.form.placeholder.remark')"
          @update:model-value="(v: string) => (localFormData.remark = v)"
        />
      </el-form-item>

      <!-- 检验明细 -->
      <el-divider content-position="left">{{
        t('purchaseInspection.form.divider.items')
      }}</el-divider>
      <el-table
        :data="localFormData.items"
        border
        :aria-label="t('purchaseInspection.form.ariaLabel.itemsTable')"
      >
        <el-table-column
          prop="product_name"
          :label="t('purchaseInspection.form.column.productName')"
          min-width="150"
        />
        <el-table-column
          prop="expected_quantity"
          :label="t('purchaseInspection.form.column.expectedQuantity')"
          width="100"
        />
        <el-table-column
          prop="inspected_quantity"
          :label="t('purchaseInspection.form.column.inspectedQuantity')"
          width="120"
        >
          <template #default="{ row }">
            <el-input-number
              :model-value="row.inspected_quantity"
              :min="0"
              size="small"
              @update:model-value="(v: number) => (row.inspected_quantity = v)"
            />
          </template>
        </el-table-column>
        <el-table-column
          prop="passed_quantity"
          :label="t('purchaseInspection.form.column.passedQuantity')"
          width="120"
        >
          <template #default="{ row }">
            <el-input-number
              :model-value="row.passed_quantity"
              :min="0"
              size="small"
              @update:model-value="(v: number) => (row.passed_quantity = v)"
            />
          </template>
        </el-table-column>
        <el-table-column
          prop="failed_quantity"
          :label="t('purchaseInspection.form.column.failedQuantity')"
          width="120"
        >
          <template #default="{ row }">
            <el-input-number
              :model-value="row.failed_quantity"
              :min="0"
              size="small"
              @update:model-value="(v: number) => (row.failed_quantity = v)"
            />
          </template>
        </el-table-column>
        <el-table-column
          prop="defect_reason"
          :label="t('purchaseInspection.form.column.defectReason')"
          min-width="150"
        >
          <template #default="{ row }">
            <el-input
              :model-value="row.defect_reason"
              size="small"
              @update:model-value="(v: string) => (row.defect_reason = v)"
            />
          </template>
        </el-table-column>
      </el-table>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('purchaseInspection.form.button.cancel')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('purchaseInspection.form.button.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { deepClone } from '@/utils';
import { ref, watch, nextTick } from 'vue';
import { type FormInstance, type FormRules } from 'element-plus';
import type { PurchaseInspectionItem } from '@/api/purchase-inspection';

const { t } = useI18n({ useScope: 'global' });

// 表单数据类型（所有字段可选，兼容父组件 reactive）
interface PurchaseInspectionFormData {
  id?: number;
  receipt_id?: number;
  inspection_date: string;
  remark: string;
  items: Partial<PurchaseInspectionItem>[];
}

const props = defineProps<{
  // 可见性
  visible: boolean;
  // 是否编辑
  isEdit: boolean;
  // 表单数据（由父组件管理，子组件通过 emit('update:formData') 回写）
  formData: PurchaseInspectionFormData;
  // 验证规则
  rules: FormRules;
  // 提交加载
  submitLoading: boolean;
  // 入库单列表
  receipts: { id: number; receipt_no: string }[];
}>();

const emit = defineEmits<{
  (e: 'update:visible', v: boolean): void;
  // 入库单变化（父组件加载明细）
  (e: 'receipt-change', receiptId: number): void;
  // 提交（父组件处理 API）
  (e: 'submit'): void;
  // 整体回写表单数据（父组件监听此事件并 Object.assign 到自己的 formData）
  (e: 'update:formData', formData: PurchaseInspectionFormData): void;
}>();

// 表单 ref
const formRef = ref<FormInstance>();

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// 注意：表单内有 items 数组，需要深拷贝以保证本地修改与父组件解耦
const localFormData = ref<PurchaseInspectionFormData>(deepClone(props.formData));

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local（如父组件编辑/新建时填充数据）
watch(
  () => props.formData,
  newData => {
    if (syncing) return;
    syncing = true;
    localFormData.value = deepClone(newData);
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 本地变化时通知父组件（用户输入）
watch(
  localFormData,
  newData => {
    if (syncing) return;
    syncing = true;
    emit('update:formData', deepClone(newData));
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 暴露给父组件
defineExpose({ formRef });

/** 提交（先校验，再通知父组件） */
const handleSubmit = async () => {
  if (!formRef.value) return;
  try {
    await formRef.value.validate();
    emit('submit');
  } catch {
    // 校验失败
  }
};
</script>
