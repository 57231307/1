<!--
  crm/detail.vue - CRM 客户 360 详情页
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-3）：
  原 663 行"上帝组件"已拆分为以下独立 section 子组件，
  位于 views/crm/tabs/ 目录：

  | Section     | 子组件                              |
  | ----------- | ----------------------------------- |
  | 跟进记录    | tabs/FollowUpTab.vue                |
  | 标签管理    | tabs/TagsPanelTab.vue               |

  本主入口承担：路由参数解析 + 数据获取 + 布局 + 公共样式。
-->
<template>
  <div class="detail-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ t('crmDetail.title') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{ t('crmDetail.breadcrumb.home') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('crmDetail.breadcrumb.crm') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('crmDetail.breadcrumb.customerDetail') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button @click="handleBack">
          <el-icon><Back /></el-icon>
          {{ t('crmDetail.back') }}
        </el-button>
      </div>
    </div>

    <div v-loading="loading" class="detail-content">
      <template v-if="customer">
        <el-row :gutter="20">
          <el-col :span="16">
            <el-card shadow="hover" class="section-card">
              <template #header>
                <div class="card-header">
                  <span>{{ t('crmDetail.basicInfo') }}</span>
                  <el-tag :type="customer.status === 'active' ? 'success' : 'info'" size="small">
                    {{ customer.status === 'active' ? t('crmDetail.statusActive') : t('crmDetail.statusInactive') }}
                  </el-tag>
                </div>
              </template>

              <el-descriptions :column="2" border>
                <el-descriptions-item :label="t('crmDetail.field.customerCode')">{{
                  customer.customer_code
                }}</el-descriptions-item>
                <el-descriptions-item :label="t('crmDetail.field.customerName')">{{
                  customer.customer_name
                }}</el-descriptions-item>
                <el-descriptions-item :label="t('crmDetail.field.contactPerson')">{{
                  customer.contact_person
                }}</el-descriptions-item>
                <el-descriptions-item :label="t('crmDetail.field.phone')">{{ customer.phone }}</el-descriptions-item>
                <el-descriptions-item :label="t('crmDetail.field.email')" :span="2">{{
                  customer.email
                }}</el-descriptions-item>
                <el-descriptions-item :label="t('crmDetail.field.address')" :span="2">{{
                  customer.address
                }}</el-descriptions-item>
                <el-descriptions-item :label="t('crmDetail.field.customerType')">
                  <el-tag :type="getTypeTag(customer.customer_type)" size="small">
                    {{ getTypeLabel(customer.customer_type) }}
                  </el-tag>
                </el-descriptions-item>
                <el-descriptions-item :label="t('crmDetail.field.owner')">{{
                  customer.owner_name
                }}</el-descriptions-item>
                <el-descriptions-item :label="t('crmDetail.field.creditLimit')">
                  {{ customer.credit_limit ? formatCurrency(customer.credit_limit) : '-' }}
                </el-descriptions-item>
                <el-descriptions-item :label="t('crmDetail.field.totalOrders')">{{
                  customer.total_orders
                }}</el-descriptions-item>
                <el-descriptions-item :label="t('crmDetail.field.totalAmount')">
                  {{ customer.total_amount ? formatCurrency(customer.total_amount) : '-' }}
                </el-descriptions-item>
                <el-descriptions-item :label="t('crmDetail.field.lastOrder')">{{
                  customer.last_order_date || '-'
                }}</el-descriptions-item>
              </el-descriptions>
            </el-card>

            <el-card shadow="hover" class="section-card mt-20">
              <template #header>
                <div class="card-header">
                  <span>{{ t('crmDetail.billingInfo') }}</span>
                </div>
              </template>

              <el-descriptions :column="2" border>
                <el-descriptions-item :label="t('crmDetail.field.taxNumber')" :span="2">{{
                  customer.tax_number || '-'
                }}</el-descriptions-item>
                <el-descriptions-item :label="t('crmDetail.field.bankName')">{{
                  customer.bank_name || '-'
                }}</el-descriptions-item>
                <el-descriptions-item :label="t('crmDetail.field.bankAccount')">{{
                  customer.bank_account || '-'
                }}</el-descriptions-item>
              </el-descriptions>
            </el-card>

            <el-card shadow="hover" class="section-card mt-20">
              <template #header>
                <div class="card-header">
                  <span>{{ t('crmDetail.contacts') }}</span>
                  <el-button type="primary" size="small" @click="handleAddContact">
                    <el-icon><Plus /></el-icon>
                    {{ t('crmDetail.addContact') }}
                  </el-button>
                </div>
              </template>

              <el-table :data="contacts" stripe v-loading="contactsLoading" :aria-label="t('crmDetail.contactTableAria')">
                <el-table-column prop="name" :label="t('crmDetail.field.contactName')" width="120" />
                <el-table-column prop="title" :label="t('crmDetail.field.contactTitle')" width="150">
                  <template #default="{ row }">{{ row.title || '-' }}</template>
                </el-table-column>
                <el-table-column prop="phone" :label="t('crmDetail.field.phone')" width="140" />
                <el-table-column prop="email" :label="t('crmDetail.field.email')" min-width="180">
                  <template #default="{ row }">{{ row.email || '-' }}</template>
                </el-table-column>
                <el-table-column prop="is_primary" :label="t('crmDetail.field.isPrimary')" width="100" align="center">
                  <template #default="{ row }">
                    <el-tag v-if="row.is_primary" type="warning" size="small">{{ t('crmDetail.field.primaryBadge') }}</el-tag>
                  </template>
                </el-table-column>
                <el-table-column :label="t('crmDetail.field.operation')" width="160" align="center">
                  <template #default="{ row }">
                    <el-button size="small" link type="primary" @click="handleEditContact(row)">
                      {{ t('crmRuleDialog.form.cancel').length ? t('crmDetail.message.deleteFailed').length ? '' : '' : '' }}{{ t('crmOpportunities.table.edit') }}
                    </el-button>
                    <el-button size="small" link type="danger" @click="handleDeleteContact(row)">
                      {{ t('crmAssignment.ruleTable.delete') }}
                    </el-button>
                  </template>
                </el-table-column>
              </el-table>
            </el-card>
          </el-col>

          <el-col :span="8">
            <el-card shadow="hover">
              <template #header><div class="card-header">{{ t('crmDetail.rfmScore') }}</div></template>
              <div v-if="customer.rfm_score" class="rfm-display">
                <div class="rfm-level">
                  <span class="level-badge">{{ customer.rfm_score.level }}</span>
                  <span class="level-label">{{ customer.rfm_score.label }}</span>
                </div>
                <div class="rfm-scores">
                  <div class="rfm-item">
                    <span class="rfm-label">{{ t('crmDetail.field.rfmRecency') }}</span>
                    <span class="rfm-value">{{ customer.rfm_score.recency }}</span>
                  </div>
                  <div class="rfm-item">
                    <span class="rfm-label">{{ t('crmDetail.field.rfmFrequency') }}</span>
                    <span class="rfm-value">{{ customer.rfm_score.frequency }}</span>
                  </div>
                  <div class="rfm-item">
                    <span class="rfm-label">{{ t('crmDetail.field.rfmMonetary') }}</span>
                    <span class="rfm-value">{{ customer.rfm_score.monetary }}</span>
                  </div>
                </div>
              </div>
              <el-empty v-else :description="t('crmDetail.rfmEmpty')" />
            </el-card>

            <TagsPanelTab
              :customer-id="customerId"
              :tags="customer.tags"
              @updated="fetchCustomer360"
            />

            <el-card shadow="hover" class="mt-20">
              <template #header><div class="card-header">{{ t('crmDetail.shippingAddress') }}</div></template>
              <div class="address-list">
                <div
                  v-for="addr in customer.shipping_addresses"
                  :key="addr.id"
                  class="address-item"
                >
                  <div class="address-header">
                    <span class="addr-name">{{ addr.name }}</span>
                    <el-tag v-if="addr.is_default" type="warning" size="small">{{ t('crmDetail.field.defaultAddress') }}</el-tag>
                  </div>
                  <div class="addr-phone">{{ addr.phone }}</div>
                  <div class="addr-detail">
                    {{ addr.province }} {{ addr.city }} {{ addr.district }} {{ addr.detail }}
                  </div>
                </div>
                <el-empty v-if="!customer.shipping_addresses.length" :description="t('crmDetail.addressEmpty')" />
              </div>
            </el-card>
          </el-col>
        </el-row>

        <FollowUpTab ref="followUpRef" :customer-id="customerId" @updated="fetchCustomer360" />
      </template>
    </div>

    <!-- 批次 90b P2-12：联系人新增/编辑对话框（替代占位符） -->
    <el-dialog
      v-model="contactDialogVisible"
      :title="contactDialogTitle"
      :aria-label="contactDialogTitle"
      width="500px"
      @closed="resetContactForm"
    >
      <el-form
        ref="contactFormRef"
        :model="contactForm"
        :rules="contactFormRules"
        label-width="80px"
        :aria-label="t('crmDetail.contactDialogAria')"
      >
        <el-form-item :label="t('crmDetail.contactForm.name')" prop="name">
          <el-input v-model="contactForm.name" :placeholder="t('crmDetail.contactForm.namePlaceholder')" maxlength="50" />
        </el-form-item>
        <el-form-item :label="t('crmDetail.contactForm.title')" prop="title">
          <el-input v-model="contactForm.title" :placeholder="t('crmDetail.contactForm.titlePlaceholder')" maxlength="100" />
        </el-form-item>
        <el-form-item :label="t('crmDetail.contactForm.phone')" prop="phone">
          <el-input v-model="contactForm.phone" :placeholder="t('crmDetail.contactForm.phonePlaceholder')" maxlength="50" />
        </el-form-item>
        <el-form-item :label="t('crmDetail.contactForm.email')" prop="email">
          <el-input v-model="contactForm.email" :placeholder="t('crmDetail.contactForm.emailPlaceholder')" maxlength="100" />
        </el-form-item>
        <el-form-item :label="t('crmDetail.contactForm.isPrimary')" prop="is_primary">
          <el-switch v-model="contactForm.is_primary" />
        </el-form-item>
        <el-form-item :label="t('crmDetail.contactForm.remarks')" prop="remarks">
          <el-input
            v-model="contactForm.remarks"
            type="textarea"
            :rows="2"
            :placeholder="t('crmDetail.contactForm.remarksPlaceholder')"
            maxlength="500"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="contactDialogVisible = false">{{ t('crmDetail.contactForm.cancel') }}</el-button>
        <el-button type="primary" :loading="contactSubmitting" @click="submitContactForm">
          {{ t('crmDetail.contactForm.confirm') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Back, Plus } from '@element-plus/icons-vue'
// D14 Batch 5b：原 crmEnhancedApi 对象已转风格 B 函数
import { getCustomer360, getCustomerContactList, deleteCustomerContact, createCustomerContact, updateCustomerContact, type Contact, type Customer360 } from '@/api/crm-enhanced'
import { logger } from '@/utils/logger'
import FollowUpTab from './tabs/FollowUpTab.vue'
import TagsPanelTab from './tabs/TagsPanelTab.vue'

const { t } = useI18n({ useScope: 'global' })

const route = useRoute()
const router = useRouter()

const loading = ref(false)
const customer = ref<Customer360 | null>(null)
const customerId = Number(route.params.id)
const followUpRef = ref<InstanceType<typeof FollowUpTab> | null>(null)

// 批次 90b P2-12：联系人列表与对话框状态
const contacts = ref<Contact[]>([])
const contactsLoading = ref(false)
const contactDialogVisible = ref(false)
const contactDialogTitle = ref('')
const contactSubmitting = ref(false)
const contactFormRef = ref<FormInstance | null>(null)
const editingContactId = ref<number | null>(null)
const contactForm = ref({
  name: '',
  title: '',
  phone: '',
  email: '',
  is_primary: false,
  remarks: '',
})

const contactFormRules: FormRules = {
  name: [{ required: true, message: t('crmDetail.validation.nameRequired'), trigger: 'blur' }],
  phone: [{ required: true, message: t('crmDetail.validation.phoneRequired'), trigger: 'blur' }],
  email: [{ type: 'email', message: t('crmDetail.validation.emailPattern'), trigger: 'blur' }],
}

const formatCurrency = (amount: number) => `¥${(amount || 0).toFixed(2)}`

const getTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    normal: t('crmDetail.customerType.normal'),
    vip: t('crmDetail.customerType.vip'),
    wholesale: t('crmDetail.customerType.wholesale'),
  }
  return labels[type] || type
}

