<!--
  VchrForm.vue - 凭证新建表单对话框
  拆分自 VoucherTab.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    title="新建凭证"
    width="800px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      :ref="(el: any) => (formRefValue = el as FormInstance)"
      :model="voucherForm"
      :rules="voucherRules"
      label-width="80px"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="凭证日期" prop="voucher_date">
            <el-date-picker
              :model-value="voucherForm.voucher_date"
              type="date"
              placeholder="选择日期"
              value-format="YYYY-MM-DD"
              style="width: 100%"
              @update:model-value="(v: string) => (voucherForm.voucher_date = v)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="凭证类型" prop="voucher_type">
            <el-select
              :model-value="voucherForm.voucher_type"
              placeholder="选择类型"
              style="width: 100%"
              @update:model-value="(v: string) => (voucherForm.voucher_type = v)"
            >
              <el-option label="记" value="JZ" />
              <el-option label="收" value="SK" />
              <el-option label="付" value="FK" />
              <el-option label="转" value="ZZ" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider>分录明细</el-divider>
      <el-table :data="voucherForm.entries" stripe>
        <el-table-column label="摘要" min-width="150">
          <template #default="{ row }">
            <el-input
              :model-value="row.summary"
              placeholder="摘要"
              @update:model-value="(v: string) => (row.summary = v)"
            />
          </template>
        </el-table-column>
        <el-table-column label="科目" min-width="200">
          <template #default="{ row }">
            <el-tree-select
              :model-value="row.subject_id"
              :data="leafSubjects"
              :props="{ label: 'name', value: 'id' }"
              placeholder="选择科目"
              check-strictly
              @update:model-value="(v: number) => (row.subject_id = v)"
            />
          </template>
        </el-table-column>
        <el-table-column label="借方金额" width="130">
          <template #default="{ row }">
            <el-input-number
              :model-value="row.debit"
              :min="0"
              :precision="2"
              :controls="false"
              style="width: 100%"
              @update:model-value="(v: number) => (row.debit = v ?? 0)"
            />
          </template>
        </el-table-column>
        <el-table-column label="贷方金额" width="130">
          <template #default="{ row }">
            <el-input-number
              :model-value="row.credit"
              :min="0"
              :precision="2"
              :controls="false"
              style="width: 100%"
              @update:model-value="(v: number) => (row.credit = v ?? 0)"
            />
          </template>
        </el-table-column>
        <el-table-column label="操作" width="80">
          <template #default="{ $index }">
            <el-button type="danger" link size="small" @click="emit('remove-entry', $index)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>
      <div class="entry-footer">
        <el-button type="primary" link @click="emit('add-entry')">添加分录</el-button>
        <div class="entry-summary">
          <span>借方合计: {{ formatMoney(totalDebit) }}</span>
          <span>贷方合计: {{ formatMoney(totalCredit) }}</span>
          <span :class="{ 'text-red': !isBalanced }">{{ isBalanced ? '已平衡' : '未平衡' }}</span>
        </div>
      </div>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button
        type="primary"
        :loading="voucherSubmitLoading"
        @click="emit('submit-form')"
        >确定</el-button
      >
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { ref, watch } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import type { AccountSubject } from '@/api/finance'

interface VoucherEntry {
  subject_id: number | undefined
  debit: number
  credit: number
  summary: string
}

interface VoucherForm {
  voucher_date: string
  voucher_type: string
  entries: VoucherEntry[]
}

/**
 * 凭证新建表单对话框组件
 * 接收父组件传入的 form / ref / 业务方法
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 表单实例 ref（父组件持有的 FormInstance 引用包装对象）
  voucherFormRef: { value: FormInstance | undefined }
  // 表单数据
  voucherForm: VoucherForm
  // 提交中状态
  voucherSubmitLoading: boolean
  // 校验规则
  voucherRules: FormRules
  // 叶子科目列表
  leafSubjects: AccountSubject[]
  // 借贷合计
  totalDebit: number
  totalCredit: number
  // 是否平衡
  isBalanced: boolean
  // 金额格式化
  formatMoney: (amount: number) => string
}>()

const emit = defineEmits<{
  // 关闭对话框
  'update:visible': [v: boolean]
  // 添加分录
  'add-entry': []
  // 删除分录
  'remove-entry': [index: number]
  // 提交表单
  'submit-form': []
}>()

// 将 el-form 的 ref 实例同步到父组件传入的 voucherFormRef.value
const formRefValue = ref<FormInstance | undefined>(undefined)
watch(
  formRefValue,
  val => {
    if (val) props.voucherFormRef.value = val
  },
  { immediate: true, flush: 'post' }
)
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
