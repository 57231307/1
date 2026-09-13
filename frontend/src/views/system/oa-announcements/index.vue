<!--
  OA 公告管理页面
  - 路由：/system/oa-announcements
  - 权限：oa-announcements:read
  - 功能：列表、创建、编辑、删除、发布（联动站内通知）、归档
-->
<template>
  <div class="oa-announcement-page">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.oaAnnouncement.title') }}</h2>
      <el-button v-permission="'oa-announcements:create'" type="primary" @click="openDialog()">
        <el-icon><Plus /></el-icon> {{ t('system.oaAnnouncement.button.create') }}
      </el-button>
    </div>

    <el-card shadow="hover">
      <div style="margin-bottom: 16px">
        <el-select
          v-model="query.status"
          :placeholder="t('system.oaAnnouncement.placeholder.status')"
          clearable
          style="width: 140px"
          @change="fetchList"
        >
          <el-option label="草稿" value="DRAFT" />
          <el-option label="已发布" value="PUBLISHED" />
          <el-option label="已归档" value="ARCHIVED" />
        </el-select>
        <el-select
          v-model="query.announcement_type"
          :placeholder="t('system.oaAnnouncement.placeholder.type')"
          clearable
          style="width: 140px; margin-left: 10px"
          @change="fetchList"
        >
          <el-option label="通知" value="NOTICE" />
          <el-option label="公告" value="ANNOUNCEMENT" />
          <el-option label="新闻" value="NEWS" />
        </el-select>
      </div>

      <el-table v-loading="loading" :data="list" stripe>
        <el-table-column
          prop="title"
          :label="t('system.oaAnnouncement.column.title')"
          min-width="180"
        />
        <el-table-column
          prop="announcement_type"
          :label="t('system.oaAnnouncement.column.type')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag size="small">{{ row.announcement_type }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="status"
          :label="t('system.oaAnnouncement.column.status')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="statusTagType(row.status)" size="small">{{
              statusLabel(row.status)
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="visibility_scope"
          :label="t('system.oaAnnouncement.column.scope')"
          width="100"
          align="center"
        />
        <el-table-column
          prop="publish_date"
          :label="t('system.oaAnnouncement.column.publishDate')"
          width="130"
        />
        <el-table-column
          prop="created_at"
          :label="t('system.oaAnnouncement.column.createdAt')"
          width="160"
        />
        <el-table-column
          :label="t('system.oaAnnouncement.column.action')"
          width="260"
          fixed="right"
        >
          <template #default="{ row }">
            <el-button
              v-if="row.status === 'DRAFT'"
              v-permission="'oa-announcements:update'"
              size="small"
              link
              @click="openDialog(row as OaAnnouncement)"
              >{{ t('system.oaAnnouncement.button.edit') }}</el-button
            >
            <el-button
              v-if="row.status === 'DRAFT'"
              v-permission="'oa-announcements:update'"
              size="small"
              link
              type="success"
              @click="handlePublish(row as OaAnnouncement)"
              >{{ t('system.oaAnnouncement.button.publish') }}</el-button
            >
            <el-button
              v-if="row.status === 'PUBLISHED'"
              v-permission="'oa-announcements:update'"
              size="small"
              link
              type="warning"
              @click="handleArchive(row as OaAnnouncement)"
              >{{ t('system.oaAnnouncement.button.archive') }}</el-button
            >
            <el-button
              v-if="row.status === 'DRAFT'"
              v-permission="'oa-announcements:delete'"
              size="small"
              link
              type="danger"
              @click="handleDelete(row as OaAnnouncement)"
              >{{ t('system.oaAnnouncement.button.delete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-model:current-page="query.page"
        v-model:page-size="query.page_size"
        :total="total"
        :page-sizes="[10, 20, 50]"
        layout="total, sizes, prev, pager, next"
        style="margin-top: 16px; justify-content: flex-end"
        @size-change="fetchList"
        @current-change="fetchList"
      />
    </el-card>

    <!-- 编辑弹窗 -->
    <el-dialog
      v-model="dialogVisible"
      :title="
        form.id
          ? t('system.oaAnnouncement.dialog.editTitle')
          : t('system.oaAnnouncement.dialog.createTitle')
      "
      width="650px"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-form-item :label="t('system.oaAnnouncement.form.label.title')" prop="title">
          <el-input v-model="form.title" :maxlength="200" />
        </el-form-item>
        <el-form-item :label="t('system.oaAnnouncement.form.label.type')" prop="announcement_type">
          <el-select v-model="form.announcement_type" style="width: 100%">
            <el-option label="通知" value="NOTICE" />
            <el-option label="公告" value="ANNOUNCEMENT" />
            <el-option label="新闻" value="NEWS" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('system.oaAnnouncement.form.label.content')" prop="content">
          <el-input v-model="form.content" type="textarea" :rows="6" :maxlength="5000" />
        </el-form-item>
        <el-form-item
          :label="t('system.oaAnnouncement.form.label.publishDate')"
          prop="publish_date"
        >
          <el-date-picker
            v-model="form.publish_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item
          :label="t('system.oaAnnouncement.form.label.effectiveDate')"
          prop="effective_date"
        >
          <el-date-picker
            v-model="form.effective_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('system.oaAnnouncement.form.label.expiryDate')">
          <el-date-picker
            v-model="form.expiry_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('system.oaAnnouncement.form.label.scope')" prop="visibility_scope">
          <el-select v-model="form.visibility_scope" style="width: 100%" @change="onScopeChange">
            <el-option label="全员" value="ALL" />
            <el-option label="按部门" value="DEPT" />
            <el-option label="按角色" value="ROLE" />
            <el-option label="指定用户" value="CUSTOM" />
          </el-select>
        </el-form-item>
        <el-form-item v-if="form.visibility_scope === 'CUSTOM'" label="目标用户">
          <el-select
            v-model="customUserIds"
            multiple
            filterable
            placeholder="选择用户"
            style="width: 100%"
          >
            <el-option v-for="u in userOptions" :key="u.id" :label="u.username" :value="u.id" />
          </el-select>
        </el-form-item>
        <el-form-item v-if="form.visibility_scope === 'DEPT'" label="目标部门">
          <el-select
            v-model="customDeptIds"
            multiple
            filterable
            placeholder="选择部门"
            style="width: 100%"
          >
            <el-option v-for="d in deptOptions" :key="d.id" :label="d.name" :value="d.id" />
          </el-select>
        </el-form-item>
        <el-form-item v-if="form.visibility_scope === 'ROLE'" label="目标角色">
          <el-select
            v-model="customRoleIds"
            multiple
            filterable
            placeholder="选择角色"
            style="width: 100%"
          >
            <el-option v-for="r in roleOptions" :key="r.id" :label="r.name" :value="r.id" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('system.oaAnnouncement.form.label.remarks')">
          <el-input v-model="form.remarks" type="textarea" :rows="2" />
        </el-form-item>
        <el-form-item :label="t('system.oaAnnouncement.form.label.isTop')">
          <el-switch v-model="form.is_top" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{
          t('system.oaAnnouncement.button.cancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="submitForm">{{
          t('system.oaAnnouncement.button.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance, FormRules } from 'element-plus';
import {
  listOaAnnouncements,
  createOaAnnouncement,
  updateOaAnnouncement,
  deleteOaAnnouncement,
  publishOaAnnouncement,
  archiveOaAnnouncement,
  type OaAnnouncement,
} from '@/api/oa-announcement';
import { getUserList, type User } from '@/api/user';
import { getRoleList, type Role } from '@/api/role';
import { request } from '@/api/request';

const { t } = useI18n({ useScope: 'global' });

const list = ref<OaAnnouncement[]>([]);
const total = ref(0);
const loading = ref(false);
const query = reactive({ page: 1, page_size: 20, status: '', announcement_type: '' });

const fetchList = async () => {
  loading.value = true;
  try {
    const res = await listOaAnnouncements({
      page: query.page,
      page_size: query.page_size,
      status: query.status || undefined,
      announcement_type: query.announcement_type || undefined,
    });
    const d = res.data as { items?: OaAnnouncement[]; data?: OaAnnouncement[] } | undefined;
    list.value = d?.items || d?.data || [];
    total.value = (res.data as { total?: number })?.total ?? 0;
  } catch (e) {
    ElMessage.error((e as Error).message || t('system.oaAnnouncement.message.fetchFailed'));
  } finally {
    loading.value = false;
  }
};

const statusTagType = (s: string) =>
  ({ DRAFT: 'info', PUBLISHED: 'success', ARCHIVED: 'warning' })[s] || 'info';
const statusLabel = (s: string) =>
  ({ DRAFT: '草稿', PUBLISHED: '已发布', ARCHIVED: '已归档' })[s] || s;

// ===== 弹窗 =====
const dialogVisible = ref(false);
const formRef = ref<FormInstance>();
const submitLoading = ref(false);
const form = reactive({
  id: 0,
  title: '',
  content: '',
  announcement_type: 'NOTICE',
  publish_date: new Date().toISOString().slice(0, 10),
  effective_date: new Date().toISOString().slice(0, 10),
  expiry_date: '',
  remarks: '',
  is_top: false,
  visibility_scope: 'ALL',
});
const customUserIds = ref<number[]>([]);
const customDeptIds = ref<number[]>([]);
const customRoleIds = ref<number[]>([]);
const userOptions = ref<User[]>([]);
const deptOptions = ref<{ id: number; name: string }[]>([]);
const roleOptions = ref<Role[]>([]);

const rules: FormRules = {
  title: [{ required: true, message: '请输入标题', trigger: 'blur' }],
  content: [{ required: true, message: '请输入内容', trigger: 'blur' }],
  announcement_type: [{ required: true, message: '请选择类型', trigger: 'change' }],
  publish_date: [{ required: true, message: '请选择发布日期', trigger: 'change' }],
  effective_date: [{ required: true, message: '请选择生效日期', trigger: 'change' }],
  visibility_scope: [{ required: true, message: '请选择可见范围', trigger: 'change' }],
};

const openDialog = (row?: OaAnnouncement) => {
  formRef.value?.resetFields();
  customUserIds.value = [];
  customDeptIds.value = [];
  customRoleIds.value = [];
  if (row) {
    Object.assign(form, {
      id: row.id ?? 0,
      title: row.title,
      content: row.content,
      announcement_type: row.announcement_type,
      publish_date: row.publish_date,
      effective_date: row.effective_date,
      expiry_date: row.expiry_date || '',
      remarks: row.remarks || '',
      is_top: row.is_top,
      visibility_scope: row.visibility_scope,
    });
    const cfg = row.visible_scope_config;
    if (cfg?.user_ids) customUserIds.value = [...cfg.user_ids];
    if (cfg?.department_ids) customDeptIds.value = [...cfg.department_ids];
    if (cfg?.role_ids) customRoleIds.value = [...cfg.role_ids];
  } else {
    Object.assign(form, {
      id: 0,
      title: '',
      content: '',
      announcement_type: 'NOTICE',
      publish_date: new Date().toISOString().slice(0, 10),
      effective_date: new Date().toISOString().slice(0, 10),
      expiry_date: '',
      remarks: '',
      is_top: false,
      visibility_scope: 'ALL',
    });
  }
  dialogVisible.value = true;
};

const onScopeChange = () => {
  customUserIds.value = [];
  customDeptIds.value = [];
  customRoleIds.value = [];
};

const buildVisibleScopeConfig = () => {
  switch (form.visibility_scope) {
    case 'CUSTOM':
      return { user_ids: customUserIds.value };
    case 'DEPT':
      return { department_ids: customDeptIds.value };
    case 'ROLE':
      return { role_ids: customRoleIds.value };
    default:
      return null;
  }
};

const submitForm = async () => {
  const valid = await formRef.value?.validate();
  if (!valid) return;
  submitLoading.value = true;
  const payload = {
    title: form.title,
    content: form.content,
    announcement_type: form.announcement_type,
    publish_date: form.publish_date,
    effective_date: form.effective_date,
    expiry_date: form.expiry_date || undefined,
    remarks: form.remarks || undefined,
    is_top: form.is_top,
    visibility_scope: form.visibility_scope,
    visible_scope_config: buildVisibleScopeConfig(),
  };
  try {
    if (form.id) {
      await updateOaAnnouncement(form.id, payload);
      ElMessage.success(t('system.oaAnnouncement.message.updateSuccess'));
    } else {
      await createOaAnnouncement(payload);
      ElMessage.success(t('system.oaAnnouncement.message.createSuccess'));
    }
    dialogVisible.value = false;
    fetchList();
  } catch (e) {
    ElMessage.error((e as Error).message || t('system.oaAnnouncement.message.operationFailed'));
  } finally {
    submitLoading.value = false;
  }
};

const handlePublish = async (row: OaAnnouncement) => {
  try {
    await ElMessageBox.confirm('确认发布该公告？发布后将自动向目标用户推送站内通知。', '发布确认', {
      type: 'warning',
    });
    const res = await publishOaAnnouncement(row.id!);
    const count = (res.data as { notified_count?: number })?.notified_count ?? 0;
    ElMessage.success(`公告已发布${count > 0 ? `，已通知 ${count} 位用户` : ''}`);
    fetchList();
  } catch (e) {
    if (e !== 'cancel') ElMessage.error((e as Error).message || '发布失败');
  }
};

const handleArchive = async (row: OaAnnouncement) => {
  try {
    await ElMessageBox.confirm('确认归档该公告？', '归档确认', { type: 'warning' });
    await archiveOaAnnouncement(row.id!);
    ElMessage.success('公告已归档');
    fetchList();
  } catch (e) {
    if (e !== 'cancel') ElMessage.error((e as Error).message || '归档失败');
  }
};

const handleDelete = async (row: OaAnnouncement) => {
  try {
    await ElMessageBox.confirm('确认删除该公告？仅草稿状态可删除。', '删除确认', {
      type: 'warning',
    });
    await deleteOaAnnouncement(row.id!);
    ElMessage.success('删除成功');
    fetchList();
  } catch (e) {
    if (e !== 'cancel') ElMessage.error((e as Error).message || '删除失败');
  }
};

const fetchOptions = async () => {
  try {
    const ures = await getUserList({ page: 1, page_size: 200 });
    const ud = ures.data as { items?: User[]; list?: User[]; data?: User[] } | undefined;
    userOptions.value = (ud?.items ||
      ud?.list ||
      ud?.data ||
      (Array.isArray(ud) ? ud : [])) as User[];
  } catch {
    /* 非管理员静默 */
  }
  try {
    const rres = await getRoleList();
    const rd = rres.data as { roles?: Role[]; items?: Role[]; data?: Role[] } | Role[] | undefined;
    roleOptions.value = (
      Array.isArray(rd) ? rd : rd?.roles || rd?.items || rd?.data || []
    ) as Role[];
  } catch {
    /* 静默 */
  }
  try {
    const dres = await request.get<{ items?: { id: number; name: string }[] }>('/departments/', {
      params: { page: 1, page_size: 200 },
    });
    const raw = dres as unknown as {
      data?: { items?: { id: number; name: string }[]; list?: { id: number; name: string }[] };
    };
    deptOptions.value = raw.data?.items || raw.data?.list || [];
  } catch {
    /* 静默 */
  }
};

onMounted(() => {
  fetchList();
  fetchOptions();
});
</script>