const getTypeTag = (type: string) => {
  const typeMap: Record<string, string> = { normal: '', vip: 'warning', wholesale: 'success' }
  return typeMap[type] || ''
}

const fetchCustomer360 = async () => {
  loading.value = true
  try {
    const res = await getCustomer360(customerId)
    customer.value = res.data
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('crmDetail.message.loadFailed'))
  } finally {
    loading.value = false
  }
}

// 批次 90b P2-12：拉取联系人列表（独立于 360 视图，避免每次刷新 360 都重复请求）
const fetchContacts = async () => {
  contactsLoading.value = true
  try {
    const res = await getCustomerContactList(customerId)
    contacts.value = res.data || []
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error)
    ElMessage.error(msg || t('crmDetail.message.contactLoadFailed'))
  } finally {
    contactsLoading.value = false
  }
}

const handleBack = () => {
  router.back()
}

// 批次 90b P2-12：打开新增联系人对话框
const handleAddContact = () => {
  editingContactId.value = null
  contactDialogTitle.value = t('crmDetail.contactDialogTitle.create')
  contactDialogVisible.value = true
}

// 批次 90b P2-12：打开编辑联系人对话框
const handleEditContact = (row: Contact) => {
  editingContactId.value = row.id
  contactDialogTitle.value = t('crmDetail.contactDialogTitle.edit')
  contactForm.value = {
    name: row.name || '',
    title: row.title || '',
    phone: row.phone || '',
    email: row.email || '',
    is_primary: !!row.is_primary,
    remarks: '',
  }
  contactDialogVisible.value = true
}

