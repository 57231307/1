<!--
  StandardTab.vue - 质量标准 Tab
  来源：原 quality/index.vue 中 质量标准 tab 内容
  拆分日期：2026-06-15 B3-4
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <div class="standard-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('quality.standardTab.pageTitle') }}</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openCreate">
          <el-icon><Plus /></el-icon>
          {{ t('quality.standardTab.createButton') }}
        </el-button>
        <el-button v-permission="'quality.standard.print'" @click="handlePrint">
          <el-icon><Printer /></el-icon>
          {{ t('quality.standardTab.printButton') }}
        </el-button>
        <el-button v-permission="'quality.standard.export'" @click="handleExport">
          <el-icon><Download /></el-icon>
          {{ t('quality.standardTab.exportButton') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :data="standards"
        stripe
        :aria-label="t('quality.standardTab.tableAriaLabel')"
      >
        <el-table-column
          prop="standard_code"
          :label="t('quality.standardTab.colStandardCode')"
          width="140"
        />
        <el-table-column
          prop="standard_name"
          :label="t('quality.standardTab.colStandardName')"
          width="180"
        />
        <el-table-column prop="type" :label="t('quality.standardTab.colType')" width="100">
          <template #default="{ row }">
            {{ getTypeLabel(row.type) }}
          </template>
        </el-table-column>
        <el-table-column prop="version" :label="t('quality.standardTab.colVersion')" width="80" />
        <el-table-column
          prop="status"
          :label="t('quality.standardTab.colStatus')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="created_by_name"
          :label="t('quality.standardTab.colCreatedBy')"
          width="100"
        />
        <el-table-column
          prop="approved_by_name"
          :label="t('quality.standardTab.colApprovedBy')"
          width="100"
        >
          <template #default="{ row }">
            {{ row.approved_by_name || '-' }}
          </template>
        </el-table-column>
        <el-table-column
          prop="approved_at"
          :label="t('quality.standardTab.colApprovedAt')"
          width="160"
        >
          <template #default="{ row }">
            {{ row.approved_at || '-' }}
          </template>
        </el-table-column>
        <el-table-column :label="t('quality.standardTab.colActions')" width="300" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">{{
              t('quality.standardTab.buttonView')
            }}</el-button>
            <el-button
              v-if="row.status !== 'draft'"
              type="primary"
              link
              size="small"
              @click="emit('openHistory', row)"
              >{{ t('quality.standardTab.buttonVersionHistory') }}</el-button
            >
            <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
            <el-button
              v-if="row.status === 'draft'"
              v-permission="'quality_standard:update'"
              type="primary"
              link
              size="small"
              @click="openEdit(row)"
              >{{ t('quality.standardTab.buttonEdit') }}</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="success"
              link
              size="small"
              @click="emit('openApprove', row)"
              >{{ t('quality.standardTab.buttonApprove') }}</el-button
            >
            <el-button
              v-if="row.status === 'approved'"
              type="warning"
              link
              size="small"
              @click="handlePublish(row)"
              >{{ t('quality.standardTab.buttonPublish') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, defineEmits, inject } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus, Download, Printer } from '@element-plus/icons-vue';
import { getQualityStandard, publishQualityStandard, type QualityStandard } from '@/api/quality';
import { logger } from '@/utils/logger';
import { escapeHtml } from '@/utils/print';
// V15 P0-S12 修复（Batch 475d）：导出改用后端带水印 xlsx 接口
// 后端 GET /quality-standards/export 已就绪（含异步审计日志 + 水印）
// 本 Tab 复用主质量标准页面的后端端点（getQualityStandardList 调用同一 /quality-standards 路由）
import { exportFromBackend } from '@/utils/export';

const { t } = useI18n({ useScope: 'global' });

const emit = defineEmits<{
  openApprove: [row: QualityStandard];
  openHistory: [row: QualityStandard];
}>();

const standards = ref<QualityStandard[]>([]);
const loading = ref(false);

const actions = inject<{
  openStandardDialog: (row: QualityStandard | null) => void;
}>('qualityActions');

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    draft: t('quality.standardTab.statusDraft'),
    approved: t('quality.standardTab.statusApproved'),
    published: t('quality.standardTab.statusPublished'),
    rejected: t('quality.standardTab.statusRejected'),
  };
  return map[status] || status;
};

