<!--
  售后工单面板
  - 4 种类型：客诉/维修/换货/退款
  - 创建工单 + 更新状态
-->
<template>
  <div class="after-sales-panel">
    <div class="action-bar">
      <el-button type="primary" @click="showCreateDialog">
        <el-icon><Plus /></el-icon>
        {{ t('common.afterSales.createTicket') }}
      </el-button>
    </div>

    <el-table
      :data="afterSales"
      border
      stripe
      :empty-text="t('common.afterSales.emptyText')"
      :aria-label="t('common.afterSales.tableAriaLabel')"
    >
      <el-table-column :label="t('common.afterSales.colType')" width="100">
        <template #default="{ row }">
          <el-tag>{{ getIssueTypeLabel(row.issue_type) }}</el-tag>
        </template>
      </el-table-column>
      <!-- 客户名由后端读侧 LEFT JOIN customers 富化出参（customer_name，恒含键）；
           客户行缺失时为 null，显示 '-'（与本文件金额/时间列空值口径一致），
           禁止回退显示 customer_id 数字冒充名称 -->
      <el-table-column
        :label="t('common.afterSales.formCustomer')"
        min-width="120"
        show-overflow-tooltip
      >
        <template #default="{ row }">
          <span v-if="row.customer_name != null">{{ row.customer_name }}</span>
          <span v-else>-</span>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.afterSales.formReasonCategory')" width="110">
        <template #default="{ row }">
          {{ getReasonCategoryLabel(row.reason_category) }}
        </template>
      </el-table-column>
      <el-table-column
        prop="reason_detail"
        :label="t('common.afterSales.formReasonDetail')"
        min-width="140"
        show-overflow-tooltip
      />
      <el-table-column
        prop="description"
        :label="t('common.afterSales.colDescription')"
        min-width="200"
        show-overflow-tooltip
      />
      <el-table-column :label="t('common.afterSales.colStatus')" width="120" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">
            {{ getStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.afterSales.colRefundAmount')" width="140" align="right">
        <template #default="{ row }">
          <!-- 缺陷 D：refund_amount 后端为 rust_decimal，JSON 出参是字符串
               （如 "1200.50"），展示必须经 Number() 归一的 formatCurrency，禁止裸显/直接 toFixed -->
          <span v-if="row.refund_amount != null">{{ formatCurrency(row.refund_amount) }}</span>
          <span v-else>-</span>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.afterSales.colOpenedAt')" width="170">
        <template #default="{ row }">
          {{ formatDate(row.opened_at) }}
        </template>
      </el-table-column>
      <el-table-column :label="t('common.afterSales.colClosedAt')" width="170">
        <template #default="{ row }">
          {{ formatDate(row.closed_at) }}
        </template>
      </el-table-column>
      <el-table-column
        prop="resolution"
        :label="t('common.afterSales.colResolution')"
        min-width="180"
        show-overflow-tooltip
      />
      <el-table-column :label="t('common.afterSales.colOperation')" width="220" fixed="right">
        <!-- 缺陷 B：动作严格对齐后端 is_valid_transition
             （custom_order_aftersales_service.rs:406-418）：
             opened→accepted/rejected/closed；accepted→processing/rejected/closed；
             processing→resolved/closed/rejected；resolved→evaluated/closed；evaluated→closed -->
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'opened'"
            size="small"
            type="primary"
            link
            @click="handleUpdate(row, 'accepted')"
          >
            {{ t('common.afterSales.accept') }}
          </el-button>
          <el-button
            v-if="row.status === 'accepted'"
            size="small"
            type="primary"
            link
            @click="handleUpdate(row, 'processing')"
          >
            {{ t('common.afterSales.process') }}
          </el-button>
          <el-button
            v-if="row.status === 'processing'"
            size="small"
            type="success"
            link
            @click="showResolveDialog(row)"
          >
            {{ t('common.afterSales.resolve') }}
          </el-button>
          <el-button
            v-if="row.status === 'resolved'"
            size="small"
            type="success"
            link
            @click="handleUpdate(row, 'evaluated')"
          >
            {{ t('common.afterSales.evaluate') }}
          </el-button>
          <el-button
            v-if="['opened', 'accepted', 'processing'].includes(row.status)"
            size="small"
            type="danger"
            link
            @click="handleUpdate(row, 'rejected')"
          >
            {{ t('common.afterSales.reject') }}
          </el-button>
          <el-button
            v-if="
              ['opened', 'accepted', 'processing', 'resolved', 'evaluated'].includes(row.status)
            "
            size="small"
            link
            @click="handleUpdate(row, 'closed')"
          >
            {{ t('common.afterSales.close') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 创建工单 -->
    <el-dialog
      v-model="createVisible"
      :title="t('common.afterSales.createDialogTitle')"
      :aria-label="t('common.afterSales.createDialogAriaLabel')"
      width="540px"
    >
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="100px"
        :aria-label="t('common.afterSales.createFormAriaLabel')"
      >
        <el-form-item :label="t('common.afterSales.formIssueType')" prop="issue_type">
          <el-radio-group v-model="form.issue_type">
            <el-radio-button label="complaint">{{
              t('common.afterSales.issueType.complaint')
            }}</el-radio-button>
            <el-radio-button label="repair">{{
              t('common.afterSales.issueType.repair')
            }}</el-radio-button>
            <el-radio-button label="exchange">{{
              t('common.afterSales.issueType.exchange')
            }}</el-radio-button>
            <el-radio-button label="return_goods">{{
              t('common.afterSales.issueType.return_goods')
            }}</el-radio-button>
            <el-radio-button label="refund">{{
              t('common.afterSales.issueType.refund')
            }}</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item :label="t('common.afterSales.formCustomer')" prop="customer_id">
          <el-input-number v-model="form.customer_id" :min="1" />
        </el-form-item>
        <el-form-item
          v-if="form.issue_type === 'refund'"
          :label="t('common.afterSales.formRefundAmount')"
          prop="refund_amount"
        >
          <el-input-number v-model="form.refund_amount" :min="0" :precision="2" :step="100" />
        </el-form-item>
        <el-form-item :label="t('common.afterSales.formDescription')" prop="description">
          <el-input v-model="form.description" type="textarea" :rows="3" />
        </el-form-item>
        <!-- 缺陷 C：后端 CreateAfterSalesDto 的 reason_category / reason_detail /
             quality_issue_id（custom_order_aftersales_service.rs:39-45）此前完全不采集。
             分类取值以后端权威注释为准（migration v15/mod.rs:1891 列 COMMENT）：
             quality / logistics / customer_preference / other，不臆造枚举。 -->
        <el-form-item :label="t('common.afterSales.formReasonCategory')" prop="reason_category">
          <el-select v-model="form.reason_category" clearable style="width: 100%">
            <el-option
              v-for="key in REASON_CATEGORY_KEYS"
              :key="key"
              :label="t(`common.afterSales.reasonCategory.${key}`)"
              :value="key"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('common.afterSales.formReasonDetail')" prop="reason_detail">
          <el-input v-model="form.reason_detail" type="textarea" :rows="2" />
        </el-form-item>
        <!-- 关联质量异常取自订单详情已有 quality_issues（后端 quality_issue_id 外键）；
             无候选时不渲染，不放假输入框 -->
        <el-form-item
          v-if="(qualityIssues || []).length > 0"
          :label="t('common.afterSales.formQualityIssue')"
          prop="quality_issue_id"
        >
          <el-select v-model="form.quality_issue_id" clearable style="width: 100%">
            <el-option
              v-for="qi in qualityIssues || []"
              :key="qi.id"
              :label="`#${qi.id} ${qi.description || ''}`"
              :value="qi.id"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">{{ t('common.afterSales.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleCreateSubmit">{{
          t('common.afterSales.submit')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 解决工单 -->
    <el-dialog
      v-model="resolveVisible"
      :title="t('common.afterSales.resolveDialogTitle')"
      :aria-label="t('common.afterSales.resolveDialogAriaLabel')"
      width="500px"
    >
      <el-form
        :model="resolveForm"
        label-width="80px"
        :aria-label="t('common.afterSales.resolveFormAriaLabel')"
      >
        <el-form-item :label="t('common.afterSales.formResolution')" required>
          <el-input v-model="resolveForm.resolution" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="resolveVisible = false">{{ t('common.afterSales.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleResolveSubmit">{{
          t('common.afterSales.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { ElMessage } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import { useI18n } from 'vue-i18n';
import {
  createAfterSales,
  updateAfterSales,
  type AfterSales,
  type AfterSalesCreateDto,
  type QualityIssue,
} from '@/api/custom-order';
import { formatCurrency } from '@/utils';

const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  orderId: number;
  afterSales: AfterSales[];
  /** 订单关联质量异常（详情页 quality_issues），供创建工单时可选关联（缺陷 C） */
  qualityIssues?: QualityIssue[];
}>();

const emit = defineEmits<{ (e: 'refresh'): void }>();

const createVisible = ref(false);
const resolveVisible = ref(false);
const submitting = ref(false);
const formRef = ref();
const currentRecord = ref<AfterSales | null>(null);

/** 原因分类取值 = 后端权威词表（列 COMMENT 见 backend/migration/src/domain/v15/mod.rs:1891）：
 * quality/logistics/customer_preference/other */
const REASON_CATEGORY_KEYS = ['quality', 'logistics', 'customer_preference', 'other'] as const;

const form = ref({
  issue_type: 'complaint',
  customer_id: undefined as number | undefined,
  description: '',
  refund_amount: undefined as number | undefined,
  reason_category: '',
  reason_detail: '',
  quality_issue_id: undefined as number | undefined,
});

const resolveForm = ref({ resolution: '' });

const rules = computed(() => ({
  issue_type: [
    { required: true, message: t('common.afterSales.rule.issueTypeRequired'), trigger: 'change' },
  ],
  customer_id: [
    { required: true, message: t('common.afterSales.rule.customerRequired'), trigger: 'blur' },
  ],
  description: [
    { required: true, message: t('common.afterSales.rule.descriptionRequired'), trigger: 'blur' },
  ],
  refund_amount: [
    {
      // validator 参数显式标注类型（unknown + 回调签名）
      validator: (_rule: unknown, val: unknown, cb: (error?: Error) => void) => {
        if (form.value.issue_type === 'refund' && (val === undefined || val === null)) {
          cb(new Error(t('common.afterSales.rule.refundAmountRequired')));
        } else {
          cb();
        }
      },
      trigger: 'blur',
    },
  ],
}));

/** 售后类型标签映射（响应式 t() 求值，替代导入的 AFTER_SALES_TYPE 中文常量） */
function getIssueTypeLabel(s: string): string {
  const known = ['complaint', 'repair', 'exchange', 'return_goods', 'refund'];
  if (!known.includes(s)) return s;
  return t(`common.afterSales.issueType.${s}`);
}

/** 原因分类标签映射：取值即 REASON_CATEGORY_KEYS 权威词表（与创建表单同源），
 * 经 common.afterSales.reasonCategory.* i18n 显示中文；空值显 '-'，未知 token 原样回显 */
function getReasonCategoryLabel(s?: string): string {
  if (!s) return '-';
  if (!(REASON_CATEGORY_KEYS as readonly string[]).includes(s)) return s;
  return t(`common.afterSales.reasonCategory.${s}`);
}

/** 状态标签映射（响应式 t() 求值，替代导入的 AFTER_SALES_STATUS 中文常量）
 * 词表对齐后端写入方全集（缺陷 B）：opened/accepted/processing/resolved/evaluated/closed/rejected */
function getStatusLabel(s: string): string {
  const known = ['opened', 'accepted', 'processing', 'resolved', 'evaluated', 'closed', 'rejected'];
  if (!known.includes(s)) return s;
  return t(`common.afterSales.status.${s}`);
}

// 状态标签类型收敛为 el-tag 合法取值的联合字面量
type TagType = 'success' | 'warning' | 'info' | 'primary' | 'danger';

function getStatusType(s: string): TagType {
  const map: Record<string, TagType> = {
    opened: 'warning',
    accepted: 'primary',
    processing: 'primary',
    resolved: 'success',
    evaluated: 'success',
    closed: 'info',
    rejected: 'danger',
  };
  return map[s] || 'info';
}

function formatDate(d: string | undefined) {
  if (!d) return '-';
  return new Date(d).toLocaleString('zh-CN');
}

function showCreateDialog() {
  form.value = {
    issue_type: 'complaint',
    customer_id: undefined,
    description: '',
    refund_amount: undefined,
    reason_category: '',
    reason_detail: '',
    quality_issue_id: undefined,
  };
  createVisible.value = true;
}

/** payload 逐字段对照后端 CreateAfterSalesDto 构造：
 * - 不发送 custom_order_id（归属由路由 path 权威提供，后端 DTO 已无该字段）；
 * - 后端 Option 字符串字段空值必须**省略该键**（Some("") 过不了 validator，
 *   范式见 views/system/tabs/UserTab.vue:358-359），不发送空串占位。 */
function buildCreatePayload(): AfterSalesCreateDto {
  return {
    customer_id: form.value.customer_id as number,
    issue_type: form.value.issue_type,
    description: form.value.description,
    ...(form.value.issue_type === 'refund' && form.value.refund_amount != null
      ? { refund_amount: form.value.refund_amount }
      : {}),
    ...(form.value.quality_issue_id ? { quality_issue_id: form.value.quality_issue_id } : {}),
    ...(form.value.reason_category ? { reason_category: form.value.reason_category } : {}),
    ...(form.value.reason_detail.trim() ? { reason_detail: form.value.reason_detail.trim() } : {}),
  };
}

async function handleCreateSubmit() {
  if (!formRef.value) return;
  try {
    await formRef.value.validate();
  } catch {
    return;
  }
  submitting.value = true;
  try {
    await createAfterSales(props.orderId, buildCreatePayload());
    ElMessage.success(t('common.afterSales.createSuccess'));
    createVisible.value = false;
    emit('refresh');
  } catch (e: unknown) {
    // 错误按 unknown 处理，经类型守卫提取可外显 message
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('common.afterSales.createFailed')
    );
  } finally {
    submitting.value = false;
  }
}

async function handleUpdate(row: AfterSales, status: string) {
  try {
    await updateAfterSales(row.id, { status, resolution: row.resolution });
    ElMessage.success(t('common.afterSales.statusUpdated'));
    emit('refresh');
  } catch (e: unknown) {
    // 错误按 unknown 处理，经类型守卫提取可外显 message
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('common.afterSales.updateFailed')
    );
  }
}

function showResolveDialog(row: AfterSales) {
  currentRecord.value = row;
  resolveForm.value = { resolution: '' };
  resolveVisible.value = true;
}

async function handleResolveSubmit() {
  if (!resolveForm.value.resolution) {
    ElMessage.warning(t('common.afterSales.pleaseInputResolution'));
    return;
  }
  submitting.value = true;
  try {
    // 未选中工单时 currentRecord 为 null，先取 id 做非空守卫
    const recordId = currentRecord.value?.id;
    if (!recordId) {
      ElMessage.warning(t('common.afterSales.pleaseSelectTicket'));
      return;
    }
    await updateAfterSales(recordId, {
      status: 'resolved',
      resolution: resolveForm.value.resolution,
    });
    ElMessage.success(t('common.afterSales.resolveSuccess'));
    resolveVisible.value = false;
    emit('refresh');
  } catch (e: unknown) {
    // 错误按 unknown 处理，经类型守卫提取可外显 message
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('common.afterSales.resolveFailed')
    );
  } finally {
    submitting.value = false;
  }
}
</script>

<style scoped>
.after-sales-panel {
  padding: 8px 0;
}
.action-bar {
  margin-bottom: 12px;
}
</style>
