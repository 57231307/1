<template>
  <div class="print-targets-page">
    <div class="page-header">
      <h2 class="page-title">{{ t('printTargets.title') }}</h2>
      <div class="header-actions">
        <el-button @click="fetchList">
          <el-icon><Refresh /></el-icon>
          {{ t('common.refresh') }}
        </el-button>
        <el-button
          v-permission="PERMISSIONS.PRINT_TARGET_CREATE"
          type="primary"
          @click="openDialog()"
        >
          <el-icon><Plus /></el-icon>
          {{ t('printTargets.create') }}
        </el-button>
      </div>
    </div>

    <el-card v-if="androidClient" class="bluetooth-card" shadow="hover">
      <template #header>
        <div class="bluetooth-header">
          <span class="bluetooth-title">{{ t('printTargets.bluetooth.title') }}</span>
          <el-tag :type="btConnectedAddress ? 'success' : 'info'" size="small">
            {{
              btConnectedAddress
                ? t('printTargets.bluetooth.connected')
                : t('printTargets.bluetooth.notConnected')
            }}
          </el-tag>
        </div>
      </template>
      <p class="bluetooth-description">{{ t('printTargets.bluetooth.description') }}</p>
      <div class="bluetooth-actions">
        <el-button :disabled="btLoading" @click="handleBluetoothSettings">
          <el-icon><Setting /></el-icon>
          {{ t('printTargets.bluetooth.openSettings') }}
        </el-button>
        <el-button :loading="btLoading" @click="refreshBluetoothState">
          <el-icon><Refresh /></el-icon>
          {{ t('printTargets.bluetooth.refresh') }}
        </el-button>
      </div>

      <el-alert
        v-if="btState && !btState.supported"
        :title="t('printTargets.bluetooth.adapterUnsupported')"
        type="error"
        :closable="false"
        show-icon
      />
      <el-alert
        v-else-if="btState && !btState.enabled"
        :title="t('printTargets.bluetooth.adapterDisabled')"
        type="warning"
        :closable="false"
        show-icon
      />
      <el-alert
        v-else-if="btState && btState.printers.length === 0"
        :title="t('printTargets.bluetooth.noPrinters')"
        type="info"
        :closable="false"
        show-icon
      />

      <el-form class="bluetooth-form" label-width="auto">
        <el-form-item :label="t('printTargets.bluetooth.printer')">
          <el-select
            v-model="btSelectedAddress"
            :placeholder="t('printTargets.bluetooth.printerPlaceholder')"
            style="width: 100%"
          >
            <el-option
              v-for="printer in btState?.printers || []"
              :key="printer.address"
              :label="printer.name ? `${printer.name} (${printer.address})` : printer.address"
              :value="printer.address"
            />
          </el-select>
        </el-form-item>
      </el-form>

      <div class="bluetooth-actions">
        <el-button type="primary" :loading="btLoading" @click="handleBluetoothConnect">
          {{ t('printTargets.bluetooth.connect') }}
        </el-button>
        <el-button :disabled="!btConnectedAddress || btLoading" @click="handleBluetoothDisconnect">
          {{ t('printTargets.bluetooth.disconnect') }}
        </el-button>
      </div>
    </el-card>

    <el-card v-if="androidClient" class="routing-card" shadow="hover">
      <template #header>
        <div class="bluetooth-header">
          <span class="bluetooth-title">{{ t('printTargets.routing.title') }}</span>
        </div>
      </template>
      <p class="bluetooth-description">{{ t('printTargets.routing.description') }}</p>
      <el-table :data="routeRows" size="small" :aria-label="t('printTargets.routing.ariaLabel')">
        <el-table-column :label="t('printTargets.routing.group')" width="90">
          <template #default="{ row }">{{ row.groupLabel }}</template>
        </el-table-column>
        <el-table-column :label="t('printTargets.routing.type')" min-width="130">
          <template #default="{ row }">{{ row.typeLabel }}</template>
        </el-table-column>
        <el-table-column :label="t('printTargets.routing.printer')" min-width="220">
          <template #default="{ row }">
            <el-select
              :model-value="row.address"
              :placeholder="t('printTargets.routing.followDefault')"
              clearable
              style="width: 100%"
              @change="handleRouteChange(row, $event)"
            >
              <el-option
                v-for="printer in btState?.printers || []"
                :key="printer.address"
                :label="printer.name ? `${printer.name} (${printer.address})` : printer.address"
                :value="printer.address"
              />
            </el-select>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-card v-if="androidClient" class="paper-card" shadow="hover">
      <template #header>
        <div class="bluetooth-header">
          <span class="bluetooth-title">{{ t('printTargets.paper.title') }}</span>
        </div>
      </template>
      <p class="bluetooth-description">{{ t('printTargets.paper.description') }}</p>
      <div class="paper-section">
        <h4 class="paper-subtitle">{{ t('printTargets.paper.documentTitle') }}</h4>
        <el-form label-width="auto">
          <el-form-item :label="t('printTargets.paper.width')">
            <el-input-number
              v-model="documentPaper.widthMm"
              :min="20"
              :max="1000"
              :controls="false"
              class="paper-number"
            />
            <span class="paper-unit">mm</span>
          </el-form-item>
          <el-form-item :label="t('printTargets.paper.height')">
            <el-input-number
              v-model="documentPaper.heightMm"
              :min="0"
              :max="1000"
              :controls="false"
              class="paper-number"
            />
            <span class="paper-unit">mm</span>
            <span class="paper-hint">{{ t('printTargets.paper.heightHint') }}</span>
          </el-form-item>
          <el-form-item :label="t('printTargets.paper.orientation')">
            <el-radio-group v-model="documentPaper.orientation">
              <el-radio-button value="portrait">
                {{ t('printTargets.paper.portrait') }}
              </el-radio-button>
              <el-radio-button value="landscape">
                {{ t('printTargets.paper.landscape') }}
              </el-radio-button>
            </el-radio-group>
          </el-form-item>
        </el-form>
      </div>
      <div class="paper-section">
        <h4 class="paper-subtitle">{{ t('printTargets.paper.labelTitle') }}</h4>
        <el-form label-width="auto">
          <el-form-item :label="t('printTargets.paper.width')">
            <el-input-number
              v-model="labelPaper.widthMm"
              :min="20"
              :max="300"
              :controls="false"
              class="paper-number"
            />
            <span class="paper-unit">mm</span>
          </el-form-item>
          <el-form-item :label="t('printTargets.paper.height')">
            <el-input-number
              v-model="labelPaper.heightMm"
              :min="10"
              :max="300"
              :controls="false"
              class="paper-number"
            />
            <span class="paper-unit">mm</span>
          </el-form-item>
          <el-form-item :label="t('printTargets.paper.orientation')">
            <el-radio-group v-model="labelPaper.orientation">
              <el-radio-button value="portrait">
                {{ t('printTargets.paper.portrait') }}
              </el-radio-button>
              <el-radio-button value="landscape">
                {{ t('printTargets.paper.landscape') }}
              </el-radio-button>
            </el-radio-group>
          </el-form-item>
          <el-form-item :label="t('printTargets.paper.labelFields')">
            <div class="paper-fields">
              <el-checkbox-group v-model="labelPaper.fields">
                <el-checkbox v-for="field in LABEL_FIELD_KEYS" :key="field" :value="field">
                  {{ t(LABEL_FIELD_TEXT[field]) }}
                </el-checkbox>
              </el-checkbox-group>
              <span class="paper-hint">{{ t('printTargets.paper.labelFieldsHint') }}</span>
            </div>
          </el-form-item>
        </el-form>
      </div>
    </el-card>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :data="list"
        stripe
        :aria-label="t('printTargets.table.ariaLabel')"
      >
        <el-table-column
          prop="target_code"
          :label="t('printTargets.table.targetCode')"
          width="160"
        />
        <el-table-column
          prop="display_name"
          :label="t('printTargets.table.displayName')"
          min-width="160"
        />
        <el-table-column prop="kind" :label="t('printTargets.table.kind')" width="120">
          <template #default="{ row }">{{ getKindLabel(row.kind) }}</template>
        </el-table-column>
        <el-table-column
          prop="is_pool"
          :label="t('printTargets.table.isPool')"
          width="90"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.is_pool ? 'warning' : 'info'" size="small">
              {{ row.is_pool ? t('printTargets.yesNo.yes') : t('printTargets.yesNo.no') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="default_copies"
          :label="t('printTargets.table.defaultCopies')"
          width="90"
          align="center"
        />
        <el-table-column
          prop="enabled"
          :label="t('printTargets.table.enabled')"
          width="90"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.enabled ? 'success' : 'info'" size="small">
              {{
                row.enabled ? t('printTargets.status.enabled') : t('printTargets.status.disabled')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" :label="t('printTargets.table.createdAt')" width="200" />
        <el-table-column prop="updated_at" :label="t('printTargets.table.updatedAt')" width="200" />
        <el-table-column :label="t('printTargets.table.operation')" width="140" fixed="right">
          <template #default="{ row }">
            <el-button
              v-permission="PERMISSIONS.PRINT_TARGET_UPDATE"
              type="primary"
              link
              size="small"
              @click="openDialog(row)"
              >{{ t('printTargets.table.edit') }}</el-button
            >
            <el-button
              v-permission="PERMISSIONS.PRINT_TARGET_DELETE"
              type="danger"
              link
              size="small"
              :loading="deletingId === row.id"
              :disabled="deletingId !== null"
              @click="handleDelete(row)"
              >{{ t('printTargets.table.delete') }}</el-button
            >
          </template>
        </el-table-column>
        <template #empty>
          <span v-if="!loading">{{ t('printTargets.emptyText') }}</span>
        </template>
      </el-table>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="form.id ? t('printTargets.dialog.editTitle') : t('printTargets.dialog.createTitle')"
      width="640px"
      :aria-label="t('printTargets.dialog.ariaLabel')"
    >
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="120px"
        :aria-label="t('printTargets.dialog.formAriaLabel')"
      >
        <el-form-item :label="t('printTargets.dialog.targetCode')" prop="target_code">
          <el-input
            v-model="form.target_code"
            :placeholder="t('printTargets.dialog.targetCodePlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="t('printTargets.dialog.displayName')" prop="display_name">
          <el-input
            v-model="form.display_name"
            :placeholder="t('printTargets.dialog.displayNamePlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="t('printTargets.dialog.kind')" prop="kind">
          <el-select
            v-model="form.kind"
            :placeholder="t('printTargets.dialog.kindPlaceholder')"
            style="width: 100%"
          >
            <el-option :label="t('printTargets.kind.cups')" value="cups" />
            <el-option :label="t('printTargets.kind.ipp')" value="ipp" />
            <el-option :label="t('printTargets.kind.network')" value="network" />
            <el-option :label="t('printTargets.kind.system_command')" value="system_command" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('printTargets.dialog.isPool')">
          <el-switch v-model="form.is_pool" />
        </el-form-item>

        <el-form-item
          v-if="form.is_pool"
          :label="t('printTargets.dialog.poolMembers')"
          prop="pool_members"
        >
          <el-select
            v-model="form.pool_members"
            multiple
            :placeholder="t('printTargets.dialog.poolMembersPlaceholder')"
            style="width: 100%"
          >
            <el-option
              v-for="opt in poolMemberOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>

        <template v-if="!form.is_pool">
          <el-form-item
            v-if="form.kind === 'cups'"
            :label="t('printTargets.dialog.queueName')"
            prop="queue_name"
          >
            <el-input
              v-model="form.queue_name"
              :placeholder="t('printTargets.dialog.queueNamePlaceholder')"
            />
          </el-form-item>
          <el-form-item
            v-if="form.kind === 'network'"
            :label="t('printTargets.dialog.host')"
            prop="host"
          >
            <el-input v-model="form.host" :placeholder="t('printTargets.dialog.hostPlaceholder')" />
          </el-form-item>
          <el-form-item
            v-if="form.kind === 'network'"
            :label="t('printTargets.dialog.port')"
            prop="port"
          >
            <!-- 端口同样不用步进键，范围与必填由本项的表单级校验给 -->
            <el-input-number
              v-model="form.port"
              :min="1"
              :max="65535"
              :controls="false"
              :placeholder="t('printTargets.dialog.portPlaceholder')"
            />
          </el-form-item>
          <el-form-item
            v-if="form.kind === 'ipp'"
            :label="t('printTargets.dialog.endpointUrl')"
            prop="endpoint_url"
          >
            <el-input
              v-model="form.endpoint_url"
              :placeholder="t('printTargets.dialog.endpointUrlPlaceholder')"
            />
          </el-form-item>
          <el-form-item
            v-if="form.kind === 'system_command'"
            :label="t('printTargets.dialog.commandTemplate')"
            prop="command_template"
          >
            <el-input
              v-model="form.command_template"
              :placeholder="t('printTargets.dialog.commandTemplatePlaceholder')"
            />
          </el-form-item>
        </template>

        <el-form-item :label="t('printTargets.dialog.defaultCopies')" prop="default_copies">
          <!-- 步进按钮在竖屏触控区过小且误触即改值，改为无步进键的数值输入加表单级范围校验 -->
          <el-input-number v-model="form.default_copies" :min="1" :max="99" :controls="false" />
        </el-form-item>
        <el-form-item :label="t('printTargets.dialog.enabled')">
          <el-switch v-model="form.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { Plus, Refresh, Setting } from '@element-plus/icons-vue';
import { isDialogDismissal } from '@/utils/monitor';
import { msg } from '@/utils/message';
import { logger } from '@/utils/logger';
import { PERMISSIONS } from '@/constants/permissions';
import {
  DOCUMENT_PRINT_TYPES,
  LABEL_PRINT_TYPES,
  connectBluetoothPrinter,
  disconnectBluetoothPrinter,
  documentPrintRoute,
  isAndroidClient,
  labelPrintRoute,
  listBluetoothPrinters,
  loadPrintRoutes,
  loadPrinterPreference,
  openBluetoothSettings,
  requestBluetoothPermission,
  savePrintRoute,
  savePrinterPreference,
  type BluetoothAdapterState,
  type PrintRouteKey,
} from '@/utils/bluetooth-print';
import {
  LABEL_FIELD_KEYS,
  loadDocumentPaper,
  loadLabelPaper,
  saveDocumentPaper,
  saveLabelPaper,
  type LabelFieldKey,
  type LabelPaperSettings,
  type PaperSettings,
} from '@/utils/print-settings';
import {
  getPrintTargetList,
  createPrintTarget,
  updatePrintTarget,
  deletePrintTarget,
  type PrintTarget,
  type PrintTargetKind,
  type CreatePrintTargetRequest,
  type UpdatePrintTargetRequest,
} from '@/api/print-targets';

const { t } = useI18n({ useScope: 'global' });

// 表单模型：连接字段用空串占位供输入框绑定，提交时按当前 kind/池配置只带有效字段（空串一律不提交）
interface TargetForm {
  id?: number;
  target_code: string;
  display_name: string;
  kind: PrintTargetKind;
  queue_name: string;
  host: string;
  port?: number;
  endpoint_url: string;
  command_template: string;
  is_pool: boolean;
  pool_members: string[];
  default_copies: number;
  enabled: boolean;
}

const emptyForm = (): TargetForm => ({
  id: undefined,
  target_code: '',
  display_name: '',
  kind: 'cups',
  queue_name: '',
  host: '',
  port: undefined,
  endpoint_url: '',
  command_template: '',
  is_pool: false,
  pool_members: [],
  default_copies: 1,
  enabled: true,
});

const list = ref<PrintTarget[]>([]);
// 初始为 true：首帧即挂遮罩，配合 #empty 门控，避免请求返回前闪现空态
const loading = ref(true);
const dialogVisible = ref(false);
const submitLoading = ref(false);
// 行级删除在途：确认框弹出与网络请求期间锁定该行，避免连点重复发起 DELETE
const deletingId = ref<number | null>(null);
const formRef = ref<FormInstance>();
const form = reactive<TargetForm>(emptyForm());
const original = ref<PrintTarget | null>(null);

async function fetchList(): Promise<void> {
  loading.value = true;
  try {
    const res = await getPrintTargetList();
    list.value = res.data;
    logger.info('[print-targets] list loaded, count=%d', res.data.length);
  } catch (error: unknown) {
    logger.error('[print-targets] list load failed', error);
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('printTargets.message.loadFailed')
    );
  } finally {
    loading.value = false;
  }
}

const getKindLabel = (kind: PrintTargetKind): string => {
  const map: Record<PrintTargetKind, string> = {
    cups: t('printTargets.kind.cups'),
    ipp: t('printTargets.kind.ipp'),
    network: t('printTargets.kind.network'),
    system_command: t('printTargets.kind.system_command'),
  };
  return map[kind];
};

// 池成员候选：仅取现有非池目标的 target_code
const poolMemberOptions = computed(() =>
  list.value
    .filter(item => !item.is_pool)
    .map(item => ({ label: `${item.display_name} (${item.target_code})`, value: item.target_code }))
);

// 切换为池时清空连接字段；切换 kind 时清掉与当前 kind 无关的连接字段，避免提交陈旧值
watch(
  () => form.is_pool,
  isPool => {
    if (isPool) {
      form.queue_name = '';
      form.host = '';
      form.port = undefined;
      form.endpoint_url = '';
      form.command_template = '';
    } else {
      form.pool_members = [];
    }
  }
);
watch(
  () => form.kind,
  () => {
    if (form.is_pool) return;
    if (form.kind !== 'cups') form.queue_name = '';
    if (form.kind !== 'network') {
      form.host = '';
      form.port = undefined;
    }
    if (form.kind !== 'ipp') form.endpoint_url = '';
    if (form.kind !== 'system_command') form.command_template = '';
  }
);

const rules = computed<FormRules>(() => {
  const base: FormRules = {
    target_code: [
      { required: true, message: t('printTargets.validation.targetCodeRequired'), trigger: 'blur' },
      { max: 100, message: t('printTargets.validation.targetCodeMax'), trigger: 'blur' },
    ],
    display_name: [
      {
        required: true,
        message: t('printTargets.validation.displayNameRequired'),
        trigger: 'blur',
      },
      { max: 200, message: t('printTargets.validation.displayNameMax'), trigger: 'blur' },
    ],
    kind: [
      { required: true, message: t('printTargets.validation.kindRequired'), trigger: 'change' },
    ],
    default_copies: [
      {
        type: 'number',
        min: 1,
        max: 99,
        message: t('printTargets.validation.copiesRange'),
        trigger: 'change',
      },
    ],
  };
  if (form.is_pool) {
    base.pool_members = [
      {
        required: true,
        validator: (_rule, _value, callback) => {
          if (form.pool_members.length < 1) {
            callback(new Error(t('printTargets.validation.poolMembersRequired')));
          } else {
            callback();
          }
        },
        trigger: 'change',
      },
    ];
  } else {
    if (form.kind === 'cups') {
      base.queue_name = [
        {
          required: true,
          message: t('printTargets.validation.queueNameRequired'),
          trigger: 'blur',
        },
      ];
    } else if (form.kind === 'network') {
      base.host = [
        { required: true, message: t('printTargets.validation.hostRequired'), trigger: 'blur' },
      ];
      base.port = [
        {
          validator: (_rule, _value, callback) => {
            if (form.port != null && (form.port < 1 || form.port > 65535)) {
              callback(new Error(t('printTargets.validation.portRange')));
            } else {
              callback();
            }
          },
          trigger: 'change',
        },
      ];
    } else if (form.kind === 'ipp') {
      base.endpoint_url = [
        {
          required: true,
          message: t('printTargets.validation.endpointUrlRequired'),
          trigger: 'blur',
        },
      ];
    } else if (form.kind === 'system_command') {
      base.command_template = [
        {
          required: true,
          message: t('printTargets.validation.commandTemplateRequired'),
          trigger: 'blur',
        },
      ];
    }
  }
  return base;
});

const openDialog = (row?: PrintTarget): void => {
  if (row) {
    original.value = row;
    Object.assign(form, {
      id: row.id,
      target_code: row.target_code,
      display_name: row.display_name,
      kind: row.kind,
      queue_name: row.queue_name ?? '',
      host: row.host ?? '',
      port: row.port ?? undefined,
      endpoint_url: row.endpoint_url ?? '',
      command_template: row.command_template ?? '',
      is_pool: row.is_pool,
      pool_members: [...row.pool_members],
      default_copies: row.default_copies,
      enabled: row.enabled,
    });
  } else {
    original.value = null;
    Object.assign(form, emptyForm());
  }
  formRef.value?.clearValidate();
  dialogVisible.value = true;
};

// 新建载荷：必填基础字段 + 当前配置对应字段；可选键缺省即省略，连接字段已由 rules 保证非空
const buildCreatePayload = (): CreatePrintTargetRequest => {
  const payload: CreatePrintTargetRequest = {
    target_code: form.target_code,
    display_name: form.display_name,
    kind: form.kind,
    default_copies: form.default_copies,
    enabled: form.enabled,
  };
  if (form.is_pool) {
    payload.is_pool = true;
    payload.pool_members = [...form.pool_members];
  } else if (form.kind === 'cups') {
    payload.queue_name = form.queue_name;
  } else if (form.kind === 'network') {
    payload.host = form.host;
    if (form.port != null) payload.port = form.port;
  } else if (form.kind === 'ipp') {
    payload.endpoint_url = form.endpoint_url;
  } else if (form.kind === 'system_command') {
    payload.command_template = form.command_template;
  }
  return payload;
};

const membersEqual = (a: string[], b: string[]): boolean =>
  a.length === b.length && a.every((item, i) => item === b[i]);

// 编辑载荷：只提交被修改的键，连接字段只提交当前 kind 对应项，且绝不提交空串
const buildUpdatePayload = (): UpdatePrintTargetRequest => {
  const o = original.value as PrintTarget;
  const payload: UpdatePrintTargetRequest = {};
  if (form.target_code !== o.target_code) payload.target_code = form.target_code;
  if (form.display_name !== o.display_name) payload.display_name = form.display_name;
  if (form.kind !== o.kind) payload.kind = form.kind;
  if (form.is_pool !== o.is_pool) payload.is_pool = form.is_pool;
  if (form.default_copies !== o.default_copies) payload.default_copies = form.default_copies;
  if (form.enabled !== o.enabled) payload.enabled = form.enabled;
  if (form.is_pool) {
    if (!membersEqual(form.pool_members, o.pool_members))
      payload.pool_members = [...form.pool_members];
  } else if (form.kind === 'cups') {
    if (form.queue_name && form.queue_name !== o.queue_name) payload.queue_name = form.queue_name;
  } else if (form.kind === 'network') {
    if (form.host && form.host !== o.host) payload.host = form.host;
    if (form.port != null && form.port !== o.port) payload.port = form.port;
  } else if (form.kind === 'ipp') {
    if (form.endpoint_url && form.endpoint_url !== o.endpoint_url)
      payload.endpoint_url = form.endpoint_url;
  } else if (form.kind === 'system_command') {
    if (form.command_template && form.command_template !== o.command_template) {
      payload.command_template = form.command_template;
    }
  }
  return payload;
};

const handleSubmit = async (): Promise<void> => {
  if (submitLoading.value) return;
  if (!formRef.value) return;
  // 锁必须在异步校验之前落下：validate 的回调要下一拍才执行，若等到回调里再置位，
  // 连续两次点击会同时通过守卫并各自建出一条打印目标。
  submitLoading.value = true;
  try {
    const valid = await formRef.value.validate().catch(() => false);
    if (!valid) return;
    if (form.id) {
      await updatePrintTarget(form.id, buildUpdatePayload());
      msg.updateOk();
      logger.info('[print-targets] updated id=%d', form.id);
    } else {
      await createPrintTarget(buildCreatePayload());
      msg.createOk();
      logger.info('[print-targets] created code=%s', form.target_code);
    }
    dialogVisible.value = false;
    await fetchList();
  } catch (error: unknown) {
    logger.error('[print-targets] submit failed', error);
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('printTargets.message.operationFailed')
    );
  } finally {
    submitLoading.value = false;
  }
};

const handleDelete = async (row: PrintTarget): Promise<void> => {
  if (deletingId.value !== null) return;
  deletingId.value = row.id;
  try {
    await ElMessageBox.confirm(
      t('printTargets.message.deleteConfirm'),
      t('printTargets.message.deleteConfirmTitle'),
      {
        type: 'warning',
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
      }
    );
    await deletePrintTarget(row.id);
    msg.deleteOk();
    logger.info('[print-targets] deleted id=%d', row.id);
    await fetchList();
  } catch (error: unknown) {
    if (!isDialogDismissal(error)) {
      logger.error('[print-targets] delete failed', error);
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          t('printTargets.message.deleteFailed')
      );
    }
  } finally {
    deletingId.value = null;
  }
};

const androidClient = isAndroidClient();
const btLoading = ref(false);
const btState = ref<BluetoothAdapterState | null>(null);
const btSelectedAddress = ref('');
const btConnectedAddress = ref<string | null>(null);

const documentPaper = reactive<PaperSettings>(loadDocumentPaper());
const labelPaper = reactive<LabelPaperSettings>(loadLabelPaper());

watch(
  documentPaper,
  value => {
    if (!Number.isFinite(value.widthMm) || value.widthMm < 20) return;
    saveDocumentPaper({ ...value });
  },
  { deep: true }
);
watch(
  labelPaper,
  value => {
    if (!Number.isFinite(value.widthMm) || value.widthMm < 20) return;
    if (!Number.isFinite(value.heightMm) || value.heightMm < 10) return;
    saveLabelPaper({ ...value, fields: [...value.fields] });
  },
  { deep: true }
);

const LABEL_FIELD_TEXT: Record<LabelFieldKey, string> = {
  piece_no: 'printTargets.paper.fields.pieceNo',
  dye_lot_no: 'printTargets.paper.fields.dyeLotNo',
  color_no: 'printTargets.paper.fields.colorNo',
  length: 'printTargets.paper.fields.length',
  weight: 'printTargets.paper.fields.weight',
  width: 'printTargets.paper.fields.width',
  gram_weight: 'printTargets.paper.fields.gramWeight',
  barcode: 'printTargets.paper.fields.barcode',
  warehouse_in_at: 'printTargets.paper.fields.warehouseInAt',
};

const ROUTE_TYPE_TEXT: Record<string, string> = {
  purchase_order: 'printTargets.routing.types.purchase_order',
  voucher: 'printTargets.routing.types.voucher',
  inventory_stock: 'printTargets.routing.types.inventory_stock',
  ap_invoice: 'printTargets.routing.types.ap_invoice',
  ar_invoice: 'printTargets.routing.types.ar_invoice',
  finance_subject: 'printTargets.routing.types.finance_subject',
  customer: 'printTargets.routing.types.customer',
  supplier: 'printTargets.routing.types.supplier',
  warehouse: 'printTargets.routing.types.warehouse',
  fabric_piece: 'printTargets.routing.types.fabric_piece',
};

interface RouteRow {
  key: PrintRouteKey;
  groupLabel: string;
  typeLabel: string;
  address: string;
}

function buildRouteRows(): RouteRow[] {
  const routes = loadPrintRoutes();
  return [
    ...DOCUMENT_PRINT_TYPES.map(type => {
      const key = documentPrintRoute(type);
      return {
        key,
        groupLabel: t('printTargets.routing.documentGroup'),
        typeLabel: t(ROUTE_TYPE_TEXT[type]),
        address: routes[key]?.address ?? '',
      };
    }),
    ...LABEL_PRINT_TYPES.map(type => {
      const key = labelPrintRoute(type);
      return {
        key,
        groupLabel: t('printTargets.routing.labelGroup'),
        typeLabel: t(ROUTE_TYPE_TEXT[type]),
        address: routes[key]?.address ?? '',
      };
    }),
  ];
}

const routeRows = ref<RouteRow[]>(buildRouteRows());

function handleRouteChange(row: RouteRow, value?: unknown): void {
  const address = typeof value === 'string' ? value : '';
  const printer = (btState.value?.printers ?? []).find(item => item.address === address);
  savePrintRoute(row.key, address ? { name: printer?.name || address, address } : null);
  row.address = address;
  ElMessage.success(t('printTargets.routing.saved'));
}

function bluetoothErrorDetail(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function refreshBluetoothState(): Promise<void> {
  if (!androidClient) return;
  btLoading.value = true;
  try {
    const state = await listBluetoothPrinters();
    btState.value = state;
    btConnectedAddress.value = state.connectedAddress ?? null;
    const saved = loadPrinterPreference();
    if (saved && state.printers.some(printer => printer.address === saved.address)) {
      btSelectedAddress.value = saved.address;
    } else if (!btSelectedAddress.value && state.printers.length === 1) {
      btSelectedAddress.value = state.printers[0].address;
    }
  } catch (error) {
    logger.error('[print-targets] 读取已配对蓝牙设备失败', error);
    ElMessage.error(bluetoothErrorDetail(error) || t('printTargets.bluetooth.message.listFailed'));
  } finally {
    btLoading.value = false;
  }
}

async function handleBluetoothSettings(): Promise<void> {
  try {
    await openBluetoothSettings();
    ElMessage.success(t('printTargets.bluetooth.message.settingsOpened'));
  } catch (error) {
    logger.error('[print-targets] 打开系统蓝牙设置失败', error);
    ElMessage.error(
      bluetoothErrorDetail(error) || t('printTargets.bluetooth.message.settingsFailed')
    );
  }
}

async function handleBluetoothConnect(): Promise<void> {
  btLoading.value = true;
  try {
    // 用时申请，常驻申请入口移除后，连接动作是本页面唯一能调起系统授权的入口，
    // 因此未授权时必须先在这里申请，被拒后保留可见提示并中止本次连接。
    if (btState.value && !btState.value.granted) {
      const permission = await requestBluetoothPermission();
      if (!permission.granted) {
        ElMessage.warning(t('printTargets.bluetooth.message.permissionDenied'));
        return;
      }
      // 壳侧的已配对列表只有授权后才可读，授权放行后立即重读一轮，
      // 用户才能在同一张卡片里选到打印机并继续本次连接。
      await refreshBluetoothState();
    }
    const address = btSelectedAddress.value;
    if (!address) {
      ElMessage.warning(t('printTargets.bluetooth.message.printerRequired'));
      return;
    }
    const connection = await connectBluetoothPrinter(address);
    btConnectedAddress.value = connection.address;
    savePrinterPreference({ name: connection.name, address: connection.address });
    ElMessage.success(t('printTargets.bluetooth.message.connectSuccess'));
  } catch (error) {
    logger.error('[print-targets] 连接蓝牙打印机失败', error);
    ElMessage.error(
      bluetoothErrorDetail(error) || t('printTargets.bluetooth.message.connectFailed')
    );
  } finally {
    btLoading.value = false;
  }
}

async function handleBluetoothDisconnect(): Promise<void> {
  btLoading.value = true;
  try {
    await disconnectBluetoothPrinter();
    btConnectedAddress.value = null;
    ElMessage.success(t('printTargets.bluetooth.message.disconnectSuccess'));
  } catch (error) {
    logger.error('[print-targets] 断开蓝牙打印机失败', error);
    ElMessage.error(
      bluetoothErrorDetail(error) || t('printTargets.bluetooth.message.disconnectFailed')
    );
  } finally {
    btLoading.value = false;
  }
}

onMounted(() => {
  void fetchList();
  if (androidClient) {
    void refreshBluetoothState();
  }
});
</script>

<style scoped>
.print-targets-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
.header-actions {
  display: flex;
  gap: 12px;
}
.bluetooth-card {
  margin-bottom: 20px;
}
.routing-card,
.paper-card {
  margin-bottom: 20px;
}
.paper-section + .paper-section {
  margin-top: 20px;
}
.paper-subtitle {
  margin: 0 0 12px;
  font-size: 14px;
  font-weight: 600;
  color: #303133;
}
.paper-number {
  width: 140px;
}
.paper-unit {
  margin: 0 12px 0 6px;
  color: #606266;
}
.paper-hint {
  color: #909399;
  font-size: 12px;
}
.paper-fields {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.bluetooth-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}
.bluetooth-title {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}
.bluetooth-description {
  margin: 0 0 12px;
  color: #606266;
  font-size: 13px;
  line-height: 1.6;
}
.bluetooth-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;
}
.bluetooth-actions :deep(.el-button + .el-button) {
  margin-left: 0;
}
.bluetooth-form {
  margin-top: 12px;
}
.bluetooth-card :deep(.el-alert) {
  margin-bottom: 12px;
}
</style>
