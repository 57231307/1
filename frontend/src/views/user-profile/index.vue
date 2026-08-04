<template>
  <div class="user-profile-page">
    <div class="header">
      <h2>{{ t('userProfile.title') }}</h2>
    </div>

    <div class="content">
      <!-- 个人信息卡片 -->
      <div class="left-column">
        <el-card class="profile-card">
          <template #header>
            <div class="card-header">
              <span>{{ t('userProfile.profile.title') }}</span>
              <el-button type="primary" :loading="profileLoading" @click="handleSaveProfile">
                {{ t('userProfile.profile.save') }}
              </el-button>
            </div>
          </template>

          <div class="profile-content">
            <!-- 头像上传 -->
            <div class="avatar-section">
              <el-upload
                class="avatar-uploader"
                :show-file-list="false"
                :before-upload="beforeAvatarUpload"
                :on-change="handleAvatarChange"
                accept="image/*"
              >
                <img
                  v-if="profileForm.avatar"
                  :src="profileForm.avatar"
                  class="avatar"
                  loading="lazy"
                  :alt="
                    profileForm.real_name
                      ? t('userProfile.profile.avatarAlt', { name: profileForm.real_name })
                      : t('userProfile.profile.avatarAltDefault')
                  "
                />
                <el-icon v-else class="avatar-uploader-icon"><Plus /></el-icon>
              </el-upload>
              <div class="avatar-tip">{{ t('userProfile.profile.avatarTip') }}</div>
            </div>

            <!-- 个人信息表单 -->
            <el-form
              ref="profileFormRef"
              :model="profileForm"
              :rules="profileRules"
              label-width="100px"
              class="profile-form"
              :aria-label="t('userProfile.profile.formAriaLabel')"
            >
              <el-form-item :label="t('userProfile.profile.username')">
                <el-input v-model="profileForm.username" disabled />
              </el-form-item>
              <el-form-item :label="t('userProfile.profile.realName')" prop="real_name">
                <el-input
                  v-model="profileForm.real_name"
                  :placeholder="t('userProfile.profile.realNamePlaceholder')"
                />
              </el-form-item>
              <el-form-item :label="t('userProfile.profile.email')" prop="email">
                <el-input
                  v-model="profileForm.email"
                  :placeholder="t('userProfile.profile.emailPlaceholder')"
                />
              </el-form-item>
              <el-form-item :label="t('userProfile.profile.phone')" prop="phone">
                <el-input
                  v-model="profileForm.phone"
                  :placeholder="t('userProfile.profile.phonePlaceholder')"
                />
              </el-form-item>
              <el-form-item :label="t('userProfile.profile.department')">
                <el-input v-model="profileForm.department_name" disabled />
              </el-form-item>
              <el-form-item :label="t('userProfile.profile.role')">
                <el-input :value="profileForm.role_names?.join(', ')" disabled />
              </el-form-item>
            </el-form>
          </div>
        </el-card>

        <!-- 安全设置快捷入口 -->
        <el-card class="security-card">
          <template #header>
            <div class="card-header">
              <span>{{ t('userProfile.security.title') }}</span>
            </div>
          </template>
          <div class="security-actions">
            <el-button type="primary" plain class="security-action-btn" @click="goTo2fa">
              {{ t('userProfile.security.twofa') }}
            </el-button>
            <el-button type="primary" plain class="security-action-btn" @click="goToChangePwd">
              {{ t('userProfile.security.changePassword') }}
            </el-button>
          </div>
        </el-card>
      </div>

      <!-- 修改密码卡片 -->
      <el-card class="password-card">
        <template #header>
          <div class="card-header">
            <span>{{ t('userProfile.password.title') }}</span>
            <el-button type="primary" :loading="passwordLoading" @click="handleChangePassword">
              {{ t('userProfile.password.button') }}
            </el-button>
          </div>
        </template>

        <el-form
          ref="passwordFormRef"
          :model="passwordForm"
          :rules="passwordRules"
          label-width="100px"
          class="password-form"
          :aria-label="t('userProfile.password.formAriaLabel')"
        >
          <el-form-item :label="t('userProfile.password.oldPassword')" prop="old_password">
            <el-input
              v-model="passwordForm.old_password"
              type="password"
              :placeholder="t('userProfile.password.oldPasswordPlaceholder')"
              show-password
            />
          </el-form-item>
          <el-form-item :label="t('userProfile.password.newPassword')" prop="new_password">
            <el-input
              v-model="passwordForm.new_password"
              type="password"
              :placeholder="t('userProfile.password.newPasswordPlaceholder')"
              show-password
            />
          </el-form-item>
          <el-form-item :label="t('userProfile.password.confirmPassword')" prop="confirm_password">
            <el-input
              v-model="passwordForm.confirm_password"
              type="password"
              :placeholder="t('userProfile.password.confirmPasswordPlaceholder')"
              show-password
            />
          </el-form-item>
        </el-form>
      </el-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules, FormItemRule, UploadFile } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import {
  getUserProfile,
  updateUserProfile,
  changePassword,
  uploadAvatar,
  type UserProfile,
  type UserProfileUpdateRequest,
  type ChangePasswordRequest,
} from '@/api/user-profile';

const { t } = useI18n({ useScope: 'global' });
const router = useRouter();

/** 跳转到 2FA 设置页 */
const goTo2fa = () => {
  router.push('/security/two-factor-setup');
};

