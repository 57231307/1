<script setup lang="ts">
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  ElTable,
  ElTableColumn,
  ElButton,
  ElDialog,
  ElForm,
  ElFormItem,
  ElInput,
  ElInputNumber,
  ElSelect,
  ElOption,
  ElDatePicker,
  ElMessageBox,
  ElMessage,
  ElRow,
  ElCol,
  ElDescriptions,
} from 'element-plus';
import { Plus, Edit, Delete, View, Check } from '@element-plus/icons-vue';
import {
  getArReconciliation,
  updateArReconciliation,
  deleteArReconciliation,
  confirmReconciliation,
  type ArReconciliationEntity,
  type UpdateArReconciliationPayload,
} from '@/api/ar-reconciliation';
import {
  generateReconciliation,
  getReconciliationDetailItems,
  type ReconciliationDetailItem,
} from '@/api/ar-reconciliation-enhanced';
import { getCustomerSelectList } from '@/api/customer';
import { logger } from '@/utils/logger';
import { useTableApi } from '@/composables/useTableApi';

// 金额单元格统一转数字格式化（后端 Decimal 序列化为字符串，直接 toFixed 会崩溃）
const fmtNum = (v: unknown): string => Number(v ?? 0).toFixed(2);

const { t } = useI18n({ useScope: 'global' });

// 后端 ListReconciliationsQuery 只读 status/customer_id/page/page_size/start_date/end_date，
// 没有按客户名模糊查询的实现：客户筛选改为 customer_id 下拉（真实参数）
const searchForm = ref({
  customer_id: undefined as number | undefined,
  status: '',
  start_date: '',
  end_date: '',
});

// 批次 272：接入 useTableApi，消除手写 pagination/tableData/total/loading + loadData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: tableData,
  loading,
  page,
  pageSize,
  total,
  refresh: loadData,
  setQueryParam,
} = useTableApi<ArReconciliationEntity>({
  url: '/ar-reconciliations',
  onError: (e: unknown) => {
    ElMessage.error(t('arReconciliationModule.index.loadFailed'));
    logger.warn(t('arReconciliationModule.index.loadListFailed'), String(e));
  },
});

const dialogVisible = ref(false);
// 新增模式采集：客户 + 对账起止（走后端 generate 汇总端点，单号/金额由服务端在事务内生成）
// 编辑模式采集：PUT /ar-reconciliations/{id} 的 DTO 只有 opening_balance/total_invoices/
// total_collections/notes 四个可改字段，客户与起止期间后端不支持修改，编辑态只读展示
const form = ref<{
  id?: number;
  customer_id?: number;
  period_start: string;
  period_end: string;
  opening_balance?: number;
  total_invoices?: number;
  total_collections?: number;
  notes?: string;
}>({
  customer_id: undefined,
  period_start: '',
  period_end: '',
});
// 对话框标题：依据 form.id 自动切换「新增 / 编辑」，computed 保证语言切换即时生效
const dialogTitle = computed(() =>
  form.value.id
    ? t('arReconciliationModule.index.editReconciliation')
    : t('arReconciliationModule.index.addReconciliation')
);

const viewDialogVisible = ref(false);
const viewData = ref<ArReconciliationEntity | null>(null);
const detailData = ref<ReconciliationDetailItem[]>([]);

const customerOptions = ref<{ label: string; value: number }[]>([]);

// 状态下拉选项（label 走 i18n，computed 保证语言切换即时生效）
// 词表写入方 models/status/finance.rs::ar：draft/sent/confirmed/disputed/closed/cancelled 小写
const statusOptions = computed(() => [
  { label: t('arReconciliationModule.index.statusAll'), value: '' },
  { label: t('arReconciliationModule.index.statusDraft'), value: 'draft' },
  { label: t('arReconciliationModule.index.statusConfirmed'), value: 'confirmed' },
]);

const getStatusLabel = (value: string | null | undefined) => {
  if (!value) return '-';
  return statusOptions.value.find(s => s.value === value)?.label || value;
};

const getStatusClass = (value: string) => {
  return value === 'draft' ? 'status-draft' : 'status-confirmed';
};

