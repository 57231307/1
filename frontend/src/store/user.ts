import { defineStore } from 'pinia'
import { ref } from 'vue'
import { login as loginApi, logout as logoutApi, getUserInfo } from '@/api/auth'
import type { UserInfo, LoginRequest } from '@/types/api'

export const useUserStore = defineStore('user', () => {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const token = ref<string | null>(null)
  const userInfo = ref<UserInfo | null>(null)

  async function login(loginData: LoginRequest) {
    const res = await loginApi(loginData)
    // Wave B-3：access_token / refresh_token 由后端写入 httpOnly Cookie，前端不再持有 token
    // FE-P-2/FE-P-3 修复：后端 LoginResponse 顶层 permissions 优先于 user.permissions
    // 批次 22 v5 P0-5：Object.freeze 防止前端组件恶意修改权限码数组
    const perms = res.permissions || res.user?.permissions || []
    userInfo.value = {
      ...(res.user || {}),
      permissions: Object.freeze([...perms]) as readonly string[],
    }
    return res
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
    // 批次 22 v5 P0-5 修复：对 permissions 字段添加 Object.freeze 运行时保护，
    // 防止前端组件恶意修改权限码数组（如 push 注入 admin:write）。
    // permissions 为 readonly 属性，通过解构创建新对象赋值，避免直接赋值类型错误。
    if (info && info.permissions) {
      userInfo.value = {
        ...info,
        permissions: Object.freeze([...info.permissions]) as readonly string[],
      }
    } else {
      userInfo.value = info
    }
    return info
  }

  function setUserInfo(info: UserInfo) {
    userInfo.value = info
  }

  return { token, userInfo, login, logout, fetchUserInfo, setUserInfo }
})
