<!--
  TagsPanelTab.vue - 客户标签管理 Tab
  来源：原 crm/detail.vue 中 标签管理 section
-->
<template>
  <el-card shadow="hover" class="mt-20">
    <template #header>
      <div class="card-header">
        <span>{{ t('crmTagsPanel.title') }}</span>
        <el-button type="primary" size="small" @click="openDialog">
          <el-icon><Plus /></el-icon>
          {{ t('crmTagsPanel.addTag') }}
        </el-button>
      </div>
    </template>

    <div class="tags-container">
      <el-tag
        v-for="tag in tags"
        :key="tag.id"
        :color="tag.color"
        class="tag-item"
        closable
        @close="handleRemove(tag.id)"
      >
        {{ tag.name }}
      </el-tag>
      <span v-if="!tags.length" class="no-tags">{{ t('crmTagsPanel.empty') }}</span>
    </div>

    <el-dialog v-model="dialogVisible" :title="t('crmTagsPanel.dialog.title')" width="400px" :aria-label="t('crmTagsPanel.dialog.ariaLabel')">
      <el-form ref="formRef" :model="form" label-width="80px" :aria-label="t('crmTagsPanel.dialog.formAriaLabel')">
        <el-form-item :label="t('crmTagsPanel.dialog.nameLabel')" prop="name">
          <el-select v-model="form.name" :placeholder="t('crmTagsPanel.dialog.namePlaceholder')" style="width: 100%">
            <el-option
              v-for="tag in availableTags"
              :key="tag.id"
              :label="tag.name"
              :value="tag.name"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('crmTagsPanel.dialog.cancel') }}</el-button>
        <el-button type="primary" @click="handleAdd">{{ t('crmTagsPanel.dialog.confirm') }}</el-button>
      </template>
    </el-dialog>
  </el-card>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import type { FormInstance } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
// D14 Batch 5b：原 crmEnhancedApi 对象已转风格 B 函数
import { getCrmTagList, createTagForCustomer, deleteTagFromCustomer, type CustomerTag } from '@/api/crm-enhanced'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

interface Props {
  customerId: number
  tags: CustomerTag[]
}

const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'updated'): void
}>()

const availableTags = ref<CustomerTag[]>([])
const dialogVisible = ref(false)
const formRef = ref<FormInstance>()

const form = reactive({
  name: '',
})

const fetchTags = async () => {
  try {
    const res = await getCrmTagList()
    availableTags.value = res.data || []
  } catch (error) {
    const err = error as Error
    logger.warn(t('crmTagsPanel.message.loadFailed'), err.message)
    availableTags.value = []
  }
}

const openDialog = () => {
  form.name = ''
  dialogVisible.value = true
}

const handleAdd = async () => {
  if (!form.name) {
    ElMessage.warning(t('crmTagsPanel.message.selectRequired'))
    return
  }

  const selectedTag = availableTags.value.find(tag => tag.name === form.name)
  if (!selectedTag) return

  try {
    await createTagForCustomer(props.customerId, selectedTag.id)
    ElMessage.success(t('crmTagsPanel.message.addSuccess'))
    dialogVisible.value = false
    form.name = ''
    emit('updated')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('crmTagsPanel.message.addFailed'))
  }
}

const handleRemove = async (tagId: number) => {
  try {
    await deleteTagFromCustomer(props.customerId, tagId)
    ElMessage.success(t('crmTagsPanel.message.removeSuccess'))
    emit('updated')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('crmTagsPanel.message.removeFailed'))
  }
}

onMounted(() => {
  fetchTags()
})
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 600;
}
.mt-20 {
  margin-top: 20px;
}
.tags-container {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  min-height: 40px;
}
.tag-item {
  border: none;
}
.no-tags {
  color: #909399;
  font-size: 13px;
}
</style>
