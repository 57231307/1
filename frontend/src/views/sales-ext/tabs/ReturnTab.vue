<!--
  ReturnTab.vue - 销售退货 Tab
  来源：原 sales-ext/index.vue 中 销售退货 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="return-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('salesExt.returnTab.pageTitle') }}</h2>
      <el-button type="primary" @click="openReturnDialog()">
        <el-icon><Plus /></el-icon> {{ t('salesExt.returnTab.buttonCreate') }}
      </el-button>
    </div>
    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="returnQuery"
        :aria-label="t('salesExt.returnTab.ariaLabelFilter')"
      >
        <el-form-item :label="t('salesExt.returnTab.labelReturnNo')">
          <el-input
            v-model="returnQuery.returnNo"
            :placeholder="t('salesExt.returnTab.placeholderReturnNo')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('salesExt.returnTab.labelCustomer')">
          <el-input
            v-model="returnQuery.customerName"
            :placeholder="t('salesExt.returnTab.placeholderCustomerName')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('salesExt.returnTab.labelStatus')">
          <el-select
            v-model="returnQuery.status"
            :placeholder="t('salesExt.returnTab.placeholderStatus')"
            clearable
          >
            <el-option :label="t('salesExt.returnTab.optionDraft')" value="draft" />
            <el-option :label="t('salesExt.returnTab.optionPending')" value="pending" />
            <el-option :label="t('salesExt.returnTab.optionApproved')" value="approved" />
            <el-option :label="t('salesExt.returnTab.optionRejected')" value="rejected" />
            <el-option :label="t('salesExt.returnTab.optionCompleted')" value="completed" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchSalesReturns">{{
            t('salesExt.returnTab.buttonSearch')
          }}</el-button>
          <el-button @click="resetReturnQuery">{{ t('salesExt.returnTab.buttonReset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>
    <el-card shadow="hover">
      <el-table
        v-loading="returnLoading"
        :data="salesReturns"
        stripe
        :aria-label="t('salesExt.returnTab.ariaLabelList')"
      >
        <el-table-column
          prop="returnNo"
          :label="t('salesExt.returnTab.columnReturnNo')"
          width="140"
        />
        <el-table-column
          prop="customerName"
          :label="t('salesExt.returnTab.columnCustomer')"
          min-width="150"
        />
        <el-table-column
          prop="salesOrderNo"
          :label="t('salesExt.returnTab.columnSalesOrderNo')"
          width="140"
        />
        <el-table-column
          prop="returnDate"
          :label="t('salesExt.returnTab.columnReturnDate')"
          width="120"
        />
        <el-table-column
          prop="totalAmount"
          :label="t('salesExt.returnTab.columnTotalAmount')"
          width="120"
          align="right"
        >
          <template #default="{ row }">
            {{ formatMoney(row.totalAmount) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="status"
          :label="t('salesExt.returnTab.columnStatus')"
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
          :label="t('salesExt.returnTab.columnCreatedBy')"
          width="100"
        />
        <el-table-column :label="t('salesExt.returnTab.columnAction')" width="180" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link @click="viewReturn(row as unknown as SalesReturn)">{{
              t('salesExt.returnTab.buttonView')
            }}</el-button>
            <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
            <el-button
              v-if="row.status === 'draft'"
              v-permission="PERMISSIONS.SALES_RETURN_UPDATE"
              size="small"
              link
              @click="openReturnDialog(row as unknown as SalesReturn)"
              >{{ t('salesExt.returnTab.buttonEdit') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 扩展指令（批次 86）：补全退货编辑对话框，替换原占位符 -->
    <el-dialog
      v-model="returnDialogVisible"
      :title="
        returnForm.id ? t('salesExt.returnTab.titleEdit') : t('salesExt.returnTab.titleCreate')
      "
      width="800px"
      :aria-label="t('salesExt.returnTab.ariaLabelDialog')"
    >
      <el-form
        ref="returnFormRef"
        :model="returnForm"
        :rules="returnRules"
        label-width="100px"
        :aria-label="t('salesExt.returnTab.ariaLabelForm')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('salesExt.returnTab.labelReturnNo')" prop="returnNo">
              <el-input v-model="returnForm.returnNo" :disabled="!!returnForm.id" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('salesExt.returnTab.labelCustomer')" prop="customerName">
              <el-input
                v-model="returnForm.customerName"
                :placeholder="t('salesExt.returnTab.placeholderCustomerName')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('salesExt.returnTab.labelRelatedOrder')" prop="salesOrderNo">
              <el-input
                v-model="returnForm.salesOrderNo"
                :placeholder="t('salesExt.returnTab.placeholderSalesOrderNo')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('salesExt.returnTab.labelReturnDate')" prop="returnDate">
              <el-date-picker
                v-model="returnForm.returnDate"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="t('salesExt.returnTab.labelReason')" prop="reason">
          <el-input v-model="returnForm.reason" type="textarea" />
        </el-form-item>
        <el-divider>{{ t('salesExt.returnTab.dividerItems') }}</el-divider>
        <el-table
          :data="returnForm.items"
          border
          style="width: 100%"
          :aria-label="t('salesExt.returnTab.ariaLabelItemsEdit')"
        >
          <el-table-column
            prop="productName"
            :label="t('salesExt.returnTab.columnProductName')"
            min-width="150"
          >
            <template #default="{ row }">
              <el-input
                v-model="row.productName"
                :placeholder="t('salesExt.returnTab.placeholderProductName')"
              />
            </template>
          </el-table-column>
          <el-table-column
            prop="productCode"
            :label="t('salesExt.returnTab.columnProductCode')"
            width="120"
          >
            <template #default="{ row }">
              <el-input
                v-model="row.productCode"
                :placeholder="t('salesExt.returnTab.placeholderProductCode')"
              />
            </template>
          </el-table-column>
          <el-table-column
            prop="quantity"
            :label="t('salesExt.returnTab.columnQuantity')"
            width="100"
          >
            <template #default="{ row }">
              <el-input-number v-model="row.quantity" :min="0" style="width: 100%" />
            </template>
          </el-table-column>
          <el-table-column
            prop="unitPrice"
            :label="t('salesExt.returnTab.columnUnitPrice')"
            width="100"
          >
            <template #default="{ row }">
              <el-input-number
                v-model="row.unitPrice"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </template>
          </el-table-column>
          <el-table-column :label="t('salesExt.returnTab.columnAmount')" width="100">
            <template #default="{ row }">
              {{ formatMoney((row.quantity || 0) * (row.unitPrice || 0)) }}
            </template>
          </el-table-column>
          <el-table-column
            prop="reason"
            :label="t('salesExt.returnTab.columnReason')"
            min-width="120"
          >
            <template #default="{ row }">
              <el-input
                v-model="row.reason"
                :placeholder="t('salesExt.returnTab.placeholderReason')"
              />
            </template>
          </el-table-column>
          <el-table-column :label="t('salesExt.returnTab.columnAction')" width="80">
            <template #default="{ $index }">
              <el-button size="small" link type="danger" @click="removeReturnItem($index)">{{
                t('salesExt.returnTab.buttonDelete')
              }}</el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-button type="primary" link style="margin-top: 8px" @click="addReturnItem">{{
          t('salesExt.returnTab.buttonAddProduct')
        }}</el-button>
      </el-form>
      <template #footer>
        <el-button @click="returnDialogVisible = false">{{
          t('salesExt.returnTab.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="returnSubmitLoading" @click="submitReturn">{{
          t('salesExt.returnTab.buttonConfirm')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 退货详情对话框 -->
    <el-dialog
      v-model="returnViewVisible"
      :title="t('salesExt.returnTab.titleDetail')"
      width="800px"
      :aria-label="t('salesExt.returnTab.ariaLabelDetail')"
    >
      <el-descriptions :column="2" border :aria-label="t('salesExt.returnTab.titleDetail')">
        <el-descriptions-item :label="t('salesExt.returnTab.labelReturnNo')">{{
          currentReturn?.returnNo
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesExt.returnTab.labelCustomer')">{{
          currentReturn?.customerName
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesExt.returnTab.labelRelatedOrder')">{{
          currentReturn?.salesOrderNo
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesExt.returnTab.labelReturnDate')">{{
          currentReturn?.returnDate
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesExt.returnTab.labelTotalAmount')">{{
          formatMoney(currentReturn?.totalAmount || 0)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesExt.returnTab.labelStatus')">
          <el-tag :type="getReturnStatusType(currentReturn?.status)">
            {{ getReturnStatusLabel(currentReturn?.status) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('salesExt.returnTab.labelCreatedBy')">{{
          currentReturn?.createdBy
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesExt.returnTab.labelApprovedBy')">{{
          currentReturn?.approved_by
        }}</el-descriptions-item>
      </el-descriptions>
      <el-divider>{{ t('salesExt.returnTab.dividerReason') }}</el-divider>
      <p>{{ currentReturn?.reason }}</p>
      <el-divider>{{ t('salesExt.returnTab.dividerItems') }}</el-divider>
      <el-table
        :data="currentReturn?.items || []"
        stripe
        :aria-label="t('salesExt.returnTab.ariaLabelItemsList')"
      >
        <el-table-column
          prop="productName"
          :label="t('salesExt.returnTab.columnProductName')"
          min-width="150"
        />
        <el-table-column
          prop="productCode"
          :label="t('salesExt.returnTab.columnProductCode')"
          width="120"
        />
        <el-table-column
          prop="quantity"
          :label="t('salesExt.returnTab.columnQuantity')"
          width="100"
          align="right"
        />
        <el-table-column prop="unit" :label="t('salesExt.returnTab.columnUnit')" width="80" />
        <el-table-column
          prop="price"
          :label="t('salesExt.returnTab.columnPrice')"
          width="100"
          align="right"
        >
          <template #default="{ row }">
            {{ formatMoney(row.price) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="amount"
          :label="t('salesExt.returnTab.columnAmount')"
          width="100"
          align="right"
        >
          <template #default="{ row }">
            {{ formatMoney(row.amount) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="reason"
          :label="t('salesExt.returnTab.columnReason')"
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
  getSalesReturnList,
  getSalesReturnById,
  updateSalesReturn,
  createSalesReturn,
  type SalesReturn,
  type SalesReturnItem,
} from '@/api/sales-return';
// Batch 462 P0-S24：引入权限码常量，与后端 sales-returns 资源对齐
import { PERMISSIONS } from '@/constants/permissions';

const { t } = useI18n({ useScope: 'global' });

const salesReturns = ref<SalesReturn[]>([]);
const returnLoading = ref(false);

const returnQuery = reactive({
  returnNo: '',
  customerName: '',
  status: '',
});

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00';
};

const getReturnStatusLabel = (status?: string) => {
  const map: Record<string, string> = {
    draft: t('salesExt.returnTab.statusDraft'),
    pending: t('salesExt.returnTab.statusPending'),
    approved: t('salesExt.returnTab.statusApproved'),
    rejected: t('salesExt.returnTab.statusRejected'),
    completed: t('salesExt.returnTab.statusCompleted'),
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

const fetchSalesReturns = async () => {
  returnLoading.value = true;
  try {
    const res = await getSalesReturnList(returnQuery);
    const d = res.data as
      | { list?: SalesReturn[]; items?: SalesReturn[]; data?: SalesReturn[] }
      | SalesReturn[]
      | undefined;
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      salesReturns.value = d.list || d.items || d.data || [];
    } else {
      salesReturns.value = (d as SalesReturn[]) || [];
    }
  } catch (error) {
    const err = error as { message?: string };
    ElMessage.error(err.message || t('salesExt.returnTab.messageFetchFailed'));
  } finally {
    returnLoading.value = false;
  }
};

const resetReturnQuery = () => {
  returnQuery.returnNo = '';
  returnQuery.customerName = '';
  returnQuery.status = '';
  fetchSalesReturns();
};

// 扩展指令（批次 86）：补全退货编辑表单状态与提交逻辑，替换原占位符
const returnDialogVisible = ref(false);
const returnFormRef = ref<FormInstance>();
const returnSubmitLoading = ref(false);
const returnForm = reactive({
  id: 0,
  returnNo: '',
  customerId: 0,
  customerName: '',
  salesOrderId: 0,
  salesOrderNo: '',
  returnDate: '',
  reason: '',
  status: 'draft',
  items: [] as SalesReturnItem[],
});

const returnRules: FormRules = {
  returnNo: [{ required: true, message: t('salesExt.returnTab.ruleReturnNo'), trigger: 'blur' }],
  customerName: [
    { required: true, message: t('salesExt.returnTab.ruleCustomerName'), trigger: 'blur' },
  ],
  returnDate: [
    { required: true, message: t('salesExt.returnTab.ruleReturnDate'), trigger: 'change' },
  ],
  reason: [{ required: true, message: t('salesExt.returnTab.ruleReason'), trigger: 'blur' }],
};

const openReturnDialog = async (row?: SalesReturn) => {
  if (row) {
    const res = await getSalesReturnById(row.id!);
    // 安全检查：防止后端返回 data 为 null 时崩溃
    if (res.data) Object.assign(returnForm, res.data);
  } else {
    Object.assign(returnForm, {
      id: 0,
      returnNo: '',
      customerId: 0,
      customerName: '',
      salesOrderId: 0,
      salesOrderNo: '',
      returnDate: '',
      reason: '',
      status: 'draft',
      items: [
        {
          productId: 0,
          productName: '',
          productCode: '',
          quantity: 0,
          unitPrice: 0,
          reason: '',
        } as SalesReturnItem,
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
      await updateSalesReturn(returnForm.id, returnForm);
      ElMessage.success(t('salesExt.returnTab.messageUpdateSuccess'));
    } else {
      await createSalesReturn(returnForm);
      ElMessage.success(t('salesExt.returnTab.messageCreateSuccess'));
    }
    returnDialogVisible.value = false;
    fetchSalesReturns();
  } catch (error) {
    const err = error as { message?: string };
    ElMessage.error(err.message || t('salesExt.returnTab.messageOperationFailed'));
  } finally {
    returnSubmitLoading.value = false;
  }
};

const addReturnItem = () => {
  returnForm.items.push({
    productId: 0,
    productName: '',
    productCode: '',
    quantity: 0,
    unitPrice: 0,
    reason: '',
  } as SalesReturnItem);
};

const removeReturnItem = (index: number) => {
  if (returnForm.items.length > 1) {
    returnForm.items.splice(index, 1);
  }
};

const returnViewVisible = ref(false);
const currentReturn = ref<SalesReturn | null>(null);

const viewReturn = async (row: SalesReturn) => {
  try {
    const res = await getSalesReturnById(row.id!);
    currentReturn.value = res.data || row;
    returnViewVisible.value = true;
  } catch (_e) {
    currentReturn.value = row;
    returnViewVisible.value = true;
  }
};

defineExpose({ refresh: fetchSalesReturns });

onMounted(() => {
  fetchSalesReturns();
});
</script>
