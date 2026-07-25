<!--
  OpportunityFollowTab.vue - 商机跟进记录对话框
  来源：原 crm/opportunities/index.vue 中 跟进记录对话框
-->
<template>
  <el-dialog v-model="visible" :title="t('crmOpportunityFollow.title')" width="600px" :aria-label="t('crmOpportunityFollow.ariaLabel')">
    <el-form :model="formData" label-width="80px" :aria-label="t('crmOpportunityFollow.formAriaLabel')">
      <el-form-item :label="t('crmOpportunityFollow.content')">
        <el-input
          v-model="formData.content"
          type="textarea"
          :rows="4"
          :placeholder="t('crmOpportunityFollow.contentPlaceholder')"
        />
      </el-form-item>
      <el-form-item :label="t('crmOpportunityFollow.nextFollowUp')">
        <el-date-picker
          v-model="formData.next_follow_up_date"
          type="date"
          :placeholder="t('crmOpportunityFollow.nextFollowUpPlaceholder')"
          style="width: 100%"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">{{ t('crmOpportunityFollow.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{ t('crmOpportunityFollow.confirm') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { logger } from '@/utils/logger'
// D14 Batch 5b：原 crmEnhancedApi 对象已转风格 B 函数
import { createFollowUp } from '@/api/crm-enhanced'

const { t } = useI18n({ useScope: 'global' })

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
    await createFollowUp(props.opportunityId, {
      type: 'opportunity',
      content: formData.content,
      next_follow_date: formData.next_follow_up_date,
    })
    ElMessage.success(t('crmOpportunityFollow.message.success'))
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('crmOpportunityFollow.message.failed'))
    logger.warn(t('crmOpportunityFollow.message.failed'), err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>
