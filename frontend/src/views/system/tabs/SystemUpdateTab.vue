<!--
  SystemUpdateTab.vue - 系统更新 Tab
  来源：原 system/index.vue 中 系统更新 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="system-update-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.systemUpdate.title') }}</h2>
    </div>
    <el-card shadow="hover">
      <el-descriptions :column="2" border>
        <el-descriptions-item :label="t('system.systemUpdate.label.currentVersion')">{{ systemVersion }}</el-descriptions-item>
        <el-descriptions-item :label="t('system.systemUpdate.label.lastUpdate')">{{ lastUpdate }}</el-descriptions-item>
      </el-descriptions>
      <div style="margin-top: 20px">
        <el-button type="primary" :loading="checkUpdateLoading" @click="checkUpdate">
          {{ t('system.systemUpdate.button.check') }}
        </el-button>
        <el-button
          v-if="hasUpdate"
          type="success"
          :loading="applyUpdateLoading"
          @click="applyUpdate"
        >
          {{ t('system.systemUpdate.button.apply') }}
        </el-button>
      </div>
      <el-alert
        v-if="updateInfo"
        :title="updateInfo"
        type="info"
        show-icon
        style="margin-top: 16px"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { request } from '@/api/request'

const { t } = useI18n({ useScope: 'global' })

interface VersionInfo {
  version: string
  updated_at: string
  message?: string
  has_update?: boolean
}

const systemVersion = ref('v2026.x.x')
const lastUpdate = ref('-')
const hasUpdate = ref(false)
const updateInfo = ref('')
const checkUpdateLoading = ref(false)
const applyUpdateLoading = ref(false)

const checkUpdate = async () => {
  checkUpdateLoading.value = true
  try {
    const res = await request.get<VersionInfo>('/system-update/check')
    const info = res
    updateInfo.value = info?.message || t('system.systemUpdate.message.upToDate')
    hasUpdate.value = info?.has_update || false
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('system.systemUpdate.message.checkFailed'))
  } finally {
    checkUpdateLoading.value = false
  }
}

const applyUpdate = async () => {
  applyUpdateLoading.value = true
  try {
    await request.post('/system-update/update')
    ElMessage.success(t('system.systemUpdate.message.updateSubmitted'))
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('system.systemUpdate.message.updateFailed'))
  } finally {
    applyUpdateLoading.value = false
  }
}

const fetchSystemVersion = async () => {
  try {
    const res = await request.get<VersionInfo>('/system-update/version')
    const info = res
    systemVersion.value = info?.version || 'unknown'
    lastUpdate.value = info?.updated_at || '-'
  } catch (_e) {
    // 静默：版本信息拉取失败时保留默认值
  }
}

defineExpose({ refresh: fetchSystemVersion })

onMounted(() => {
  fetchSystemVersion()
})
</script>
