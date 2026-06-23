<!--
  VchrLstForm.vue - 凭证新建/编辑对话框
  拆分自 voucher/tabs/VoucherListTab.vue（P14 批 2 I-3 第 1 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <ElDialog
    :model-value="visible"
    :title="title"
    width="800px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <ElForm :model="localForm" label-width="100px">
      <ElRow :gutter="20">
        <ElCol :span="12">
          <ElFormItem label="凭证号" prop="voucher_no">
            <ElInput v-model="localForm.voucher_no" readonly />
          </ElFormItem>
        </ElCol>
        <ElCol :span="12">
          <ElFormItem label="凭证日期" prop="voucher_date">
            <ElDatePicker v-model="localForm.voucher_date" type="date" />
          </ElFormItem>
        </ElCol>
      </ElRow>
      <ElRow :gutter="20">
        <ElCol :span="12">
          <ElFormItem label="凭证类型" prop="type">
            <ElSelect v-model="localForm.type" placeholder="请选择凭证类型">
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
            <ElInput v-model="localForm.description" placeholder="请输入摘要" />
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
          <div v-for="(entry, index) in (localForm.entries || [])" :key="index" class="entries-row">
            <ElSelect
              v-model="entry.account_subject_id"
              placeholder="选择科目"
              class="col-subject"
            >
              <ElOption
                v-for="subject in accountSubjectOptions"
                :key="subject.value"
                :label="subject.label"
                :value="subject.value"
              />
            </ElSelect>
            <ElInputNumber
              v-model="entry.debit_amount"
              :precision="2"
              class="col-debit"
            />
            <ElInputNumber
              v-model="entry.credit_amount"
              :precision="2"
              class="col-credit"
            />
            <ElInput v-model="entry.description" placeholder="摘要" class="col-desc" />
            <ElButton
              v-if="(localForm.entries || []).length > 1"
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
          <span class="value debit">{{ formatAmount(localForm.total_debit) }}</span>
        </ElCol>
        <ElCol :span="12" class="total-item">
          <span class="label">贷方合计:</span>
          <span class="value credit">{{ formatAmount(localForm.total_credit) }}</span>
          <span
            v-if="Math.abs((localForm.total_debit ?? 0) - (localForm.total_credit ?? 0)) > 0.01"
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
import { ref, watch, nextTick } from 'vue'
import { formatAmount } from '../composables/vchrLstFmts'

interface VoucherEntry {
  account_subject_id: number
  debit_amount: number
  credit_amount: number
  description?: string
}

/** 父组件传 Partial 类型，所有字段均可选 */
interface VoucherForm {
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
  // 表单数据（由父组件管理，子组件通过 emit 回写）
  form: VoucherForm
  // 凭证类型下拉选项
  voucherTypes: { label: string; value: string }[]
  // 科目下拉选项
  accountSubjectOptions: SubjectOption[]
}>()

const emit = defineEmits<{
  // 关闭对话框
  (e: 'update:visible', v: boolean): void
  // 添加分录
  (e: 'add-entry'): void
  // 删除分录
  (e: 'remove-entry', index: number): void
  // 提交表单
  (e: 'submit'): void
  // 整体回写表单（父组件监听此事件并 Object.assign 到自己的 form）
  (e: 'update:form', form: VoucherForm): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// 注意：表单内有 entries 数组，需要深拷贝以保证本地修改与父组件解耦
const localForm = ref<VoucherForm>(JSON.parse(JSON.stringify(props.form)))

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件新建/编辑时填充数据）
watch(
  () => props.form,
  (newForm) => {
    if (syncing) return
    syncing = true
    localForm.value = JSON.parse(JSON.stringify(newForm))
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    emit('update:form', JSON.parse(JSON.stringify(newForm)))
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
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
