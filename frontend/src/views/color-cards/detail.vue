<template>
  <div class="color-card-detail">
    <el-page-header :icon="ArrowLeft" :content="$t('colorCards.detail.back')" @back="$router.push('/color-cards/list')" />

    <el-card v-loading="loading" style="margin-top: 16px">
      <template #header>
        <div class="card-header">
          <div>
            <h2 style="margin: 0">{{ card?.card_name }}</h2>
            <div style="color: #909399; margin-top: 4px">
              <span>{{ card?.card_no }}</span>
              <el-divider direction="vertical" />
              <el-tag size="small" :type="(COLOR_CARD_STATUS_COLORS[card?.status || ''] as TagType)">
                {{ getStatusLabel(card?.status || '') || card?.status }}
              </el-tag>
            </div>
          </div>
          <div>
            <el-button :icon="Download" @click="handleExport">{{ $t('colorCards.detail.exportExcel') }}</el-button>
            <el-button type="primary" :icon="Plus" @click="showAddItemDialog = true">{{ $t('colorCards.detail.addItem') }}</el-button>
            <el-button :icon="Box" @click="showImportDialog = true">{{ $t('colorCards.detail.batchImport') }}</el-button>
          </div>
        </div>
      </template>

      <el-tabs v-model="activeTab">
        <!-- 基本信息 -->
        <el-tab-pane :label="$t('colorCards.detail.tab.info')" name="info">
          <el-descriptions :column="2" border>
            <el-descriptions-item :label="$t('colorCards.detail.info.cardNo')">{{ card?.card_no }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.detail.info.cardName')">{{ card?.card_name }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.detail.info.cardType')">{{ getCardTypeLabel(card?.card_type || '') || card?.card_type }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.filter.season')">{{ card?.season || '-' }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.detail.info.brand')">{{ card?.brand || '-' }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.detail.info.totalColors')">{{ card?.total_colors }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.table.status')">
              <el-tag :type="(COLOR_CARD_STATUS_COLORS[card?.status || ''] as TagType)">
                {{ getStatusLabel(card?.status || '') || card?.status }}
              </el-tag>
            </el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.table.createdAt')">{{ formatDate(card?.created_at) }}</el-descriptions-item>
            <el-descriptions-item :label="$t('colorCards.detail.info.description')" :span="2">{{ card?.description || '-' }}</el-descriptions-item>
          </el-descriptions>
        </el-tab-pane>

        <!-- 色号列表 -->
        <el-tab-pane :label="$t('colorCards.detail.tab.items', { count: items.length })" name="items">
          <ColorCardGrid :items="items" @delete="handleDeleteItem" @scan="handleScanItem" />
        </el-tab-pane>

        <!-- 发放历史 -->
        <el-tab-pane :label="$t('colorCards.detail.tab.issue')" name="issue">
          <IssueRecordTimeline :records="issueRecords" />
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <!-- 添加色号对话框 -->
    <el-dialog v-model="showAddItemDialog" :title="$t('colorCards.detail.addItemDialog.title')" width="640px" :aria-label="$t('colorCards.detail.addItemDialog.ariaLabel')">
      <ColorItemEditor v-model="newItem" />
      <template #footer>
        <el-button @click="showAddItemDialog = false">{{ $t('colorCards.detail.addItemDialog.cancel') }}</el-button>
        <el-button type="primary" :loading="adding" @click="handleAddItem">{{ $t('colorCards.detail.addItemDialog.add') }}</el-button>
      </template>
    </el-dialog>

    <!-- 批量导入对话框 -->
    <el-dialog v-model="showImportDialog" :title="$t('colorCards.detail.importDialog.title')" width="600px" :aria-label="$t('colorCards.detail.importDialog.ariaLabel')">
      <el-alert type="info" :closable="false" style="margin-bottom: 16px">
        {{ $t('colorCards.detail.importDialog.alert') }}
      </el-alert>
      <el-input v-model="importText" type="textarea" :rows="10" :placeholder="$t('colorCards.detail.importDialog.placeholder')" />
      <template #footer>
        <el-button @click="showImportDialog = false">{{ $t('colorCards.detail.importDialog.cancel') }}</el-button>
        <el-button type="primary" :loading="importing" @click="handleBatchImport">{{ $t('colorCards.detail.importDialog.import') }}</el-button>
      </template>
    </el-dialog>

    <!-- 扫码详情对话框 -->
    <el-dialog v-model="showScanDialog" :title="$t('colorCards.detail.scanDialog.title')" width="720px" :aria-label="$t('colorCards.detail.scanDialog.ariaLabel')">
      <el-descriptions v-if="scanResult" :column="3" border>
        <el-descriptions-item :label="$t('colorCards.detail.scanDialog.colorCode')">{{ scanResult.color_item?.color_code }}</el-descriptions-item>
        <el-descriptions-item :label="$t('colorCards.detail.scanDialog.colorName')">{{ scanResult.color_item?.color_name }}</el-descriptions-item>
        <el-descriptions-item :label="$t('colorCards.detail.scanDialog.colorCard')">{{ scanResult.color_card_name }}</el-descriptions-item>
        <el-descriptions-item :label="$t('colorCards.detail.scanDialog.hex')">
          <div :style="{ background: scanResult.color_item?.hex_value, width: '40px', height: '20px', display: 'inline-block', marginRight: '8px' }" />
          {{ scanResult.color_item?.hex_value }}
        </el-descriptions-item>
        <el-descriptions-item :label="$t('colorCards.detail.scanDialog.rgb')">
          {{ scanResult.color_item?.rgb_r }}, {{ scanResult.color_item?.rgb_g }}, {{ scanResult.color_item?.rgb_b }}
        </el-descriptions-item>
        <el-descriptions-item :label="$t('colorCards.detail.scanDialog.cmyk')">
          {{ scanResult.color_item?.cmyk_c }}, {{ scanResult.color_item?.cmyk_m }}, {{ scanResult.color_item?.cmyk_y }}, {{ scanResult.color_item?.cmyk_k }}
        </el-descriptions-item>
        <el-descriptions-item :label="$t('colorCards.detail.scanDialog.cieLab')" :span="3">
          L={{ scanResult.color_item?.lab_l }}, a={{ scanResult.color_item?.lab_a }}, b={{ scanResult.color_item?.lab_b }}
        </el-descriptions-item>
        <el-descriptions-item v-if="scanResult.recipe_summary" :label="$t('colorCards.detail.scanDialog.recipe')" :span="2">
          {{ scanResult.recipe_summary.recipe_name }} ({{ scanResult.recipe_summary.color_no }})
        </el-descriptions-item>
        <el-descriptions-item v-if="scanResult.price_summary" :label="$t('colorCards.detail.scanDialog.price')">
          {{ scanResult.price_summary.currency }} {{ scanResult.price_summary.base_price }}
        </el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { ArrowLeft, Plus, Box, Download } from '@element-plus/icons-vue'
