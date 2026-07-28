<!--
  VoucherForm.vue - 凭证新建表单对话框
  拆分自 VoucherTab.vue（P14 批 1 B3 I-2）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('finance.voucherForm.dialogTitle')"
    width="800px"
    :aria-label="t('finance.voucherForm.ariaLabel')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      :ref="(el: unknown) => (formRefValue = el as FormInstance)"
      :model="localVoucherForm"
      :rules="voucherRules"
      label-width="80px"
      :aria-label="t('finance.voucherForm.formAriaLabel')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('finance.voucherForm.labelVoucherDate')" prop="voucher_date">
            <el-date-picker
              v-model="localVoucherForm.voucher_date"
              type="date"
              :placeholder="t('finance.voucherForm.placeholderVoucherDate')"
              value-format="YYYY-MM-DD"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('finance.voucherForm.labelVoucherType')" prop="voucher_type">
            <el-select
              v-model="localVoucherForm.voucher_type"
              :placeholder="t('finance.voucherForm.placeholderVoucherType')"
              style="width: 100%"
            >
              <el-option :label="t('finance.voucherForm.optionJZ')" value="JZ" />
              <el-option :label="t('finance.voucherForm.optionSK')" value="SK" />
              <el-option :label="t('finance.voucherForm.optionFK')" value="FK" />
              <el-option :label="t('finance.voucherForm.optionZZ')" value="ZZ" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider>{{ t('finance.voucherForm.dividerEntries') }}</el-divider>
      <el-table
        :data="localVoucherForm.entries"
        stripe
        :aria-label="t('finance.voucherForm.entriesAriaLabel')"
      >
        <el-table-column :label="t('finance.voucherForm.columnSummary')" min-width="150">
          <template #default="{ row }">
            <el-input
              v-model="row.summary"
              :placeholder="t('finance.voucherForm.placeholderSummary')"
            />
          </template>
        </el-table-column>
        <el-table-column :label="t('finance.voucherForm.columnSubject')" min-width="200">
          <template #default="{ row }">
            <el-tree-select
              v-model="row.subject_id"
              :data="leafSubjects"
              :props="{ label: 'name', value: 'id' }"
              :placeholder="t('finance.voucherForm.placeholderSubject')"
              check-strictly
            />
          </template>
        </el-table-column>
        <el-table-column :label="t('finance.voucherForm.columnDebitAmount')" width="130">
          <template #default="{ row }">
            <el-input-number
              v-model="row.debit"
              :min="0"
              :precision="2"
              :controls="false"
              style="width: 100%"
            />
          </template>
        </el-table-column>
        <el-table-column :label="t('finance.voucherForm.columnCreditAmount')" width="130">
          <template #default="{ row }">
            <el-input-number
              v-model="row.credit"
              :min="0"
              :precision="2"
              :controls="false"
              style="width: 100%"
            />
          </template>
        </el-table-column>
        <el-table-column :label="t('finance.voucherForm.columnAction')" width="80">
          <template #default="{ $index }">
            <el-button type="danger" link size="small" @click="emit('remove-entry', $index)">{{
              t('finance.voucherForm.buttonDelete')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>
      <div class="entry-footer">
        <el-button type="primary" link @click="emit('add-entry')">{{
          t('finance.voucherForm.buttonAddEntry')
        }}</el-button>
        <div class="entry-summary">
          <span>{{ t('finance.voucherForm.labelDebitTotal') }}: {{ formatMoney(totalDebit) }}</span>
          <span
            >{{ t('finance.voucherForm.labelCreditTotal') }}: {{ formatMoney(totalCredit) }}</span
          >
          <span :class="{ 'text-red': !isBalanced }">{{
            isBalanced
              ? t('finance.voucherForm.textBalanced')
              : t('finance.voucherForm.textNotBalanced')
          }}</span>
        </div>
      </div>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('finance.voucherForm.buttonCancel')
      }}</el-button>
      <el-button type="primary" :loading="voucherSubmitLoading" @click="emit('submit-form')">{{
        t('finance.voucherForm.buttonConfirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { deepClone } from '@/utils';
import { ref, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FormInstance, FormRules } from 'element-plus';
import type { AccountSubject } from '@/api/finance';

const { t } = useI18n({ useScope: 'global' });

interface VoucherEntry {
  subject_id: number | undefined;
  debit: number;
  credit: number;
  summary: string;
}

interface VoucherForm {
  voucher_date: string;
  voucher_type: string;
  entries: VoucherEntry[];
}

/**
 * 凭证新建表单对话框组件
 * 接收父组件传入的 form / ref / 业务方法
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean;
  // 表单实例 ref（父组件持有的 FormInstance 引用包装对象）
  voucherFormRef: { value: FormInstance | undefined };
  // 表单数据（由父组件管理，子组件通过 emit 回写）
  voucherForm: VoucherForm;
  // 提交中状态
  voucherSubmitLoading: boolean;
  // 校验规则
  voucherRules: FormRules;
  // 叶子科目列表
  leafSubjects: AccountSubject[];
  // 借贷合计
  totalDebit: number;
  totalCredit: number;
  // 是否平衡
  isBalanced: boolean;
  // 金额格式化
  formatMoney: (amount: number) => string;
}>();

const emit = defineEmits<{
  // 关闭对话框
  (e: 'update:visible', v: boolean): void;
  // 添加分录
  (e: 'add-entry'): void;
  // 删除分录
  (e: 'remove-entry', index: number): void;
  // 提交表单
  (e: 'submit-form'): void;
  // 整体回写表单（父组件监听此事件并 Object.assign 到自己的 voucherForm）
  (e: 'update:voucherForm', voucherForm: VoucherForm): void;
  // el-form 实例就绪（替代直接修改 prop voucherFormRef.value）
  (e: 'form-ref-ready', val: FormInstance): void;
}>();

// 将 el-form 的 ref 实例通过 emit 通知父组件（避免直接修改 prop 触发 vue/no-mutating-props）
const formRefValue = ref<FormInstance | undefined>(undefined);
watch(
  formRefValue,
  val => {
    if (val) emit('form-ref-ready', val);
  },
  { immediate: true, flush: 'post' }
);

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// 注意：表单内有 entries 数组，需要深拷贝以保证本地修改与父组件解耦
const localVoucherForm = ref<VoucherForm>(deepClone(props.voucherForm));

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local（如父组件打开对话框时填充数据）
watch(
  () => props.voucherForm,
  newForm => {
    if (syncing) return;
    syncing = true;
    localVoucherForm.value = deepClone(newForm);
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 本地变化时通知父组件（用户输入）
watch(
  localVoucherForm,
  newForm => {
    if (syncing) return;
    syncing = true;
    emit('update:voucherForm', deepClone(newForm));
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);
</script>

<style scoped>
.entry-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 12px;
}
.entry-summary {
  display: flex;
  gap: 16px;
  font-size: 13px;
  color: #606266;
}
.text-red {
  color: #f56c6c;
  font-weight: 600;
}
</style>
