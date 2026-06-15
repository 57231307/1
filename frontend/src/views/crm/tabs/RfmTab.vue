<!--
  RfmTab.vue - CRM 客户分级 (RFM) Tab
  来源：原 crm/index.vue 中 客户分级 (RFM) tab 内容
  拆分日期：2026-06-15 B3-3
-->
<template>
  <div class="rfm-tab">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">客户分级 (RFM)</h1>
      </div>
    </div>

    <div class="rfm-section">
      <el-row :gutter="20" class="mb-20">
        <el-col v-for="(count, level) in rfmDistribution" :key="level" :span="4">
          <el-card shadow="hover" class="rfm-card">
            <div class="rfm-card-content">
              <span class="rfm-card-level">{{ level }}</span>
              <span class="rfm-card-count">{{ count }} 人</span>
            </div>
          </el-card>
        </el-col>
      </el-row>

      <el-table v-loading="rfmLoading" :data="rfmCustomers" stripe>
        <el-table-column prop="customer_code" label="客户编码" width="120" />
        <el-table-column prop="customer_name" label="客户名称" min-width="180">
          <template #default="{ row }">
            <el-button type="primary" link @click="viewDetail(row.id)">{{
              row.customer_name
            }}</el-button>
          </template>
        </el-table-column>
        <el-table-column prop="owner_name" label="负责人" width="100" />
        <el-table-column prop="rfm_score.level" label="等级" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="getRfmLevelTag(row.rfm_score?.level)" size="small">
              {{ row.rfm_score?.level || '-' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="rfm_score.label" label="分级" width="100" />
        <el-table-column prop="rfm_score.recency" label="R" width="80" align="center" />
        <el-table-column prop="rfm_score.frequency" label="F" width="80" align="center" />
        <el-table-column prop="rfm_score.monetary" label="M" width="80" align="center" />
        <el-table-column prop="total_amount" label="累计金额" width="120" align="right">
          <template #default="{ row }">
            {{ row.total_amount ? formatCurrency(row.total_amount) : '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="total_orders" label="订单数" width="80" align="center" />
        <el-table-column label="操作" width="100" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewDetail(row.id)">详情</el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import crmEnhancedApi, { type CustomerWithTags } from '@/api/crm-enhanced'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'

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
    const res = await crmEnhancedApi.getCustomerList({ page: 1, page_size: 100 })
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
    const res = await crmEnhancedApi.getRfmDistribution()
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
