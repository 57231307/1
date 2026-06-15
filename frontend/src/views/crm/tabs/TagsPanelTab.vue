<!--
  TagsPanelTab.vue - 客户标签管理 Tab
  来源：原 crm/detail.vue 中 标签管理 section
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-card shadow="hover" class="mt-20">
    <template #header>
      <div class="card-header">
        <span>标签管理</span>
        <el-button type="primary" size="small" @click="openDialog">
          <el-icon><Plus /></el-icon>
          添加标签
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
      <span v-if="!tags.length" class="no-tags">暂无标签</span>
    </div>

    <el-dialog v-model="dialogVisible" title="添加标签" width="400px">
      <el-form ref="formRef" :model="form" label-width="80px">
        <el-form-item label="标签名称" prop="name">
          <el-select v-model="form.name" placeholder="选择已有标签" style="width: 100%">
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
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleAdd">确定</el-button>
      </template>
    </el-dialog>
  </el-card>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import crmEnhancedApi, { type CustomerTag } from '@/api/crm-enhanced'
import { logger } from '@/utils/logger'

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
    const res = await crmEnhancedApi.getTags()
    availableTags.value = res.data || []
  } catch (error) {
    const err = error as Error
    logger.warn('获取标签列表失败', err.message)
    availableTags.value = []
  }
}

const openDialog = () => {
  form.name = ''
  dialogVisible.value = true
}

const handleAdd = async () => {
  if (!form.name) {
    ElMessage.warning('请选择标签')
    return
  }

  const selectedTag = availableTags.value.find(t => t.name === form.name)
  if (!selectedTag) return

  try {
    await crmEnhancedApi.addTagToCustomer(props.customerId, selectedTag.id)
    ElMessage.success('标签已添加')
    dialogVisible.value = false
    form.name = ''
    emit('updated')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '添加标签失败')
  }
}

const handleRemove = async (tagId: number) => {
  try {
    await crmEnhancedApi.removeTagFromCustomer(props.customerId, tagId)
    ElMessage.success('标签已移除')
    emit('updated')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '移除标签失败')
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
