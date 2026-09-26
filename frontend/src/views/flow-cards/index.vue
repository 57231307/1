<template>
  <div class="flow-cards-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>{{ t('flowCards.index.pageTitle') }}</h2>
        <div class="header-actions">
          <el-select
            v-model="queryStatus"
            :placeholder="t('flowCards.index.placeholderAllStatus')"
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
            t('flowCards.index.buttonCreate')
          }}</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="cardList" border>
        <el-table-column prop="card_no" :label="t('flowCards.index.colCardNo')" min-width="180" />
        <el-table-column
          prop="production_order_id"
          :label="t('flowCards.index.colProductionOrder')"
          width="100"
          align="center"
        />
        <el-table-column
          prop="product_name"
          :label="t('flowCards.index.colProduct')"
          min-width="120"
          show-overflow-tooltip
        >
          <template #default="{ row }">{{ row.product_name || '-' }}</template>
        </el-table-column>
        <el-table-column :label="t('flowCards.index.colColorNo')" prop="color_no" width="120">
          <template #default="{ row }">{{ row.color_no || '-' }}</template>
        </el-table-column>
        <el-table-column prop="status" :label="t('common.status')" width="110" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTagMap[row.status as FlowCardStatus] ?? 'info'">
              {{ statusTextMap[row.status as FlowCardStatus] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="planned_fabric_weight"
          :label="t('flowCards.index.colPlannedWeight')"
          width="120"
          align="right"
        >
          <template #default="{ row }">{{ row.planned_fabric_weight ?? '-' }}</template>
        </el-table-column>
        <el-table-column prop="dye_lot_no" :label="t('flowCards.index.colDyeLotNo')" width="130">
          <template #default="{ row }">{{ row.dye_lot_no || '-' }}</template>
        </el-table-column>
        <el-table-column :label="t('common.operation')" width="260" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link type="primary" @click="handleDetail(row)">
              {{ t('common.detail') }}
            </el-button>
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
              v-if="canDelete(row)"
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
      :title="t('flowCards.index.titleCreate')"
      width="560px"
      @close="resetForm"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="110px">
        <el-form-item
          :label="t('flowCards.index.labelProductionOrderId')"
          prop="production_order_id"
        >
          <el-input-number
            v-model="formData.production_order_id"
            :min="1"
            :precision="0"
            style="width: 100%"
            :placeholder="t('flowCards.index.placeholderRequired')"
          />
        </el-form-item>
        <el-form-item :label="t('flowCards.index.labelProductId')" prop="product_id">
          <el-input-number
            v-model="formData.product_id"
            :min="1"
            :precision="0"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('flowCards.index.labelProductName')" prop="product_name">
          <el-input
            v-model="formData.product_name"
            :placeholder="t('flowCards.index.placeholderOptional')"
          />
        </el-form-item>
        <el-form-item :label="t('flowCards.index.colColorNo')" prop="color_no">
          <el-input
            v-model="formData.color_no"
            :placeholder="t('flowCards.index.placeholderOptional')"
          />
        </el-form-item>
        <el-form-item :label="t('flowCards.index.colPlannedWeight')" prop="planned_fabric_weight">
          <el-input-number
            v-model="formData.planned_fabric_weight"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item
          :label="t('flowCards.index.labelDyeingRequirements')"
          prop="dyeing_requirements"
        >
          <el-input v-model="formData.dyeing_requirements" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="submitCreate">{{
          t('flowCards.index.buttonConfirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" :title="t('flowCards.index.titleDetail')" width="640px">
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item :label="t('flowCards.index.colCardNo')">{{
          detailRow.card_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('flowCards.index.labelBarcode')">{{
          detailRow.barcode
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('flowCards.index.labelProductionOrderId')">{{
          detailRow.production_order_id
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('common.status')">
          <el-tag :type="statusTagMap[detailRow.status] ?? 'info'">
            {{ statusTextMap[detailRow.status] ?? detailRow.status }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('flowCards.index.colProduct')">{{
          detailRow.product_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('flowCards.index.colColorNo')">{{
          detailRow.color_no || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('flowCards.index.labelCustomer')">{{
          detailRow.customer_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('flowCards.index.colDyeLotNo')">{{
          detailRow.dye_lot_no || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('flowCards.index.colPlannedWeight')">{{
          detailRow.planned_fabric_weight ?? '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('flowCards.index.labelActualWeight')">{{
          detailRow.actual_fabric_weight ?? '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('flowCards.index.labelCurrentStepSeq')">{{
          detailRow.current_step_seq
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('common.createTime')">{{
          detailRow.created_at
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('flowCards.index.labelDyeingRequirements')" :span="2">{{
          detailRow.dyeing_requirements || '-'
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
  getFlowCardList,
  createFlowCard,
  deleteFlowCard,
  scheduleFlowCard,
  startPreparing,
  completePreparing,
  startDyeing,
  completeDyeing,
  startInspecting,
  completeFlowCard,
  shipFlowCard,
  terminateFlowCard,
  reactivateFlowCard,
  type FlowCard,
  type FlowCardStatus,
} from '@/api/flow-card';

const { t } = useI18n({ useScope: 'global' });

const statusTextMap: Record<FlowCardStatus, string> = {
  pending: t('flowCards.index.statusPending'),
  scheduled: t('flowCards.index.statusScheduled'),
  preparing: t('flowCards.index.statusPreparing'),
  dyeing: t('flowCards.index.statusDyeing'),
  dyed: t('flowCards.index.statusDyed'),
  inspecting: t('flowCards.index.statusInspecting'),
  completed: t('flowCards.index.statusCompleted'),
  shipped: t('flowCards.index.statusShipped'),
  terminated: t('flowCards.index.statusTerminated'),
};

const statusTagMap: Record<FlowCardStatus, 'info' | 'warning' | 'primary' | 'success' | 'danger'> =
  {
    pending: 'info',
    scheduled: 'warning',
    preparing: 'warning',
    dyeing: 'primary',
    dyed: 'primary',
    inspecting: 'warning',
    completed: 'success',
    shipped: 'success',
    terminated: 'danger',
  };

interface StatusAction {
  key: string;
  label: string;
  danger?: boolean;
}

/** 按后端状态机（validate_status_transition）生成下一步操作 */
const nextActionMap: Partial<Record<FlowCardStatus, StatusAction[]>> = {
  pending: [
    { key: 'schedule', label: t('flowCards.index.buttonSchedule') },
    { key: 'terminate', label: t('flowCards.index.buttonTerminate'), danger: true },
  ],
  scheduled: [
    { key: 'start-preparing', label: t('flowCards.index.buttonStartPreparing') },
    { key: 'terminate', label: t('flowCards.index.buttonTerminate'), danger: true },
  ],
  preparing: [
    { key: 'complete-preparing', label: t('flowCards.index.buttonCompletePreparing') },
    { key: 'start-dyeing', label: t('flowCards.index.buttonStartDyeing') },
    { key: 'terminate', label: t('flowCards.index.buttonTerminate'), danger: true },
  ],
  dyeing: [
    { key: 'complete-dyeing', label: t('flowCards.index.buttonCompleteDyeing') },
    { key: 'terminate', label: t('flowCards.index.buttonTerminate'), danger: true },
  ],
  dyed: [{ key: 'start-inspecting', label: t('flowCards.index.buttonStartInspecting') }],
  inspecting: [{ key: 'complete', label: t('flowCards.index.buttonCompleteInspecting') }],
  completed: [{ key: 'ship', label: t('flowCards.index.buttonShip') }],
  terminated: [{ key: 'reactivate', label: t('flowCards.index.buttonReactivate') }],
};

const actionRunners: Record<string, (id: number) => Promise<unknown>> = {
  schedule: (id: number) => scheduleFlowCard(id),
  'start-preparing': (id: number) => startPreparing(id),
  'start-dyeing': (id: number) => startDyeing(id),
  'complete-dyeing': (id: number) => completeDyeing(id),
  'start-inspecting': (id: number) => startInspecting(id),
  complete: (id: number) => completeFlowCard(id),
  ship: (id: number) => shipFlowCard(id),
  reactivate: (id: number) => reactivateFlowCard(id),
};

const loading = ref(false);
const submitLoading = ref(false);
const cardList = ref<FlowCard[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const queryStatus = ref('');
const createVisible = ref(false);
const detailVisible = ref(false);
const detailRow = ref<FlowCard | null>(null);
const formRef = ref<FormInstance>();

const formData = reactive<{
  production_order_id: number | undefined;
  product_id: number | undefined;
  product_name: string;
  color_no: string;
  planned_fabric_weight: number | undefined;
  dyeing_requirements: string;
}>({
  production_order_id: undefined,
  product_id: undefined,
  product_name: '',
  color_no: '',
  planned_fabric_weight: undefined,
  dyeing_requirements: '',
});

const formRules: FormRules = {
  production_order_id: [
    {
      required: true,
      message: t('flowCards.index.ruleProductionOrderIdRequired'),
      trigger: 'blur',
    },
  ],
};

/** 响应解包防御：兼容数组 / { items } 分页包装，避免 el-table "r is not iterable" */
const unwrapList = (payload: unknown): FlowCard[] => {
  if (Array.isArray(payload)) return payload;
  const paged = payload as { items?: FlowCard[] } | null;
  return paged?.items ?? [];
};

const loadList = async () => {
  loading.value = true;
  try {
    const res = await getFlowCardList({
      page: page.value,
      page_size: pageSize.value,
      status: queryStatus.value || undefined,
    });
    cardList.value = unwrapList(res.data);
    total.value =
      res.data && !Array.isArray(res.data) ? (res.data.total ?? 0) : cardList.value.length;
  } catch {
    ElMessage.error(t('flowCards.index.messageFetchListFailed'));
  } finally {
    loading.value = false;
  }
};

const handleFilter = () => {
  page.value = 1;
  loadList();
};

const getNextActions = (row: FlowCard): StatusAction[] =>
  nextActionMap[row.status as FlowCardStatus] ?? [];

const runAction = async (action: StatusAction, row: FlowCard) => {
  try {
    if (action.key === 'terminate') {
      const { value } = await ElMessageBox.prompt(
        t('flowCards.index.messageTerminatePrompt', { no: row.card_no }),
        t('flowCards.index.titleTerminateConfirm'),
        {
          confirmButtonText: t('flowCards.index.buttonConfirmTerminate'),
          cancelButtonText: t('common.cancel'),
          inputPlaceholder: t('flowCards.index.placeholderTerminateReason'),
        }
      );
      await terminateFlowCard(row.id, { reason: value || undefined });
    } else if (action.key === 'complete-preparing') {
      const { value } = await ElMessageBox.prompt(
        t('flowCards.index.messageCompletePreparingPrompt', { no: row.card_no }),
        t('flowCards.index.buttonCompletePreparing'),
        {
          confirmButtonText: t('common.confirm'),
          cancelButtonText: t('common.cancel'),
          inputPlaceholder: t('flowCards.index.placeholderActualWeightInput'),
          inputPattern: /^\d+(\.\d{1,2})?$/,
          inputErrorMessage: t('flowCards.index.messageInvalidWeight'),
        }
      );
      await completePreparing(row.id, { actual_fabric_weight: Number(value) });
    } else {
      await ElMessageBox.confirm(
        t('flowCards.index.messageConfirmAction', { no: row.card_no, action: action.label }),
        t('flowCards.index.titleActionConfirm'),
        {
          confirmButtonText: t('common.confirm'),
          cancelButtonText: t('common.cancel'),
          type: 'warning',
        }
      );
      await actionRunners[action.key](row.id);
    }
    ElMessage.success(t('flowCards.index.messageOperationSuccess', { action: action.label }));
    await loadList();
  } catch (error) {
    if (error === 'cancel' || (error as { message?: string })?.message === 'cancel') return;
    ElMessage.error(t('flowCards.index.messageOperationFailed', { action: action.label }));
  }
};

const handleCreate = () => {
  createVisible.value = true;
};

const resetForm = () => {
  formData.production_order_id = undefined;
  formData.product_id = undefined;
  formData.product_name = '';
  formData.color_no = '';
  formData.planned_fabric_weight = undefined;
  formData.dyeing_requirements = '';
  formRef.value?.resetFields();
};

const handleDetail = (row: FlowCard) => {
  detailRow.value = row;
  detailVisible.value = true;
};

const submitCreate = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    if (!formData.production_order_id) return;
    submitLoading.value = true;
    try {
      await createFlowCard({
        production_order_id: formData.production_order_id,
        product_id: formData.product_id,
        product_name: formData.product_name || undefined,
        color_no: formData.color_no || undefined,
        planned_fabric_weight: formData.planned_fabric_weight,
        dyeing_requirements: formData.dyeing_requirements || undefined,
      });
      ElMessage.success(t('flowCards.index.messageCreateSuccess'));
      createVisible.value = false;
      await loadList();
    } catch {
      ElMessage.error(t('flowCards.index.messageCreateFailed'));
    } finally {
      submitLoading.value = false;
    }
  });
};

const handleDelete = async (row: FlowCard) => {
  try {
    await ElMessageBox.confirm(
      t('flowCards.index.messageDeleteConfirm', { no: row.card_no }),
      t('flowCards.index.titleDeleteConfirm'),
      {
        confirmButtonText: t('flowCards.index.buttonConfirmDelete'),
        cancelButtonText: t('common.cancel'),
        type: 'warning',
      }
    );
    await deleteFlowCard(row.id);
    ElMessage.success(t('common.message.deleteSuccess'));
    await loadList();
  } catch (error) {
    if (error === 'cancel' || (error as { message?: string })?.message === 'cancel') return;
    ElMessage.error(t('flowCards.index.messageDeleteFailed'));
  }
};

const canDelete = (row: FlowCard): boolean =>
  row.status === 'pending' || row.status === 'terminated';

onMounted(() => {
  loadList();
});
</script>

<style scoped>
.flow-cards-page {
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