const getTypeLabel = (type: string) => {
  if (type === 'product') return t('quality.standardTab.typeProduct');
  return t('quality.standardTab.typeProcess');
};

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    approved: 'warning',
    published: 'success',
    rejected: 'danger',
  };
  return map[status] || 'info';
};

const fetchStandards = async () => {
  loading.value = true;
  try {
    const { getQualityStandardList } = await import('@/api/quality');
    const res = await getQualityStandardList();
    standards.value = (res.data as QualityStandard[] | undefined) || [];
  } catch (error) {
    const err = error as Error;
    logger.error(t('quality.standardTab.messageFetchFailed'), err.message);
  } finally {
    loading.value = false;
  }
};

const openCreate = () => {
  actions?.openStandardDialog(null);
};

const openEdit = (row: QualityStandard) => {
  actions?.openStandardDialog(row);
};

const handleView = async (row: QualityStandard) => {
  try {
    const res = await getQualityStandard(row.id);
    actions?.openStandardDialog((res.data as QualityStandard | undefined) || null);
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('quality.standardTab.messageFetchDetailFailed'));
  }
};

const handlePublish = async (row: QualityStandard) => {
  try {
    await ElMessageBox.confirm(
      t('quality.standardTab.messagePublishConfirm'),
      t('quality.standardTab.messagePublishTitle'),
      {
        type: 'warning',
      }
    );
    await publishQualityStandard(row.id);
    ElMessage.success(t('quality.standardTab.messagePublishSuccess'));
    fetchStandards();
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error;
      ElMessage.error(err.message || t('quality.standardTab.messageOperationFailed'));
    }
  }
};

// 导出 Excel（V15 P0-S12 修复 Batch 475d）
// 规则 3：导出统一使用 xlsx 格式（禁止 CSV 作为最终交付格式）
// 改为调用后端 GET /quality-standards/export，后端注入水印 + 异步审计日志
// 本 Tab 无独立筛选条件，导出全量数据
const handleExport = async () => {
  await exportFromBackend('/quality-standards/export', {}, 'quality_standards_export');
  logger.info(t('quality.standardTab.messageExported'));
};

// 构造打印表格行 HTML
const buildPrintRows = (): string => {
  return standards.value
    .map(
      item => `
    <tr>
      <td>${escapeHtml(item.standard_code)}</td><td>${escapeHtml(item.standard_name)}</td>
      <td>${escapeHtml(getTypeLabel(item.type))}</td>
      <td>${escapeHtml(item.version)}</td><td>${escapeHtml(getStatusLabel(item.status))}</td>
      <td>${escapeHtml(item.created_by_name || '-')}</td>
    </tr>
  `
    )
    .join('');
};

const handlePrint = () => {
  const printWindow = window.open('', '_blank');
  if (!printWindow) {
    ElMessage.error(t('quality.standardTab.messageCannotOpenPrintWindow'));
    return;
  }
  const rows = buildPrintRows();
  const printDate = new Date().toISOString().split('T')[0];
  const totalCount = standards.value.length;
  printWindow.document
    .write(`<html><head><meta charset="utf-8"><title>${t('quality.standardTab.print.title')}</title>
    <style>@media print{@page{size:landscape;}}body{font-family:"Microsoft YaHei",sans-serif;font-size:12px;}h1{text-align:center;}table{width:100%;border-collapse:collapse;margin-top:12px;}th,td{border:1px solid #333;padding:6px 8px;}th{background:#f5f5f5;}.meta{text-align:center;color:#666;font-size:11px;}</style></head><body>
    <h1>${t('quality.standardTab.print.headerTitle')}</h1><div class="meta">${t('quality.standardTab.print.dateLabel')}: ${printDate} | ${t('quality.standardTab.print.totalLabel')} ${totalCount} ${t('quality.standardTab.print.totalUnit')}</div>
    <table><thead><tr><th>${t('quality.standardTab.print.colStandardCode')}</th><th>${t('quality.standardTab.print.colStandardName')}</th><th>${t('quality.standardTab.print.colType')}</th><th>${t('quality.standardTab.print.colVersion')}</th><th>${t('quality.standardTab.print.colStatus')}</th><th>${t('quality.standardTab.print.colCreatedBy')}</th></tr></thead><tbody>${rows}</tbody></table></body></html>`);
  printWindow.document.close();
  printWindow.onload = () => printWindow.print();
  logger.info(t('quality.standardTab.messagePrintGenerated'));
};

onMounted(() => {
  fetchStandards();
});

defineExpose({ fetchStandards });
</script>
