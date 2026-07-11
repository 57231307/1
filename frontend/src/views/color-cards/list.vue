<template>
  <div class="color-card-list">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>色卡列表</span>
          <div>
            <el-button type="primary" :icon="Plus" @click="$router.push('/color-cards/create')">
              新建色卡
            </el-button>
            <el-button :icon="Box" @click="$router.push('/color-cards/borrow')">
              借出管理
            </el-button>
          </div>
        </div>
      </template>

      <!-- 筛选 -->
      <el-form :inline="true" :model="filterForm" class="filter-form">
        <el-form-item label="色卡类型">
          <el-select v-model="filterForm.card_type" placeholder="全部" clearable style="width: 140px">
            <el-option v-for="(label, value) in COLOR_CARD_TYPE_LABELS" :key="value" :label="label" :value="value" />
          </el-select>
        </el-form-item>
        <el-form-item label="季节">
          <el-select v-model="filterForm.season" placeholder="全部" clearable style="width: 140px">
            <el-option v-for="(label, value) in SEASON_LABELS" :key="value" :label="label" :value="value" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="filterForm.status" placeholder="全部" clearable style="width: 120px">
            <el-option v-for="(label, value) in COLOR_CARD_STATUS" :key="value" :label="label" :value="value" />
          </el-select>
        </el-form-item>
        <el-form-item label="关键字">
          <el-input v-model="filterForm.keyword" placeholder="名称" clearable style="width: 180px" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :icon="Search" @click="handleSearch">查询</el-button>
          <el-button :icon="Refresh" @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>

      <!-- 列表 -->
      <el-table :data="tableData" v-loading="loading" border stripe>
        <el-table-column prop="card_no" label="色卡编号" width="180" />
        <el-table-column prop="card_name" label="色卡名称" min-width="200" />
        <el-table-column label="类型" width="100">
          <template #default="{ row }">
            {{ COLOR_CARD_TYPE_LABELS[row.card_type] || row.card_type }}
          </template>
        </el-table-column>
        <el-table-column label="季节" width="100">
          <template #default="{ row }">
            {{ row.season ? (SEASON_LABELS[row.season] || row.season) : '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="brand" label="品牌" width="100" />
        <el-table-column prop="total_colors" label="色号数" width="80" align="center" />
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="tagType(row.status)">
              {{ COLOR_CARD_STATUS[row.status as keyof typeof COLOR_CARD_STATUS] || row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="创建时间" width="180">
          <template #default="{ row }">{{ formatDate(row.created_at) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" @click="handleView(row)">详情</el-button>
            <el-button link type="primary" @click="handleEdit(row)">编辑</el-button>
            <el-button link type="danger" @click="handleArchive(row)" v-if="row.status === 'active'">归档</el-button>
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
          @size-change="handleSizeChange"
          @current-change="handleCurrentChange"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Search, Refresh, Box } from '@element-plus/icons-vue'
import {
  archiveColorCard,
  COLOR_CARD_TYPE_LABELS,
  COLOR_CARD_STATUS,
  COLOR_CARD_STATUS_COLORS,
  SEASON_LABELS,
  type ColorCardListItem,
} from '@/api/color-card'
import { useTableApi } from '@/composables/useTableApi'

const router = useRouter()

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
  onError: () => ElMessage.error('加载色卡列表失败'),
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
    await ElMessageBox.confirm(`确认归档色卡「${row.card_name}」？归档后不可再编辑。`, '提示', {
      type: 'warning',
    })
    await archiveColorCard(row.id)
    ElMessage.success('已归档')
    loadData()
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    if (e !== 'cancel') ElMessage.error('归档失败: ' + (e instanceof Error ? e.message : String(e)))
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
