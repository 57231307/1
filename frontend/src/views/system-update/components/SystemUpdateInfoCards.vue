<!--
  SystemUpdateInfoCards.vue - 系统更新顶部信息卡
  拆分自 system-update/index.vue（P14 批 2 I-3 第 1 批）
  最新版本/更新状态改为消费后端 check 权威响应（CheckUpdateResult），
  并在有更新时展示 release_notes / file_size / published_at。
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
            {{ t('systemUpdate.infoCards.labelBuildDate') }}:
            {{ currentVersion?.build_date || '-' }}
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
          <el-tag :type="hasUpdate ? 'warning' : 'success'" size="large">
            {{
              hasUpdate
                ? t('systemUpdate.infoCards.statusHasUpdate')
                : t('systemUpdate.infoCards.statusUpToDate')
            }}
          </el-tag>
        </div>
      </el-card>
    </el-col>
  </el-row>

  <!-- 更新内容：仅在后端权威 has_update 为 true 且已拿到 check 结果时展示 -->
  <el-row v-if="hasUpdate && latestVersion" class="release-notes-row">
    <el-col :span="24">
      <el-card shadow="hover">
        <template #header>
          <span>{{ t('systemUpdate.infoCards.headerReleaseNotes') }}</span>
        </template>
        <div v-if="latestVersion.file_size !== null" class="file-size">
          {{ t('systemUpdate.infoCards.labelFileSize') }}:
          {{ formatFileSize(latestVersion.file_size) }}
        </div>
        <!-- release_notes 为 GitHub release 正文（Markdown 源文本）。
             依赖树无 Markdown 渲染器，且红线禁止擅自装库，故以 <pre> 保留换行、
             {{ }} 插值由 Vue 自动转义（无 v-html）呈现真实更新内容，样式次之。 -->
        <pre v-if="latestVersion.release_notes" class="release-notes">{{
          latestVersion.release_notes
        }}</pre>
        <!-- 有更新但后端未带 release_notes：显式空态，不静默、不兜底伪造 -->
        <div v-else class="release-notes-empty">
          {{ t('systemUpdate.infoCards.noReleaseNotes') }}
        </div>
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
 * 显示当前版本、最新版本（含发布时间）、更新状态；有更新时展示更新内容与文件大小。
 */
const props = defineProps<{
  // 当前版本（GET /system-update/current-version）
  currentVersion: { version: string; build_date: string } | null;
  // 最新检查结果（GET /system-update/check）
  latestVersion: CheckUpdateResult | null;
  // 是否有可用更新（后端权威 has_update）
  hasUpdate: boolean;
  // 文件大小格式化（复用 sysUpdFmts.formatFileSize）
  formatFileSize: (bytes: number) => string;
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
