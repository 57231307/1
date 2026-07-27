<!--
  SupplierList.vue - 供应商列表子组件
  来源：原 supplier/index.vue 中 列表+筛选区（line 28-99）
  拆分日期：2026-06-17 P1-3-Batch-6
-->
<template>
  <div>
    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="localQuery"
        class="filter-form"
        :aria-label="t('supplier.list.filterAriaLabel')"
      >
        <el-form-item :label="t('supplier.list.filter.keyword')">
          <el-input
            v-model="localQuery.keyword"
            :placeholder="t('supplier.list.filter.keywordPlaceholder')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('supplier.list.filter.grade')">
          <el-select
            v-model="localQuery.grade"
            :placeholder="t('supplier.list.filter.gradePlaceholder')"
            clearable
          >
            <el-option :label="t('supplier.list.option.gradeA')" value="A" />
            <el-option :label="t('supplier.list.option.gradeB')" value="B" />
            <el-option :label="t('supplier.list.option.gradeC')" value="C" />
            <el-option :label="t('supplier.list.option.gradeD')" value="D" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('supplier.list.filter.status')">
          <el-select
            v-model="localQuery.status"
            :placeholder="t('supplier.list.filter.statusPlaceholder')"
            clearable
          >
            <el-option :label="t('supplier.list.option.statusActive')" value="active" />
            <el-option :label="t('supplier.list.option.statusInactive')" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">{{
            t('supplier.list.button.search')
          }}</el-button>
          <el-button @click="handleReset">{{ t('supplier.list.button.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table
        v-loading="loading"
        :data="suppliers"
        stripe
        :aria-label="t('supplier.list.tableAriaLabel')"
      >
        <el-table-column
          prop="supplier_code"
          :label="t('supplier.list.column.supplierCode')"
          width="120"
          fixed
        />
        <el-table-column
          prop="supplier_name"
          :label="t('supplier.list.column.supplierName')"
          min-width="180"
          fixed
        />
        <el-table-column
          prop="supplier_short_name"
          :label="t('supplier.list.column.shortName')"
          width="100"
        />
        <el-table-column
          prop="contact_phone"
          :label="t('supplier.list.column.contactPhone')"
          width="130"
        />
        <el-table-column
          prop="email"
          :label="t('supplier.list.column.email')"
          width="180"
          show-overflow-tooltip
        />
        <el-table-column prop="grade" :label="t('supplier.list.column.grade')" width="80">
          <template #default="{ row }">
            <el-tag :type="getGradeTag(row.grade)" size="small">
              {{ row.grade || '-' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="supplier_type" :label="t('supplier.list.column.type')" width="100" />
        <el-table-column prop="status" :label="t('supplier.list.column.status')" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{
                row.status === 'active'
                  ? t('supplier.list.option.statusActive')
                  : t('supplier.list.option.statusInactive')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('supplier.list.column.operation')" width="180" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="$emit('edit', row)">{{
              t('supplier.list.button.edit')
            }}</el-button>
            <el-button type="danger" link size="small" @click="$emit('delete', row)">{{
              t('supplier.list.button.delete')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="localQuery.page"
          v-model:page-size="localQuery.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('supplier.list.paginationAriaLabel')"
          @size-change="handleQuery"
          @current-change="handleQuery"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
// v11 批次 176 P2-1 修复：引入 Supplier 和 SupplierQueryParams 替代 any
import type { Supplier, SupplierQueryParams } from '@/api/supplier';

const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  suppliers: Supplier[];
  total: number;
  loading: boolean;
  queryParams: SupplierQueryParams;
  dialogMode: 'list' | 'view' | 'add' | 'edit';
}>();

const emit = defineEmits<{
  search: [];
  reset: [];
  'update:queryParams': [value: SupplierQueryParams];
  add: [];
  view: [row: Supplier];
  edit: [row: Supplier];
  delete: [row: Supplier];
}>();

const localQuery = reactive({ ...props.queryParams });

/** 等级 → Element Plus Tag 类型映射 */
const getGradeTag = (grade: string): 'success' | 'warning' | 'danger' | 'info' => {
  if (grade === 'A') return 'success';
  if (grade === 'D') return 'danger';
  return 'warning';
};

watch(
  () => props.queryParams,
  newQ => Object.assign(localQuery, newQ),
  { deep: true }
);

const handleQuery = () => {
  emit('update:queryParams', { ...localQuery, page: 1 });
  emit('search');
};

const handleReset = () => {
  localQuery.keyword = '';
  localQuery.grade = '';
  localQuery.status = '';
  localQuery.page = 1;
  localQuery.page_size = 20;
  handleQuery();
};
</script>
