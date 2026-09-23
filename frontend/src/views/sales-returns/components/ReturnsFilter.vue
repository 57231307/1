<!--
  ReturnsFilter.vue - 销售退货筛选栏
  退货单号 / 状态 / 客户 三项筛选 + 查询/重置；分页由列表页维护
  筛选值经 update:queryParams 回写父级，子组件不直接改 prop
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="localQuery" :aria-label="t('salesReturns.filter.ariaForm')">
      <el-form-item :label="t('salesReturns.filter.labelReturnNo')">
        <el-input
          v-model="localQuery.return_no"
          :placeholder="t('salesReturns.filter.placeholderReturnNo')"
          clearable
          @keyup.enter="handleSearch"
        />
      </el-form-item>
      <el-form-item :label="t('salesReturns.filter.labelStatus')">
        <el-select
          v-model="localQuery.status"
          :placeholder="t('salesReturns.filter.placeholderStatus')"
          clearable
          @change="handleSearch"
        >
          <el-option
            v-for="st in SALES_RETURN_STATUSES"
            :key="st"
            :label="t(salesReturnStatusLabelKey(st) ?? '')"
            :value="st"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('salesReturns.filter.labelCustomer')">
        <el-select
          v-model="localQuery.customer_id"
          :placeholder="t('salesReturns.filter.placeholderCustomer')"
          clearable
          filterable
          @change="handleSearch"
        >
          <el-option
            v-for="customer in customers"
            :key="customer.id"
            :label="customer.customer_name"
            :value="customer.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="handleSearch">
          {{ t('salesReturns.filter.buttonSearch') }}
        </el-button>
        <el-button @click="handleReset">{{ t('salesReturns.filter.buttonReset') }}</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { CustomerOption } from '../composables/useSr';
import { SALES_RETURN_STATUSES, salesReturnStatusLabelKey } from '@/utils/sales-return-status';

const { t } = useI18n({ useScope: 'global' });

interface FilterValues {
  return_no: string;
  status: string;
  customer_id?: number;
}

const props = defineProps<{
  queryParams: FilterValues;
  customers: CustomerOption[];
}>();

const emit = defineEmits<{
  'update:queryParams': [value: FilterValues];
  search: [];
  reset: [];
}>();

// 本地镜像：避免直接修改 props；父级 queryParams 变化（如重置）时同步回来
const localQuery = reactive<FilterValues>({
  return_no: props.queryParams.return_no,
  status: props.queryParams.status,
  customer_id: props.queryParams.customer_id,
});

watch(
  () => props.queryParams,
  q => {
    localQuery.return_no = q.return_no;
    localQuery.status = q.status;
    localQuery.customer_id = q.customer_id;
  },
  { deep: true }
);

// 搜索：回写筛选值后通知父级触发加载（空串/未选由请求层剔除，此处不手工剥离）
const handleSearch = () => {
  emit('update:queryParams', {
    return_no: localQuery.return_no,
    status: localQuery.status,
    customer_id: localQuery.customer_id,
  });
  emit('search');
};

// 重置：清空本地筛选并回写、通知父级
const handleReset = () => {
  localQuery.return_no = '';
  localQuery.status = '';
  localQuery.customer_id = undefined;
  emit('update:queryParams', {
    return_no: '',
    status: '',
    customer_id: undefined,
  });
  emit('reset');
};
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
</style>
