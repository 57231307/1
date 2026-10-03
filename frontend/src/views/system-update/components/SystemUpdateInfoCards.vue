<!--
  SystemUpdateInfoCards.vue - 系统更新顶部信息卡
  拆分自 system-update/index.vue（P14 批 2 I-3 第 1 批）
  最新版本/更新状态消费后端 check 权威响应（CheckUpdateResult）。
  有更新时并列展示「当前版本更新内容」与「新版本更新内容」两份正文；无更新时仅展示当前版本那份。
  更新触发为后端同步单请求，故用不确定态 indeterminate 进度 + is_updating 轮询，禁伪造百分比。
-->
<template>
  <el-row :gutter="20" class="info-cards">
    <el-col :span="8">
      <el-card shadow="hover">
        <template #header>
          <span>{{ t('systemUpdate.infoCards.headerCurrentVersion') }}</span>
        </template>
        <div class="info-content">
          <div class="version">{{ currentVersion?.version || '-' }}</div>
          <div class="date">
            {{ t('systemUpdate.infoCards.labelReleaseDate') }}:
            {{ currentVersion?.release_date || '-' }}
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="8">
      <el-card shadow="hover">
        <template #header>
          <span>{{ t('systemUpdate.infoCards.headerLatestVersion') }}</span>
        </template>
        <div class="info-content">
          <div class="version">{{ latestVersion?.latest_version || '-' }}</div>
          <div class="date">
            {{ t('systemUpdate.infoCards.labelReleaseDate') }}:
            {{ latestVersion?.published_at || '-' }}
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="8">
      <el-card shadow="hover">
        <template #header>
          <span>{{ t('systemUpdate.infoCards.headerUpdateStatus') }}</span>
        </template>
        <div class="info-content">
          <!-- 不确定态：仅显 indeterminate 进度条（无百分比）+ 阶段文案，绝不显 % -->
          <template v-if="isUpdating">
            <el-progress
              :indeterminate="true"
              :duration="6"
              :show-text="false"
              striped
              striped-flow
            />
            <div class="updating-label">{{ t('systemUpdate.infoCards.statusUpdating') }}</div>
          </template>
          <template v-else>
            <el-tag :type="hasUpdate ? 'warning' : 'success'" size="large">
              {{
                hasUpdate
                  ? t('systemUpdate.infoCards.statusHasUpdate')
                  : t('systemUpdate.infoCards.statusUpToDate')
              }}
            </el-tag>
            <div v-if="hasUpdate" class="apply-row">
              <el-button type="primary" @click="emit('trigger-update')">
                {{ t('systemUpdate.infoCards.buttonApply') }}
              </el-button>
            </div>
          </template>
        </div>
      </el-card>
    </el-col>
  </el-row>

  <!-- 更新内容：有更新时并列两份，无更新时仅显当前版本一份（均取后端真实字段，无则显空态） -->
  <el-row v-if="latestVersion" class="release-notes-row">
    <el-col v-if="hasUpdate" :span="12">
      <el-card shadow="hover">
        <template #header>
          <span>{{ t('systemUpdate.infoCards.headerCurrentReleaseNotes') }}</span>
        </template>
        <!-- 纯文本：<pre> 保留换行，Vue 插值自动转义（零 v-html、零 XSS）。
             后端 current_release_notes 为 null 时显诚实空态，不拿另一份冒充。 -->
        <pre v-if="latestVersion.current_release_notes" class="release-notes">{{
          latestVersion.current_release_notes
        }}</pre>
        <div v-else class="release-notes-empty">
          {{ t('systemUpdate.infoCards.noReleaseNotesRecord') }}
        </div>
      </el-card>
    </el-col>
    <el-col :span="hasUpdate ? 12 : 24">
      <el-card shadow="hover">
        <template #header>
          <span>{{
            hasUpdate
              ? t('systemUpdate.infoCards.headerNewReleaseNotes')
              : t('systemUpdate.infoCards.headerCurrentReleaseNotes')
          }}</span>
        </template>
        <div v-if="hasUpdate && latestVersion.file_size !== null" class="file-size">
          {{ t('systemUpdate.infoCards.labelFileSize') }}:
          {{ formatFileSize(latestVersion.file_size) }}
        </div>
        <template v-if="hasUpdate">
          <pre v-if="latestVersion.release_notes" class="release-notes">{{
            latestVersion.release_notes
          }}</pre>
          <div v-else class="release-notes-empty">
            {{ t('systemUpdate.infoCards.noReleaseNotesRecord') }}
          </div>
        </template>
        <!-- 无更新：第二张卡承载「当前版本更新内容」（同 current_release_notes，不重复第一份来源） -->
        <template v-else>
          <pre v-if="latestVersion.current_release_notes" class="release-notes">{{
            latestVersion.current_release_notes
          }}</pre>
          <div v-else class="release-notes-empty">
            {{ t('systemUpdate.infoCards.noReleaseNotesRecord') }}
          </div>
        </template>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { CheckUpdateResult } from '@/api/system-update';

const { t } = useI18n({ useScope: 'global' });

/**
 * 系统更新顶部信息卡组件
 * 显示当前版本、最新版本（含发布时间）、更新状态；
 * 有更新时并列展示当前/新版本两份更新内容，并给出触发更新入口（不确定态）。
 */
const props = defineProps<{
  // 当前版本（GET /system-update/current-version）
  currentVersion: { version: string; release_date: string } | null;
  // 最新检查结果（GET /system-update/check，含两份 release_notes）
  latestVersion: CheckUpdateResult | null;
  // 是否有可用更新（后端权威 has_update）
  hasUpdate: boolean;
  // 是否正在应用更新（不确定态，来自 useSysUpd.isUpdating）
  isUpdating: boolean;
  // 文件大小格式化（复用 sysUpdFmts.formatFileSize）
  formatFileSize: (bytes: number) => string;
}>();

const emit = defineEmits<{
  'trigger-update': [];
}>();

void props;
</script>

<style scoped>
.info-cards {
  margin-bottom: 20px;
}
.info-content {
  text-align: center;
  padding: 10px 0;
}
.version {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 8px;
}
.date {
  font-size: 14px;
  color: #909399;
}
.updating-label {
  margin-top: 8px;
  font-size: 14px;
  color: #e6a23c;
}
.apply-row {
  margin-top: 12px;
}
.release-notes-row {
  margin-bottom: 20px;
}
.file-size {
  font-size: 14px;
  color: #909399;
  margin-bottom: 8px;
}
.release-notes {
  margin: 0;
  font: inherit;
  white-space: pre-wrap;
  word-break: break-word;
  text-align: left;
  line-height: 1.6;
  max-height: 420px;
  overflow-y: auto;
  padding: 12px 16px;
  background-color: #f5f7fa;
  border-radius: 4px;
}
.release-notes-empty {
  text-align: center;
  color: #909399;
  font-size: 14px;
  padding: 12px 0;
}
</style>
