<!--
  CustomerListTab.vue - CRM 客户列表 Tab
  读写走标准客户域端点（落/读 customers 表），界面文案经 locales 的 crmCustomer.* 命名空间。
-->
<template>
  <div class="customer-list-tab">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ $t('crmCustomer.title') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{
            $t('crmCustomer.breadcrumb.home')
          }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('crmCustomer.breadcrumb.crm') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('crmCustomer.breadcrumb.customerList') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          {{ $t('crmCustomer.create') }}
        </el-button>
        <el-button v-permission="PERMISSIONS.CUSTOMER_READ" @click="handlePrint">
          <el-icon><Printer /></el-icon>
          {{ $t('crmCustomer.print') }}
        </el-button>
        <el-button
          v-permission="EXPORT_APPROVAL_REQUEST_PERMISSION_KEY"
          :loading="exportSubmitting"
          @click="handleExport"
        >
          <el-icon><Download /></el-icon>
          {{ $t('crmCustomer.export') }}
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
          {{ $t('crmCustomer.pool') }}
        </el-button>
        <el-button @click="router.push('/crm/assignment')">
          <el-icon><Share /></el-icon>
          {{ $t('crmCustomer.assignment') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="queryParams"
        class="filter-form"
        :aria-label="$t('crmCustomer.filter.ariaLabel')"
      >
        <el-form-item :label="$t('crmCustomer.filter.keyword')">
          <el-input
            v-model="queryParams.keyword"
            :placeholder="$t('crmCustomer.filter.keywordPlaceholder')"
            clearable
            @clear="handleQuery"
            @keyup.enter="handleQuery"
          />
        </el-form-item>
        <el-form-item :label="$t('crmCustomer.filter.customerType')">
          <el-select
            v-model="queryParams.customer_type"
            :placeholder="$t('crmCustomer.filter.customerTypePlaceholder')"
            clearable
          >
            <!-- value 取后端唯一词表 constants::customer_type::ALLOWED 的五个渠道 token；
                 分层词 normal/vip 不属本列（后端 400 拒绝）。 -->
            <el-option :label="$t('crmCustomer.customerType.retail')" value="retail" />
            <el-option :label="$t('crmCustomer.customerType.wholesale')" value="wholesale" />
            <el-option :label="$t('crmCustomer.customerType.distributor')" value="distributor" />
            <el-option :label="$t('crmCustomer.customerType.manufacturer')" value="manufacturer" />
            <el-option :label="$t('crmCustomer.customerType.other')" value="other" />
          </el-select>
        </el-form-item>
        <!-- 标准客户列表 /crm/customers 查询参数集不含 tag_id，后端 serde 对未知 query 键
             静默忽略，保留标签筛选控件即"筛了但没筛"的假筛选，故此处不设该控件。 -->
        <el-form-item :label="$t('crmCustomer.filter.status')">
          <el-select
            v-model="queryParams.status"
            :placeholder="$t('crmCustomer.filter.statusPlaceholder')"
            clearable
          >
            <!-- active/inactive/blacklist 三态取值与后端写入方状态词表逐字符同源 -->
            <el-option :label="$t('crmCustomer.status.active')" value="active" />
            <el-option :label="$t('crmCustomer.status.inactive')" value="inactive" />
            <el-option :label="$t('crmCustomer.status.blacklist')" value="blacklist" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">{{
            $t('crmCustomer.filter.query')
          }}</el-button>
          <el-button @click="handleReset">{{ $t('crmCustomer.filter.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table
        v-loading="loading"
        :data="customers"
        stripe
        :aria-label="$t('crmCustomer.table.ariaLabel')"
      >
        <el-table-column
          prop="customer_code"
          :label="$t('crmCustomer.table.customerCode')"
          width="120"
          fixed
        />
        <el-table-column
          prop="customer_name"
          :label="$t('crmCustomer.table.customerName')"
          min-width="180"
          fixed
        >
          <template #default="{ row }">
            <el-button type="primary" link @click="viewDetail(row.id)">{{
              row.customer_name
            }}</el-button>
          </template>
        </el-table-column>
        <el-table-column
          prop="contact_person"
          :label="$t('crmCustomer.table.contactPerson')"
          width="100"
        />
        <el-table-column prop="contact_phone" :label="$t('crmCustomer.table.phone')" width="130" />
        <el-table-column prop="customer_type" :label="$t('crmCustomer.table.type')" width="100">
          <template #default="{ row }">
            <el-tag :type="getCustomerTypeTag(row.customer_type)" size="small">
              {{ getCustomerTypeLabel(row.customer_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <!-- 标签列不绑行内 tags 键（客户域列表行无该键）；
             数据源 = GET /crm/customers/{id}/tags 逐行取数，
             ready+空数组 才显示"-"（真·无标签）；loading/failed 各自显式呈现，
             不把"取数中/取数失败"伪装成"没有标签"。 -->
        <el-table-column :label="$t('crmCustomer.table.tag')" min-width="150">
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
              $t('crmCustomer.table.tagsLoading')
            }}</span>
            <span v-else class="cell-load-failed">{{
              $t('crmCustomer.table.tagsLoadFailed')
            }}</span>
          </template>
        </el-table-column>
        <!-- 负责人列不绑行内 owner_name 键（客户域列表行无该键）；
             数据源 = row.owner_id + /crm/sales-users 联显 -->
        <el-table-column :label="$t('crmCustomer.table.owner')" width="100">
          <template #default="{ row }">{{ ownerDisplay(row.owner_id) }}</template>
        </el-table-column>
        <el-table-column prop="status" :label="$t('crmCustomer.table.status')" width="80">
          <template #default="{ row }">
            <el-tag :type="getStatusTag(row.status)" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('crmCustomer.table.operation')" width="180" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewDetail(row.id)">{{
              $t('crmCustomer.table.detail')
            }}</el-button>
            <!-- 编辑/删除按钮受权限键控制（customers:update / customers:delete） -->
            <el-button
              v-permission="PERMISSIONS.CUSTOMER_UPDATE"
              type="primary"
              link
              size="small"
              @click="handleEdit(row)"
              >{{ $t('crmCustomer.table.edit') }}</el-button
            >
            <el-button
              v-permission="PERMISSIONS.CUSTOMER_DELETE"
              type="danger"
              link
              size="small"
              @click="handleDelete(row)"
              >{{ $t('crmCustomer.table.delete') }}</el-button
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
          :aria-label="$t('crmCustomer.table.paginationAriaLabel')"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <!-- 新增/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogTitle"
      width="700px"
      :close-on-click-modal="false"
      :aria-label="$t('crmCustomer.dialog.ariaLabel')"
      @close="resetForm"
    >
      <el-form
        ref="formRef"
        :model="formData"
        :rules="formRules"
        label-width="100px"
        :aria-label="$t('crmCustomer.dialog.formAriaLabel')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.customerCode')" prop="customer_code">
              <!-- 客户编码由系统建档自动生成、编辑态建档后不可改：表单只读回显，不采集不发送 -->
              <el-input
                v-model="formData.customer_code"
                readonly
                :placeholder="$t('crmCustomer.dialog.customerCodePlaceholder')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.customerName')" prop="customer_name">
              <el-input
                v-model="formData.customer_name"
                :placeholder="$t('crmCustomer.dialog.customerNamePlaceholder')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.contactPerson')" prop="contact_person">
              <el-input
                v-model="formData.contact_person"
                :placeholder="$t('crmCustomer.dialog.contactPersonPlaceholder')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.phone')" prop="contact_phone">
              <el-input
                v-model="formData.contact_phone"
                :placeholder="$t('crmCustomer.dialog.phonePlaceholder')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.email')" prop="contact_email">
              <el-input
                v-model="formData.contact_email"
                :placeholder="$t('crmCustomer.dialog.emailPlaceholder')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.customerType')" prop="customer_type">
              <el-select
                v-model="formData.customer_type"
                :placeholder="$t('crmCustomer.dialog.customerTypePlaceholder')"
                style="width: 100%"
              >
                <!-- 与筛选下拉同源：五个渠道 token（constants::customer_type::ALLOWED）全覆盖，
                     少列 distributor/manufacturer 就建不出这两类客户。 -->
                <el-option :label="$t('crmCustomer.customerType.retail')" value="retail" />
                <el-option :label="$t('crmCustomer.customerType.wholesale')" value="wholesale" />
                <el-option
                  :label="$t('crmCustomer.customerType.distributor')"
                  value="distributor"
                />
                <el-option
                  :label="$t('crmCustomer.customerType.manufacturer')"
                  value="manufacturer"
                />
                <el-option :label="$t('crmCustomer.customerType.other')" value="other" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="$t('crmCustomer.dialog.address')" prop="address">
          <el-input
            v-model="formData.address"
            :placeholder="$t('crmCustomer.dialog.addressPlaceholder')"
          />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="12">
            <!-- prop/v-model 键名 = tax_id，逐字对齐后端 Create/UpdateCustomerRequest；
                 旧键 tax_number 是线索域 DTO 的键名，标准客户域 serde 无此键会被静默丢弃 -->
            <el-form-item :label="$t('crmCustomer.dialog.taxNumber')" prop="tax_id">
              <el-input
                v-model="formData.tax_id"
                :placeholder="$t('crmCustomer.dialog.taxNumberPlaceholder')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.creditLimit')" prop="credit_limit">
              <el-input-number
                v-model="formData.credit_limit"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.bankName')" prop="bank_name">
              <el-input
                v-model="formData.bank_name"
                :placeholder="$t('crmCustomer.dialog.bankNamePlaceholder')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.bankAccount')" prop="bank_account">
              <el-input
                v-model="formData.bank_account"
                :placeholder="$t('crmCustomer.dialog.bankAccountPlaceholder')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="$t('crmCustomer.dialog.status')" prop="status">
          <el-radio-group v-model="formData.status">
            <el-radio value="active">{{ $t('crmCustomer.status.active') }}</el-radio>
            <el-radio value="inactive">{{ $t('crmCustomer.status.inactive') }}</el-radio>
          </el-radio-group>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ $t('crmCustomer.dialog.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
          $t('crmCustomer.dialog.save')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { isDialogDismissal } from '@/utils/monitor';
import { PERMISSIONS } from '@/constants/permissions';
import { ref, reactive, computed, watch, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { Plus, Coin, Share, Download, Printer } from '@element-plus/icons-vue';
// 客户管理页读写走标准客户域端点（customer_handler，落/读 customers 表），写函数与行类型
// 从 '@/api/customer' 引入；与线索域（'@/api/crm-enhanced' 的 /crm/customers/enhanced =
// crm_lead）不同域，不可混用。
import {
  createCustomer,
  updateCustomer,
  deleteCustomer,
  getCustomerTags,
  type Customer,
  type CustomerTagBrief,
  type CustomerCreatePayload,
  type CustomerUpdatePayload,
} from '@/api/customer';
// 负责人联显数据源：GET /crm/sales-users（客户分配流程的候选人唯一出口，
// ManualAssignDialogTab/assignment.vue 消费同源）
import { getSalesUserList, type SalesUser } from '@/api/crm-enhanced';
import { useTableApi } from '@/composables/useTableApi';
// 导出走后端带水印 xlsx 接口 GET /crm/customers/export（后端注入水印 + 行级数据权限 +
// 异步审计日志），前端下载 Blob。
import { EXPORT_APPROVAL_REQUEST_PERMISSION_KEY } from '@/constants/export-approval';
import { useExportApproval } from '@/composables/useExportApproval';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { escapeHtml } from '@/utils/print';

const { t } = useI18n({ useScope: 'global' });

/**
 * 列表行真实形状：GET /crm/customers → customer_handler::list_customers
 * → services/customer_ops/query.rs::list_customers_with_filter 整行序列化
 * models/customer.rs::Model（admin 走无字段过滤路径）。
 * customer_code/customer_name/contact_person/contact_phone/customer_type/status
 * 为该实体真实键；owner_id 为 NOT NULL 列（0=公海/未分配），不标 `?`。
 * 行内**没有** tags / owner_name 键：标签经独立端点 getCustomerTags 逐行取数渲染，
 * 负责人经 /crm/sales-users 联显，均不绑行内不存在的键。
 */
type CustomerListRow = Customer & { owner_id: number };

const hasLoaded = createLazyLoader();

const router = useRouter();
const submitLoading = ref(false);
const dialogVisible = ref(false);
const isEdit = ref(false);
const formRef = ref<FormInstance>();

// —— 标签列数据源：GET /crm/customers/{id}/tags 逐行取数（客户域唯一标签端点，
// 无批量出口；行数上限=page_size≤100）。行内 tags 键不存在，不得绑定。
type TagsCellState =
  { status: 'loading' } | { status: 'failed' } | { status: 'ready'; tags: CustomerTagBrief[] };
const tagsStates = ref(new Map<number, TagsCellState>());
const tagsInFlight = new Set<number>();

// —— 负责人列数据源：/crm/sales-users 单次拉全量后本地联显。
// owner_id=0=公海/未分配；real_name 后端恒 null，展示名先例 = real_name || username
// （与 ManualAssignDialogTab 同源口径）。
const salesUsers = ref<SalesUser[]>([]);
const ownerLoadState = ref<'loading' | 'ready' | 'failed'>('loading');
const ownerNameById = computed(
  () => new Map(salesUsers.value.map(u => [u.id, u.real_name || u.username]))
);

// 筛选条件（仅筛选字段，分页由 useTableApi 管理）。
// 不含 tag_id：标准客户列表查询参数集 = page/page_size/status/customer_type/keyword
// （customer_handler::list_customers），后端对未知 query 键 serde 静默忽略，
// 保留 tag_id 只会做出"筛了但没筛"的假筛选 ⇒ 移除该控件与参数。
const queryParams = reactive({
  keyword: '',
  customer_type: '',
  status: '',
});

// 接入 useTableApi 统一消除手写 customers/total/loading/fetchCustomerList 重复：
// 自动管理分页状态与数据加载，watch page/pageSize 变化触发重载。
const {
  data: customers,
  loading,
  page,
  pageSize,
  total,
  refresh: fetchCustomerList,
  setQueryParam,
} = useTableApi<CustomerListRow>({
  url: '/crm/customers',
  // 后端返回 ApiResponse<PaginatedResponse>，承载列表的键为 items、总数键为 total
  // （utils/response.rs 的 PaginatedResponse 形状），显式钉住 listKey。
  listKey: 'items',
  onError: (err: unknown) =>
    ElMessage.error(
      (err instanceof Error ? err.message : String(err)) || t('crmCustomer.message.loadFailed')
    ),
});

// 同步筛选条件到 useTableApi.queryParams 并刷新
const syncQueryParams = () => {
  setQueryParam('keyword', queryParams.keyword || undefined);
  setQueryParam('customer_type', queryParams.customer_type || undefined);
  setQueryParam('status', queryParams.status || undefined);
};

// 缺省渠道与后端 `constants::customer_type::validate(None)` 同源（详见 formData 处注释）。
// 显式标注 string：const 字面量的类型是 `"other"`，直接喂给 reactive 会让 customer_type
// 收窄成该单一 token，下拉改选其它渠道时模板赋值过不了 vue-tsc。
const DEFAULT_CUSTOMER_TYPE: string = 'other';

const formData = reactive({
  id: undefined as number | undefined,
  customer_code: '',
  customer_name: '',
  contact_person: '',
  contact_phone: '',
  contact_email: '',
  address: '',
  // 缺省与后端 `constants::customer_type::validate(None)` 同源写 `other`（渠道未知），
  // 不由前端替业务方断言"零售/普通"；后端该字段为 Option 无必填门，此处不额外收紧校验。
  customer_type: DEFAULT_CUSTOMER_TYPE,
  // 键名逐字对齐后端 DTO（CreateCustomerRequest/UpdateCustomerRequest 的税号键 = tax_id，
  // models/customer.rs 列同为 tax_id）。旧表单绑 tax_number 是线索域 EnhancedCustomerUpdateInput
  // 的键名，标准客户域 serde 无此键 ⇒ 静默丢字段。
  tax_id: '',
  // el-input-number 采集为 number；提交时必须转 string（后端 credit_limit=Option<String>，
  // JSON number 反序列化即 422）。
  credit_limit: 0,
  bank_name: '',
  bank_account: '',
  status: 'active',
});

/**
 * 编辑基线快照（handleEdit 时按行**展示值**归一存入，string 形态）：
 * 更新提交只下发改过的键（后端 UpdateCustomerRequest 缺键=该列不改）。
 * 这不是可选优化而是数据完整性硬要求：非 admin 会话读回的 contact_phone/contact_email
 * 是打码值、address 整键不下发，把掩码值原样提交会把 `138****8888` 写进库、
 * 把"看不见但库里非空"的地址清空——地址簿对同一禁令亦同（见 api/customer.ts 地址更新注释）。
 */
const editOriginal = ref<Record<string, string>>({});

const formRules = computed<FormRules>(() => ({
  customer_name: [
    { required: true, message: t('crmCustomer.validation.customerNameRequired'), trigger: 'blur' },
  ],
  contact_person: [
    { required: true, message: t('crmCustomer.validation.contactPersonRequired'), trigger: 'blur' },
  ],
  contact_phone: [
    { required: true, message: t('crmCustomer.validation.phoneRequired'), trigger: 'blur' },
    {
      pattern: /^1[3-9]\d{9}$/,
      message: t('crmCustomer.validation.phonePattern'),
      trigger: 'blur',
    },
  ],
}));

const dialogTitle = computed(() =>
  isEdit.value ? t('crmCustomer.dialog.editTitle') : t('crmCustomer.dialog.createTitle')
);

// getCustomerTypeLabel 用函数返回，使 t() 在每次渲染时响应式求值。
// 回读值是后端渠道 token（constants::customer_type::ALLOWED 五值），映射表按同一词表逐值取文案；
// 未知/历史脏值不吞不造假名，回落显示原始 token。
// 入参非可选：列表行里 customer_type 是必填 string（后端 NOT NULL 列），
// 前端不得用可选/兜底参数掩盖缺键。
const getCustomerTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    retail: t('crmCustomer.customerType.retail'),
    wholesale: t('crmCustomer.customerType.wholesale'),
    distributor: t('crmCustomer.customerType.distributor'),
    manufacturer: t('crmCustomer.customerType.manufacturer'),
    other: t('crmCustomer.customerType.other'),
  };
  return labels[type] || type;
};

// tag 颜色仅供列表视觉区分，不承载状态/层级语义（同 token 在不同页面配色不同不影响契约）。
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

// 状态词表按**写入方**全量覆盖（backend models/customer.rs = active/inactive/blacklist 三态）。
// 比较点必须与写入值逐字符对齐，否则 blacklist 行会被误显为"禁用"；
// 未知/历史脏值不吞不造假名，回落显示原始 token。
const getStatusLabel = (status: string) => {
  const labels: Record<string, string> = {
    active: t('crmCustomer.status.active'),
    inactive: t('crmCustomer.status.inactive'),
    blacklist: t('crmCustomer.status.blacklist'),
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

// 标签列取数：客户域唯一标签端点为单客户粒度 GET /crm/customers/{id}/tags，
// 无批量出口 ⇒ 对当前页行逐 id 拉取（行数上限=page_size≤100）。
// 失败不静默：该行标记 failed 并在单元格显式呈现"标签加载失败"（不把取数失败
// 伪装成"该客户没有标签"），同时 logAuxLoadFailure 留痕。
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
      logAuxLoadFailure(t('crmCustomer.table.tagsLoadFailed'), error);
    })
    .finally(() => {
      tagsInFlight.delete(id);
    });
};

// 列表数据每次到达（含翻页/筛选重载）为新出现的客户 id 触发标签取数；已取过的走缓存。
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

// 负责人列取数：sales-users 单次全量（客户分配流程候选人同源出口）。
// 失败不静默：ownerLoadState=failed，单元格显式呈现"负责人加载失败"。
const fetchOwners = async () => {
  try {
    const res = await getSalesUserList();
    salesUsers.value = res.data;
    ownerLoadState.value = 'ready';
  } catch (error) {
    ownerLoadState.value = 'failed';
    logAuxLoadFailure(t('crmCustomer.message.loadOwnersFailed'), error);
  }
};

// owner_id 语义：0=公海/未分配（models/customer.rs）；
// 非 0 但不在销售用户集合内 = 数据漂移（如越权指派/历史脏值），显示 `#<id>`
// 原样暴露不可解析的 id——不臆造姓名、不吞成空，让漂移可见。
const ownerDisplay = (ownerId: number): string => {
  if (ownerId === 0) return t('crmCustomer.table.ownerUnassigned');
  if (ownerLoadState.value === 'loading') return t('crmCustomer.table.ownerLoading');
  if (ownerLoadState.value === 'failed') return t('crmCustomer.table.ownerLoadFailed');
  return ownerNameById.value.get(ownerId) ?? `#${ownerId}`;
};

// 查询 - 同步筛选条件并回到首页重载
const handleQuery = () => {
  syncQueryParams();
  page.value = 1;
  fetchCustomerList();
};

// 重置 - 清空筛选并回到首页重载
const handleReset = () => {
  queryParams.keyword = '';
  queryParams.customer_type = '';
  queryParams.status = '';
  syncQueryParams();
  page.value = 1;
  fetchCustomerList();
};

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p;
};

const handleSizeChange = (s: number) => {
  pageSize.value = s;
  page.value = 1;
};

const resetForm = () => {
  formData.id = undefined;
  formData.customer_code = '';
  formData.customer_name = '';
  formData.contact_person = '';
  formData.contact_phone = '';
  formData.contact_email = '';
  formData.address = '';
  formData.customer_type = DEFAULT_CUSTOMER_TYPE;
  formData.tax_id = '';
  formData.credit_limit = 0;
  formData.bank_name = '';
  formData.bank_account = '';
  formData.status = 'active';
  editOriginal.value = {};
  formRef.value?.clearValidate();
};

const handleCreate = () => {
  resetForm();
  isEdit.value = false;
  dialogVisible.value = true;
};

// 编辑回填逐键显式赋值：Object.assign 整行会把 tier/owner_id/department_id/created_at
// 等实体键灌进表单，credit_limit 的 Decimal 字符串也会污染 el-input-number 的 number 绑定。
// Option 列 null → '' 仅作表单编辑态归一（可编辑文本框的"空"），不参与提交判据；
// 提交按 editOriginal diff（见其定义处注释）。
const handleEdit = (row: CustomerListRow) => {
  resetForm();
  formData.id = row.id;
  formData.customer_code = row.customer_code;
  formData.customer_name = row.customer_name;
  formData.contact_person = row.contact_person ?? '';
  formData.contact_phone = row.contact_phone ?? '';
  formData.contact_email = row.contact_email ?? '';
  formData.address = row.address ?? '';
  formData.customer_type = row.customer_type;
  formData.tax_id = row.tax_id ?? '';
  formData.credit_limit = Number(row.credit_limit);
  formData.bank_name = row.bank_name ?? '';
  formData.bank_account = row.bank_account ?? '';
  formData.status = row.status;
  editOriginal.value = {
    customer_name: row.customer_name,
    contact_person: row.contact_person ?? '',
    contact_phone: row.contact_phone ?? '',
    contact_email: row.contact_email ?? '',
    address: row.address ?? '',
    customer_type: row.customer_type,
    tax_id: row.tax_id ?? '',
    credit_limit: row.credit_limit,
    bank_name: row.bank_name ?? '',
    bank_account: row.bank_account ?? '',
    status: row.status,
  };
  isEdit.value = true;
  dialogVisible.value = true;
};

const handleDelete = async (row: CustomerListRow) => {
  try {
    await ElMessageBox.confirm(
      t('crmCustomer.message.deleteConfirm', { name: row.customer_name }),
      t('crmCustomer.message.deleteConfirmTitle'),
      {
        type: 'warning',
      }
    );
    await deleteCustomer(row.id);
    ElMessage.success(t('crmCustomer.message.deleteSuccess'));
    fetchCustomerList();
  } catch (error) {
    if (!isDialogDismissal(error)) {
      const err = error as Error;
      ElMessage.error(err.message || t('crmCustomer.message.deleteFailed'));
    }
  }
};

// 提交载荷构造逐键对齐后端 CreateCustomerRequest/UpdateCustomerRequest：
// - 不发送 customer_code（创建 DTO 该键由服务端建档生成，编辑 DTO 无此键，多发被 serde 丢弃）；
// - 空串可选键转 undefined（JSON.stringify 剔除）：contact_email 带 #[validate(email)]，
//   空串会以"邮箱格式不正确"422，留空=不填=不发键，语义一致；
// - credit_limit 转 string（后端 Option<String>，number 反序列化即 422）。
const optionalStr = (v: string): string | undefined => (v === '' ? undefined : v);

const buildCreatePayload = (): CustomerCreatePayload => ({
  customer_name: formData.customer_name,
  contact_person: optionalStr(formData.contact_person),
  contact_phone: optionalStr(formData.contact_phone),
  contact_email: optionalStr(formData.contact_email),
  address: optionalStr(formData.address),
  customer_type: formData.customer_type,
  tax_id: optionalStr(formData.tax_id),
  credit_limit: String(formData.credit_limit),
  bank_name: optionalStr(formData.bank_name),
  bank_account: optionalStr(formData.bank_account),
  status: formData.status,
});

// 更新只下发改过的键（缺键=该列不改；防掩码回写）。
const UPDATE_STR_FIELDS = [
  'customer_name',
  'contact_person',
  'contact_phone',
  'contact_email',
  'address',
  'customer_type',
  'tax_id',
  'bank_name',
  'bank_account',
  'status',
] as const;

const buildUpdatePayload = (): CustomerUpdatePayload => {
  const payload: CustomerUpdatePayload = {};
  for (const key of UPDATE_STR_FIELDS) {
    if (formData[key] !== (editOriginal.value[key] ?? '')) {
      payload[key] = formData[key];
    }
  }
  // credit_limit 按数值比较（行值 "0.00" 与表单 0 同值，不算改动），改动才转 string 下发
  if (Number(editOriginal.value.credit_limit ?? '0') !== formData.credit_limit) {
    payload.credit_limit = String(formData.credit_limit);
  }
  return payload;
};

const handleSubmit = async () => {
  if (!formRef.value) return;

  await formRef.value.validate(async valid => {
    if (!valid) return;

    submitLoading.value = true;
    try {
      if (isEdit.value) {
        await updateCustomer(formData.id as number, buildUpdatePayload());
        ElMessage.success(t('crmCustomer.message.updateSuccess'));
      } else {
        await createCustomer(buildCreatePayload());
        ElMessage.success(t('crmCustomer.message.createSuccess'));
      }
      dialogVisible.value = false;
      fetchCustomerList();
    } catch (error) {
      const err = error as Error;
      ElMessage.error(err.message || t('crmCustomer.message.operationFailed'));
    } finally {
      submitLoading.value = false;
    }
  });
};

const viewDetail = (id: number) => {
  router.push(`/crm/detail/${id}`);
};

/**
 * 导出客户清单：敏感资源，后端 GET /crm/customers/export 以 download_token 做
 * fail-closed 审批令牌校验，直连必 403。改为提交导出审批申请（useExportApproval
 * 统一入口，参数形状由 constants/export-approval 注册表收敛为 status/customer_type/
 * keyword），estimated_rows 传当前筛选命中总数，决定后端风险等级与审批层级。批准后
 * 由「下载我的导出」按钮在本页就地下载（downloadApprovedExport 拉取本人该资源下
 * 可用令牌申请，不再集中到审批页下载）。
 */
const { exportSubmitting, requestExport, approvedDownloading, downloadApprovedExport } =
  useExportApproval();
const handleExport = () =>
  requestExport(
    'customer',
    {
      status: queryParams.status,
      customer_type: queryParams.customer_type,
      keyword: queryParams.keyword.trim(),
    },
    total.value
  );
const handleDownloadMine = () => downloadApprovedExport('customer');

const handlePrint = () => {
  const printWindow = window.open('', '_blank');
  if (!printWindow) {
    ElMessage.error(t('crmCustomer.message.printWindowFailed'));
    return;
  }
  const rows = customers.value
    .map(
      item => `
    <tr>
      <td>${escapeHtml(item.customer_code)}</td>
      <td>${escapeHtml(item.customer_name)}</td>
      <td>${escapeHtml(item.contact_person ?? '')}</td>
      <td>${escapeHtml(item.contact_phone ?? '')}</td>
      <td>${escapeHtml(getCustomerTypeLabel(item.customer_type))}</td>
      <td>${escapeHtml(ownerDisplay(item.owner_id))}</td>
      <td>${escapeHtml(getStatusLabel(item.status))}</td>
    </tr>
  `
    )
    .join('');
  const now = new Date().toISOString().split('T')[0];
  printWindow.document.write(`
    <html><head><meta charset="utf-8"><title>${t('crmCustomer.printDialog.title')}</title>
    <style>
      @media print { @page { size: landscape; } }
      body { font-family: "Microsoft YaHei", sans-serif; font-size: 12px; }
      h1 { text-align: center; }
      table { width: 100%; border-collapse: collapse; margin-top: 12px; }
      th, td { border: 1px solid #333; padding: 6px 8px; }
      th { background: #f5f5f5; }
      .meta { text-align: center; color: #666; font-size: 11px; }
    </style></head><body>
    <h1>${t('crmCustomer.printDialog.title')}</h1>
    <div class="meta">${t('crmCustomer.printDialog.date')}: ${now} | ${t('crmCustomer.printDialog.total', { count: customers.value.length })}</div>
    <table>
      <thead><tr><th>${t('crmCustomer.table.customerCode')}</th><th>${t('crmCustomer.table.customerName')}</th><th>${t('crmCustomer.table.contactPerson')}</th><th>${t('crmCustomer.table.phone')}</th><th>${t('crmCustomer.table.type')}</th><th>${t('crmCustomer.table.owner')}</th><th>${t('crmCustomer.table.status')}</th></tr></thead>
      <tbody>${rows}</tbody>
    </table>
    </body></html>
  `);
  printWindow.document.close();
  printWindow.onload = () => printWindow.print();
  logger.info(t('crmCustomer.printDialog.logMessage'));
};

onMounted(() => {
  // 负责人联显数据源单次加载（标签列由 watch(customers) 按当前页逐行取数）
  loadIfNot('fetchOwners', fetchOwners, hasLoaded);
  // 客户列表由 useTableApi 在 setup 阶段自动加载，无需在此手动调用
});
</script>
