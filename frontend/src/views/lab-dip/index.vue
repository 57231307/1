<template>
  <div class="lab-dip-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>打样通知管理</h2>
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
          <el-button type="primary" @click="handleCreate">新建打样通知</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="requestList" border>
        <el-table-column prop="request_no" label="通知单号" min-width="180" />
        <el-table-column prop="customer_color_no" label="客户色号" width="130">
          <template #default="{ row }">{{ row.customer_color_no || '-' }}</template>
        </el-table-column>
        <el-table-column
          prop="customer_color_name"
          label="客户色名"
          min-width="130"
          show-overflow-tooltip
        >
          <template #default="{ row }">{{ row.customer_color_name || '-' }}</template>
        </el-table-column>
        <el-table-column prop="light_source" label="主光源" width="100" align="center" />
        <el-table-column prop="sample_versions" label="版数" width="70" align="center" />
        <el-table-column prop="required_date" label="要求交期" width="120" />
        <el-table-column prop="status" label="状态" width="110" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTagMap[row.status as LabDipRequestStatus] ?? 'info'">
              {{ statusTextMap[row.status as LabDipRequestStatus] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="280" fixed="right">
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

    <el-dialog v-model="createVisible" title="新建打样通知" width="580px" @close="resetForm">
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="110px">
        <el-form-item label="客户 ID" prop="customer_id">
          <el-input-number
            v-model="formData.customer_id"
            :min="1"
            :precision="0"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="客户色号" prop="customer_color_no">
          <el-input v-model="formData.customer_color_no" placeholder="选填" />
        </el-form-item>
        <el-form-item label="客户色名" prop="customer_color_name">
          <el-input v-model="formData.customer_color_name" placeholder="选填" />
        </el-form-item>
        <el-form-item label="主对色光源" prop="light_source">
          <el-select v-model="formData.light_source" placeholder="必填">
            <el-option v-for="ls in LIGHT_SOURCES" :key="ls" :label="ls" :value="ls" />
          </el-select>
        </el-form-item>
        <el-form-item label="坯布规格" prop="fabric_spec">
          <el-input v-model="formData.fabric_spec" placeholder="纱支/成分/组织，选填" />
        </el-form-item>
        <el-form-item label="染料类别" prop="dye_category">
          <el-select v-model="formData.dye_category" placeholder="选填" clearable>
            <el-option label="活性" value="reactive" />
            <el-option label="分散" value="disperse" />
            <el-option label="酸性" value="acid" />
            <el-option label="还原" value="vat" />
            <el-option label="硫化" value="sulfur" />
            <el-option label="直接" value="direct" />
          </el-select>
        </el-form-item>
        <el-form-item label="打样版数" prop="sample_versions">
          <el-input-number
            v-model="formData.sample_versions"
            :min="1"
            :max="8"
            :precision="0"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="要求交期" prop="required_date">
          <el-date-picker
            v-model="formData.required_date"
            type="date"
            value-format="YYYY-MM-DD"
            placeholder="必填"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="备注" prop="remarks">
          <el-input v-model="formData.remarks" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="submitCreate">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" title="打样通知详情" width="640px">
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="通知单号">{{ detailRow.request_no }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="statusTagMap[detailRow.status] ?? 'info'">
            {{ statusTextMap[detailRow.status] ?? detailRow.status }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="客户色号">{{
          detailRow.customer_color_no || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="客户色名">{{
          detailRow.customer_color_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="来样类型">{{
          detailRow.sample_type || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="主光源">{{ detailRow.light_source }}</el-descriptions-item>
        <el-descriptions-item label="副光源">{{
          detailRow.secondary_light_source || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="版数">{{ detailRow.sample_versions }}</el-descriptions-item>
        <el-descriptions-item label="坯布规格">{{
          detailRow.fabric_spec || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="纤维成分">{{
          detailRow.fabric_component || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="染料类别">{{
          detailRow.dye_category || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="要求交期">{{ detailRow.required_date }}</el-descriptions-item>
        <el-descriptions-item label="创建时间" :span="2">{{
          detailRow.created_at
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

const LIGHT_SOURCES = ['D65', 'TL84', 'U3000', 'CWF', 'A'];

const statusTextMap: Record<LabDipRequestStatus, string> = {
  pending: '待打样',
  sampling: '打样中',
  submitted: '已送客户',
  approved: '客户确认',
  rejected: '客户拒绝',
  completed: '已完成',
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
  pending: [{ key: 'start-sampling', label: '开始打样' }],
  sampling: [{ key: 'submit', label: '送客户确认' }],
  submitted: [
    { key: 'approve', label: '客户确认通过' },
    { key: 'reject', label: '拒绝重打', danger: true },
  ],
  rejected: [{ key: 'restart', label: '重新打样' }],
  approved: [{ key: 'complete', label: '完成打样' }],
};

const actionRunners: Record<string, (id: number) => Promise<unknown>> = {
  'start-sampling': (id: number) => startLabDipSampling(id),
  submit: (id: number) => submitLabDipRequest(id),
  approve: (id: number) => approveLabDipRequest(id),
  reject: (id: number) => rejectLabDipRequest(id),
  restart: (id: number) => restartLabDipSampling(id),
  complete: (id: number) => completeLabDipRequest(id),
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
  light_source: [{ required: true, message: '请选择主对色光源', trigger: 'change' }],
  required_date: [{ required: true, message: '请选择要求交期', trigger: 'change' }],
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
    ElMessage.error('加载打样通知列表失败');
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
    await ElMessageBox.confirm(
      `确认对打样通知 ${row.request_no} 执行「${action.label}」吗？`,
      '操作确认',
      { confirmButtonText: '确认', cancelButtonText: '取消', type: 'warning' }
    );
    await actionRunners[action.key](row.id);
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
      ElMessage.success('打样通知创建成功');
      createVisible.value = false;
      await loadList();
    } catch {
      ElMessage.error('打样通知创建失败');
    } finally {
      submitLoading.value = false;
    }
  });
};

const handleDelete = async (row: LabDipRequest) => {
  try {
    await ElMessageBox.confirm(`确认删除打样通知 ${row.request_no} 吗？`, '删除确认', {
      confirmButtonText: '确认删除',
      cancelButtonText: '取消',
      type: 'warning',
    });
    await deleteLabDipRequest(row.id);
    ElMessage.success('删除成功');
    await loadList();
  } catch (error) {
    if (error === 'cancel' || (error as { message?: string })?.message === 'cancel') return;
    ElMessage.error('删除失败（仅待打样状态可删除）');
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
