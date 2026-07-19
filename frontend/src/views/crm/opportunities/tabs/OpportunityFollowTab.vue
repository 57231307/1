<!--
  OpportunityFollowTab.vue - 商机跟进记录对话框
  来源：原 crm/opportunities/index.vue 中 跟进记录对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog v-model="visible" title="跟进记录" width="600px" aria-label="跟进记录对话框">
    <el-form :model="formData" label-width="80px" aria-label="商机跟进表单">
      <el-form-item label="跟进内容">
        <el-input
          v-model="formData.content"
          type="textarea"
          :rows="4"
          placeholder="请输入跟进内容"
        />
      </el-form-item>
      <el-form-item label="下次跟进">
        <el-date-picker
          v-model="formData.next_follow_up_date"
          type="date"
          placeholder="请选择下次跟进日期"
          style="width: 100%"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
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
  opportunityId: number | null
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'submitted'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const visible = ref(props.modelValue)
const submitLoading = ref(false)

const formData = reactive({
  content: '',
  next_follow_up_date: '',
})

watch(
  () => props.modelValue,
  val => {
    visible.value = val
    if (val) {
      formData.content = ''
      formData.next_follow_up_date = ''
    }
  }
)

watch(visible, val => {
  emit('update:modelValue', val)
})

const handleSubmit = async () => {
  if (!props.opportunityId) return
  try {
    submitLoading.value = true
    // P1-5：实际调用跟进记录保存 API
    await crmEnhancedApi.createFollowUp(props.opportunityId, {
      type: 'opportunity',
      content: formData.content,
      next_follow_date: formData.next_follow_up_date,
    })
    ElMessage.success('跟进成功')
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '跟进失败')
    logger.warn('跟进失败', err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>
