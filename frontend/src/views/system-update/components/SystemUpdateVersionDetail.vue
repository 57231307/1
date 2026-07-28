<!--
  SystemUpdateVersionDetail.vue - 系统版本详情对话框
  拆分自 system-update/index.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('systemUpdate.versionDetail.dialogTitle')"
    width="700px"
    :aria-label="t('systemUpdate.versionDetail.dialogAriaLabel')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item :label="t('systemUpdate.versionDetail.labelVersion')">{{
        currentVersionDetail?.version
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('systemUpdate.versionDetail.labelReleaseDate')">{{
        currentVersionDetail?.release_date
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('systemUpdate.versionDetail.labelFileSize')" :span="2">{{
        formatFileSize(currentVersionDetail?.file_size || 0)
      }}</el-descriptions-item>
    </el-descriptions>
    <div class="detail-section">
      <h4>{{ t('systemUpdate.versionDetail.sectionReleaseNotes') }}</h4>
      <p>
        {{
          currentVersionDetail?.release_notes || t('systemUpdate.versionDetail.emptyReleaseNotes')
        }}
      </p>
    </div>
    <div class="detail-section">
      <h4>{{ t('systemUpdate.versionDetail.sectionFeatures') }}</h4>
      <ul>
        <li v-for="(feature, index) in currentVersionDetail?.features || []" :key="index">
          {{ feature }}
        </li>
        <li v-if="!currentVersionDetail?.features?.length">
          {{ t('systemUpdate.versionDetail.emptyList') }}
        </li>
      </ul>
    </div>
    <div class="detail-section">
      <h4>{{ t('systemUpdate.versionDetail.sectionBugFixes') }}</h4>
      <ul>
        <li v-for="(fix, index) in currentVersionDetail?.bug_fixes || []" :key="index">
          {{ fix }}
        </li>
        <li v-if="!currentVersionDetail?.bug_fixes?.length">
          {{ t('systemUpdate.versionDetail.emptyList') }}
        </li>
      </ul>
    </div>
    <div class="detail-section">
      <h4>{{ t('systemUpdate.versionDetail.sectionBreakingChanges') }}</h4>
      <ul>
        <li v-for="(change, index) in currentVersionDetail?.breaking_changes || []" :key="index">
          {{ change }}
        </li>
        <li v-if="!currentVersionDetail?.breaking_changes?.length">
          {{ t('systemUpdate.versionDetail.emptyList') }}
        </li>
      </ul>
    </div>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('systemUpdate.versionDetail.buttonClose')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { SystemVersion } from '@/api/system-update';
import { formatFileSize } from '../composables/sysUpdFmts';

const { t } = useI18n({ useScope: 'global' });

/**
 * 系统版本详情对话框组件
 */
const props = defineProps<{
  visible: boolean;
  currentVersionDetail: SystemVersion | null;
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
}>();

void props;
</script>

<style scoped>
.detail-section {
  margin-top: 16px;
}
.detail-section h4 {
  margin-bottom: 8px;
  color: #303133;
}
.detail-section ul {
  margin: 0;
  padding-left: 20px;
}
.detail-section li {
  margin-bottom: 4px;
  color: #606266;
}
</style>
