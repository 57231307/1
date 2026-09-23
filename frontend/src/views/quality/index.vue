<!--
  quality/index.vue - 质量管理
  D05 Batch 3：接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts
-->
<template>
  <div class="quality-page">
    <el-tabs v-model="activeTab" :aria-label="$t('quality.tabAriaLabel')">
      <el-tab-pane :label="$t('quality.tab.standard')" name="standard">
        <StandardTab @open-history="viewVersionHistory" @open-approve="approveStandard" />
      </el-tab-pane>

      <el-tab-pane :label="$t('quality.tab.record')" name="record">
        <RecordTab />
      </el-tab-pane>

      <el-tab-pane :label="$t('quality.tab.defect')" name="defect">
        <DefectTab />
      </el-tab-pane>
    </el-tabs>

    <el-dialog
      v-model="standardDialogVisible"
      :title="
        standardForm.id
          ? $t('quality.standardDialog.editTitle')
          : $t('quality.standardDialog.createTitle')
      "
      width="700px"
      :aria-label="$t('quality.standardDialog.ariaLabel')"
    >
      <el-form
        ref="standardFormRef"
        :model="standardForm"
        :rules="standardFormRules"
        label-width="100px"
        :aria-label="$t('quality.standardDialog.formAriaLabel')"
      >
        <el-form-item :label="$t('quality.standardDialog.standardCode')" prop="standard_code">
          <el-input
            v-model="standardForm.standard_code"
            :disabled="!!standardForm.id"
            :placeholder="$t('quality.standardDialog.standardCodePlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('quality.standardDialog.standardName')" prop="standard_name">
          <el-input
            v-model="standardForm.standard_name"
            :placeholder="$t('quality.standardDialog.standardNamePlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('quality.standardDialog.type')" prop="type">
          <el-select
            v-model="standardForm.type"
            :placeholder="$t('quality.standardDialog.typePlaceholder')"
            style="width: 100%"
          >
            <el-option :label="$t('quality.standardDialog.typeProduct')" value="product" />
            <el-option :label="$t('quality.standardDialog.typeProcess')" value="process" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('quality.standardDialog.version')" prop="version">
          <el-input
            v-model="standardForm.version"
            :placeholder="$t('quality.standardDialog.versionPlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('quality.standardDialog.content')" prop="content">
          <el-input
            v-model="standardForm.content"
            type="textarea"
            :rows="6"
            :placeholder="$t('quality.standardDialog.contentPlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('quality.standardDialog.attachments')" prop="attachments">
          <el-input
            v-model="attachmentsText"
            type="textarea"
            :placeholder="$t('quality.standardDialog.attachmentsPlaceholder')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="standardDialogVisible = false">{{
          $t('quality.standardDialog.cancel')
        }}</el-button>
        <el-button type="primary" :loading="standardSubmitLoading" @click="submitStandard">{{
          $t('quality.standardDialog.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="approveDialogVisible"
      :title="$t('quality.approveDialog.title')"
      width="500px"
      :aria-label="$t('quality.approveDialog.ariaLabel')"
    >
      <el-form
        ref="approveFormRef"
        :model="approveForm"
        :rules="approveFormRules"
        label-width="80px"
        :aria-label="$t('quality.approveDialog.formAriaLabel')"
      >
        <el-form-item :label="$t('quality.approveDialog.standardCode')">
          <el-input :model-value="approveStandardItem?.standard_code" disabled />
        </el-form-item>
        <el-form-item :label="$t('quality.approveDialog.standardName')">
          <el-input :model-value="approveStandardItem?.standard_name" disabled />
        </el-form-item>
        <el-form-item :label="$t('quality.approveDialog.currentVersion')">
          <el-input :model-value="approveStandardItem?.version" disabled />
        </el-form-item>
        <el-form-item :label="$t('quality.approveDialog.approvalComment')" prop="approval_comment">
          <el-input
            v-model="approveForm.approval_comment"
            type="textarea"
            :rows="4"
            :placeholder="$t('quality.approveDialog.approvalCommentPlaceholder')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="approveDialogVisible = false">{{
          $t('quality.approveDialog.cancel')
        }}</el-button>
        <el-button type="warning" :loading="approveSubmitLoading" @click="rejectStandard">{{
          $t('quality.approveDialog.reject')
        }}</el-button>
        <el-button type="primary" :loading="approveSubmitLoading" @click="confirmApprove">{{
          $t('quality.approveDialog.pass')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="versionHistoryVisible"
      :title="$t('quality.versionHistory.title')"
      width="800px"
      :aria-label="$t('quality.versionHistory.ariaLabel')"
    >
      <el-table
        v-loading="versionHistoryLoading"
        :data="versionHistoryList"
        stripe
        :aria-label="$t('quality.versionHistory.tableAriaLabel')"
      >
        <el-table-column prop="version" :label="$t('quality.versionHistory.version')" width="100" />
        <el-table-column prop="status" :label="$t('quality.versionHistory.status')" width="100">
          <template #default="{ row }">
            <el-tag :type="getStandardStatusType(row.status)" size="small">
              {{ getStandardStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="created_by_name"
          :label="$t('quality.versionHistory.createdBy')"
          width="100"
        />
        <el-table-column
          prop="created_at"
          :label="$t('quality.versionHistory.createdAt')"
          width="160"
        />
        <el-table-column
          prop="approved_by_name"
          :label="$t('quality.versionHistory.approvedBy')"
          width="100"
        >
          <template #default="{ row }">
            {{ row.approved_by_name || '-' }}
          </template>
        </el-table-column>
        <el-table-column
          prop="approved_at"
          :label="$t('quality.versionHistory.approvedAt')"
          width="160"
        >
          <template #default="{ row }">
            {{ row.approved_at || '-' }}
          </template>
        </el-table-column>
      </el-table>
      <template #footer>
        <el-button @click="versionHistoryVisible = false">{{
          $t('quality.versionHistory.close')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="recordDialogVisible"
      :title="
        recordForm.id
          ? $t('quality.recordDialog.editTitle')
          : $t('quality.recordDialog.createTitle')
      "
      width="700px"
      :aria-label="$t('quality.recordDialog.ariaLabel')"
    >
      <el-form
        ref="recordFormRef"
        :model="recordForm"
        :rules="recordFormRules"
        label-width="100px"
        :aria-label="$t('quality.recordDialog.formAriaLabel')"
      >
        <el-form-item :label="$t('quality.recordDialog.recordNo')" prop="inspection_no">
          <el-input v-model="recordForm.inspection_no" :disabled="!!recordForm.id" />
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.inspectionType')" prop="inspection_type">
          <el-select v-model="recordForm.inspection_type" style="width: 100%">
            <el-option
              v-for="item in inspectionTypeOptions"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.product')" prop="product_id">
          <el-select
            v-model="recordForm.product_id"
            filterable
            :placeholder="$t('quality.recordDialog.productPlaceholder')"
            style="width: 100%"
          >
            <el-option
              v-for="item in productOptions"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.batchNo')" prop="batch_no">
          <el-input v-model="recordForm.batch_no" />
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.inspectionDate')" prop="inspection_date">
          <el-date-picker
            v-model="recordForm.inspection_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.inspector')" prop="inspector_id">
          <el-select
            v-model="recordForm.inspector_id"
            filterable
            clearable
            :placeholder="$t('quality.recordDialog.inspectorPlaceholder')"
            style="width: 100%"
          >
            <el-option
              v-for="item in inspectorOptions"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.totalQty')" prop="total_qty">
          <el-input-number
            v-model="recordForm.total_qty"
            :min="0"
            :precision="2"
            :controls="false"
          />
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.inspectedQty')" prop="inspected_qty">
          <el-input-number
            v-model="recordForm.inspected_qty"
            :min="0"
            :precision="2"
            :controls="false"
          />
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.result')" prop="inspection_result">
          <el-radio-group v-model="recordForm.inspection_result">
            <el-radio
              v-for="item in resultOptions"
              :key="item.value"
              :label="item.value"
              :value="item.value"
            >
              {{ item.label }}
            </el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.remark')" prop="remark">
          <el-input v-model="recordForm.remark" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="recordDialogVisible = false">{{
          $t('quality.recordDialog.cancel')
        }}</el-button>
        <el-button type="primary" :loading="recordSubmitLoading" @click="submitRecord">{{
          $t('quality.recordDialog.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, reactive, onMounted, provide } from 'vue';
import { useI18n } from 'vue-i18n';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import StandardTab from './tabs/StandardTab.vue';
import RecordTab from './tabs/RecordTab.vue';
import DefectTab from './tabs/DefectTab.vue';
import {
  getQualityStandardList,
  createQualityStandard,
  updateQualityStandard,
  approveQualityStandard,
  rejectQualityStandard,
  getQualityRecordList,
  createQualityRecord,
  // 批次 94 P2-12 修复：补全 updateQualityRecord 用于实现更新功能
  updateQualityRecord,
  getDefectList,
  getQualityStandardVersions,
  type QualityStandard,
  type QualityRecord,
  type Defect,
} from '@/api/quality';
import { useQualityLookups } from './composables/useQualityLookups';
import {
  QUALITY_RECORD_RESULT,
  QUALITY_RECORD_RESULT_LABEL_KEY,
  QUALITY_RECORD_RESULT_VALUES,
} from '@/constants/quality-inspection-record';
import {
  QUALITY_INSPECTION_TYPE_LABEL_KEY,
  QUALITY_INSPECTION_TYPE_VALUES,
} from '@/constants/quality-inspection-type';

// v11 批次 173 P2-1 修复：el-tag type 类型
type TagType = 'success' | 'warning' | 'info' | 'primary' | 'danger';

// v11 批次 161 P2-5 修复：后端已支持分页（返回 PaginatedResponse 含 items+total），
// RecordTab 通过 useTableApi 自动消费分页元数据，无需在此维护分页状态

const { t } = useI18n({ useScope: 'global' });

// 为 StandardTab/RecordTab 提供 actions（inject('qualityActions')）
// 注意：provide 移到所有函数定义之后，避免 hoisting 问题（vue-tsc 报 used before declaration）
const activeTab = ref('standard');
const standards = ref<QualityStandard[]>([]);
const records = ref<QualityRecord[]>([]);
const defects = ref<Defect[]>([]);
const standardLoading = ref(false);
const recordLoading = ref(false);
const defectLoading = ref(false);

const fetchStandards = async () => {
  standardLoading.value = true;
  try {
    // v11 批次 173 P2-1 修复：const res: any 改为直接使用 API 返回类型
    const res = await getQualityStandardList();
    // 安全检查：防止后端返回 data 为 null 时崩溃
    standards.value = res.data || [];
  } finally {
    standardLoading.value = false;
  }
};

const fetchRecords = async () => {
  recordLoading.value = true;
  try {
    // 后端 list_records 以 PaginatedResponse 返回（data.items + total），按 items 显式读取
    const res = await getQualityRecordList({ page: 1, page_size: 10 });
    records.value = res.data.items;
  } finally {
    recordLoading.value = false;
  }
};

const fetchDefects = async () => {
  defectLoading.value = true;
  try {
    // v11 批次 173 P2-1 修复：const res: any 改为直接使用 API 返回类型
    const res = await getDefectList();
    // 安全检查：防止后端返回 data 为 null 时崩溃
    defects.value = res.data || [];
  } finally {
    defectLoading.value = false;
  }
};

const getStandardStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    draft: t('quality.standardStatus.draft'),
    approved: t('quality.standardStatus.approved'),
    published: t('quality.standardStatus.published'),
    rejected: t('quality.standardStatus.rejected'),
  };
  return map[status] || status;
};

const getStandardStatusType = (status: string): TagType => {
  // v11 批次 173 P2-1 修复：Record<string, any> 改为 Record<string, TagType>
  const map: Record<string, TagType> = {
    draft: 'info',
    approved: 'warning',
    published: 'success',
    rejected: 'danger',
  };
  return map[status] || 'info';
};

const standardDialogVisible = ref(false);
const standardFormRef = ref<FormInstance>();
const standardSubmitLoading = ref(false);
const attachmentsText = ref('');
const standardForm = reactive({
  id: 0,
  standard_code: '',
  standard_name: '',
  version: '1.0',
  type: 'product' as const,
  status: 'draft' as const,
  content: '',
  attachments: [] as string[],
});
const standardFormRules: FormRules = {
  standard_code: [
    { required: true, message: t('quality.validation.standardCodeRequired'), trigger: 'blur' },
  ],
  standard_name: [
    { required: true, message: t('quality.validation.standardNameRequired'), trigger: 'blur' },
  ],
  type: [{ required: true, message: t('quality.validation.typeRequired'), trigger: 'change' }],
  version: [{ required: true, message: t('quality.validation.versionRequired'), trigger: 'blur' }],
  content: [{ required: true, message: t('quality.validation.contentRequired'), trigger: 'blur' }],
};

const openStandardDialog = (row?: QualityStandard) => {
  if (row) {
    Object.assign(standardForm, row);
    attachmentsText.value = JSON.stringify(row.attachments || [], null, 2);
  } else {
    Object.assign(standardForm, {
      id: 0,
      standard_code: '',
      standard_name: '',
      version: '1.0',
      type: 'product',
      status: 'draft',
      content: '',
      attachments: [],
    });
    attachmentsText.value = '';
  }
  standardDialogVisible.value = true;
};

const submitStandard = async () => {
  if (!standardFormRef.value) return;
  await standardFormRef.value.validate(async valid => {
    if (!valid) return;

    standardSubmitLoading.value = true;
    try {
      if (attachmentsText.value) {
        try {
          standardForm.attachments = JSON.parse(attachmentsText.value);
        } catch (e) {
          ElMessage.error(t('quality.message.attachmentsFormatError'));
          return;
        }
      }
      if (standardForm.id) {
        await updateQualityStandard(standardForm.id, standardForm as Partial<QualityStandard>);
      } else {
        await createQualityStandard(standardForm as Partial<QualityStandard>);
      }
      ElMessage.success(t('quality.message.operationSuccess'));
      standardDialogVisible.value = false;
      fetchStandards();
    } catch (e: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
      ElMessage.error(
        (e instanceof Error ? e.message : String(e)) || t('quality.message.operationFailed')
      );
    } finally {
      standardSubmitLoading.value = false;
    }
  });
};

const approveDialogVisible = ref(false);
const approveFormRef = ref<FormInstance>();
const approveSubmitLoading = ref(false);
const approveStandardItem = ref<QualityStandard | null>(null);
const approveForm = reactive({ approval_comment: '' });
const approveFormRules: FormRules = {
  approval_comment: [
    { required: true, message: t('quality.validation.approvalCommentRequired'), trigger: 'blur' },
  ],
};

const approveStandard = async (row: QualityStandard) => {
  approveStandardItem.value! = row;
  approveForm.approval_comment = '';
  approveDialogVisible.value = true;
};

const confirmApprove = async () => {
  if (!approveFormRef.value || !approveStandardItem.value!) return;
  await approveFormRef.value.validate(async valid => {
    if (!valid) return;

    approveSubmitLoading.value = true;
    try {
      await approveQualityStandard(approveStandardItem.value!.id);
      ElMessage.success(t('quality.message.approveSuccess'));
      approveDialogVisible.value = false;
      fetchStandards();
    } catch (e: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
      ElMessage.error(
        (e instanceof Error ? e.message : String(e)) || t('quality.message.operationFailed')
      );
    } finally {
      approveSubmitLoading.value = false;
    }
  });
};

const rejectStandard = async () => {
  if (!approveStandardItem.value!) return;
  try {
    const reason = await ElMessageBox.prompt(
      t('quality.message.rejectPrompt'),
      t('quality.message.rejectTitle'),
      {
        type: 'warning',
        confirmButtonText: t('quality.message.rejectConfirmButton'),
        cancelButtonText: t('quality.message.rejectCancelButton'),
        inputPlaceholder: t('quality.message.rejectPlaceholder'),
        inputType: 'textarea',
      }
    );
    // 批次 157d-2 修复：接入 rejectQualityStandard API
    await rejectQualityStandard(approveStandardItem.value!.id, {
      reject_reason: reason.value || undefined,
    });
    ElMessage.success(t('quality.message.rejectSuccess'));
    approveDialogVisible.value = false;
    fetchStandards();
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    if (e !== 'cancel')
      ElMessage.error(
        (e instanceof Error ? e.message : String(e)) || t('quality.message.operationFailed')
      );
  }
};

const versionHistoryVisible = ref(false);
const versionHistoryLoading = ref(false);
const versionHistoryList = ref<QualityStandard[]>([]);

const viewVersionHistory = async (row: QualityStandard) => {
  versionHistoryLoading.value = true;
  try {
    // v11 批次 173 P2-1 修复：const res: any 改为直接使用 API 返回类型
    const res = await getQualityStandardVersions(row.id);
    // 安全检查：防止后端返回 data 为 null 时崩溃
    versionHistoryList.value = res.data || [];
    versionHistoryVisible.value = true;
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('quality.message.fetchVersionHistoryFailed')
    );
  } finally {
    versionHistoryLoading.value = false;
  }
};

const recordDialogVisible = ref(false);
const recordFormRef = ref<FormInstance>();
const recordSubmitLoading = ref(false);

// 记录表只存 product_id / inspector_id，界面按主数据选择而不是让人手输名称
const { load: loadLookups, productOptions, inspectorOptions } = useQualityLookups();

// 检验类型与检验结论的取值词表出自 constants，切语言时文案随之更新
const inspectionTypeOptions = computed(() =>
  QUALITY_INSPECTION_TYPE_VALUES.map(value => ({
    value,
    label: t(QUALITY_INSPECTION_TYPE_LABEL_KEY[value]),
  }))
);
const resultOptions = computed(() =>
  QUALITY_RECORD_RESULT_VALUES.map(value => ({
    value,
    label: t(QUALITY_RECORD_RESULT_LABEL_KEY[value]),
  }))
);

/**
 * 表单模型与后端 CreateInspectionRecordRequest 同名同域：
 * inspection_no / inspection_type / product_id / inspection_date / total_qty /
 * inspected_qty / inspection_result 六项在后端都是非 Option，缺任一项即被反序列化拒绝。
 */
const emptyRecordForm = () => ({
  id: 0,
  inspection_no: '',
  inspection_type: '',
  product_id: undefined as number | undefined,
  batch_no: '',
  inspection_date: '',
  inspector_id: undefined as number | undefined,
  total_qty: undefined as number | undefined,
  inspected_qty: undefined as number | undefined,
  inspection_result: QUALITY_RECORD_RESULT.pending as string,
  remark: '',
});
const recordForm = reactive(emptyRecordForm());

const recordFormRules: FormRules = {
  inspection_no: [{ required: true, message: t('quality.recordDialog.ruleNo'), trigger: 'blur' }],
  inspection_type: [
    { required: true, message: t('quality.recordDialog.ruleType'), trigger: 'change' },
  ],
  product_id: [
    { required: true, message: t('quality.recordDialog.ruleProduct'), trigger: 'change' },
  ],
  inspection_date: [
    { required: true, message: t('quality.recordDialog.ruleDate'), trigger: 'change' },
  ],
  total_qty: [{ required: true, message: t('quality.recordDialog.ruleTotalQty'), trigger: 'blur' }],
  inspected_qty: [
    { required: true, message: t('quality.recordDialog.ruleInspectedQty'), trigger: 'blur' },
  ],
  inspection_result: [
    { required: true, message: t('quality.recordDialog.ruleResult'), trigger: 'change' },
  ],
};

const openRecordDialog = (row?: QualityRecord) => {
  if (row) {
    Object.assign(recordForm, {
      id: row.id,
      inspection_no: row.inspection_no,
      inspection_type: row.inspection_type,
      product_id: row.product_id,
      batch_no: row.batch_no ?? '',
      inspection_date: row.inspection_date,
      inspector_id: row.inspector_id ?? undefined,
      total_qty: Number(row.total_qty),
      inspected_qty: Number(row.inspected_qty),
      inspection_result: row.inspection_result,
      remark: row.remark ?? '',
    });
  } else {
    Object.assign(recordForm, emptyRecordForm());
  }
  recordDialogVisible.value = true;
};

const submitRecord = async () => {
  const formInstance = recordFormRef.value;
  if (!formInstance) return;
  try {
    await formInstance.validate();
  } catch {
    // 必填项缺失时表单已就地标红，不再发必然被后端拒绝的请求
    return;
  }
  const { product_id, total_qty, inspected_qty } = recordForm;
  if (product_id === undefined || total_qty === undefined || inspected_qty === undefined) {
    // validate() 已拦下缺失项；这里只是让类型收敛到后端要求的非空值
    return;
  }
  const fields = {
    inspection_type: recordForm.inspection_type,
    product_id,
    inspection_date: recordForm.inspection_date,
    total_qty,
    inspected_qty,
    inspection_result: recordForm.inspection_result,
    batch_no: recordForm.batch_no || undefined,
    inspector_id: recordForm.inspector_id,
    remark: recordForm.remark || undefined,
  };
  recordSubmitLoading.value = true;
  try {
    if (recordForm.id) {
      // 批次 94 P2-12 修复：原占位"更新功能待实现"，现接入真实更新 API
      await updateQualityRecord(recordForm.id, fields);
    } else {
      await createQualityRecord({ inspection_no: recordForm.inspection_no, ...fields });
    }
    ElMessage.success(t('quality.message.operationSuccess'));
    recordDialogVisible.value = false;
    fetchRecords();
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('quality.message.operationFailed')
    );
  } finally {
    recordSubmitLoading.value = false;
  }
};

const hasLoaded = createLazyLoader();

onMounted(() => {
  fetchStandards();
  loadIfNot('records', fetchRecords, hasLoaded);
  loadIfNot('defects', fetchDefects, hasLoaded);
  void loadLookups();
});

// provide 必须在所有函数定义之后，避免 hoisting 问题
provide('qualityActions', {
  openStandardDialog,
  openRecordDialog,
});
// 主数据（产品/检验人）由 useQualityLookups 模块级缓存，父页与子 tab 共用一份
</script>

<style scoped>
.quality-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
</style>
