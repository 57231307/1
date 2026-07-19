<template>
  <div class="user-profile-page">
    <div class="header">
      <h2>个人中心</h2>
    </div>

    <div class="content">
      <!-- 个人信息卡片 -->
      <div class="left-column">
        <el-card class="profile-card">
          <template #header>
            <div class="card-header">
              <span>个人信息</span>
              <el-button type="primary" :loading="profileLoading" @click="handleSaveProfile">
                保存修改
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
                  :alt="profileForm.real_name ? `${profileForm.real_name}的头像` : '用户头像'"
                />
                <el-icon v-else class="avatar-uploader-icon"><Plus /></el-icon>
              </el-upload>
              <div class="avatar-tip">点击上传头像</div>
            </div>

            <!-- 个人信息表单 -->
            <el-form
              ref="profileFormRef"
              :model="profileForm"
              :rules="profileRules"
              label-width="100px"
              class="profile-form"
            >
              <el-form-item label="用户名">
                <el-input v-model="profileForm.username" disabled />
              </el-form-item>
              <el-form-item label="姓名" prop="real_name">
                <el-input v-model="profileForm.real_name" placeholder="请输入姓名" />
              </el-form-item>
              <el-form-item label="邮箱" prop="email">
                <el-input v-model="profileForm.email" placeholder="请输入邮箱" />
              </el-form-item>
              <el-form-item label="手机号" prop="phone">
                <el-input v-model="profileForm.phone" placeholder="请输入手机号" />
              </el-form-item>
              <el-form-item label="部门">
                <el-input v-model="profileForm.department_name" disabled />
              </el-form-item>
              <el-form-item label="角色">
                <el-input :value="profileForm.role_names?.join(', ')" disabled />
              </el-form-item>
            </el-form>
          </div>
        </el-card>

        <!-- 安全设置快捷入口 -->
        <el-card class="security-card">
          <template #header>
            <div class="card-header">
              <span>安全设置</span>
            </div>
          </template>
          <div class="security-actions">
            <el-button
              type="primary"
              plain
              class="security-action-btn"
              @click="goTo2fa"
            >
              2FA 设置
            </el-button>
            <el-button
              type="primary"
              plain
              class="security-action-btn"
              @click="goToChangePwd"
            >
              修改密码
            </el-button>
          </div>
        </el-card>
      </div>

      <!-- 修改密码卡片 -->
      <el-card class="password-card">
        <template #header>
          <div class="card-header">
            <span>修改密码</span>
            <el-button type="primary" :loading="passwordLoading" @click="handleChangePassword">
              修改密码
            </el-button>
          </div>
        </template>

        <el-form
          ref="passwordFormRef"
          :model="passwordForm"
          :rules="passwordRules"
          label-width="100px"
          class="password-form"
        >
          <el-form-item label="原密码" prop="old_password">
            <el-input
              v-model="passwordForm.old_password"
              type="password"
              placeholder="请输入原密码"
              show-password
            />
          </el-form-item>
          <el-form-item label="新密码" prop="new_password">
            <el-input
              v-model="passwordForm.new_password"
              type="password"
              placeholder="请输入新密码"
              show-password
            />
          </el-form-item>
          <el-form-item label="确认密码" prop="confirm_password">
            <el-input
              v-model="passwordForm.confirm_password"
              type="password"
              placeholder="请再次输入新密码"
              show-password
            />
          </el-form-item>
        </el-form>
      </el-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules, FormItemRule, UploadFile } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  getUserProfile,
  updateUserProfile,
  changePassword,
  uploadAvatar,
  type UserProfile,
  type UserProfileUpdateRequest,
  type ChangePasswordRequest,
} from '@/api/user-profile'

const router = useRouter()

/** 跳转到 2FA 设置页 */
const goTo2fa = () => {
  router.push('/security/two-factor-setup')
}

/** 跳转到修改密码页 */
const goToChangePwd = () => {
  router.push('/security/change-password')
}

const profileLoading = ref(false)
const passwordLoading = ref(false)
const profileFormRef = ref<FormInstance>()
const passwordFormRef = ref<FormInstance>()

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
})

const passwordForm = reactive<ChangePasswordRequest>({
  old_password: '',
  new_password: '',
  confirm_password: '',
})

const profileRules: FormRules = {
  real_name: [{ required: true, message: '请输入姓名', trigger: 'blur' }],
  email: [{ type: 'email', message: '请输入正确的邮箱地址', trigger: ['blur', 'change'] }],
  phone: [{ pattern: /^1[3-9]\d{9}$/, message: '请输入正确的手机号', trigger: 'blur' }],
}

const passwordRules: FormRules = {
  old_password: [{ required: true, message: '请输入原密码', trigger: 'blur' }],
  new_password: [
    { required: true, message: '请输入新密码', trigger: 'blur' },
    { min: 6, message: '密码长度不能少于6位', trigger: 'blur' },
  ],
  confirm_password: [
    { required: true, message: '请再次输入新密码', trigger: 'blur' },
    {
      validator: ((_rule: unknown, value: string, callback: (error?: Error) => void) => {
        if (value !== passwordForm.new_password) {
          callback(new Error('两次输入的密码不一致'))
        } else {
          callback()
        }
      }) as FormItemRule['validator'],
      trigger: 'blur',
    },
  ],
}

const loadUserProfile = async () => {
  try {
    const res = await getUserProfile()
    if (res.data) {
      Object.assign(profileForm, res.data)
    }
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || '加载个人信息失败')
  }
}

const beforeAvatarUpload = (file: File) => {
  const isImage = file.type.startsWith('image/')
  const isLt2M = file.size / 1024 / 1024 < 2

  if (!isImage) {
    ElMessage.error('只能上传图片文件!')
    return false
  }
  if (!isLt2M) {
    ElMessage.error('头像图片大小不能超过 2MB!')
    return false
  }
  return true
}

const handleAvatarChange = async (uploadFile: UploadFile) => {
  if (!uploadFile.raw) return

  try {
    const res = await uploadAvatar(uploadFile.raw)
    if (res.data?.avatar_url) {
      profileForm.avatar = res.data.avatar_url
      ElMessage.success('头像上传成功')
    }
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || '头像上传失败')
  }
}

const handleSaveProfile = async () => {
  if (!profileFormRef.value) return

  await profileFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return

    profileLoading.value = true
    try {
      const updateData: UserProfileUpdateRequest = {
        real_name: profileForm.real_name,
        email: profileForm.email,
        phone: profileForm.phone,
        department_id: profileForm.department_id,
        role_ids: profileForm.role_ids,
      }
      await updateUserProfile(updateData)
      ElMessage.success('个人信息保存成功')
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error((error instanceof Error ? error.message : String(error)) || '保存失败')
    } finally {
      profileLoading.value = false
    }
  })
}

const handleChangePassword = async () => {
  if (!passwordFormRef.value) return

  await passwordFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return

    passwordLoading.value = true
    try {
      await changePassword(passwordForm)
      ElMessage.success('密码修改成功')
      // 清空表单
      passwordFormRef.value?.resetFields()
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error((error instanceof Error ? error.message : String(error)) || '密码修改失败')
    } finally {
      passwordLoading.value = false
    }
  })
}

onMounted(() => {
  loadUserProfile()
})
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
