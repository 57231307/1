<template>
  <div class="bulk-color-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>大货批色审批</h2>
        <div class="header-actions">
          <el-button type="primary" @click="handleCreate">新建批色申请</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="list" border>
        <el-table-column prop="approval_no" label="申请编号" min-width="160" />
        <el-table-column prop="order_no" label="订单号" min-width="140">
          <template #default="{ row }">{{ row.order_no || '-' }}</template>
        </el-table-column>
        <el-table-column prop="color_no" label="色号" width="120">
          <template #default="{ row }">{{ row.color_no || '-' }}</template>
        </el-table-column>
        <el-table-column prop="dye_lot_no" label="缸号" width="130">
          <template #default="{ row }">{{ row.dye_lot_no || '-' }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="120" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTag(row.status)">{{ statusText(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="applicant_name" label="申请人" width="100" />
        <el-table-column label="操作" width="280" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link type="primary" @click="handleCut(row)">剪样</el-button>
            <el-button size="small" link type="primary" @click="handleSend(row)">送客户</el-button>
            <el-button size="small" link type="success" @click="handleApprove(row)"
              >批色通过</el-button
            >
            <el-button size="small" link type="warning" @click="handleRework(row)">回修</el-button>
            <el-button size="small" link type="danger" @click="handleReject(row)">拒绝</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" title="新建批色申请" width="520px">
      <el-form :model="form" label-width="90px">
        <el-form-item label="订单号" required>
          <el-input v-model="form.order_no" />
        </el-form-item>
        <el-form-item label="色号" required>
          <el-input v-model="form.color_no" />
        </el-form-item>
        <el-form-item label="缸号" required>
          <el-input v-model="form.dye_lot_no" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="form.notes" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="submitCreate">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  getBulkColorApprovalList,
  createBulkColorApproval,
  cutBulkColorSample,
  sendBulkColorToCustomer,
  approveBulkColor,
  rejectBulkColor,
  reworkBulkColor,
  type BulkColorApproval,
  type BulkColorApprovalStatus,
} from '@/api/bulk-color-approval';

const loading = ref(false);
const submitting = ref(false);
const dialogVisible = ref(false);
const list = ref<BulkColorApproval[]>([]);

const form = reactive({
  sales_order_id: 1,
  dye_batch_id: 1,
  customer_id: 1,
  order_no: '',
  color_no: '',
  dye_lot_no: '',
  notes: '',
});

const statusText = (s: string) =>
  ({
    draft: '草稿',
    pending: '待审批',
    cut: '已剪样',
    sent_to_customer: '已送客户',
    customer_approved: '客户批色通过',
    customer_rejected: '客户批色拒绝',
    approved: '已通过',
    rejected: '已拒绝',
    rework: '回修中',
    downgraded: '已降级',
    scrapped: '已报废',
    cancelled: '已取消',
  })[s] ?? s;
const statusTag = (s: BulkColorApprovalStatus | string) =>
  (
    ({
      approved: 'success',
      customer_approved: 'success',
      rejected: 'danger',
      customer_rejected: 'danger',
      scrapped: 'danger',
      rework: 'warning',
      downgraded: 'warning',
    }) as Record<string, string>
  )[s] ?? 'info';

const unwrap = (payload: unknown): BulkColorApproval[] => {
  const p = payload as unknown;
  return Array.isArray(p) ? p : ((p as { items?: BulkColorApproval[] })?.items ?? []);
};

const loadList = async () => {
  loading.value = true;
  try {
    const res = await getBulkColorApprovalList({ page: 1, page_size: 50 });
    list.value = unwrap(res.data);
  } catch (e) {
    ElMessage.error(`加载批色列表失败: ${(e as Error).message}`);
  } finally {
    loading.value = false;
  }
};

const handleCreate = () => {
  Object.assign(form, {
    sales_order_id: 1,
    dye_batch_id: 1,
    customer_id: 1,
    order_no: '',
    color_no: '',
    dye_lot_no: '',
    notes: '',
  });
  dialogVisible.value = true;
};

const submitCreate = async () => {
  if (!form.order_no || !form.color_no || !form.dye_lot_no) {
    ElMessage.warning('订单号/色号/缸号必填');
    return;
  }
  submitting.value = true;
  try {
    await createBulkColorApproval(form);
    ElMessage.success('批色申请已创建');
    dialogVisible.value = false;
    await loadList();
  } catch (e) {
    ElMessage.error(`创建失败: ${(e as Error).message}`);
  } finally {
    submitting.value = false;
  }
};

const runAction = async (_row: BulkColorApproval, label: string, fn: () => Promise<unknown>) => {
  try {
    await fn();
    ElMessage.success(`${label}成功`);
    await loadList();
  } catch (e) {
    ElMessage.error(`${label}失败: ${(e as Error).message}`);
  }
};

const handleCut = (row: BulkColorApproval) =>
  runAction(row, '剪样', () => cutBulkColorSample(row.id, { sample_length_m: 1 }));
const handleSend = (row: BulkColorApproval) =>
  runAction(row, '送客户', () => sendBulkColorToCustomer(row.id));
const handleApprove = (row: BulkColorApproval) =>
  runAction(row, '批色通过', () => approveBulkColor(row.id, { feedback: '客户确认批色通过' }));
const handleRework = (row: BulkColorApproval) =>
  runAction(row, '回修', () => reworkBulkColor(row.id, { reject_reason: '批色回修' }));
const handleReject = (row: BulkColorApproval) =>
  runAction(row, '拒绝', async () => {
    const { value } = await ElMessageBox.prompt('请输入拒绝原因', '批色拒绝', { inputValue: '' });
    await rejectBulkColor(row.id, { reject_reason: value || '客户拒绝' });
  });

onMounted(() => {
  loadList();
});
</script>

<style scoped>
.bulk-color-page {
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
