<!--
  PurchaseContractFilter.vue - 采购合同过滤栏
  拆分自 purchase-contract/index.vue（P14 批 2 I-3 第 3 批）
  批次 284：接入 useTableApi 模式（localQuery + handleSearch/handleReset）
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form
      :inline="true"
      :model="localQuery"
      class="filter-form"
      :aria-label="t('purchaseContract.filter.ariaLabel')"
    >
      <el-form-item :label="t('purchaseContract.filter.keyword')">
        <el-input
          v-model="localQuery.keyword"
          :placeholder="t('purchaseContract.filter.keywordPlaceholder')"
          clearable
          @clear="handleSearch"
        />
      </el-form-item>
      <el-form-item :label="t('purchaseContract.filter.supplier')">
        <el-select
          v-model="localQuery.supplier_id"
          :placeholder="t('purchaseContract.filter.supplierPlaceholder')"
          clearable
          @change="handleSearch"
        >
          <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('purchaseContract.filter.status')">
        <el-select
          v-model="localQuery.status"
          :placeholder="t('purchaseContract.filter.statusPlaceholder')"
          clearable
          @change="handleSearch"
        >
          <el-option :label="t('purchaseContract.filter.statusDraft')" value="draft" />
          <el-option :label="t('purchaseContract.filter.statusPending')" value="pending" />
          <el-option :label="t('purchaseContract.filter.statusActive')" value="active" />
          <el-option :label="t('purchaseContract.filter.statusCompleted')" value="completed" />
          <el-option :label="t('purchaseContract.filter.statusCancelled')" value="cancelled" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('purchaseContract.filter.dateRange')">
        <el-date-picker
          v-model="localQuery.date_range"
          type="daterange"
          :range-separator="t('purchaseContract.filter.dateRangeSeparator')"
          :start-placeholder="t('purchaseContract.filter.dateStartPlaceholder')"
          :end-placeholder="t('purchaseContract.filter.dateEndPlaceholder')"
          @change="handleSearch"
        />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="handleSearch">
          <el-icon><Search /></el-icon>
          {{ t('purchaseContract.filter.query') }}
        </el-button>
        <el-button @click="handleReset">
          <el-icon><Refresh /></el-icon>
          {{ t('purchaseContract.filter.reset') }}
        </el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import { Search, Refresh } from '@element-plus/icons-vue';
import type { Supplier } from '@/api/supplier';

const { t } = useI18n({ useScope: 'global' });

/**
 * 采购合同过滤栏组件（批次 284：localQuery + handleSearch/handleReset 模式）
 */
const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: Record<string, unknown>;
  // 供应商列表
  suppliers: Supplier[];
}>();

const emit = defineEmits<{
  // 触发加载
  fetch: [];
  // 整体回写查询参数
  'update:queryParams': [value: Record<string, unknown>];
}>();

// 本地查询条件（筛选字段，不含分页参数）
const localQuery = reactive<{
  keyword: string;
  supplier_id: number | undefined;
  status: string;
  date_range: string[];
}>({
  keyword: (props.queryParams.keyword as string) ?? '',
  supplier_id: props.queryParams.supplier_id as number | undefined,
  status: (props.queryParams.status as string) ?? '',
  date_range: [...((props.queryParams.date_range as string[]) ?? [])],
});

/** 搜索：先同步筛选条件到父组件，再触发加载 */
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery, date_range: [...localQuery.date_range] });
  emit('fetch');
};

/** 重置：清空筛选条件 + 同步 + 触发加载 */
const handleReset = () => {
  localQuery.keyword = '';
  localQuery.supplier_id = undefined;
  localQuery.status = '';
  localQuery.date_range = [];
  emit('update:queryParams', { ...localQuery, date_range: [] });
  emit('fetch');
};
</script>

<style scoped>
.filter-card {
  margin-bottom: 16px;
}
.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
</style>
