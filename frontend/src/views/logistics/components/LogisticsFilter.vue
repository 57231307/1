<!--
  LogisticsFilter.vue - 物流管理过滤栏
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  批次 287：接入 useTableApi 模式（localQuery + handleSearch/handleReset）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card class="filter-card">
    <el-form :inline="true" :model="localQuery" :aria-label="t('logistics.filter.aria.form')">
      <el-form-item :label="t('logistics.filter.label.waybillNo')">
        <el-input
          v-model="localQuery.keyword"
          :placeholder="t('logistics.filter.placeholder.waybillNo')"
          clearable
          @keyup.enter="handleSearch"
        />
      </el-form-item>
      <el-form-item :label="t('logistics.filter.label.logisticsCompany')">
        <el-select
          v-model="localQuery.logistics_company"
          :placeholder="t('logistics.filter.placeholder.logisticsCompany')"
          clearable
          @change="handleSearch"
        >
          <el-option
            :label="t('logistics.common.company.sf')"
            :value="t('logistics.common.company.sf')"
          />
          <el-option
            :label="t('logistics.common.company.zto')"
            :value="t('logistics.common.company.zto')"
          />
          <el-option
            :label="t('logistics.common.company.yto')"
            :value="t('logistics.common.company.yto')"
          />
          <el-option
            :label="t('logistics.common.company.yunda')"
            :value="t('logistics.common.company.yunda')"
          />
          <el-option
            :label="t('logistics.common.company.jd')"
            :value="t('logistics.common.company.jd')"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('logistics.filter.label.status')">
        <el-select
          v-model="localQuery.status"
          :placeholder="t('logistics.filter.placeholder.status')"
          clearable
          @change="handleSearch"
        >
          <el-option :label="t('logistics.common.status.pending')" value="pending" />
          <el-option :label="t('logistics.common.status.shipped')" value="shipped" />
          <el-option :label="t('logistics.common.status.inTransit')" value="in_transit" />
          <el-option :label="t('logistics.common.status.delivered')" value="delivered" />
          <el-option :label="t('logistics.common.status.cancelled')" value="cancelled" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('logistics.filter.label.dateRange')">
        <el-date-picker
          v-model="localDateRange"
          type="daterange"
          :range-separator="t('logistics.filter.separator.to')"
          :start-placeholder="t('logistics.filter.placeholder.dateStart')"
          :end-placeholder="t('logistics.filter.placeholder.dateEnd')"
          @update:model-value="onDateChange"
        />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="handleSearch">{{
          t('logistics.filter.button.search')
        }}</el-button>
        <el-button @click="handleReset">{{ t('logistics.filter.button.reset') }}</el-button>
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
  logistics_company: string;
  status: string;
}>({
  keyword: (props.queryParams.keyword as string) ?? '',
  logistics_company: (props.queryParams.logistics_company as string) ?? '',
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
  localQuery.logistics_company = '';
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
