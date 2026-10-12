<!--
  VoucherListForm.vue - 凭证新建/编辑对话框
  拆分自 voucher/tabs/VoucherListTab.vue（P14 批 2 I-3 第 1 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <ElDialog
    :model-value="visible"
    :title="title"
    width="800px"
    :aria-label="title"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <ElForm
      :model="localForm"
      label-width="100px"
      :aria-label="t('voucher.voucherListForm.formAriaLabel')"
    >
      <ElRow :gutter="20">
        <ElCol :span="12">
          <ElFormItem :label="t('voucher.voucherListForm.labelVoucherNo')" prop="voucher_no">
            <ElInput v-model="localForm.voucher_no" readonly />
          </ElFormItem>
        </ElCol>
        <ElCol :span="12">
          <ElFormItem :label="t('voucher.voucherListForm.labelVoucherDate')" prop="voucher_date">
            <!-- P0 修复：value-format 输出 YYYY-MM-DD 字符串，与后端 NaiveDate 契约一致
                 （原无 value-format 时 v-model 为 Date 对象，序列化成 RFC3339 导致解析失败/400） -->
            <ElDatePicker v-model="localForm.voucher_date" type="date" value-format="YYYY-MM-DD" />
          </ElFormItem>
        </ElCol>
      </ElRow>
      <ElRow :gutter="20">
        <ElCol :span="12">
          <!-- P0 修复：绑定后端真实键 voucher_type（原自创键 type 提交即 400/更新 no-op）；
               选项来自 GET /vouchers/types（后端 available_voucher_types 单一真源） -->
          <ElFormItem :label="t('voucher.voucherListForm.labelVoucherType')" prop="voucher_type">
            <ElSelect
              v-model="localForm.voucher_type"
              :placeholder="t('voucher.voucherListForm.placeholderVoucherType')"
            >
              <ElOption
                v-for="vt in voucherTypes"
                :key="vt.value"
                :label="vt.label"
                :value="vt.value"
              />
            </ElSelect>
          </ElFormItem>
        </ElCol>
      </ElRow>
      <ElFormItem :label="t('voucher.voucherListForm.labelEntries')">
        <div class="entries-table">
          <div class="entries-header">
            <span class="col-subject">{{ t('voucher.voucherListForm.columnSubject') }}</span>
            <span class="col-debit">{{ t('voucher.voucherListForm.columnDebit') }}</span>
            <span class="col-credit">{{ t('voucher.voucherListForm.columnCredit') }}</span>
            <span class="col-desc">{{ t('voucher.voucherListForm.columnSummary') }}</span>
            <span class="col-action">{{ t('voucher.voucherListForm.columnAction') }}</span>
          </div>
          <div v-for="(entry, index) in localForm.entries || []" :key="index" class="entries-row">
            <ElSelect
              v-model="entry.account_subject_id"
              :placeholder="t('voucher.voucherListForm.placeholderSubject')"
              class="col-subject"
            >
              <ElOption
                v-for="subject in accountSubjectOptions"
                :key="subject.value"
                :label="subject.label"
                :value="subject.value"
              />
            </ElSelect>
            <ElInputNumber v-model="entry.debit_amount" :precision="2" class="col-debit" />
            <ElInputNumber v-model="entry.credit_amount" :precision="2" class="col-credit" />
            <ElInput
              v-model="entry.description"
              :placeholder="t('voucher.voucherListForm.placeholderSummaryEntry')"
              class="col-desc"
            />
            <ElButton
              v-if="(localForm.entries || []).length > 1"
              size="small"
              type="danger"
              @click="emit('remove-entry', index)"
            >
              {{ t('voucher.voucherListForm.buttonDelete') }}
            </ElButton>
          </div>
          <ElButton type="text" @click="emit('add-entry')">{{
            t('voucher.voucherListForm.buttonAddEntry')
          }}</ElButton>
        </div>
      </ElFormItem>
      <ElRow :gutter="20" class="total-row">
        <ElCol :span="12" class="total-item">
          <span class="label">{{ t('voucher.voucherListForm.labelDebitTotal') }}</span>
          <span class="value debit">{{ formatAmount(totalDebit) }}</span>
        </ElCol>
        <ElCol :span="12" class="total-item">
          <span class="label">{{ t('voucher.voucherListForm.labelCreditTotal') }}</span>
          <span class="value credit">{{ formatAmount(totalCredit) }}</span>
          <span v-if="Math.abs(totalDebit - totalCredit) > 0.01" class="error">
            {{ t('voucher.voucherListForm.textNotBalanced') }}
          </span>
          <span v-else class="success">{{ t('voucher.voucherListForm.textBalanced') }}</span>
        </ElCol>
      </ElRow>
    </ElForm>
    <template #footer>
      <ElButton @click="emit('update:visible', false)">{{
        t('voucher.voucherListForm.buttonCancel')
      }}</ElButton>
      <ElButton type="primary" @click="emit('submit')">{{
        t('voucher.voucherListForm.buttonConfirm')
      }}</ElButton>
    </template>
  </ElDialog>
</template>