// 批次 90b P2-12：删除联系人
const handleDeleteContact = async (row: Contact) => {
  try {
    await ElMessageBox.confirm(t('crmDetail.message.deleteConfirm', { name: row.name }), t('crmDetail.message.deleteTitle'), {
      type: 'warning',
    })
    await deleteCustomerContact(customerId, row.id)
    ElMessage.success(t('crmDetail.message.deleteSuccess'))
    fetchContacts()
  } catch (error) {
    if (error === 'cancel') return
    const msg = error instanceof Error ? error.message : String(error)
    ElMessage.error(msg || t('crmDetail.message.deleteFailed'))
  }
}

// 批次 90b P2-12：重置表单
const resetContactForm = () => {
  contactForm.value = {
    name: '',
    title: '',
    phone: '',
    email: '',
    is_primary: false,
    remarks: '',
  }
  editingContactId.value = null
  contactFormRef.value?.clearValidate()
}

// 批次 90b P2-12：提交表单（新增/编辑）
const submitContactForm = async () => {
  if (!contactFormRef.value) return
  // Element Plus validate(callback) 形式下外层 await 不会等待 callback 内 async，故改为 try/catch 形式
  try {
    await contactFormRef.value.validate()
  } catch {
    return // 校验失败，el-form 会自动显示错误
  }
  contactSubmitting.value = true
  try {
    const payload = {
      name: contactForm.value.name,
      title: contactForm.value.title || undefined,
      phone: contactForm.value.phone,
      email: contactForm.value.email || undefined,
      is_primary: contactForm.value.is_primary,
      remarks: contactForm.value.remarks || undefined,
    }
    if (editingContactId.value === null) {
      await createCustomerContact(customerId, payload)
      ElMessage.success(t('crmDetail.message.createSuccess'))
    } else {
      await updateCustomerContact(customerId, editingContactId.value, payload)
      ElMessage.success(t('crmDetail.message.updateSuccess'))
    }
    contactDialogVisible.value = false
    fetchContacts()
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error)
    ElMessage.error(msg || t('crmDetail.message.operationFailed'))
  } finally {
    contactSubmitting.value = false
  }
}