/** 跳转到修改密码页 */
const goToChangePwd = () => {
  router.push('/security/change-password');
};

const profileLoading = ref(false);
const passwordLoading = ref(false);
const profileFormRef = ref<FormInstance>();
const passwordFormRef = ref<FormInstance>();

const profileForm = reactive<UserProfile>({
  id: 0,
  username: '',
  real_name: '',
  email: '',
  phone: '',
  avatar: '',
  department_id: undefined,
  department_name: '',
  role_ids: [],
  role_names: [],
  status: 1,
  created_at: '',
  updated_at: '',
});

const passwordForm = reactive<ChangePasswordRequest>({
  old_password: '',
  new_password: '',
  confirm_password: '',
});

const profileRules: FormRules = {
  real_name: [
    { required: true, message: t('userProfile.validation.realNameRequired'), trigger: 'blur' },
  ],
  email: [
    {
      type: 'email',
      message: t('userProfile.validation.emailPattern'),
      trigger: ['blur', 'change'],
    },
  ],
  phone: [
    {
      pattern: /^1[3-9]\d{9}$/,
      message: t('userProfile.validation.phonePattern'),
      trigger: 'blur',
    },
  ],
};

const passwordRules: FormRules = {
  old_password: [
    { required: true, message: t('userProfile.validation.oldPasswordRequired'), trigger: 'blur' },
  ],
  new_password: [
    { required: true, message: t('userProfile.validation.newPasswordRequired'), trigger: 'blur' },
    { min: 6, message: t('userProfile.validation.newPasswordMinLength'), trigger: 'blur' },
  ],
  confirm_password: [
    {
      required: true,
      message: t('userProfile.validation.confirmPasswordRequired'),
      trigger: 'blur',
    },
    {
      validator: ((_rule: unknown, value: string, callback: (error?: Error) => void) => {
        if (value !== passwordForm.new_password) {
          callback(new Error(t('userProfile.validation.passwordMismatch')));
        } else {
          callback();
        }
      }) as FormItemRule['validator'],
      trigger: 'blur',
    },
  ],
};

const loadUserProfile = async () => {
  try {
    const res = await getUserProfile();
    if (res.data) {
      Object.assign(profileForm, res.data);
    }
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('userProfile.message.loadProfileFailed')
    );
  }
};

const beforeAvatarUpload = (file: File) => {
  const isImage = file.type.startsWith('image/');
  const isLt2M = file.size / 1024 / 1024 < 2;

  if (!isImage) {
    ElMessage.error(t('userProfile.message.avatarTypeInvalid'));
    return false;
  }
  if (!isLt2M) {
    ElMessage.error(t('userProfile.message.avatarSizeExceeded'));
    return false;
  }
  return true;
};

const handleAvatarChange = async (uploadFile: UploadFile) => {
  if (!uploadFile.raw) return;

  try {
    const res = await uploadAvatar(uploadFile.raw);
    if (res.data?.avatar_url) {
      profileForm.avatar = res.data.avatar_url;
      ElMessage.success(t('userProfile.message.avatarUploadSuccess'));
    }
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('userProfile.message.avatarUploadFailed')
    );
  }
};

const handleSaveProfile = async () => {
  if (!profileFormRef.value) return;

  await profileFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return;

    profileLoading.value = true;
    try {
      const updateData: UserProfileUpdateRequest = {
        real_name: profileForm.real_name,
        email: profileForm.email,
        phone: profileForm.phone,
        department_id: profileForm.department_id,
        role_ids: profileForm.role_ids,
      };
      await updateUserProfile(updateData);
      ElMessage.success(t('userProfile.message.profileSaveSuccess'));
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          t('userProfile.message.profileSaveFailed')
      );
    } finally {
      profileLoading.value = false;
    }
  });
};

const handleChangePassword = async () => {
  if (!passwordFormRef.value) return;

  await passwordFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return;

    passwordLoading.value = true;
    try {
      await changePassword(passwordForm);
      ElMessage.success(t('userProfile.message.passwordChangeSuccess'));
      // 清空表单
      passwordFormRef.value?.resetFields();
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          t('userProfile.message.passwordChangeFailed')
      );
    } finally {
      passwordLoading.value = false;
    }
  });
};

onMounted(() => {
  loadUserProfile();
});
</script>

<style scoped>
.user-profile-page {
  padding: 20px;
}

.header {
  margin-bottom: 20px;
}

.content {
  display: flex;
  gap: 20px;
}

.left-column {
  flex: 2;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.profile-card {
  width: 100%;
}

.security-card {
  width: 100%;
}

.security-actions {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.security-action-btn {
  min-width: 140px;
}

.password-card {
  flex: 1;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.profile-content {
  display: flex;
  gap: 40px;
}

.avatar-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
}

.avatar-uploader {
  width: 120px;
  height: 120px;
  border: 1px dashed #d9d9d9;
  border-radius: 6px;
  cursor: pointer;
  position: relative;
  overflow: hidden;
}

.avatar-uploader:hover {
  border-color: #409eff;
}

.avatar-uploader-icon {
  font-size: 28px;
  color: #8c939d;
  width: 120px;
  height: 120px;
  display: flex;
  justify-content: center;
  align-items: center;
}

.avatar {
  width: 120px;
  height: 120px;
  display: block;
  object-fit: cover;
}

.avatar-tip {
  font-size: 12px;
  color: #999;
}

.profile-form {
  flex: 1;
}

.password-form {
  max-width: 400px;
}
</style>
