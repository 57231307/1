<!--
  SalesPriceFilter.vue - 销售价格过滤栏
  拆分自 sales-price/index.vue（P14 批 2 I-3 第 3 批）
  批次 284：接入 useTableApi 模式（localQuery + handleSearch/handleReset）
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form
      :inline="true"
      :model="localQuery"
      class="filter-form"
      :aria-label="t('salesPrice.filter.ariaLabel')"
    >
      <el-form-item :label="t('salesPrice.filter.labelKeyword')">
        <el-input
          v-model="localQuery.keyword"
          :placeholder="t('salesPrice.filter.placeholderKeyword')"
          clearable
          @clear="handleSearch"
        />
      </el-form-item>
      <el-form-item :label="t('salesPrice.filter.labelCustomer')">
        <el-select
          v-model="localQuery.customer_id"
          :placeholder="t('salesPrice.filter.placeholderCustomer')"
          clearable
          filterable
          @change="handleSearch"
        >
          <el-option v-for="c in customers" :key="c.id" :label="c.customer_name" :value="c.id" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('salesPrice.filter.labelProduct')">
        <el-select
          v-model="localQuery.product_id"
          :placeholder="t('salesPrice.filter.placeholderProduct')"
          clearable
          filterable
          @change="handleSearch"
        >
          <el-option v-for="p in products" :key="p.id" :label="p.product_name" :value="p.id" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('salesPrice.filter.labelStatus')">
        <el-select
          v-model="localQuery.status"
          :placeholder="t('salesPrice.filter.placeholderStatus')"
          clearable
          @change="handleSearch"
        >
          <el-option :label="t('salesPrice.filter.optionPending')" value="pending" />
          <el-option :label="t('salesPrice.filter.optionActive')" value="active" />
          <el-option :label="t('salesPrice.filter.optionExpired')" value="expired" />
          <el-option :label="t('salesPrice.filter.optionInactive')" value="inactive" />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="handleSearch">
          <el-icon><Search /></el-icon>
          {{ t('salesPrice.filter.buttonSearch') }}
        </el-button>
        <el-button @click="handleReset">
          <el-icon><Refresh /></el-icon>
          {{ t('salesPrice.filter.buttonReset') }}
        </el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import { Search, Refresh } from '@element-plus/icons-vue';
import type { Customer } from '@/api/customer';
import type { Product } from '@/api/product';

const { t } = useI18n({ useScope: 'global' });

/**
 * 销售价格过滤栏组件（批次 284：localQuery + handleSearch/handleReset 模式）
 */
const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: Record<string, unknown>;
  // 客户列表
  customers: Customer[];
  // 产品列表
  products: Product[];
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
  customer_id: number | undefined;
  product_id: number | undefined;
  status: string;
}>({
  keyword: (props.queryParams.keyword as string) ?? '',
  customer_id: props.queryParams.customer_id as number | undefined,
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
  localQuery.keyword = '';
  localQuery.customer_id = undefined;
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
