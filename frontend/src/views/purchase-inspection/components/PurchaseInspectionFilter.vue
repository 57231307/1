<!--
  PurchaseInspectionFilter.vue - 采购验货过滤栏
  拆分自 purchase-inspection/index.vue（P14 批 2 I-3 第 5 批）
  批次 286：接入 useTableApi 模式（localQuery + handleSearch/handleReset）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card class="filter-card">
    <el-form
      :inline="true"
      :model="localQuery"
      :aria-label="t('purchaseInspection.filter.ariaLabel')"
    >
      <el-form-item :label="t('purchaseInspection.filter.label.inspectionNo')">
        <el-input
          v-model="localQuery.keyword"
          :placeholder="t('purchaseInspection.filter.placeholder.inspectionNo')"
          clearable
          @keyup.enter="handleSearch"
        />
      </el-form-item>
      <el-form-item :label="t('purchaseInspection.filter.label.supplier')">
        <el-select
          v-model="localQuery.supplier_id"
          :placeholder="t('purchaseInspection.filter.placeholder.supplier')"
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
      <el-form-item :label="t('purchaseInspection.filter.label.status')">
        <el-select
          v-model="localQuery.status"
          :placeholder="t('purchaseInspection.filter.placeholder.status')"
          clearable
          @change="handleSearch"
        >
          <el-option :label="t('purchaseInspection.filter.status.draft')" value="draft" />
          <el-option :label="t('purchaseInspection.filter.status.pending')" value="pending" />
          <el-option :label="t('purchaseInspection.filter.status.completed')" value="completed" />
          <el-option :label="t('purchaseInspection.filter.status.rejected')" value="rejected" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('purchaseInspection.filter.label.result')">
        <el-select
          v-model="localQuery.result"
          :placeholder="t('purchaseInspection.filter.placeholder.result')"
          clearable
          @change="handleSearch"
        >
          <el-option :label="t('purchaseInspection.filter.result.pass')" value="pass" />
          <el-option :label="t('purchaseInspection.filter.result.fail')" value="fail" />
          <el-option :label="t('purchaseInspection.filter.result.partial')" value="partial" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('purchaseInspection.filter.label.inspectionDate')">
        <el-date-picker
          v-model="localDateRange"
          type="daterange"
          :range-separator="t('purchaseInspection.filter.rangeSeparator')"
          :start-placeholder="t('purchaseInspection.filter.placeholder.startDate')"
          :end-placeholder="t('purchaseInspection.filter.placeholder.endDate')"
          @update:model-value="onDateChange"
        />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="handleSearch">{{
          t('purchaseInspection.filter.button.search')
        }}</el-button>
        <el-button @click="handleReset">{{
          t('purchaseInspection.filter.button.reset')
        }}</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: Record<string, unknown>;
  // 日期范围（由父组件管理，子组件通过 emit('date-change') 回写）
  dateRange: [Date, Date] | null;
  // 供应商列表
  suppliers: { id: number; name: string }[];
}>();

const emit = defineEmits<{
  // 触发加载
  fetch: [];
  // 整体回写查询参数
  'update:queryParams': [value: Record<string, unknown>];
  // 日期范围变化
  'date-change': [value: [Date, Date] | null];
}>();

// 本地查询条件（筛选字段，不含分页参数）
const localQuery = reactive<{
  keyword: string;
  supplier_id: number | undefined;
  status: string;
  result: string;
}>({
  keyword: (props.queryParams.keyword as string) ?? '',
  supplier_id: props.queryParams.supplier_id as number | undefined,
  status: (props.queryParams.status as string) ?? '',
  result: (props.queryParams.result as string) ?? '',
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
  localQuery.supplier_id = undefined;
  localQuery.status = '';
  localQuery.result = '';
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