import {
  getColorCard,
  getIssueList,
  createColorItem,
  deleteColorItem,
  batchImportItems,
  scanColorCode,
  exportColorCardUrl,
  COLOR_CARD_STATUS_COLORS,
  type ColorCardDetail,
  type ColorItemInfo,
  type IssueRecordInfo,
} from '@/api/color-card'
import ColorCardGrid from '@/components/ColorCardGrid.vue'
import ColorItemEditor from '@/components/ColorItemEditor.vue'
import IssueRecordTimeline from '@/components/IssueRecordTimeline.vue'

const { t } = useI18n({ useScope: 'global' })

// 状态码 → 本地化标签（响应式：随语言切换自动更新）
const getCardTypeLabel = (key: string) => t(`colorCards.cardType.${key}`)
const getStatusLabel = (key: string) => t(`colorCards.cardStatus.${key}`)

// v11 批次 173 P2-1 修复：扫码结果接口类型，替代 any
interface ScanRecipeSummary {
  recipe_name: string
  color_no: string
}
interface ScanPriceSummary {
  currency: string
  base_price: number
}
interface ScanResult {
  color_item?: ColorItemInfo
  color_card_name?: string
  recipe_summary?: ScanRecipeSummary
  price_summary?: ScanPriceSummary
}

// v11 批次 173 P2-1 修复：el-tag type 类型
type TagType = 'success' | 'warning' | 'info' | 'primary' | 'danger'

const route = useRoute()
const cardId = computed(() => Number(route.params.id))
const loading = ref(false)
const card = ref<ColorCardDetail | null>(null)
const items = ref<ColorItemInfo[]>([])
const issueRecords = ref<IssueRecordInfo[]>([])
const activeTab = ref('info')

