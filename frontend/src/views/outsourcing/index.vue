<template>
  <div class="page">
    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>委外管理</span>
          <el-button type="primary" @click="dialogVisible = true">新建委外单</el-button>
        </div>
      </template>
      <el-table v-loading="loading" :data="orders" border>
        <el-table-column prop="order_no" label="委外单号" width="150" />
        <el-table-column prop="order_type" label="类型" width="100" />
        <el-table-column label="状态" width="110">
          <template #default="{ row }">
            <el-tag :type="statusTag(row.status)">{{ OUTSOURCING_STATUS_LABEL[row.status] ?? row.status }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="supplier_id" label="供应商ID" width="100" />
        <el-table-column prop="issue_date" label="发出日期" width="120" />
        <el-table-column prop="expected_return_date" label="预计回厂" width="120" />
        <el-table-column prop="issue_quantity" label="发出数量" width="110" />
        <el-table-column label="操作" width="300" fixed="right">
          <template #default="{ row }">
            <el-button v-if="row.status === 'draft'" size="small" type="primary" @click="onIssue(row)">发出</el-button>
            <el-button v-if="row.status === 'issued'" size="small" type="primary" @click="onProcess(row)">加工中</el-button>
            <el-button v-if="row.status === 'processing'" size="small" type="success" @click="onSettle(row)">结算</el-button>
            <el-button v-if="row.status === 'settled'" size="small" type="success" plain @click="onClose(row)">关闭</el-button>
            <el-button v-if="row.status === 'draft'" size="small" type="danger" plain @click="onCancel(row)">取消</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" title="新建委外单" width="560">
      <el-form :model="form" label-width="110px">
        <el-form-item label="委外单号" required>
          <el-input v-model="form.order_no" placeholder="OUT-YYYYMMDD-XXX" />
        </el-form-item>
        <el-form-item label="类型" required>
          <el-select v-model="form.order_type" class="w-full">
            <el-option label="染色加工" value="dyeing" />
            <el-option label="后整理" value="finishing" />
            <el-option label="其他" value="other" />
          </el-select>
        </el-form-item>
        <el-form-item label="供应商ID" required>
          <el-input-number v-model="form.supplier_id" :min="1" class="w-full" />
        </el-form-item>
        <el-form-item label="发出日期" required>
          <el-date-picker v-model="form.issue_date" type="date" value-format="YYYY-MM-DD" class="w-full" />
        </el-form-item>
        <el-form-item label="预计回厂">
          <el-date-picker v-model="form.expected_return_date" type="date" value-format="YYYY-MM-DD" class="w-full" />
        </el-form-item>
        <el-form-item label="发出数量" required>
          <el-input-number v-model="form.issue_quantity" :min="0.01" :precision="2" class="w-full" />
        </el-form-item>
        <el-form-item label="单位">
          <el-input v-model="form.issue_unit" placeholder="kg / m" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onCreate">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  cancelOutsourcingOrder,
  closeOutsourcingOrder,
  createOutsourcingOrder,
  getOutsourcingOrderList,
  issueOutsourcingOrder,
  processOutsourcingOrder,
  settleOutsourcingOrder,
  OUTSOURCING_STATUS_LABEL,
  type OutsourcingOrder,
} from '@/api/outsourcing';

const orders = ref<OutsourcingOrder[]>([]);
const loading = ref(false);
const saving = ref(false);
const dialogVisible = ref(false);

const form = reactive({
  order_no: '',
  order_type: 'dyeing',
  supplier_id: undefined as number | undefined,
  issue_date: '',
  expected_return_date: '',
  issue_quantity: undefined as number | undefined,
  issue_unit: '',
});

const unwrapList = (p: unknown): OutsourcingOrder[] =>
  Array.isArray(p) ? p : ((p as { items?: OutsourcingOrder[] })?.items ?? []);

const statusTag = (s: string) =>
  ({ draft: 'info', issued: 'primary', processing: 'warning', settled: 'success', closed: 'info', cancelled: 'danger' })[s] ?? 'info';

async function load() {
  loading.value = true;
  try {
    orders.value = unwrapList(await getOutsourcingOrderList());
  } finally {
    loading.value = false;
  }
}

async function onCreate() {
  if (!form.order_no || !form.supplier_id || !form.issue_date || !form.issue_quantity) {
    ElMessage.warning('请填写必填项：单号/供应商/日期/数量');
    return;
  }
  saving.value = true;
  try {
    await createOutsourcingOrder({
      order_no: form.order_no,
      order_type: form.order_type,
      supplier_id: form.supplier_id,
      issue_date: form.issue_date,
      expected_return_date: form.expected_return_date || undefined,
      issue_quantity: form.issue_quantity,
      issue_unit: form.issue_unit || undefined,
    });
    ElMessage.success('委外单已创建');
    dialogVisible.value = false;
    form.order_no = '';
    form.issue_quantity = undefined;
    await load();
  } finally {
    saving.value = false;
  }
}

const act = async (row: OutsourcingOrder, fn: (id: number, d?: Record<string, unknown>) => Promise<unknown>, msg: string, prompt = false) => {
  if (prompt) await ElMessageBox.confirm(`确认${msg}？`, '确认');
  await fn(row.id);
  ElMessage.success(msg + '成功');
  await load();
};

const onIssue = (row: OutsourcingOrder) => act(row, issueOutsourcingOrder, '发出', true);
const onProcess = (row: OutsourcingOrder) => act(row, processOutsourcingOrder, '开始加工', true);
const onSettle = (row: OutsourcingOrder) => act(row, settleOutsourcingOrder, '结算', true);
const onClose = (row: OutsourcingOrder) => act(row, closeOutsourcingOrder, '关闭', true);
const onCancel = (row: OutsourcingOrder) => act(row, cancelOutsourcingOrder, '取消', true);

onMounted(load);
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.w-full {
  width: 100%;
}
</style>
