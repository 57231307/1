<!--
  AdjustmentDialog.vue - 库存调整对话框
  功能：采集调整目标（产品与仓库或预置库存行）、增减方向、调整量与原因，submit 时把
  表单回传给父组件（views/inventory/index.vue 的 onSubmitAdjustment）由父侧定位库存行并调
  createStockAdjustment 建调整单。
  调整量为十进制串（后端 inventory_adjustment_items.quantity 是 Decimal，入参按串传避免
  浮点失真），范围与精度由 el-form 校验器保证（有限正数），全项目禁步进数字框。
  修复（缺陷A）：从工具栏进入时（无预置 product_name/warehouse_name），
  渲染可选 el-select 供用户选定目标库存；从行内进入时保留只读回显。
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('inventory.adjustmentDialog.title')"
    width="500px"
    :close-on-click-modal="false"
    :aria-label="t('inventory.adjustmentDialog.ariaLabel')"
    @update:model-value="onClose"
  >
    <el-form
      ref="formRef"
      :model="localForm"
      :rules="rules"
      label-width="100px"
      :aria-label="t('inventory.adjustmentDialog.formAria')"
    >
      <!-- 产品：行内入口（有预置 name）→ 只读回显；工具栏入口 → 可选 el-select -->
      <el-form-item :label="t('inventory.adjustmentDialog.product')">
        <el-input v-if="localForm.product_name" :value="localForm.product_name" disabled />
        <el-select
          v-else
          v-model="localForm.product_id"
          filterable
          :placeholder="t('inventory.adjustmentDialog.productPlaceholder')"
          style="width: 100%"
        >
          <el-option
            v-for="p in products"
            :key="p.id"
            :label="`${p.product_code} - ${p.product_name}`"
            :value="p.id"
          />
        </el-select>
      </el-form-item>
      <!-- 仓库：同上逻辑 -->
      <el-form-item :label="t('inventory.adjustmentDialog.warehouse')">
        <el-input v-if="localForm.warehouse_name" :value="localForm.warehouse_name" disabled />
        <el-select
          v-else
          v-model="localForm.warehouse_id"
          :placeholder="t('inventory.adjustmentDialog.warehousePlaceholder')"
          style="width: 100%"
        >
          <el-option
            v-for="wh in warehouses"
            :key="wh.id"
            :label="wh.warehouse_name"
            :value="wh.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item
        v-if="localForm.current_quantity"
        :label="t('inventory.adjustmentDialog.currentQty')"
      >
        <el-input :value="localForm.current_quantity" disabled />
      </el-form-item>
      <el-form-item :label="t('inventory.adjustmentDialog.adjustType')">
        <el-radio-group v-model="localForm.adjustment_type">
          <el-radio value="increase">{{ t('inventory.adjustmentDialog.typeIncrease') }}</el-radio>
          <el-radio value="decrease">{{ t('inventory.adjustmentDialog.typeDecrease') }}</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item :label="t('inventory.adjustmentDialog.adjustQty')" prop="adjustment_quantity">
        <el-input
          v-model="localForm.adjustment_quantity"
          :placeholder="t('inventory.adjustmentDialog.qtyPlaceholder')"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item :label="t('inventory.adjustmentDialog.reason')">
        <el-input
          v-model="localForm.reason"
          type="textarea"
          :rows="3"
          :placeholder="t('inventory.adjustmentDialog.reasonPlaceholder')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onClose(false)">{{ t('inventory.adjustmentDialog.cancel') }}</el-button>
      <el-button type="primary" :loading="submitting" :disabled="submitting" @click="onSubmit">{{
        t('inventory.adjustmentDialog.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { deepClone } from '@/utils';
import { reactive, ref, watch } from 'vue';
import type { FormInstance, FormRules } from 'element-plus';
import type { Warehouse } from '@/api/warehouse';
import type { Product } from '@/api/product';

// 接入 i18n，替换硬编码中文文案
const { t } = useI18n({ useScope: 'global' });

// 库存调整表单数据结构（字段与 inventory 父组件 initialForm 保持一致）。
// adjustment_quantity 为十进制串：后端调整量是 Decimal，界面按串采集、校验器保证有限正数，
// 提交时由父侧原样传入 createStockAdjustment 明细 quantity 键。
export interface AdjustmentForm {
  stock_id: number | null;
  product_id: number | null;
  warehouse_id: number | null;
  product_name?: string;
  warehouse_name?: string;
  current_quantity?: number;
  adjustment_type: 'increase' | 'decrease';
  adjustment_quantity: string;
  reason: string;
}

const props = defineProps<{
  visible: boolean;
  initialForm: AdjustmentForm;
  warehouses: Warehouse[];
  products: Product[];
  // 提交在途（父组件管理）：为真时确认按钮转圈并禁用，避免重复生成调整单
  submitting: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void;
  (e: 'submit', data: AdjustmentForm): void;
}>();

// 浅拷贝 initialForm 同步初始值（不直接突变 prop）
const localForm = reactive<AdjustmentForm>({} as AdjustmentForm);
watch(
  () => props.initialForm,
  newVal => {
    // AdjustmentForm 字段固定，直接 Object.assign 覆盖即可（无需逐键 delete）
    Object.assign(localForm, deepClone(newVal));
  },
  { immediate: true, deep: true }
);

const formRef = ref<FormInstance>();

// 调整量校验器：必填且为有限正数（零与负数不构成调整，与后端受理门同口径），范围靠校验器不靠按钮
const rules: FormRules = {
  adjustment_quantity: [
    {
      validator: (_rule: unknown, value: string, callback: (err?: Error) => void) => {
        const raw = (value ?? '').trim();
        if (raw === '') {
          callback(new Error(t('inventory.adjustmentDialog.qtyRequired')));
          return;
        }
        const n = Number(raw);
        callback(
          Number.isFinite(n) && n > 0
            ? undefined
            : new Error(t('inventory.adjustmentDialog.qtyPositive'))
        );
      },
      trigger: 'blur',
    },
  ],
};

const onClose = (val: boolean) => {
  emit('update:visible', val);
};

const onSubmit = async () => {
  if (props.submitting) return;
  const valid = await formRef.value?.validate().catch(() => false);
  if (!valid) return;
  emit('submit', deepClone(localForm));
};
</script>
