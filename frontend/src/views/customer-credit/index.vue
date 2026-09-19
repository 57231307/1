<!--
  customerCredit/index.vue - 客户信用管理主入口
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-3）：
  原 400+ 行"上帝组件"已拆分为：
  - tabs/RatingDialogTab.vue - 设置信用评级对话框
  - tabs/AdjustDialogTab.vue - 调整额度对话框
  - tabs/AmountDialogTab.vue - 占用/释放额度对话框

  本主入口承担：列表 + 工具栏 + 公共样式。
-->
<template>
  <div class="customer-credit">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>{{ t('customerCredit.index.title') }}</span>
        </div>
      </template>

      <div class="toolbar">
        <el-button type="primary" @click="openRatingDialog">{{
          t('customerCredit.index.button.setRating')
        }}</el-button>
      </div>

      <el-table
        :data="creditList"
        border
        stripe
        :aria-label="t('customerCredit.index.table.ariaLabel')"
      >
        <el-table-column
          prop="customer_name"
          :label="t('customerCredit.index.table.column.customerName')"
        />
        <el-table-column
          prop="credit_rating"
          :label="t('customerCredit.index.table.column.creditGrade')"
        >
          <template #default="{ row }">
            <el-tag v-if="row.credit_rating === 'AAA'" type="success">AAA</el-tag>
            <el-tag v-else-if="row.credit_rating === 'AA'" type="success">AA</el-tag>
            <el-tag v-else-if="row.credit_rating === 'A'" type="success">A</el-tag>
            <el-tag v-else-if="row.credit_rating === 'BBB'" type="warning">BBB</el-tag>
            <el-tag v-else-if="row.credit_rating === 'BB'" type="warning">BB</el-tag>
            <el-tag v-else-if="row.credit_rating === 'B'" type="warning">B</el-tag>
            <el-tag v-else type="danger">{{ row.credit_rating || '-' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="credit_limit"
          :label="t('customerCredit.index.table.column.creditLimit')"
        />
        <el-table-column
          prop="used_credit"
          :label="t('customerCredit.index.table.column.usedCredit')"
        />
        <el-table-column
          prop="available_credit"
          :label="t('customerCredit.index.table.column.availableCredit')"
        >
          <template #default="{ row }">
            <span
              :style="{
                color: row.available_credit && row.available_credit > 0 ? '#67c23a' : '#f56c6c',
              }"
            >
              {{ row.available_credit }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="status" :label="t('customerCredit.index.table.column.status')">
          <template #default="{ row }">
            <el-tag v-if="row.status === 'active'" type="success">{{
              t('customerCredit.index.status.active')
            }}</el-tag>
            <el-tag v-else type="danger">{{ t('customerCredit.index.status.inactive') }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          :label="t('customerCredit.index.table.column.action')"
          fixed="right"
          width="520"
        >
          <template #default="{ row }">
            <el-button link type="primary" @click="handleViewDetail(row)">{{
              t('customerCredit.index.button.view')
            }}</el-button>
            <el-button link type="warning" @click="handleEdit(row)">{{
              t('customerCredit.index.button.edit')
            }}</el-button>
            <el-button link type="primary" @click="openAdjustDialog(row)">{{
              t('customerCredit.index.button.adjust')
            }}</el-button>
            <el-button link type="primary" @click="openOccupyDialog(row)">{{
              t('customerCredit.index.button.occupy')
            }}</el-button>
            <el-button link type="primary" @click="openReleaseDialog(row)">{{
              t('customerCredit.index.button.release')
            }}</el-button>
            <el-button link type="primary" @click="handleEvaluate(row)">{{
              t('customerCredit.index.button.evaluate')
            }}</el-button>
            <el-button
              v-if="row.status === 'active'"
              link
              type="danger"
              @click="handleDeactivate(row)"
              >{{ t('customerCredit.index.button.deactivate') }}</el-button
            >
            <el-button link type="danger" @click="handleDelete(row)">{{
              t('customerCredit.index.button.delete')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next, jumper"
        :aria-label="t('customerCredit.index.table.paginationAria')"
        @size-change="handleSizeChange"
      />
    </el-card>

    <RatingDialogTab
      v-model="ratingDialogVisible"
      :customers="customerOptions"
      @submitted="fetchCredits"
    />

    <AdjustDialogTab
      v-model="adjustDialogVisible"
      :customer-id="currentCustomerId"
      @submitted="fetchCredits"
    />

    <AmountDialogTab
      v-model="amountDialogVisible"
      :customer-id="currentCustomerId"
      :operation-type="amountOperationType"
      @submitted="fetchCredits"
    />

    <!-- 详情对话框 -->
    <el-dialog
      v-model="detailDialogVisible"
      :title="t('customerCredit.index.detail.title')"
      width="500px"
    >
      <el-descriptions :column="1" border>
        <el-descriptions-item :label="t('customerCredit.index.table.column.customerName')">{{
          detailRecord.customer_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('customerCredit.index.table.column.creditGrade')">{{
          detailRecord.credit_rating
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('customerCredit.index.table.column.creditLimit')">{{
          detailRecord.credit_limit
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('customerCredit.index.table.column.usedCredit')">{{
          detailRecord.used_credit
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('customerCredit.index.table.column.availableCredit')">{{
          detailRecord.available_credit
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('customerCredit.index.table.column.status')">{{
          detailRecord.status === 'active'
            ? t('customerCredit.index.status.active')
            : t('customerCredit.index.status.inactive')
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('customerCredit.index.detail.validFrom')">{{
          detailRecord.valid_from
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('customerCredit.index.detail.validTo')">{{
          detailRecord.valid_to
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('customerCredit.index.detail.remarks')">{{
          detailRecord.remarks
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>

    <!-- 编辑对话框 -->
    <el-dialog
      v-model="editDialogVisible"
      :title="t('customerCredit.index.edit.title')"
      width="480px"
    >
      <el-form
        ref="editFormRef"
        :model="editForm"
        label-width="100px"
        :aria-label="t('customerCredit.index.edit.ariaLabel')"
      >
        <el-form-item :label="t('customerCredit.index.table.column.creditLimit')">
          <el-input-number v-model="editForm.credit_limit" :min="0" controls-position="right" />
        </el-form-item>
        <el-form-item :label="t('customerCredit.index.table.column.status')">
          <el-select v-model="editForm.status" style="width: 100%">
            <el-option :label="t('customerCredit.index.status.active')" value="active" />
            <el-option :label="t('customerCredit.index.status.inactive')" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('customerCredit.index.detail.validFrom')">
          <el-input v-model="editForm.valid_from" />
        </el-form-item>
        <el-form-item :label="t('customerCredit.index.detail.validTo')">
          <el-input v-model="editForm.valid_to" />
        </el-form-item>
        <el-form-item :label="t('customerCredit.index.detail.remarks')">
          <el-input v-model="editForm.remarks" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editDialogVisible = false">{{
          t('customerCredit.index.dialog.cancel')
        }}</el-button>
        <el-button type="primary" :loading="editSubmitting" @click="submitEdit">{{
          t('customerCredit.index.dialog.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  deactivateCredit,
  deleteCustomerCredit,
  evaluateCustomerCredit,
  getCustomerCredit,
  updateCustomerCredit,
  type CustomerCredit,
} from '@/api/customer-credit';
import { getCustomerList, type Customer } from '@/api/customer';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { logger } from '@/utils/logger';
import { useTableApi } from '@/composables/useTableApi';
import RatingDialogTab from './tabs/RatingDialogTab.vue';
import AdjustDialogTab from './tabs/AdjustDialogTab.vue';
import AmountDialogTab from './tabs/AmountDialogTab.vue';

const { t } = useI18n({ useScope: 'global' });

const hasLoaded = createLazyLoader();

const customerOptions = ref<Customer[]>([]);

// 批次 272：接入 useTableApi，消除手写 pagination/creditList/total + fetchCredits 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: creditList,
  page,
  pageSize,
  total,
  refresh: fetchCredits,
} = useTableApi<CustomerCredit>({
  url: '/crm/customer-credits',
  onError: (e: unknown) => {
    ElMessage.error(t('customerCredit.index.message.fetchListFailed'));
    logger.warn(t('customerCredit.index.log.fetchListFailed'), String(e));
  },
});

const ratingDialogVisible = ref(false);
const adjustDialogVisible = ref(false);
const amountDialogVisible = ref(false);
const amountOperationType = ref<'occupy' | 'release'>('occupy');
const currentCustomerId = ref<number | null>(null);
const detailDialogVisible = ref(false);
const detailRecord = ref<Partial<CustomerCredit>>({});
const editDialogVisible = ref(false);
const editSubmitting = ref(false);
const editForm = ref<Partial<CustomerCredit>>({});

// fetchCredits 由 useTableApi 的 refresh 提供（批次 272）

const fetchCustomers = async () => {
  try {
    const res = await getCustomerList({ page: 1, page_size: 100 });
    customerOptions.value = res.data?.items || [];
  } catch (error) {
    customerOptions.value = [];
  }
};

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handleSizeChange = () => {
  page.value = 1;
};

const openRatingDialog = () => {
  ratingDialogVisible.value = true;
};

const openAdjustDialog = (row: CustomerCredit) => {
  if (!row.id) return;
  currentCustomerId.value = row.id;
  adjustDialogVisible.value = true;
};

const openOccupyDialog = (row: CustomerCredit) => {
  if (!row.id) return;
  currentCustomerId.value = row.id;
  amountOperationType.value = 'occupy';
  amountDialogVisible.value = true;
};

const openReleaseDialog = (row: CustomerCredit) => {
  if (!row.id) return;
  currentCustomerId.value = row.id;
  amountOperationType.value = 'release';
  amountDialogVisible.value = true;
};

const handleViewDetail = async (row: CustomerCredit) => {
  if (!row.id) return;
  try {
    const res = await getCustomerCredit(row.id);
    detailRecord.value = res.data || {};
    detailDialogVisible.value = true;
  } catch (e) {
    const err = e as Error;
    ElMessage.error(err.message || t('customerCredit.index.message.fetchDetailFailed'));
    logger.warn(t('customerCredit.index.log.fetchDetailFailed'), String(e));
  }
};

const handleEvaluate = async (row: CustomerCredit) => {
  if (!row.id || !row.customer_id) return;
  try {
    await ElMessageBox.confirm(
      t('customerCredit.index.dialog.evaluateConfirmMessage'),
      t('customerCredit.index.dialog.evaluateConfirmTitle'),
      {
        confirmButtonText: t('customerCredit.index.dialog.confirm'),
        cancelButtonText: t('customerCredit.index.dialog.cancel'),
        type: 'info',
      }
    );

    await evaluateCustomerCredit({
      id: row.id,
      customer_id: row.customer_id,
      evaluation_date: new Date().toISOString().slice(0, 10),
    });
    ElMessage.success(t('customerCredit.index.message.evaluateSuccess'));
    fetchCredits();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error;
      ElMessage.error(err.message || t('customerCredit.index.message.evaluateFailed'));
    }
  }
};

const handleEdit = (row: CustomerCredit) => {
  if (!row.id) return;
  currentCustomerId.value = row.id;
  editForm.value = {
    credit_limit: row.credit_limit,
    status: row.status,
    valid_from: row.valid_from,
    valid_to: row.valid_to,
    remarks: row.remarks,
  };
  editDialogVisible.value = true;
};

const submitEdit = async () => {
  if (!currentCustomerId.value) return;
  editSubmitting.value = true;
  try {
    await updateCustomerCredit(currentCustomerId.value, editForm.value);
    ElMessage.success(t('customerCredit.index.message.updateSuccess'));
    editDialogVisible.value = false;
    fetchCredits();
  } catch (e) {
    const err = e as Error;
    ElMessage.error(err.message || t('customerCredit.index.message.updateFailed'));
  } finally {
    editSubmitting.value = false;
  }
};

const handleDelete = async (row: CustomerCredit) => {
  if (!row.id) return;

  try {
    await ElMessageBox.confirm(
      t('customerCredit.index.dialog.deleteConfirmMessage'),
      t('customerCredit.index.dialog.deleteConfirmTitle'),
      {
        confirmButtonText: t('customerCredit.index.dialog.confirm'),
        cancelButtonText: t('customerCredit.index.dialog.cancel'),
        type: 'warning',
      }
    );

    await deleteCustomerCredit(row.id);
    ElMessage.success(t('customerCredit.index.message.deleteSuccess'));
    fetchCredits();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error;
      ElMessage.error(err.message || t('customerCredit.index.message.deleteFailed'));
    }
  }
};

const handleDeactivate = async (row: CustomerCredit) => {
  if (!row.id) return;

  try {
    await ElMessageBox.confirm(
      t('customerCredit.index.dialog.deactivateConfirmMessage'),
      t('customerCredit.index.dialog.deactivateConfirmTitle'),
      {
        confirmButtonText: t('customerCredit.index.dialog.confirm'),
        cancelButtonText: t('customerCredit.index.dialog.cancel'),
        type: 'warning',
      }
    );

    await deactivateCredit(row.id);
    ElMessage.success(t('customerCredit.index.message.deactivateSuccess'));
    fetchCredits();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error;
      ElMessage.error(err.message || t('customerCredit.index.message.deactivateFailed'));
    }
  }
};

// 批次 272：useTableApi 构造时自动初始加载，无需 onMounted 调用 fetchCredits
onMounted(() => {
  loadIfNot('fetchCustomers', fetchCustomers, hasLoaded);
});
</script>

<style scoped>
.customer-credit .card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.customer-credit .toolbar {
  margin-bottom: 16px;
}

.customer-credit .el-table {
  margin-bottom: 16px;
}
</style>
