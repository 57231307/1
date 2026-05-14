import axios from 'axios'
import type { AxiosInstance, AxiosRequestConfig, AxiosResponse } from 'axios'
import { ElMessage } from 'element-plus'
import { getToken, removeToken } from '@/utils/storage'
import router from '@/router'

export interface ApiResponse<T = any> {
  code: number
  message: string
  data: T
}

export interface PageResult<T = any> {
  list: T[]
  total: number
  page: number
  page_size: number
}

class Request {
  private instance: AxiosInstance

  constructor() {
    this.instance = axios.create({
      baseURL: import.meta.env.VITE_API_BASE_URL || '/api/v1/erp',
      timeout: 30000,
      headers: {
        'Content-Type': 'application/json',
      },
    })

    this.setupInterceptors()
  }

  private setupInterceptors() {
    this.instance.interceptors.request.use(
      (config) => {
        const token = getToken()
        if (token) {
          config.headers.Authorization = `Bearer ${token}`
        }
        return config
      },
      (error) => {
        return Promise.reject(error)
      }
    )

    this.instance.interceptors.response.use(
      (response: AxiosResponse<ApiResponse>) => {
        const res = response.data
        if (res.code !== 200 && res.code !== 0) {
          ElMessage.error(res.message || '请求失败')
          if (res.code === 401) {
            removeToken()
            router.push('/login')
          }
          return Promise.reject(new Error(res.message || '请求失败'))
        }
        return response
      },
      async (error) => {
        const originalRequest = error.config
        
        if (shouldRetry(error) && originalRequest && !originalRequest._retry) {
          originalRequest._retry = true
          originalRequest._retryCount = originalRequest._retryCount || 0
          
          if (originalRequest._retryCount < 3) {
            originalRequest._retryCount++
            const delay = Math.pow(2, originalRequest._retryCount) * 1000
            await new Promise(resolve => setTimeout(resolve, delay))
            
            console.log(`请求重试 ${originalRequest._retryCount}/3:`, originalRequest.url)
            return this.instance(originalRequest)
          }
        }
        
        const message = error.response?.data?.message || error.message || '网络错误'
        ElMessage.error(message)
        if (error.response?.status === 401) {
          removeToken()
          router.push('/login')
        }
        return Promise.reject(error)
      }
    )
  }

  public get<T = any>(url: string, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.get(url, config).then((res) => res.data)
  }

  public post<T = any>(url: string, data?: any, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.post(url, data, config).then((res) => res.data)
  }

  public put<T = any>(url: string, data?: any, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.put(url, data, config).then((res) => res.data)
  }

  public delete<T = any>(url: string, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.delete(url, config).then((res) => res.data)
  }

  public patch<T = any>(url: string, data?: any, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.patch(url, data, config).then((res) => res.data)
  }
}

function shouldRetry(error: any): boolean {
  if (error.response) {
    return [502, 503, 504].includes(error.response.status)
  }
  return error.code === 'ECONNABORTED' || error.code === 'NETWORK_ERROR' || !error.response
}

export const request = new Request()
