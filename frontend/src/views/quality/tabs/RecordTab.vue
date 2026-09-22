<!--
  RecordTab.vue - 检验记录 Tab（V2Table 迁移版）
  ----------------------------------------------------------------
  迁移说明（2026-06-16 P2-1 PR-5）：
  - 替换 el-table 为 V2Table 组件（基于 el-table-v2 的虚拟滚动通用组件）
  - 使用 useTableApi composable 接管分页/loading/重试
  - 保留原交互：page-header / 8 列表 / 结果 el-tag / 查看按钮 /
                    inject('qualityActions') openRecordDialog / openCreate /
                    handleExport (Batch 475d：改用后端 xlsx 导出) / handlePrint (新窗口) /
                    defineExpose({ fetchRecords }) / logger
  - 路径：/production/quality-inspection/records
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <div class="record-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('quality.recordTab.pageTitle') }}</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openCreate">
          <el-icon><Plus /></el-icon>
          {{ t('quality.recordTab.createButton') }}
        </el-button>
        <el-button v-permission="'quality.record.print'" @click="handlePrint">
          <el-icon><Printer /></el-icon>
          {{ t('quality.recordTab.printButton') }}
        </el-button>
        <el-button v-permission="'quality.record.export'" @click="handleExport">
          <el-icon><Download /></el-icon>
          {{ t('quality.recordTab.exportButton') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <V2Table
        :columns="columns"
        :data="data"
        :loading="loading"
        :page="page"
        :page-size="pageSize"
        :total="total"
        :height="600"
        @page-change="handlePageChange"
        @size-change="handleSizeChange"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
/**
 * 检验记录 Tab（V2Table 迁移版）
 * - V2Table：基于 el-table-v2 的虚拟滚动通用组件
 * - useTableApi：通用数据 composable（分页/loading/重试）
 * 保留原交互：page-header / 8 列表 / 结果 el-tag / 查看按钮 /
 *           inject('qualityActions') openRecordDialog / openCreate /
 *           handleExport (Batch 475d：改用后端 xlsx 导出) / handlePrint (新窗口) /
 *           defineExpose({ fetchRecords }) / logger
 */
import { h, onMounted, inject } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElTag, ElButton } from 'element-plus';
import { Plus, Download, Printer } from '@element-plus/icons-vue';
import { useTableApi } from '@/composables/useTableApi';
import V2Table from '@/components/V2Table/index.vue';
import type { ColumnDef } from '@/components/V2Table/types';
import { type QualityRecord } from '@/api/quality';
import { useQualityLookups } from '../composables/useQualityLookups';
import {
  QUALITY_INSPECTION_TYPE_LABEL_KEY,
  QUALITY_INSPECTION_TYPE_VALUES,
} from '@/constants/quality-inspection-type';
import {
  QUALITY_INSPECTION_SOURCE_LABEL_KEY,
  QUALITY_RECORD_RESULT_LABEL_KEY,
  QUALITY_RECORD_RESULT_TAG_TYPE,
  isQualityRecordResult,
  type QualityRecordResultValue,
} from '@/constants/quality-inspection-record';
import { logger } from '@/utils/logger';
import { escapeHtml } from '@/utils/print';
// V15 P0-S12 修复（Batch 475d）：导出改用后端带水印 xlsx 接口
// 后端 GET /production/quality-inspection/records/export 已就绪（含异步审计日志 + 水印）
import { exportFromBackend } from '@/utils/export';

const { t } = useI18n({ useScope: 'global' });

// 父组件注入：openRecordDialog(row | null)
const actions = inject<{
  openRecordDialog: (row: QualityRecord | null) => void;
}>('qualityActions');

// 检验记录列表（由 useTableApi 接管分页/loading/重试）
const { data, loading, page, pageSize, total, refresh } = useTableApi<QualityRecord>(
  '/production/quality-inspection/records'
);

// 记录表只存 id，名称按主数据翻（与父页面共用同一份缓存）
const { load: loadLookups, productName, inspectorName } = useQualityLookups();

// 检验类型：四个稳定码 + 委外回仓自动记录的来源标识，词表外原样展示
const inspectionTypeLabel = (value: string): string => {
  const typeKey = (QUALITY_INSPECTION_TYPE_VALUES as string[]).includes(value)
    ? QUALITY_INSPECTION_TYPE_LABEL_KEY[value as keyof typeof QUALITY_INSPECTION_TYPE_LABEL_KEY]
    : QUALITY_INSPECTION_SOURCE_LABEL_KEY[value];
  return typeKey ? t(typeKey) : value;
};

// 检验结论：取值词表见 constants/quality-inspection-record（库里落的是中文稳定值）
const resultLabel = (value: string): string => {
  if (isQualityRecordResult(value)) return t(QUALITY_RECORD_RESULT_LABEL_KEY[value]);
  logger.warn('质检记录存在词表外的检验结论值', { inspection_result: value });
  return value;
};

const resultTagType = (value: string): 'success' | 'danger' | 'warning' =>
  isQualityRecordResult(value)
    ? QUALITY_RECORD_RESULT_TAG_TYPE[value as QualityRecordResultValue]
    : 'warning';

/**
 * 列定义
 * - 列名与后端出参字段一致：单号是 inspection_no，产品与检验人按主数据翻名称
 * - 结果列：el-tag 三色映射（合格→success, 不合格→danger, 待检/越界→warning）
 * - 操作列：查看按钮（fixed right）
 */
const columns: ColumnDef<QualityRecord>[] = [
  { key: 'inspection_no', title: t('quality.recordTab.colRecordNo'), width: 140, fixed: 'left' },
  {
    key: 'inspection_type',
    title: t('quality.recordTab.colInspectionType'),
    width: 120,
    formatter: (row: QualityRecord) => inspectionTypeLabel(row.inspection_type),
  },
  {
    key: 'product_id',
    title: t('quality.recordTab.colProduct'),
    width: 150,
    formatter: (row: QualityRecord) => productName(row.product_id),
  },
  { key: 'batch_no', title: t('quality.recordTab.colBatchNo'), width: 140 },
  { key: 'inspection_date', title: t('quality.recordTab.colInspectionDate'), width: 120 },
  {
    key: 'inspector_id',
    title: t('quality.recordTab.colInspector'),
    width: 100,
    formatter: (row: QualityRecord) => inspectorName(row.inspector_id),
  },
  {
    key: 'inspection_result',
    title: t('quality.recordTab.colResult'),
    width: 100,
    align: 'center',
    renderCell: (row: QualityRecord) => {
      const type = resultTagType(row.inspection_result);
      const text = resultLabel(row.inspection_result);
      return h(ElTag, { type, size: 'small' }, () => text);
    },
  },
  {
    key: '__actions__',
    title: t('quality.recordTab.colActions'),
    width: 120,
    fixed: 'right',
    renderCell: (row: QualityRecord) =>
      h(
        ElButton,
        { type: 'primary', link: true, size: 'small', onClick: () => handleView(row) },
        () => t('quality.recordTab.buttonView')
      ),
  },
];

// 分页变化
const handlePageChange = (newPage: number) => {
  page.value = newPage;
};

const handleSizeChange = (newSize: number) => {
  pageSize.value = newSize;
};

// 打开新建对话框
const openCreate = () => {
  actions?.openRecordDialog(null);
};

// 查看检验记录（v11 批次 159 P1-1 修复：接入 openRecordDialog 显示详情，替代占位 ElMessage.info）
const handleView = (row: QualityRecord) => {
  actions?.openRecordDialog(row);
};

// 导出 Excel（V15 P0-S12 修复 Batch 475d）
// 规则 3：导出统一使用 xlsx 格式（禁止 CSV 作为最终交付格式）
// 改为调用后端 GET /production/quality-inspection/records/export，后端注入水印 + 异步审计日志
// 当前页面无筛选条件（前端未暴露 queryParams），导出全量数据
const handleExport = async () => {
  await exportFromBackend(
    '/production/quality-inspection/records/export',
    {},
    'quality_inspection_records_export'
  );
  logger.info(t('quality.recordTab.messageExported'));
};

// 构造打印表格行 HTML（与列表同口径：单号/类型文案/产品名称/检验人名称/结论文案）
const buildPrintRows = (): string => {
  return data.value
    .map(
      item => `
    <tr>
      <td>${escapeHtml(item.inspection_no)}</td><td>${escapeHtml(inspectionTypeLabel(item.inspection_type))}</td>
      <td>${escapeHtml(productName(item.product_id))}</td><td>${escapeHtml(item.batch_no ?? '')}</td>
      <td>${escapeHtml(item.inspection_date)}</td><td>${escapeHtml(inspectorName(item.inspector_id))}</td>
      <td>${escapeHtml(resultLabel(item.inspection_result))}</td>
    </tr>
  `
    )
    .join('');
};

// 打印
const handlePrint = () => {
  const printWindow = window.open('', '_blank');
  if (!printWindow) {
    ElMessage.error(t('quality.recordTab.messageCannotOpenPrintWindow'));
    return;
  }
  const rows = buildPrintRows();
  const printDate = new Date().toISOString().split('T')[0];
  const totalCount = data.value.length;
  printWindow.document
    .write(`<html><head><meta charset="utf-8"><title>${t('quality.recordTab.print.title')}</title>
    <style>@media print{@page{size:landscape;}}body{font-family:"Microsoft YaHei",sans-serif;font-size:12px;}h1{text-align:center;}table{width:100%;border-collapse:collapse;margin-top:12px;}th,td{border:1px solid #333;padding:6px 8px;}th{background:#f5f5f5;}.meta{text-align:center;color:#666;font-size:11px;}</style></head><body>
    <h1>${t('quality.recordTab.print.headerTitle')}</h1><div class="meta">${t('quality.recordTab.print.dateLabel')}: ${printDate} | ${t('quality.recordTab.print.totalLabel')} ${totalCount} ${t('quality.recordTab.print.totalUnit')}</div>
    <table><thead><tr><th>${t('quality.recordTab.print.colRecordNo')}</th><th>${t('quality.recordTab.print.colInspectionType')}</th><th>${t('quality.recordTab.print.colProduct')}</th><th>${t('quality.recordTab.print.colBatchNo')}</th><th>${t('quality.recordTab.print.colInspectionDate')}</th><th>${t('quality.recordTab.print.colInspector')}</th><th>${t('quality.recordTab.print.colResult')}</th></tr></thead><tbody>${rows}</tbody></table></body></html>`);
  printWindow.document.close();
  printWindow.onload = () => printWindow.print();
  logger.info(t('quality.recordTab.messagePrintGenerated'));
};

// 组件挂载时获取数据（主数据名称先取齐，否则产品/检验人两列只能显示 ID）
onMounted(() => {
  void loadLookups();
  refresh();
});

// 暴露给父组件调用（兼容外部刷新接口）
defineExpose({ fetchRecords: refresh });
</script>

<style scoped>
.record-tab {
  padding: 0;
}
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}
.page-title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}
.header-actions {
  display: flex;
  gap: 8px;
}
</style>
