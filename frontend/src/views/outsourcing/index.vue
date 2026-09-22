<template>
  <div class="page">
    <el-card shadow="never">
      <el-tabs v-model="activeTab">
        <el-tab-pane label="委外单" name="orders">
          <div class="toolbar mb">
            <el-button type="primary" @click="openCreate">新建委外单</el-button>
          </div>
          <el-table v-loading="loading" :data="orders" border>
            <el-table-column prop="order_no" label="委外单号" width="150" />
            <el-table-column prop="order_type" label="类型" width="100" />
            <el-table-column label="状态" width="110">
              <template #default="{ row }">
                <el-tag :type="statusTag(row.status)">{{
                  OUTSOURCING_STATUS_LABEL[row.status] ?? row.status
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="supplier_id" label="供应商ID" width="100" />
            <el-table-column prop="issue_date" label="发出日期" width="120" />
            <el-table-column prop="expected_return_date" label="预计回厂" width="120" />
            <el-table-column prop="issue_quantity" label="发出数量" width="110" />
            <el-table-column label="操作" width="340" fixed="right">
              <template #default="{ row }">
                <el-button size="small" @click="openDetail(row)">详情</el-button>
                <el-button v-if="row.status === 'draft'" size="small" @click="openEdit(row)"
                  >编辑</el-button
                >
                <el-button
                  v-if="row.status === 'draft'"
                  size="small"
                  type="primary"
                  @click="onIssue(row)"
                  >发出</el-button
                >
                <el-button
                  v-if="row.status === 'issued'"
                  size="small"
                  type="primary"
                  @click="onProcess(row)"
                  >加工中</el-button
                >
                <el-button
                  v-if="row.status === 'processing'"
                  size="small"
                  type="success"
                  @click="onSettle(row)"
                  >结算</el-button
                >
                <el-button
                  v-if="row.status === 'settled'"
                  size="small"
                  type="success"
                  plain
                  @click="onClose(row)"
                  >关闭</el-button
                >
                <el-button
                  v-if="row.status === 'draft'"
                  size="small"
                  type="danger"
                  plain
                  @click="onCancel(row)"
                  >取消</el-button
                >
                <el-button
                  v-if="row.status === 'draft'"
                  size="small"
                  type="danger"
                  @click="onDelete(row)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <el-tab-pane label="收回单" name="receipts">
          <div class="toolbar mb">
            <el-button type="primary" @click="openCreateReceipt">新建收回单</el-button>
            <el-button plain @click="loadReceipts">刷新</el-button>
          </div>
          <el-table v-loading="receiptLoading" :data="receipts" border>
            <el-table-column prop="id" label="ID" width="70" />
            <el-table-column prop="receipt_no" label="收回单号" width="160" />
            <el-table-column prop="outsourcing_order_id" label="委外单ID" width="100" />
            <el-table-column prop="receipt_date" label="收回日期" width="120" />
            <el-table-column prop="product_id" label="产品ID" width="90" />
            <el-table-column prop="return_quantity" label="收回数量" width="110" />
            <el-table-column prop="loss_quantity" label="损耗数量" width="110" />
            <el-table-column label="质量状态" width="110">
              <template #default="{ row }">
                <el-tag :type="qualityTagType(row.quality_status)" size="small">
                  {{ qualityLabel(row.quality_status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="grade" label="等级" width="80" />
            <el-table-column label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="row.status === 'confirmed' ? 'success' : 'info'">{{
                  row.status
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="100" fixed="right">
              <template #default="{ row }">
                <el-button
                  v-if="row.status === 'draft'"
                  size="small"
                  type="success"
                  @click="onConfirmReceipt(row)"
                  >确认</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="editingId ? '编辑委外单' : '新建委外单'" width="560">
      <el-form :model="form" label-width="110px">
        <el-form-item v-if="!editingId" label="委外单号" required>
          <el-input v-model="form.order_no" readonly />
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
          <el-date-picker
            v-model="form.issue_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="预计回厂">
          <el-date-picker
            v-model="form.expected_return_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="发出数量" required>
          <el-input-number
            v-model="form.issue_quantity"
            :min="0.01"
            :precision="2"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="单位">
          <el-input v-model="form.issue_unit" placeholder="kg / m" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onSave">{{
          editingId ? '更新' : '保存'
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" title="委外单详情" width="820">
      <el-descriptions v-if="detailOrder" :column="2" border>
        <el-descriptions-item label="委外单号">{{ detailOrder.order_no }}</el-descriptions-item>
        <el-descriptions-item label="类型">{{ detailOrder.order_type }}</el-descriptions-item>
        <el-descriptions-item label="状态">{{
          OUTSOURCING_STATUS_LABEL[detailOrder.status] ?? detailOrder.status
        }}</el-descriptions-item>
        <el-descriptions-item label="供应商ID">{{ detailOrder.supplier_id }}</el-descriptions-item>
        <el-descriptions-item label="发出日期">{{ detailOrder.issue_date }}</el-descriptions-item>
        <el-descriptions-item label="预计回厂">{{
          detailOrder.expected_return_date || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="发出数量">{{
          detailOrder.issue_quantity
        }}</el-descriptions-item>
        <el-descriptions-item label="单位">{{
          detailOrder.issue_unit || '-'
        }}</el-descriptions-item>
      </el-descriptions>

      <div class="items-section">
        <div class="items-toolbar">
          <span class="items-title">发料明细</span>
          <el-button
            v-if="detailOrder && detailOrder.status === 'draft'"
            type="primary"
            size="small"
            @click="itemDialogVisible = true"
            >添加明细</el-button
          >
        </div>
        <el-table v-loading="itemLoading" :data="orderItems" border size="small" max-height="300">
          <el-table-column prop="id" label="ID" width="60" />
          <el-table-column prop="product_id" label="产品ID" width="90" />
          <el-table-column prop="color_no" label="色号" width="110" />
          <el-table-column prop="dye_lot_no" label="缸号" width="110" />
          <el-table-column prop="quantity" label="数量" width="100" />
          <el-table-column prop="unit" label="单位" width="80" />
          <el-table-column prop="unit_cost" label="单价" width="100" />
          <el-table-column prop="processing_fee" label="加工费" width="100" />
          <el-table-column prop="freight_fee" label="运费" width="100" />
        </el-table>
      </div>
      <template #footer>
        <el-button @click="detailVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="itemDialogVisible" title="添加发料明细" width="520">
      <el-form :model="itemForm" label-width="110px">
        <el-form-item label="产品ID" required>
          <el-input-number v-model="itemForm.product_id" :min="1" class="w-full" />
        </el-form-item>
        <el-form-item label="色号"><el-input v-model="itemForm.color_no" /></el-form-item>
        <el-form-item label="缸号"><el-input v-model="itemForm.dye_lot_no" /></el-form-item>
        <el-form-item label="数量" required>
          <el-input-number v-model="itemForm.quantity" :min="0.01" :precision="2" class="w-full" />
        </el-form-item>
        <el-form-item label="单位"
          ><el-input v-model="itemForm.unit" placeholder="kg / m"
        /></el-form-item>
        <el-form-item label="单价" required>
          <el-input-number v-model="itemForm.unit_cost" :min="0" :precision="2" class="w-full" />
        </el-form-item>
        <el-form-item label="加工费">
          <el-input-number
            v-model="itemForm.processing_fee"
            :min="0"
            :precision="2"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="运费">
          <el-input-number v-model="itemForm.freight_fee" :min="0" :precision="2" class="w-full" />
        </el-form-item>
        <el-form-item label="备注"
          ><el-input v-model="itemForm.remarks" type="textarea" :rows="2"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="itemDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="itemSaving" @click="onSaveItem">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="receiptDialogVisible" title="新建收回单" width="560">
      <el-form :model="receiptForm" label-width="110px">
        <el-form-item label="收回单号" required>
          <el-input v-model="receiptForm.receipt_no" readonly />
        </el-form-item>
        <el-form-item label="委外单ID" required>
          <el-input-number v-model="receiptForm.outsourcing_order_id" :min="1" class="w-full" />
        </el-form-item>
        <el-form-item label="收回日期" required>
          <el-date-picker
            v-model="receiptForm.receipt_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="产品ID" required>
          <el-input-number v-model="receiptForm.product_id" :min="1" class="w-full" />
        </el-form-item>
        <el-form-item label="色号"><el-input v-model="receiptForm.color_no" /></el-form-item>
        <el-form-item label="缸号"><el-input v-model="receiptForm.dye_lot_no" /></el-form-item>
        <el-form-item label="收回数量" required>
          <el-input-number
            v-model="receiptForm.return_quantity"
            :min="0.01"
            :precision="2"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="损耗数量">
          <el-input-number
            v-model="receiptForm.loss_quantity"
            :min="0"
            :precision="2"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="质量状态">
          <el-select v-model="receiptForm.quality_status" class="w-full">
            <el-option
              v-for="value in OUTSOURCING_QUALITY_FORM_VALUES"
              :key="value"
              :label="OUTSOURCING_QUALITY_STATUS_LABELS[value]"
              :value="value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="等级">
          <el-input v-model="receiptForm.grade" placeholder="A / B / C" />
        </el-form-item>
        <el-form-item label="备注"
          ><el-input v-model="receiptForm.remarks" type="textarea" :rows="2"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="receiptDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="receiptSaving" @click="onSaveReceipt">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { generateUniqueDocNo } from '@/utils/document-no';
import { logger } from '@/utils/logger';
import {
  OUTSOURCING_QUALITY_FORM_VALUES,
  OUTSOURCING_QUALITY_STATUS,
  OUTSOURCING_QUALITY_STATUS_LABELS,
  OUTSOURCING_QUALITY_STATUS_TAG_TYPE,
  OUTSOURCING_QUALITY_STATUS_VALUES,
  type OutsourcingQualityTagType,
} from '@/constants/outsourcing-quality';
import {
  cancelOutsourcingOrder,
  closeOutsourcingOrder,
  createOutsourcingOrder,
  getOutsourcingOrderList,
  issueOutsourcingOrder,
  processOutsourcingOrder,
  settleOutsourcingOrder,
  updateOutsourcingOrder,
  deleteOutsourcingOrder,
  getOutsourcingItems,
  createOutsourcingItem,
  getOutsourcingReceiptList,
  createOutsourcingReceipt,
  confirmOutsourcingReceipt,
  OUTSOURCING_STATUS_LABEL,
  type OutsourcingOrder,
} from '@/api/outsourcing';

/**
 * 收回质检结论展示：结论为空只可能出现在未归一的历史行上（v15 域内的归一语句 已把
 * NULL 归一为 pending），统一按「待检」显示；取值域外的值告警后原样显示。
 */
function qualityLabel(value: string | null | undefined): string {
  const raw = value ?? OUTSOURCING_QUALITY_STATUS.pending;
  const label = OUTSOURCING_QUALITY_STATUS_LABELS[raw];
  if (!label) {
    logger.warn(
      `[outsourcing] 未知收回质检结论「${raw}」，不在取值域 ${OUTSOURCING_QUALITY_STATUS_VALUES.join('/')} 内`
    );
    return raw;
  }
  return label;
}

function qualityTagType(value: string | null | undefined): OutsourcingQualityTagType {
  const raw = value ?? OUTSOURCING_QUALITY_STATUS.pending;
  return OUTSOURCING_QUALITY_STATUS_TAG_TYPE[raw] ?? 'info';
}

const activeTab = ref('orders');
const orders = ref<OutsourcingOrder[]>([]);
const loading = ref(false);
const saving = ref(false);
const dialogVisible = ref(false);
const editingId = ref<number | null>(null);

/** 打开新建委外单：自动预生成单据号（查重唯一后只读展示，防手动输入重复） */
const openCreate = async () => {
  editingId.value = null;
  form.order_no = await generateUniqueDocNo('OUT', 'outsourcing_order');
  dialogVisible.value = true;
};

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
  ({
    draft: 'info',
    issued: 'primary',
    processing: 'warning',
    settled: 'success',
    closed: 'info',
    cancelled: 'danger',
  })[s] ?? 'info';

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

// 编辑委外单（draft 态）
const openEdit = (row: OutsourcingOrder) => {
  editingId.value = row.id;
  Object.assign(form, {
    order_no: row.order_no,
    order_type: row.order_type || 'dyeing',
    supplier_id: row.supplier_id,
    issue_date: row.issue_date || '',
    expected_return_date: row.expected_return_date || '',
    issue_quantity: row.issue_quantity,
    issue_unit: row.issue_unit || '',
  });
  dialogVisible.value = true;
};

const onSave = async () => {
  if (editingId.value) {
    if (!form.supplier_id || !form.issue_date || !form.issue_quantity) {
      ElMessage.warning('请填写必填项：供应商/日期/数量');
      return;
    }
    saving.value = true;
    try {
      await updateOutsourcingOrder(editingId.value, {
        order_type: form.order_type,
        supplier_id: form.supplier_id,
        issue_date: form.issue_date,
        expected_return_date: form.expected_return_date || undefined,
        issue_quantity: form.issue_quantity,
        issue_unit: form.issue_unit || undefined,
      });
      ElMessage.success('委外单已更新');
      dialogVisible.value = false;
      editingId.value = null;
      await load();
    } finally {
      saving.value = false;
    }
  } else {
    editingId.value = null;
    await onCreate();
  }
};

// 删除委外单（draft 态）
const onDelete = async (row: OutsourcingOrder) => {
  try {
    await ElMessageBox.confirm(`确认删除委外单 ${row.order_no}？`, '确认', { type: 'warning' });
  } catch {
    return;
  }
  try {
    await deleteOutsourcingOrder(row.id);
    ElMessage.success('删除成功');
    await load();
  } catch (e) {
    ElMessage.error((e as Error).message || '删除失败');
  }
};

// 详情 + 发料明细
const detailVisible = ref(false);
const detailOrder = ref<OutsourcingOrder | null>(null);
const orderItems = ref<Array<Record<string, unknown>>>([]);
const itemLoading = ref(false);
const itemDialogVisible = ref(false);
const itemSaving = ref(false);
const itemForm = reactive({
  product_id: undefined as number | undefined,
  color_no: '',
  dye_lot_no: '',
  quantity: undefined as number | undefined,
  unit: '',
  unit_cost: 0,
  processing_fee: 0,
  freight_fee: 0,
  remarks: '',
});

const openDetail = async (row: OutsourcingOrder) => {
  detailOrder.value = row;
  detailVisible.value = true;
  itemLoading.value = true;
  try {
    const res = await getOutsourcingItems(row.id);
    orderItems.value = Array.isArray(res)
      ? res
      : ((res as { items?: Array<Record<string, unknown>> })?.items ?? []);
  } finally {
    itemLoading.value = false;
  }
};

const onSaveItem = async () => {
  if (!detailOrder.value || !itemForm.product_id || !itemForm.quantity) {
    ElMessage.warning('请填写必填项：产品/数量');
    return;
  }
  itemSaving.value = true;
  try {
    await createOutsourcingItem(detailOrder.value.id, {
      outsourcing_order_id: detailOrder.value.id,
      product_id: itemForm.product_id,
      color_no: itemForm.color_no || null,
      dye_lot_no: itemForm.dye_lot_no || null,
      quantity: itemForm.quantity,
      unit: itemForm.unit || null,
      unit_cost: itemForm.unit_cost,
      processing_fee: itemForm.processing_fee || null,
      freight_fee: itemForm.freight_fee || null,
      remarks: itemForm.remarks || null,
    });
    ElMessage.success('明细已添加');
    itemDialogVisible.value = false;
    await openDetail(detailOrder.value);
  } finally {
    itemSaving.value = false;
  }
};

// 收回单
const receipts = ref<Array<Record<string, unknown>>>([]);
const receiptLoading = ref(false);
const receiptDialogVisible = ref(false);
const receiptSaving = ref(false);
const receiptForm = reactive({
  receipt_no: '',
  outsourcing_order_id: undefined as number | undefined,
  receipt_date: '',
  product_id: undefined as number | undefined,
  color_no: '',
  dye_lot_no: '',
  return_quantity: undefined as number | undefined,
  loss_quantity: 0,
  quality_status: OUTSOURCING_QUALITY_STATUS.qualified,
  grade: '',
  remarks: '',
});

async function loadReceipts() {
  receiptLoading.value = true;
  try {
    const res = await getOutsourcingReceiptList();
    receipts.value = Array.isArray(res)
      ? res
      : ((res as { items?: Array<Record<string, unknown>> })?.items ?? []);
  } finally {
    receiptLoading.value = false;
  }
}

const openCreateReceipt = async () => {
  receiptForm.receipt_no = await generateUniqueDocNo('ORC', 'outsourcing_receipt');
  receiptDialogVisible.value = true;
};

const onSaveReceipt = async () => {
  if (
    !receiptForm.receipt_no ||
    !receiptForm.outsourcing_order_id ||
    !receiptForm.receipt_date ||
    !receiptForm.product_id ||
    !receiptForm.return_quantity
  ) {
    ElMessage.warning('请填写必填项：单号/委外单/日期/产品/数量');
    return;
  }
  receiptSaving.value = true;
  try {
    await createOutsourcingReceipt({
      receipt_no: receiptForm.receipt_no,
      outsourcing_order_id: receiptForm.outsourcing_order_id,
      receipt_date: receiptForm.receipt_date,
      product_id: receiptForm.product_id,
      color_no: receiptForm.color_no || null,
      dye_lot_no: receiptForm.dye_lot_no || null,
      return_quantity: receiptForm.return_quantity,
      loss_quantity: receiptForm.loss_quantity || null,
      quality_status: receiptForm.quality_status || null,
      grade: receiptForm.grade || null,
      remarks: receiptForm.remarks || null,
    });
    ElMessage.success('收回单已创建');
    receiptDialogVisible.value = false;
    await loadReceipts();
  } finally {
    receiptSaving.value = false;
  }
};

const onConfirmReceipt = async (row: Record<string, unknown>) => {
  try {
    await ElMessageBox.confirm(`确认收回单 #${row.id}？确认后触发入库与质检。`, '确认');
  } catch {
    return;
  }
  try {
    await confirmOutsourcingReceipt(row.id as number);
    ElMessage.success('收回单已确认');
    await loadReceipts();
  } catch (e) {
    ElMessage.error((e as Error).message || '确认失败');
  }
};

const act = async (
  row: OutsourcingOrder,
  fn: (id: number, d?: Record<string, unknown>) => Promise<unknown>,
  msg: string,
  prompt = false
) => {
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

onMounted(() => {
  load();
  loadReceipts();
});
</script>

<style scoped>
.w-full {
  width: 100%;
}
.mb {
  margin-bottom: 12px;
}
.toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
}
.items-section {
  margin-top: 16px;
}
.items-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.items-title {
  font-weight: 600;
  font-size: 14px;
}
</style>
