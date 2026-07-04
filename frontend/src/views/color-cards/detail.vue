<template>
  <div class="color-card-detail">
    <el-page-header :icon="ArrowLeft" content="返回色卡列表" @back="$router.push('/color-cards/list')" />

    <el-card v-loading="loading" style="margin-top: 16px">
      <template #header>
        <div class="card-header">
          <div>
            <h2 style="margin: 0">{{ card?.card_name }}</h2>
            <div style="color: #909399; margin-top: 4px">
              <span>{{ card?.card_no }}</span>
              <el-divider direction="vertical" />
              <el-tag size="small" :type="(COLOR_CARD_STATUS_COLORS[card?.status || ''] as any)">
                {{ COLOR_CARD_STATUS[card?.status as keyof typeof COLOR_CARD_STATUS] || card?.status }}
              </el-tag>
            </div>
          </div>
          <div>
            <el-button :icon="Download" @click="handleExport">导出 CSV</el-button>
            <el-button type="primary" :icon="Plus" @click="showAddItemDialog = true">添加色号</el-button>
            <el-button :icon="Box" @click="showImportDialog = true">批量导入</el-button>
          </div>
        </div>
      </template>

      <el-tabs v-model="activeTab">
        <!-- 基本信息 -->
        <el-tab-pane label="基本信息" name="info">
          <el-descriptions :column="2" border>
            <el-descriptions-item label="色卡编号">{{ card?.card_no }}</el-descriptions-item>
            <el-descriptions-item label="色卡名称">{{ card?.card_name }}</el-descriptions-item>
            <el-descriptions-item label="色卡类型">{{ COLOR_CARD_TYPE_LABELS[card?.card_type || ''] || card?.card_type }}</el-descriptions-item>
            <el-descriptions-item label="季节">{{ card?.season || '-' }}</el-descriptions-item>
            <el-descriptions-item label="品牌">{{ card?.brand || '-' }}</el-descriptions-item>
            <el-descriptions-item label="色号总数">{{ card?.total_colors }}</el-descriptions-item>
            <el-descriptions-item label="状态">
              <el-tag :type="(COLOR_CARD_STATUS_COLORS[card?.status || ''] as any)">
                {{ COLOR_CARD_STATUS[card?.status as keyof typeof COLOR_CARD_STATUS] || card?.status }}
              </el-tag>
            </el-descriptions-item>
            <el-descriptions-item label="创建时间">{{ formatDate(card?.created_at) }}</el-descriptions-item>
            <el-descriptions-item label="描述" :span="2">{{ card?.description || '-' }}</el-descriptions-item>
          </el-descriptions>
        </el-tab-pane>

        <!-- 色号列表 -->
        <el-tab-pane :label="`色号列表 (${items.length})`" name="items">
          <ColorCardGrid :items="items" @delete="handleDeleteItem" @scan="handleScanItem" />
        </el-tab-pane>

        <!-- 借出历史 -->
        <el-tab-pane label="借出历史" name="borrow">
          <BorrowRecordTimeline :records="borrowRecords" />
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <!-- 添加色号对话框 -->
    <el-dialog v-model="showAddItemDialog" title="添加色号" width="640px">
      <ColorItemEditor v-model="newItem" />
      <template #footer>
        <el-button @click="showAddItemDialog = false">取消</el-button>
        <el-button type="primary" :loading="adding" @click="handleAddItem">添加</el-button>
      </template>
    </el-dialog>

    <!-- 批量导入对话框 -->
    <el-dialog v-model="showImportDialog" title="批量导入色号" width="600px">
      <el-alert type="info" :closable="false" style="margin-bottom: 16px">
        支持 CSV 格式，必填字段：color_code, color_name, rgb_r, rgb_g, rgb_b, hex_value
      </el-alert>
      <el-input v-model="importText" type="textarea" :rows="10" placeholder="一行一个色号，字段用逗号分隔" />
      <template #footer>
        <el-button @click="showImportDialog = false">取消</el-button>
        <el-button type="primary" :loading="importing" @click="handleBatchImport">导入</el-button>
      </template>
    </el-dialog>

    <!-- 扫码详情对话框 -->
    <el-dialog v-model="showScanDialog" title="色号详情" width="720px">
      <el-descriptions v-if="scanResult" :column="3" border>
        <el-descriptions-item label="色号编码">{{ scanResult.color_item?.color_code }}</el-descriptions-item>
        <el-descriptions-item label="色号名称">{{ scanResult.color_item?.color_name }}</el-descriptions-item>
        <el-descriptions-item label="所属色卡">{{ scanResult.color_card_name }}</el-descriptions-item>
        <el-descriptions-item label="HEX">
          <div :style="{ background: scanResult.color_item?.hex_value, width: '40px', height: '20px', display: 'inline-block', marginRight: '8px' }" />
          {{ scanResult.color_item?.hex_value }}
        </el-descriptions-item>
        <el-descriptions-item label="RGB">
          {{ scanResult.color_item?.rgb_r }}, {{ scanResult.color_item?.rgb_g }}, {{ scanResult.color_item?.rgb_b }}
        </el-descriptions-item>
        <el-descriptions-item label="CMYK">
          {{ scanResult.color_item?.cmyk_c }}, {{ scanResult.color_item?.cmyk_m }}, {{ scanResult.color_item?.cmyk_y }}, {{ scanResult.color_item?.cmyk_k }}
        </el-descriptions-item>
        <el-descriptions-item label="CIELab" :span="3">
          L={{ scanResult.color_item?.lab_l }}, a={{ scanResult.color_item?.lab_a }}, b={{ scanResult.color_item?.lab_b }}
        </el-descriptions-item>
        <el-descriptions-item v-if="scanResult.recipe_summary" label="关联配方" :span="2">
          {{ scanResult.recipe_summary.recipe_name }} ({{ scanResult.recipe_summary.color_no }})
        </el-descriptions-item>
        <el-descriptions-item v-if="scanResult.price_summary" label="色号价格">
          {{ scanResult.price_summary.currency }} {{ scanResult.price_summary.base_price }}
        </el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { ArrowLeft, Plus, Box, Download } from '@element-plus/icons-vue'
