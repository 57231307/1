<!--
  WebhookTab.vue - Webhook 配置 Tab
  来源：原 system/index.vue 中 Webhook tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="webhook-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.webhook.title') }}</h2>
      <el-button type="primary" @click="openWebhookDialog()">
        <el-icon><Plus /></el-icon> {{ t('system.webhook.button.create') }}
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table
        v-loading="webhookLoading"
        :data="webhookList"
        stripe
        :aria-label="t('system.webhook.aria.list')"
      >
        <el-table-column prop="name" :label="t('system.webhook.column.name')" width="150" />
        <el-table-column prop="url" label="URL" min-width="250" show-overflow-tooltip />
        <el-table-column prop="event_type" :label="t('system.webhook.column.event')" width="120" />
        <el-table-column
          prop="is_active"
          :label="t('system.webhook.column.status')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.is_active ? 'success' : 'info'" size="small">
              {{ getStatusLabel(row.is_active) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('system.webhook.column.action')" width="200" fixed="right">
          <template #default="{ row }">
            <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
            <el-button
              v-permission="'webhook:update'"
              size="small"
              link
              @click="openWebhookDialog(row as unknown as WebhookRow)"
              >{{ t('system.webhook.button.edit') }}</el-button
            >
            <el-button
              size="small"
              link
              type="warning"
              @click="testWebhook(row as unknown as WebhookRow)"
              >{{ t('system.webhook.button.test') }}</el-button
            >
            <el-button
              v-permission="'webhook:delete'"
              size="small"
              link
              type="danger"
              @click="deleteWebhook(row as unknown as WebhookRow)"
              >{{ t('system.webhook.button.delete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="webhookDialogVisible"
      :title="
        webhookForm.id
          ? t('system.webhook.dialog.editTitle')
          : t('system.webhook.dialog.createTitle')
      "
      width="500px"
      :aria-label="t('system.webhook.dialog.aria')"
    >
      <el-form
        ref="webhookFormRef"
        :model="webhookForm"
        label-width="100px"
        :aria-label="t('system.webhook.form.aria')"
      >
        <el-form-item :label="t('system.webhook.form.label.name')" prop="name">
          <el-input v-model="webhookForm.name" />
        </el-form-item>
        <el-form-item label="URL" prop="url">
          <el-input v-model="webhookForm.url" placeholder="https://" />
        </el-form-item>
        <el-form-item :label="t('system.webhook.form.label.eventType')">
          <el-select v-model="webhookForm.event_type" style="width: 100%">
            <el-option :label="t('system.webhook.event.orderCreated')" value="order.created" />
            <el-option :label="t('system.webhook.event.orderUpdated')" value="order.updated" />
            <el-option
              :label="t('system.webhook.event.inventoryChanged')"
              value="inventory.changed"
            />
            <el-option
              :label="t('system.webhook.event.approvalCompleted')"
              value="approval.completed"
            />
            <el-option :label="t('system.webhook.event.all')" value="all" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('system.webhook.form.label.secret')">
          <el-input
            v-model="webhookForm.secret"
            :placeholder="t('system.webhook.form.placeholder.secret')"
          />
        </el-form-item>
        <el-form-item :label="t('system.webhook.form.label.status')">
          <el-switch v-model="webhookForm.is_active" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="webhookDialogVisible = false">{{
          t('system.webhook.form.button.cancel')
        }}</el-button>
        <el-button type="primary" @click="saveWebhook">{{
          t('system.webhook.form.button.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance } from 'element-plus';
import { request } from '@/api/request';

const { t } = useI18n({ useScope: 'global' });

// 状态标签映射（响应式）
const getStatusLabel = (active: boolean): string =>
  active ? t('system.webhook.status.enabled') : t('system.webhook.status.disabled');

interface WebhookRow {
  id: number;
  name: string;
  url: string;
  event_type: string;
  is_active: boolean;
  secret?: string;
}

const webhookList = ref<WebhookRow[]>([]);
const webhookLoading = ref(false);
const webhookDialogVisible = ref(false);
const webhookFormRef = ref<FormInstance>();
const webhookForm = reactive<WebhookRow>({
  id: 0,
  name: '',
  url: '',
  event_type: 'all',
  secret: '',
  is_active: true,
});

const fetchWebhooks = async () => {
  webhookLoading.value = true;
  try {
    const res = await request.get<{ items?: WebhookRow[] } | WebhookRow[]>(
      '/webhooks/integrations'
    );
    const d = res;
    if (d && typeof d === 'object' && 'items' in d) {
      webhookList.value = d.items || [];
    } else {
      webhookList.value = (d as WebhookRow[]) || [];
    }
  } catch (_e) {
    webhookList.value = [];
  } finally {
    webhookLoading.value = false;
  }
};

const openWebhookDialog = (row?: WebhookRow) => {
  if (row) {
    Object.assign(webhookForm, row);
  } else {
    Object.assign(webhookForm, {
      id: 0,
      name: '',
      url: '',
      event_type: 'all',
      secret: '',
      is_active: true,
    });
  }
  webhookDialogVisible.value = true;
};

const saveWebhook = async () => {
  try {
    if (webhookForm.id) {
      await request.put(`/webhooks/integrations/${webhookForm.id}`, webhookForm);
    } else {
      await request.post('/webhooks/integrations', webhookForm);
    }
    ElMessage.success(t('system.webhook.message.saveSuccess'));
    webhookDialogVisible.value = false;
    fetchWebhooks();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('system.webhook.message.saveFailed'));
  }
};

const deleteWebhook = async (row: WebhookRow) => {
  try {
    await ElMessageBox.confirm(
      t('system.webhook.message.deleteConfirm'),
      t('system.webhook.message.deleteTitle'),
      { type: 'warning' }
    );
    await request.delete(`/webhooks/integrations/${row.id}`);
    ElMessage.success(t('system.webhook.message.deleteSuccess'));
    fetchWebhooks();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('system.webhook.message.deleteFailed'));
    }
  }
};

const testWebhook = async (row: WebhookRow) => {
  try {
    await request.post(`/webhooks/integrations/${row.id}`);
    ElMessage.success(t('system.webhook.message.testSent'));
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('system.webhook.message.testFailed'));
  }
};

defineExpose({ refresh: fetchWebhooks });

onMounted(() => {
  fetchWebhooks();
});
</script>
