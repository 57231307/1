import { defineStore } from 'pinia'
import { ref } from 'vue'
import { login as loginApi, logout as logoutApi } from '@/api/auth'
import { setToken, removeToken, setRefreshToken } from '@/utils/storage'
import type { UserInfo, LoginRequest } from '@/types/api'

export const useUserStore = defineStore('user', () => {
  const token = ref<string | null>(null)
  const userInfo = ref<UserInfo | null>(null)

  async function login(loginData: LoginRequest) {
    const res = await loginApi(loginData) as any
    // 后端返回 {code, data: {token, refresh_token, user, ...}, message}
    const responseData = res.data || res
    token.value = responseData.token
    setToken(responseData.token)
    if (responseData.refresh_token) {
      setRefreshToken(responseData.refresh_token)
    }
    userInfo.value = responseData.user
    return responseData
  }

  async function logout() {
    try {
      await logoutApi()
    } finally {
      token.value = null
      userInfo.value = null
      removeToken()
    }
  }

  function setUserInfo(info: UserInfo) {
    userInfo.value = info
  }

  return { token, userInfo, login, logout, setUserInfo }
})
