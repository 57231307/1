<!--
  TagsPanelTab.vue - 客户标签管理 Tab
  来源：原 crm/detail.vue 中 标签管理 section
-->
<template>
  <el-card shadow="hover" class="mt-20">
    <template #header>
      <div class="card-header">
        <span>{{ t('crmTagsPanel.title') }}</span>
        <div style="display: flex; gap: 8px">
          <el-button size="small" @click="openGlobalTagDialog">
            {{ t('crmTagsPanel.newGlobalTag') || '新建全局标签' }}
          </el-button>
          <el-button type="primary" size="small" @click="openDialog">
            <el-icon><Plus /></el-icon>
            {{ t('crmTagsPanel.addTag') }}
          </el-button>
        </div>
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

    <div v-if="availableTags.length" style="margin-top: 12px">
      <span style="font-size: 12px; color: #909399">全局标签库：</span>
      <el-tag
        v-for="tag in availableTags"
        :key="`g-${tag.id}`"
        class="tag-item"
        style="margin-right: 6px"
        closable
        @close="removeGlobalTag(tag)"
      >
        {{ tag.name }}
      </el-tag>
    </div>

    <el-dialog
      v-model="dialogVisible"
      :title="t('crmTagsPanel.dialog.title')"
      width="400px"
      :aria-label="t('crmTagsPanel.dialog.ariaLabel')"
    >
      <el-form
        ref="formRef"
        :model="form"
        label-width="80px"
        :aria-label="t('crmTagsPanel.dialog.formAriaLabel')"
      >
        <el-form-item :label="t('crmTagsPanel.dialog.nameLabel')" prop="name">
          <el-select
            v-model="form.name"
            :placeholder="t('crmTagsPanel.dialog.namePlaceholder')"
            style="width: 100%"
          >
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
        <el-button type="primary" @click="handleAdd">{{
          t('crmTagsPanel.dialog.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 新建全局标签对话框 -->
    <el-dialog v-model="createTagVisible" title="新建全局标签" width="440px">
      <el-form :model="globalTagForm" label-width="90px">
        <el-form-item label="标签名称">
          <el-input v-model="globalTagForm.name" />
        </el-form-item>
        <el-form-item label="颜色">
          <el-color-picker v-model="globalTagForm.color" />
        </el-form-item>
        <el-form-item label="分类">
          <el-input v-model="globalTagForm.category" placeholder="general" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createTagVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="globalTagSubmitting" @click="submitGlobalTag">
          {{ t('common.save') }}
        </el-button>
      </template>
    </el-dialog>
  </el-card>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
// D14 Batch 5b：原 crmEnhancedApi 对象已转风格 B 函数
import {
  getCrmTagList,
  createTagForCustomer,
  deleteTagFromCustomer,
  createCrmTag,
  deleteCrmTag,
  type CustomerTag,
} from '@/api/crm-enhanced';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  customerId: number;
  tags: CustomerTag[];
}

const props = defineProps<Props>();
const emit = defineEmits<{
  (e: 'updated'): void;
}>();

const availableTags = ref<CustomerTag[]>([]);
const dialogVisible = ref(false);
const formRef = ref<FormInstance>();

const form = reactive({
  name: '',
});

const fetchTags = async () => {
  try {
    const res = await getCrmTagList();
    availableTags.value = res.data || [];
  } catch (error) {
    const err = error as Error;
    logger.warn(t('crmTagsPanel.message.loadFailed'), err.message);
    availableTags.value = [];
  }
};

const openDialog = () => {
  form.name = '';
  dialogVisible.value = true;
};

const handleAdd = async () => {
  if (!form.name) {
    ElMessage.warning(t('crmTagsPanel.message.selectRequired'));
    return;
  }

  const selectedTag = availableTags.value.find(tag => tag.name === form.name);
  if (!selectedTag) return;

  try {
    await createTagForCustomer(props.customerId, selectedTag.id);
    ElMessage.success(t('crmTagsPanel.message.addSuccess'));
    dialogVisible.value = false;
    form.name = '';
    emit('updated');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('crmTagsPanel.message.addFailed'));
  }
};

const handleRemove = async (tagId: number) => {
  try {
    await deleteTagFromCustomer(props.customerId, tagId);
    ElMessage.success(t('crmTagsPanel.message.removeSuccess'));
    emit('updated');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('crmTagsPanel.message.removeFailed'));
  }
};

// 全局标签库管理：新建全局标签（createCrmTag）/ 删除全局标签（deleteCrmTag）
const createTagVisible = ref(false);
const globalTagForm = reactive({ name: '', color: '#409EFF', category: '' });
const globalTagSubmitting = ref(false);

const openGlobalTagDialog = () => {
  globalTagForm.name = '';
  globalTagForm.color = '#409EFF';
  globalTagForm.category = '';
  createTagVisible.value = true;
};

const submitGlobalTag = async () => {
  if (!globalTagForm.name.trim()) {
    ElMessage.warning(t('crmTagsPanel.message.selectRequired'));
    return;
  }
  globalTagSubmitting.value = true;
  try {
    await createCrmTag({
      name: globalTagForm.name.trim(),
      color: globalTagForm.color,
      category: globalTagForm.category.trim() || 'general',
    });
    ElMessage.success(t('crmTagsPanel.message.addSuccess'));
    createTagVisible.value = false;
    fetchTags();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('crmTagsPanel.message.addFailed'));
  } finally {
    globalTagSubmitting.value = false;
  }
};

const removeGlobalTag = async (tag: CustomerTag) => {
  try {
    await ElMessageBox.confirm(`确认删除全局标签「${tag.name}」？`, '删除标签', {
      type: 'warning',
    });
    await deleteCrmTag(tag.id);
    ElMessage.success(t('crmTagsPanel.message.removeSuccess'));
    fetchTags();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('crmTagsPanel.message.removeFailed'));
    }
  }
};

onMounted(() => {
  fetchTags();
});
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
