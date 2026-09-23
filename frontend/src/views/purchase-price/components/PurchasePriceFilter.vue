<!--
  PurchasePriceFilter.vue - 采购价格过滤栏
  拆分自 purchase-price/index.vue（P14 批 2 I-3 第 3 批）
  批次 285：接入 useTableApi 模式（localQuery + handleSearch/handleReset）
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form
      :inline="true"
      :model="localQuery"
      class="filter-form"
      :aria-label="t('purchasePrice.filter.ariaLabel')"
    >
      <!-- 后端 PurchasePriceQuery 不读 keyword：原关键词输入框为假筛选，已移除 -->
      <el-form-item :label="t('purchasePrice.filter.label.supplier')">
        <el-select
          v-model="localQuery.supplier_id"
          :placeholder="t('purchasePrice.filter.placeholder.supplier')"
          clearable
          @change="handleSearch"
        >
          <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('purchasePrice.filter.label.product')">
        <el-select
          v-model="localQuery.product_id"
          :placeholder="t('purchasePrice.filter.placeholder.product')"
          clearable
          filterable
          @change="handleSearch"
        >
          <el-option v-for="p in products" :key="p.id" :label="p.product_name" :value="p.id" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('purchasePrice.filter.label.status')">
        <el-select
          v-model="localQuery.status"
          :placeholder="t('purchasePrice.filter.placeholder.status')"
          clearable
          @change="handleSearch"
        >
          <el-option :label="t('purchasePrice.statusLabels.pending')" value="pending" />
          <el-option :label="t('purchasePrice.statusLabels.approved')" value="approved" />
          <el-option :label="t('purchasePrice.statusLabels.inactive')" value="inactive" />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="handleSearch">
          <el-icon><Search /></el-icon>
          {{ t('purchasePrice.filter.button.search') }}
        </el-button>
        <el-button @click="handleReset">
          <el-icon><Refresh /></el-icon>
          {{ t('purchasePrice.filter.button.reset') }}
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
import type { Product } from '@/api/product';

const { t } = useI18n({ useScope: 'global' });

/**
 * 采购价格过滤栏组件（批次 285：localQuery + handleSearch/handleReset 模式）
 */
const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: Record<string, unknown>;
  // 供应商列表
  suppliers: Supplier[];
  // 产品列表
  products: Product[];
}>();

const emit = defineEmits<{
  // 触发加载
  fetch: [];
  // 整体回写查询参数
  'update:queryParams': [value: Record<string, unknown>];
}>();

// 本地查询条件（筛选字段，不含分页参数；与后端 PurchasePriceQuery 真实字段对齐）
const localQuery = reactive<{
  supplier_id: number | undefined;
  product_id: number | undefined;
  status: string;
}>({
  supplier_id: props.queryParams.supplier_id as number | undefined,
  product_id: props.queryParams.product_id as number | undefined,
  status: (props.queryParams.status as string) ?? '',
});

/** 搜索：先同步筛选条件到父组件，再触发加载 */
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery });
  emit('fetch');
};

/** 重置：清空筛选条件 + 同步 + 触发加载 */
const handleReset = () => {
  localQuery.supplier_id = undefined;
  localQuery.product_id = undefined;
  localQuery.status = '';
  emit('update:queryParams', { ...localQuery });
  emit('fetch');
};
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
</style>
