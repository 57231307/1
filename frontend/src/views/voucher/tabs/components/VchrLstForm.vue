<!--
  VchrLstForm.vue - 凭证新建/编辑对话框
  拆分自 voucher/tabs/VoucherListTab.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <ElDialog
    :model-value="visible"
    :title="title"
    width="800px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <ElForm :model="form" label-width="100px">
      <ElRow :gutter="20">
        <ElCol :span="12">
          <ElFormItem label="凭证号" prop="voucher_no">
            <ElInput :model-value="form.voucher_no" readonly @update:model-value="(v: string) => (form.voucher_no = v)" />
          </ElFormItem>
        </ElCol>
        <ElCol :span="12">
          <ElFormItem label="凭证日期" prop="voucher_date">
            <ElDatePicker
              :model-value="form.voucher_date"
              type="date"
              @update:model-value="(v: string) => (form.voucher_date = v ?? '')"
            />
          </ElFormItem>
        </ElCol>
      </ElRow>
      <ElRow :gutter="20">
        <ElCol :span="12">
          <ElFormItem label="凭证类型" prop="type">
            <ElSelect
              :model-value="form.type"
              placeholder="请选择凭证类型"
              @update:model-value="(v: string) => (form.type = v)"
            >
              <ElOption
                v-for="t in voucherTypes"
                :key="t.value"
                :label="t.label"
                :value="t.value"
              />
            </ElSelect>
          </ElFormItem>
        </ElCol>
        <ElCol :span="12">
          <ElFormItem label="摘要" prop="description">
            <ElInput
              :model-value="form.description ?? ''"
              placeholder="请输入摘要"
              @update:model-value="(v: string) => (form.description = v)"
            />
          </ElFormItem>
        </ElCol>
      </ElRow>
      <ElFormItem label="分录明细">
        <div class="entries-table">
          <div class="entries-header">
            <span class="col-subject">会计科目</span>
            <span class="col-debit">借方金额</span>
            <span class="col-credit">贷方金额</span>
            <span class="col-desc">摘要</span>
            <span class="col-action">操作</span>
          </div>
          <div v-for="(entry, index) in form.entries" :key="index" class="entries-row">
            <ElSelect
              :model-value="entry.account_subject_id"
              placeholder="选择科目"
              class="col-subject"
              @update:model-value="(v: number) => (entry.account_subject_id = v)"
            >
              <ElOption
                v-for="subject in accountSubjectOptions"
                :key="subject.value"
                :label="subject.label"
                :value="subject.value"
              />
            </ElSelect>
            <ElInputNumber
              :model-value="entry.debit_amount"
              :precision="2"
              class="col-debit"
              @update:model-value="(v: number) => (entry.debit_amount = v ?? 0)"
            />
            <ElInputNumber
              :model-value="entry.credit_amount"
              :precision="2"
              class="col-credit"
              @update:model-value="(v: number) => (entry.credit_amount = v ?? 0)"
            />
            <ElInput
              :model-value="entry.description ?? ''"
              placeholder="摘要"
              class="col-desc"
              @update:model-value="(v: string) => (entry.description = v)"
            />
            <ElButton
              v-if="(form.entries || []).length > 1"
              size="small"
              type="danger"
              @click="emit('remove-entry', index)"
            >
              删除
            </ElButton>
          </div>
          <ElButton type="text" @click="emit('add-entry')">+ 添加分录</ElButton>
        </div>
      </ElFormItem>
      <ElRow :gutter="20" class="total-row">
        <ElCol :span="12" class="total-item">
          <span class="label">借方合计:</span>
          <span class="value debit">{{ formatAmount(form.total_debit) }}</span>
        </ElCol>
        <ElCol :span="12" class="total-item">
          <span class="label">贷方合计:</span>
          <span class="value credit">{{ formatAmount(form.total_credit) }}</span>
          <span
            v-if="Math.abs((form.total_debit ?? 0) - (form.total_credit ?? 0)) > 0.01"
            class="error"
          >
            借贷不平
          </span>
          <span v-else class="success">借贷平衡</span>
        </ElCol>
      </ElRow>
    </ElForm>
    <template #footer>
      <ElButton @click="emit('update:visible', false)">取消</ElButton>
      <ElButton type="primary" @click="emit('submit')">确定</ElButton>
    </template>
  </ElDialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import type { VoucherEntity } from '@/api/voucher'
import { formatAmount } from '../composables/vchrLstFmts'

interface VoucherEntry {
  account_subject_id: number
  debit_amount: number
  credit_amount: number
  description?: string
}

/** 父组件传 Partial<VoucherEntity>，所有字段均可选 */
type VoucherForm = {
  id?: number
  voucher_no?: string
  voucher_date?: string
  type?: string
  status?: string
  description?: string
  total_debit?: number
  total_credit?: number
  entries?: VoucherEntry[]
  [key: string]: unknown
}

interface SubjectOption {
  label: string
  value: number
}

/**
 * 凭证新建/编辑对话框组件
 * 接收父组件传入的 form / voucherTypes / accountSubjectOptions
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 对话框标题
  title: string
  // 表单数据
  form: VoucherForm
  // 凭证类型下拉选项
  voucherTypes: { label: string; value: string }[]
  // 科目下拉选项
  accountSubjectOptions: SubjectOption[]
}>()

const emit = defineEmits<{
  // 关闭对话框
  'update:visible': [v: boolean]
  // 添加分录
  'add-entry': []
  // 删除分录
  'remove-entry': [index: number]
  // 提交表单
  submit: []
}>()

void props
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
