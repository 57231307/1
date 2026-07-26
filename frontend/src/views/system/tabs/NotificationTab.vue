<!--
  NotificationTab.vue - 通知设置 Tab
  来源：原 system/index.vue 中 通知设置 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="notification-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.notification.title') }}</h2>
    </div>
    <el-card shadow="hover" style="max-width: 600px">
      <el-form :model="notificationForm" label-width="140px" :aria-label="t('system.notification.aria.form')">
        <el-form-item :label="t('system.notification.label.email')">
          <el-switch v-model="notificationForm.email_enabled" />
        </el-form-item>
        <el-form-item :label="t('system.notification.label.internal')">
          <el-switch v-model="notificationForm.internal_enabled" />
        </el-form-item>
        <el-divider content-position="left">{{ t('system.notification.divider.type') }}</el-divider>
        <el-form-item :label="t('system.notification.label.order')">
          <el-select v-model="notificationForm.order_notification_type" style="width: 100%">
            <el-option :label="t('system.notification.option.emailOnly')" value="email" />
            <el-option :label="t('system.notification.option.internalOnly')" value="internal" />
            <el-option :label="t('system.notification.option.all')" value="both" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('system.notification.label.approval')">
          <el-select v-model="notificationForm.approval_notification_type" style="width: 100%">
            <el-option :label="t('system.notification.option.emailOnly')" value="email" />
            <el-option :label="t('system.notification.option.internalOnly')" value="internal" />
            <el-option :label="t('system.notification.option.all')" value="both" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('system.notification.label.inventory')">
          <el-select v-model="notificationForm.inventory_notification_type" style="width: 100%">
            <el-option :label="t('system.notification.option.emailOnly')" value="email" />
            <el-option :label="t('system.notification.option.internalOnly')" value="internal" />
            <el-option :label="t('system.notification.option.all')" value="both" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('system.notification.label.system')">
          <el-select v-model="notificationForm.system_notification_type" style="width: 100%">
            <el-option :label="t('system.notification.option.emailOnly')" value="email" />
            <el-option :label="t('system.notification.option.internalOnly')" value="internal" />
            <el-option :label="t('system.notification.option.all')" value="both" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="notifSaving" @click="saveNotificationSetting">{{ t('system.notification.button.save') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { request } from '@/api/request'

const { t } = useI18n({ useScope: 'global' })

type NotificationChannel = 'email' | 'internal' | 'both'

interface NotificationForm {
  email_enabled: boolean
  internal_enabled: boolean
  order_notification_type: NotificationChannel
  approval_notification_type: NotificationChannel
  inventory_notification_type: NotificationChannel
  system_notification_type: NotificationChannel
  purchase_notification_type: NotificationChannel
  finance_notification_type: NotificationChannel
}

const notificationForm = reactive<NotificationForm>({
  email_enabled: true,
  internal_enabled: true,
  order_notification_type: 'both',
  approval_notification_type: 'both',
  inventory_notification_type: 'both',
  system_notification_type: 'internal',
  purchase_notification_type: 'both',
  finance_notification_type: 'both',
})

const notifSaving = ref(false)

const fetchNotificationSetting = async () => {
  try {
    const res = await request.get<Partial<NotificationForm>>('/user/notification-setting')
    if (res) {
      Object.assign(notificationForm, res)
    }
  } catch (_e) {
    // 静默：通知设置接口失败时不影响界面默认显示
  }
}

const saveNotificationSetting = async () => {
  notifSaving.value = true
  try {
    await request.put('/user/notification-setting', notificationForm)
    ElMessage.success(t('system.notification.message.saveSuccess'))
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('system.notification.message.saveFailed'))
  } finally {
    notifSaving.value = false
  }
}

defineExpose({ refresh: fetchNotificationSetting })

onMounted(() => {
  fetchNotificationSetting()
})
</script>
