<!--
  SupplierProductFormDialog - 供应商商品新建/编辑对话框
  遵循对照表页 proc(reactive 代理) + el-form 范式。
-->
<template>
  <el-dialog
    :model-value="proc.dialogVisible.value"
    :title="proc.isEdit.value ? t('supplierProduct.editTitle') : t('supplierProduct.createTitle')"
    width="560px"
    destroy-on-close
    @update:model-value="(v: boolean) => (proc.dialogVisible.value = v)"
  >
    <el-form :model="proc.form" label-width="110px">
      <el-form-item :label="t('supplierProduct.form.supplier')">
        <el-select
          v-model="proc.form.supplier_id"
          :placeholder="t('supplierProduct.formPlaceholders.supplier')"
          filterable
          :disabled="proc.isEdit.value"
          style="width: 100%"
        >
          <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
        </el-select>
      </el-form-item>

      <el-form-item :label="t('supplierProduct.form.productCode')">
        <el-input
          v-model="proc.form.product_code"
          :placeholder="t('supplierProduct.formPlaceholders.productCode')"
          maxlength="100"
        />
      </el-form-item>

      <el-form-item :label="t('supplierProduct.form.productName')">
        <el-input
          v-model="proc.form.product_name"
          :placeholder="t('supplierProduct.formPlaceholders.productName')"
          maxlength="200"
        />
      </el-form-item>

      <el-form-item :label="t('supplierProduct.form.unit')">
        <el-input
          v-model="proc.form.unit"
          :placeholder="t('supplierProduct.formPlaceholders.unit')"
          maxlength="20"
        />
      </el-form-item>

      <el-form-item :label="t('supplierProduct.form.productDescription')">
        <el-input
          v-model="proc.form.product_description"
          type="textarea"
          :rows="2"
          :placeholder="t('supplierProduct.formPlaceholders.productDescription')"
        />
      </el-form-item>

      <el-form-item :label="t('supplierProduct.form.isEnabled')">
        <el-switch v-model="proc.form.is_enabled" />
      </el-form-item>

      <el-form-item :label="t('supplierProduct.form.remarks')">
        <el-input
          v-model="proc.form.remarks"
          type="textarea"
          :rows="2"
          :placeholder="t('supplierProduct.formPlaceholders.remarks')"
        />
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
import { useI18n } from 'vue-i18n';
import type { Supplier } from '@/api/supplier';
import type { useSupplierProductDialog } from '../composables/useSupplierProductDialog';

const { t } = useI18n({ useScope: 'global' });

type ProcState = ReturnType<typeof useSupplierProductDialog>;

defineProps<{
  proc: ProcState;
  suppliers: Supplier[];
}>();
</script>
