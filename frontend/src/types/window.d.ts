/// 扩展 Window 接口，声明 V2Table 性能测试采集字段
declare global {
  interface Window {
    __renderCellTotal?: import('vue').Ref<number>
  }
}

export {}
