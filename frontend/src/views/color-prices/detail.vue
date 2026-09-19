<!--
  面料多色号定价扩展 - 详情页
  色号价格详情 + 历史图表 + 阶梯价管理
  创建时间: 2026-06-18
-->
<template>
  <div v-loading="loading" class="color-price-detail">
    <el-page-header :content="$t('colorPrices.detail.back')" @back="$router.back()" />

    <el-row :gutter="20" style="margin-top: 20px">
      <el-col :span="12">
        <el-card>
          <template #header>
            <div class="card-header">
              <span>{{ $t('colorPrices.detail.basicInfo') }}</span>
              <span>
                <el-button size="small" type="primary" plain @click="openEditDialog">{{
                  $t('colorPrices.detail.edit')
                }}</el-button>
                <el-button size="small" type="success" plain @click="openApproveDialog">{{
                  $t('colorPrices.detail.approve')
                }}</el-button>
              </span>
            </div>
          </template>
          <el-descriptions :column="2" border>
            <el-descriptions-item :label="$t('colorPrices.detail.info.id')">{{
              price?.id
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorPrices.detail.info.productId')">{{
              price?.product_id
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorPrices.detail.info.colorId')">{{
              price?.color_id
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorPrices.detail.info.currency')">{{
              price?.currency
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorPrices.detail.info.basePrice')">
              <span v-if="price">{{ formatPrice(price.base_price, price.currency) }}</span>
            </el-descriptions-item>
            <el-descriptions-item :label="$t('colorPrices.detail.info.customerLevel')">
              <el-tag v-if="price?.customer_level" :type="getLevelColor(price.customer_level)">
                {{ getLevelLabel(price.customer_level) }}
              </el-tag>
              <span v-else>-</span>
            </el-descriptions-item>
            <el-descriptions-item :label="$t('colorPrices.detail.info.season')">
              <el-tag v-if="price?.season" :type="getSeasonColor(price.season)">
                {{ getSeasonLabel(price.season) }}
              </el-tag>
              <span v-else>-</span>
            </el-descriptions-item>
            <el-descriptions-item :label="$t('colorPrices.detail.info.priority')">{{
              price?.priority
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorPrices.detail.info.effectiveDate')">
              {{ price?.effective_from }} ~
              {{ price?.effective_to || $t('colorPrices.detail.info.longTerm') }}
            </el-descriptions-item>
            <el-descriptions-item :label="$t('colorPrices.detail.info.status')">
              <el-tag :type="price?.is_active ? 'success' : 'info'">
                {{
                  price?.is_active
                    ? $t('colorPrices.common.enable')
                    : $t('colorPrices.common.disable')
                }}
              </el-tag>
            </el-descriptions-item>
            <el-descriptions-item :label="$t('colorPrices.detail.info.approvalStatus')" :span="2">
              <el-tag :type="getApprovalColor(price?.approval_status || '')">
                {{ getApprovalLabel(price?.approval_status || '') }}
              </el-tag>
            </el-descriptions-item>
            <el-descriptions-item :label="$t('colorPrices.detail.info.notes')" :span="2">{{
              price?.notes || '-'
            }}</el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>

      <el-col :span="12">
        <el-card :header="$t('colorPrices.detail.priceHistory')">
          <PriceHistoryChart
            v-if="price"
            :history-data="history"
            :currency="price.currency"
            :height="300"
          />
          <el-empty v-else :description="$t('colorPrices.detail.noData')" />
        </el-card>
      </el-col>
    </el-row>

    <el-card :header="$t('colorPrices.detail.tier.title')" style="margin-top: 20px">
      <template #header>
        <div class="card-header">
          <span>{{ $t('colorPrices.detail.tier.title') }}</span>
          <el-button type="primary" :icon="Plus" @click="handleAddTier">{{
            $t('colorPrices.detail.tier.addTier')
          }}</el-button>
        </div>
      </template>
      <el-table :data="tiers" border :aria-label="$t('colorPrices.detail.tier.tableAriaLabel')">
        <el-table-column
          prop="sequence"
          :label="$t('colorPrices.detail.tier.sequence')"
          width="80"
        />
        <el-table-column
          prop="min_quantity"
          :label="$t('colorPrices.detail.tier.minQuantity')"
          width="120"
        />
        <el-table-column
          prop="max_quantity"
          :label="$t('colorPrices.detail.tier.maxQuantity')"
          width="120"
        >
          <template #default="{ row }">{{
            row.max_quantity || $t('colorPrices.detail.tier.unlimited')
          }}</template>
        </el-table-column>
        <el-table-column
          prop="tier_price"
          :label="$t('colorPrices.detail.tier.tierPrice')"
          width="120"
        />
        <el-table-column :label="$t('colorPrices.detail.tier.customerLevel')" width="120">
          <template #default="{ row }">
            <el-tag v-if="row.customer_level" :type="getLevelColor(row.customer_level)">
              {{ getLevelLabel(row.customer_level) }}
            </el-tag>
            <span v-else>{{ $t('colorPrices.detail.tier.general') }}</span>
          </template>
        </el-table-column>
        <el-table-column :label="$t('colorPrices.detail.tier.operation')" width="120" fixed="right">
          <template #default="{ row }">
            <el-button link type="danger" @click="handleDeleteTier(row)">{{
              $t('colorPrices.common.delete')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 批次 157c P1-1 修复：添加阶梯价对话框 -->
    <el-dialog
      v-model="tierDialogVisible"
      :title="$t('colorPrices.detail.tierDialog.title')"
      width="480px"
      :aria-label="$t('colorPrices.detail.tierDialog.ariaLabel')"
    >
      <el-form
        ref="tierFormRef"
        :model="tierForm"
        :rules="tierRules"
        label-width="100px"
        :aria-label="$t('colorPrices.detail.tierDialog.formAriaLabel')"
      >
        <el-form-item :label="$t('colorPrices.detail.tierDialog.minQuantity')" prop="min_quantity">
          <el-input-number v-model="tierForm.min_quantity" :min="1" style="width: 100%" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.detail.tierDialog.maxQuantity')">
          <el-input-number
            v-model="tierForm.max_quantity"
            :min="0"
            style="width: 100%"
            :placeholder="$t('colorPrices.detail.tierDialog.maxQuantityPlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.detail.tierDialog.tierPrice')" prop="tier_price">
          <el-input-number
            v-model="tierForm.tier_price"
            :min="0"
            :precision="4"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.detail.tierDialog.customerLevel')">
          <el-select
            v-model="tierForm.customer_level"
            :placeholder="$t('colorPrices.detail.tierDialog.customerLevelPlaceholder')"
            clearable
            style="width: 100%"
          >
            <el-option label="VIP" value="VIP" />
            <el-option label="A" value="A" />
            <el-option label="B" value="B" />
            <el-option label="C" value="C" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('colorPrices.detail.tierDialog.sequence')">
          <el-input-number v-model="tierForm.sequence" :min="1" style="width: 100%" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="tierDialogVisible = false">{{
          $t('colorPrices.common.cancel')
        }}</el-button>
        <el-button type="primary" :loading="tierSubmitting" @click="onSubmitTier">{{
          $t('colorPrices.common.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="editDialogVisible"
      :title="$t('colorPrices.detail.editDialog.title')"
      width="480"
    >
      <el-form :model="editForm" label-width="110px">
        <el-form-item :label="$t('colorPrices.detail.editDialog.basePrice')" required>
          <el-input-number v-model="editForm.base_price" :precision="2" :min="0" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.detail.editDialog.effectiveFrom')" required>
          <el-input v-model="editForm.effective_from" placeholder="2026-01-01" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.detail.editDialog.effectiveTo')">
          <el-input v-model="editForm.effective_to" placeholder="2026-12-31" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.detail.editDialog.priority')">
          <el-input-number v-model="editForm.priority" :min="0" :max="999" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.detail.editDialog.notes')">
          <el-input v-model="editForm.notes" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editDialogVisible = false">{{
          $t('colorPrices.common.cancel')
        }}</el-button>
        <el-button type="primary" :loading="editSaving" @click="onSubmitEdit">{{
          $t('colorPrices.common.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="approveDialogVisible"
      :title="$t('colorPrices.detail.approveDialog.title')"
      width="440"
    >
      <el-form :model="approveForm" label-width="110px">
        <el-form-item :label="$t('colorPrices.detail.approveDialog.decision')" required>
          <el-radio-group v-model="approveForm.decision">
            <el-radio value="APPROVED">{{
              $t('colorPrices.detail.approveDialog.approved')
            }}</el-radio>
            <el-radio value="REJECTED">{{
              $t('colorPrices.detail.approveDialog.rejected')
            }}</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item :label="$t('colorPrices.detail.approveDialog.comments')">
          <el-input
            v-model="approveForm.comments"
            type="textarea"
            :rows="2"
            :placeholder="$t('colorPrices.detail.approveDialog.commentsPlaceholder')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="approveDialogVisible = false">{{
          $t('colorPrices.common.cancel')
        }}</el-button>
        <el-button type="primary" :loading="approveSaving" @click="onSubmitApprove">{{
          $t('colorPrices.common.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute } from 'vue-router';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import {
  getColorPrice,
  getColorPriceHistory,
  getTierList,
  createTier,
  deleteTier,
  updateColorPrice,
  approveColorPrice,
  formatPrice,
  getLevelColor,
  getSeasonColor,
  getApprovalColor,
  type ColorPriceDetail,
  type PriceHistoryItem,
  type PriceTier,
  type CreatePriceTierDto,
} from '@/api/color-price';
import PriceHistoryChart from '@/components/PriceHistoryChart.vue';

const { t } = useI18n({ useScope: 'global' });

// 状态码 → 本地化标签（响应式：随语言切换自动更新）
const getLevelLabel = (level: string | null | undefined) =>
  t(`colorPrices.customerLevel.${level || 'default'}`);
const getSeasonLabel = (season: string | null | undefined) =>
  t(`colorPrices.season.${season || 'default'}`);
const getApprovalLabel = (status: string) => t(`colorPrices.approvalStatus.${status}`);

const route = useRoute();
const loading = ref(false);
const price = ref<ColorPriceDetail | null>(null);
const history = ref<PriceHistoryItem[]>([]);
const tiers = ref<PriceTier[]>([]);

const priceId = Number(route.params.id);

const loadData = async () => {
  loading.value = true;
  try {
    price.value = await getColorPrice(priceId);
    const h = await getColorPriceHistory(priceId);
    history.value = h.items;
    const tierRes = await getTierList(priceId);
    tiers.value = tierRes.items;
  } catch (e: unknown) {
    // v11 批次 174 P2-1 修复：catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      t('colorPrices.message.loadFailed', { msg: e instanceof Error ? e.message : String(e) })
    );
  } finally {
    loading.value = false;
  }
};

// 批次 157c P1-1 修复：添加阶梯价对话框接入 createTier API
const tierDialogVisible = ref(false);
const tierSubmitting = ref(false);
const tierFormRef = ref<FormInstance>();
const tierForm = reactive<{
  min_quantity: number;
  max_quantity: number | null;
  tier_price: number;
  customer_level: string | null;
  sequence: number;
}>({
  min_quantity: 1,
  max_quantity: null,
  tier_price: 0,
  customer_level: null,
  sequence: 1,
});
// 表单校验规则（响应式：随语言切换自动更新提示文案）
const tierRules = computed<FormRules>(() => ({
  min_quantity: [
    { required: true, message: t('colorPrices.validation.minQuantityRequired'), trigger: 'blur' },
  ],
  tier_price: [
    { required: true, message: t('colorPrices.validation.tierPriceRequired'), trigger: 'blur' },
  ],
}));

const handleAddTier = () => {
  tierForm.min_quantity = 1;
  tierForm.max_quantity = null;
  tierForm.tier_price = 0;
  tierForm.customer_level = null;
  tierForm.sequence = (tiers.value.length || 0) + 1;
  tierDialogVisible.value = true;
};

const onSubmitTier = async () => {
  if (!tierFormRef.value) return;
  await tierFormRef.value.validate(async valid => {
    if (!valid) return;
    tierSubmitting.value = true;
    try {
      const payload: CreatePriceTierDto = {
        product_color_price_id: priceId,
        min_quantity: tierForm.min_quantity,
        max_quantity:
          tierForm.max_quantity && tierForm.max_quantity > 0 ? tierForm.max_quantity : null,
        tier_price: tierForm.tier_price,
        customer_level: tierForm.customer_level || null,
        sequence: tierForm.sequence,
      };
      await createTier(payload);
      ElMessage.success(t('colorPrices.message.tierAddSuccess'));
      tierDialogVisible.value = false;
      loadData();
    } catch (e: unknown) {
      // v11 批次 174 P2-1 修复：catch (e: any) 改为 unknown + 类型守卫
      ElMessage.error(
        t('colorPrices.message.tierAddFailed', { msg: e instanceof Error ? e.message : String(e) })
      );
    } finally {
      tierSubmitting.value = false;
    }
  });
};

const handleDeleteTier = async (row: PriceTier) => {
  try {
    await ElMessageBox.confirm(
      t('colorPrices.message.deleteTierConfirm', { id: row.id }),
      t('colorPrices.common.confirm'),
      { type: 'warning' }
    );
    await deleteTier(row.id);
    ElMessage.success(t('colorPrices.message.deleteSuccess'));
    loadData();
  } catch (e: unknown) {
    // v11 批次 174 P2-1 修复：catch (e: any) 改为 unknown + 类型守卫
    if (e === 'cancel') return;
    ElMessage.error(
      t('colorPrices.message.deleteFailed', { msg: e instanceof Error ? e.message : String(e) })
    );
  }
};

// 编辑色号价格（updateColorPrice）
const editDialogVisible = ref(false);
const editSaving = ref(false);
const editForm = reactive({
  base_price: 0,
  effective_from: '',
  effective_to: '',
  priority: 0,
  notes: '',
});

const openEditDialog = () => {
  if (!price.value) return;
  Object.assign(editForm, {
    base_price: Number(price.value.base_price) || 0,
    effective_from: price.value.effective_from || '',
    effective_to: price.value.effective_to || '',
    priority: price.value.priority || 0,
    notes: price.value.notes || '',
  });
  editDialogVisible.value = true;
};

const onSubmitEdit = async () => {
  if (!editForm.effective_from) {
    ElMessage.warning(t('colorPrices.detail.editDialog.effectiveFrom'));
    return;
  }
  editSaving.value = true;
  try {
    await updateColorPrice(priceId, {
      base_price: editForm.base_price,
      effective_from: editForm.effective_from,
      effective_to: editForm.effective_to || null,
      priority: editForm.priority,
      notes: editForm.notes || null,
    });
    ElMessage.success(t('colorPrices.message.createSuccess'));
    editDialogVisible.value = false;
    loadData();
  } catch (e: unknown) {
    ElMessage.error(
      t('colorPrices.message.loadFailed', { msg: e instanceof Error ? e.message : String(e) })
    );
  } finally {
    editSaving.value = false;
  }
};

// 价格审批（approveColorPrice）
const approveDialogVisible = ref(false);
const approveSaving = ref(false);
const approveForm = reactive({
  decision: 'APPROVED' as 'APPROVED' | 'REJECTED',
  comments: '',
});

const openApproveDialog = () => {
  approveForm.decision = 'APPROVED';
  approveForm.comments = '';
  approveDialogVisible.value = true;
};

const onSubmitApprove = async () => {
  approveSaving.value = true;
  try {
    await approveColorPrice(priceId, {
      decision: approveForm.decision,
      comments: approveForm.comments || undefined,
    });
    ElMessage.success(t('colorPrices.message.createSuccess'));
    approveDialogVisible.value = false;
    loadData();
  } catch (e: unknown) {
    ElMessage.error(
      t('colorPrices.message.loadFailed', { msg: e instanceof Error ? e.message : String(e) })
    );
  } finally {
    approveSaving.value = false;
  }
};

onMounted(() => {
  loadData();
});
</script>

<style scoped>
.color-price-detail {
  padding: 20px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
