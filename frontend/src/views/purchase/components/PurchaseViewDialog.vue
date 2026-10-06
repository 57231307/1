<script setup lang="ts">
/**
 * ViewDlg - 采购单详情对话框
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 查看对话框）
 *
 * 明细编辑（任务 #145）：DRAFT 订单可对每行编辑「色号 / 辅助数量 / 折扣率 /
 * 交货允差」四个写侧字段，经 PUT /purchase/orders/{id}/items/{item_id}
 * （后端 UpdateOrderItemRequest）落库并回显。
 * - 读键用后端出参真实 snake_case 键（color_code / quantity_alt / discount_percent /
 *   quantity_tolerance_pct）；写键按请求契约（color_no / quantity_alt_ordered /
 *   discount_percent / quantity_tolerance_pct）——读写键名不同属既有口径。
 * - discount_percent/quantity_alt/quantity_tolerance_pct 为 rust_decimal 出参，
 *   JSON 是**字符串**（如 "5.0000"），展示与编辑绑定必须先 Number() 归一，
 *   禁止对字符串直接 .toFixed/参与运算（本仓已多次因此运行期崩溃）。
 * - 「缺省即不改」：允差未填（undefined）不提交该键，保持库中原值；
 *   行级允差/色号「显式清回 NULL」当前写侧契约（Option=None 即不改）尚不可表达，
 *   留空输入框只会触发校验提示而不提交清空，等待契约三态扩展（勿静默吞掉）。
 * - supplier_product_code/supplier_color_no 为保密快照列，由后端按 color_no
 *   走与创建路径同一套映射反查逻辑派生，前端不提交、只回显。
 */
import { computed, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
import { PURCHASE_ORDER_STATUS } from '@/utils/purchase-status';
import {
  updatePurchaseOrderItem,
  type PurchaseOrder,
  type PurchaseOrderItem,
  type UpdatePurchaseOrderItemPayload,
} from '@/api/purchase';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  data: PurchaseOrder | null;
  getStatusType: (s: string) => string;
  getStatusText: (s: string) => string;
  getPaymentStatusType: (s: string) => string;
  getPaymentStatusText: (s: string) => string;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void;
  (e: 'saved'): void;
}>();

/** 单行明细编辑草稿（仅写侧四字段，键名 = UpdatePurchaseOrderItemPayload）。
 * el-input-number 清空时回写 null，与未填 undefined 一并视为「本次不提交该键」 */
interface ItemDraft {
  color_no: string;
  quantity_alt_ordered: number | null | undefined;
  discount_percent: number | null | undefined;
  quantity_tolerance_pct: number | null | undefined;
  saving: boolean;
}

/** 仅 DRAFT 允许编辑（与后端 update_order_item 状态门控同词表：models/status 原值） */
const editable = computed(() => props.data?.status === PURCHASE_ORDER_STATUS.DRAFT);

const drafts = reactive<Record<number, ItemDraft>>({});

/** Decimal 出参归一：null/undefined → undefined；字符串数字 → number（NaN 如实抛出而非吞掉） */
function toNumber(raw: string | number | null | undefined): number | undefined {
  if (raw === null || raw === undefined || raw === '') return undefined;
  const n = Number(raw);
  if (Number.isNaN(n)) {
    throw new Error(`[PurchaseViewDialog] 非预期的 Decimal 出参形态: ${JSON.stringify(raw)}`);
  }
  return n;
}

/** 从明细行初始化草稿（后端 Model/DTO 真实 snake_case 键） */
function seedDrafts(data: PurchaseOrder | null) {
  for (const key of Object.keys(drafts)) delete drafts[Number(key)];
  if (!data?.items) return;
  for (const item of data.items as PurchaseOrderItem[]) {
    drafts[item.id] = {
      color_no: item.color_code ?? '',
      quantity_alt_ordered: toNumber(item.quantity_alt),
      discount_percent: toNumber(item.discount_percent),
      quantity_tolerance_pct: toNumber(item.quantity_tolerance_pct),
      saving: false,
    };
  }
}

watch(() => props.data, seedDrafts, { immediate: true });

