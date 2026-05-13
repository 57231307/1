import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { UserPermission } from '@/types/api'

export const usePermissionStore = defineStore('permission', () => {
  const permissions = ref<UserPermission[]>([])

  function setPermissions(perms: UserPermission[]) {
    permissions.value = perms
  }

  function hasPermission(resource: string, action: string): boolean {
    return permissions.value.some(
      (p) => p.resource === resource && (p.action === action || p.action === '*')
    )
  }

  function hasAnyPermission(resource: string): boolean {
    return permissions.value.some((p) => p.resource === resource)
  }

  const userResources = computed(() => {
    return [...new Set(permissions.value.map((p) => p.resource))]
  })

  return { permissions, setPermissions, hasPermission, hasAnyPermission, userResources }
})
