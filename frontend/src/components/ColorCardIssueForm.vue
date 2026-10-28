<template>
  <!--
    色卡发放表单组件（V15 P0-F12）
    拆分自 views/color-cards/issues.vue 的「发放色卡」Tab
    供 issues.vue / 其他页面复用，emit('success', record) 通知父组件刷新
  -->
  <el-form
    ref="formRef"
    :model="form"
    :rules="rules"
    label-width="100px"
    :aria-label="t('components.colorCardIssueForm.formAriaLabel')"
    style="max-width: 600px"
  >
    <el-form-item :label="t('components.colorCardIssueForm.selectCard')" prop="color_card_id">
      <el-select
        v-model="form.color_card_id"
        filterable
        :placeholder="t('components.colorCardIssueForm.searchCardPlaceholder')"
        style="width: 100%"
      >
        <el-option
          v-for="card in availableCards"
          :key="card.id"
          :label="`${card.card_no} - ${card.card_name} ${t('components.colorCardIssueForm.stockLabel', { available: card.stock_quantity - card.issued_quantity, total: card.stock_quantity })}`"
          :value="card.id"
          :disabled="card.stock_quantity - card.issued_quantity <= 0"
        />
      </el-select>
    </el-form-item>
    <el-form-item :label="t('components.colorCardIssueForm.customerId')" prop="customer_id">
      <!-- 视图态数值字段以字符串承载（el-input 手写录入），提交边界显式转回契约数字 -->
      <el-input
        v-model="form.customer_id"
        inputmode="numeric"
        style="width: 100%"
        :placeholder="t('components.colorCardIssueForm.customerIdPlaceholder')"
      />
    </el-form-item>
    <el-form-item :label="t('components.colorCardIssueForm.issueQty')" prop="issue_qty">
      <!-- 手写录入 发放数量 正整数（原步进控件 min=1 step=1 约束落到 rules） -->
      <el-input v-model="form.issue_qty" inputmode="numeric" style="width: 100%" />
    </el-form-item>
    <el-form-item :label="t('components.colorCardIssueForm.dyeLotNo')">
      <el-input
        v-model="form.dye_lot_no"
        :placeholder="t('components.colorCardIssueForm.dyeLotNoPlaceholder')"
      />
    </el-form-item>
    <el-form-item :label="t('components.colorCardIssueForm.expectedReturn')">
      <el-date-picker
        v-model="form.expected_return_date"
        type="date"
        :placeholder="t('components.colorCardIssueForm.expectedReturnPlaceholder')"
        style="width: 100%"
        value-format="YYYY-MM-DD"
      />
    </el-form-item>
    <el-form-item :label="t('components.colorCardIssueForm.purpose')">
      <el-input
        v-model="form.purpose"
        :placeholder="t('components.colorCardIssueForm.purposePlaceholder')"
      />
    </el-form-item>
    <el-form-item :label="t('components.colorCardIssueForm.remark')">
      <el-input v-model="form.remark" type="textarea" :rows="2" />
    </el-form-item>
    <el-form-item>
      <el-button type="primary" :loading="loading" @click="handleSubmit">{{
        t('components.colorCardIssueForm.submit')
      }}</el-button>
    </el-form-item>
  </el-form>
</template>

<script setup lang="ts">
// 色卡发放表单组件（V15 P0-F12）
// 创建时间：2026-07-18（Batch 477 P0-F12）

import { ref, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FormInstance } from 'element-plus';
import { useColorCardIssue } from '@/composables/useColorCardIssue';
import { numberInputRules } from '@/utils/number-input';
import type { ColorCardListItem } from '@/api/color-card';
import type { CreateIssueDto, IssueRecordInfo } from '@/types/colorCardIssue';

const { t } = useI18n({ useScope: 'global' });

defineProps<{
  availableCards: ColorCardListItem[];
}>();

const emit = defineEmits<{
  (e: 'success', record: IssueRecordInfo): void;
}>();

const formRef = ref<FormInstance>();
const loading = ref(false);
const { issue } = useColorCardIssue();

const form = reactive({
  color_card_id: undefined as number | undefined,
  // 视图态数值字段以字符串承载（el-input 手写录入），提交边界显式转回契约数字
  customer_id: '1',
  issue_qty: '1',
  expected_return_date: '',
  purpose: '',
  remark: '',
  dye_lot_no: '',
});

const rules = {
  color_card_id: [
    {
      required: true,
      message: t('components.colorCardIssueForm.validation.selectCard'),
      trigger: 'change',
    },
  ],
  customer_id: [
    {
      required: true,
      message: t('components.colorCardIssueForm.validation.customerId'),
      trigger: 'blur',
    },
    // 客户 ID 正整数（原步进控件 min=1 硬约束逐条落到 rules）
    ...numberInputRules({
      label: t('components.colorCardIssueForm.customerId'),
      min: 1,
      integer: true,
    }),
  ],
  issue_qty: [
    {
      required: true,
      message: t('components.colorCardIssueForm.validation.issueQty'),
      trigger: 'blur',
    },
    // 发放数量 正整数（原步进控件 min=1 与默认精度 0 的硬约束逐条落到 rules）
    ...numberInputRules({
      label: t('components.colorCardIssueForm.issueQty'),
      min: 1,
      integer: true,
    }),
  ],
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return;
    if (!form.color_card_id) return;
    loading.value = true;
    try {
      const dto: CreateIssueDto = {
        color_card_id: form.color_card_id,
        customer_id: Number(form.customer_id),
        // rules 已保证正整数；保留原「空回落 1」提交语义
        issue_qty: Number(form.issue_qty) || 1,
        expected_return_date: form.expected_return_date || undefined,
        purpose: form.purpose || undefined,
        remark: form.remark || undefined,
        dye_lot_no: form.dye_lot_no || undefined,
      };
      const record = await issue(dto);
      if (record) {
        emit('success', record);
        // 重置部分字段，保留客户 ID 便于连续发放
        form.color_card_id = undefined;
        form.purpose = '';
        form.remark = '';
        form.dye_lot_no = '';
      }
    } finally {
      loading.value = false;
    }
  });
};
</script>
