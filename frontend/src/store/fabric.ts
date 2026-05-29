import { defineStore } from 'pinia'
import { ref } from 'vue'
import { fabricApi, type Fabric, type FabricCategory, type FabricQueryParams } from '@/api/fabric'
import type { ApiResponse } from '@/types/api'

export const useFabricStore = defineStore('fabric', () => {
  const fabrics = ref<Fabric[]>([])
  const categories = ref<FabricCategory[]>([])
  const total = ref(0)
  const loading = ref(false)
  const currentFabric = ref<Fabric | null>(null)

  const fetchFabrics = async (params?: FabricQueryParams) => {
    loading.value = true
    try {
      const res = await fabricApi.list(params)
      fabrics.value = res.data!.list
      total.value = res.data!.total
    } catch (error) {
      console.error('Failed to fetch fabrics:', error)
    } finally {
      loading.value = false
    }
  }

  const fetchCategories = async () => {
    try {
      const res = await fabricApi.getCategories()
      categories.value = res.data!
    } catch (error) {
      console.error('Failed to fetch categories:', error)
    }
  }

  const getFabricById = async (id: number) => {
    try {
      const res = await fabricApi.getById(id)
      currentFabric.value = res.data!
      return res.data!
    } catch (error) {
      console.error('Failed to fetch fabric:', error)
      return null
    }
  }

  const createFabric = async (data: Partial<Fabric>): Promise<ApiResponse<Fabric> | null> => {
    try {
      const res = await fabricApi.create(data)
      await fetchFabrics()
      return res
    } catch (error) {
      console.error('Failed to create fabric:', error)
      return null
    }
  }

  const updateFabric = async (
    id: number,
    data: Partial<Fabric>
  ): Promise<ApiResponse<Fabric> | null> => {
    try {
      const res = await fabricApi.update(id, data)
      await fetchFabrics()
      return res
    } catch (error) {
      console.error('Failed to update fabric:', error)
      return null
    }
  }

  const deleteFabric = async (id: number): Promise<boolean> => {
    try {
      await fabricApi.delete(id)
      await fetchFabrics()
      return true
    } catch (error) {
      console.error('Failed to delete fabric:', error)
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
