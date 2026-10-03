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
              @click="openProcessDialog(row)"
              >{{ t('quality.defectTab.buttonProcess') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!--
      处理缺陷对话框：后端 ProcessUnqualifiedRequest 要求 unqualified_qty / unqualified_reason /
      handling_method 三个必填项（services/quality_inspection_service.rs:179-186），
      原先仅 prompt 收集 remark 的写法必然 422，这里改为真实采集全部必填项。
    -->
    <el-dialog
      v-model="processDialogVisible"
      :title="t('quality.defectTab.messageProcessTitle')"
      width="520px"
      :aria-label="t('quality.defectTab.dialogAriaLabel')"
    >
      <el-form label-width="100px">
        <el-form-item :label="t('quality.defectTab.dialogUnqualifiedQty')" required>
          <el-input-number
            v-model="processForm.unqualified_qty"
            :min="0"
            style="width: 100%"
            :aria-label="t('quality.defectTab.dialogUnqualifiedQty')"
          />
        </el-form-item>
        <el-form-item :label="t('quality.defectTab.dialogHandlingMethod')" required>
          <el-select
            v-model="processForm.handling_method"
            :placeholder="t('quality.defectTab.dialogHandlingMethod')"
            style="width: 100%"
          >
            <el-option
              :label="t('quality.defectTab.handlingDowngradeSale')"
              value="downgrade_sale"
            />
            <el-option :label="t('quality.defectTab.handlingRework')" value="rework" />
            <el-option :label="t('quality.defectTab.handlingScrap')" value="scrap" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('quality.defectTab.dialogUnqualifiedReason')" required>
          <el-input
            v-model="processForm.unqualified_reason"
            type="textarea"
            :rows="2"
            :placeholder="t('quality.defectTab.dialogUnqualifiedReason')"
          />
        </el-form-item>
        <el-form-item :label="t('quality.defectTab.dialogHandlingResult')">
          <el-input
            v-model="processForm.handling_result"
            :placeholder="t('quality.defectTab.dialogHandlingResult')"
          />
        </el-form-item>
        <el-form-item :label="t('quality.defectTab.dialogRemark')">
          <el-input v-model="processForm.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="processDialogVisible = false">{{
          t('quality.recordDialog.cancel')
        }}</el-button>
        <el-button type="primary" :loading="processing" @click="submitProcess">
          {{ t('quality.recordDialog.confirm') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import {
  processDefect as processDefectApi,
  type Defect,
  type DefectHandlingMethod,
} from '@/api/quality';
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

const processDialogVisible = ref(false);
const processing = ref(false);
const processTarget = ref<Defect | null>(null);
const processForm = reactive({
  unqualified_qty: undefined as number | undefined,
  handling_method: '' as DefectHandlingMethod | '',
  unqualified_reason: '',
  handling_result: '',
  remark: '',
});

const openProcessDialog = (row: Defect) => {
  processTarget.value = row;
  Object.assign(processForm, {
    unqualified_qty: undefined,
    handling_method: '',
    unqualified_reason: '',
    handling_result: '',
    remark: '',
  });
  processDialogVisible.value = true;
};

const submitProcess = async () => {
  const row = processTarget.value;
  if (!row) return;
  const { unqualified_qty, handling_method, unqualified_reason, handling_result, remark } =
    processForm;
  if (unqualified_qty === undefined || unqualified_qty < 0) {
    ElMessage.warning(t('quality.defectTab.ruleQtyRequired'));
    return;
  }
  if (!handling_method) {
    ElMessage.warning(t('quality.defectTab.ruleMethodRequired'));
    return;
  }
  if (!unqualified_reason.trim()) {
    ElMessage.warning(t('quality.defectTab.ruleReasonRequired'));
    return;
  }
  processing.value = true;
  try {
    // Option<String> 字段空值时省略键（Some("") 会被后端原样写库），参照 UserTab.vue:358-359 范式
    await processDefectApi(row.id, {
      unqualified_qty,
      unqualified_reason: unqualified_reason.trim(),
      handling_method,
      ...(handling_result.trim() ? { handling_result: handling_result.trim() } : {}),
      ...(remark.trim() ? { remark: remark.trim() } : {}),
    });
    ElMessage.success(t('quality.defectTab.messageProcessSuccess'));
    processDialogVisible.value = false;
    fetchDefects();
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('quality.defectTab.messageOperationFailed'));
  } finally {
    processing.value = false;
  }
};

onMounted(() => {
  fetchDefects();
});

defineExpose({ fetchDefects });
</script>
