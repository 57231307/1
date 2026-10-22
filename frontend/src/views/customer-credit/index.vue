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
        <!--
          等级列只绑得出 credit_level（出参载体 CustomerCreditView 的
          credit_level: Option<String>，见 services/customer_credit_service.rs）。
          credit_rating 是评估端点出参 CreditEvaluationResult 的键，本载体没有它；
          绑错的结果是每行都读成 undefined，再被 || '-' 一类占位写法把"读不到"
          与"库里真没值"压成同一个显示。
          取值口径：null 显示显式"未评"文案；词表外的存量取值（列无 CHECK）原样显示并标 danger。
        -->
        <el-table-column
          prop="credit_level"
          :label="t('customerCredit.index.table.column.creditGrade')"
        >
          <template #default="{ row }">
            <el-tag v-if="isUnratedCreditLevel(row.credit_level)" type="info">{{
              t('customerCredit.index.creditLevel.none')
            }}</el-tag>
            <el-tag v-else :type="creditLevelTagType(row.credit_level)">{{
              row.credit_level
            }}</el-tag>
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
                color: decimalWireToNumber(row.available_credit) > 0 ? '#67c23a' : '#f56c6c',
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
      <!--
        详情项只列得出参载体真有键的字段。valid_from / valid_to / remarks 既不在
        CustomerCreditView 也不在表 customer_credit_ratings 里（建表 DDL 见迁移
        m0012_add_ap_ar_finance_analysis），绑它们就是一项恒空的假字段——按
        "无载体即不设项"处理，不写占位文案去掩盖。
        若产品需要"有效期"，须由后端先补列并加进出参载体，前端不凭空造键。
      -->
      <el-descriptions v-if="detailRecord" :column="1" border>
        <el-descriptions-item :label="t('customerCredit.index.table.column.customerName')">{{
          detailRecord.customer_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('customerCredit.index.table.column.creditGrade')">{{
          isUnratedCreditLevel(detailRecord.credit_level)
            ? t('customerCredit.index.creditLevel.none')
            : detailRecord.credit_level
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
      </el-descriptions>
    </el-dialog>

    <!--
      编辑对话框：字段集合 = 后端 PUT 端点 UpdateCreditDto 真正接收的写入口径
      （见 handlers/customer_credit_handler.rs），不是出参实体。
      不设的表单项：
      - valid_from / valid_to / remarks：既不在 UpdateCreditDto 也不在表列里，提交即被 serde 丢弃；
      - 状态下拉：UpdateCreditDto 没有 status 字段，带它提交是静默无效的假控件；
        改状态的真实入口是行上的"停用"（POST /{id}/deactivate），"设置信用评级"另有链路。
        若产品要在编辑里直接切状态，需后端先给 UpdateCreditDto 补 status，不在前端假装有这能力。
    -->
    <el-dialog
      v-model="editDialogVisible"
      :title="t('customerCredit.index.edit.title')"
      width="480px"
    >
      <el-form
        ref="editFormRef"
        :model="editForm"
        :rules="editRules"
        label-width="100px"
        :aria-label="t('customerCredit.index.edit.ariaLabel')"
      >
        <el-form-item
          :label="t('customerCredit.index.table.column.creditLimit')"
          prop="credit_limit"
        >
          <el-input-number
            v-model="editForm.credit_limit"
            :min="0"
            :precision="2"
            controls-position="right"
            style="width: 100%"
            :aria-label="t('customerCredit.index.table.column.creditLimit')"
          />
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
import { isDialogDismissal } from '@/utils/monitor';
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import {
  deactivateCredit,
  deleteCustomerCredit,
  evaluateCustomerCredit,
  getCustomerCredit,
  updateCustomerCredit,
  type CustomerCredit,
  type UpdateCustomerCreditInput,
} from '@/api/customer-credit';
import { getCustomerList, type Customer } from '@/api/customer';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { decimalWireToNumber } from '@/utils/money';
import { creditLevelTagType, isUnratedCreditLevel } from '@/constants/credit-level';
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
/**
 * 本域所有 {id} 路径参数的实际语义都是 customer_id（客户自然键），不是信用行的主键 id：
 * 后端 get_credit/update_credit/delete_credit/occupy/release/adjust/deactivate 的
 * Path 形参一律命名为 customer_id（handlers/customer_credit_handler.rs），
 * 且服务层全部按 Column::CustomerId.eq 查行
 * （services/customer_credit_limit.rs 与 customer_credit_service.rs 的
 * get_view_by_customer_id）。句柄必须传 row.customer_id：信用行主键与客户主键
 * 是两把独立的键，传错即读错行/改错行或 404。
 */
const currentCustomerId = ref<number | null>(null);
const detailDialogVisible = ref(false);
const detailRecord = ref<CustomerCredit | null>(null);
const editDialogVisible = ref(false);
const editSubmitting = ref(false);
const editFormRef = ref<FormInstance>();
/**
 * 编辑表单只承载后端 UpdateCreditDto 真正接收的写入字段（见 api/customer-credit.ts
 * 的 UpdateCustomerCreditInput 说明）。未填态是 undefined，不预置 0：
 * credit_limit 在后端是 Option<Decimal>，None 表示"保持原值"，
 * 前端替用户写 0 就是替用户把额度清零。
 */
const editForm = ref<{ credit_limit?: number | null }>({});
const editRules: FormRules = {
  credit_limit: [
    {
      // required 对数字 0 判为有效值（0 是"额度清零"的合法业务取值，必须由用户显式输入），
      // 只拦 undefined/null 这两种"没填"，避免把没填当成 0 提交上去。
      required: true,
      message: t('customerCredit.index.edit.creditLimitRequired'),
      trigger: 'blur',
    },
  ],
};

// fetchCredits 由 useTableApi 的 refresh 提供（批次 272）

const fetchCustomers = async () => {
  try {
    const res = await getCustomerList({ page: 1, page_size: 100 });
    customerOptions.value = res.data?.items || [];
  } catch (error) {
    logAuxLoadFailure(t('customerCredit.index.log.fetchCustomersFailed'), error);
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

/**
 * 端点句柄取行的 customer_id（理由见 currentCustomerId 上方注释）。
 * 拿不到时显式报错并拒绝操作：既不许静默 return（用户点了没反应、也不留日志），
 * 也不许退回用信用行主键 id 去猜一行（那会把改错行的缺陷藏起来）。
 */
const requireCustomerId = (row: CustomerCredit): number | null => {
  if (
    typeof row.customer_id !== 'number' ||
    !Number.isFinite(row.customer_id) ||
    row.customer_id <= 0
  ) {
    ElMessage.error(t('customerCredit.index.message.missingCustomerId'));
    logger.warn(
      t('customerCredit.index.log.missingCustomerId'),
      JSON.stringify({ id: row.id, customer_id: row.customer_id })
    );
    return null;
  }
  return row.customer_id;
};

const openAdjustDialog = (row: CustomerCredit) => {
  const customerId = requireCustomerId(row);
  if (customerId === null) return;
  currentCustomerId.value = customerId;
  adjustDialogVisible.value = true;
};

const openOccupyDialog = (row: CustomerCredit) => {
  const customerId = requireCustomerId(row);
  if (customerId === null) return;
  currentCustomerId.value = customerId;
  amountOperationType.value = 'occupy';
  amountDialogVisible.value = true;
};

const openReleaseDialog = (row: CustomerCredit) => {
  const customerId = requireCustomerId(row);
  if (customerId === null) return;
  currentCustomerId.value = customerId;
  amountOperationType.value = 'release';
  amountDialogVisible.value = true;
};

const handleViewDetail = async (row: CustomerCredit) => {
  const customerId = requireCustomerId(row);
  if (customerId === null) return;
  try {
    const res = await getCustomerCredit(customerId);
    // 后端 2xx 的 data 恒是实体本体（无行时走 AppError::not_found，见 handler 的 get_credit），
    // 不写 || {} 兜底：那会把"信封形状漂移/缺键"伪装成"这条信用记录是空的"。
    detailRecord.value = res.data;
    detailDialogVisible.value = true;
  } catch (e) {
    const err = e as Error;
    ElMessage.error(err.message || t('customerCredit.index.message.fetchDetailFailed'));
    logger.warn(t('customerCredit.index.log.fetchDetailFailed'), String(e));
  }
};

const handleEvaluate = async (row: CustomerCredit) => {
  const customerId = requireCustomerId(row);
  if (customerId === null) return;
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

    // 评估端点无路径参数，CreditEvaluationRequest 只有 customer_id + evaluation_date；
    // 载荷不带 id——该键不在 DTO 上，传了也只会被 serde 丢弃（幽灵写键）。
    await evaluateCustomerCredit({
      customer_id: customerId,
      evaluation_date: new Date().toISOString().slice(0, 10),
    });
    ElMessage.success(t('customerCredit.index.message.evaluateSuccess'));
    fetchCredits();
  } catch (e) {
    if (!isDialogDismissal(e)) {
      const err = e as Error;
      ElMessage.error(err.message || t('customerCredit.index.message.evaluateFailed'));
    }
  }
};

const handleEdit = (row: CustomerCredit) => {
  const customerId = requireCustomerId(row);
  if (customerId === null) return;
  currentCustomerId.value = customerId;
  // 出参 credit_limit 是 Decimal 的十进制字符串，编辑控件需要数值：显式归一后回填当前真值。
  // 不再回填 status/valid_from/valid_to/remarks：前三个不是 PUT 的写入字段，后两个连列都不存在。
  editForm.value = { credit_limit: decimalWireToNumber(row.credit_limit) };
  editDialogVisible.value = true;
};

const submitEdit = async () => {
  if (currentCustomerId.value === null || !editFormRef.value) return;
  editSubmitting.value = true;
  try {
    await editFormRef.value.validate();
    const limit = editForm.value.credit_limit;
    if (typeof limit !== 'number') {
      // 校验已过却仍拿不到数值 = 表单状态与校验脱节，显式失败，
      // 绝不替用户提交 0/null 去"凑"一个载荷（那会静默清零额度或静默保持原值）。
      ElMessage.error(t('customerCredit.index.edit.creditLimitRequired'));
      logger.warn(
        t('customerCredit.index.log.submitEditBlocked'),
        JSON.stringify({ credit_limit: editForm.value.credit_limit })
      );
      return;
    }
    const payload: UpdateCustomerCreditInput = { credit_limit: limit };
    await updateCustomerCredit(currentCustomerId.value, payload);
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
  const customerId = requireCustomerId(row);
  if (customerId === null) return;

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

    await deleteCustomerCredit(customerId);
    ElMessage.success(t('customerCredit.index.message.deleteSuccess'));
    fetchCredits();
  } catch (e) {
    if (!isDialogDismissal(e)) {
      const err = e as Error;
      ElMessage.error(err.message || t('customerCredit.index.message.deleteFailed'));
    }
  }
};

const handleDeactivate = async (row: CustomerCredit) => {
  const customerId = requireCustomerId(row);
  if (customerId === null) return;

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

    await deactivateCredit(customerId);
    ElMessage.success(t('customerCredit.index.message.deactivateSuccess'));
    fetchCredits();
  } catch (e) {
    if (!isDialogDismissal(e)) {
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
