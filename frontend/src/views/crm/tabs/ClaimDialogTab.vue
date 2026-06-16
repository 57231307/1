<!--
  ClaimDialogTab.vue - 客户公海池 - 领取对话框
  来源：原 crm/pool.vue 中 领取对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog v-model="visible" title="领取客户" width="500px">
    <p>
      确认将客户 <strong>{{ customerName }}</strong> 领取到我的客户池？
    </p>
    <el-form :model="form" label-width="80px">
      <el-form-item label="备注">
        <el-input v-model="form.remark" type="textarea" :rows="3" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确认领取</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { logger } from '@/utils/logger'

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
const form = reactive({ remark: '' })

watch(
  () => props.modelValue,
  val => {
    visible.value = val
    if (val) form.remark = ''
  }
)

watch(visible, val => {
  emit('update:modelValue', val)
})

const handleSubmit = async () => {
  if (!props.customerId) return
  try {
    submitLoading.value = true
    // TODO: 实际调用领取 API
    ElMessage.success('领取成功')
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '领取失败')
    logger.warn('领取失败', err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>