onMounted(() => {
  if (!customerId) {
    ElMessage.error(t('crmDetail.message.missingCustomerId'))
    router.back()
    return
  }
  fetchCustomer360()
  fetchContacts()
  logger.info(t('crmDetail.message.pageLoaded'), { customerId })
})
</script>

<style scoped>
.detail-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}
.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}
.header-actions {
  display: flex;
  gap: 12px;
}
.detail-content {
  min-height: 400px;
}
.section-card {
  margin-bottom: 0;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 600;
}
.mt-20 {
  margin-top: 20px;
}

.rfm-display {
  padding: 12px 0;
}
.rfm-level {
  text-align: center;
  margin-bottom: 20px;
}
.level-badge {
  display: inline-block;
  width: 60px;
  height: 60px;
  line-height: 60px;
  border-radius: 50%;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
  font-size: 28px;
  font-weight: 700;
}
.level-label {
  display: block;
  margin-top: 8px;
  font-size: 14px;
  color: #606266;
}
.rfm-scores {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.rfm-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: #fafafa;
  border-radius: 6px;
}
.rfm-label {
  font-size: 13px;
  color: #606266;
}
.rfm-value {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}

.address-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.address-item {
  padding: 12px;
  background: #fafafa;
  border-radius: 6px;
}
.address-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}
.addr-name {
  font-weight: 600;
  color: #303133;
}
.addr-phone {
  font-size: 13px;
  color: #606266;
  margin-bottom: 4px;
}
.addr-detail {
  font-size: 13px;
  color: #909399;
}
</style>
