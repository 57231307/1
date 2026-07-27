<!--
  SalesOrderFilter.vue - 销售订单列表过滤栏
  拆分自 sales/views/OrderListView.vue（P14 批 2 I-3 第 3 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="localFilterForm" :aria-label="t('sales.filter.formAriaLabel')">
      <el-form-item :label="t('sales.filter.orderNo')">
        <el-input
          v-model="localFilterForm.order_no"
          :placeholder="t('sales.filter.orderNoPlaceholder')"
          clearable
        />
      </el-form-item>
      <el-form-item :label="t('sales.filter.customer')">
        <el-input
          v-model="localFilterForm.customer_name"
          :placeholder="t('sales.filter.customerPlaceholder')"
          clearable
        />
      </el-form-item>
      <el-form-item :label="t('sales.filter.status')">
        <el-select
          v-model="localFilterForm.status"
          :placeholder="t('sales.filter.statusPlaceholder')"
          clearable
        >
          <el-option :label="t('sales.statusLabels.pending')" value="pending" />
          <el-option :label="t('sales.statusLabels.approved')" value="approved" />
          <el-option :label="t('sales.statusLabels.shipped')" value="shipped" />
          <el-option :label="t('sales.statusLabels.completed')" value="completed" />
          <el-option :label="t('sales.statusLabels.cancelled')" value="cancelled" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('sales.filter.date')">
        <el-date-picker
          v-model="localFilterForm.dateRange"
          type="daterange"
          :range-separator="t('sales.filter.dateRangeSeparator')"
          :start-placeholder="t('sales.filter.startDate')"
          :end-placeholder="t('sales.filter.endDate')"
        />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="emit('query')">{{ t('sales.filter.query') }}</el-button>
        <el-button @click="emit('reset')">{{ t('sales.filter.reset') }}</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n({ useScope: 'global' });

// 销售订单过滤表单类型
interface OlvFilterForm {
  order_no: string;
  customer_name: string;
  status: string;
  dateRange: Date[] | null;
}

/**
 * 销售订单列表过滤栏组件
 */
const props = defineProps<{
  // 过滤表单（由父组件管理，子组件通过 emit('update:filterForm') 回写）
  filterForm: OlvFilterForm;
}>();

const emit = defineEmits<{
  // 查询
  (e: 'query'): void;
  // 重置
  (e: 'reset'): void;
  // 整体回写过滤表单（父组件监听此事件并 Object.assign 到自己的 filterForm）
  (e: 'update:filterForm', filterForm: OlvFilterForm): void;
}>();

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localFilterForm = ref<OlvFilterForm>({
  ...props.filterForm,
  dateRange: props.filterForm.dateRange ? [...props.filterForm.dateRange] : null,
});

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local（如父组件重置）
watch(
  () => props.filterForm,
  newForm => {
    if (syncing) return;
    syncing = true;
    localFilterForm.value = {
      ...newForm,
      dateRange: newForm.dateRange ? [...newForm.dateRange] : null,
    };
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 本地变化时通知父组件（用户输入）
watch(
  localFilterForm,
  newForm => {
    if (syncing) return;
    syncing = true;
    emit('update:filterForm', {
      ...newForm,
      dateRange: newForm.dateRange ? [...newForm.dateRange] : null,
    });
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
</style>
