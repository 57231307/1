<!--
  customer/index.vue - 客户管理主入口
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-3）：
  原 551 行"上帝组件"已拆分为：
  - tabs/CustomerFormTab.vue - 新建/编辑客户对话框

  本主入口承担：页面布局 + 列表数据 + 公共样式。
-->
<template>
  <div class="customer-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ $t('customer.index.title') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{
            $t('customer.index.breadcrumb.home')
          }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('customer.index.breadcrumb.basicData') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('customer.index.breadcrumb.customer') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <!-- P2-10 修复（批次 82 v1 复审）：补齐 v-permission 按钮权限 -->
        <el-button
          v-permission="PERMISSIONS.CUSTOMER_CREATE"
          type="primary"
          @click="openCreateDialog"
        >
          <el-icon><Plus /></el-icon>
          {{ $t('customer.index.button.create') }}
        </el-button>
        <el-button v-permission="PERMISSIONS.CUSTOMER_READ" @click="handlePrint">
          <el-icon><Printer /></el-icon>
          {{ $t('customer.index.button.print') }}
        </el-button>
        <el-button
          v-permission="EXPORT_APPROVAL_REQUEST_PERMISSION_KEY"
          :loading="exportSubmitting"
          @click="handleExport"
        >
          <el-icon><Download /></el-icon>
          {{ $t('customer.index.button.export') }}
        </el-button>
        <!-- 就地下载本人在本资源下已批准的导出（令牌随申请落库，批准后回到本页下载） -->
        <el-button
          v-permission="EXPORT_APPROVAL_REQUEST_PERMISSION_KEY"
          :loading="approvedDownloading"
          @click="handleDownloadMine"
        >
          <el-icon><Download /></el-icon>
          {{ $t('exportApprovals.downloadMine') }}
        </el-button>
        <el-button @click="router.push('/crm/pool')">
          <el-icon><Coin /></el-icon>
          {{ $t('customer.index.button.pool') }}
        </el-button>
        <el-button @click="router.push('/crm/assignment')">
          <el-icon><Share /></el-icon>
          {{ $t('customer.index.button.assignment') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="queryParams"
        class="filter-form"
        :aria-label="$t('customer.index.filter.ariaLabel')"
      >
        <el-form-item :label="$t('customer.index.filter.keyword')">
          <el-input
            v-model="queryParams.keyword"
            :placeholder="$t('customer.index.filter.keywordPlaceholder')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="$t('customer.index.filter.customerType')">
          <el-select
            v-model="queryParams.customer_type"
            :placeholder="$t('customer.index.filter.customerTypePlaceholder')"
            clearable
          >
            <!-- value 取后端唯一词表 constants::customer_type::ALLOWED 的五个渠道 token；
                 分层词 normal/vip 不属本列（后端 400 拒绝），少列一个合法值就筛不出该类客户。 -->
            <el-option :label="$t('customer.index.filterOption.typeRetail')" value="retail" />
            <el-option :label="$t('customer.index.filterOption.typeWholesale')" value="wholesale" />
            <el-option
              :label="$t('customer.index.filterOption.typeDistributor')"
              value="distributor"
            />
            <el-option
              :label="$t('customer.index.filterOption.typeManufacturer')"
              value="manufacturer"
            />
            <el-option :label="$t('customer.index.filterOption.typeOther')" value="other" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('customer.index.filter.status')">
          <el-select
            v-model="queryParams.status"
            :placeholder="$t('customer.index.filter.statusPlaceholder')"
            clearable
          >
            <el-option :label="$t('customer.index.filterOption.statusActive')" value="active" />
            <el-option :label="$t('customer.index.filterOption.statusInactive')" value="inactive" />
            <el-option
              :label="$t('customer.index.filterOption.statusBlacklist')"
              value="blacklist"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">{{
            $t('customer.index.button.query')
          }}</el-button>
          <el-button @click="handleReset">{{ $t('customer.index.button.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table
        v-loading="loading"
        :data="customers"
        stripe
        :aria-label="$t('customer.index.table.ariaLabel')"
      >
        <el-table-column
          prop="customer_code"
          :label="$t('customer.index.table.column.customerCode')"
          width="120"
          fixed
        />
        <el-table-column
          prop="customer_name"
          :label="$t('customer.index.table.column.customerName')"
          min-width="180"
          fixed
        >
          <template #default="{ row }">
            <el-button type="primary" link @click="openDetail(row)">{{
              row.customer_name
            }}</el-button>
          </template>
        </el-table-column>
        <el-table-column
          prop="contact_person"
          :label="$t('customer.index.table.column.contactPerson')"
          width="100"
        />
        <el-table-column
          prop="contact_phone"
          :label="$t('customer.index.table.column.phone')"
          width="130"
        />
        <el-table-column
          prop="contact_email"
          :label="$t('customer.index.table.column.email')"
          width="180"
          show-overflow-tooltip
        />
        <el-table-column
          prop="customer_type"
          :label="$t('customer.index.table.column.type')"
          width="100"
        >
          <template #default="{ row }">
            <el-tag :type="getCustomerTypeTag(row.customer_type)" size="small">
              {{ getCustomerTypeLabel(row.customer_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('customer.index.table.column.tag')" min-width="150">
          <template #default="{ row }">
            <template v-if="tagsCellView(row.id).state === 'ready'">
              <el-tag
                v-for="tag in tagsCellView(row.id).tags"
                :key="tag.id"
                size="small"
                class="table-tag"
              >
                {{ tag.name }}
              </el-tag>
              <span v-if="tagsCellView(row.id).tags.length === 0" class="no-tags">-</span>
            </template>
            <span v-else-if="tagsCellView(row.id).state === 'loading'" class="no-tags">{{
              $t('customer.index.table.cell.tagsLoading')
            }}</span>
            <span v-else class="cell-load-failed">{{
              $t('customer.index.table.cell.tagsLoadFailed')
            }}</span>
          </template>
        </el-table-column>
        <el-table-column :label="$t('customer.index.table.column.owner')" width="100">
          <template #default="{ row }">{{ ownerDisplay(row.owner_id) }}</template>
        </el-table-column>
        <el-table-column
          prop="province"
          :label="$t('customer.index.table.column.province')"
          width="100"
        />
        <el-table-column
          prop="credit_limit"
          :label="$t('customer.index.table.column.creditLimit')"
          width="120"
          align="right"
        >
          <template #default="{ row }">
            {{ row.credit_limit ? formatCurrency(row.credit_limit) : '-' }}
          </template>
        </el-table-column>
        <el-table-column
          prop="payment_terms"
          :label="$t('customer.index.table.column.paymentTerms')"
          width="90"
          align="center"
        />
        <el-table-column prop="status" :label="$t('customer.index.table.column.status')" width="80">
          <template #default="{ row }">
            <el-tag :type="getStatusTag(row.status)" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          :label="$t('customer.index.table.column.action')"
          width="180"
          fixed="right"
        >
          <template #default="{ row }">
            <!-- P3 维度 10 修复（批次 87）：编辑/删除按钮补齐 v-permission -->
            <el-button
              v-permission="PERMISSIONS.CUSTOMER_UPDATE"
              type="primary"
              link
              size="small"
              @click="openEditDialog(row)"
              >{{ $t('customer.index.button.edit') }}</el-button
            >
            <el-button type="primary" link size="small" @click="openDetail(row)">{{
              $t('customer.index.button.detail')
            }}</el-button>
            <el-button
              v-permission="PERMISSIONS.CUSTOMER_DELETE"
              type="danger"
              link
              size="small"
              @click="handleDelete(row)"
              >{{ $t('customer.index.button.delete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="$t('customer.index.table.paginationAria')"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <CustomerFormTab
      v-model="formDialogVisible"
      :title="formDialogTitle"
      :row-data="currentRow"
      @submitted="handleFormSubmitted"
    />
  </div>
</template>

<script setup lang="ts">
import { isDialogDismissal } from '@/utils/monitor';
import { ref, reactive, computed, watch, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus, Download, Printer, Coin, Share } from '@element-plus/icons-vue';
import {
  deleteCustomer,
  getCustomerTags,
  type Customer,
  type CustomerTagBrief,
} from '@/api/customer';
import { getSalesUserList, type SalesUser } from '@/api/crm-enhanced';
import { formatCurrency } from '@/utils';
import { EXPORT_APPROVAL_REQUEST_PERMISSION_KEY } from '@/constants/export-approval';
import { useExportApproval } from '@/composables/useExportApproval';
import { printData } from '@/utils/print';
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { useTableApi } from '@/composables/useTableApi';
// Batch 468 P0-S28：引入权限码常量，与后端 customers 资源对齐
import { PERMISSIONS } from '@/constants/permissions';
import CustomerFormTab from './tabs/CustomerFormTab.vue';

const { t } = useI18n({ useScope: 'global' });

const router = useRouter();
const hasLoaded = createLazyLoader();

// 行内没有 tags / owner_name 键：标签经单客户端点逐行取数，负责人经销售用户集合联显
type CustomerRow = Customer & { owner_id: number };

const formDialogVisible = ref(false);
const formDialogTitle = ref(t('customer.index.dialog.createTitle'));
const currentRow = ref<Customer | null>(null);

const queryParams = reactive({
  keyword: '',
  customer_type: '',
  status: '',
});

// 批次 276：接入 useTableApi，消除手写 customers/total/loading/fetchData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: customers,
  loading,
  page,
  pageSize,
  total,
  refresh: fetchData,
  setQueryParam,
} = useTableApi<CustomerRow>({
  url: '/crm/customers',
  // 钉到后端真实键（customer_handler::list_customers → PaginatedResponse{ items, total, page, page_size }）
  listKey: 'items',
  onError: (err: unknown) =>
    ElMessage.error(
      (err instanceof Error ? err.message : String(err)) ||
        t('customer.index.message.fetchListFailed')
    ),
});

// 批次 276：同步筛选条件到 useTableApi.queryParams 并刷新
const syncQueryParams = () => {
  setQueryParam('keyword', queryParams.keyword || undefined);
  setQueryParam('customer_type', queryParams.customer_type || undefined);
  setQueryParam('status', queryParams.status || undefined);
};

const handleQuery = () => {
  syncQueryParams();
  page.value = 1;
  fetchData();
};

const handleReset = () => {
  queryParams.keyword = '';
  queryParams.customer_type = '';
  queryParams.status = '';
  syncQueryParams();
  page.value = 1;
  fetchData();
};

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p;
};

const handleSizeChange = (s: number) => {
  pageSize.value = s;
  page.value = 1;
};

// 回读值是后端渠道 token（constants::customer_type::ALLOWED 五值），映射表按同一词表逐值取文案；
// 文案复用同命名空间的 filterOption.type*。未知/历史脏值不吞不造假名，回落显示原始 token。
// 入参非可选：api/customer.ts 的 Customer.customer_type 跟随 DB 的 NOT NULL 列，恒有值。
const getCustomerTypeLabel = (type: string) => {
  const labelMap: Record<string, string> = {
    retail: t('customer.index.filterOption.typeRetail'),
    wholesale: t('customer.index.filterOption.typeWholesale'),
    distributor: t('customer.index.filterOption.typeDistributor'),
    manufacturer: t('customer.index.filterOption.typeManufacturer'),
    other: t('customer.index.filterOption.typeOther'),
  };
  return labelMap[type] || type;
};

// tag 颜色仅供列表视觉区分，不承载状态/层级语义；同一 token 在四个页面各自配色不影响契约。
const getCustomerTypeTag = (type: string) => {
  const typeMap: Record<string, string> = {
    retail: '',
    wholesale: 'success',
    distributor: 'warning',
    manufacturer: 'info',
    other: 'danger',
  };
  return typeMap[type] || '';
};

// 状态词表按写入方全量覆盖（backend models/customer.rs = active/inactive/blacklist 三态）；
// blacklist 若落进 inactive 分支会被误显为"停用"。未知历史脏值回落原始 token，不造假名。
const getStatusLabel = (status: string) => {
  const labels: Record<string, string> = {
    active: t('customer.index.statusLabel.active'),
    inactive: t('customer.index.statusLabel.inactive'),
    blacklist: t('customer.index.statusLabel.blacklist'),
  };
  return labels[status] || status;
};

const getStatusTag = (status: string) => {
  const typeMap: Record<string, string> = {
    active: 'success',
    inactive: 'info',
    blacklist: 'danger',
  };
  return typeMap[status] || 'info';
};

// 标签列数据源：GET /crm/customers/{id}/tags 无批量出口，按当前页行逐 id 取数；
// 失败标记 failed 并在单元格显式呈现，不把"取数失败"伪装成"没有标签"。
type TagsCellState =
  { status: 'loading' } | { status: 'failed' } | { status: 'ready'; tags: CustomerTagBrief[] };
const tagsStates = ref(new Map<number, TagsCellState>());
const tagsInFlight = new Set<number>();

const loadTagsForRow = (id: number) => {
  if (tagsStates.value.has(id) || tagsInFlight.has(id)) return;
  tagsInFlight.add(id);
  tagsStates.value.set(id, { status: 'loading' });
  getCustomerTags(id)
    .then(res => {
      tagsStates.value.set(id, { status: 'ready', tags: res.data });
    })
    .catch((error: unknown) => {
      tagsStates.value.set(id, { status: 'failed' });
      logAuxLoadFailure(t('customer.index.table.cell.tagsLoadFailed'), error);
    })
    .finally(() => {
      tagsInFlight.delete(id);
    });
};

watch(
  customers,
  rows => {
    rows.forEach(row => loadTagsForRow(row.id));
  },
  { immediate: true }
);

const tagsCellView = (
  id: number
): { state: 'loading' | 'failed' | 'ready'; tags: CustomerTagBrief[] } => {
  const cell = tagsStates.value.get(id);
  if (!cell || cell.status === 'loading') return { state: 'loading', tags: [] };
  if (cell.status === 'failed') return { state: 'failed', tags: [] };
  return { state: 'ready', tags: cell.tags };
};

// 负责人列：/crm/sales-users 单次拉全量后本地联显；owner_id=0 为公海未分配。
// 非 0 但不在集合内显示 #id，让数据漂移可见而不是臆造姓名或吞成空。
const salesUsers = ref<SalesUser[]>([]);
const ownerLoadState = ref<'loading' | 'ready' | 'failed'>('loading');
const ownerNameById = computed(
  () => new Map(salesUsers.value.map(u => [u.id, u.real_name || u.username]))
);

const fetchOwners = async () => {
  try {
    const res = await getSalesUserList();
    salesUsers.value = res.data;
    ownerLoadState.value = 'ready';
  } catch (error) {
    ownerLoadState.value = 'failed';
    logAuxLoadFailure(t('customer.index.table.cell.ownerLoadFailed'), error);
  }
};

const ownerDisplay = (ownerId: number): string => {
  if (ownerId === 0) return t('customer.index.table.cell.ownerUnassigned');
  if (ownerLoadState.value === 'loading') return t('customer.index.table.cell.ownerLoading');
  if (ownerLoadState.value === 'failed') return t('customer.index.table.cell.ownerLoadFailed');
  return ownerNameById.value.get(ownerId) ?? `#${ownerId}`;
};

const openCreateDialog = () => {
  currentRow.value = null;
  formDialogTitle.value = t('customer.index.dialog.createTitle');
  formDialogVisible.value = true;
};

const openEditDialog = (row: Customer) => {
  currentRow.value = row;
  formDialogTitle.value = t('customer.index.dialog.editTitle');
  formDialogVisible.value = true;
};

// 客户详情唯一入口：客户 360 路由（/crm/detail/:id），列表不再维护第二份只读详情弹窗。
const openDetail = (row: Customer) => {
  router.push(`/crm/detail/${row.id}`);
};

const handleFormSubmitted = () => {
  formDialogVisible.value = false;
  fetchData();
};

const handleDelete = async (row: Customer) => {
  try {
    await ElMessageBox.confirm(
      t('customer.index.dialog.deleteConfirmMessage', { name: row.customer_name }),
      t('customer.index.dialog.deleteConfirmTitle'),
      {
        type: 'warning',
      }
    );
    await deleteCustomer(row.id);
    ElMessage.success(t('customer.index.message.deleteSuccess'));
    fetchData();
  } catch (error) {
    if (!isDialogDismissal(error)) {
      const err = error as Error;
      ElMessage.error(err.message || t('customer.index.message.deleteFailed'));
    }
  }
};

// 客户清单属敏感导出：后端 GET /crm/customers/export 以 download_token 做 fail-closed
// 审批令牌校验，直连必 403。导出按钮改为提交审批申请（useExportApproval 统一入口，
// 参数形状由 constants/export-approval 注册表收敛），estimated_rows 传当前筛选命中总数，
// 决定后端风险等级与审批层级。批准后由「下载我的导出」按钮在本页就地下载（走
// downloadApprovedExport 拉取本人该资源下可用令牌申请，不再集中到审批页下载）。
const { exportSubmitting, requestExport, approvedDownloading, downloadApprovedExport } =
  useExportApproval();
const handleExport = () =>
  requestExport(
    'customer',
    {
      keyword: queryParams.keyword,
      customer_type: queryParams.customer_type,
      status: queryParams.status,
    },
    total.value
  );
const handleDownloadMine = () => downloadApprovedExport('customer');

const handlePrint = () => {
  printData({
    title: t('customer.index.print.title'),
    resourceType: 'customer',
    documentKey: 'customer',
    columns: [
      {
        key: 'customer_code',
        title: t('customer.index.table.column.customerCode'),
        width: '100px',
      },
      { key: 'customer_name', title: t('customer.index.table.column.customerName') },
      {
        key: 'contact_person',
        title: t('customer.index.table.column.contactPerson'),
        width: '80px',
      },
      { key: 'contact_phone', title: t('customer.index.table.column.phone'), width: '120px' },
      {
        key: 'customer_type',
        title: t('customer.index.table.column.type'),
        width: '80px',
        // 打印列值取自 Record<string, unknown>，缺值时留空而不是 String(v) 打成字面量
        // "undefined"；有值一律按渠道词表取文案（不编造渠道名）。
        formatter: v => (v == null ? '' : getCustomerTypeLabel(String(v))),
      },
      {
        key: 'owner_id',
        title: t('customer.index.table.column.owner'),
        width: '80px',
        formatter: v => (v == null ? '' : ownerDisplay(Number(v))),
      },
      {
        key: 'status',
        title: t('customer.index.table.column.status'),
        width: '60px',
        formatter: v => (typeof v === 'string' ? getStatusLabel(v) : ''),
      },
    ],
    data: customers.value as unknown as Record<string, unknown>[],
  });
  logger.info(t('customer.index.print.logGenerated'));
};

onMounted(() => {
  loadIfNot('owners', fetchOwners, hasLoaded);
});
</script>

<style scoped>
.customer-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}
.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}
.header-actions {
  display: flex;
  gap: 12px;
}
.filter-card {
  margin-bottom: 20px;
}
.table-card {
  margin-bottom: 20px;
}
.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
.table-tag {
  border: none;
  margin-right: 4px;
}
.no-tags {
  color: #909399;
  font-size: 12px;
}
.cell-load-failed {
  color: #f56c6c;
  font-size: 12px;
}
</style>