<script setup lang="ts">
import { deepClone } from '@/utils';
import { ref, computed, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatAmount } from '../composables/vchrLstFmts';

const { t } = useI18n({ useScope: 'global' });

/**
 * 分录编辑模型：ElInputNumber 双向绑定与借贷合计实时计算均以 number 为契约。
 * 入参金额来自后端 rust_decimal（JSON 序列化为字符串，见 @/api/voucher 的 VoucherEntry），
 * 在 prop→local 边界经 Number() 归一，避免以字符串驱动数值控件。
 * UI 键沿用后端详情双命名里的 account_subject_id 与 debit/credit 金额、description 一族。
 */
interface VoucherEntryForm {
  account_subject_id: number;
  debit_amount: number | string;
  credit_amount: number | string;
  description?: string | null;
}

/** 表单模型（P0 契约修复：真实键 voucher_type；删除后端不存在列的 type/description/total_* 键） */
interface VoucherFormData {
  id?: number;
  voucher_no?: string;
  voucher_date?: string;
  voucher_type?: string;
  status?: string;
  entries?: VoucherEntryForm[];
  [key: string]: unknown;
}

interface SubjectOption {
  label: string;
  value: number;
}

/**
 * 凭证新建/编辑对话框组件
 * 接收父组件传入的 form / voucherTypes / accountSubjectOptions
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean;
  // 对话框标题
  title: string;
  // 表单数据（由父组件管理，金额字段为后端 Decimal 字符串，子组件通过 emit 回写）
  form: Partial<VoucherFormData>;
  // 凭证类型下拉选项
  voucherTypes: { label: string; value: string }[];
  // 科目下拉选项
  accountSubjectOptions: SubjectOption[];
}>();

const emit = defineEmits<{
  // 关闭对话框
  (e: 'update:visible', v: boolean): void;
  // 添加分录
  (e: 'add-entry'): void;
  // 删除分录
  (e: 'remove-entry', index: number): void;
  // 提交表单
  (e: 'submit'): void;
  // 整体回写表单（父组件监听此事件并 Object.assign 到自己的 form）
  (e: 'update:form', form: VoucherFormData): void;
}>();

// prop→local 边界归一：后端 Decimal 字符串金额转为 number（nullish 视为 0）。
const toNumberEntry = (e: VoucherEntryForm): VoucherEntryForm => ({
  account_subject_id: Number(e.account_subject_id ?? 0),
  debit_amount: Number(e.debit_amount ?? 0),
  credit_amount: Number(e.credit_amount ?? 0),
  description: e.description,
});

const toLocalForm = (f: Partial<VoucherFormData>): VoucherFormData => ({
  ...f,
  entries: (f.entries ?? []).map(toNumberEntry),
});

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// 注意：表单内有 entries 数组，需要深拷贝以保证本地修改与父组件解耦
const localForm = ref<VoucherFormData>(toLocalForm(deepClone(props.form)));

// 借/贷合计按本地分录实时派生，用于底部展示与不平衡提示。
// 与 useVchrLst.ts 的 totalDebit/totalCredit 同源（对 entries 求和，nullish 视为 0）。
const sumEntries = (pick: (entry: VoucherEntryForm) => number | string | undefined) =>
  (localForm.value.entries || []).reduce((sum, entry) => sum + Number(pick(entry) || 0), 0);
const totalDebit = computed(() => sumEntries(e => e.debit_amount));
const totalCredit = computed(() => sumEntries(e => e.credit_amount));

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local（如父组件新建/编辑时填充数据）
watch(
  () => props.form,
  newForm => {
    if (syncing) return;
    syncing = true;
    localForm.value = toLocalForm(deepClone(newForm));
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 本地变化时通知父组件（用户输入）
watch(
  localForm,
  newForm => {
    if (syncing) return;
    syncing = true;
    emit('update:form', deepClone(newForm));
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);
</script>

<style scoped>
.entries-table {
  border: 1px solid #ebeef5;
  border-radius: 4px;
}
.entries-header {
  display: flex;
  background: #f5f7fa;
  padding: 10px;
  font-weight: bold;
}
.entries-row {
  display: flex;
  padding: 10px;
  border-top: 1px solid #ebeef5;
}
.col-subject {
  flex: 2;
  margin-right: 10px;
}
.col-debit,
.col-credit {
  width: 120px;
  margin-right: 10px;
}
.col-desc {
  flex: 1;
  margin-right: 10px;
}
.col-action {
  width: 60px;
}
.total-row {
  display: flex;
  justify-content: flex-end;
  padding: 10px;
  background: #fafafa;
  margin-top: 10px;
}
.total-item {
  margin-left: 30px;
}
.total-item .label {
  margin-right: 10px;
  font-weight: bold;
}
.total-item .value {
  font-weight: bold;
  font-size: 16px;
}
.total-item .value.debit {
  color: #e74c3c;
}
.total-item .value.credit {
  color: #27ae60;
}
.total-item .error {
  color: #e74c3c;
  margin-left: 10px;
}
.total-item .success {
  color: #27ae60;
  margin-left: 10px;
}
</style>
