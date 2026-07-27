<!--
  FollowUpTab.vue - 客户跟进记录 Tab
  来源：原 crm/detail.vue 中 跟进记录 section
-->
<template>
  <el-card shadow="hover" class="section-card mt-20">
    <template #header>
      <div class="card-header">
        <span>{{ t('crmFollowUp.title') }}</span>
        <el-button type="primary" size="small" @click="handleAddFollowUp">
          <el-icon><Plus /></el-icon>
          {{ t('crmFollowUp.addFollowUp') }}
        </el-button>
      </div>
    </template>

    <el-timeline>
      <el-timeline-item
        v-for="record in followUps"
        :key="record.id"
        :timestamp="record.created_at"
        placement="top"
        :type="getFollowUpType(record.type)"
      >
        <el-card>
          <div class="follow-up-header">
            <span class="follow-up-type">{{ getFollowUpTypeLabel(record.type) }}</span>
            <span class="follow-up-operator">{{
              t('crmFollowUp.operatorLabel', { name: record.operator_name })
            }}</span>
          </div>
          <p class="follow-up-content">{{ record.content }}</p>
          <div v-if="record.next_follow_date" class="follow-up-next">
            <el-icon><Clock /></el-icon>
            {{ t('crmFollowUp.nextFollowUpLabel', { date: record.next_follow_date }) }}
          </div>
        </el-card>
      </el-timeline-item>
    </el-timeline>

    <div class="pagination-wrapper">
      <el-pagination
        v-model:current-page="query.page"
        v-model:page-size="query.page_size"
        :aria-label="t('crmFollowUp.paginationAriaLabel')"
        :page-sizes="[10, 20, 50]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="fetchFollowUps"
        @current-change="fetchFollowUps"
      />
    </div>

    <el-dialog
      v-model="dialogVisible"
      :aria-label="t('crmFollowUp.dialog.ariaLabel')"
      :title="t('crmFollowUp.dialog.title')"
      width="500px"
      :close-on-click-modal="false"
    >
      <el-form
        ref="formRef"
        :model="form"
        label-width="100px"
        :aria-label="t('crmFollowUp.dialog.formAriaLabel')"
      >
        <el-form-item :label="t('crmFollowUp.form.type')" prop="type">
          <el-select
            v-model="form.type"
            :placeholder="t('crmFollowUp.form.typePlaceholder')"
            style="width: 100%"
          >
            <el-option :label="t('crmFollowUp.followUpType.phone')" value="phone" />
            <el-option :label="t('crmFollowUp.followUpType.meeting')" value="meeting" />
            <el-option :label="t('crmFollowUp.followUpType.email')" value="email" />
            <el-option :label="t('crmFollowUp.followUpType.wechat')" value="wechat" />
            <el-option :label="t('crmFollowUp.followUpType.visit')" value="visit" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('crmFollowUp.form.content')" prop="content">
          <el-input
            v-model="form.content"
            type="textarea"
            :rows="4"
            :placeholder="t('crmFollowUp.form.contentPlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="t('crmFollowUp.form.nextFollowUp')">
          <el-date-picker
            v-model="form.next_follow_date"
            type="date"
            :placeholder="t('crmFollowUp.form.datePlaceholder')"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('crmFollowUp.form.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
          t('crmFollowUp.form.save')
        }}</el-button>
      </template>
    </el-dialog>
  </el-card>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance } from 'element-plus';
import { Plus, Clock } from '@element-plus/icons-vue';
// D14 Batch 5b：原 crmEnhancedApi 对象已转风格 B 函数
import { getFollowUpList, createFollowUp, type FollowUpRecord } from '@/api/crm-enhanced';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

// 接收父组件传入的客户 ID
interface Props {
  customerId: number;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  (e: 'updated'): void;
}>();

const followUps = ref<FollowUpRecord[]>([]);
const total = ref(0);
const dialogVisible = ref(false);
const submitLoading = ref(false);
const formRef = ref<FormInstance>();

const query = reactive({
  page: 1,
  page_size: 10,
});

const form = reactive({
  type: 'phone',
  content: '',
  next_follow_date: '',
});

const getFollowUpType = (type: string) => {
  const typeMap: Record<string, string> = {
    phone: 'primary',
    meeting: 'success',
    email: 'info',
    wechat: 'warning',
    visit: 'danger',
  };
  return typeMap[type] || '';
};

const getFollowUpTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    phone: t('crmFollowUp.followUpType.phone'),
    meeting: t('crmFollowUp.followUpType.meeting'),
    email: t('crmFollowUp.followUpType.email'),
    wechat: t('crmFollowUp.followUpType.wechat'),
    visit: t('crmFollowUp.followUpType.visit'),
  };
  return labels[type] || type;
};

const fetchFollowUps = async () => {
  try {
    const res = await getFollowUpList(props.customerId, query);
    followUps.value = res.data?.list || [];
    total.value = res.data?.total || 0;
  } catch (error) {
    const err = error as Error;
    logger.warn(t('crmFollowUp.message.loadFailed'), err.message);
    followUps.value = [];
    total.value = 0;
  }
};

const handleAddFollowUp = () => {
  form.type = 'phone';
  form.content = '';
  form.next_follow_date = '';
  dialogVisible.value = true;
};

const handleSubmit = async () => {
  if (!form.content.trim()) {
    ElMessage.warning(t('crmFollowUp.message.contentRequired'));
    return;
  }

  submitLoading.value = true;
  try {
    await createFollowUp(props.customerId, {
      type: form.type,
      content: form.content,
      next_follow_date: form.next_follow_date || undefined,
    });
    ElMessage.success(t('crmFollowUp.message.saveSuccess'));
    dialogVisible.value = false;
    fetchFollowUps();
    emit('updated');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('crmFollowUp.message.saveFailed'));
  } finally {
    submitLoading.value = false;
  }
};

// 暴露给父组件调用的方法
defineExpose({ fetchFollowUps });
</script>

<style scoped>
.section-card {
  margin-bottom: 0;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 600;
}
.mt-20 {
  margin-top: 20px;
}
.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}

.follow-up-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.follow-up-type {
  font-weight: 600;
  color: #303133;
}
.follow-up-operator {
  font-size: 12px;
  color: #909399;
}
.follow-up-content {
  color: #606266;
  margin: 0 0 8px 0;
  line-height: 1.6;
}
.follow-up-next {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: #409eff;
}
</style>
