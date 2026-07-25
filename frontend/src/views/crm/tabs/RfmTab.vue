<!--
  RfmTab.vue - CRM 客户分级 (RFM) Tab
  来源：原 crm/index.vue 中 客户分级 (RFM) tab 内容
-->
<template>
  <div class="rfm-tab">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ t('crmRfm.title') }}</h1>
      </div>
    </div>

    <div class="rfm-section">
      <el-row :gutter="20" class="mb-20">
        <el-col v-for="(count, level) in rfmDistribution" :key="level" :span="4">
          <el-card shadow="hover" class="rfm-card">
            <div class="rfm-card-content">
              <span class="rfm-card-level">{{ level }}</span>
              <span class="rfm-card-count">{{ count }} {{ t('crmRfm.countUnit') }}</span>
            </div>
          </el-card>
        </el-col>
      </el-row>

      <el-table v-loading="rfmLoading" :data="rfmCustomers" stripe :aria-label="t('crmRfm.table.ariaLabel')">
        <el-table-column prop="customer_code" :label="t('crmRfm.table.customerCode')" width="120" />
        <el-table-column prop="customer_name" :label="t('crmRfm.table.customerName')" min-width="180">
          <template #default="{ row }">
            <el-button type="primary" link @click="viewDetail(row.id)">{{
              row.customer_name
            }}</el-button>
          </template>
        </el-table-column>
        <el-table-column prop="owner_name" :label="t('crmRfm.table.owner')" width="100" />
        <el-table-column prop="rfm_score.level" :label="t('crmRfm.table.level')" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="getRfmLevelTag(row.rfm_score?.level)" size="small">
              {{ row.rfm_score?.level || '-' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="rfm_score.label" :label="t('crmRfm.table.label')" width="100" />
        <el-table-column prop="rfm_score.recency" :label="t('crmRfm.table.recency')" width="80" align="center" />
        <el-table-column prop="rfm_score.frequency" :label="t('crmRfm.table.frequency')" width="80" align="center" />
        <el-table-column prop="rfm_score.monetary" :label="t('crmRfm.table.monetary')" width="80" align="center" />
        <el-table-column prop="total_amount" :label="t('crmRfm.table.totalAmount')" width="120" align="right">
          <template #default="{ row }">
            {{ row.total_amount ? formatCurrency(row.total_amount) : '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="total_orders" :label="t('crmRfm.table.totalOrders')" width="80" align="center" />
        <el-table-column :label="t('crmRfm.table.operation')" width="100" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewDetail(row.id)">{{ t('crmRfm.table.detail') }}</el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
// D14 Batch 5b：原 crmEnhancedApi 对象已转风格 B 函数
import { getCustomerList, getCustomerRfmDistribution, type CustomerWithTags } from '@/api/crm-enhanced'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'

const { t } = useI18n({ useScope: 'global' })

const hasLoaded = createLazyLoader()

const router = useRouter()
const rfmLoading = ref(false)
const rfmCustomers = ref<CustomerWithTags[]>([])
const rfmDistribution = ref<Record<string, number>>({})

const formatCurrency = (amount: number) => `¥${(amount || 0).toFixed(2)}`

const getRfmLevelTag = (level: string) => {
  const tagMap: Record<string, string> = {
    A: 'success',
    B: 'primary',
    C: 'warning',
    D: 'info',
    E: 'danger',
  }
  return tagMap[level] || ''
}

const fetchRfmCustomers = async () => {
  rfmLoading.value = true
  try {
    const res = await getCustomerList({ page: 1, page_size: 100 })
    rfmCustomers.value = res.data?.list || []
    fetchRfmDistribution()
  } catch (error) {
    rfmCustomers.value = []
  } finally {
    rfmLoading.value = false
  }
}

const fetchRfmDistribution = async () => {
  try {
    const res = await getCustomerRfmDistribution()
    rfmDistribution.value = res.data || {}
  } catch (error) {
    rfmDistribution.value = {}
  }
}

const viewDetail = (id: number) => {
  router.push(`/crm/detail/${id}`)
}

onMounted(() => {
  loadIfNot('fetchRfmCustomers', fetchRfmCustomers, hasLoaded)
})
</script>
