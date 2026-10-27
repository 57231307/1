<!--
  SkuMappingBatchDialog - SKU 对照表批量添加对话框（多行录入）

  表头定住「我方产品 + 供应商 + 供应商商品」，一次勾选多个我方色号即生成多行，
  每行再单独填供应商色号/协议价/最小起订量/交期/优先级/主供/启用，可手工增删行。
  行表用 V2Table（el-table-v2 虚拟滚动），几百行录入不卡；数值列一律 el-input + 校验器，
  不使用带加减按钮的数字输入框。
  提交前界面先跑逐行校验并汇总点名行号与字段；后端整批原子，任一行不合格零落库。
-->
<template>
  <el-dialog
    :model-value="proc.batchDialogVisible.value"
    :title="t('skuMapping.batchTitle')"
    width="1200px"
    top="5vh"
    destroy-on-close
    @update:model-value="(v: boolean) => (proc.batchDialogVisible.value = v)"
  >
    <el-form :inline="true" class="batch-head">
      <el-form-item :label="t('skuMapping.fields.product')">
        <el-select
          v-model="proc.head.product_id"
          filterable
          clearable
          :placeholder="t('skuMapping.placeholders.product')"
          style="width: 240px"
          @change="proc.handleHeadProductChange"
        >
          <el-option
            v-for="p in products"
            :key="p.id"
            :label="`${p.product_code} - ${p.product_name}`"
            :value="p.id"
          />
        </el-select>
      </el-form-item>

      <el-form-item :label="t('skuMapping.fields.supplier')">
        <el-select
          v-model="proc.head.supplier_id"
          filterable
          clearable
          :placeholder="t('skuMapping.placeholders.supplier')"
          style="width: 200px"
          @change="proc.handleHeadSupplierChange"
        >
          <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
        </el-select>
      </el-form-item>

      <el-form-item :label="t('skuMapping.fields.supplierProduct')">
        <el-select-v2
          v-model="proc.head.supplier_product_id"
          :options="proc.supplierProductOptions.value"
          :placeholder="
            proc.head.supplier_id
              ? t('skuMapping.placeholders.supplierProduct')
              : t('skuMapping.placeholders.supplierFirst')
          "
          :disabled="!proc.head.supplier_id"
          filterable
          remote
          clearable
          :remote-method="proc.searchSupplierProducts"
          :loading="proc.supplierProductLoading.value"
          style="width: 240px"
          @change="proc.handleHeadSupplierProductChange"
        />
      </el-form-item>

      <el-form-item :label="t('skuMapping.batch.pickColors')">
        <el-select-v2
          v-model="proc.selectedColorIds.value"
          :options="proc.colorSelectOptions.value"
          :placeholder="t('skuMapping.batch.pickColorsPlaceholder')"
          :disabled="!proc.head.product_id"
          :loading="proc.colorLoading.value"
          filterable
          multiple
          clearable
          :max-collapse-tags="3"
          style="width: 320px"
        />
      </el-form-item>
    </el-form>

    <div class="batch-toolbar">
      <span class="batch-count">{{ t('skuMapping.batch.rowCount', { count: rowCount }) }}</span>
      <el-button link type="primary" @click="proc.addBlankRow()">
        {{ t('skuMapping.batch.addRow') }}
      </el-button>
      <el-button link :disabled="rowCount === 0" @click="proc.clearAllRows()">
        {{ t('skuMapping.batch.clearRows') }}
      </el-button>
      <el-button
        link
        :disabled="proc.existingLoading.value || !proc.head.product_id || !proc.head.supplier_id"
        @click="proc.reloadExistingKeys()"
      >
        {{ t('skuMapping.batch.reloadExisting') }}
      </el-button>
      <span v-if="proc.existingTruncated.value" class="batch-warn">
        {{ t('skuMapping.batch.existingTruncated') }}
      </span>
      <span v-else-if="proc.head.product_id && proc.head.supplier_id" class="batch-hint">
        {{ t('skuMapping.batch.existingCount', { count: proc.existingCount.value }) }}
      </span>
      <span v-if="proc.lastSummary.value" :class="summaryClass">
        {{ proc.lastSummary.value.text }}
      </span>
    </div>

    <V2Table
      :columns="columns"
      :data="proc.rows"
      :height="340"
      row-key="_key"
      :estimated-row-height="44"
      :empty-text="t('skuMapping.batch.emptyRows')"
    />

    <div v-if="proc.summaryText.value.length > 0" class="batch-summary">
      <el-alert type="error" :closable="false" show-icon>
        <template #title>
          {{ t('skuMapping.batch.rejectTitle', { count: proc.summaryText.value.length }) }}
        </template>
        <ul class="batch-summary-list">
          <li v-for="(line, idx) in proc.summaryText.value" :key="idx">{{ line }}</li>
        </ul>
      </el-alert>
    </div>

    <template #footer>
      <el-button @click="proc.batchDialogVisible.value = false">
        {{ t('common.cancel') }}
      </el-button>
      <el-button
        type="primary"
        :disabled="!proc.canSubmit.value"
        :loading="proc.submitting.value"
        @click="proc.submit()"
      >
        {{ t('skuMapping.batch.submit') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, h, inject } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElButton, ElInput, ElSelectV2, ElSwitch, ElTag } from 'element-plus';
import V2Table from '@/components/V2Table/index.vue';
import type { ColumnDef } from '@/components/V2Table/types';
import type { Product } from '@/api/product';
import type { Supplier } from '@/api/supplier';
import type { SkuMappingBatchProc, SkuMappingBatchRow } from '../composables/useSkuMappingBatch';

const { t } = useI18n({ useScope: 'global' });

