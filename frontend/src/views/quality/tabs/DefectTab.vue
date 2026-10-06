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
          prop="unqualified_no"
          :label="t('quality.recordTab.colRecordNo')"
          width="140"
        />
        <el-table-column prop="batch_no" :label="t('quality.recordTab.colBatchNo')" width="120" />
        <el-table-column
          prop="unqualified_qty"
          :label="t('quality.defectTab.colQuantity')"
          width="100"
          align="right"
        />
        <el-table-column
          prop="unqualified_reason"
          :label="t('quality.defectTab.dialogUnqualifiedReason')"
          min-width="180"
          show-overflow-tooltip
        />
        <el-table-column prop="grade" :label="t('inventory.stockTab.colGrade')" width="90">
          <template #default="{ row }">
            <el-tag v-if="row.grade" :type="gradeTagType(row.grade)" size="small">
              {{ gradeLabel(row.grade) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="handling_method"
          :label="t('quality.defectTab.dialogHandlingMethod')"
          width="110"
        >
          <template #default="{ row }">{{ handlingMethodLabel(row.handling_method) }}</template>
        </el-table-column>
        <el-table-column
          prop="handling_status"
          :label="t('quality.defectTab.colProcessed')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="handlingStatusTagType(row.handling_status)" size="small">
              {{ handlingStatusLabel(row.handling_status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('quality.defectTab.colActions')" width="120" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="canProcess(row)"
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
      处置结果对话框（D1②）：本对话框提交的是**台账行原地更新**，契约 = 后端
      ProcessResultRequest 两键（handling_method 必填 + reason 可选，
      services/quality_inspection_service.rs:197-206），路径 id 用行自身 id。
      开单字段（unqualified_qty / unqualified_reason / handling_result / remark）属
      ProcessUnqualifiedRequest 的质检记录开单契约，不再在此采集。
      选项排除 scrap 与端点状态门一致：处置结果端点对 scrap 恒 BUSINESS 拒绝
      （报废终态只能经财务/总经理两级审批端点达成，service :614-618）。
    -->
    <el-dialog
      v-model="processDialogVisible"
      :title="t('quality.defectTab.messageProcessTitle')"
      width="520px"
      :aria-label="t('quality.defectTab.dialogAriaLabel')"
    >
      <el-form label-width="100px">
        <el-form-item :label="t('quality.defectTab.dialogHandlingMethod')" required>
          <el-select
            v-model="processForm.handling_method"
            :placeholder="t('quality.defectTab.dialogHandlingMethod')"
            style="width: 100%"
          >
            <el-option
              v-for="item in processMethodOptions"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('quality.defectTab.dialogReason')">
          <el-input
            v-model="processForm.reason"
            type="textarea"
            :rows="2"
            :placeholder="t('quality.defectTab.dialogReason')"
          />
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
import { computed, ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import {
  processDefectRow,
  type UnqualifiedProductRecord,
  type DefectHandlingMethod,
} from '@/api/quality';
import {
  QUALITY_HANDLING_METHOD,
  QUALITY_HANDLING_METHOD_VALUES,
  QUALITY_HANDLING_METHOD_LABEL_KEY,
  QUALITY_HANDLING_STATUS,
  QUALITY_HANDLING_STATUS_LABEL_KEY,
  QUALITY_HANDLING_STATUS_TAG_TYPE,
  QUALITY_UNQUALIFIED_GRADE_LABEL_KEY,
  QUALITY_UNQUALIFIED_GRADE_TAG_TYPE,
  isQualityHandlingMethod,
  isQualityHandlingStatus,
  isQualityUnqualifiedGrade,
} from '@/constants/quality-unqualified-handling';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

const defects = ref<UnqualifiedProductRecord[]>([]);
const loading = ref(false);

// 处理方式选项与列表文案共用同一份词表（constants 是唯一真相源，取值即落库值）
const handlingMethodOptions = computed(() =>
  QUALITY_HANDLING_METHOD_VALUES.map(value => ({
    value,
    label: t(QUALITY_HANDLING_METHOD_LABEL_KEY[value]),
  }))
);

/**
 * 处置结果对话框可选方式：词表同源、仅剔除本端点语义上拒绝的 scrap
 * （报废终态走两级审批端点，处置结果端点直提交必 BUSINESS 拒绝）。
 */
const processMethodOptions = computed(() =>
  handlingMethodOptions.value.filter(item => item.value !== QUALITY_HANDLING_METHOD.scrap)
);

const handlingMethodLabel = (method: string): string =>
  isQualityHandlingMethod(method) ? t(QUALITY_HANDLING_METHOD_LABEL_KEY[method]) : method;

/**
 * 等级文案沿用通用质检严重度的插值键（实参为等级码 A/B/C）。
 * 该列与库存等级（一等品/二等品/等外品）是两套取值域，不可互换。
 */
const gradeLabel = (grade: string): string =>
  isQualityUnqualifiedGrade(grade) ? t(QUALITY_UNQUALIFIED_GRADE_LABEL_KEY, { n: grade }) : grade;

const gradeTagType = (grade: string) =>
  isQualityUnqualifiedGrade(grade) ? QUALITY_UNQUALIFIED_GRADE_TAG_TYPE[grade] : 'info';

const handlingStatusLabel = (status: string): string =>
  isQualityHandlingStatus(status) ? t(QUALITY_HANDLING_STATUS_LABEL_KEY[status]) : status;

const handlingStatusTagType = (status: string) =>
  isQualityHandlingStatus(status) ? QUALITY_HANDLING_STATUS_TAG_TYPE[status] : 'info';

/**
 * 处理动作的可达条件（与 process-result 端点状态门逐项一致，杜绝必然 4xx 的假按钮）：
 * - 只有写入方常量 pending 才是待处理，词表外的脏值一律不放行（fail-closed）；
 * - scrap 行排除：处置结果端点对报废恒 BUSINESS 拒绝（service :614-618），
 *   报废终态只能经财务/总经理两级审批端点推进，台账内不给"处理能改报废行"的假象；
 *   审批流中（pending_fin/pending_gm）的行必为 scrap 开单产物，同被此两条拦下。
 * 原第二条件（要求本行必须有来源质检记录 id 才放行）是旧"处理=按质检记录开单"契约的
 * 临时规避（无来源记录则开单必 404/误伤）；D1② 后端点按台账行自身 id 原地更新、与
 * 来源记录无关，该临时判据已如实收敛删除（形状锁对该回潮写法有负断言）。
 */
const canProcess = (row: UnqualifiedProductRecord): boolean =>
  row.handling_status === QUALITY_HANDLING_STATUS.pending &&
  isQualityHandlingMethod(row.handling_method) &&
  row.handling_method !== QUALITY_HANDLING_METHOD.scrap;

const fetchDefects = async () => {
  loading.value = true;
  try {
    const { getDefectList } = await import('@/api/quality');
    const res = await getDefectList();
    defects.value = res.data;
  } catch (error) {
    const err = error as Error;
    logger.error(t('quality.defectTab.messageFetchFailed'), err.message);
  } finally {
    loading.value = false;
  }
};

const processDialogVisible = ref(false);
const processing = ref(false);
const processTarget = ref<UnqualifiedProductRecord | null>(null);
// 处置结果表单 = ProcessResultRequest 两键（handling_method 必填 + reason 可选），
// 不含任何身份键（操作人由服务端会话派生）
const processForm = reactive({
  handling_method: '' as DefectHandlingMethod | '',
  reason: '',
});

const openProcessDialog = (row: UnqualifiedProductRecord) => {
  processTarget.value = row;
  Object.assign(processForm, {
    handling_method: '',
    reason: '',
  });
  processDialogVisible.value = true;
};

const submitProcess = async () => {
  const row = processTarget.value;
  if (!row) return;
  const { handling_method, reason } = processForm;
  if (!handling_method) {
    ElMessage.warning(t('quality.defectTab.ruleMethodRequired'));
    return;
  }
  processing.value = true;
  try {
    // 路径 id = 台账行自身 id（unqualified_products.id）；reason 为 Option 键，
    // 未填时省略（后端对空白串理由判 VALIDATION），不上送任何身份字段
    await processDefectRow(row.id, {
      handling_method,
      ...(reason.trim() ? { reason: reason.trim() } : {}),
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
