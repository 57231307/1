<template>
  <div class="lab-dip-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>{{ t('labDip.index.pageTitle') }}</h2>
        <div class="header-actions">
          <el-select
            v-model="queryStatus"
            :placeholder="t('labDip.index.placeholderAllStatus')"
            clearable
            style="width: 160px"
            @change="handleFilter"
          >
            <el-option
              v-for="(label, key) in statusTextMap"
              :key="key"
              :label="label"
              :value="key"
            />
          </el-select>
          <el-button type="primary" @click="handleCreate">{{
            t('labDip.index.buttonCreate')
          }}</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="requestList" border>
        <el-table-column
          prop="request_no"
          :label="t('labDip.index.colRequestNo')"
          min-width="180"
        />
        <el-table-column
          :label="t('labDip.index.colCustomerColorNo')"
          prop="customer_color_no"
          width="130"
        >
          <template #default="{ row }">{{ row.customer_color_no || '-' }}</template>
        </el-table-column>
        <el-table-column
          prop="customer_color_name"
          :label="t('labDip.index.colCustomerColorName')"
          min-width="130"
          show-overflow-tooltip
        >
          <template #default="{ row }">{{ row.customer_color_name || '-' }}</template>
        </el-table-column>
        <el-table-column
          prop="light_source"
          :label="t('labDip.index.colLightSource')"
          width="100"
          align="center"
        />
        <el-table-column
          prop="sample_versions"
          :label="t('labDip.index.colSampleVersions')"
          width="70"
          align="center"
        />
        <el-table-column
          prop="required_date"
          :label="t('labDip.index.colRequiredDate')"
          width="120"
        />
        <el-table-column prop="status" :label="t('common.status')" width="110" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTagMap[row.status as LabDipRequestStatus] ?? 'info'">
              {{ statusTextMap[row.status as LabDipRequestStatus] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('common.operation')" width="280" fixed="right">
          <template #default="{ row }">
            <el-button
              v-for="action in getNextActions(row)"
              :key="action.key"
              size="small"
              link
              :type="action.danger ? 'danger' : 'primary'"
              @click="runAction(action, row)"
            >
              {{ action.label }}
            </el-button>
            <el-button
              v-if="row.status === 'pending'"
              size="small"
              link
              type="danger"
              @click="handleDelete(row)"
            >
              {{ t('common.delete') }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50]"
        layout="total, sizes, prev, pager, next"
        style="margin-top: 16px; justify-content: flex-end"
        @current-change="loadList"
        @size-change="handleFilter"
      />
    </el-card>

    <el-dialog
      v-model="createVisible"
      :title="t('labDip.index.titleCreate')"
      width="580px"
      @close="resetForm"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="110px">
        <el-form-item :label="t('labDip.index.labelCustomerId')" prop="customer_id">
          <el-input-number
            v-model="formData.customer_id"
            :min="1"
            :precision="0"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('labDip.index.colCustomerColorNo')" prop="customer_color_no">
          <el-input
            v-model="formData.customer_color_no"
            :placeholder="t('labDip.index.placeholderOptional')"
          />
        </el-form-item>
        <el-form-item :label="t('labDip.index.colCustomerColorName')" prop="customer_color_name">
          <el-input
            v-model="formData.customer_color_name"
            :placeholder="t('labDip.index.placeholderOptional')"
          />
        </el-form-item>
        <el-form-item :label="t('labDip.index.labelLightSource')" prop="light_source">
          <el-select
            v-model="formData.light_source"
            :placeholder="t('labDip.index.placeholderRequired')"
          >
            <el-option v-for="ls in LIGHT_SOURCES" :key="ls" :label="ls" :value="ls" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('labDip.index.labelFabricSpec')" prop="fabric_spec">
          <el-input
            v-model="formData.fabric_spec"
            :placeholder="t('labDip.index.placeholderFabricSpec')"
          />
        </el-form-item>
        <el-form-item :label="t('labDip.index.labelDyeCategory')" prop="dye_category">
          <el-select
            v-model="formData.dye_category"
            :placeholder="t('labDip.index.placeholderOptional')"
            clearable
          >
            <el-option :label="t('labDip.index.optionReactive')" value="reactive" />
            <el-option :label="t('labDip.index.optionDisperse')" value="disperse" />
            <el-option :label="t('labDip.index.optionAcid')" value="acid" />
            <el-option :label="t('labDip.index.optionVat')" value="vat" />
            <el-option :label="t('labDip.index.optionSulfur')" value="sulfur" />
            <el-option :label="t('labDip.index.optionDirect')" value="direct" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('labDip.index.labelSampleVersions')" prop="sample_versions">
          <el-input-number
            v-model="formData.sample_versions"
            :min="1"
            :max="8"
            :precision="0"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('labDip.index.colRequiredDate')" prop="required_date">
          <el-date-picker
            v-model="formData.required_date"
            type="date"
            value-format="YYYY-MM-DD"
            :placeholder="t('labDip.index.placeholderRequired')"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('labDip.index.labelRemarks')" prop="remarks">
          <el-input v-model="formData.remarks" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="submitCreate">{{
          t('labDip.index.buttonConfirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" :title="t('labDip.index.titleDetail')" width="640px">
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item :label="t('labDip.index.colRequestNo')">{{
          detailRow.request_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('common.status')">
          <el-tag :type="statusTagMap[detailRow.status] ?? 'info'">
            {{ statusTextMap[detailRow.status] ?? detailRow.status }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('labDip.index.colCustomerColorNo')">{{
          detailRow.customer_color_no || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('labDip.index.colCustomerColorName')">{{
          detailRow.customer_color_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('labDip.index.labelSampleType')">{{
          detailRow.sample_type || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('labDip.index.colLightSource')">{{
          detailRow.light_source
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('labDip.index.labelSecondaryLightSource')">{{
          detailRow.secondary_light_source || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('labDip.index.colSampleVersions')">{{
          detailRow.sample_versions
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('labDip.index.labelFabricSpec')">{{
          detailRow.fabric_spec || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('labDip.index.labelFabricComponent')">{{
          detailRow.fabric_component || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('labDip.index.labelDyeCategory')">{{
          detailRow.dye_category || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('labDip.index.colRequiredDate')">{{
          detailRow.required_date
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('common.createTime')" :span="2">{{
          detailRow.created_at
        }}</el-descriptions-item>
      </el-descriptions>
      <template #footer>
        <el-button @click="detailVisible = false">{{ t('common.close') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { useI18n } from 'vue-i18n';
import {
  getLabDipRequestList,
  createLabDipRequest,
  deleteLabDipRequest,
  startLabDipSampling,
  submitLabDipRequest,
  approveLabDipRequest,
  rejectLabDipRequest,
  restartLabDipSampling,
  completeLabDipRequest,
  type LabDipRequest,
  type LabDipRequestStatus,
} from '@/api/lab-dip';

const { t } = useI18n({ useScope: 'global' });
const LIGHT_SOURCES = ['D65', 'TL84', 'U3000', 'CWF', 'A'];

const statusTextMap: Record<LabDipRequestStatus, string> = {
  pending: t('labDip.index.statusPending'),
  sampling: t('labDip.index.statusSampling'),
  submitted: t('labDip.index.statusSubmitted'),
  approved: t('labDip.index.statusApproved'),
  rejected: t('labDip.index.statusRejected'),
  completed: t('labDip.index.statusCompleted'),
};

const statusTagMap: Record<
  LabDipRequestStatus,
  'info' | 'warning' | 'primary' | 'success' | 'danger'
> = {
  pending: 'info',
  sampling: 'warning',
  submitted: 'primary',
  approved: 'success',
  rejected: 'danger',
  completed: 'success',
};

interface StatusAction {
  key: string;
  label: string;
  danger?: boolean;
}

/** 按后端状态机（lab_dip_request 状态机）生成下一步操作 */
const nextActionMap: Partial<Record<LabDipRequestStatus, StatusAction[]>> = {
  pending: [{ key: 'start-sampling', label: t('labDip.index.buttonStartSampling') }],
  sampling: [{ key: 'submit', label: t('labDip.index.buttonSubmitToCustomer') }],
  submitted: [
    { key: 'approve', label: t('labDip.index.buttonApprove') },
    { key: 'reject', label: t('labDip.index.buttonReject'), danger: true },
  ],
  rejected: [{ key: 'restart', label: t('labDip.index.buttonRestart') }],
  approved: [{ key: 'complete', label: t('labDip.index.buttonComplete') }],
};

const actionRunners: Record<string, (id: number) => Promise<unknown>> = {
  'start-sampling': (id: number) => startLabDipSampling(id),
  submit: (id: number) => submitLabDipRequest(id),
  reject: (id: number) => rejectLabDipRequest(id),
  restart: (id: number) => restartLabDipSampling(id),
};

const loading = ref(false);
const submitLoading = ref(false);
const requestList = ref<LabDipRequest[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const queryStatus = ref('');
const createVisible = ref(false);
const detailVisible = ref(false);
const detailRow = ref<LabDipRequest | null>(null);
const formRef = ref<FormInstance>();

const formData = reactive<{
  customer_id: number | undefined;
  customer_color_no: string;
  customer_color_name: string;
  light_source: string;
  fabric_spec: string;
  dye_category: string;
  sample_versions: number;
  required_date: string;
  remarks: string;
}>({
  customer_id: undefined,
  customer_color_no: '',
  customer_color_name: '',
  light_source: '',
  fabric_spec: '',
  dye_category: '',
  sample_versions: 4,
  required_date: '',
  remarks: '',
});

const formRules: FormRules = {
  light_source: [
    { required: true, message: t('labDip.index.ruleLightSourceRequired'), trigger: 'change' },
  ],
  required_date: [
    { required: true, message: t('labDip.index.ruleRequiredDateRequired'), trigger: 'change' },
  ],
};

/** 响应解包防御：兼容数组 / { items } 分页包装，避免 el-table "r is not iterable" */
const unwrapList = (payload: unknown): LabDipRequest[] => {
  if (Array.isArray(payload)) return payload;
  const paged = payload as { items?: LabDipRequest[] } | null;
  return paged?.items ?? [];
};

const loadList = async () => {
  loading.value = true;
  try {
    const res = await getLabDipRequestList({
      page: page.value,
      page_size: pageSize.value,
      status: queryStatus.value || undefined,
    });
    requestList.value = unwrapList(res.data);
    total.value =
      res.data && !Array.isArray(res.data) ? (res.data.total ?? 0) : requestList.value.length;
  } catch {
    ElMessage.error(t('labDip.index.messageFetchListFailed'));
  } finally {
    loading.value = false;
  }
};

const handleFilter = () => {
  page.value = 1;
  loadList();
};

const getNextActions = (row: LabDipRequest): StatusAction[] =>
  nextActionMap[row.status as LabDipRequestStatus] ?? [];

const runAction = async (action: StatusAction, row: LabDipRequest) => {
  try {
    if (action.key === 'approve') {
      const { value } = await ElMessageBox.prompt(
        t('labDip.index.messageApprovePrompt', { no: row.request_no }),
        t('labDip.index.buttonApprove'),
        {
          confirmButtonText: t('common.confirm'),
          cancelButtonText: t('common.cancel'),
          inputPlaceholder: t('labDip.index.placeholderSampleId'),
          inputPattern: /^[1-9]\d*$/,
          inputErrorMessage: t('labDip.index.messageInvalidSampleId'),
        }
      );
      await approveLabDipRequest(row.id, { sample_id: Number(value) });
    } else if (action.key === 'complete') {
      const { value } = await ElMessageBox.prompt(
        t('labDip.index.messageCompletePrompt', { no: row.request_no }),
        t('labDip.index.titleCompleteToRecipe'),
        {
          confirmButtonText: t('common.confirm'),
          cancelButtonText: t('common.cancel'),
          inputPlaceholder: t('labDip.index.placeholderProductionRecipeId'),
          inputPattern: /^[1-9]\d*$/,
          inputErrorMessage: t('labDip.index.messageInvalidRecipeId'),
        }
      );
      await completeLabDipRequest(row.id, { production_recipe_id: Number(value) });
    } else {
      await ElMessageBox.confirm(
        t('labDip.index.messageConfirmAction', { no: row.request_no, action: action.label }),
        t('labDip.index.titleActionConfirm'),
        {
          confirmButtonText: t('common.confirm'),
          cancelButtonText: t('common.cancel'),
          type: 'warning',
        }
      );
      await actionRunners[action.key](row.id);
    }
    ElMessage.success(t('labDip.index.messageOperationSuccess', { action: action.label }));
    await loadList();
  } catch (error) {
    if (error === 'cancel' || (error as { message?: string })?.message === 'cancel') return;
    ElMessage.error(t('labDip.index.messageOperationFailed', { action: action.label }));
  }
};

const handleCreate = () => {
  createVisible.value = true;
};

const resetForm = () => {
  formData.customer_id = undefined;
  formData.customer_color_no = '';
  formData.customer_color_name = '';
  formData.light_source = '';
  formData.fabric_spec = '';
  formData.dye_category = '';
  formData.sample_versions = 4;
  formData.required_date = '';
  formData.remarks = '';
  formRef.value?.resetFields();
};

const submitCreate = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    submitLoading.value = true;
    try {
      await createLabDipRequest({
        customer_id: formData.customer_id,
        customer_color_no: formData.customer_color_no || undefined,
        customer_color_name: formData.customer_color_name || undefined,
        light_source: formData.light_source,
        fabric_spec: formData.fabric_spec || undefined,
        dye_category: formData.dye_category || undefined,
        sample_versions: formData.sample_versions,
        required_date: formData.required_date,
        remarks: formData.remarks || undefined,
      });
      ElMessage.success(t('labDip.index.messageCreateSuccess'));
      createVisible.value = false;
      await loadList();
    } catch {
      ElMessage.error(t('labDip.index.messageCreateFailed'));
    } finally {
      submitLoading.value = false;
    }
  });
};

const handleDelete = async (row: LabDipRequest) => {
  try {
    await ElMessageBox.confirm(
      t('labDip.index.messageDeleteConfirm', { no: row.request_no }),
      t('labDip.index.titleDeleteConfirm'),
      {
        confirmButtonText: t('labDip.index.buttonConfirmDelete'),
        cancelButtonText: t('common.cancel'),
        type: 'warning',
      }
    );
    await deleteLabDipRequest(row.id);
    ElMessage.success(t('common.message.deleteSuccess'));
    await loadList();
  } catch (error) {
    if (error === 'cancel' || (error as { message?: string })?.message === 'cancel') return;
    ElMessage.error(t('labDip.index.messageDeleteFailed'));
  }
};

onMounted(() => {
  loadList();
});
</script>

<style scoped>
.lab-dip-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.page-header h2 {
  margin: 0;
  font-size: 18px;
}

.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}
</style>
