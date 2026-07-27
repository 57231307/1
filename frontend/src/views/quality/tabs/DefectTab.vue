<!--
  DefectTab.vue - 缺陷管理 Tab
  来源：原 quality/index.vue 中 缺陷管理 tab 内容
  拆分日期：2026-06-15 B3-4
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <div class="defect-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('quality.defectTab.pageTitle') }}</h2>
    </div>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :data="defects"
        stripe
        :aria-label="t('quality.defectTab.tableAriaLabel')"
      >
        <el-table-column
          prop="defect_type"
          :label="t('quality.defectTab.colDefectType')"
          width="140"
        />
        <el-table-column
          prop="defect_description"
          :label="t('quality.defectTab.colDefectDescription')"
          min-width="200"
        />
        <el-table-column
          prop="severity"
          :label="t('quality.defectTab.colSeverity')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getSeverityType(row.severity)" size="small">
              {{ getSeverityLabel(row.severity) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="quantity"
          :label="t('quality.defectTab.colQuantity')"
          width="80"
          align="right"
        />
        <el-table-column
          prop="processed"
          :label="t('quality.defectTab.colProcessed')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.processed ? 'success' : 'info'" size="small">
              {{
                row.processed
                  ? t('quality.defectTab.processedYes')
                  : t('quality.defectTab.processedNo')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('quality.defectTab.colActions')" width="120" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="!row.processed"
              type="primary"
              link
              size="small"
              @click="processDefect(row)"
              >{{ t('quality.defectTab.buttonProcess') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { processDefect as processDefectApi, type Defect } from '@/api/quality';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

const defects = ref<Defect[]>([]);
const loading = ref(false);

// 严重程度标签映射函数
const getSeverityLabel = (severity: string): string => {
  const map: Record<string, string> = {
    critical: t('quality.defectTab.severityCritical'),
    major: t('quality.defectTab.severityMajor'),
    minor: t('quality.defectTab.severityMinor'),
  };
  return map[severity] || severity;
};

// 严重程度颜色映射
const getSeverityType = (severity: string): 'danger' | 'warning' | 'info' => {
  if (severity === 'critical') return 'danger';
  if (severity === 'major') return 'warning';
  return 'info';
};

const fetchDefects = async () => {
  loading.value = true;
  try {
    const { getDefectList } = await import('@/api/quality');
    const res = await getDefectList();
    defects.value = (res.data as Defect[] | undefined) || [];
  } catch (error) {
    const err = error as Error;
    logger.error(t('quality.defectTab.messageFetchFailed'), err.message);
  } finally {
    loading.value = false;
  }
};

const processDefect = async (row: Defect) => {
  try {
    const { value } = await ElMessageBox.prompt(
      t('quality.defectTab.messageProcessPrompt'),
      t('quality.defectTab.messageProcessTitle')
    );
    await processDefectApi(row.id, { remark: value });
    ElMessage.success(t('quality.defectTab.messageProcessSuccess'));
    fetchDefects();
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error;
      ElMessage.error(err.message || t('quality.defectTab.messageOperationFailed'));
    }
  }
};

onMounted(() => {
  fetchDefects();
});

defineExpose({ fetchDefects });
</script>
