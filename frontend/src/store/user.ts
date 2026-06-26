import { defineStore } from 'pinia'
import { ref } from 'vue'
import { login as loginApi, logout as logoutApi, getUserInfo } from '@/api/auth'
import type { UserInfo, LoginRequest } from '@/types/api'

export const useUserStore = defineStore('user', () => {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const token = ref<string | null>(null)
  const userInfo = ref<UserInfo | null>(null)

  async function login(loginData: LoginRequest) {
    const res = (await loginApi(loginData)) as any
    // 后端返回 {code, data: {user, permissions, ...}, message}
    // Wave B-3：access_token / refresh_token 由后端写入 httpOnly Cookie，前端不再持有 token
    const responseData = res.data || res
    // 兼容：如果后端仍返回 token 字段，本地保留以避免破坏 userStore.token 引用方
    if (responseData.token) {
      token.value = responseData.token
    }
    // FE-P-2/FE-P-3 修复（2026-06-26 第二次审计第二优先级）：
    // 后端 LoginResponse 顶层有 `permissions: Vec<String>` 字段（格式 "{resource}:{action}"），
    // 但原实现只取 `responseData.user`，丢弃了顶层 permissions，导致 v-permission 指令
    // 读 user.permissions 永远是 undefined。合并到 userInfo 使其生效。
    userInfo.value = {
      ...(responseData.user || {}),
      permissions: responseData.permissions || responseData.user?.permissions,
    }
    return responseData
  }

  async function logout() {
    try {
      await logoutApi()
    } finally {
      // 后端通过 Set-Cookie + max-age=0 自动清除所有登录态 Cookie，前端无需清 localStorage
      token.value = null
      userInfo.value = null
    }
  }

  async function fetchUserInfo() {
    const info = await getUserInfo()
    userInfo.value = info
    return info
  }

  function setUserInfo(info: UserInfo) {
    userInfo.value = info
  }

  return { token, userInfo, login, logout, fetchUserInfo, setUserInfo }
})
