<!--
  ColorCardIssueForm.vue - 色卡发放表单组件（V15 P0-F11）

  设计原则：纯展示 + 受控表单。组件只负责 UI 渲染与表单校验，
  提交时 emit('submit', form) 由父组件调用 composable 处理业务。

  关联：
    - composables/useColorCardIssue.ts
    - store/colorCardIssue.ts
    - types/colorCardIssue.ts::IssueFormState
-->
<template>
  <el-form
    :model="formState"
    :rules="rules"
    ref="formRef"
    label-width="100px"
    style="max-width: 600px"
  >
    <el-form-item label="选择色卡" prop="color_card_id">
      <el-select
        v-model="formState.color_card_id"
        filterable
        placeholder="搜索色卡编号或名称"
        style="width: 100%"
      >
        <el-option
          v-for="card in availableCards"
          :key="card.id"
          :label="`${card.card_no} - ${card.card_name}`"
          :value="card.id"
        />
      </el-select>
    </el-form-item>

    <el-form-item label="客户 ID" prop="customer_id">
      <el-input-number
        v-model="formState.customer_id"
        :min="1"
        style="width: 100%"
        placeholder="请输入客户 ID"
      />
    </el-form-item>

    <el-form-item label="发放数量">
      <el-input-number
        v-model="formState.issue_qty"
        :min="1"
        :step="1"
        style="width: 100%"
      />
      <span class="form-hint">V15 P0-F10：后端会校验 stock_quantity >= 发放数量</span>
    </el-form-item>

    <el-form-item label="染色批号">
      <el-input
        v-model="formState.dye_lot_no"
        placeholder="可选: 染色批号（lot 概念，防色差混批）"
      />
    </el-form-item>

    <el-form-item label="预计归还">
      <el-date-picker
        v-model="formState.expected_return_date"
        type="date"
        placeholder="可选: 预计归还日期（不超过 30 天）"
        style="width: 100%"
        value-format="YYYY-MM-DD"
      />
    </el-form-item>

    <el-form-item label="用途">
      <el-input v-model="formState.purpose" placeholder="例如: 客户选色 / 展会展示" />
    </el-form-item>

    <el-form-item label="备注">
      <el-input v-model="formState.remark" type="textarea" :rows="2" />
    </el-form-item>

    <el-form-item>
      <el-button type="primary" :loading="loading" @click="handleSubmit">
        确认发放
      </el-button>
      <el-button @click="handleReset">重置</el-button>
    </el-form-item>
  </el-form>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import type { FormInstance } from 'element-plus'
import type { ColorCardListItem } from '@/api/color-card'
import type { IssueFormState } from '@/types/colorCardIssue'

const props = defineProps<{
  availableCards: ColorCardListItem[]
  loading: boolean
}>()

const emit = defineEmits<{
  (e: 'submit', form: IssueFormState): void
}>()

const formRef = ref<FormInstance>()

// 默认表单值（reset 时复用）
const createDefaultForm = (): IssueFormState => ({
  color_card_id: undefined,
  customer_id: 1,
  issue_qty: 1,
  expected_return_date: '',
  purpose: '',
  remark: '',
  dye_lot_no: '',
})

const formState = reactive<IssueFormState>(createDefaultForm())

const rules = {
  color_card_id: [{ required: true, message: '请选择色卡', trigger: 'change' }],
  customer_id: [{ required: true, message: '请输入客户 ID', trigger: 'blur' }],
}

async function handleSubmit(): Promise<void> {
  if (!formRef.value) return
  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    emit('submit', { ...formState })
  })
}

function handleReset(): void {
  Object.assign(formState, createDefaultForm())
  formRef.value?.clearValidate()
}

// 暴露 reset 方法给父组件（可选）
defineExpose({
  reset: handleReset,
})

// 兼容外部 reset 触发：监听 loading 由 true → false 且表单已提交后清空
watch(
  () => props.loading,
  (newVal, oldVal) => {
    // loading 由 true 转 false 表示操作结束，父组件可在 emit('submit') 成功后调用 reset
    if (oldVal === true && newVal === false) {
      // 不在此自动重置，保留用户输入供多次发放
    }
  },
)
</script>

<style scoped>
.form-hint {
  margin-left: 12px;
  color: #909399;
  font-size: 12px;
}
</style>