// 批次 272：同步筛选条件到 useTableApi.queryParams 并刷新
// useTableApi 自动 watch page/pageSize 变化触发重载，无需手动 loadData
const syncQueryParams = () => {
  setQueryParam('customer_id', searchForm.value.customer_id);
  setQueryParam('status', searchForm.value.status || undefined);
  setQueryParam('start_date', searchForm.value.start_date || undefined);
  setQueryParam('end_date', searchForm.value.end_date || undefined);
};

const loadCustomers = async () => {
  try {
    // v11 批次 146 P1-4 修复：改用 customer.ts 统一封装的 getCustomerSelectList，
    // 避免绕过 API 层直接调用 request.get，并正确处理 PaginatedResponse → {label, value}[] 映射
    customerOptions.value = await getCustomerSelectList();
  } catch (error) {
    logger.warn(t('arReconciliationModule.index.loadCustomersFailed'));
  }
};

const handleSearch = () => {
  syncQueryParams();
  page.value = 1;
  loadData();
};

const handleReset = () => {
  searchForm.value = {
    customer_id: undefined,
    status: '',
    start_date: '',
    end_date: '',
  };
  syncQueryParams();
  page.value = 1;
  loadData();
};

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (val: number) => {
  page.value = val;
};

const handlePageSizeChange = (val: number) => {
  pageSize.value = val;
  page.value = 1;
};

const openAddDialog = () => {
  form.value = {
    customer_id: undefined,
    period_start: '',
    period_end: '',
  };
  dialogVisible.value = true;
};

const openEditDialog = (row: ArReconciliationEntity) => {
  // 金额来自列表真实键（Decimal 字符串），转数值供 input-number 编辑
  form.value = {
    id: row.id,
    customer_id: row.customer_id,
    period_start: row.period_start,
    period_end: row.period_end,
    opening_balance: Number(row.opening_balance),
    total_invoices: Number(row.total_invoices),
    total_collections: Number(row.total_collections),
    notes: '',
  };
  dialogVisible.value = true;
};

const openViewDialog = async (row: ArReconciliationEntity) => {
  try {
    const res = (await getArReconciliation(row.id)) as { data?: ArReconciliationEntity };
    // 安全检查：防止后端返回 data 为 null 时崩溃
    if (res.data) viewData.value = res.data;
    // 对账明细端点在增强路由：GET /ar-reconciliations-enhanced/{id}/details
    // 返回单对象 { reconciliation, details }，明细在 details 键（不是裸数组）
    const detailRes = (await getReconciliationDetailItems(row.id)) as {
      data?: { details?: ReconciliationDetailItem[] };
    };
    detailData.value = detailRes.data?.details ?? [];
    viewDialogVisible.value = true;
  } catch (error) {
    ElMessage.error(t('arReconciliationModule.index.fetchDetailFailed'));
  }
};

const handleSubmit = async () => {
  if (!form.value.customer_id || !form.value.period_start || !form.value.period_end) {
    ElMessage.warning(t('arReconciliationModule.index.requiredFieldsMissing'));
    return;
  }
  try {
    if (form.value.id) {
      // UpdateReconciliationApiRequest 全部 Option：只提交界面实际修改的四个键
      const payload: UpdateArReconciliationPayload = {
        opening_balance: form.value.opening_balance,
        total_invoices: form.value.total_invoices,
        total_collections: form.value.total_collections,
        ...(form.value.notes ? { notes: form.value.notes } : {}),
      };
      await updateArReconciliation(form.value.id, payload);
      ElMessage.success(t('common.message.updateSuccess'));
    } else {
      // 新增走后端汇总端点：单号在事务内自动生成、金额按发票/收款汇总，
      // 界面不采集也不伪造单号/金额（POST /ar-reconciliations 要求客户端提供单号与三项金额）
      await generateReconciliation({
        customer_id: form.value.customer_id,
        start_date: form.value.period_start,
        end_date: form.value.period_end,
      });
      ElMessage.success(t('common.message.createSuccess'));
    }
    dialogVisible.value = false;
    loadData();
  } catch (error) {
    ElMessage.error(t('common.message.operationFailed'));
  }
};

