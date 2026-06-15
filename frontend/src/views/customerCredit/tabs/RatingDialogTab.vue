<!--
  RatingDialogTab.vue - 客户信用评级对话框
  来源：原 customerCredit/index.vue 中 设置评级对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog v-model="visible" title="设置信用评级" width="500px">
    <el-form ref="formRef" :model="form" :rules="rules" label-width="120px">
      <el-form-item label="客户" prop="customer_id">
        <el-select v-model="form.customer_id" placeholder="请选择客户" style="width: 100%">
          <el-option v-for="c in customers" :key="c.id" :label="c.customer_name" :value="c.id" />
        </el-select>
      </el-form-item>
      <el-form-item label="信用等级" prop="creditLevel">
        <el-select v-model="form.creditLevel" placeholder="请选择信用等级" style="width: 100%">
          <el-option label="AAA" value="AAA" />
          <el-option label="AA" value="AA" />
          <el-option label="A" value="A" />
          <el-option label="BBB" value="BBB" />
          <el-option label="BB" value="BB" />
          <el-option label="B" value="B" />
          <el-option label="C" value="C" />
          <el-option label="D" value="D" />
        </el-select>
      </el-form-item>
      <el-form-item label="信用分" prop="creditScore">
        <el-input-number v-model="form.creditScore" :min="0" :max="100" style="width: 100%" />
      </el-form-item>
      <el-form-item label="信用额度" prop="creditLimit">
        <el-input-number v-model="form.creditLimit" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item label="账期(天)" prop="creditDays">
        <el-input-number v-model="form.creditDays" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item label="备注" prop="remark">
        <el-input v-model="form.remark" type="textarea" :rows="3" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">保存</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { setCreditRating } from '@/api/customer-credit'
import type { Customer } from '@/api/customer'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
  customers: Customer[]
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
  customer_id: undefined as number | undefined,
  creditLevel: '',
  creditScore: 0,
  creditLimit: 0,
  creditDays: 0,
  remark: '',
})

const rules: FormRules = {
  customer_id: [{ required: true, message: '请选择客户', trigger: 'change' }],
  creditLevel: [{ required: true, message: '请选择信用等级', trigger: 'change' }],
  creditScore: [{ required: true, message: '请输入信用分', trigger: 'blur' }],
  creditLimit: [{ required: true, message: '请输入信用额度', trigger: 'blur' }],
  creditDays: [{ required: true, message: '请输入账期', trigger: 'blur' }],
}

watch(
  () => props.modelValue,
  val => {
    visible.value = val
    if (val) {
      resetForm()
    }
  }
)

watch(visible, val => {
  emit('update:modelValue', val)
})

const resetForm = () => {
  form.customer_id = undefined
  form.creditLevel = ''
  form.creditScore = 0
  form.creditLimit = 0
  form.creditDays = 0
  form.remark = ''
  formRef.value?.clearValidate()
}

const handleSubmit = async () => {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
    submitLoading.value = true
    await setCreditRating(form.customer_id as number, {
      rating: form.creditLevel,
      credit_limit: form.creditLimit,
      reason: form.remark,
    })
    ElMessage.success('设置成功')
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '设置失败')
    logger.warn('设置信用评级失败', err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>