const proc = inject<SkuMappingBatchProc>('skuMappingBatchProc')!;

defineProps<{
  products: Product[];
  suppliers: Supplier[];
}>();

const rowCount = computed(() => proc.rows.length);

const summaryClass = computed(() => (proc.lastSummary.value?.ok ? 'batch-hint' : 'batch-warn'));

/** 行号按当前行序现算（删行即前移），与上送 items 的 1 基下标、后端回传的 row 同一口径。 */
const rowNoOf = (row: SkuMappingBatchRow): number => proc.rows.indexOf(row) + 1;

const renderTextCell = (
  row: SkuMappingBatchRow,
  column: 'supplier_price' | 'min_order_quantity' | 'lead_time' | 'priority',
  placeholder: string
) =>
  h(ElInput, {
    modelValue: row[column],
    size: 'small',
    placeholder,
    'onUpdate:modelValue': (value: string) => {
      row[column] = value;
    },
  });

const renderSwitchCell = (row: SkuMappingBatchRow, column: 'is_primary' | 'is_enabled') =>
  h(ElSwitch, {
    modelValue: row[column],
    'onUpdate:modelValue': (value: string | number | boolean) => {
      row[column] = Boolean(value);
    },
  });

const columns = computed<ColumnDef<SkuMappingBatchRow>[]>(() => [
  {
    key: 'row_no',
    title: t('skuMapping.batch.columns.rowNo'),
    width: 60,
    align: 'center',
    formatter: (row: SkuMappingBatchRow) => String(rowNoOf(row)),
  },
  {
    key: 'product_color_id',
    title: t('skuMapping.batch.columns.ourColor'),
    width: 190,
    renderCell: (row: SkuMappingBatchRow) =>
      h(ElSelectV2, {
        modelValue: row.product_color_id,
        options: proc.colorSelectOptions.value,
        size: 'small',
        filterable: true,
        placeholder: t('skuMapping.placeholders.ourColor'),
        'onUpdate:modelValue': (value: number | null) => proc.applyRowColor(row, value),
      }),
  },
  {
    key: 'supplier_product_color_id',
    title: t('skuMapping.batch.columns.supplierColor'),
    width: 190,
    renderCell: (row: SkuMappingBatchRow) =>
      h(ElSelectV2, {
        modelValue: row.supplier_product_color_id,
        options: proc.supplierColorOptions.value,
        size: 'small',
        filterable: true,
        clearable: true,
        disabled: !proc.head.supplier_product_id,
        placeholder: proc.head.supplier_product_id
          ? t('skuMapping.placeholders.supplierColor')
          : t('skuMapping.placeholders.supplierProductFirst'),
        'onUpdate:modelValue': (value: number | null) => proc.applyRowSupplierColor(row, value),
      }),
  },
  {
    key: 'supplier_price',
    title: t('skuMapping.batch.columns.price'),
    width: 110,
    renderCell: (row: SkuMappingBatchRow) =>
      renderTextCell(row, 'supplier_price', t('skuMapping.placeholders.supplierPrice')),
  },
  {
    key: 'min_order_quantity',
    title: t('skuMapping.batch.columns.minOrderQuantity'),
    width: 110,
    renderCell: (row: SkuMappingBatchRow) =>
      renderTextCell(row, 'min_order_quantity', t('skuMapping.placeholders.minOrderQuantity')),
  },
  {
    key: 'lead_time',
    title: t('skuMapping.batch.columns.leadTime'),
    width: 90,
    renderCell: (row: SkuMappingBatchRow) =>
      renderTextCell(row, 'lead_time', t('skuMapping.placeholders.leadTime')),
  },
  {
    key: 'priority',
    title: t('skuMapping.batch.columns.priority'),
    width: 90,
    renderCell: (row: SkuMappingBatchRow) =>
      renderTextCell(row, 'priority', t('skuMapping.placeholders.priority')),
  },
  {
    key: 'is_primary',
    title: t('skuMapping.batch.columns.isPrimary'),
    width: 70,
    align: 'center',
    renderCell: (row: SkuMappingBatchRow) => renderSwitchCell(row, 'is_primary'),
  },
  {
    key: 'is_enabled',
    title: t('skuMapping.batch.columns.isEnabled'),
    width: 70,
    align: 'center',
    renderCell: (row: SkuMappingBatchRow) => renderSwitchCell(row, 'is_enabled'),
  },
  {
    key: 'validation',
    title: t('skuMapping.batch.columns.status'),
    width: 120,
    renderCell: (row: SkuMappingBatchRow) => {
      const list = proc.errorsByRowKey.value.get(row._key);
      if (!list || list.length === 0) {
        return h(ElTag, { type: 'success', size: 'small' }, () => t('skuMapping.batch.rowOk'));
      }
      return h(
        ElTag,
        { type: 'danger', size: 'small' },
        () => `${t('skuMapping.batch.rowBad')} ${list.length}`
      );
    },
  },
  {
    key: 'operation',
    title: t('common.operation'),
    width: 80,
    fixed: 'right',
    renderCell: (row: SkuMappingBatchRow) =>
      h(
        ElButton,
        { type: 'danger', link: true, size: 'small', onClick: () => proc.removeRow(row._key) },
        () => t('common.delete')
      ),
  },
]);
</script>

<style scoped>
.batch-head {
  margin-bottom: 4px;
}

.batch-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
}

.batch-count {
  font-weight: 600;
}

.batch-hint {
  color: var(--el-text-color-secondary);
}

.batch-warn {
  color: var(--el-color-warning);
}

.batch-summary {
  margin-top: 12px;
}

.batch-summary-list {
  margin: 4px 0 0;
  padding-left: 18px;
  text-align: left;
}
</style>