const handleDelete = async (row: ArReconciliationEntity) => {
  if (row.reconciliation_status === 'confirmed') {
    ElMessage.warning(t('arReconciliationModule.index.cannotDeleteConfirmed'));
    return;
  }
  try {
    await ElMessageBox.confirm(
      t('arReconciliationModule.index.deleteConfirm'),
      t('common.message.confirmTitle'),
      { type: 'warning' }
    );
    await deleteArReconciliation(row.id);
    ElMessage.success(t('common.message.deleteSuccess'));
    loadData();
  } catch (error) {
    ElMessage.info(t('arReconciliationModule.index.deleteCancelled'));
  }
};

const handleConfirm = async (row: ArReconciliationEntity) => {
  try {
    await ElMessageBox.confirm(
      t('arReconciliationModule.index.confirmReconciliationConfirm'),
      t('common.message.confirmTitle'),
      { type: 'warning' }
    );
    await confirmReconciliation(row.id);
    ElMessage.success(t('arReconciliationModule.index.confirmSuccess'));
    loadData();
  } catch (error) {
    ElMessage.info(t('arReconciliationModule.index.confirmCancelled'));
  }
};

// 批次 272：useTableApi 构造时自动初始加载，无需 setup 顶层调用 loadData
loadCustomers();
</script>

