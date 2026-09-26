<!--
  SkuMappingFormDialog - SKU 对照表新建/编辑对话框
  色号选择使用 el-select-v2 虚拟滚动，支持上千色号场景。
-->
<template>
  <el-dialog
    :model-value="proc.dialogVisible.value"
    :title="proc.isEdit.value ? t('skuMapping.editTitle') : t('skuMapping.createTitle')"
    width="680px"
    destroy-on-close
    @update:model-value="(v: boolean) => (proc.dialogVisible.value = v)"
  >
    <el-form :model="proc.form" label-width="130px">
      <el-form-item :label="t('skuMapping.fields.product')">
        <el-select
          v-model="proc.form.product_id"
          :placeholder="t('skuMapping.placeholders.product')"
          filterable
          style="width: 100%"
          @change="proc.handleProductChange"
        >
          <el-option
            v-for="p in products"
            :key="p.id"
            :label="`${p.product_code} - ${p.product_name}`"
            :value="p.id"
          />
        </el-select>
      </el-form-item>

      <el-form-item :label="t('skuMapping.fields.ourColor')">
        <el-select-v2
          v-model="proc.form.product_color_id"
          :options="colorV2Options"
          :placeholder="t('skuMapping.placeholders.ourColor')"
          filterable
          remote
          :remote-method="searchColors"
          :loading="proc.colorLoading.value"
          clearable
          style="width: 100%"
          @change="proc.handleColorChange"
        />
      </el-form-item>

      <el-form-item :label="t('skuMapping.fields.ourColorNo')">
        <el-input
          v-model="proc.form.color_no"
          :placeholder="t('skuMapping.placeholders.ourColorNo')"
          disabled
        />
      </el-form-item>

      <el-form-item :label="t('skuMapping.fields.supplier')">
        <el-select
          v-model="proc.form.supplier_id"
          :placeholder="t('skuMapping.placeholders.supplier')"
          filterable
          style="width: 100%"
        >
          <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
        </el-select>
      </el-form-item>

      <el-form-item :label="t('skuMapping.fields.supplierProductId')">
        <el-input
          v-model.number="proc.form.supplier_product_id"
          :placeholder="t('skuMapping.placeholders.supplierProductId')"
          type="number"
        />
      </el-form-item>

      <el-form-item :label="t('skuMapping.fields.supplierProductCode')">
        <el-input
          v-model="proc.form.supplier_product_code"
          :placeholder="t('skuMapping.placeholders.supplierProductCode')"
        />
      </el-form-item>

      <el-form-item :label="t('skuMapping.fields.supplierColorNo')">
        <el-input
          v-model="proc.form.supplier_color_no"
          :placeholder="t('skuMapping.placeholders.supplierColorNo')"
        />
      </el-form-item>

      <el-form-item :label="t('skuMapping.fields.supplierPrice')">
        <el-input
          v-model="proc.form.supplier_price"
          :placeholder="t('skuMapping.placeholders.supplierPrice')"
        />
      </el-form-item>

      <el-form-item :label="t('skuMapping.fields.minOrderQuantity')">
        <el-input
          v-model="proc.form.min_order_quantity"
          :placeholder="t('skuMapping.placeholders.minOrderQuantity')"
        />
      </el-form-item>

      <el-form-item :label="t('skuMapping.fields.leadTime')">
        <el-input-number
          v-model="proc.form.lead_time"
          :min="0"
          :placeholder="t('skuMapping.placeholders.leadTime')"
          style="width: 100%"
        />
      </el-form-item>

      <el-form-item :label="t('skuMapping.fields.isPrimary')">
        <el-switch v-model="proc.form.is_primary" />
      </el-form-item>

      <el-form-item :label="t('skuMapping.fields.isEnabled')">
        <el-switch v-model="proc.form.is_enabled" />
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="proc.dialogVisible.value = false">{{ t('common.cancel') }}</el-button>
      <el-button type="primary" :loading="proc.saving.value" @click="proc.submit">
        {{ t('common.confirm') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Product } from '@/api/product';
import type { Supplier } from '@/api/supplier';
import type { useSkuMappingDialog } from '../composables/useSkuMappingDialog';

const { t } = useI18n({ useScope: 'global' });

type ProcState = ReturnType<typeof useSkuMappingDialog>;

const props = defineProps<{
  proc: ProcState;
  products: Product[];
  suppliers: Supplier[];
}>();

// 将 colorOptions 转为 el-select-v2 需要的 {value, label}[] 格式
const colorV2Options = computed(() =>
  props.proc.colorOptions.value.map(c => ({
    value: c.id,
    label: `${c.color_no} - ${c.color_name}`,
  }))
);

// 远程搜索色号（虚拟滚动场景）
const searchColors = (query: string) => {
  if (props.proc.form.product_id) {
    props.proc.loadColors(props.proc.form.product_id, query);
  }
};
</script>
