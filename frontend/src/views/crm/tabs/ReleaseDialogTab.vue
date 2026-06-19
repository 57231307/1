<!--
  ReleaseDialogTab.vue - 客户公海池 - 释放对话框
  来源：原 crm/pool.vue 中 释放对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog v-model="visible" title="释放到公海" width="500px">
    <p>
      将客户 <strong>{{ customerName }}</strong> 释放到公海池？
    </p>
    <el-form :model="form" label-width="80px">
      <el-form-item label="释放原因">
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
import { logger } from '@/utils/logger'
import { crmEnhancedApi } from '@/api/crm-enhanced'

interface Props {
  modelValue: boolean
  customerName: string
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
const form = reactive({ reason: '' })

watch(
  () => props.modelValue,
  val => {
    visible.value = val
    if (val) form.reason = ''
  }
)

watch(visible, val => {
  emit('update:modelValue', val)
})

const handleSubmit = async () => {
  if (!props.customerId) return
  try {
    submitLoading.value = true
    // P1-5：实际调用释放 API（recycle 释放客户到公海池）
    await crmEnhancedApi.recycleToPool({
      customer_ids: [props.customerId],
      reason: form.reason,
    })
    ElMessage.success('释放成功')
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '释放失败')
    logger.warn('释放失败', err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>