const showAddItemDialog = ref(false)
const adding = ref(false)
const newItem = ref<Partial<ColorItemInfo>>({
  color_code: '',
  color_name: '',
  rgb_r: 0,
  rgb_g: 0,
  rgb_b: 0,
  hex_value: '#000000',
})

const showImportDialog = ref(false)
const importing = ref(false)
const importText = ref('')

const showScanDialog = ref(false)
// v11 批次 173 P2-1 修复：ref<any>(null) 改为 ref<ScanResult | null>(null)
const scanResult = ref<ScanResult | null>(null)

const loadData = async () => {
  loading.value = true
  try {
    // v11 批次 173 P2-1 修复：const res: any 改为 as 具体类型
    const cardRes = (await getColorCard(cardId.value)) as { data: ColorCardDetail }
    card.value = cardRes.data
    items.value = cardRes.data.items || []

    const recordsRes = (await getIssueList({ color_card_id: cardId.value, page_size: 50 })) as {
      data?: { items?: IssueRecordInfo[] }
    }
    issueRecords.value = recordsRes.data?.items || []
  } finally {
    loading.value = false
  }
}

const handleAddItem = async () => {
  if (!newItem.value.color_code || !newItem.value.color_name) {
    ElMessage.warning(t('colorCards.detail.message.codeAndNameRequired'))
    return
  }
  adding.value = true
  try {
    await createColorItem(cardId.value, newItem.value)
    ElMessage.success(t('colorCards.detail.message.addItemSuccess'))
    showAddItemDialog.value = false
    newItem.value = { color_code: '', color_name: '', rgb_r: 0, rgb_g: 0, rgb_b: 0, hex_value: '#000000' }
    loadData()
  } finally {
    adding.value = false
  }
}

const handleDeleteItem = async (item: ColorItemInfo) => {
  try {
    await ElMessageBox.confirm(t('colorCards.detail.message.deleteItemConfirm', { code: item.color_code }), t('colorCards.detail.message.deleteItemConfirmTitle'), { type: 'warning' })
    await deleteColorItem(cardId.value, item.id)
    ElMessage.success(t('colorCards.detail.message.deleteSuccess'))
    loadData()
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    if (e !== 'cancel') ElMessage.error(t('colorCards.detail.message.deleteFailed'))
  }
}

const handleScanItem = async (item: ColorItemInfo) => {
  try {
    // v11 批次 173 P2-1 修复：const res: any 改为 as { data: ScanResult }
    const res = (await scanColorCode(item.color_code)) as { data: ScanResult }
    scanResult.value = res.data
    showScanDialog.value = true
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(t('colorCards.detail.message.scanFailed') + ': ' + (e instanceof Error ? e.message : String(e)))
  }
}

const handleBatchImport = async () => {
  if (!importText.value.trim()) {
    ElMessage.warning(t('colorCards.detail.message.importEmpty'))
    return
  }
  importing.value = true
  try {
    const lines = importText.value.trim().split('\n')
    const items = lines.map((line) => {
      const parts = line.split(',').map((s) => s.trim())
      return {
        color_code: parts[0],
        color_name: parts[1],
        rgb_r: parseInt(parts[2] || '0'),
        rgb_g: parseInt(parts[3] || '0'),
        rgb_b: parseInt(parts[4] || '0'),
        hex_value: parts[5] || '#000000',
      }
    })
    // v11 批次 173 P2-1 修复：const res: any 改为 as 具体类型
    const res = (await batchImportItems(cardId.value, items)) as {
      data?: { success_count?: number; failed_count?: number }
    }
    ElMessage.success(t('colorCards.detail.message.importResult', { success: res.data?.success_count, failed: res.data?.failed_count }))
    showImportDialog.value = false
    importText.value = ''
    loadData()
  } finally {
    importing.value = false
  }
}

const handleExport = () => {
  window.open(exportColorCardUrl(cardId.value), '_blank')
}

const formatDate = (s?: string) => (s ? new Date(s).toLocaleString('zh-CN') : '-')

watch(() => route.query.tab, (val) => {
  if (val) activeTab.value = String(val)
})

onMounted(loadData)
</script>

<style scoped>
.color-card-detail { padding: 16px; }
.card-header { display: flex; justify-content: space-between; align-items: center; }
</style>
