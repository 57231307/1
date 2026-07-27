<!--
  SalesPriceForm.vue - 销售价格新建/编辑对话框
  拆分自 sales-price/index.vue（P14 批 2 I-3 第 3 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="700px"
    :close-on-click-modal="false"
    :aria-label="title"
    @update:model-value="onVisibleChange"
  >
    <el-form
      :model="localFormData"
      :rules="formRules"
      label-width="100px"
      :aria-label="t('salesPrice.form.ariaLabel')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesPrice.form.labelProduct')" prop="product_id">
            <el-select
              v-model="localFormData.product_id"
              :placeholder="t('salesPrice.form.placeholderProduct')"
              filterable
            >
              <el-option v-for="p in products" :key="p.id" :label="p.product_name" :value="p.id" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesPrice.form.labelCustomer')" prop="customer_id">
            <el-select
              v-model="localFormData.customer_id"
              :placeholder="t('salesPrice.form.placeholderCustomer')"
              filterable
              clearable
            >
              <el-option
                v-for="c in customers"
                :key="c.id"
                :label="c.customer_name"
                :value="c.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesPrice.form.labelPrice')" prop="price">
            <el-input-number
              v-model="localFormData.price"
              :precision="6"
              :min="0"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesPrice.form.labelCurrency')" prop="currency">
            <el-select
              v-model="localFormData.currency"
              :placeholder="t('salesPrice.form.placeholderCurrency')"
            >
              <el-option :label="t('salesPrice.form.optionCNY')" value="CNY" />
              <el-option :label="t('salesPrice.form.optionUSD')" value="USD" />
              <el-option :label="t('salesPrice.form.optionEUR')" value="EUR" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesPrice.form.labelUnit')" prop="unit">
            <el-select
              v-model="localFormData.unit"
              :placeholder="t('salesPrice.form.placeholderUnit')"
            >
              <el-option :label="t('salesPrice.form.optionMeter')" value="meter" />
              <el-option :label="t('salesPrice.form.optionKg')" value="kg" />
              <el-option :label="t('salesPrice.form.optionPiece')" value="piece" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesPrice.form.labelMinOrderQty')" prop="min_order_qty">
            <el-input-number
              v-model="localFormData.min_order_qty"
              :precision="2"
              :min="0"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesPrice.form.labelPriceType')" prop="price_type">
            <el-select
              v-model="localFormData.price_type"
              :placeholder="t('salesPrice.form.placeholderPriceType')"
            >
              <el-option :label="t('salesPrice.form.optionStandard')" value="STANDARD" />
              <el-option :label="t('salesPrice.form.optionAgreed')" value="AGREED" />
              <el-option :label="t('salesPrice.form.optionPromotion')" value="PROMOTION" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesPrice.form.labelPriceLevel')" prop="price_level">
            <el-select
              v-model="localFormData.price_level"
              :placeholder="t('salesPrice.form.placeholderPriceLevel')"
            >
              <el-option :label="t('salesPrice.form.optionLevelA')" value="A" />
              <el-option :label="t('salesPrice.form.optionLevelB')" value="B" />
              <el-option :label="t('salesPrice.form.optionLevelC')" value="C" />
              <el-option :label="t('salesPrice.form.optionLevelD')" value="D" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesPrice.form.labelEffectiveDate')" prop="effective_date">
            <el-date-picker
              v-model="localFormData.effective_date"
              type="date"
              :placeholder="t('salesPrice.form.placeholderEffectiveDate')"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesPrice.form.labelExpiryDate')" prop="expiry_date">
            <el-date-picker
              v-model="localFormData.expiry_date"
              type="date"
              :placeholder="t('salesPrice.form.placeholderExpiryDate')"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('salesPrice.form.labelRemarks')" prop="remarks">
        <el-input
          v-model="localFormData.remarks"
          type="textarea"
          :rows="3"
          :placeholder="t('salesPrice.form.placeholderRemarks')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('salesPrice.form.buttonCancel')
      }}</el-button>
      <el-button type="primary" @click="emit('submit')">{{
        t('salesPrice.form.buttonConfirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Customer } from '@/api/customer';
import type { Product } from '@/api/product';

const { t } = useI18n({ useScope: 'global' });

// 表单数据类型（所有字段可选，兼容 Partial<SalesPrice>）
interface SpFormData {
  id?: number | undefined;
  product_id?: number | undefined;
  customer_id?: number | undefined;
  price?: number;
  currency?: string;
  unit?: string;
  min_order_qty?: number;
  price_type?: string;
  price_level?: string;
  effective_date?: string;
  expiry_date?: string;
  remarks?: string;
}

// 表单校验规则
interface FormRules {
  product_id: Array<{ required: boolean; message: string; trigger: string }>;
  price: Array<{ required: boolean; message: string; trigger: string }>;
  currency: Array<{ required: boolean; message: string; trigger: string }>;
  unit: Array<{ required: boolean; message: string; trigger: string }>;
  effective_date: Array<{ required: boolean; message: string; trigger: string }>;
  price_type: Array<{ required: boolean; message: string; trigger: string }>;
}

/**
 * 销售价格新建/编辑对话框组件
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean;
  // 标题
  title: string;
  // 表单数据（由父组件管理，子组件通过 emit('update:formData') 回写）
  formData: SpFormData;
  // 表单校验规则
  formRules: FormRules;
  // 客户列表
  customers: Customer[];
  // 产品列表
  products: Product[];
}>();

const emit = defineEmits<{
  (e: 'update:visible', v: boolean): void;
  (e: 'submit'): void;
  // 整体回写表单数据（父组件监听此事件并 Object.assign 到自己的 formData）
  (e: 'update:formData', formData: SpFormData): void;
}>();

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localFormData = ref<SpFormData>({ ...props.formData });

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local（如父组件编辑/新建时填充数据）
watch(
  () => props.formData,
  newData => {
    if (syncing) return;
    syncing = true;
    localFormData.value = { ...newData };
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
    emit('update:formData', { ...newData });
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v);
};
</script>
