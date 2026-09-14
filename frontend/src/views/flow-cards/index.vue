<template>
  <div class="flow-cards-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>流转卡管理</h2>
        <div class="header-actions">
          <el-select
            v-model="queryStatus"
            placeholder="全部状态"
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
          <el-button type="primary" @click="handleCreate">新建流转卡</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="cardList" border>
        <el-table-column prop="card_no" label="流转卡号" min-width="180" />
        <el-table-column prop="production_order_id" label="生产订单" width="100" align="center" />
        <el-table-column prop="product_name" label="产品" min-width="120" show-overflow-tooltip>
          <template #default="{ row }">{{ row.product_name || '-' }}</template>
        </el-table-column>
        <el-table-column prop="color_no" label="色号" width="120">
          <template #default="{ row }">{{ row.color_no || '-' }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="110" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTagMap[row.status as FlowCardStatus] ?? 'info'">
              {{ statusTextMap[row.status as FlowCardStatus] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="planned_fabric_weight"
          label="计划重量(kg)"
          width="120"
          align="right"
        >
          <template #default="{ row }">{{ row.planned_fabric_weight ?? '-' }}</template>
        </el-table-column>
        <el-table-column prop="dye_lot_no" label="缸号" width="130">
          <template #default="{ row }">{{ row.dye_lot_no || '-' }}</template>
        </el-table-column>
        <el-table-column label="操作" width="260" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link type="primary" @click="handleDetail(row)">详情</el-button>
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
              删除
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

    <el-dialog v-model="createVisible" title="新建流转卡" width="560px" @close="resetForm">
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="110px">
        <el-form-item label="生产订单 ID" prop="production_order_id">
          <el-input-number
            v-model="formData.production_order_id"
            :min="1"
            :precision="0"
            style="width: 100%"
            placeholder="必填"
          />
        </el-form-item>
        <el-form-item label="产品 ID" prop="product_id">
          <el-input-number
            v-model="formData.product_id"
            :min="1"
            :precision="0"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="产品名称" prop="product_name">
          <el-input v-model="formData.product_name" placeholder="选填" />
        </el-form-item>
        <el-form-item label="色号" prop="color_no">
          <el-input v-model="formData.color_no" placeholder="选填" />
        </el-form-item>
        <el-form-item label="计划重量(kg)" prop="planned_fabric_weight">
          <el-input-number
            v-model="formData.planned_fabric_weight"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="染整要求" prop="dyeing_requirements">
          <el-input v-model="formData.dyeing_requirements" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="submitCreate">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" title="流转卡详情" width="640px">
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="流转卡号">{{ detailRow.card_no }}</el-descriptions-item>
        <el-descriptions-item label="条码">{{ detailRow.barcode }}</el-descriptions-item>
        <el-descriptions-item label="生产订单 ID">{{
          detailRow.production_order_id
        }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="statusTagMap[detailRow.status] ?? 'info'">
            {{ statusTextMap[detailRow.status] ?? detailRow.status }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="产品">{{
          detailRow.product_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="色号">{{ detailRow.color_no || '-' }}</el-descriptions-item>
        <el-descriptions-item label="客户">{{
          detailRow.customer_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="缸号">{{ detailRow.dye_lot_no || '-' }}</el-descriptions-item>
        <el-descriptions-item label="计划重量(kg)">{{
          detailRow.planned_fabric_weight ?? '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="实际重量(kg)">{{
          detailRow.actual_fabric_weight ?? '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="当前工序序号">{{
          detailRow.current_step_seq
        }}</el-descriptions-item>
        <el-descriptions-item label="创建时间">{{ detailRow.created_at }}</el-descriptions-item>
        <el-descriptions-item label="染整要求" :span="2">{{
          detailRow.dyeing_requirements || '-'
        }}</el-descriptions-item>
      </el-descriptions>
      <template #footer>
        <el-button @click="detailVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
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

const statusTextMap: Record<FlowCardStatus, string> = {
  pending: '待排缸',
  scheduled: '已排缸',
  preparing: '备布中',
  dyeing: '染色中',
  dyed: '已出缸',
  inspecting: '验布中',
  completed: '已完成',
  shipped: '已发货',
  terminated: '已终止',
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
    { key: 'schedule', label: '排产' },
    { key: 'terminate', label: '终止', danger: true },
  ],
  scheduled: [
    { key: 'start-preparing', label: '开工准备' },
    { key: 'terminate', label: '终止', danger: true },
  ],
  preparing: [
    { key: 'start-dyeing', label: '开始染色' },
    { key: 'terminate', label: '终止', danger: true },
  ],
  dyeing: [
    { key: 'complete-dyeing', label: '完成染色' },
    { key: 'terminate', label: '终止', danger: true },
  ],
  dyed: [{ key: 'start-inspecting', label: '开始检验' }],
  inspecting: [{ key: 'complete', label: '完成验布' }],
  completed: [{ key: 'ship', label: '发货' }],
  terminated: [{ key: 'reactivate', label: '重新激活' }],
};

const actionRunners: Record<string, (id: number) => Promise<unknown>> = {
  schedule: (id: number) => scheduleFlowCard(id),
  'start-preparing': (id: number) => startPreparing(id),
  'complete-preparing': (id: number) => completePreparing(id),
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
  production_order_id: [{ required: true, message: '请输入生产订单 ID', trigger: 'blur' }],
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
    ElMessage.error('加载流转卡列表失败');
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
        `确认终止流转卡 ${row.card_no} 吗？可填写终止原因。`,
        '终止确认',
        {
          confirmButtonText: '确认终止',
          cancelButtonText: '取消',
          inputPlaceholder: '终止原因（选填）',
        }
      );
      await terminateFlowCard(row.id, { reason: value || undefined });
    } else {
      await ElMessageBox.confirm(
        `确认对流转卡 ${row.card_no} 执行「${action.label}」吗？`,
        '操作确认',
        {
          confirmButtonText: '确认',
          cancelButtonText: '取消',
          type: 'warning',
        }
      );
      await actionRunners[action.key](row.id);
    }
    ElMessage.success(`操作成功：${action.label}`);
    await loadList();
  } catch (error) {
    if (error === 'cancel' || (error as { message?: string })?.message === 'cancel') return;
    ElMessage.error(`操作失败：${action.label}`);
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
      ElMessage.success('流转卡创建成功');
      createVisible.value = false;
      await loadList();
    } catch {
      ElMessage.error('流转卡创建失败');
    } finally {
      submitLoading.value = false;
    }
  });
};

const handleDelete = async (row: FlowCard) => {
  try {
    await ElMessageBox.confirm(`确认删除流转卡 ${row.card_no} 吗？`, '删除确认', {
      confirmButtonText: '确认删除',
      cancelButtonText: '取消',
      type: 'warning',
    });
    await deleteFlowCard(row.id);
    ElMessage.success('删除成功');
    await loadList();
  } catch (error) {
    if (error === 'cancel' || (error as { message?: string })?.message === 'cancel') return;
    ElMessage.error('删除失败');
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
