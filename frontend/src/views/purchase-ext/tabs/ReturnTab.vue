<!--
  ReturnTab.vue - 采购退货 Tab
  来源：原 purchase-ext/index.vue 中 采购退货 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="return-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('purchaseExt.returnTab.title') }}</h2>
      <el-button type="primary" @click="openReturnDialog()">
        <el-icon><Plus /></el-icon> {{ t('purchaseExt.returnTab.create') }}
      </el-button>
    </div>
    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="returnQuery"
        :aria-label="t('purchaseExt.returnTab.filterAria')"
      >
        <el-form-item :label="t('purchaseExt.returnTab.returnNo')">
          <el-input
            v-model="returnQuery.returnNo"
            :placeholder="t('purchaseExt.returnTab.returnNoPlaceholder')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('purchaseExt.returnTab.supplier')">
          <el-input
            v-model="returnQuery.supplierName"
            :placeholder="t('purchaseExt.returnTab.supplierNamePlaceholder')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('purchaseExt.returnTab.status')">
          <el-select
            v-model="returnQuery.status"
            :placeholder="t('purchaseExt.returnTab.statusPlaceholder')"
            clearable
          >
            <el-option :label="t('purchaseExt.returnTab.statusDraft')" value="draft" />
            <el-option :label="t('purchaseExt.returnTab.statusPending')" value="pending" />
            <el-option :label="t('purchaseExt.returnTab.statusApproved')" value="approved" />
            <el-option :label="t('purchaseExt.returnTab.statusRejected')" value="rejected" />
            <el-option :label="t('purchaseExt.returnTab.statusCompleted')" value="completed" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchPurchaseReturns">{{
            t('purchaseExt.returnTab.query')
          }}</el-button>
          <el-button @click="resetReturnQuery">{{ t('purchaseExt.returnTab.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>
    <el-card shadow="hover">
      <el-table
        v-loading="returnLoading"
        :data="purchaseReturns"
        stripe
        :aria-label="t('purchaseExt.returnTab.listAria')"
      >
        <el-table-column
          prop="returnNo"
          :label="t('purchaseExt.returnTab.colReturnNo')"
          width="140"
        />
        <el-table-column
          prop="supplierName"
          :label="t('purchaseExt.returnTab.colSupplier')"
          min-width="150"
        />
        <el-table-column
          prop="purchaseOrderNo"
          :label="t('purchaseExt.returnTab.colOrderNo')"
          width="140"
        />
        <el-table-column
          prop="returnDate"
          :label="t('purchaseExt.returnTab.colReturnDate')"
          width="120"
        />
        <el-table-column
          prop="totalAmount"
          :label="t('purchaseExt.returnTab.colTotalAmount')"
          width="120"
          align="right"
        >
          <template #default="{ row }">
            {{ formatMoney(row.totalAmount) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="status"
          :label="t('purchaseExt.returnTab.colStatus')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getReturnStatusType(row.status)" size="small">
              {{ getReturnStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="createdBy"
          :label="t('purchaseExt.returnTab.colCreator')"
          width="100"
        />
        <el-table-column :label="t('purchaseExt.returnTab.colOperation')" width="180" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link @click="viewReturn(row as unknown as PurchaseReturn)">{{
              t('purchaseExt.returnTab.view')
            }}</el-button>
            <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
            <el-button
              v-if="row.status === 'draft'"
              v-permission="'purchase_return:update'"
              size="small"
              link
              @click="openReturnDialog(row as unknown as PurchaseReturn)"
              >{{ t('purchaseExt.returnTab.edit') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 退货编辑对话框 -->
    <el-dialog
      v-model="returnDialogVisible"
      :title="
        returnForm.id
          ? t('purchaseExt.returnTab.editTitle')
          : t('purchaseExt.returnTab.createTitle')
      "
      width="800px"
      :aria-label="t('purchaseExt.returnTab.dialogAria')"
    >
      <el-form
        ref="returnFormRef"
        :model="returnForm"
        :rules="returnRules"
        label-width="100px"
        :aria-label="t('purchaseExt.returnTab.formAria')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('purchaseExt.returnTab.returnNo')" prop="returnNo">
              <el-input v-model="returnForm.returnNo" :disabled="!!returnForm.id" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('purchaseExt.returnTab.supplierName')" prop="supplierName">
              <el-input
                v-model="returnForm.supplierName"
                :placeholder="t('purchaseExt.returnTab.supplierNamePlaceholder')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('purchaseExt.returnTab.orderNo')" prop="purchaseOrderNo">
              <el-input
                v-model="returnForm.purchaseOrderNo"
                :placeholder="t('purchaseExt.returnTab.orderNoPlaceholder')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('purchaseExt.returnTab.returnDate')" prop="returnDate">
              <el-date-picker
                v-model="returnForm.returnDate"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="t('purchaseExt.returnTab.reason')" prop="reason">
          <el-input v-model="returnForm.reason" type="textarea" />
        </el-form-item>
        <el-divider>{{ t('purchaseExt.returnTab.detailDivider') }}</el-divider>
        <el-table
          :data="returnForm.items"
          border
          style="width: 100%"
          :aria-label="t('purchaseExt.returnTab.detailEditAria')"
        >
          <el-table-column
            prop="productName"
            :label="t('purchaseExt.returnTab.colProductName')"
            min-width="150"
          >
            <template #default="{ row }">
              <el-input
                v-model="row.productName"
                :placeholder="t('purchaseExt.returnTab.productNamePlaceholder')"
              />
            </template>
          </el-table-column>
          <el-table-column
            prop="productCode"
            :label="t('purchaseExt.returnTab.colProductCode')"
            width="120"
          >
            <template #default="{ row }">
              <el-input
                v-model="row.productCode"
                :placeholder="t('purchaseExt.returnTab.productCodePlaceholder')"
              />
            </template>
          </el-table-column>
          <el-table-column
            prop="quantity"
            :label="t('purchaseExt.returnTab.colQuantity')"
            width="100"
          >
            <template #default="{ row }">
              <el-input-number v-model="row.quantity" :min="0" style="width: 100%" />
            </template>
          </el-table-column>
          <el-table-column prop="unit" :label="t('purchaseExt.returnTab.colUnit')" width="80">
            <template #default="{ row }">
              <el-input
                v-model="row.unit"
                :placeholder="t('purchaseExt.returnTab.unitPlaceholder')"
              />
            </template>
          </el-table-column>
          <el-table-column prop="price" :label="t('purchaseExt.returnTab.colPrice')" width="100">
            <template #default="{ row }">
              <el-input-number v-model="row.price" :min="0" :precision="2" style="width: 100%" />
            </template>
          </el-table-column>
          <el-table-column prop="amount" :label="t('purchaseExt.returnTab.colAmount')" width="100">
            <template #default="{ row }">
              {{ formatMoney((row.quantity || 0) * (row.price || 0)) }}
            </template>
          </el-table-column>
          <el-table-column
            prop="reason"
            :label="t('purchaseExt.returnTab.colReason')"
            min-width="120"
          >
            <template #default="{ row }">
              <el-input
                v-model="row.reason"
                :placeholder="t('purchaseExt.returnTab.reasonPlaceholder')"
              />
            </template>
          </el-table-column>
          <el-table-column :label="t('purchaseExt.returnTab.colOperation')" width="80">
            <template #default="{ $index }">
              <el-button size="small" link type="danger" @click="removeReturnItem($index)">{{
                t('purchaseExt.returnTab.delete')
              }}</el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-button type="primary" link style="margin-top: 8px" @click="addReturnItem">{{
          t('purchaseExt.returnTab.addProduct')
        }}</el-button>
      </el-form>
      <template #footer>
        <el-button @click="returnDialogVisible = false">{{
          t('purchaseExt.returnTab.cancelBtn')
        }}</el-button>
        <el-button type="primary" :loading="returnSubmitLoading" @click="submitReturn">{{
          t('purchaseExt.returnTab.confirmBtn')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 退货详情对话框 -->
    <el-dialog
      v-model="returnViewVisible"
      :title="t('purchaseExt.returnTab.viewTitle')"
      width="800px"
      :aria-label="t('purchaseExt.returnTab.viewDialogAria')"
    >
      <el-descriptions :column="2" border :aria-label="t('purchaseExt.returnTab.viewDetailAria')">
        <el-descriptions-item :label="t('purchaseExt.returnTab.returnNo')">{{
          currentReturn?.returnNo
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseExt.returnTab.colSupplier')">{{
          currentReturn?.supplierName
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseExt.returnTab.relatedOrder')">{{
          currentReturn?.purchaseOrderNo
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseExt.returnTab.returnDate')">{{
          currentReturn?.returnDate
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseExt.returnTab.colTotalAmount')">{{
          formatMoney(currentReturn?.totalAmount || 0)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseExt.returnTab.colStatus')">
          <el-tag :type="getReturnStatusType(currentReturn?.status)">
            {{ getReturnStatusLabel(currentReturn?.status) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('purchaseExt.returnTab.colCreator')">{{
          currentReturn?.createdBy
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseExt.returnTab.approver')">{{
          currentReturn?.approved_by
        }}</el-descriptions-item>
      </el-descriptions>
      <el-divider>{{ t('purchaseExt.returnTab.reasonDivider') }}</el-divider>
      <p>{{ currentReturn?.reason }}</p>
      <el-divider>{{ t('purchaseExt.returnTab.detailDivider') }}</el-divider>
      <el-table
        :data="currentReturn?.items || []"
        stripe
        :aria-label="t('purchaseExt.returnTab.detailListAria')"
      >
        <el-table-column
          prop="productName"
          :label="t('purchaseExt.returnTab.colProductName')"
          min-width="150"
        />
        <el-table-column
          prop="productCode"
          :label="t('purchaseExt.returnTab.colProductCode')"
          width="120"
        />
        <el-table-column
          prop="quantity"
          :label="t('purchaseExt.returnTab.colQuantity')"
          width="100"
          align="right"
        />
        <el-table-column prop="unit" :label="t('purchaseExt.returnTab.colUnit')" width="80" />
        <el-table-column
          prop="price"
          :label="t('purchaseExt.returnTab.colPrice')"
          width="100"
          align="right"
        >
          <template #default="{ row }">
            {{ formatMoney(row.price) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="amount"
          :label="t('purchaseExt.returnTab.colAmount')"
          width="100"
          align="right"
        >
          <template #default="{ row }">
            {{ formatMoney(row.amount) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="reason"
          :label="t('purchaseExt.returnTab.colReason')"
          min-width="120"
        />
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance, FormRules } from 'element-plus';
import {
  getPurchaseReturnList,
  getPurchaseReturnById,
  updatePurchaseReturn,
  createPurchaseReturn,
  type PurchaseReturn,
  type PurchaseReturnItem,
} from '@/api/purchase-return';

const { t } = useI18n({ useScope: 'global' });

const purchaseReturns = ref<PurchaseReturn[]>([]);
const returnLoading = ref(false);

const returnQuery = reactive({
  returnNo: '',
  supplierName: '',
  status: '',
});

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00';
};

const getReturnStatusLabel = (status?: string) => {
  const map: Record<string, string> = {
    draft: t('purchaseExt.returnTab.statusDraft'),
    pending: t('purchaseExt.returnTab.statusPending'),
    approved: t('purchaseExt.returnTab.statusApproved'),
    rejected: t('purchaseExt.returnTab.statusRejected'),
    completed: t('purchaseExt.returnTab.statusCompleted'),
  };
  return map[status || ''] || status || '';
};

const getReturnStatusType = (status?: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    pending: 'warning',
    approved: 'success',
    rejected: 'danger',
    completed: 'success',
  };
  return map[status || ''] || 'info';
};

const fetchPurchaseReturns = async () => {
  returnLoading.value = true;
  try {
    const res = await getPurchaseReturnList(returnQuery);
    purchaseReturns.value = res.data?.list || [];
  } catch (error) {
    const err = error as { message?: string };
    ElMessage.error(err.message || t('purchaseExt.returnTab.fetchFailed'));
  } finally {
    returnLoading.value = false;
  }
};

const resetReturnQuery = () => {
  returnQuery.returnNo = '';
  returnQuery.supplierName = '';
  returnQuery.status = '';
  fetchPurchaseReturns();
};

const returnDialogVisible = ref(false);
const returnFormRef = ref<FormInstance>();
const returnSubmitLoading = ref(false);
const returnForm = reactive({
  id: 0,
  returnNo: '',
  supplierId: 0,
  supplierName: '',
  orderId: 0,
  purchaseOrderNo: '',
  returnDate: '',
  totalAmount: 0,
  reason: '',
  status: 'draft' as 'draft' | 'pending' | 'approved' | 'rejected' | 'completed',
  items: [] as PurchaseReturnItem[],
});

const returnRules: FormRules = {
  returnNo: [{ required: true, message: t('purchaseExt.returnTab.ruleReturnNo'), trigger: 'blur' }],
  supplierName: [
    { required: true, message: t('purchaseExt.returnTab.ruleSupplierName'), trigger: 'blur' },
  ],
  returnDate: [
    { required: true, message: t('purchaseExt.returnTab.ruleReturnDate'), trigger: 'change' },
  ],
  reason: [{ required: true, message: t('purchaseExt.returnTab.ruleReason'), trigger: 'blur' }],
};

const openReturnDialog = async (row?: PurchaseReturn) => {
  if (row) {
    const res = await getPurchaseReturnById(row.id!);
    // 安全检查：防止后端返回 data 为 null 时崩溃
    if (res.data) Object.assign(returnForm, res.data);
  } else {
    Object.assign(returnForm, {
      id: 0,
      returnNo: '',
      supplierId: 0,
      supplierName: '',
      orderId: 0,
      purchaseOrderNo: '',
      returnDate: '',
      totalAmount: 0,
      reason: '',
      status: 'draft',
      items: [
        {
          id: 0,
          returnId: 0,
          productId: 0,
          productName: '',
          productCode: '',
          quantity: 0,
          unit: '',
          price: 0,
          amount: 0,
          reason: '',
        },
      ],
    });
  }
  returnDialogVisible.value = true;
};

const submitReturn = async () => {
  const valid = await returnFormRef.value?.validate();
  if (!valid) return;

  returnSubmitLoading.value = true;
  try {
    if (returnForm.id) {
      await updatePurchaseReturn(returnForm.id, returnForm);
      ElMessage.success(t('purchaseExt.returnTab.updateSuccess'));
    } else {
      await createPurchaseReturn(returnForm);
      ElMessage.success(t('purchaseExt.returnTab.createSuccess'));
    }
    returnDialogVisible.value = false;
    fetchPurchaseReturns();
  } catch (error) {
    const err = error as { message?: string };
    ElMessage.error(err.message || t('purchaseExt.returnTab.operationFailed'));
  } finally {
    returnSubmitLoading.value = false;
  }
};

const returnViewVisible = ref(false);
const currentReturn = ref<PurchaseReturn | null>(null);

const viewReturn = async (row: PurchaseReturn) => {
  const res = await getPurchaseReturnById(row.id!);
  // 安全检查：防止后端返回 data 为 null 时崩溃
  if (res.data) currentReturn.value = res.data;
  returnViewVisible.value = true;
};

const addReturnItem = () => {
  returnForm.items.push({
    id: 0,
    returnId: 0,
    productId: 0,
    productName: '',
    productCode: '',
    quantity: 0,
    unitPrice: 0,
    reason: '',
  } as PurchaseReturnItem);
};

const removeReturnItem = (index: number) => {
  if (returnForm.items.length > 1) {
    returnForm.items.splice(index, 1);
  }
};

defineExpose({ refresh: fetchPurchaseReturns });

onMounted(() => {
  fetchPurchaseReturns();
});
</script>
