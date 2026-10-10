<!--
  BpmDefinitionFilter.vue - BPM 流程定义过滤栏
  拆分自 bpm/definitions.vue（P14 批 2 I-3 第 5 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  批次 282：接入 useTableApi 模式（handleSearch 同步筛选条件 + emit fetch）
-->
<template>
  <el-card class="filter-card">
    <el-form :inline="true" :model="localQuery" :aria-label="t('bpm.definitions.filter.ariaLabel')">
      <!-- 流程名称(keyword)筛选控件已移除：后端 ProcessDefinitionQuery 无 keyword 字段，
           发送即被静默丢弃=假筛选（后端补齐名称模糊搜索后恢复，见串行清单） -->
      <el-form-item :label="t('bpm.definitions.filter.category')">
        <el-select
          v-model="localQuery.category"
          :placeholder="t('bpm.definitions.filter.categoryPlaceholder')"
          clearable
        >
          <el-option :label="t('bpm.definitions.category.finance')" value="finance" />
          <el-option :label="t('bpm.definitions.category.hr')" value="hr" />
          <el-option :label="t('bpm.definitions.category.purchase')" value="purchase" />
          <el-option :label="t('bpm.definitions.category.sales')" value="sales" />
          <el-option :label="t('bpm.definitions.category.production')" value="production" />
          <el-option :label="t('bpm.definitions.category.inventory')" value="inventory" />
          <el-option :label="t('bpm.definitions.category.other')" value="other" />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="handleSearch">{{
          t('bpm.definitions.filter.query')
        }}</el-button>
        <el-button @click="handleReset">{{ t('bpm.definitions.filter.reset') }}</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { reactive } from 'vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n({ useScope: 'global' });

// 批次 282：queryParams 类型放宽为 Record<string, unknown>（兼容 useTableApi）
const props = defineProps<{
  queryParams: Record<string, unknown>;
}>();

const emit = defineEmits<{
  fetch: [];
  'update:queryParams': [value: Record<string, unknown>];
}>();

// 本地查询条件（筛选字段，不含分页参数；keyword 已随契约核实移除）
const localQuery = reactive<{ category: string }>({
  category: (props.queryParams.category as string) ?? '',
});

/** 搜索：先同步筛选条件到父组件，再触发加载 */
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery });
  emit('fetch');
};

/** 重置：清空筛选条件 + 同步 + 触发加载 */
const handleReset = () => {
  localQuery.category = '';
  emit('update:queryParams', { ...localQuery });
  emit('fetch');
};
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
</style>
