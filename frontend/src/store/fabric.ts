import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  getFabricList,
  getFabricCategoryList,
  getFabricById as getFabricByIdApi,
  createFabric as createFabricApi,
  updateFabric as updateFabricApi,
  deleteFabric as deleteFabricApi,
  type Fabric,
  type FabricCategory,
  type FabricQueryParams,
} from '@/api/fabric'
import type { ApiResponse } from '@/types/api'
import { logger } from '@/utils/logger'

export const useFabricStore = defineStore('fabric', () => {
  const fabrics = ref<Fabric[]>([])
  const categories = ref<FabricCategory[]>([])
  const total = ref(0)
  const loading = ref(false)
  const currentFabric = ref<Fabric | null>(null)

  const fetchFabrics = async (params?: FabricQueryParams) => {
    loading.value = true
    try {
      const res = await getFabricList(params)
      // 仅在后端返回有效数据时更新，防止 data 为 null 时崩溃
      if (res.data) {
        fabrics.value = res.data.list
        total.value = res.data.total
      }
    } catch (error) {
      logger.error('获取面料列表失败:', error)
    } finally {
      loading.value = false
    }
  }

  const fetchCategories = async () => {
    try {
      const res = await getFabricCategoryList()
      // 仅在后端返回有效数据时更新，防止 data 为 null 时崩溃
      if (res.data) categories.value = res.data
    } catch (error) {
      logger.error('获取面料分类失败:', error)
    }
  }

  const getFabricById = async (id: number) => {
    try {
      const res = await getFabricByIdApi(id)
      // 仅在后端返回有效数据时更新并返回，data 为 null 时返回 null
      if (res.data) {
        currentFabric.value = res.data
        return res.data
      }
      return null
    } catch (error) {
      logger.error('获取面料详情失败:', error)
      return null
    }
  }

  const createFabric = async (data: Partial<Fabric>): Promise<ApiResponse<Fabric> | null> => {
    try {
      const res = await createFabricApi(data)
      await fetchFabrics()
      return res
    } catch (error) {
      logger.error('创建面料失败:', error)
      return null
    }
  }

  const updateFabric = async (
    id: number,
    data: Partial<Fabric>
  ): Promise<ApiResponse<Fabric> | null> => {
    try {
      const res = await updateFabricApi(id, data)
      await fetchFabrics()
      return res
    } catch (error) {
      logger.error('更新面料失败:', error)
      return null
    }
  }

  const deleteFabric = async (id: number): Promise<boolean> => {
    try {
      await deleteFabricApi(id)
      await fetchFabrics()
      return true
    } catch (error) {
      logger.error('删除面料失败:', error)
      return false
    }
  }

  return {
    fabrics,
    categories,
    total,
    loading,
    currentFabric,
    fetchFabrics,
    fetchCategories,
    getFabricById,
    createFabric,
    updateFabric,
    deleteFabric,
  }
})
