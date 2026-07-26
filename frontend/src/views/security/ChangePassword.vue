<!--
  修改密码页面
  - 表单：当前密码 / 新密码 / 确认新密码
  - 复用 PasswordStrengthMeter 组件实时显示新密码强度
  - 调 POST /api/v1/erp/users/change-password
  - 校验：新密码不能与旧密码相同、确认密码需一致、必须满足后端策略（≥8 字符 + 大小写数字特殊字符）
  批次 D05 B4：接入 useI18n
-->
<template>
  <div class="change-pwd-page">
    <el-card class="change-pwd-card" shadow="hover">
      <template #header>
        <div class="card-header">
          <span>{{ t('security.changePassword.title') }}</span>
        </div>
      </template>

      <el-alert
        v-if="successTip"
        :title="t('security.changePassword.successAlert.title')"
        type="success"
        :description="t('security.changePassword.successAlert.description')"
        :closable="false"
        show-icon
        class="success-tip"
      />

      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="100px"
        class="change-pwd-form"
        :aria-label="t('security.changePassword.ariaLabel')"
      >
        <el-form-item :label="t('security.changePassword.form.oldPassword')" prop="old_password">
          <el-input
            v-model="form.old_password"
            type="password"
            :placeholder="t('security.changePassword.placeholder.oldPassword')"
            show-password
            autocomplete="current-password"
          />
        </el-form-item>

        <el-form-item :label="t('security.changePassword.form.newPassword')" prop="new_password">
          <el-input
            v-model="form.new_password"
            type="password"
            :placeholder="t('security.changePassword.placeholder.newPassword')"
            show-password
            autocomplete="new-password"
          />
          <!-- 实时密码强度可视化（PasswordStrengthMeter 用 defineProps 接收 password，单向绑定即可） -->
          <PasswordStrengthMeter
            :password="form.new_password"
            class="strength-meter"
          />
        </el-form-item>

        <el-form-item :label="t('security.changePassword.form.confirmPassword')" prop="confirm_password">
          <el-input
            v-model="form.confirm_password"
            type="password"
            :placeholder="t('security.changePassword.placeholder.confirmPassword')"
            show-password
            autocomplete="new-password"
          />
        </el-form-item>

        <el-form-item label=" ">
          <el-button
            type="primary"
            :loading="submitting"
            @click="handleSubmit"
          >
            {{ t('security.changePassword.button.submit') }}
          </el-button>
          <el-button @click="handleReset">{{ t('security.changePassword.button.reset') }}</el-button>
        </el-form-item>
      </el-form>

      <el-divider />

      <div class="pwd-tips">
        <div class="tips-title">{{ t('security.changePassword.tips.title') }}</div>
        <ul>
          <li>{{ t('security.changePassword.tips.length') }}</li>
          <li>{{ t('security.changePassword.tips.uppercase') }}</li>
          <li>{{ t('security.changePassword.tips.lowercase') }}</li>
          <li>{{ t('security.changePassword.tips.digit') }}</li>
          <li>{{ t('security.changePassword.tips.special') }}</li>
          <li>{{ t('security.changePassword.tips.notSame') }}</li>
        </ul>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { changePassword } from '@/api/user'
import PasswordStrengthMeter from '@/components/PasswordStrengthMeter.vue'

const { t } = useI18n({ useScope: 'global' })

interface ChangePwdForm {
  old_password: string
  new_password: string
  confirm_password: string
}

const formRef = ref<FormInstance>()
const submitting = ref(false)
const successTip = ref(false)

const form = reactive<ChangePwdForm>({
  old_password: '',
  new_password: '',
  confirm_password: '',
})

/** 密码策略校验（与后端 password_validator 默认策略对齐） */
const validatePasswordPolicy = (_rule: unknown, value: string, callback: (err?: Error) => void) => {
  if (!value) {
    callback(new Error(t('security.changePassword.message.newPasswordRequired')))
    return
  }
  if (value.length < 8) {
    callback(new Error(t('security.changePassword.message.lengthMin')))
    return
  }
  if (value.length > 128) {
    callback(new Error(t('security.changePassword.message.lengthMax')))
    return
  }
  if (!/[A-Z]/.test(value)) {
    callback(new Error(t('security.changePassword.message.uppercase')))
    return
  }
  if (!/[a-z]/.test(value)) {
    callback(new Error(t('security.changePassword.message.lowercase')))
    return
  }
  if (!/[0-9]/.test(value)) {
    callback(new Error(t('security.changePassword.message.digit')))
    return
  }
  if (!/[^A-Za-z0-9]/.test(value)) {
    callback(new Error(t('security.changePassword.message.special')))
    return
  }
  if (value === form.old_password) {
    callback(new Error(t('security.changePassword.message.sameAsOld')))
    return
  }
  callback()
}

/** 确认密码校验 */
const validateConfirm = (_rule: unknown, value: string, callback: (err?: Error) => void) => {
  if (!value) {
    callback(new Error(t('security.changePassword.message.confirmRequired')))
    return
  }
  if (value !== form.new_password) {
    callback(new Error(t('security.changePassword.message.confirmMismatch')))
    return
  }
  callback()
}

const rules: FormRules = {
  old_password: [{ required: true, message: t('security.changePassword.message.oldPasswordRequired'), trigger: 'blur' }],
  new_password: [{ validator: validatePasswordPolicy, trigger: 'blur' }],
  confirm_password: [{ validator: validateConfirm, trigger: 'blur' }],
}

/** 提交修改密码 */
const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    submitting.value = true
    successTip.value = false
    try {
      await changePassword({
        old_password: form.old_password,
        new_password: form.new_password,
      })
      ElMessage.success(t('security.changePassword.message.success'))
      successTip.value = true
      // 清空表单（确认密码同步清空）
      formRef.value?.resetFields()
      form.old_password = ''
      form.new_password = ''
      form.confirm_password = ''
    } catch (error) {
      // request.ts 拦截器已经显示 ElMessage.error，这里不再重复提示
      void error
    } finally {
      submitting.value = false
    }
  })
}

/** 重置表单 */
const handleReset = () => {
  formRef.value?.resetFields()
  form.old_password = ''
  form.new_password = ''
  form.confirm_password = ''
  successTip.value = false
}
</script>

<style scoped>
.change-pwd-page {
  padding: 20px;
  max-width: 720px;
  margin: 0 auto;
}

.change-pwd-card {
  width: 100%;
}

.card-header {
  font-weight: 600;
  font-size: 16px;
}

.success-tip {
  margin-bottom: 16px;
}

.change-pwd-form {
  max-width: 520px;
}

.strength-meter {
  margin-top: 4px;
}

.pwd-tips {
  background: #f5f7fa;
  border-radius: 6px;
  padding: 12px 16px;
  font-size: 13px;
  color: #606266;
}

.tips-title {
  font-weight: 600;
  margin-bottom: 6px;
}

.pwd-tips ul {
  margin: 0;
  padding-left: 20px;
  line-height: 1.8;
}
</style>
