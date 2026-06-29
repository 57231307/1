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
    // 批次 24 v6 P0-1 修复：删除 if (responseData.token) 死代码分支。
    // 后端 LoginResponse 不再返回 token 字段（access_token 通过 httpOnly Cookie 写入），
    // 此分支永远为 false，保留会误导开发者以为 token 还在响应体中。
    // FE-P-2/FE-P-3 修复（2026-06-26 第二次审计第二优先级）：
    // 后端 LoginResponse 顶层有 `permissions: Vec<String>` 字段（格式 "{resource}:{action}"），
    // 但原实现只取 `responseData.user`，丢弃了顶层 permissions，导致 v-permission 指令
    // 读 user.permissions 永远是 undefined。合并到 userInfo 使其生效。
    //
    // 批次 22 v5 P0-5 修复：对 permissions 字段添加 Object.freeze 运行时保护，
    // 防止前端组件恶意修改权限码数组（如 push 注入 admin:write）。
    // [...perms] 创建副本避免冻结后端原始数组，readonly 类型与 api.ts 对齐。
    //
    // 批次 24 v6 P0-2 修复：后端 UserInfo 已补全 role_name 和 permissions 字段，
    // 此处合并逻辑保留顶层 permissions 优先级（兼容历史行为）。
    const perms = responseData.permissions || responseData.user?.permissions || []
    userInfo.value = {
      ...(responseData.user || {}),
      permissions: Object.freeze([...perms]) as readonly string[],
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
