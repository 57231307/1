<!--
  报价单列表页
  - 筛选（客户/状态）
  - 表格 + 分页
  - 行操作：查看 / 编辑（draft, rejected） / 转订单（approved） / 取消（draft）
-->
<template>
  <div class="quotation-list">
    <el-card>
      <template #header>
        <div class="card-header">
          <span class="title">{{ t('quotations.list.title') }}</span>
          <div style="display: flex; gap: 8px">
            <el-button @click="showExpiring(false)">
              {{ t('quotations.list.expiringSoon') }}
            </el-button>
            <el-button @click="showExpiring(true)">
              {{ t('quotations.list.expired') }}
            </el-button>
            <el-button type="primary" @click="$router.push('/quotations/new')">
              <el-icon><Plus /></el-icon>
              {{ t('quotations.list.createNew') }}
            </el-button>
          </div>
        </div>
      </template>

      <!-- 筛选区 -->
      <el-form
        :inline="true"
        :model="filters"
        class="filter-form"
        :aria-label="t('quotations.list.filterAriaLabel')"
      >
        <el-form-item :label="t('quotations.list.labelCustomer')">
          <el-select
            v-model="filters.customer_id"
            clearable
            filterable
            :placeholder="t('quotations.list.allCustomers')"
            style="width: 200px"
          >
            <el-option
              v-for="c in customers"
              :key="c.id"
              :label="c.customer_name || c.name"
              :value="c.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('quotations.list.labelStatus')">
          <el-select
            v-model="filters.status"
            clearable
            :placeholder="t('quotations.list.allStatus')"
            style="width: 160px"
          >
            <el-option
              v-for="(label, value) in QUOTATION_STATUS_LABELS"
              :key="value"
              :label="label"
              :value="value"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">{{
            t('quotations.list.search')
          }}</el-button>
          <el-button @click="handleReset">{{ t('quotations.list.reset') }}</el-button>
        </el-form-item>
      </el-form>

      <!-- 列表 -->
      <el-table
        v-loading="loading"
        :data="quotations"
        stripe
        border
        style="width: 100%"
        :empty-text="t('quotations.list.emptyText')"
        :aria-label="t('quotations.list.tableAriaLabel')"
      >
        <el-table-column
          prop="quotation_no"
          :label="t('quotations.list.colQuotationNo')"
          width="170"
        />
        <el-table-column :label="t('quotations.list.colCustomer')" min-width="160">
          <template #default="{ row }">
            {{ row.customer_name || row.customer_id }}
          </template>
        </el-table-column>
        <el-table-column
          prop="quotation_date"
          :label="t('quotations.list.colQuotationDate')"
          width="120"
        />
        <el-table-column
          prop="valid_until"
          :label="t('quotations.list.colValidUntil')"
          width="120"
        />
        <el-table-column :label="t('quotations.list.colPriceTerms')" width="80">
          <template #default="{ row }">{{ row.price_terms }}</template>
        </el-table-column>
        <el-table-column :label="t('quotations.list.colAmount')" width="160" align="right">
          <template #default="{ row }">
            {{ row.currency }} {{ formatAmount(row.total_amount) }}
          </template>
        </el-table-column>
        <el-table-column :label="t('quotations.list.colStatus')" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="tagType(row.status as QuotationStatus)">
              {{ QUOTATION_STATUS_LABELS[row.status as QuotationStatus] || row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('quotations.list.colAction')" width="340" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" @click="goDetail(row)">{{
              t('quotations.list.view')
            }}</el-button>
            <el-button
              v-if="row.status === 'draft' || row.status === 'rejected'"
              v-permission="'quotation:update'"
              link
              type="primary"
              @click="goEdit(row)"
            >
              {{ t('quotations.list.edit') }}
            </el-button>
            <el-button
              v-if="row.status === 'approved'"
              link
              type="success"
              @click="handleConvert(row)"
            >
              {{ t('quotations.list.convertOrder') }}
            </el-button>
            <el-button
              v-if="row.status === 'draft'"
              v-permission="'quotation:cancel'"
              link
              type="danger"
              @click="handleCancel(row)"
            >
              {{ t('quotations.list.cancel') }}
            </el-button>
            <el-button link type="warning" @click="showTerms(row)">
              {{ t('quotations.list.terms') }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        :aria-label="t('quotations.list.paginationAriaLabel')"
        @current-change="onPageChange"
        @size-change="onSizeChange"
      />
    </el-card>

    <!-- 到期/过期报价弹窗（getExpiringQuotationList / getExpiredQuotationList） -->
    <el-dialog v-model="expiringVisible" :title="t('quotations.list.expiringSoon')" width="720px">
      <el-table :data="expiringRows" border size="small" max-height="420">
        <el-table-column prop="quotation_no" :label="t('quotations.list.colNo')" min-width="150" />
        <el-table-column
          prop="customer_name"
          :label="t('quotations.list.colCustomer')"
          min-width="140"
        />
        <el-table-column prop="valid_until" label="有效期至" width="120" />
        <el-table-column prop="status" label="状态" width="100" />
      </el-table>
    </el-dialog>

    <!-- 贸易条款查看/维护弹窗（getQuotationTerms / setQuotationTerms） -->
    <el-dialog v-model="termsVisible" :title="t('quotations.list.terms')" width="600px">
      <el-table v-loading="termsLoading" :data="termsRows" border size="small">
        <el-table-column prop="term_type" label="条款类型" width="120" />
        <el-table-column prop="term_value" label="条款内容" min-width="220" />
        <el-table-column :label="t('common.action')" width="90">
          <template #default="{ row }">
            <el-button size="small" type="danger" link @click="removeTerm(row)">
              {{ t('common.delete') }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>
      <div style="display: flex; gap: 8px; margin-top: 12px">
        <el-input v-model="newTermType" placeholder="条款类型（如 PAYMENT）" style="width: 200px" />
        <el-input v-model="newTermContent" placeholder="条款内容" />
        <el-button type="primary" :loading="termsSaving" @click="addTerm">
          {{ t('common.save') }}
        </el-button>
      </div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
// 报价单列表页脚本
// - 列表加载
// - 行操作：查看/编辑/转订单/取消
import { ref, reactive, onMounted } from 'vue';
import { logger } from '@/utils/logger';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import { useTableApi } from '@/composables/useTableApi';
import {
  cancelQuotation,
  convertQuotation,
  getQuotationTerms,
  setQuotationTerms,
  getExpiringQuotationList,
  getExpiredQuotationList,
  QUOTATION_STATUS_LABELS,
  QUOTATION_STATUS_TAG_TYPES,
  type QuotationResponseDto,
  type QuotationStatus,
  type CreateQuotationTermDto,
  type TermType,
} from '@/api/quotation';
import { getCustomerList } from '@/api/customer';

/** el-tag 类型联合（与 element-plus TagProps.type 对齐） */
type TagType = '' | 'success' | 'warning' | 'info' | 'danger';

/** 计算状态对应的 el-tag 类型 */
function tagType(s: QuotationStatus): TagType {
  return (QUOTATION_STATUS_TAG_TYPES[s] || '') as TagType;
}

const { t } = useI18n({ useScope: 'global' });

const router = useRouter();
const customers = ref<Array<{ id: number; customer_name?: string; name?: string }>>([]);

const filters = reactive({
  customer_id: undefined as number | undefined,
  status: undefined as QuotationStatus | undefined,
});

// 批次 268：接入 useTableApi，消除手写 pagination + loadData 重复
// API 返回兼容数组或 { list/items, total }，useTableApi detectList 自动探测
const {
  data: quotations,
  loading,
  page,
  pageSize,
  total,
  refresh: loadData,
  setQueryParam,
} = useTableApi<QuotationResponseDto>({
  url: '/quotations',
  // 钉到后端真实键（quotation_handler::list_quotations → ListQuotationsResponse{ list, total }）
  listKey: 'list',
  onError: (e: unknown) =>
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('quotations.list.loadFailed')
    ),
});

/** 同步筛选条件到 useTableApi.queryParams */
function syncQueryParams() {
  setQueryParam('customer_id', filters.customer_id);
  setQueryParam('status', filters.status);
}

/** 加载客户下拉 */
async function loadCustomers() {
  try {
    const res = await getCustomerList({ page: 1, page_size: 1000 });
    // 后端返回 PaginatedResponse { items, total }，兼容历史 list 字段
    const payload = res.data as {
      items?: Array<{ id: number; customer_name?: string; name?: string }>;
      list?: Array<{ id: number; customer_name?: string; name?: string }>;
    } | null;
    customers.value = payload?.items || payload?.list || [];
  } catch (error) {
    logger.error(t('quotations.list.customerLoadFailed'), error);
    customers.value = [];
  }
}

function handleSearch() {
  syncQueryParams();
  page.value = 1;
  loadData();
}

function handleReset() {
  filters.customer_id = undefined;
  filters.status = undefined;
  handleSearch();
}

// 批次 268：分页变化（useTableApi 自动 watch 重载，此处无需手动调用）
function onPageChange(_p: number) {
  // useTableApi watch page 自动触发 refresh
}

function onSizeChange(s: number) {
  pageSize.value = s;
  page.value = 1;
}

function goDetail(row: QuotationResponseDto) {
  router.push(`/quotations/${row.id}`);
}

function goEdit(row: QuotationResponseDto) {
  router.push(`/quotations/${row.id}/edit`);
}

async function handleCancel(row: QuotationResponseDto) {
  try {
    await ElMessageBox.confirm(
      t('quotations.list.cancelConfirmText', { no: row.quotation_no }),
      t('quotations.list.cancelConfirmTitle'),
      {
        type: 'warning',
      }
    );
  } catch {
    return;
  }
  await cancelQuotation(row.id);
  ElMessage.success(t('quotations.list.cancelSuccess'));
  loadData();
}

async function handleConvert(row: QuotationResponseDto) {
  try {
    await ElMessageBox.confirm(
      t('quotations.list.convertConfirmText', { no: row.quotation_no }),
      t('quotations.list.convertConfirmTitle'),
      { type: 'warning' }
    );
  } catch {
    return;
  }
  const res = await convertQuotation(row.id);
  const order = res.data;
  ElMessage.success(t('quotations.list.convertSuccess', { id: order?.id }));
  if (order?.id) {
    router.push(`/sales/orders/${order.id}`);
  } else {
    loadData();
  }
}

/** 金额格式化（保留 2 位 + 千分位） */
function formatAmount(value?: number): string {
  if (value === undefined || value === null) return '0.00';
  return Number(value).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

// 批次 268：useTableApi 构造时自动初始加载列表，onMounted 仅加载客户下拉
onMounted(() => {
  loadCustomers();
});

// 贸易条款维护（getQuotationTerms / setQuotationTerms）
const termsVisible = ref(false);
const termsLoading = ref(false);
const termsSaving = ref(false);
const termsRows = ref<Array<Record<string, unknown>>>([]);
const currentQuotationId = ref<number | null>(null);
const newTermType = ref('');
const newTermContent = ref('');

const showTerms = async (row: QuotationResponseDto) => {
  currentQuotationId.value = row.id;
  termsVisible.value = true;
  termsLoading.value = true;
  try {
    const res = await getQuotationTerms(row.id);
    termsRows.value = (res.data as unknown as Array<Record<string, unknown>>) || [];
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    termsLoading.value = false;
  }
};

/** 将条款行/表单输入映射为后端 CreateQuotationTermDto（sequence 按顺序重排） */
const mapTermDto = (row: Record<string, unknown>, idx: number): CreateQuotationTermDto => ({
  term_type: String(row.term_type || '') as TermType,
  term_key: String(row.term_key ?? row.term_type ?? ''),
  term_value: String(row.term_value ?? row.content ?? ''),
  sequence: idx + 1,
});

const addTerm = async () => {
  if (!currentQuotationId.value) return;
  if (!newTermType.value.trim() || !newTermContent.value.trim()) {
    ElMessage.warning(t('quotations.list.termRequired'));
    return;
  }
  termsSaving.value = true;
  try {
    const newTerm: CreateQuotationTermDto = {
      term_type: newTermType.value.trim() as TermType,
      term_key: newTermType.value.trim(),
      term_value: newTermContent.value.trim(),
      sequence: termsRows.value.length + 1,
    };
    await setQuotationTerms(currentQuotationId.value, [
      ...termsRows.value.map((row, idx) => mapTermDto(row, idx)),
      newTerm,
    ]);
    ElMessage.success(t('common.success'));
    newTermType.value = '';
    newTermContent.value = '';
    const res = await getQuotationTerms(currentQuotationId.value);
    termsRows.value = (res.data as unknown as Array<Record<string, unknown>>) || [];
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    termsSaving.value = false;
  }
};

const removeTerm = async (row: Record<string, unknown>) => {
  if (!currentQuotationId.value) return;
  const remain = (termsRows.value as Array<{ id?: number }>).filter(r => r.id !== row.id);
  termsSaving.value = true;
  try {
    await setQuotationTerms(
      currentQuotationId.value,
      remain.map((r, idx) => mapTermDto(r, idx))
    );
    ElMessage.success(t('common.success'));
    termsRows.value = remain as unknown as Array<Record<string, unknown>>;
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    termsSaving.value = false;
  }
};

// 到期/过期报价查询（弹窗展示，用于催单提醒）
const expiringVisible = ref(false);
const expiringRows = ref<QuotationResponseDto[]>([]);
const showExpiring = async (expired: boolean) => {
  expiringVisible.value = true;
  expiringRows.value = [];
  try {
    const res = expired ? await getExpiredQuotationList() : await getExpiringQuotationList();
    const d = res.data as unknown as
      QuotationResponseDto[] | { list?: QuotationResponseDto[]; items?: QuotationResponseDto[] };
    expiringRows.value = Array.isArray(d) ? d : d?.list || d?.items || [];
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
};
</script>

<style scoped>
.quotation-list {
  padding: 16px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.title {
  font-size: 18px;
  font-weight: 600;
}
.filter-form {
  margin-bottom: 16px;
}
</style>
