<!--
  VoucherListFilter.vue - 凭证列表过滤与操作栏
  拆分自 voucher/tabs/VoucherListTab.vue（P14 批 2 I-3 第 1 批）
  批次 287：改造为 localQuery + handleSearch 模式，接入 useTableApi queryParams
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="filter-container">
    <ElRow :gutter="20">
      <ElCol :span="6">
        <ElInput
          v-model="localQuery.voucher_no"
          :placeholder="t('voucher.voucherListFilter.placeholderVoucherNo')"
          class="filter-item"
          @keyup.enter="handleSearch"
        />
      </ElCol>
      <ElCol :span="6">
        <!-- P0 修复：value-format 输出 YYYY-MM-DD（后端 VoucherQuery.start_date/end_date 按 NaiveDate parse，Date 对象序列化后 parse 失败即筛选静默失效） -->
        <ElDatePicker
          v-model="localQuery.start_date"
          type="date"
          value-format="YYYY-MM-DD"
          :placeholder="t('voucher.voucherListFilter.placeholderStartDate')"
          class="filter-item"
        />
      </ElCol>
      <ElCol :span="6">
        <ElDatePicker
          v-model="localQuery.end_date"
          type="date"
          value-format="YYYY-MM-DD"
          :placeholder="t('voucher.voucherListFilter.placeholderEndDate')"
          class="filter-item"
        />
      </ElCol>
      <ElCol :span="6">
        <ElSelect
          v-model="localQuery.status"
          :placeholder="t('voucher.voucherListFilter.placeholderStatus')"
          class="filter-item"
        >
          <ElOption :label="t('voucher.voucherListFilter.optionAll')" value="" />
          <!-- 状态词表三端同源：后端 status::finance::voucher draft/submitted/reviewed/posted -->
          <ElOption :label="t('voucher.voucherListFilter.optionDraft')" value="draft" />
          <ElOption :label="t('voucher.voucherListFilter.optionSubmitted')" value="submitted" />
          <ElOption :label="t('voucher.voucherListFilter.optionReviewed')" value="reviewed" />
          <ElOption :label="t('voucher.voucherListFilter.optionPosted')" value="posted" />
        </ElSelect>
      </ElCol>
    </ElRow>
    <div class="filter-actions">
      <ElButton type="primary" @click="handleSearch">{{
        t('voucher.voucherListFilter.buttonSearch')
      }}</ElButton>
      <ElButton @click="handleReset">{{ t('voucher.voucherListFilter.buttonReset') }}</ElButton>
      <ElButton type="success" @click="emit('add')">
        <Plus /> {{ t('voucher.voucherListFilter.buttonAdd') }}</ElButton
      >
      <ElButton v-permission="'voucher.print'" @click="emit('print')">
        <Printer /> {{ t('voucher.voucherListFilter.buttonPrint') }}</ElButton
      >
      <ElButton v-permission="'voucher.export'" @click="emit('export')">
        <Download /> {{ t('voucher.voucherListFilter.buttonExport') }}</ElButton
      >
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import { Plus, Printer, Download } from '@element-plus/icons-vue';

const { t } = useI18n({ useScope: 'global' });

/**
 * 凭证列表过滤与操作栏组件
 * 接收父组件传入的 queryParams，通过 emit('update:queryParams') 同步筛选条件
 * 查询/重置时先同步 queryParams 再触发 fetch
 */
const props = defineProps<{
  // 查询条件（由父组件 useTableApi 管理，类型放宽为 Record 兼容 useTableApi）
  queryParams: Record<string, unknown>;
}>();

const emit = defineEmits<{
  // 触发查询（父组件监听后调用 handleSearch 重置页码并加载）
  (e: 'fetch'): void;
  // 同步查询条件到父组件
  (e: 'update:queryParams', params: Record<string, unknown>): void;
  // 新增凭证
  (e: 'add'): void;
  // 打印
  (e: 'print'): void;
  // 导出
  (e: 'export'): void;
}>();

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// P0 修复：查询键对齐后端 VoucherQuery（start_date/end_date/voucher_type，
// 原 voucher_date_start/voucher_date_end/type 后端不识别 ⇒ 日期/类型筛选恒不生效）
const localQuery = reactive({
  voucher_no: props.queryParams.voucher_no as string,
  start_date: props.queryParams.start_date as string,
  end_date: props.queryParams.end_date as string,
  voucher_type: props.queryParams.voucher_type as string,
  status: props.queryParams.status as string,
});

/** 查询：先同步筛选条件到父组件，再触发 fetch */
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery });
  emit('fetch');
};

/** 重置：清空本地筛选条件，同步后触发 fetch */
const handleReset = () => {
  localQuery.voucher_no = '';
  localQuery.start_date = '';
  localQuery.end_date = '';
  localQuery.voucher_type = '';
  localQuery.status = '';
  emit('update:queryParams', { ...localQuery });
  emit('fetch');
};
</script>
