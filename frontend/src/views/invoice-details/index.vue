<template>
  <div class="invoice-details-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>发票审批</h2>
        <div class="header-actions">
          <el-button type="primary" @click="handleCreate">新建发票</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="invoiceList" border>
        <el-table-column prop="invoice_no" label="发票号" min-width="170" />
        <el-table-column prop="order_id" label="关联订单" width="100" align="center">
          <template #default="{ row }">{{ row.order_id ?? '-' }}</template>
        </el-table-column>
        <el-table-column prop="amount" label="发票金额" width="130" align="right">
          <template #default="{ row }">{{ row.amount ?? '-' }}</template>
        </el-table-column>
        <el-table-column prop="tax_amount" label="税额" width="120" align="right">
          <template #default="{ row }">{{ row.tax_amount ?? '-' }}</template>
        </el-table-column>
        <el-table-column prop="total_amount" label="价税合计" width="130" align="right">
          <template #default="{ row }">{{ row.total_amount ?? '-' }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTagMap[row.status as FinanceInvoiceStatus] ?? 'info'">
              {{ statusTextMap[row.status as FinanceInvoiceStatus] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="invoice_date" label="开票日期" min-width="160">
          <template #default="{ row }">{{ formatDate(row.invoice_date) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="220" fixed="right">
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
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-drawer v-model="detailVisible" title="发票详情" size="480px">
      <el-descriptions v-if="detailRow" :column="1" border>
        <el-descriptions-item label="发票号">{{ detailRow.invoice_no }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="statusTagMap[detailRow.status] ?? 'info'">
            {{ statusTextMap[detailRow.status] ?? detailRow.status }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="关联订单 ID">{{
          detailRow.order_id ?? '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="发票金额">{{ detailRow.amount }}</el-descriptions-item>
        <el-descriptions-item label="税额">{{ detailRow.tax_amount }}</el-descriptions-item>
        <el-descriptions-item label="价税合计">{{ detailRow.total_amount }}</el-descriptions-item>
        <el-descriptions-item label="开票日期">{{
          formatDate(detailRow.invoice_date)
        }}</el-descriptions-item>
        <el-descriptions-item label="付款日期">
          {{ detailRow.paid_date ? formatDate(detailRow.paid_date) : '-' }}
        </el-descriptions-item>
        <el-descriptions-item label="付款方式">{{
          detailRow.payment_method || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="备注">{{ detailRow.notes || '-' }}</el-descriptions-item>
        <el-descriptions-item label="创建时间">{{
          formatDate(detailRow.created_at)
        }}</el-descriptions-item>
        <el-descriptions-item label="更新时间">{{
          formatDate(detailRow.updated_at)
        }}</el-descriptions-item>
      </el-descriptions>
    </el-drawer>

    <el-dialog v-model="createVisible" title="新建发票" width="520px" @close="resetForm">
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-form-item label="发票号" prop="invoice_no">
          <el-input v-model="formData.invoice_no" readonly />
        </el-form-item>
        <el-form-item label="发票金额" prop="amount">
          <el-input-number
            v-model="formData.amount"
            :min="0"
            :precision="2"
            style="width: 100%"
            placeholder="必填"
          />
        </el-form-item>
        <el-form-item label="税额" prop="tax_amount">
          <el-input-number
            v-model="formData.tax_amount"
            :min="0"
            :precision="2"
            style="width: 100%"
            placeholder="必填"
          />
        </el-form-item>
        <el-form-item label="价税合计" prop="total_amount">
          <el-input-number
            v-model="formData.total_amount"
            :min="0"
            :precision="2"
            style="width: 100%"
            placeholder="必填"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="submitCreate">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { generateUniqueDocNo } from '@/utils/document-no';
import {
  getFinanceInvoiceList,
  getFinanceInvoice,
  createFinanceInvoice,
  updateFinanceInvoice,
  approveFinanceInvoice,
  verifyFinanceInvoice,
  type FinanceInvoice,
  type FinanceInvoiceStatus,
} from '@/api/invoice-detail';

const statusTextMap: Record<FinanceInvoiceStatus, string> = {
  pending: '待审批',
  approved: '已审批',
  verified: '已核销',
  rejected: '已驳回',
};

const statusTagMap: Record<
  FinanceInvoiceStatus,
  'info' | 'warning' | 'primary' | 'success' | 'danger'
> = {
  pending: 'warning',
  approved: 'primary',
  verified: 'success',
  rejected: 'danger',
};

interface StatusAction {
  key: 'approve' | 'reject' | 'verify';
  label: string;
  danger?: boolean;
}

/** 状态链：pending 审批/驳回 → approved 核销 → verified */
const nextActionMap: Record<FinanceInvoiceStatus, StatusAction[]> = {
  pending: [
    { key: 'approve', label: '审批' },
    { key: 'reject', label: '驳回', danger: true },
  ],
  approved: [{ key: 'verify', label: '核销' }],
  verified: [],
  rejected: [],
};

const loading = ref(false);
const submitLoading = ref(false);
const invoiceList = ref<FinanceInvoice[]>([]);
const createVisible = ref(false);
const detailVisible = ref(false);
const detailRow = ref<FinanceInvoice | null>(null);
const formRef = ref<FormInstance>();

const formData = reactive<{
  invoice_no: string;
  amount: number | undefined;
  tax_amount: number | undefined;
  total_amount: number | undefined;
}>({
  invoice_no: '',
  amount: undefined,
  tax_amount: undefined,
  total_amount: undefined,
});

const formRules: FormRules = {
  invoice_no: [{ required: true, message: '请输入发票号', trigger: 'blur' }],
  amount: [{ required: true, message: '请输入发票金额', trigger: 'blur' }],
  tax_amount: [{ required: true, message: '请输入税额', trigger: 'blur' }],
  total_amount: [{ required: true, message: '请输入价税合计', trigger: 'blur' }],
};

const formatDate = (value: string | null): string => {
  if (!value) return '-';
  return value.replace('T', ' ').slice(0, 19);
};

const unwrapList = (payload: unknown): FinanceInvoice[] => {
  if (Array.isArray(payload)) return payload;
  const paged = payload as { invoices?: FinanceInvoice[]; items?: FinanceInvoice[] } | null;
  return paged?.invoices ?? paged?.items ?? [];
};

const loadList = async () => {
  loading.value = true;
  try {
    const res = await getFinanceInvoiceList();
    invoiceList.value = unwrapList(res.data);
  } catch {
    ElMessage.error('加载发票列表失败');
  } finally {
    loading.value = false;
  }
};

const getNextActions = (row: FinanceInvoice): StatusAction[] =>
  nextActionMap[row.status as FinanceInvoiceStatus] ?? [];

const runAction = async (action: StatusAction, row: FinanceInvoice) => {
  try {
    if (action.key === 'approve') {
      await ElMessageBox.confirm(`确认审批通过发票 ${row.invoice_no} 吗？`, '审批确认', {
        confirmButtonText: '通过',
        cancelButtonText: '取消',
        type: 'warning',
      });
      await approveFinanceInvoice(row.id);
    } else if (action.key === 'reject') {
      const { value } = await ElMessageBox.prompt(
        `确认驳回发票 ${row.invoice_no} 吗？可填写驳回原因。`,
        '驳回确认',
        {
          confirmButtonText: '确认驳回',
          cancelButtonText: '取消',
          inputPlaceholder: '驳回原因（选填）',
        }
      );
      await updateFinanceInvoice(row.id, { status: 'rejected', notes: value || undefined });
    } else {
      await ElMessageBox.confirm(`确认核销发票 ${row.invoice_no} 吗？`, '核销确认', {
        confirmButtonText: '确认核销',
        cancelButtonText: '取消',
        type: 'warning',
      });
      await verifyFinanceInvoice(row.id);
    }
    ElMessage.success(`操作成功：${action.label}`);
    await loadList();
  } catch (error) {
    if (error === 'cancel' || (error as { message?: string })?.message === 'cancel') return;
    ElMessage.error(`操作失败：${action.label}`);
  }
};

const handleCreate = async () => {
  resetForm();
  // 新建时预生成发票号（查重唯一后只读展示，防手动输入重复）
  formData.invoice_no = await generateUniqueDocNo('INV', 'finance_invoice');
  createVisible.value = true;
};

const resetForm = () => {
  formData.invoice_no = '';
  formData.amount = undefined;
  formData.tax_amount = undefined;
  formData.total_amount = undefined;
  formRef.value?.resetFields();
};

/** 详情抽屉：按行内数据展示，并回源 GET /finance/invoices/{id} 保证最新 */
const handleDetail = async (row: FinanceInvoice) => {
  detailRow.value = row;
  detailVisible.value = true;
  try {
    const res = await getFinanceInvoice(row.id);
    if (res.data) detailRow.value = res.data;
  } catch {
    ElMessage.error('加载发票详情失败');
  }
};

const submitCreate = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    if (
      formData.amount === undefined ||
      formData.tax_amount === undefined ||
      formData.total_amount === undefined
    ) {
      return;
    }
    submitLoading.value = true;
    try {
      await createFinanceInvoice({
        invoice_no: formData.invoice_no,
        amount: formData.amount,
        tax_amount: formData.tax_amount,
        total_amount: formData.total_amount,
      });
      ElMessage.success('发票创建成功');
      createVisible.value = false;
      await loadList();
    } catch {
      ElMessage.error('发票创建失败');
    } finally {
      submitLoading.value = false;
    }
  });
};

onMounted(() => {
  loadList();
});
</script>

<style scoped>
.invoice-details-page {
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
