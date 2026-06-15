<!--
  AmountDialogTab.vue - 客户信用占用/释放对话框
  来源：原 customerCredit/index.vue 中 占用/释放额度对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog
    v-model="visible"
    :title="operationType === 'occupy' ? '占用额度' : '释放额度'"
    width="500px"
  >
    <el-form ref="formRef" :model="form" :rules="rules" label-width="120px">
      <el-form-item label="金额" prop="amount">
        <el-input-number v-model="form.amount" :min="0" style="width: 100%" />
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
import { occupyCredit, releaseCredit } from '@/api/customer-credit'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
  customerId: number | null
  operationType: 'occupy' | 'release'
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
  amount: 0,
})

const rules: FormRules = {
  amount: [{ required: true, message: '请输入金额', trigger: 'blur' }],
}

watch(
  () => props.modelValue,
  val => {
    visible.value = val
    if (val) {
      form.amount = 0
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
    if (props.operationType === 'occupy') {
      await occupyCredit(props.customerId, {
        amount: form.amount,
        business_type: 'manual',
        business_id: 0,
      })
    } else {
      await releaseCredit(props.customerId, 0)
    }
    ElMessage.success('操作成功')
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '操作失败')
    logger.warn('信用操作失败', err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>
