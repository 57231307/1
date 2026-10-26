<template>
  <div class="color-card-detail">
    <el-page-header
      :icon="ArrowLeft"
      :content="$t('colorCards.detail.back')"
      @back="$router.push('/color-cards/list')"
    />

    <el-card v-loading="loading" style="margin-top: 16px">
      <template #header>
        <div class="card-header">
          <div>
            <h2 style="margin: 0">{{ card?.card_name }}</h2>
            <div style="color: #909399; margin-top: 4px">
              <span>{{ card?.card_no }}</span>
              <el-divider direction="vertical" />
              <el-tag size="small" :type="COLOR_CARD_STATUS_COLORS[card?.status || ''] as TagType">
                {{ getStatusLabel(card?.status || '') || card?.status }}
              </el-tag>
            </div>
          </div>
          <div>
            <!-- 卡面信息编辑入口与列表页编辑入口同源判据，草稿态且持 color-cards 更新键才渲染。
                 已在编辑态时收起，保存与取消的落点在基本信息页签内的表单底部 -->
            <el-button
              v-if="isDraftCard && !editing"
              v-permission-detail="{ resource: 'color-cards', action: 'update' }"
              :icon="Edit"
              @click="openEdit"
              >{{ $t('common.edit') }}</el-button
            >
            <el-button :icon="Download" @click="handleExport">{{
              $t('colorCards.detail.exportExcel')
            }}</el-button>
            <!-- 色号维护入口按卡状态门控（后端 98f3bb0b：色号创建只允许草稿态色卡）。
                 判据与 color-cards/list.vue:138 归档按钮同源（status === draft，
                 token 由 COLOR_CARD_STATUS 权威表锁定）；非草稿态禁用入口并给出
                 有解释的 tooltip，消除“按钮永远可点、点了必失败”的假可用 -->
            <el-tooltip :disabled="isDraftCard" :content="addItemDisabledTip" placement="top">
              <span>
                <el-button
                  type="primary"
                  :icon="Plus"
                  :disabled="!isDraftCard"
                  @click="showAddItemDialog = true"
                >
                  {{ $t('colorCards.detail.addItem') }}
                </el-button>
              </span>
            </el-tooltip>
            <!-- 批量导入入口与单条添加同源门控：批量端点同样落色号创建 -->
            <el-tooltip :disabled="isDraftCard" :content="batchImportDisabledTip" placement="top">
              <span>
                <el-button :icon="Box" :disabled="!isDraftCard" @click="showImportDialog = true">
                  {{ $t('colorCards.detail.batchImport') }}
                </el-button>
              </span>
            </el-tooltip>
          </div>
        </div>
      </template>

      <el-tabs v-model="activeTab">
        <!-- 基本信息 -->
        <el-tab-pane :label="$t('colorCards.detail.tab.info')" name="info">
          <!-- 只读区里的卡号在建卡时登记且后端更新端点不收该键，色号总数与三个数量均由后端派生，
               编辑态也不给这些列输入口，提交载荷的字段集里结构上不存在它们 -->
          <el-descriptions :column="2" border>
            <el-descriptions-item :label="$t('colorCards.detail.info.cardNo')">{{
              card?.card_no
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.detail.info.totalColors')">{{
              card?.total_colors
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.table.status')">
              <el-tag :type="COLOR_CARD_STATUS_COLORS[card?.status || ''] as TagType">
                {{ getStatusLabel(card?.status || '') || card?.status }}
              </el-tag>
            </el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.table.createdAt')">{{
              formatDate(card?.created_at)
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.detail.info.stockQuantity')">{{
              card?.stock_quantity ?? '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.detail.info.issuedQuantity')">{{
              card?.issued_quantity ?? '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.detail.info.availableQuantity')">{{
              availableStock
            }}</el-descriptions-item>
          </el-descriptions>
          <div class="stock-actions">
            <el-button type="primary" plain @click="openStockDialog">{{
              $t('colorCards.detail.stockDialog.entry')
            }}</el-button>
          </div>

          <!-- 卡面信息只读展示，与下方编辑表单同一字段集，按编辑态互斥渲染 -->
          <el-descriptions v-if="!editing" :column="2" border class="card-info-panel">
            <el-descriptions-item :label="$t('colorCards.detail.info.cardName')">{{
              card?.card_name
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.detail.info.cardType')">{{
              getCardTypeLabel(card?.card_type || '') || card?.card_type
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.filter.season')">{{
              card?.season ? getSeasonLabel(card.season) || card.season : '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.detail.info.brand')">{{
              card?.brand || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.create.coverImageUrl')">{{
              card?.cover_image_url || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.detail.info.description')" :span="2">{{
              card?.description || '-'
            }}</el-descriptions-item>
          </el-descriptions>

          <!-- 卡面信息编辑态，字段集与后端更新 DTO 的可写键逐一对齐，缺省即不改的语义由后端持有，
               提交后整卡重读，页面回显以库里的真值为准 -->
          <el-form
            v-else
            ref="editFormRef"
            :model="editForm"
            :rules="editRules"
            label-width="100px"
            class="card-info-panel"
            :aria-label="$t('colorCards.detail.tab.info')"
            @submit.prevent
          >
            <el-form-item :label="$t('colorCards.detail.info.cardName')" prop="card_name">
              <el-input
                v-model="editForm.card_name"
                :placeholder="$t('colorCards.create.cardNamePlaceholder')"
              />
            </el-form-item>
            <el-form-item :label="$t('colorCards.detail.info.cardType')" prop="card_type">
              <el-select
                v-model="editForm.card_type"
                :placeholder="$t('colorCards.create.pleaseSelect')"
                style="width: 100%"
              >
                <el-option
                  v-for="value in cardTypeKeys"
                  :key="value"
                  :label="getCardTypeLabel(value)"
                  :value="value"
                />
              </el-select>
            </el-form-item>
            <el-form-item :label="$t('colorCards.filter.season')" prop="season">
              <el-select
                v-model="editForm.season"
                :placeholder="$t('colorCards.create.pleaseSelect')"
                style="width: 100%"
              >
                <el-option
                  v-for="value in seasonKeys"
                  :key="value"
                  :label="getSeasonLabel(value)"
                  :value="value"
                />
              </el-select>
            </el-form-item>
            <el-form-item :label="$t('colorCards.detail.info.brand')" prop="brand">
              <el-input
                v-model="editForm.brand"
                :placeholder="$t('colorCards.create.brandPlaceholder')"
              />
            </el-form-item>
            <el-form-item :label="$t('colorCards.create.coverImageUrl')" prop="cover_image_url">
              <el-input
                v-model="editForm.cover_image_url"
                :placeholder="$t('colorCards.create.coverImageUrlPlaceholder')"
              />
            </el-form-item>
            <el-form-item :label="$t('colorCards.detail.info.description')" prop="description">
              <el-input
                v-model="editForm.description"
                type="textarea"
                :rows="3"
                :placeholder="$t('colorCards.create.descriptionPlaceholder')"
              />
            </el-form-item>
            <el-form-item>
              <el-button
                v-permission-detail="{ resource: 'color-cards', action: 'update' }"
                type="primary"
                :loading="saving"
                @click="handleSaveEdit"
                >{{ $t('common.save') }}</el-button
              >
              <el-button @click="cancelEdit">{{ $t('common.cancel') }}</el-button>
            </el-form-item>
          </el-form>
        </el-tab-pane>

        <!-- 色号列表 -->
        <el-tab-pane
          :label="$t('colorCards.detail.tab.items', { count: items.length })"
          name="items"
        >
          <ColorCardGrid :items="items" @delete="handleDeleteItem" @scan="handleScanItem" />
        </el-tab-pane>

        <!-- 发放历史 -->
        <el-tab-pane :label="$t('colorCards.detail.tab.issue')" name="issue">
          <IssueRecordTimeline :records="issueRecords" />
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <!-- 添加色号对话框 -->
    <el-dialog
      v-model="showAddItemDialog"
      :title="$t('colorCards.detail.addItemDialog.title')"
      width="640px"
      :aria-label="$t('colorCards.detail.addItemDialog.ariaLabel')"
    >
      <ColorItemEditor v-model="newItem" />
      <template #footer>
        <el-button @click="showAddItemDialog = false">{{
          $t('colorCards.detail.addItemDialog.cancel')
        }}</el-button>
        <el-button type="primary" :loading="adding" @click="handleAddItem">{{
          $t('colorCards.detail.addItemDialog.add')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 库存登记对话框 -->
    <el-dialog
      v-model="showStockDialog"
      :title="$t('colorCards.detail.stockDialog.title')"
      width="420px"
      :aria-label="$t('colorCards.detail.stockDialog.ariaLabel')"
    >
      <el-form label-width="96px" @submit.prevent>
        <el-form-item :label="$t('colorCards.detail.stockDialog.field')">
          <NumericInput
v-model="stockInput"
:placeholder="$t('colorCards.detail.stockDialog.placeholder')"
:min="0"
:integer="true"
/>
        </el-form-item>
        <el-form-item>
          <span class="stock-hint">{{
            $t('colorCards.detail.stockDialog.hint', {
              issued: card?.issued_quantity ?? 0,
              available: availableStock,
            })
          }}</span>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showStockDialog = false">{{
          $t('colorCards.detail.stockDialog.cancel')
        }}</el-button>
        <el-button type="primary" :loading="stockSubmitting" @click="handleRegisterStock">{{
          $t('colorCards.detail.stockDialog.save')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 批量导入对话框 -->
    <el-dialog
      v-model="showImportDialog"
      :title="$t('colorCards.detail.importDialog.title')"
      width="600px"
      :aria-label="$t('colorCards.detail.importDialog.ariaLabel')"
    >
      <el-alert type="info" :closable="false" style="margin-bottom: 16px">
        {{ $t('colorCards.detail.importDialog.alert') }}
      </el-alert>
      <el-input
        v-model="importText"
        type="textarea"
        :rows="10"
        :placeholder="$t('colorCards.detail.importDialog.placeholder')"
      />
      <template #footer>
        <el-button @click="showImportDialog = false">{{
          $t('colorCards.detail.importDialog.cancel')
        }}</el-button>
        <el-button type="primary" :loading="importing" @click="handleBatchImport">{{
          $t('colorCards.detail.importDialog.import')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 扫码详情对话框 -->
    <el-dialog
      v-model="showScanDialog"
      :title="$t('colorCards.detail.scanDialog.title')"
      width="720px"
      :aria-label="$t('colorCards.detail.scanDialog.ariaLabel')"
    >
      <el-descriptions v-if="scanResult" :column="3" border>
        <el-descriptions-item :label="$t('colorCards.detail.scanDialog.colorCode')">{{
          scanResult.color_item?.color_code
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('colorCards.detail.scanDialog.colorName')">{{
          scanResult.color_item?.color_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('colorCards.detail.scanDialog.colorCard')">{{
          scanResult.color_card_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('colorCards.detail.scanDialog.hex')">
          <div
            :style="{
              background: scanResult.color_item?.hex_value,
              width: '40px',
              height: '20px',
              display: 'inline-block',
              marginRight: '8px',
            }"
          />
          {{ scanResult.color_item?.hex_value }}
        </el-descriptions-item>
        <el-descriptions-item :label="$t('colorCards.detail.scanDialog.rgb')">
          {{ scanResult.color_item?.rgb_r }}, {{ scanResult.color_item?.rgb_g }},
          {{ scanResult.color_item?.rgb_b }}
        </el-descriptions-item>
        <el-descriptions-item :label="$t('colorCards.detail.scanDialog.cmyk')">
          {{ scanResult.color_item?.cmyk_c }}, {{ scanResult.color_item?.cmyk_m }},
          {{ scanResult.color_item?.cmyk_y }}, {{ scanResult.color_item?.cmyk_k }}
        </el-descriptions-item>
        <el-descriptions-item :label="$t('colorCards.detail.scanDialog.cieLab')" :span="3">
          L={{ scanResult.color_item?.lab_l }}, a={{ scanResult.color_item?.lab_a }}, b={{
            scanResult.color_item?.lab_b
          }}
        </el-descriptions-item>
        <el-descriptions-item
          v-if="scanResult.recipe_summary"
          :label="$t('colorCards.detail.scanDialog.recipe')"
          :span="2"
        >
          {{ scanResult.recipe_summary.recipe_name }} ({{ scanResult.recipe_summary.color_no }})
        </el-descriptions-item>
        <el-descriptions-item
          v-if="scanResult.price_summary"
          :label="$t('colorCards.detail.scanDialog.price')"
        >
          {{ scanResult.price_summary.currency }} {{ scanResult.price_summary.base_price }}
        </el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { isDialogDismissal } from '@/utils/monitor';
import NumericInput from '@/components/NumericInput.vue';
import { ref, reactive, computed, onMounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';
import { ElMessage, ElMessageBox, FormInstance, FormRules } from 'element-plus';
import { ArrowLeft, Plus, Box, Download, Edit } from '@element-plus/icons-vue';
import {
  getColorCard,
  getIssueList,
  createColorItem,
  deleteColorItem,
  batchImportItems,
  scanColorCode,
  exportColorCardUrl,
  updateColorCard,
  COLOR_CARD_STATUS,
  COLOR_CARD_STATUS_COLORS,
  COLOR_CARD_TYPE,
  SEASON_LABELS,
  type ColorCardDetail,
  type ColorItemInfo,
  type ColorItemPayload,
  type IssueRecordInfo,
} from '@/api/color-card';
import { logAuxLoadFailure } from '@/utils/logger';
import ColorCardGrid from '@/components/ColorCardGrid.vue';
import ColorItemEditor from '@/components/ColorItemEditor.vue';
import IssueRecordTimeline from '@/components/IssueRecordTimeline.vue';

const { t } = useI18n({ useScope: 'global' });

// 状态码 → 本地化标签（响应式：随语言切换自动更新）
const getCardTypeLabel = (key: string) => t(`colorCards.cardType.${key}`);
const getSeasonLabel = (key: string) => t(`colorCards.season.${key}`);
const getStatusLabel = (key: string) => t(`colorCards.cardStatus.${key}`);

// 编辑表单的下拉取值域，键集来自色卡 api 层的权威表，标签走 i18n，
// 与色卡列表页的筛选下拉同源，不新引入第二套词表
const cardTypeKeys = Object.keys(COLOR_CARD_TYPE);
const seasonKeys = Object.keys(SEASON_LABELS);

// v11 批次 173 P2-1 修复：扫码结果接口类型，替代 any
interface ScanRecipeSummary {
  recipe_name: string;
  color_no: string;
}
interface ScanPriceSummary {
  currency: string;
  base_price: number;
}
interface ScanResult {
  color_item?: ColorItemInfo;
  color_card_name?: string;
  recipe_summary?: ScanRecipeSummary;
  price_summary?: ScanPriceSummary;
}

// v11 批次 173 P2-1 修复：el-tag type 类型
type TagType = 'success' | 'warning' | 'info' | 'primary' | 'danger';

const route = useRoute();
const router = useRouter();
const cardId = computed(() => Number(route.params.id));
const loading = ref(false);
const card = ref<ColorCardDetail | null>(null);
const items = ref<ColorItemInfo[]>([]);
const issueRecords = ref<IssueRecordInfo[]>([]);
const activeTab = ref('info');

// ===== 色号维护入口的草稿态门控 =====
// 色卡状态 token 锁定在 api/color-card.ts 权威表 COLOR_CARD_STATUS 的键上
//（键 = 后端 chk_color_card_status 全集 token，值为标签）——'draft' 字面量
// 经 keyof 编译期校验，不新引入第二套词表，拼错即红。
type ColorCardStatusToken = keyof typeof COLOR_CARD_STATUS;
const DRAFT: ColorCardStatusToken = 'draft';
// 判据与 color-cards/list.vue 归档按钮（row.status === 'draft'）同源
const isDraftCard = computed(() => card.value?.status === DRAFT);

// 可用量按后端同一口径算（stock_quantity - issued_quantity），仅用于展示，
// 真正的发放守门仍在服务端，前端不做第二套判定
const availableStock = computed(() => {
  if (!card.value) return '-';
  return (card.value.stock_quantity ?? 0) - (card.value.issued_quantity ?? 0);
});

// 非草稿态禁用解释文案：全部由既有条目词表动态组装（卡状态标签、入口名），不硬编码文案。
const draftOnlyTip = (actionLabel: string) =>
  t('colorCards.detail.draftOnlyTip', {
    draft: getStatusLabel(DRAFT),
    action: actionLabel,
    current: card.value ? getStatusLabel(card.value.status) : '',
  });
const addItemDisabledTip = computed(() => draftOnlyTip(t('colorCards.detail.addItem')));
const batchImportDisabledTip = computed(() => draftOnlyTip(t('colorCards.detail.batchImport')));

// ===== 卡面信息编辑态 =====
// 可写字段集与后端更新 DTO 的键逐一对齐。卡号（建卡时登记且更新端点不收该键）、色号总数、
// 状态、已发放量与创建时间均由后端派生，不在该集合内，因此提交载荷里结构上不可能出现这些键，
// 不靠运行时兜底默认值去挡。
type EditableCardField =
  'card_name' | 'card_type' | 'season' | 'brand' | 'description' | 'cover_image_url';

const EDITABLE_FIELDS: readonly EditableCardField[] = [
  'card_name',
  'card_type',
  'season',
  'brand',
  'description',
  'cover_image_url',
];

const editing = ref(false);
const saving = ref(false);
const editFormRef = ref<FormInstance>();
const editForm = reactive<Record<EditableCardField, string>>({
  card_name: '',
  card_type: '',
  season: '',
  brand: '',
  description: '',
  cover_image_url: '',
});
// 进入编辑态时的库里现值快照，提交只发与快照不同的键，校验也靠它区分
// "该列本就没有值"与"用户把已有的值清掉了"
const cardSnapshot = ref<Record<EditableCardField, string>>({
  card_name: '',
  card_type: '',
  season: '',
  brand: '',
  description: '',
  cover_image_url: '',
});

// 文本与下拉控件取不了空值以外的"未填"形态，可空列在表单里以空字符串呈现。
// 这只是控件取值，不改变库里该列是否为空的判定，判定一律读快照
const editableSnapshotOf = (detail: ColorCardDetail): Record<EditableCardField, string> => ({
  card_name: detail.card_name,
  card_type: detail.card_type,
  season: detail.season ?? '',
  brand: detail.brand ?? '',
  description: detail.description ?? '',
  cover_image_url: detail.cover_image_url ?? '',
});

/**
 * 可空列的清空守门，供编辑表单的校验规则使用，入参为待校验字段与该列的填法提示键与触发时机。
 * 库里该列已有值而本次被清空时拒绝提交，要求按原提示填回，交给表单校验展示，不写库。
 * 后端更新 DTO 以"缺省即不改"表达更新，没有把可空列写回空值的入参形态，
 * 提交空串只会把空字符串落库而不是空值，因此这里宁可拒绝也不静默丢弃，也不落错值。
 */
const keepFilledRule = (
  field: EditableCardField,
  messageKey: string,
  trigger: 'blur' | 'change'
) => ({
  validator: (_rule: unknown, value: unknown, callback: (error?: Error) => void) => {
    const text = typeof value === 'string' ? value.trim() : '';
    if (cardSnapshot.value[field] !== '' && text === '') {
      callback(new Error(t(messageKey)));
      return;
    }
    callback();
  },
  trigger,
});

// 校验规则随语言切换自动更新提示文案，与新建页同源口径
const editRules = computed<FormRules>(() => ({
  card_name: [
    { required: true, message: t('colorCards.validation.cardNameRequired'), trigger: 'blur' },
  ],
  card_type: [
    { required: true, message: t('colorCards.validation.cardTypeRequired'), trigger: 'change' },
  ],
  season: [keepFilledRule('season', 'colorCards.create.pleaseSelect', 'change')],
  brand: [keepFilledRule('brand', 'colorCards.create.brandPlaceholder', 'blur')],
  description: [keepFilledRule('description', 'colorCards.create.descriptionPlaceholder', 'blur')],
  cover_image_url: [
    keepFilledRule('cover_image_url', 'colorCards.create.coverImageUrlPlaceholder', 'blur'),
  ],
}));

/**
 * 进入编辑态，由页头编辑按钮与地址栏编辑意图判定调用，入参是当前整卡数据。
 * 把库里现值灌进编辑表单并留一份快照，不写库。
 * 编辑表单落在基本信息页签，因此同时把页签切过去，避免编辑态在别的页签后面无迹可循。
 */
const openEdit = () => {
  if (!card.value) return;
  const snapshot = editableSnapshotOf(card.value);
  cardSnapshot.value = snapshot;
  Object.assign(editForm, snapshot);
  activeTab.value = 'info';
  editing.value = true;
};

// 编辑意图是一次性导航参数，进入过编辑态就从地址栏摘掉，避免刷新页面又落回编辑态
const stripEditQuery = () => {
  if (route.query.edit === undefined) return;
  const nextQuery = { ...route.query };
  delete nextQuery.edit;
  router.replace({ query: nextQuery });
};

const cancelEdit = () => {
  editing.value = false;
  editFormRef.value?.clearValidate();
  stripEditQuery();
};

/**
 * 组装提交载荷，只保留与库里现值不同且非空的键，入参为编辑表单与进入编辑态时的快照。
 * 载荷传给 api 层的 updateColorCard，即后端色卡更新端点，本函数不写库，只出差集载荷。
 * 空值不进载荷，后端把缺省解释为不改该列。卡号与后端派生量不在字段集内，结构上不可能被带上。
 */
const buildCardUpdatePayload = () => {
  const payload: Parameters<typeof updateColorCard>[1] = {};
  EDITABLE_FIELDS.forEach(field => {
    const next = editForm[field].trim();
    if (next === '' || next === cardSnapshot.value[field]) return;
    payload[field] = next;
  });
  return payload;
};

/**
 * 保存卡面信息，由编辑表单的保存按钮触发，入参是编辑表单的差集载荷。
 * 经 updateColorCard 写后端色卡更新端点，成功后整卡重读，页面回显以库里的真值为准。
 * 无差集时不发请求也不空转一次更新，直接退出编辑态。
 * 拒绝原因由请求拦截器统一外显（含后端点名的成文口径），这里只留痕并保持编辑态，
 * 用户按提示改完可再存。
 */
const handleSaveEdit = async () => {
  const valid = await editFormRef.value?.validate().catch(() => false);
  if (!valid) return;
  const payload = buildCardUpdatePayload();
  if (Object.keys(payload).length === 0) {
    cancelEdit();
    return;
  }
  saving.value = true;
  try {
    await updateColorCard(cardId.value, payload);
    // 不信任 PUT 响应体形状，提交后整卡重读，页面上的卡面信息以库里的值呈现
    await loadData();
    editing.value = false;
    stripEditQuery();
    ElMessage.success(t('common.message.updateSuccess'));
  } catch (e: unknown) {
    logAuxLoadFailure('色卡卡面信息保存失败', e);
  } finally {
    saving.value = false;
  }
};

/**
 * 地址栏编辑意图判定，在本页挂载且整卡数据就绪之后与 edit 参数变化时调用，
 * 入参是路由 query 与整卡数据，交给 editing 状态与只读展示，不写库。
 * 判据与页头编辑入口同源，草稿态且不在编辑态才进编辑态。
 * 非草稿态点名解释并摘掉参数，不让用户停在"点了编辑其实只是查看"的假可供性上。
 */
const applyEditIntent = () => {
  if (route.query.edit === undefined) {
    editing.value = false;
    return;
  }
  // 整卡数据晚于导航到达，取不到卡就不判，等数据就绪后由 onMounted 补判一次
  if (!card.value) return;
  if (!isDraftCard.value) {
    ElMessage.warning(draftOnlyTip(t('common.edit')));
    stripEditQuery();
    return;
  }
  if (!editing.value) openEdit();
};

const showAddItemDialog = ref(false);
const adding = ref(false);
const newItem = ref<Partial<ColorItemInfo>>({
  color_code: '',
  color_name: '',
  rgb_r: 0,
  rgb_g: 0,
  rgb_b: 0,
  hex_value: '#000000',
});

const showImportDialog = ref(false);
const importing = ref(false);

// ===== 色卡实物库存登记 =====
// 库存不是卡面属性，而是"这批色卡印了多少张"的离散记账动作，因此不套草稿态门控：
// 归档或停用后仍可能补登印制数量，而发放守门读的始终是库里的 stock_quantity。
// 初值取库里现值而不是 0：0 会让"没印够"被读成一次合法的低库存声明。
const showStockDialog = ref(false);
const stockSubmitting = ref(false);
const stockInput = ref<number | undefined>(undefined);

const openStockDialog = () => {
  stockInput.value = card.value?.stock_quantity;
  showStockDialog.value = true;
};

const handleRegisterStock = async () => {
  if (stockInput.value == null) {
    ElMessage.warning(t('colorCards.detail.stockDialog.required'));
    return;
  }
  stockSubmitting.value = true;
  try {
    await updateColorCard(cardId.value, { stock_quantity: stockInput.value });
    // 不信任 PUT 响应体形状：提交后整卡重读，页面上三个数量都以库里的值呈现
    await loadData();
    ElMessage.success(t('colorCards.detail.stockDialog.saved'));
    showStockDialog.value = false;
  } catch (e: unknown) {
    const errObj = e as { response?: { data?: { message?: string } } };
    const reason = errObj?.response?.data?.message;
    // 库存不得低于已发放量这类拒绝是后端点名的成文口径，原文要外显给用户，
    // 只报"保存失败"会让人以为是不受控的前端问题
    if (reason) {
      ElMessage.error(reason);
    } else {
      logAuxLoadFailure('色卡库存登记失败', e);
      ElMessage.error(t('colorCards.detail.stockDialog.saveFailed'));
    }
  } finally {
    stockSubmitting.value = false;
  }
};
const importText = ref('');

const showScanDialog = ref(false);
// v11 批次 173 P2-1 修复：ref<any>(null) 改为 ref<ScanResult | null>(null)
const scanResult = ref<ScanResult | null>(null);

const loadData = async () => {
  loading.value = true;
  try {
    // v11 批次 173 P2-1 修复：const res: any 改为 as 具体类型
    const cardRes = (await getColorCard(cardId.value)) as { data: ColorCardDetail };
    card.value = cardRes.data;
    items.value = cardRes.data.items || [];

    const recordsRes = (await getIssueList({ color_card_id: cardId.value, page_size: 50 })) as {
      data?: { items?: IssueRecordInfo[] };
    };
    issueRecords.value = recordsRes.data?.items || [];
  } catch (e: unknown) {
    logAuxLoadFailure('色卡详情加载失败', e);
  } finally {
    loading.value = false;
  }
};

// 编辑器绑定的是采集态（Partial），提交前收敛为后端 ColorItemDto 全量必填载荷
const buildColorItemPayload = (n: Partial<ColorItemInfo>): ColorItemPayload => {
  const r = n.rgb_r ?? 0;
  const g = n.rgb_g ?? 0;
  const b = n.rgb_b ?? 0;
  const hexFromRgb = `#${[r, g, b]
    .map(v => Math.max(0, Math.min(255, v)).toString(16).padStart(2, '0'))
    .join('')
    .toUpperCase()}`;
  const payload: ColorItemPayload = {
    color_code: n.color_code ?? '',
    color_name: n.color_name ?? '',
    rgb_r: r,
    rgb_g: g,
    rgb_b: b,
    // 后端要求 hex 恰为 #RRGGBB（validate length=7 + service 格式校验），非法/缺失时按 RGB 兜底换算
    hex_value: n.hex_value && /^#[0-9A-Fa-f]{6}$/.test(n.hex_value) ? n.hex_value : hexFromRgb,
  };
  // Option 字段空值省略键（Some("") 会被后端原样写库），CMYK/Lab 省略由后端自动计算
  if (n.pantone_code) payload.pantone_code = n.pantone_code;
  if (n.cncs_code) payload.cncs_code = n.cncs_code;
  if (n.custom_code) payload.custom_code = n.custom_code;
  if (n.swatch_image_url) payload.swatch_image_url = n.swatch_image_url;
  if (typeof n.sequence === 'number') payload.sequence = n.sequence;
  return payload;
};

const handleAddItem = async () => {
  // 双保险：入口按钮已由 isDraftCard 禁用，提交路径再校验一次（防对话框
  // 已打开时卡状态变化后的竞态提交）；非草稿态显式拒绝并解释，
  // 不是“点了必失败”地放行给后端 98f3bb0b 的 400
  if (!isDraftCard.value) {
    ElMessage.warning(addItemDisabledTip.value);
    return;
  }
  if (!newItem.value.color_code || !newItem.value.color_name) {
    ElMessage.warning(t('colorCards.detail.message.codeAndNameRequired'));
    return;
  }
  adding.value = true;
  try {
    await createColorItem(cardId.value, buildColorItemPayload(newItem.value));
    ElMessage.success(t('colorCards.detail.message.addItemSuccess'));
    showAddItemDialog.value = false;
    newItem.value = {
      color_code: '',
      color_name: '',
      rgb_r: 0,
      rgb_g: 0,
      rgb_b: 0,
      hex_value: '#000000',
    };
    loadData();
  } finally {
    adding.value = false;
  }
};

const handleDeleteItem = async (item: ColorItemInfo) => {
  try {
    await ElMessageBox.confirm(
      t('colorCards.detail.message.deleteItemConfirm', { code: item.color_code }),
      t('colorCards.detail.message.deleteItemConfirmTitle'),
      { type: 'warning' }
    );
    await deleteColorItem(cardId.value, item.id);
    ElMessage.success(t('colorCards.detail.message.deleteSuccess'));
    loadData();
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    if (!isDialogDismissal(e)) ElMessage.error(t('colorCards.detail.message.deleteFailed'));
  }
};

const handleScanItem = async (item: ColorItemInfo) => {
  try {
    // v11 批次 173 P2-1 修复：const res: any 改为 as { data: ScanResult }
    const res = (await scanColorCode(item.color_code)) as { data: ScanResult };
    scanResult.value = res.data;
    showScanDialog.value = true;
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      t('colorCards.detail.message.scanFailed') +
        ': ' +
        (e instanceof Error ? e.message : String(e))
    );
  }
};

const handleBatchImport = async () => {
  // 与单条添加同源门控：批量端点同样落色号创建，非草稿态拒绝提交
  if (!isDraftCard.value) {
    ElMessage.warning(batchImportDisabledTip.value);
    return;
  }
  if (!importText.value.trim()) {
    ElMessage.warning(t('colorCards.detail.message.importEmpty'));
    return;
  }
  importing.value = true;
  try {
    const lines = importText.value.trim().split('\n');
    const items = lines.map(line => {
      const parts = line.split(',').map(s => s.trim());
      return {
        color_code: parts[0],
        color_name: parts[1],
        rgb_r: parseInt(parts[2] || '0'),
        rgb_g: parseInt(parts[3] || '0'),
        rgb_b: parseInt(parts[4] || '0'),
        hex_value: parts[5] || '#000000',
      };
    });
    // v11 批次 173 P2-1 修复：const res: any 改为 as 具体类型
    const res = (await batchImportItems(cardId.value, items)) as {
      data?: { success_count?: number; failed_count?: number };
    };
    ElMessage.success(
      t('colorCards.detail.message.importResult', {
        success: res.data?.success_count,
        failed: res.data?.failed_count,
      })
    );
    showImportDialog.value = false;
    importText.value = '';
    loadData();
  } finally {
    importing.value = false;
  }
};

const handleExport = () => {
  window.open(exportColorCardUrl(cardId.value), '_blank');
};

const formatDate = (s?: string) => (s ? new Date(s).toLocaleString('zh-CN') : '-');

watch(
  () => route.query.tab,
  val => {
    if (val) activeTab.value = String(val);
  }
);

// 页签与编辑意图都在导航上表达，编辑意图必须等整卡数据就绪后再判，
// 否则草稿态门控会拿还没取回的卡状态误判
watch(() => route.query.edit, applyEditIntent);

onMounted(async () => {
  await loadData();
  applyEditIntent();
});
</script>

<style scoped>
.color-card-detail {
  padding: 16px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.stock-actions {
  margin-top: 12px;
}
.card-info-panel {
  margin-top: 16px;
}
</style>
