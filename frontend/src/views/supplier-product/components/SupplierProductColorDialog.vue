<!--
  SupplierProductColorDialog - 供应商商品色号维护对话框
  外层列表（搜索/分页）+ 内层单条色号表单对话框；表被 FK 引用无硬删，停用走 is_enabled。
-->
<template>
  <el-dialog
    :model-value="proc.dialogVisible.value"
    :title="t('supplierProduct.color.title')"
    width="860px"
    destroy-on-close
    @update:model-value="(v: boolean) => (proc.dialogVisible.value = v)"
  >
    <div class="color-context">
      {{ t('supplierProduct.color.currentProduct') }}:
      <strong>{{ currentProductLabel }}</strong>
    </div>

    <div class="color-toolbar">
      <el-input
        v-model="proc.queryParams.keyword"
        :placeholder="t('supplierProduct.filter.keyword')"
        clearable
        style="width: 220px"
        @keyup.enter="proc.handleQuery"
        @clear="proc.handleQuery"
      />
      <el-button type="primary" @click="proc.handleQuery">{{ t('common.search') }}</el-button>
      <el-button v-permission="PERMISSIONS.SUPPLIER_PRODUCT_COLOR_CREATE" @click="proc.openCreate()">
        {{ t('supplierProduct.color.create') }}
      </el-button>
    </div>

    <el-table v-loading="proc.loading.value" :data="proc.list.value" border stripe>
      <el-table-column
        prop="color_no"
        :label="t('supplierProduct.color.columns.colorNo')"
        width="140"
      />
      <el-table-column
        prop="color_name"
        :label="t('supplierProduct.color.columns.colorName')"
        min-width="140"
      />
      <el-table-column
        prop="pantone_code"
        :label="t('supplierProduct.color.columns.pantoneCode')"
        width="150"
      />
      <el-table-column :label="t('supplierProduct.color.columns.extraCost')" width="120">
        <template #default="{ row }">{{ formatCost(row.extra_cost) }}</template>
      </el-table-column>
      <el-table-column :label="t('supplierProduct.color.columns.isEnabled')" width="90" align="center">
        <template #default="{ row }">
          <el-tag v-if="row.is_enabled" type="success" size="small">{{ t('common.yes') }}</el-tag>
          <el-tag v-else type="danger" size="small">{{ t('common.no') }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.operation')" width="160" fixed="right">
        <template #default="{ row }">
          <el-button
            v-permission="PERMISSIONS.SUPPLIER_PRODUCT_COLOR_UPDATE"
            link
            type="primary"
            size="small"
            @click="proc.openEdit(row)"
          >
            {{ t('common.edit') }}
          </el-button>
          <el-button
            v-permission="PERMISSIONS.SUPPLIER_PRODUCT_COLOR_UPDATE"
            link
            size="small"
            @click="proc.toggleEnabled(row)"
          >
            {{ row.is_enabled ? t('common.disable') : t('common.enable') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-pagination
      v-model:current-page="proc.queryParams.page"
      v-model:page-size="proc.queryParams.page_size"
      :total="proc.total.value"
      :page-sizes="[20, 50, 100]"
      layout="total, sizes, prev, pager, next"
      class="pagination"
      @size-change="proc.handleSizeChange"
      @current-change="proc.handlePageChange"
    />

    <SupplierProductColorFormDialog :proc="proc" />
  </el-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { PERMISSIONS } from '@/constants/permissions';
import type { useSupplierProductColor } from '../composables/useSupplierProductColor';
import SupplierProductColorFormDialog from './SupplierProductColorFormDialog.vue';

const { t } = useI18n({ useScope: 'global' });

type ProcState = ReturnType<typeof useSupplierProductColor>;

const props = defineProps<{
  proc: ProcState;
}>();

const currentProductLabel = computed(() => {
  const p = props.proc.currentProduct.value;
  return p ? `${p.product_code} - ${p.product_name}` : '';
});

// extra_cost 为字符串承载 Decimal，展示前先 Number() 归一，禁止对字符串直接 .toFixed
const formatCost = (raw: string): string => {
  const n = Number(raw);
  return Number.isFinite(n) ? n.toFixed(2) : '';
};
</script>

<style scoped>
.color-context {
  margin-bottom: 12px;
  font-size: 14px;
}

.color-toolbar {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}

.pagination {
  margin-top: 16px;
  justify-content: flex-end;
}
</style>
