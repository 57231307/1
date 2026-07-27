<!--
  TransferTab.vue - 转账记录 Tab
  来源：原 fund/index.vue 中 转账记录 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="transfer-tab">
    <el-card class="table-card">
      <template #header>
        <div class="card-header">
          <span>{{ t('fund.transferTab.sectionTitle') }}</span>
          <el-button type="success" @click="openTransferDialog()">
            <el-icon><Money /></el-icon>{{ t('fund.transferTab.buttonNewTransfer') }}
          </el-button>
        </div>
      </template>

      <el-table
        v-loading="transferLoading"
        :data="transferList"
        stripe
        border
        :aria-label="t('fund.transferTab.tableAriaLabel')"
      >
        <el-table-column
          prop="transfer_no"
          :label="t('fund.transferTab.columnTransferNo')"
          width="180"
        />
        <el-table-column
          prop="from_account_name"
          :label="t('fund.transferTab.columnFromAccount')"
          min-width="140"
        />
        <el-table-column
          prop="to_account_name"
          :label="t('fund.transferTab.columnToAccount')"
          min-width="140"
        />
        <el-table-column prop="amount" :label="t('fund.transferTab.columnAmount')" width="140">
          <template #default="{ row }">
            <span class="balance-positive">¥{{ row.amount.toFixed(2) }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="status" :label="t('fund.transferTab.columnStatus')" width="100">
          <template #default="{ row }">
            <el-tag :type="getTransferStatusType(row.status)">
              {{ getTransferStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="remark"
          :label="t('fund.transferTab.columnRemark')"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column
          prop="created_at"
          :label="t('fund.transferTab.columnCreatedAt')"
          width="160"
        />
        <el-table-column :label="t('fund.transferTab.columnActions')" width="100" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewTransferDetail(row)">{{
              t('fund.transferTab.buttonDetail')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="transferTotal"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('fund.transferTab.paginationAriaLabel')"
        />
      </div>
    </el-card>

    <el-dialog
      v-model="transferVisible"
      :title="t('fund.transferTab.dialogTitle')"
      width="600px"
      :aria-label="t('fund.transferTab.dialogAriaLabel')"
    >
      <el-form
        ref="transferFormRef"
        :model="transferForm"
        :rules="transferRules"
        label-width="120px"
        :aria-label="t('fund.transferTab.formAriaLabel')"
      >
        <el-form-item :label="t('fund.transferTab.fieldFromAccount')" prop="from_account_id">
          <el-select
            v-model="transferForm.from_account_id"
            :placeholder="t('fund.transferTab.placeholderFromAccount')"
            style="width: 100%"
            filterable
            @change="handleFromAccountChange"
          >
            <el-option
              v-for="account in activeAccounts"
              :key="account.id"
              :label="formatFromAccountLabel(account)"
              :value="account.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('fund.transferTab.fieldToAccount')" prop="to_account_id">
          <el-select
            v-model="transferForm.to_account_id"
            :placeholder="t('fund.transferTab.placeholderToAccount')"
            style="width: 100%"
            filterable
          >
            <el-option
              v-for="account in otherAccounts"
              :key="account.id"
              :label="formatToAccountLabel(account)"
              :value="account.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('fund.transferTab.fieldAmount')" prop="amount">
          <el-input-number
            v-model="transferForm.amount"
            :min="0.01"
            :max="availableBalance"
            :precision="2"
            style="width: 100%"
            :placeholder="t('fund.transferTab.placeholderAmount')"
          />
          <div v-if="selectedFromAccount" class="balance-hint">
            {{ t('fund.transferTab.availableBalance') }}:
            <span class="balance-available"
              >¥{{
                (
                  selectedFromAccount.available_balance ||
                  selectedFromAccount.current_balance ||
                  selectedFromAccount.balance ||
                  0
                ).toFixed(2)
              }}</span
            >
          </div>
        </el-form-item>
        <el-form-item :label="t('fund.transferTab.fieldRemark')">
          <el-input
            v-model="transferForm.remark"
            type="textarea"
            :rows="3"
            :placeholder="t('fund.transferTab.placeholderRemark')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="transferVisible = false">{{
          t('fund.transferTab.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="transferSubmitLoading" @click="handleTransferSubmit">{{
          t('fund.transferTab.buttonConfirmTransfer')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { Money } from '@element-plus/icons-vue';
import {
  getFundAccountList,
  getFundTransfer,
  transferFund,
  type FundAccount,
  type FundTransferRecord,
} from '@/api/fund';
// 批次 280：接入 useTableApi，消除手写 transferList/transferLoading/transferTotal/fetchTransfers 重复
import { useTableApi } from '@/composables/useTableApi';

const { t } = useI18n({ useScope: 'global' });

const transferSubmitLoading = ref(false);
const transferVisible = ref(false);
const accountList = ref<FundAccount[]>([]);
const transferFormRef = ref<FormInstance>();

// 批次 280：useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
// getFundTransferList 返回 ApiResponse<FundTransferRecord[]>（{ data: T[] }），useTableApi detectList 会 fallback 到 obj.data
const {
  data: transferList,
  loading: transferLoading,
  page,
  pageSize,
  total: transferTotal,
  refresh: fetchTransfers,
} = useTableApi<FundTransferRecord>({
  url: '/fund-management/transfers',
  onError: (err: unknown) =>
    ElMessage.error(
      (err instanceof Error ? err.message : String(err)) || t('fund.transferTab.messageFetchFailed')
    ),
});

const transferForm = reactive({
  from_account_id: undefined as number | undefined,
  to_account_id: undefined as number | undefined,
  amount: 0,
  remark: '',
});

const transferRules: FormRules = {
  from_account_id: [
    { required: true, message: t('fund.transferTab.validateFromAccount'), trigger: 'change' },
  ],
  to_account_id: [
    { required: true, message: t('fund.transferTab.validateToAccount'), trigger: 'change' },
  ],
  amount: [
    { required: true, message: t('fund.transferTab.validateAmount'), trigger: 'blur' },
    {
      validator: (_rule, value, callback) => {
        if (value <= 0) {
          callback(new Error(t('fund.transferTab.validateAmountPositive')));
        } else if (value > availableBalance.value) {
          callback(new Error(t('fund.transferTab.validateAmountExceed')));
        } else {
          callback();
        }
      },
      trigger: 'blur',
    },
  ],
};

const activeAccounts = computed(() => {
  return accountList.value.filter(acc => acc.status === 'active');
});

const otherAccounts = computed(() => {
  return activeAccounts.value.filter(acc => acc.id !== transferForm.from_account_id);
});

const selectedFromAccount = computed(() => {
  return accountList.value.find(acc => acc.id === transferForm.from_account_id);
});

const availableBalance = computed(() => {
  return selectedFromAccount.value
    ? selectedFromAccount.value.available_balance ||
        selectedFromAccount.value.current_balance ||
        selectedFromAccount.value.balance ||
        0
    : 999999999;
});

/** 格式化转出账户下拉选项标签 */
const formatFromAccountLabel = (account: FundAccount): string => {
  const balance = (
    account.available_balance ||
    account.current_balance ||
    account.balance ||
    0
  ).toFixed(2);
  return `${account.account_name} (${t('fund.transferTab.available')}: ¥${balance})`;
};

/** 格式化转入账户下拉选项标签 */
const formatToAccountLabel = (account: FundAccount): string => {
  const balance = (account.current_balance || account.balance || 0).toFixed(2);
  return `${account.account_name} (${t('fund.transferTab.current')}: ¥${balance})`;
};

const fetchAccounts = async () => {
  try {
    const res = await getFundAccountList();
    const d = res.data as
      | { list?: FundAccount[]; items?: FundAccount[]; data?: FundAccount[] }
      | FundAccount[];
    accountList.value = Array.isArray(d) ? d : d?.list || d?.items || [];
  } catch (e) {
    const err = e as Error;
    ElMessage.error(err.message || t('fund.transferTab.messageFetchAccountsFailed'));
  }
};

const openTransferDialog = () => {
  transferForm.from_account_id = undefined;
  transferForm.to_account_id = undefined;
  transferForm.amount = 0;
  transferForm.remark = '';
  transferVisible.value = true;
};

const handleFromAccountChange = () => {
  if (transferForm.from_account_id === transferForm.to_account_id) {
    transferForm.to_account_id = undefined;
  }
};

const handleTransferSubmit = async () => {
  if (!transferFormRef.value) return;
  await transferFormRef.value.validate(async valid => {
    if (!valid) return;
    transferSubmitLoading.value = true;
    try {
      await transferFund({
        from_account_id: transferForm.from_account_id!,
        to_account_id: transferForm.to_account_id!,
        amount: transferForm.amount,
        remark: transferForm.remark,
      });
      ElMessage.success(t('fund.transferTab.messageTransferSuccess'));
      transferVisible.value = false;
      fetchTransfers();
    } catch (e) {
      const err = e as Error;
      ElMessage.error(err.message || t('fund.transferTab.messageTransferFailed'));
    } finally {
      transferSubmitLoading.value = false;
    }
  });
};

/** 构造转账详情多行文本（拆分以控制 viewTransferDetail 行数） */
const buildTransferDetailLines = (d: FundTransferRecord): string[] => {
  return [
    t('fund.transferTab.detailTransferNo', { value: d.transfer_no }),
    t('fund.transferTab.detailFromAccount', { value: d.from_account_name || '-' }),
    t('fund.transferTab.detailToAccount', { value: d.to_account_name || '-' }),
    t('fund.transferTab.detailAmount', { value: d.amount.toFixed(2) }),
    t('fund.transferTab.detailCurrentStatus', { value: getTransferStatusLabel(d.status) }),
    t('fund.transferTab.detailCreatedAt', { value: d.created_at }),
    t('fund.transferTab.detailRemark', { value: d.remark || '-' }),
  ];
};

// 批次 157a P1-1 修复：接入 getFundTransfer API 展示转账详情
const viewTransferDetail = async (row: FundTransferRecord) => {
  try {
    const res = await getFundTransfer(row.id);
    const d = res.data;
    if (!d) {
      ElMessage.warning(t('fund.transferTab.messageDetailNotFound'));
      return;
    }
    const lines = buildTransferDetailLines(d);
    await ElMessageBox.alert(lines.join('\n'), t('fund.transferTab.detailTitle'), {
      confirmButtonText: t('fund.transferTab.buttonClose'),
    });
  } catch (e) {
    const err = e as Error;
    ElMessage.error(err.message || t('fund.transferTab.messageFetchDetailFailed'));
  }
};

const getTransferStatusType = (status: string) => {
  const map: Record<string, string> = {
    success: 'success',
    pending: 'warning',
    failed: 'danger',
    processing: 'info',
  };
  return map[status] || 'info';
};

/** 转账状态 → i18n 标签（语言切换响应） */
const getTransferStatusLabel = (status: string): string => {
  switch (status) {
    case 'success':
      return t('fund.transferTab.statusSuccess');
    case 'pending':
      return t('fund.transferTab.statusPending');
    case 'failed':
      return t('fund.transferTab.statusFailed');
    case 'processing':
      return t('fund.transferTab.statusProcessing');
    default:
      return status;
  }
};

onMounted(() => {
  fetchAccounts();
});
</script>
