<!--
  PurchaseReturnFilter.vue - 采购退货过滤栏
  任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
  批次 286：接入 useTableApi 模式（localQuery + handleSearch/handleReset）
-->
<template>
  <el-card class="filter-card">
    <el-form :inline="true" :model="localQuery" :aria-label="t('purchaseReturn.filter.aria.form')">
      <el-form-item :label="t('purchaseReturn.filter.label.returnNo')">
        <el-input
          v-model="localQuery.keyword"
          :placeholder="t('purchaseReturn.filter.placeholder.returnNo')"
          clearable
          @keyup.enter="handleSearch"
        />
      </el-form-item>
      <el-form-item :label="t('purchaseReturn.filter.label.supplier')">
        <el-select
          v-model="localQuery.supplierId"
          :placeholder="t('purchaseReturn.filter.placeholder.supplier')"
          clearable
          filterable
          @change="handleSearch"
        >
          <el-option
            v-for="supplier in suppliers"
            :key="supplier.id"
            :label="supplier.name"
            :value="supplier.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('purchaseReturn.filter.label.status')">
        <el-select
          v-model="localQuery.status"
          :placeholder="t('purchaseReturn.filter.placeholder.status')"
          clearable
          @change="handleSearch"
        >
          <el-option :label="t('purchaseReturn.filter.status.draft')" value="draft" />
          <el-option :label="t('purchaseReturn.filter.status.pending')" value="pending" />
          <el-option :label="t('purchaseReturn.filter.status.approved')" value="approved" />
          <el-option :label="t('purchaseReturn.filter.status.rejected')" value="rejected" />
          <el-option :label="t('purchaseReturn.filter.status.completed')" value="completed" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('purchaseReturn.filter.label.returnDate')">
        <el-date-picker
          v-model="localDateRange"
          type="daterange"
          :range-separator="t('purchaseReturn.filter.rangeSeparator')"
          :start-placeholder="t('purchaseReturn.filter.placeholder.startDate')"
          :end-placeholder="t('purchaseReturn.filter.placeholder.endDate')"
          @update:model-value="onDateChange"
        />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="handleSearch">{{
          t('purchaseReturn.filter.button.search')
        }}</el-button>
        <el-button @click="handleReset">{{ t('purchaseReturn.filter.button.reset') }}</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n({ useScope: 'global' });

// 供应商数据结构
interface Supplier {
  id: number;
  name: string;
}

const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: Record<string, unknown>;
  // 供应商列表
  suppliers: Supplier[];
  // 日期范围（由父组件管理，子组件通过 emit('date-change') 回写）
  dateRange: [Date, Date] | null;
}>();

const emit = defineEmits<{
  // 触发加载
  fetch: [];
  // 整体回写查询参数
  'update:queryParams': [value: Record<string, unknown>];
  // 日期变化事件
  'date-change': [value: [Date, Date] | null];
}>();

// 本地查询条件（筛选字段，不含分页参数）
const localQuery = reactive<{
  keyword: string;
  supplierId: number | undefined;
  status: string;
}>({
  keyword: (props.queryParams.keyword as string) ?? '',
  supplierId: props.queryParams.supplierId as number | undefined,
  status: (props.queryParams.status as string) ?? '',
});

// 本地日期范围镜像（避免直接修改 prop）
const localDateRange = ref<[Date, Date] | null>(props.dateRange);

/** 日期范围变化：同步本地镜像 + emit 通知父组件 */
const onDateChange = (v: [Date, Date] | null) => {
  localDateRange.value = v;
  emit('date-change', v);
};

/** 搜索：先同步筛选条件到父组件，再触发加载 */
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery });
  emit('fetch');
};

/** 重置：清空筛选条件 + 同步 + 触发加载 */
const handleReset = () => {
  localQuery.keyword = '';
  localQuery.supplierId = undefined;
  localQuery.status = '';
  localDateRange.value = null;
  emit('date-change', null);
  emit('update:queryParams', { ...localQuery });
  emit('fetch');
};
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
</style>
