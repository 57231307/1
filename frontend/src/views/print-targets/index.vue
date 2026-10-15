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
            <el-input-number
              v-model="form.port"
              :min="1"
              :max="65535"
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
          <el-input-number v-model="form.default_copies" :min="1" :max="99" />
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
import { Plus, Refresh } from '@element-plus/icons-vue';
import { isDialogDismissal } from '@/utils/monitor';
import { msg } from '@/utils/message';
import { logger } from '@/utils/logger';
import { PERMISSIONS } from '@/constants/permissions';
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
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    submitLoading.value = true;
    try {
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
  });
};

const handleDelete = async (row: PrintTarget): Promise<void> => {
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
  }
};

onMounted(fetchList);
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
</style>
