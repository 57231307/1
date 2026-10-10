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
        <el-table-column prop="webhook_url" label="URL" min-width="250" show-overflow-tooltip />
        <el-table-column prop="platform" :label="t('system.webhook.column.platform')" width="120" />
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
        <el-form-item label="URL" prop="webhook_url">
          <el-input v-model="webhookForm.webhook_url" placeholder="https://" />
        </el-form-item>
        <el-form-item :label="t('system.webhook.form.label.platform')" prop="platform">
          <el-select v-model="webhookForm.platform" style="width: 100%">
            <el-option :label="t('system.webhook.platform.GENERIC')" value="GENERIC" />
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
        <el-button type="primary" :loading="submitLoading" @click="saveWebhook">{{
          t('system.webhook.form.button.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { isDialogDismissal } from '@/utils/monitor';
import { reactive, ref, onMounted } from 'vue';
import { logger } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance } from 'element-plus';
import { request } from '@/api/request';
import type { ApiResponse } from '@/types/api';

const { t } = useI18n({ useScope: 'global' });

// 状态标签映射（响应式）
const getStatusLabel = (active: boolean): string =>
  active ? t('system.webhook.status.enabled') : t('system.webhook.status.disabled');

interface WebhookRow {
  id: number;
  name: string;
  platform: string;
  webhook_url: string;
  is_active: boolean;
  last_triggered_at: string | null;
  last_status: string | null;
  created_at: string;
}

// 编辑对话框表单状态：与后端 CreateWebhookIntegrationRequest /
// UpdateWebhookIntegrationRequest 的键逐一对齐（webhook_url + platform，无 event_type）
interface WebhookForm {
  id: number;
  name: string;
  platform: string;
  webhook_url: string;
  secret: string;
  is_active: boolean;
}

const webhookList = ref<WebhookRow[]>([]);
const webhookLoading = ref(false);
const submitLoading = ref(false);
const webhookDialogVisible = ref(false);
const webhookFormRef = ref<FormInstance>();
const webhookForm = reactive<WebhookForm>({
  id: 0,
  name: '',
  platform: 'GENERIC',
  webhook_url: '',
  secret: '',
  is_active: true,
});

const fetchWebhooks = async () => {
  webhookLoading.value = true;
  try {
    const res = await request.get<ApiResponse<WebhookRow[]>>('/webhooks/integrations');
    // list_integrations 出参为 ApiResponse<Vec<WebhookIntegrationItem>>，业务数组即 data 字段
    webhookList.value = res.data;
  } catch (_e) {
    logger.error(t('system.webhook.message.loadFailed'), _e);
    webhookList.value = [];
  } finally {
    webhookLoading.value = false;
  }
};

const openWebhookDialog = (row?: WebhookRow) => {
  if (row) {
    // 后端出参不回显 secret，编辑时留空表示不修改现有密钥
    webhookForm.id = row.id;
    webhookForm.name = row.name;
    webhookForm.platform = row.platform;
    webhookForm.webhook_url = row.webhook_url;
    webhookForm.secret = '';
    webhookForm.is_active = row.is_active;
  } else {
    webhookForm.id = 0;
    webhookForm.name = '';
    webhookForm.platform = 'GENERIC';
    webhookForm.webhook_url = '';
    webhookForm.secret = '';
    webhookForm.is_active = true;
  }
  webhookDialogVisible.value = true;
};

const saveWebhook = async () => {
  if (submitLoading.value) return;
  submitLoading.value = true;
  try {
    if (webhookForm.id) {
      // UpdateWebhookIntegrationRequest：platform 不参与更新；secret 仅在用户填写时提交，
      // 缺席即保持后端原值（后端出参不含 secret，避免每次编辑都清空密钥）
      const payload: {
        name: string;
        webhook_url: string;
        is_active: boolean;
        secret?: string;
      } = {
        name: webhookForm.name,
        webhook_url: webhookForm.webhook_url,
        is_active: webhookForm.is_active,
      };
      if (webhookForm.secret !== '') {
        payload.secret = webhookForm.secret;
      }
      await request.put(`/webhooks/integrations/${webhookForm.id}`, payload);
    } else {
      // CreateWebhookIntegrationRequest：platform + webhook_url 为非 Option 必填
      await request.post('/webhooks/integrations', {
        name: webhookForm.name,
        platform: webhookForm.platform,
        webhook_url: webhookForm.webhook_url,
        secret: webhookForm.secret === '' ? null : webhookForm.secret,
        is_active: webhookForm.is_active,
      });
    }
    ElMessage.success(t('system.webhook.message.saveSuccess'));
    webhookDialogVisible.value = false;
    fetchWebhooks();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('system.webhook.message.saveFailed'));
  } finally {
    submitLoading.value = false;
  }
};

const deleteWebhook = async (row: WebhookRow) => {
  try {
    await ElMessageBox.confirm(
      t('system.webhook.message.deleteConfirm'),
      t('system.webhook.message.deleteTitle'),
      { type: 'warning' }
    );
    await request.delete(`/webhooks/integrations/integration/${row.id}`);
    ElMessage.success(t('system.webhook.message.deleteSuccess'));
    fetchWebhooks();
  } catch (e) {
    if (!isDialogDismissal(e)) {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('system.webhook.message.deleteFailed'));
    }
  }
};

const testWebhook = async (row: WebhookRow) => {
  try {
    await request.post(`/webhooks/integrations/test-integration/${row.id}`);
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
