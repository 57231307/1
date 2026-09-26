<!--
  SupplierProductColorFormDialog - 单条供应商商品色号新建/编辑对话框
  extra_cost 为字符串承载 Decimal，表单直接以文本录入，不做数值兜底。
-->
<template>
  <el-dialog
    :model-value="proc.formVisible.value"
    :title="proc.isEdit.value ? t('supplierProduct.color.editTitle') : t('supplierProduct.color.createTitle')"
    width="520px"
    destroy-on-close
    append-to-body
    @update:model-value="(v: boolean) => (proc.formVisible.value = v)"
  >
    <el-form :model="proc.form" label-width="120px">
      <el-form-item :label="t('supplierProduct.color.form.colorNo')">
        <el-input
          v-model="proc.form.color_no"
          :placeholder="t('supplierProduct.color.formPlaceholders.colorNo')"
          maxlength="50"
        />
      </el-form-item>

      <el-form-item :label="t('supplierProduct.color.form.colorName')">
        <el-input
          v-model="proc.form.color_name"
          :placeholder="t('supplierProduct.color.formPlaceholders.colorName')"
          maxlength="100"
        />
      </el-form-item>

      <el-form-item :label="t('supplierProduct.color.form.pantoneCode')">
        <el-input
          v-model="proc.form.pantone_code"
          :placeholder="t('supplierProduct.color.formPlaceholders.pantoneCode')"
          maxlength="50"
        />
      </el-form-item>

      <el-form-item :label="t('supplierProduct.color.form.extraCost')">
        <el-input
          v-model="proc.form.extra_cost"
          :placeholder="t('supplierProduct.color.formPlaceholders.extraCost')"
        />
      </el-form-item>

      <el-form-item :label="t('supplierProduct.color.form.isEnabled')">
        <el-switch v-model="proc.form.is_enabled" />
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="proc.formVisible.value = false">{{ t('common.cancel') }}</el-button>
      <el-button type="primary" :loading="proc.saving.value" @click="proc.submit">
        {{ t('common.confirm') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { useSupplierProductColor } from '../composables/useSupplierProductColor';

const { t } = useI18n({ useScope: 'global' });

type ProcState = ReturnType<typeof useSupplierProductColor>;

defineProps<{
  proc: ProcState;
}>();
</script>