/** 保存单行明细：只提交四个写侧字段；undefined 键经 JSON 序列化自然省略（缺省即不改） */
async function saveItem(item: PurchaseOrderItem) {
  const order = props.data;
  const draft = drafts[item.id];
  if (!order || !draft) return;
  // 写侧契约（Option=None 即不改）暂不能表达「显式清回 NULL」：用户把原本有值的
  // 色号/允差清空（输入框清空回写 null，与未填 undefined 同类）时，该键省略提交
  // （保持原值）并如实提示原因，但**不阻断**其余字段的保存——与占位文案
  // 「色号，留空不改」一致（留空 ≠ 不可保存）。
  const clearedToUnexpressibleNull =
    (draft.color_no === '' && (item.color_code ?? '') !== '') ||
    (draft.quantity_tolerance_pct == null &&
      item.quantity_tolerance_pct !== null &&
      item.quantity_tolerance_pct !== undefined);
  if (clearedToUnexpressibleNull) {
    msg.warning('purchaseItemFieldKeepHint');
  }
  // null/undefined 统一归一为 undefined：JSON.stringify 省略 undefined 键，
  // 而后端对显式 null 同样解码为 None——两条路都是「缺省即不改」，不产生歧义
  const payload: UpdatePurchaseOrderItemPayload = {
    quantity_alt_ordered: draft.quantity_alt_ordered ?? undefined,
    discount_percent: draft.discount_percent ?? undefined,
    quantity_tolerance_pct: draft.quantity_tolerance_pct ?? undefined,
    color_no: draft.color_no || undefined,
  };
  draft.saving = true;
  try {
    await updatePurchaseOrderItem(order.id, item.id, payload);
    msg.success('purchaseOrderItemUpdated', { orderNo: order.order_no });
    // 派生列（subtotal/discount_amount/tax_amount/total_amount、保密快照）由后端权威
    // 重算/反查，本地不自行拼算——通知父组件回源刷新详情与列表合计
    emit('saved');
  } catch (error: unknown) {
    const errMsg = error instanceof Error ? error.message : String(error);
    ElMessage.error(errMsg || msg.translate('updateFailed'));
  } finally {
    draft.saving = false;
  }
}
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    :title="t('purchase.viewDlg.title')"
    width="800px"
    :aria-label="t('purchase.viewDlg.ariaLabel')"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <template v-if="data">
      <el-descriptions :column="2" border>
        <el-descriptions-item :label="t('purchase.viewDlg.orderNo')">{{
          data.order_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.supplier')">{{
          data.supplier_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.orderDate')">{{
          data.order_date
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.requiredDate')">{{
          data.expected_delivery_date
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.totalAmount')"
          >¥{{ data.total_amount?.toLocaleString() }}</el-descriptions-item
        >
        <el-descriptions-item :label="t('purchase.viewDlg.receivedAmount')"
          >¥{{ data.received_amount }}</el-descriptions-item
        >
        <el-descriptions-item :label="t('purchase.viewDlg.paymentStatus')">
          <el-tag :type="getPaymentStatusType(data.payment_status ?? '')">{{
            getPaymentStatusText(data.payment_status ?? '')
          }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.status')">
          <el-tag :type="getStatusType(data.status)">{{ getStatusText(data.status) }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.creator')">{{
          data.creator_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.createdAt')">{{
          data.created_at
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.remark')" :span="2">{{
          data.notes
        }}</el-descriptions-item>
        <!-- 审批结论回显：与报价单域同范式（quotations/approval.vue:56-69）——后端出参
             （PurchaseOrderDto 的 approval_reason/rejected_reason，Option<String> 键恒存在）
             有值才出行；无值（null）整行不出、不补「无/暂无/未提供」占位文案（那属编造），
             更不做 ?? '' 兜底（会把缺键与无值混成一谈）；标签复用既有双语键 -->
        <el-descriptions-item
          v-if="data.approval_reason"
          :label="t('actionForm.approvalReasonTitle')"
          :span="2"
        >
          {{ data.approval_reason }}
        </el-descriptions-item>
        <el-descriptions-item
          v-if="data.rejected_reason"
          :label="t('actionForm.rejectReasonTitle')"
          :span="2"
        >
          {{ data.rejected_reason }}
        </el-descriptions-item>
        <!-- 附件回显（缺陷①修复后 PurchaseOrderDto.attachment_urls 真实出参，
             键名与 purchase_orders.attachment_urls 列同源；无附件显示 '-'，
             禁止静默吞键。locales 冻结期标签用中文字面量（同 outsourcing 页先例） -->
        <el-descriptions-item label="附件" :span="2">
          <template v-if="data.attachment_urls && data.attachment_urls.length > 0">
            <a
              v-for="url in data.attachment_urls"
              :key="url"
              :href="url"
              target="_blank"
              rel="noopener"
              style="display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap"
              >{{ url }}</a
            >
          </template>
          <span v-else>-</span>
        </el-descriptions-item>
      </el-descriptions>
      <div style="margin-top: 20px">
        <h4>{{ t('purchase.viewDlg.detailTitle') }}</h4>
        <el-table
          :data="data.items || []"
          border
          style="width: 100%"
          :aria-label="t('purchase.viewDlg.detailListAria')"
        >
          <el-table-column
            prop="product_name"
            :label="t('purchase.viewDlg.colProduct')"
            width="140"
          />
          <el-table-column
            prop="product_code"
            :label="t('purchase.viewDlg.colProductCode')"
            width="110"
          />
          <el-table-column prop="color_code" :label="t('purchase.viewDlg.colColor')" width="120">
            <template #default="{ row }">
              <el-input
                v-if="editable"
                v-model="drafts[row.id].color_no"
                size="small"
                :placeholder="t('purchase.viewDlg.colorPlaceholder')"
              />
              <span v-else>{{ row.color_code ?? '-' }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="quantity" :label="t('purchase.viewDlg.colQuantity')" width="90" />
          <el-table-column prop="quantity_alt" :label="t('purchase.viewDlg.colAltQty')" width="110">
            <template #default="{ row }">
              <el-input-number
                v-if="editable"
                v-model="drafts[row.id].quantity_alt_ordered"
                size="small"
                :controls="false"
                :min="0"
                style="width: 100%"
              />
              <!-- Decimal 出参是字符串：Number() 归一后展示，禁止对字符串直接 .toFixed -->
              <span v-else>{{ Number(row.quantity_alt ?? 0) }}</span>
            </template>
          </el-table-column>
          <el-table-column
            prop="discount_percent"
            :label="t('purchase.viewDlg.colDiscount')"
            width="100"
            align="right"
          >
            <template #default="{ row }">
              <el-input-number
                v-if="editable"
                v-model="drafts[row.id].discount_percent"
                size="small"
                :controls="false"
                :min="0"
                :max="100"
                style="width: 100%"
              />
              <span v-else>{{
                row.discount_percent != null ? Number(row.discount_percent) + '%' : '-'
              }}</span>
            </template>
          </el-table-column>
          <el-table-column
            prop="quantity_tolerance_pct"
            :label="t('purchase.viewDlg.colTolerance')"
            width="100"
            align="right"
          >
            <template #default="{ row }">
              <el-input-number
                v-if="editable"
                v-model="drafts[row.id].quantity_tolerance_pct"
                size="small"
                :controls="false"
                :min="0"
                :max="100"
                style="width: 100%"
              />
              <span v-else>{{
                row.quantity_tolerance_pct != null ? Number(row.quantity_tolerance_pct) + '%' : '-'
              }}</span>
            </template>
          </el-table-column>
          <el-table-column
            prop="unit_price"
            :label="t('purchase.viewDlg.colUnitPrice')"
            width="90"
          />
          <el-table-column prop="subtotal" :label="t('purchase.viewDlg.colSubtotal')" width="100" />
          <el-table-column
            prop="received_quantity"
            :label="t('purchase.viewDlg.colReceived')"
            width="90"
          />
          <el-table-column prop="notes" :label="t('purchase.viewDlg.colRemark')" width="120" />
          <el-table-column
            v-if="editable"
            :label="t('purchase.viewDlg.colOperation')"
            width="110"
            fixed="right"
          >
            <template #default="{ row }">
              <el-button
                size="small"
                type="primary"
                :loading="drafts[row.id]?.saving"
                @click="saveItem(row as PurchaseOrderItem)"
              >
                {{ t('purchase.viewDlg.saveItem') }}
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </div>
    </template>
  </el-dialog>
</template>