<template>
  <div class="app-container">
    <div class="filter-container">
      <ElRow :gutter="20">
        <ElCol :span="6">
          <ElSelect
            v-model="searchForm.customer_id"
            clearable
            :placeholder="$t('arReconciliationModule.index.selectCustomerPlaceholder')"
            class="filter-item"
          >
            <ElOption
              v-for="c in customerOptions"
              :key="c.value"
              :label="c.label"
              :value="c.value"
            />
          </ElSelect>
        </ElCol>
        <ElCol :span="6">
          <ElSelect
            v-model="searchForm.status"
            :placeholder="$t('arReconciliationModule.index.statusPlaceholder')"
            class="filter-item"
          >
            <ElOption v-for="s in statusOptions" :key="s.value" :label="s.label" :value="s.value" />
          </ElSelect>
        </ElCol>
        <ElCol :span="6">
          <ElDatePicker
            v-model="searchForm.start_date"
            type="date"
            value-format="YYYY-MM-DD"
            :placeholder="$t('arReconciliationModule.index.startDatePlaceholder')"
            class="filter-item"
          />
        </ElCol>
        <ElCol :span="6">
          <ElDatePicker
            v-model="searchForm.end_date"
            type="date"
            value-format="YYYY-MM-DD"
            :placeholder="$t('arReconciliationModule.index.endDatePlaceholder')"
            class="filter-item"
          />
        </ElCol>
      </ElRow>
      <div class="filter-actions">
        <ElButton type="primary" @click="handleSearch">{{
          $t('arReconciliationModule.index.query')
        }}</ElButton>
        <ElButton @click="handleReset">{{ $t('common.reset') }}</ElButton>
        <ElButton type="success" @click="openAddDialog">
          <Plus /> {{ $t('arReconciliationModule.index.addReconciliation') }}
        </ElButton>
      </div>
    </div>

    <ElTable
      :data="tableData"
      :loading="loading"
      border
      fit
      highlight-current-row
      style="width: 100%"
      :aria-label="$t('arReconciliationModule.index.listAria')"
    >
      <ElTableColumn
        prop="reconciliation_no"
        :label="$t('arReconciliationModule.index.reconciliationNo')"
        width="150"
      />
      <ElTableColumn
        prop="customer_name"
        :label="$t('arReconciliationModule.index.customerName')"
        width="150"
      />
      <ElTableColumn
        prop="period_start"
        :label="$t('arReconciliationModule.index.reconciliationStart')"
        width="120"
      />
      <ElTableColumn
        prop="period_end"
        :label="$t('arReconciliationModule.index.reconciliationEnd')"
        width="120"
      />
      <ElTableColumn
        prop="total_invoices"
        :label="$t('arReconciliationModule.index.invoiceAmount')"
        width="120"
        align="right"
      >
        <template #default="scope">{{ fmtNum(scope.row.total_invoices) }}</template>
      </ElTableColumn>
      <ElTableColumn
        prop="total_collections"
        :label="$t('arReconciliationModule.index.paymentAmount')"
        width="120"
        align="right"
      >
        <template #default="scope">{{ fmtNum(scope.row.total_collections) }}</template>
      </ElTableColumn>
      <ElTableColumn
        prop="closing_balance"
        :label="$t('arReconciliationModule.index.balance')"
        width="120"
        align="right"
      >
        <template #default="scope">{{ fmtNum(scope.row.closing_balance) }}</template>
      </ElTableColumn>
      <ElTableColumn
        prop="reconciliation_status"
        :label="$t('arReconciliationModule.index.status')"
        width="100"
      >
        <template #default="scope">
          <span :class="['status-tag', getStatusClass(scope.row.reconciliation_status)]">
            {{ getStatusLabel(scope.row.reconciliation_status) }}
          </span>
        </template>
      </ElTableColumn>
      <ElTableColumn prop="created_at" :label="$t('common.createTime')" width="150" />
      <ElTableColumn :label="$t('common.operation')" width="250" align="center">
        <template #default="scope">
          <ElButton size="small" @click="openViewDialog(scope.row as ArReconciliationEntity)">
            <View />
          </ElButton>
          <ElButton
            v-if="scope.row.reconciliation_status === 'draft'"
            size="small"
            type="primary"
            @click="openEditDialog(scope.row as ArReconciliationEntity)"
          >
            <Edit />
          </ElButton>
          <ElButton
            v-if="scope.row.reconciliation_status === 'draft'"
            size="small"
            type="warning"
            @click="handleConfirm(scope.row as ArReconciliationEntity)"
          >
            <Check /> {{ $t('arReconciliationModule.index.confirm') }}
          </ElButton>
          <ElButton
            v-if="scope.row.reconciliation_status === 'draft'"
            size="small"
            type="danger"
            @click="handleDelete(scope.row as ArReconciliationEntity)"
          >
            <Delete />
          </ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <div class="pagination-wrapper" style="margin-top: 16px; text-align: right">
      <ElPagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        :aria-label="$t('arReconciliationModule.index.paginationAria')"
        @size-change="handlePageSizeChange"
        @current-change="handlePageChange"
      />
    </div>

    <ElDialog
      :title="dialogTitle"
      v-model="dialogVisible"
      width="500px"
      :aria-label="dialogTitle"
      @close="dialogVisible = false"
    >
      <ElForm
        :model="form"
        label-width="100px"
        :aria-label="$t('arReconciliationModule.index.formAria')"
      >
        <ElFormItem :label="$t('arReconciliationModule.index.customer')" prop="customer_id">
          <ElSelect
            v-model="form.customer_id"
            :disabled="!!form.id"
            :placeholder="$t('arReconciliationModule.index.selectCustomerPlaceholder')"
          >
            <ElOption
              v-for="c in customerOptions"
              :key="c.value"
              :label="c.label"
              :value="c.value"
            />
          </ElSelect>
        </ElFormItem>
        <ElRow :gutter="20">
          <ElCol :span="12">
            <ElFormItem
              :label="$t('arReconciliationModule.index.startDateLabel')"
              prop="period_start"
            >
              <ElDatePicker
                v-model="form.period_start"
                :disabled="!!form.id"
                type="date"
                value-format="YYYY-MM-DD"
              />
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem :label="$t('arReconciliationModule.index.endDateLabel')" prop="period_end">
              <ElDatePicker
                v-model="form.period_end"
                :disabled="!!form.id"
                type="date"
                value-format="YYYY-MM-DD"
              />
            </ElFormItem>
          </ElCol>
        </ElRow>
        <!-- 编辑态：PUT 端点仅支持修改期初/发票总额/回款总额/备注（closing_balance 后端自动重算） -->
        <template v-if="form.id">
          <ElFormItem :label="$t('arReconciliationModule.index.openingBalance')">
            <ElInputNumber
              v-model="form.opening_balance"
              :precision="2"
              :controls="false"
              style="width: 100%"
            />
          </ElFormItem>
          <ElFormItem :label="$t('arReconciliationModule.index.invoiceAmount')">
            <ElInputNumber
              v-model="form.total_invoices"
              :precision="2"
              :controls="false"
              style="width: 100%"
            />
          </ElFormItem>
          <ElFormItem :label="$t('arReconciliationModule.index.paymentAmount')">
            <ElInputNumber
              v-model="form.total_collections"
              :precision="2"
              :controls="false"
              style="width: 100%"
            />
          </ElFormItem>
          <ElFormItem :label="$t('arReconciliationModule.remark')">
            <ElInput
              v-model="form.notes"
              type="textarea"
              :rows="2"
              :placeholder="$t('arReconciliationModule.remark')"
            />
          </ElFormItem>
        </template>
      </ElForm>
      <template #footer>
        <ElButton @click="dialogVisible = false">{{ $t('common.cancel') }}</ElButton>
        <ElButton type="primary" @click="handleSubmit">{{ $t('common.confirm') }}</ElButton>
      </template>
    </ElDialog>

    <ElDialog
      :title="$t('arReconciliationModule.index.detailTitle')"
      v-model="viewDialogVisible"
      width="800px"
      :aria-label="$t('arReconciliationModule.index.detailTitle')"
      @close="viewDialogVisible = false"
    >
      <div v-if="viewData">
        <ElDescriptions :column="4" border>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.reconciliationNo')">
            {{ viewData.reconciliation_no }}
          </ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.customerName')">
            {{ viewData.customer_name || '-' }}
          </ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.reconciliationStart')">
            {{ viewData.period_start }}
          </ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.reconciliationEnd')">
            {{ viewData.period_end }}
          </ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.openingBalance')">
            {{ fmtNum(viewData.opening_balance) }}
          </ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.invoiceAmount')">
            {{ fmtNum(viewData.total_invoices) }}
          </ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.paymentAmount')">
            {{ fmtNum(viewData.total_collections) }}
          </ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.balance')">
            {{ fmtNum(viewData.closing_balance) }}
          </ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('arReconciliationModule.index.status')">
            {{ getStatusLabel(viewData.reconciliation_status) }}
          </ElDescriptionsItem>
          <ElDescriptionsItem :label="$t('common.createTime')">
            {{ viewData.created_at }}
          </ElDescriptionsItem>
        </ElDescriptions>

        <div style="margin-top: 20px">
          <h4>{{ $t('arReconciliationModule.index.detailItems') }}</h4>
          <ElTable
            :data="detailData"
            border
            style="width: 100%"
            :aria-label="$t('arReconciliationModule.index.detailItemsAria')"
          >
            <ElTableColumn
              prop="item_type"
              :label="$t('arReconciliationModule.index.type')"
              width="100"
            />
            <ElTableColumn
              prop="document_no"
              :label="$t('arReconciliationModule.index.sourceNo')"
              width="150"
            />
            <ElTableColumn
              prop="document_date"
              :label="$t('arReconciliationModule.index.date')"
              width="120"
            />
            <ElTableColumn
              prop="amount"
              :label="$t('arReconciliationModule.index.amount')"
              width="120"
              align="right"
            >
              <template #default="scope">{{ fmtNum(scope.row.amount) }}</template>
            </ElTableColumn>
            <ElTableColumn
              prop="matched_amount"
              :label="$t('arReconciliationModule.matchedAmount')"
              width="120"
              align="right"
            >
              <template #default="scope">{{ fmtNum(scope.row.matched_amount) }}</template>
            </ElTableColumn>
            <ElTableColumn
              prop="match_status"
              :label="$t('arReconciliationModule.matchStatus')"
              width="100"
            />
            <ElTableColumn prop="remarks" :label="$t('common.description')" />
          </ElTable>
        </div>
      </div>
    </ElDialog>
  </div>
</template>

<style scoped>
.app-container {
  padding: 20px;
}

.filter-container {
  margin-bottom: 20px;
}

.filter-item {
  width: 100%;
}

.filter-actions {
  margin-top: 10px;
}

.status-tag {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
}

.status-draft {
  background: #f5f7fa;
  color: #909399;
}

.status-confirmed {
  background: #f0f9eb;
  color: #67c23a;
}
</style>
