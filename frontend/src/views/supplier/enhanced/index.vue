<template>
  <div class="supplier-enhanced-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>{{ t('supplier.enhanced.title') }}</h2>
        <div class="header-actions">
          <el-input-number
            v-model="supplierId"
            :min="1"
            :precision="0"
            :placeholder="t('supplier.enhanced.idPlaceholder')"
            style="width: 180px"
          />
          <el-button type="primary" :loading="pageLoading" @click="loadAll">
            {{ t('supplier.enhanced.query') }}
          </el-button>
        </div>
      </div>

      <el-descriptions v-if="supplier" :column="4" border class="block-gap">
        <el-descriptions-item :label="t('supplier.enhanced.descriptions.code')">
          {{ supplier.supplier_code }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('supplier.enhanced.descriptions.name')">
          {{ supplier.supplier_name }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('supplier.enhanced.descriptions.grade')">
          {{ supplier.grade || '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('supplier.enhanced.descriptions.status')">
          <el-tag>{{ supplier.status }}</el-tag>
        </el-descriptions-item>
      </el-descriptions>

      <el-empty v-if="!loaded" :description="t('supplier.enhanced.emptyHint')" />

      <el-tabs v-else v-model="activeTab">
        <!-- 页签一：联系人 -->
        <el-tab-pane :label="t('supplier.enhanced.tabs.contacts')" name="contacts">
          <div class="toolbar">
            <el-button type="primary" @click="openContactDialog">
              {{ t('supplier.enhanced.contact.add') }}
            </el-button>
          </div>
          <el-table v-loading="contactsLoading" :data="contacts" border>
            <el-table-column
              prop="contact_name"
              :label="t('supplier.enhanced.contact.name')"
              min-width="100"
            />
            <el-table-column :label="t('supplier.enhanced.contact.department')" width="110">
              <template #default="{ row }">{{ row.department || '-' }}</template>
            </el-table-column>
            <el-table-column :label="t('supplier.enhanced.contact.position')" width="110">
              <template #default="{ row }">{{ row.position || '-' }}</template>
            </el-table-column>
            <el-table-column
              prop="mobile_phone"
              :label="t('supplier.enhanced.contact.mobile')"
              width="130"
            />
            <el-table-column :label="t('supplier.enhanced.contact.tel')" width="130">
              <template #default="{ row }">{{ row.tel_phone || '-' }}</template>
            </el-table-column>
            <el-table-column :label="t('supplier.enhanced.contact.email')" min-width="150">
              <template #default="{ row }">{{ row.email || '-' }}</template>
            </el-table-column>
            <el-table-column :label="t('supplier.enhanced.contact.wechat')" width="120">
              <template #default="{ row }">{{ row.wechat || '-' }}</template>
            </el-table-column>
            <el-table-column
              :label="t('supplier.enhanced.contact.primary')"
              width="110"
              align="center"
            >
              <template #default="{ row }">
                <el-tag v-if="row.is_primary" type="success" size="small">{{
                  t('common.yes')
                }}</el-tag>
                <span v-else>{{ t('common.no') }}</span>
              </template>
            </el-table-column>
            <el-table-column
              :label="t('supplier.enhanced.contact.remarks')"
              min-width="140"
              show-overflow-tooltip
            >
              <template #default="{ row }">{{ row.remarks || '-' }}</template>
            </el-table-column>
            <el-table-column :label="t('common.action')" width="120" fixed="right">
              <template #default="{ row }">
                <el-button link type="primary" size="small" @click="openEditContact(row)">
                  {{ t('common.edit') }}
                </el-button>
                <el-button link type="danger" size="small" @click="handleDeleteContact(row)">
                  {{ t('common.delete') }}
                </el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <!-- 页签二：资质 -->
        <el-tab-pane :label="t('supplier.enhanced.tabs.qualifications')" name="qualifications">
          <div class="toolbar">
            <el-button type="primary" @click="openQualificationDialog">
              {{ t('supplier.enhanced.qualification.add') }}
            </el-button>
          </div>
          <el-table v-loading="qualificationsLoading" :data="qualifications" border>
            <el-table-column
              prop="qualification_name"
              :label="t('supplier.enhanced.qualification.name')"
              min-width="140"
            />
            <el-table-column
              prop="qualification_type"
              :label="t('supplier.enhanced.qualification.type')"
              width="120"
            />
            <el-table-column
              prop="qualification_no"
              :label="t('supplier.enhanced.qualification.no')"
              min-width="140"
            />
            <el-table-column
              prop="issuing_authority"
              :label="t('supplier.enhanced.qualification.authority')"
              min-width="140"
            />
            <el-table-column
              prop="issue_date"
              :label="t('supplier.enhanced.qualification.issueDate')"
              width="110"
            />
            <el-table-column
              prop="valid_until"
              :label="t('supplier.enhanced.qualification.validUntil')"
              width="110"
            />
            <!-- 附件查看入口：经鉴权端点取回字节后本地预览（列表行 attachment_path 仅作有无判定） -->
            <el-table-column
              :label="t('supplier.enhanced.qualification.attachment')"
              width="110"
              align="center"
            >
              <template #default="{ row }">
                <el-button
                  v-if="row.attachment_path"
                  link
                  type="primary"
                  size="small"
                  @click="viewQualificationAttachment(row.supplier_id, row.id)"
                >
                  {{ t('supplier.enhanced.qualification.view') }}
                </el-button>
                <span v-else>{{ t('supplier.enhanced.qualification.notUploaded') }}</span>
              </template>
            </el-table-column>
            <el-table-column
              :label="t('supplier.enhanced.qualification.annualCheck')"
              width="90"
              align="center"
            >
              <template #default="{ row }">
                {{
                  row.need_annual_check
                    ? t('supplier.enhanced.qualification.annualNeed')
                    : t('supplier.enhanced.qualification.annualNoNeed')
                }}
              </template>
            </el-table-column>
            <!-- is_expired 真相源：后端写入时按 valid_until 落库、列表输出按 valid_until 重算，前端直接消费 -->
            <el-table-column
              :label="t('supplier.enhanced.qualification.expiredCol')"
              width="100"
              align="center"
            >
              <template #default="{ row }">
                <el-tag v-if="row.is_expired" type="danger" size="small">
                  {{ t('supplier.enhanced.qualification.expired') }}
                </el-tag>
                <el-tag v-else type="success" size="small">
                  {{ t('supplier.enhanced.qualification.valid') }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column :label="t('common.action')" width="120" fixed="right">
              <template #default="{ row }">
                <el-button link type="primary" size="small" @click="openEditQualification(row)">
                  {{ t('common.edit') }}
                </el-button>
                <el-button link type="danger" size="small" @click="handleDeleteQualification(row)">
                  {{ t('common.delete') }}
                </el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <!-- 页签三：采购历史与余额 -->
        <el-tab-pane :label="t('supplier.enhanced.tabs.history')" name="history">
          <h3 class="section-title">{{ t('supplier.enhanced.history.balanceTitle') }}</h3>
          <el-empty
            v-if="!balance"
            :description="t('supplier.enhanced.history.balanceEmpty')"
            :image-size="60"
          />
          <el-descriptions v-else :column="4" border>
            <el-descriptions-item :label="t('supplier.enhanced.history.totalAmount')">
              {{ fmtAmount(balance.total_amount) }}
            </el-descriptions-item>
            <el-descriptions-item :label="t('supplier.enhanced.history.paidAmount')">
              {{ fmtAmount(balance.paid_amount) }}
            </el-descriptions-item>
            <el-descriptions-item :label="t('supplier.enhanced.history.balance')">
              {{ fmtAmount(balance.balance) }}
            </el-descriptions-item>
            <el-descriptions-item :label="t('supplier.enhanced.history.orderCount')">
              {{ balance.order_count }}
            </el-descriptions-item>
          </el-descriptions>

          <h3 class="section-title">{{ t('supplier.enhanced.history.purchaseTitle') }}</h3>
          <el-table v-loading="historyLoading" :data="historyRows" border>
            <el-table-column
              prop="order_no"
              :label="t('supplier.enhanced.history.orderNo')"
              min-width="160"
            />
            <el-table-column
              prop="order_date"
              :label="t('supplier.enhanced.history.orderDate')"
              width="120"
            />
            <el-table-column
              :label="t('supplier.enhanced.history.amount')"
              width="140"
              align="right"
            >
              <template #default="{ row }">{{ fmtAmount(row.total_amount) }}</template>
            </el-table-column>
            <el-table-column prop="status" :label="t('common.status')" width="110" />
            <el-table-column
              prop="item_count"
              :label="t('supplier.enhanced.history.itemCount')"
              width="100"
              align="right"
            />
          </el-table>

          <h3 class="section-title">{{ t('supplier.enhanced.history.evalTitle') }}</h3>
          <div class="toolbar" style="margin-bottom: 8px">
            <el-input-number
              v-model="evalForm.score"
              :min="0"
              :max="100"
              :placeholder="t('supplier.enhanced.history.scorePlaceholder')"
            />
            <el-select
              v-model="evalForm.rating"
              style="width: 120px"
              :placeholder="t('supplier.enhanced.history.ratingPlaceholder')"
            >
              <el-option label="A" value="A" />
              <el-option label="B" value="B" />
              <el-option label="C" value="C" />
              <el-option label="D" value="D" />
            </el-select>
            <el-input
              v-model="evalForm.remark"
              :placeholder="t('supplier.enhanced.history.remarkPlaceholder')"
              style="width: 220px"
            />
            <el-button type="primary" :loading="evalSubmitting" @click="handleEvaluate">
              {{ t('supplier.enhanced.history.submitEval') }}
            </el-button>
            <el-button :loading="evalHistoryLoading" @click="loadEvaluationHistory">
              {{ t('supplier.enhanced.history.evalHistoryBtn') }}
            </el-button>
          </div>
          <el-table v-if="evaluationHistory.length" :data="evaluationHistory" border size="small">
            <el-table-column prop="id" label="ID" width="70" />
            <el-table-column
              prop="score"
              :label="t('supplier.enhanced.history.score')"
              width="90"
            />
            <el-table-column
              prop="rating"
              :label="t('supplier.enhanced.history.rating')"
              width="90"
            />
            <el-table-column
              prop="remark"
              :label="t('supplier.enhanced.contact.remarks')"
              min-width="160"
              show-overflow-tooltip
            >
              <template #default="{ row }">{{ row.remark || '-' }}</template>
            </el-table-column>
            <el-table-column
              prop="evaluated_at"
              :label="t('supplier.enhanced.history.evaluatedAt')"
              min-width="160"
            >
              <template #default="{ row }">{{
                row.evaluated_at || row.created_at || '-'
              }}</template>
            </el-table-column>
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <!-- 新增联系人弹窗 -->
    <el-dialog
      v-model="contactDialogVisible"
      :title="t('supplier.enhanced.contact.add')"
      width="520px"
      @close="resetContactForm"
    >
      <el-form ref="contactFormRef" :model="contactForm" :rules="contactRules" label-width="100px">
        <el-form-item :label="t('supplier.enhanced.contact.name')" prop="contact_name">
          <el-input
            v-model="contactForm.contact_name"
            :placeholder="t('supplier.enhanced.form.required')"
          />
        </el-form-item>
        <el-form-item :label="t('supplier.enhanced.contact.department')" prop="department">
          <el-input
            v-model="contactForm.department"
            :placeholder="t('supplier.enhanced.form.optional')"
          />
        </el-form-item>
        <el-form-item :label="t('supplier.enhanced.contact.position')" prop="position">
          <el-input
            v-model="contactForm.position"
            :placeholder="t('supplier.enhanced.form.optional')"
          />
        </el-form-item>
        <el-form-item :label="t('supplier.enhanced.contact.mobile')" prop="mobile_phone">
          <el-input
            v-model="contactForm.mobile_phone"
            :placeholder="t('supplier.enhanced.form.required')"
          />
        </el-form-item>
        <el-form-item :label="t('supplier.enhanced.contact.tel')" prop="tel_phone">
          <el-input
            v-model="contactForm.tel_phone"
            :placeholder="t('supplier.enhanced.form.optional')"
          />
        </el-form-item>
        <el-form-item :label="t('supplier.enhanced.contact.email')" prop="email">
          <el-input
            v-model="contactForm.email"
            :placeholder="t('supplier.enhanced.form.optional')"
          />
        </el-form-item>
        <el-form-item :label="t('supplier.enhanced.contact.wechat')" prop="wechat">
          <el-input
            v-model="contactForm.wechat"
            :placeholder="t('supplier.enhanced.form.optional')"
          />
        </el-form-item>
        <el-form-item label="QQ" prop="qq">
          <el-input v-model="contactForm.qq" :placeholder="t('supplier.enhanced.form.optional')" />
        </el-form-item>
        <el-form-item :label="t('supplier.enhanced.contact.primary')" prop="is_primary">
          <el-switch v-model="contactForm.is_primary" />
        </el-form-item>
        <el-form-item :label="t('supplier.enhanced.contact.remarks')" prop="remarks">
          <el-input v-model="contactForm.remarks" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="contactDialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="contactSubmitting" @click="submitContact">
          {{ t('supplier.enhanced.form.confirm') }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 新增资质弹窗 -->
    <el-dialog
      v-model="qualificationDialogVisible"
      :title="t('supplier.enhanced.qualification.add')"
      width="520px"
      @close="resetQualificationForm"
    >
      <el-form
        ref="qualificationFormRef"
        :model="qualificationForm"
        :rules="qualificationRules"
        label-width="110px"
      >
        <el-form-item :label="t('supplier.enhanced.qualification.name')" prop="qualification_name">
          <el-input
            v-model="qualificationForm.qualification_name"
            :placeholder="t('supplier.enhanced.form.required')"
          />
        </el-form-item>
        <el-form-item
          :label="t('supplier.enhanced.qualification.typeLabel')"
          prop="qualification_type"
        >
          <el-input
            v-model="qualificationForm.qualification_type"
            :placeholder="t('supplier.enhanced.form.required')"
          />
        </el-form-item>
        <el-form-item :label="t('supplier.enhanced.qualification.no')" prop="qualification_no">
          <el-input
            v-model="qualificationForm.qualification_no"
            :placeholder="t('supplier.enhanced.form.required')"
          />
        </el-form-item>
        <el-form-item
          :label="t('supplier.enhanced.qualification.authority')"
          prop="issuing_authority"
        >
          <el-input
            v-model="qualificationForm.issuing_authority"
            :placeholder="t('supplier.enhanced.form.required')"
          />
        </el-form-item>
        <el-form-item :label="t('supplier.enhanced.qualification.issueDate')" prop="issue_date">
          <el-date-picker
            v-model="qualificationForm.issue_date"
            type="date"
            value-format="YYYY-MM-DD"
            :placeholder="t('supplier.enhanced.form.required')"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('supplier.enhanced.qualification.validUntil')" prop="valid_until">
          <el-date-picker
            v-model="qualificationForm.valid_until"
            type="date"
            value-format="YYYY-MM-DD"
            :placeholder="t('supplier.enhanced.form.required')"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('supplier.enhanced.qualification.attachment')">
          <div class="qualification-attachment">
            <el-upload
              :auto-upload="false"
              :show-file-list="false"
              accept=".pdf,.jpg,.jpeg,.png"
              :disabled="!editingQualificationId"
              :on-change="handleQualificationAttachmentChange"
            >
              <el-button link type="primary" :disabled="!editingQualificationId">
                {{
                  pendingAttachment
                    ? pendingAttachment.name
                    : t('supplier.enhanced.qualification.uploadBtn')
                }}
              </el-button>
            </el-upload>
            <div class="attachment-tip">{{ t('supplier.enhanced.qualification.uploadTip') }}</div>
            <div v-if="!editingQualificationId" class="attachment-tip">
              {{ t('supplier.enhanced.qualification.uploadAfterSave') }}
            </div>
            <div v-if="editingQualificationId && qualificationForm.attachment_path">
              <el-button
                link
                type="primary"
                size="small"
                @click="viewQualificationAttachment(supplierId, editingQualificationId)"
              >
                {{ t('supplier.enhanced.qualification.view') }}
              </el-button>
            </div>
          </div>
        </el-form-item>
        <el-form-item
          :label="t('supplier.enhanced.qualification.needAnnualCheck')"
          prop="need_annual_check"
        >
          <el-switch v-model="qualificationForm.need_annual_check" />
        </el-form-item>
        <el-form-item
          :label="t('supplier.enhanced.qualification.annualCheckRecord')"
          prop="annual_check_record"
        >
          <el-input v-model="qualificationForm.annual_check_record" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="qualificationDialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="qualificationSubmitting" @click="submitQualification">
          {{ t('supplier.enhanced.form.confirm') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { isDialogDismissal, rethrowNonDismissal } from '@/utils/monitor';
import type { FormInstance, FormRules, UploadFile } from 'element-plus';
import {
  createSupplierContact,
  createSupplierQualification,
  updateSupplierContact,
  updateSupplierQualification,
  deleteSupplierContact,
  deleteSupplierQualification,
  evaluateSupplier,
  getSupplierEvaluationHistory,
  getSupplierBalance,
  getSupplierById,
  getSupplierContactList,
  getSupplierPurchaseHistory,
  getSupplierQualificationList,
  getSupplierQualificationAttachment,
  uploadSupplierQualificationAttachment,
  type PurchaseHistoryItem,
  type Supplier,
  type SupplierBalance,
  type SupplierContact,
  type SupplierContactInput,
  type SupplierQualification,
  type SupplierQualificationInput,
} from '@/api/supplier';

const { t } = useI18n({ useScope: 'global' });

/** 响应解包防御：兼容数组 / { items } / { history } 包装，避免 el-table "r is not iterable" */
const unwrapList = <T,>(payload: unknown): T[] => {
  if (Array.isArray(payload)) return payload as T[];
  const paged = payload as { items?: T[]; history?: T[] } | null;
  return paged?.items ?? paged?.history ?? [];
};

const fmtAmount = (value: number | null | undefined): string =>
  value == null ? '-' : Number(value).toLocaleString('zh-CN', { minimumFractionDigits: 2 });

// ============== 主查询 ==============

const supplierId = ref<number | undefined>(undefined);
const loaded = ref(false);
const pageLoading = ref(false);
const activeTab = ref('contacts');
const supplier = ref<Supplier | null>(null);
const contacts = ref<SupplierContact[]>([]);
const qualifications = ref<SupplierQualification[]>([]);
const balance = ref<SupplierBalance | null>(null);
const historyRows = ref<PurchaseHistoryItem[]>([]);

const contactsLoading = ref(false);
const qualificationsLoading = ref(false);
const historyLoading = ref(false);

const loadSupplierInfo = async () => {
  const id = supplierId.value;
  if (!id) return;
  try {
    const res = await getSupplierById(id);
    supplier.value = res.data ?? null;
  } catch {
    supplier.value = null;
    ElMessage.error(t('supplier.enhanced.message.loadSupplierFailed'));
  }
};

const loadContacts = async () => {
  const id = supplierId.value;
  if (!id) return;
  contactsLoading.value = true;
  try {
    const res = await getSupplierContactList(id);
    contacts.value = unwrapList<SupplierContact>(res.data);
  } catch {
    ElMessage.error(t('supplier.enhanced.message.loadContactsFailed'));
  } finally {
    contactsLoading.value = false;
  }
};

const loadQualifications = async () => {
  const id = supplierId.value;
  if (!id) return;
  qualificationsLoading.value = true;
  try {
    const res = await getSupplierQualificationList(id);
    qualifications.value = unwrapList<SupplierQualification>(res.data);
  } catch {
    ElMessage.error(t('supplier.enhanced.message.loadQualificationsFailed'));
  } finally {
    qualificationsLoading.value = false;
  }
};

const loadBalance = async () => {
  const id = supplierId.value;
  if (!id) return;
  try {
    const res = await getSupplierBalance(id);
    balance.value = res.data ?? null;
  } catch {
    balance.value = null;
    ElMessage.error(t('supplier.enhanced.message.loadBalanceFailed'));
  }
};

const loadHistory = async () => {
  const id = supplierId.value;
  if (!id) return;
  historyLoading.value = true;
  try {
    const res = await getSupplierPurchaseHistory(id, { limit: 50 });
    historyRows.value = unwrapList<PurchaseHistoryItem>(res.data);
  } catch {
    ElMessage.error(t('supplier.enhanced.message.loadHistoryFailed'));
  } finally {
    historyLoading.value = false;
  }
};

const loadAll = async () => {
  if (!supplierId.value) {
    ElMessage.warning(t('supplier.enhanced.message.inputSupplierId'));
    return;
  }
  loaded.value = true;
  pageLoading.value = true;
  const tasks = [
    loadSupplierInfo(),
    loadContacts(),
    loadQualifications(),
    loadBalance(),
    loadHistory(),
  ];
  await Promise.all(tasks);
  pageLoading.value = false;
};

// ============== 新增/编辑联系人 ==============

const contactDialogVisible = ref(false);
const contactSubmitting = ref(false);
const contactFormRef = ref<FormInstance>();
const editingContactId = ref<number | null>(null);

const contactForm = reactive<SupplierContactInput>({
  contact_name: '',
  department: '',
  position: '',
  mobile_phone: '',
  tel_phone: '',
  email: '',
  wechat: '',
  qq: '',
  is_primary: false,
  remarks: '',
});

const contactRules = computed<FormRules>(() => ({
  contact_name: [
    { required: true, message: t('supplier.enhanced.contact.validationName'), trigger: 'blur' },
  ],
  mobile_phone: [
    { required: true, message: t('supplier.enhanced.contact.validationMobile'), trigger: 'blur' },
  ],
}));

// 资质弹窗校验规则（模板 :rules="qualificationRules" 引用，缺失会导致弹窗声明崩溃）
const qualificationRules = computed<FormRules>(() => ({
  qualification_name: [
    {
      required: true,
      message: t('supplier.enhanced.qualification.validation.name'),
      trigger: 'blur',
    },
  ],
  qualification_type: [
    {
      required: true,
      message: t('supplier.enhanced.qualification.validation.type'),
      trigger: 'blur',
    },
  ],
  qualification_no: [
    {
      required: true,
      message: t('supplier.enhanced.qualification.validation.no'),
      trigger: 'blur',
    },
  ],
  issuing_authority: [
    {
      required: true,
      message: t('supplier.enhanced.qualification.validation.authority'),
      trigger: 'blur',
    },
  ],
  issue_date: [
    {
      required: true,
      message: t('supplier.enhanced.qualification.validation.issueDate'),
      trigger: 'change',
    },
  ],
  valid_until: [
    {
      required: true,
      message: t('supplier.enhanced.qualification.validation.validUntil'),
      trigger: 'change',
    },
    // 与后端 supplier_service::check_qualification_dates 同源的前置拦截：
    // value-format=YYYY-MM-DD，字典序即日期序，有效期至不得早于发证日期。
    {
      validator: (_rule, value, callback) => {
        if (value && qualificationForm.issue_date && value < qualificationForm.issue_date) {
          callback(new Error(t('supplier.enhanced.qualification.validation.dateOrder')));
        } else {
          callback();
        }
      },
      trigger: 'change',
    },
  ],
}));

const resetContactForm = () => {
  contactForm.contact_name = '';
  contactForm.department = '';
  contactForm.position = '';
  contactForm.mobile_phone = '';
  contactForm.tel_phone = '';
  contactForm.email = '';
  contactForm.wechat = '';
  contactForm.qq = '';
  contactForm.is_primary = false;
  contactForm.remarks = '';
  contactFormRef.value?.resetFields();
};

const openContactDialog = () => {
  resetContactForm();
  editingContactId.value = null;
  contactDialogVisible.value = true;
};

const openEditContact = (row: SupplierContact) => {
  editingContactId.value = row.id;
  Object.assign(contactForm, {
    contact_name: row.contact_name,
    department: row.department || '',
    position: row.position || '',
    mobile_phone: row.mobile_phone,
    tel_phone: row.tel_phone || '',
    email: row.email || '',
    wechat: row.wechat || '',
    qq: row.qq || '',
    is_primary: row.is_primary,
    remarks: row.remarks || '',
  });
  contactDialogVisible.value = true;
};

const handleDeleteContact = async (row: SupplierContact) => {
  const id = supplierId.value;
  if (!id) return;
  try {
    await ElMessageBox.confirm(
      t('supplier.enhanced.contact.deleteConfirm', { name: row.contact_name }),
      t('common.confirm'),
      {
        type: 'warning',
      }
    );
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('supplierEnhanced.handleDeleteContact', error);
  }
  try {
    await deleteSupplierContact(id, row.id);
    ElMessage.success(t('supplier.enhanced.message.contactDeleteSuccess'));
    await loadContacts();
  } catch {
    ElMessage.error(t('supplier.enhanced.message.contactDeleteFailed'));
  }
};

const submitContact = async () => {
  const id = supplierId.value;
  if (!id) {
    ElMessage.warning(t('supplier.enhanced.message.queryFirst'));
    return;
  }
  if (!contactFormRef.value) return;
  await contactFormRef.value.validate(async valid => {
    if (!valid) return;
    contactSubmitting.value = true;
    try {
      // 空则省略该键（条件展开）：后端 Option 字段收到 Some("") 会被 validator 判 422
      const payload: SupplierContactInput = {
        contact_name: contactForm.contact_name,
        ...(contactForm.department ? { department: contactForm.department } : {}),
        ...(contactForm.position ? { position: contactForm.position } : {}),
        mobile_phone: contactForm.mobile_phone,
        ...(contactForm.tel_phone ? { tel_phone: contactForm.tel_phone } : {}),
        ...(contactForm.email ? { email: contactForm.email } : {}),
        ...(contactForm.wechat ? { wechat: contactForm.wechat } : {}),
        ...(contactForm.qq ? { qq: contactForm.qq } : {}),
        is_primary: contactForm.is_primary,
        ...(contactForm.remarks ? { remarks: contactForm.remarks } : {}),
      };
      if (editingContactId.value) {
        await updateSupplierContact(id, editingContactId.value, payload);
        ElMessage.success(t('supplier.enhanced.message.contactUpdateSuccess'));
      } else {
        await createSupplierContact(id, payload);
        ElMessage.success(t('supplier.enhanced.message.contactCreateSuccess'));
      }
      contactDialogVisible.value = false;
      await loadContacts();
    } catch {
      ElMessage.error(t('supplier.enhanced.message.contactSaveFailed'));
    } finally {
      contactSubmitting.value = false;
    }
  });
};

// ============== 新增/编辑资质 ==============
const openQualificationDialog = () => {
  resetQualificationForm();
  editingQualificationId.value = null;
  qualificationDialogVisible.value = true;
};

const openEditQualification = (row: SupplierQualification) => {
  editingQualificationId.value = row.id;
  pendingAttachment.value = null;
  Object.assign(qualificationForm, {
    qualification_name: row.qualification_name,
    qualification_type: row.qualification_type,
    qualification_no: row.qualification_no,
    issuing_authority: row.issuing_authority,
    issue_date: row.issue_date,
    valid_until: row.valid_until,
    attachment_path: row.attachment_path || '',
    need_annual_check: row.need_annual_check,
    annual_check_record: row.annual_check_record || '',
  });
  qualificationDialogVisible.value = true;
};

const handleDeleteQualification = async (row: SupplierQualification) => {
  const id = supplierId.value;
  if (!id) return;
  try {
    await ElMessageBox.confirm(
      t('supplier.enhanced.qualification.deleteConfirm', { name: row.qualification_name }),
      t('common.confirm'),
      { type: 'warning' }
    );
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('supplierEnhanced.handleDeleteQualification', error);
  }
  try {
    await deleteSupplierQualification(id, row.id);
    ElMessage.success(t('supplier.enhanced.message.qualDeleteSuccess'));
    await loadQualifications();
  } catch {
    ElMessage.error(t('supplier.enhanced.message.qualDeleteFailed'));
  }
};

const resetQualificationForm = () => {
  qualificationForm.qualification_name = '';
  qualificationForm.qualification_type = '';
  qualificationForm.qualification_no = '';
  qualificationForm.issuing_authority = '';
  qualificationForm.issue_date = '';
  qualificationForm.valid_until = '';
  qualificationForm.attachment_path = '';
  qualificationForm.need_annual_check = false;
  qualificationForm.annual_check_record = '';
  pendingAttachment.value = null;
  qualificationFormRef.value?.resetFields();
};

// 资质对话框状态（此前被引用但未声明，打开即抛 ReferenceError，此处补齐修复）
const qualificationDialogVisible = ref(false);
const qualificationSubmitting = ref(false);
const qualificationFormRef = ref<FormInstance>();
const editingQualificationId = ref<number | null>(null);
const qualificationForm = reactive<SupplierQualificationInput>({
  qualification_name: '',
  qualification_type: '',
  qualification_no: '',
  issuing_authority: '',
  issue_date: '',
  valid_until: '',
  attachment_path: '',
  need_annual_check: false,
  annual_check_record: '',
});

const submitQualification = async () => {
  const id = supplierId.value;
  if (!id) {
    ElMessage.warning(t('supplier.enhanced.message.queryFirst'));
    return;
  }
  if (!qualificationFormRef.value) return;
  await qualificationFormRef.value.validate(async valid => {
    if (!valid) return;
    qualificationSubmitting.value = true;
    try {
      // 空则省略该键（条件展开）：annual_check_record 为后端 Option 字段，
      // 提交 Some("") 会触发校验失败 422，未填写时不携带。
      // attachment_path 不再由手敲文本提交：编辑时回传既有受控 URL 防 PUT 清空，
      // 新附件走专用上传端点（保存成功后真实上传）。
      const payload: SupplierQualificationInput = {
        qualification_name: qualificationForm.qualification_name,
        qualification_type: qualificationForm.qualification_type,
        qualification_no: qualificationForm.qualification_no,
        issuing_authority: qualificationForm.issuing_authority,
        issue_date: qualificationForm.issue_date,
        valid_until: qualificationForm.valid_until,
        ...(qualificationForm.attachment_path
          ? { attachment_path: qualificationForm.attachment_path }
          : {}),
        need_annual_check: qualificationForm.need_annual_check,
        ...(qualificationForm.annual_check_record
          ? { annual_check_record: qualificationForm.annual_check_record }
          : {}),
      };
      let savedQualificationId: number | null = null;
      if (editingQualificationId.value) {
        await updateSupplierQualification(id, editingQualificationId.value, payload);
        savedQualificationId = editingQualificationId.value;
        ElMessage.success(t('supplier.enhanced.message.qualUpdateSuccess'));
      } else {
        const res = await createSupplierQualification(id, payload);
        savedQualificationId = res.data.id;
        ElMessage.success(t('supplier.enhanced.message.qualCreateSuccess'));
      }
      qualificationDialogVisible.value = false;
      await loadQualifications();
      // 待上传附件（本地暂存，随保存动作真实提交到新端点）；
      // 失败原因外显后端真实 message（见 uploadQualificationAttachment），不吞错
      const pending = pendingAttachment.value;
      pendingAttachment.value = null;
      if (pending && savedQualificationId !== null) {
        const uploaded = await uploadQualificationAttachment(id, savedQualificationId, pending);
        if (!uploaded) {
          ElMessage.warning(t('supplier.enhanced.message.qualSavedButAttachmentFailed'));
          await loadQualifications();
        }
      }
    } catch {
      // 拦截器已按后端失败信封外显真实 message（business_displayable 时为原文），
      // 此处再给一条归属明确的兜底文案，指明失败发生在资质保存环节
      ElMessage.error(t('supplier.enhanced.message.qualSaveFailed'));
    } finally {
      qualificationSubmitting.value = false;
    }
  });
};

// ============== 资质附件上传/查看（真上传，非手敲路径） ==============

/** 与后端 supplier_handler.rs 白名单/大小上限同源同值（双端校验） */
const ATTACHMENT_ALLOWED_EXTS = ['pdf', 'jpg', 'jpeg', 'png'];
const ATTACHMENT_MAX_SIZE_BYTES = 5 * 1024 * 1024;

/** 本地待上传文件（点「确定」保存资质成功后真实提交；仅编辑态可选，新增态 disabled） */
const pendingAttachment = ref<File | null>(null);

/**
 * 从失败响应提取后端真实 message：
 * - JSON 错误信封（上传失败）直接取 data.message；
 * - blob 响应（附件读取 responseType:'blob'）需先解文本再取 message。
 * 提取不到才回退 Error.message，保证失败原因不静默。
 */
const extractBackendErrorReason = async (error: unknown): Promise<string> => {
  const data = (error as { response?: { data?: unknown } })?.response?.data;
  let message: unknown;
  if (data instanceof Blob) {
    try {
      message = JSON.parse(await data.text())?.message;
    } catch {
      message = undefined;
    }
  } else if (data && typeof data === 'object') {
    message = (data as { message?: unknown }).message;
  }
  if (typeof message === 'string' && message.trim() !== '') return message;
  return error instanceof Error ? error.message : String(error);
};

/** el-upload on-change（auto-upload=false）：本地校验后缀/大小并暂存，提交发生在资质保存后 */
const handleQualificationAttachmentChange = (uploadFile: UploadFile) => {
  const raw = uploadFile.raw;
  if (!raw) return;
  const ext = raw.name.includes('.') ? (raw.name.split('.').pop()?.toLowerCase() ?? '') : '';
  if (!ATTACHMENT_ALLOWED_EXTS.includes(ext)) {
    pendingAttachment.value = null;
    ElMessage.error(t('supplier.enhanced.message.attachmentTypeRejected'));
    return;
  }
  if (raw.size > ATTACHMENT_MAX_SIZE_BYTES) {
    pendingAttachment.value = null;
    ElMessage.error(t('supplier.enhanced.message.attachmentSizeRejected'));
    return;
  }
  pendingAttachment.value = raw;
};

/** 真实调用上传端点；成功/失败均给用户可见反馈（失败=后端真实 message） */
const uploadQualificationAttachment = async (
  supplierId: number,
  qualificationId: number,
  file: File
): Promise<boolean> => {
  try {
    await uploadSupplierQualificationAttachment(supplierId, qualificationId, file);
    ElMessage.success(t('supplier.enhanced.message.attachmentUploadSuccess'));
    return true;
  } catch (error: unknown) {
    const reason = await extractBackendErrorReason(error);
    ElMessage.error(t('supplier.enhanced.message.attachmentUploadFailed', { reason }));
    return false;
  }
};

/** 经鉴权端点取回附件字节并本地预览（blob + object URL；失败外显后端真实 message） */
const viewQualificationAttachment = async (supplier?: number, qualificationId?: number | null) => {
  if (!supplier || !qualificationId) {
    ElMessage.warning(t('supplier.enhanced.message.queryFirst'));
    return;
  }
  try {
    const blob = await getSupplierQualificationAttachment(supplier, qualificationId);
    const url = URL.createObjectURL(blob);
    window.open(url, '_blank', 'noopener');
    setTimeout(() => URL.revokeObjectURL(url), 60_000);
  } catch (error: unknown) {
    const reason = await extractBackendErrorReason(error);
    ElMessage.error(t('supplier.enhanced.message.attachmentViewFailed', { reason }));
  }
};

// ============== 供应商评估 ==============
const evalForm = reactive({ score: 80, rating: 'A', remark: '' });
const evalSubmitting = ref(false);
const evalHistoryLoading = ref(false);
const evaluationHistory = ref<Array<Record<string, unknown>>>([]);

const handleEvaluate = async () => {
  const id = supplierId.value;
  if (!id) {
    ElMessage.warning(t('supplier.enhanced.message.queryFirst'));
    return;
  }
  evalSubmitting.value = true;
  try {
    await evaluateSupplier(id, {
      score: evalForm.score,
      rating: evalForm.rating,
      // 空则省略该键（条件展开范式）
      ...(evalForm.remark ? { remark: evalForm.remark } : {}),
    });
    ElMessage.success(t('supplier.enhanced.message.evalSubmitted'));
    await loadEvaluationHistory();
  } catch {
    ElMessage.error(t('supplier.enhanced.message.evalFailed'));
  } finally {
    evalSubmitting.value = false;
  }
};

const loadEvaluationHistory = async () => {
  const id = supplierId.value;
  if (!id) {
    ElMessage.warning(t('supplier.enhanced.message.queryFirst'));
    return;
  }
  evalHistoryLoading.value = true;
  try {
    const res = await getSupplierEvaluationHistory(id);
    evaluationHistory.value = unwrapList(res.data);
  } catch {
    ElMessage.error(t('supplier.enhanced.message.evalHistoryFailed'));
  } finally {
    evalHistoryLoading.value = false;
  }
};
</script>

<style scoped>
.supplier-enhanced-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.page-header h2 {
  margin: 0;
  font-size: 18px;
}

.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}

.section-title {
  margin: 20px 0 12px;
  font-size: 15px;
  font-weight: 600;
}

.toolbar {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 12px;
}

.block-gap {
  margin-bottom: 16px;
}
</style>