import {
  getColorCard,
  listBorrowRecords,
  createColorItem,
  deleteColorItem,
  batchImportItems,
  scanColorCode,
  exportColorCardUrl,
  COLOR_CARD_TYPE_LABELS,
  COLOR_CARD_STATUS,
  COLOR_CARD_STATUS_COLORS,
  type ColorCardDetail,
  type ColorItemInfo,
  type BorrowRecordInfo,
} from '@/api/color-card'
import ColorCardGrid from '@/components/ColorCardGrid.vue'
import ColorItemEditor from '@/components/ColorItemEditor.vue'
import BorrowRecordTimeline from '@/components/BorrowRecordTimeline.vue'

const route = useRoute()
const cardId = computed(() => Number(route.params.id))
const loading = ref(false)
const card = ref<ColorCardDetail | null>(null)
const items = ref<ColorItemInfo[]>([])
const borrowRecords = ref<BorrowRecordInfo[]>([])
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
const scanResult = ref<any>(null)

const loadData = async () => {
  loading.value = true
  try {
    const cardRes: any = await getColorCard(cardId.value)
    card.value = cardRes.data
    items.value = cardRes.data.items || []

    const recordsRes: any = await listBorrowRecords({ color_card_id: cardId.value, page_size: 50 })
    borrowRecords.value = recordsRes.data?.items || []
  } finally {
    loading.value = false
  }
}

const handleAddItem = async () => {
  if (!newItem.value.color_code || !newItem.value.color_name) {
    ElMessage.warning('请填写色号编码和名称')
    return
  }
  adding.value = true
  try {
    await createColorItem(cardId.value, newItem.value)
    ElMessage.success('色号添加成功')
    showAddItemDialog.value = false
    newItem.value = { color_code: '', color_name: '', rgb_r: 0, rgb_g: 0, rgb_b: 0, hex_value: '#000000' }
    loadData()
  } finally {
    adding.value = false
  }
}

const handleDeleteItem = async (item: ColorItemInfo) => {
  try {
    await ElMessageBox.confirm(`确认删除色号「${item.color_code}」？`, '提示', { type: 'warning' })
    await deleteColorItem(cardId.value, item.id)
    ElMessage.success('删除成功')
    loadData()
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    if (e !== 'cancel') ElMessage.error('删除失败')
  }
}

const handleScanItem = async (item: ColorItemInfo) => {
  try {
    const res: any = await scanColorCode(item.color_code)
    scanResult.value = res.data
    showScanDialog.value = true
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error('扫码查询失败: ' + (e instanceof Error ? e.message : String(e)))
  }
}

const handleBatchImport = async () => {
  if (!importText.value.trim()) {
    ElMessage.warning('请输入要导入的数据')
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
    const res: any = await batchImportItems(cardId.value, items)
    ElMessage.success(`导入完成：成功 ${res.data?.success_count} 条，失败 ${res.data?.failed_count} 条`)
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
