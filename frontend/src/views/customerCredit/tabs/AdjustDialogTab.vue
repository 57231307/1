<!--
  AdjustDialogTab.vue - 客户信用额度调整对话框
  来源：原 customerCredit/index.vue 中 调整额度对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog v-model="visible" title="调整信用额度" width="500px">
    <el-form ref="formRef" :model="form" :rules="rules" label-width="120px">
      <el-form-item label="调整类型" prop="adjustmentType">
        <el-radio-group v-model="form.adjustmentType">
          <el-radio value="increase">增加额度</el-radio>
          <el-radio value="decrease">减少额度</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item label="调整金额" prop="amount">
        <el-input-number v-model="form.amount" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item label="调整原因" prop="reason">
        <el-input v-model="form.reason" type="textarea" :rows="3" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确认</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { adjustCreditLimit } from '@/api/customer-credit'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
  customerId: number | null
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'submitted'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const visible = ref(props.modelValue)
const submitLoading = ref(false)
const formRef = ref<FormInstance>()

const form = reactive({
  adjustmentType: 'increase' as 'increase' | 'decrease',
  amount: 0,
  reason: '',
})

const rules: FormRules = {
  adjustmentType: [{ required: true, message: '请选择调整类型', trigger: 'change' }],
  amount: [{ required: true, message: '请输入调整金额', trigger: 'blur' }],
  reason: [{ required: true, message: '请输入调整原因', trigger: 'blur' }],
}

watch(
  () => props.modelValue,
  val => {
    visible.value = val
    if (val) {
      form.adjustmentType = 'increase'
      form.amount = 0
      form.reason = ''
    }
  }
)

watch(visible, val => {
  emit('update:modelValue', val)
})

const handleSubmit = async () => {
  if (!formRef.value || !props.customerId) return
  try {
    await formRef.value.validate()
    submitLoading.value = true
    await adjustCreditLimit(props.customerId, {
      type: form.adjustmentType,
      amount: form.amount,
      reason: form.reason,
    })
    ElMessage.success('调整成功')
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '调整失败')
    logger.warn('调整信用额度失败', err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>
