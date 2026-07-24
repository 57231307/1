<template>
  <div class="color-card-list">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>{{ $t('colorCards.list.title') }}</span>
          <div>
            <el-button type="primary" :icon="Plus" @click="$router.push('/color-cards/create')">
              {{ $t('colorCards.list.create') }}
            </el-button>
            <el-button :icon="Box" @click="$router.push('/color-cards/issues')">
              {{ $t('colorCards.list.issueManagement') }}
            </el-button>
          </div>
        </div>
      </template>

      <!-- 筛选 -->
      <el-form :inline="true" :model="filterForm" class="filter-form" :aria-label="$t('colorCards.filter.ariaLabel')">
        <el-form-item :label="$t('colorCards.filter.cardType')">
          <el-select v-model="filterForm.card_type" :placeholder="$t('colorCards.filter.all')" clearable style="width: 140px">
            <el-option v-for="value in cardTypeKeys" :key="value" :label="getCardTypeLabel(value)" :value="value" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('colorCards.filter.season')">
          <el-select v-model="filterForm.season" :placeholder="$t('colorCards.filter.all')" clearable style="width: 140px">
            <el-option v-for="value in seasonKeys" :key="value" :label="getSeasonLabel(value)" :value="value" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('colorCards.filter.status')">
          <el-select v-model="filterForm.status" :placeholder="$t('colorCards.filter.all')" clearable style="width: 120px">
            <el-option v-for="value in statusKeys" :key="value" :label="getStatusLabel(value)" :value="value" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('colorCards.filter.keyword')">
          <el-input v-model="filterForm.keyword" :placeholder="$t('colorCards.filter.keywordPlaceholder')" clearable style="width: 180px" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :icon="Search" @click="handleSearch">{{ $t('colorCards.filter.query') }}</el-button>
          <el-button :icon="Refresh" @click="handleReset">{{ $t('colorCards.filter.reset') }}</el-button>
        </el-form-item>
      </el-form>

      <!-- 列表 -->
      <el-table :data="tableData" v-loading="loading" border stripe :aria-label="$t('colorCards.table.ariaLabel')">
        <el-table-column prop="card_no" :label="$t('colorCards.table.cardNo')" width="180" />
        <el-table-column prop="card_name" :label="$t('colorCards.table.cardName')" min-width="200" />
        <el-table-column :label="$t('colorCards.table.type')" width="100">
          <template #default="{ row }">
            {{ getCardTypeLabel(row.card_type) || row.card_type }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('colorCards.filter.season')" width="100">
          <template #default="{ row }">
            {{ row.season ? (getSeasonLabel(row.season) || row.season) : '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="brand" :label="$t('colorCards.table.brand')" width="100" />
        <el-table-column prop="total_colors" :label="$t('colorCards.table.totalColors')" width="80" align="center" />
        <el-table-column :label="$t('colorCards.table.status')" width="100">
          <template #default="{ row }">
            <el-tag :type="tagType(row.status)">
              {{ getStatusLabel(row.status) || row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" :label="$t('colorCards.table.createdAt')" width="180">
          <template #default="{ row }">{{ formatDate(row.created_at) }}</template>
        </el-table-column>
        <el-table-column :label="$t('colorCards.table.operation')" width="200" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" @click="handleView(row)">{{ $t('colorCards.table.detail') }}</el-button>
            <el-button link type="primary" @click="handleEdit(row)">{{ $t('colorCards.table.edit') }}</el-button>
            <el-button link type="danger" @click="handleArchive(row)" v-if="row.status === 'active'">{{ $t('colorCards.table.archive') }}</el-button>
          </template>
        </el-table-column>
      </el-table>

      <!-- 分页 -->
      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :total="total"
          :page-sizes="[10, 20, 50, 100]"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="$t('colorCards.table.paginationAriaLabel')"
          @size-change="handleSizeChange"
          @current-change="handleCurrentChange"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Search, Refresh, Box } from '@element-plus/icons-vue'
import {
  archiveColorCard,
  COLOR_CARD_TYPE,
  COLOR_CARD_STATUS,
  COLOR_CARD_STATUS_COLORS,
  SEASON_LABELS,
  type ColorCardListItem,
} from '@/api/color-card'
import { useTableApi } from '@/composables/useTableApi'

const { t } = useI18n({ useScope: 'global' })

const router = useRouter()

// 枚举键（响应式：随语言切换自动更新标签）
const cardTypeKeys = Object.keys(COLOR_CARD_TYPE)
const statusKeys = Object.keys(COLOR_CARD_STATUS)
const seasonKeys = Object.keys(SEASON_LABELS)

// 状态码 → 本地化标签（响应式）
const getCardTypeLabel = (key: string) => t(`colorCards.cardType.${key}`)
const getSeasonLabel = (key: string) => t(`colorCards.season.${key}`)
const getStatusLabel = (key: string) => t(`colorCards.cardStatus.${key}`)

// 批次 274：接入 useTableApi，消除手写 tableData/total/loading/filterForm.page/page_size + loadData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: tableData,
  loading,
  page,
  pageSize,
  total,
  refresh: loadData,
  setQueryParam,
} = useTableApi<ColorCardListItem>({
  url: '/color-cards',
  listKey: 'items',
  onError: () => ElMessage.error(t('colorCards.message.loadListFailed')),
})

const filterForm = reactive({
  card_type: '',
  season: '',
  status: '',
  keyword: '',
})

// 批次 274：同步筛选条件到 useTableApi.queryParams 并刷新
// useTableApi 自动 watch page/pageSize 变化触发重载，无需手动 loadData
const syncQueryParams = () => {
  setQueryParam('card_type', filterForm.card_type || undefined)
  setQueryParam('season', filterForm.season || undefined)
  setQueryParam('status', filterForm.status || undefined)
  setQueryParam('keyword', filterForm.keyword || undefined)
}

const handleSearch = () => {
  syncQueryParams()
  page.value = 1
  loadData()
}

const handleReset = () => {
  filterForm.card_type = ''
  filterForm.season = ''
  filterForm.status = ''
  filterForm.keyword = ''
  syncQueryParams()
  page.value = 1
  loadData()
}

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handleSizeChange = (s: number) => {
  pageSize.value = s
  page.value = 1
}

const handleCurrentChange = (p: number) => {
  page.value = p
}

const handleView = (row: ColorCardListItem) => {
  router.push(`/color-cards/detail/${row.id}`)
}

const handleEdit = (row: ColorCardListItem) => {
  router.push(`/color-cards/detail/${row.id}?edit=1`)
}

const handleArchive = async (row: ColorCardListItem) => {
  try {
    await ElMessageBox.confirm(t('colorCards.message.archiveConfirm', { name: row.card_name }), t('colorCards.message.archiveConfirmTitle'), {
      type: 'warning',
    })
    await archiveColorCard(row.id)
    ElMessage.success(t('colorCards.message.archiveSuccess'))
    loadData()
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    if (e !== 'cancel') ElMessage.error(t('colorCards.message.archiveFailed') + ': ' + (e instanceof Error ? e.message : String(e)))
  }
}

const formatDate = (s: string) => {
  if (!s) return '-'
  return new Date(s).toLocaleString('zh-CN')
}

/** el-tag 类型联合（与 element-plus TagProps.type 对齐） */
type TagType = '' | 'success' | 'warning' | 'info' | 'danger'

/** 根据色卡状态返回对应的 el-tag 类型 */
const tagType = (status: string): TagType =>
  (COLOR_CARD_STATUS_COLORS[status] || '') as TagType

// 批次 274：useTableApi 构造时自动初始加载，无需 onMounted 调用 loadData
</script>

<style scoped>
.color-card-list { padding: 16px; }
.card-header { display: flex; justify-content: space-between; align-items: center; }
.filter-form { margin-bottom: 16px; }
.pagination-wrapper { margin-top: 16px; text-align: right; }
</style>
