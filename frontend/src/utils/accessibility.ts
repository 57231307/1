/**
 * 无障碍访问工具函数
 */

/**
 * 设置元素的 aria 属性
 */
export function setAriaAttribute(
  element: HTMLElement,
  attribute: string,
  value: string
): void {
  element.setAttribute(`aria-${attribute}`, value)
}

/**
 * 设置元素的 tabindex
 */
export function setTabIndex(element: HTMLElement, index: number): void {
  element.setAttribute('tabindex', String(index))
}

/**
 * 设置元素的角色
 */
export function setRole(element: HTMLElement, role: string): void {
  element.setAttribute('role', role)
}

/**
 * 设置元素的标签
 */
export function setLabel(element: HTMLElement, label: string): void {
  element.setAttribute('aria-label', label)
}

/**
 * 设置元素的描述
 */
export function setDescription(element: HTMLElement, description: string): void {
  element.setAttribute('aria-describedby', description)
}

/**
 * 设置元素的状态
 */
export function setState(
  element: HTMLElement,
  state: string,
  value: string | boolean
): void {
  element.setAttribute(`aria-${state}`, String(value))
}

/**
 * 设置元素的实时区域
 */
export function setLiveRegion(
  element: HTMLElement,
  politeness: 'polite' | 'assertive' | 'off' = 'polite'
): void {
  element.setAttribute('aria-live', politeness)
}

/**
 * 设置元素的原子性
 */
export function setAtomic(element: HTMLElement, atomic: boolean): void {
  element.setAttribute('aria-atomic', String(atomic))
}

/**
 * 设置元素的相关性
 */
export function setRelevant(
  element: HTMLElement,
  relevant: 'additions' | 'removals' | 'text' | 'all'
): void {
  element.setAttribute('aria-relevant', relevant)
}

/**
 * 设置元素的忙碌状态
 */
export function setBusy(element: HTMLElement, busy: boolean): void {
  element.setAttribute('aria-busy', String(busy))
}

/**
 * 设置元素的隐藏状态
 */
export function setHidden(element: HTMLElement, hidden: boolean): void {
  element.setAttribute('aria-hidden', String(hidden))
}

/**
 * 设置元素的展开状态
 */
export function setExpanded(element: HTMLElement, expanded: boolean): void {
  element.setAttribute('aria-expanded', String(expanded))
}

/**
 * 设置元素的选中状态
 */
export function setSelected(element: HTMLElement, selected: boolean): void {
  element.setAttribute('aria-selected', String(selected))
}

/**
 * 设置元素的按下状态
 */
export function setPressed(element: HTMLElement, pressed: boolean): void {
  element.setAttribute('aria-pressed', String(pressed))
}

/**
 * 设置元素的禁用状态
 */
export function setDisabled(element: HTMLElement, disabled: boolean): void {
  element.setAttribute('aria-disabled', String(disabled))
}

/**
 * 设置元素的必填状态
 */
export function setRequired(element: HTMLElement, required: boolean): void {
  element.setAttribute('aria-required', String(required))
}

/**
 * 设置元素的无效状态
 */
export function setInvalid(element: HTMLElement, invalid: boolean): void {
  element.setAttribute('aria-invalid', String(invalid))
}

/**
 * 设置元素的标签 ID
 */
export function setLabelledBy(element: HTMLElement, id: string): void {
  element.setAttribute('aria-labelledby', id)
}

/**
 * 设置元素的控制目标
 */
export function setControls(element: HTMLElement, id: string): void {
  element.setAttribute('aria-controls', id)
}

/**
 * 设置元素的拥有者
 */
export function setOwns(element: HTMLElement, id: string): void {
  element.setAttribute('aria-owns', id)
}

/**
 * 设置元素的活动后代
 */
export function setActivedescendant(element: HTMLElement, id: string): void {
  element.setAttribute('aria-activedescendant', id)
}

/**
 * 创建无障碍访问的按钮
 */
export function createAccessibleButton(
  text: string,
  onClick: () => void,
  options: {
    disabled?: boolean
    expanded?: boolean
    pressed?: boolean
    describedBy?: string
  } = {}
): HTMLButtonElement {
  const button = document.createElement('button')
  button.textContent = text
  button.addEventListener('click', onClick)

  setRole(button, 'button')
  setLabel(button, text)

  if (options.disabled) {
    setDisabled(button, true)
  }
  if (options.expanded !== undefined) {
    setExpanded(button, options.expanded)
  }
  if (options.pressed !== undefined) {
    setPressed(button, options.pressed)
  }
  if (options.describedBy) {
    setDescription(button, options.describedBy)
  }

  return button
}

/**
 * 创建无障碍访问的输入框
 */
export function createAccessibleInput(
  type: string,
  label: string,
  options: {
    required?: boolean
    invalid?: boolean
    describedBy?: string
    placeholder?: string
  } = {}
): HTMLInputElement {
  const input = document.createElement('input')
  input.type = type

  setLabel(input, label)

  if (options.required) {
    setRequired(input, true)
  }
  if (options.invalid) {
    setInvalid(input, true)
  }
  if (options.describedBy) {
    setDescription(input, options.describedBy)
  }
  if (options.placeholder) {
    input.placeholder = options.placeholder
  }

  return input
}

/**
 * 创建无障碍访问的表格
 */
export function createAccessibleTable(
  caption: string,
  headers: string[],
  rows: string[][]
): HTMLTableElement {
  const table = document.createElement('table')
  setRole(table, 'table')

  // 创建标题
  const captionElement = document.createElement('caption')
  captionElement.textContent = caption
  table.appendChild(captionElement)

  // 创建表头
  const thead = document.createElement('thead')
  const headerRow = document.createElement('tr')
  headers.forEach((header) => {
    const th = document.createElement('th')
    th.textContent = header
    setScope(th, 'col')
    headerRow.appendChild(th)
  })
  thead.appendChild(headerRow)
  table.appendChild(thead)

  // 创建表体
  const tbody = document.createElement('tbody')
  rows.forEach((row) => {
    const tr = document.createElement('tr')
    row.forEach((cell, index) => {
      const td = document.createElement('td')
      td.textContent = cell
      if (index === 0) {
        setScope(td, 'row')
      }
      tr.appendChild(td)
    })
    tbody.appendChild(tr)
  })
  table.appendChild(tbody)

  return table
}

/**
 * 设置元素的作用域
 */
export function setScope(
  element: HTMLElement,
  scope: 'row' | 'col' | 'rowgroup' | 'colgroup'
): void {
  element.setAttribute('scope', scope)
}

/**
 * 创建无障碍访问的列表
 */
export function createAccessibleList(
  items: string[],
  ordered: boolean = false
): HTMLUListElement | HTMLOListElement {
  const list = ordered
    ? document.createElement('ol')
    : document.createElement('ul')
  setRole(list, 'list')

  items.forEach((item) => {
    const li = document.createElement('li')
    li.textContent = item
    setRole(li, 'listitem')
    list.appendChild(li)
  })

  return list
}

/**
 * 创建无障碍访问的对话框
 */
export function createAccessibleDialog(
  title: string,
  content: string,
  options: {
    modal?: boolean
    describedBy?: string
  } = {}
): HTMLDivElement {
  const dialog = document.createElement('div')
  setRole(dialog, 'dialog')
  setLabel(dialog, title)

  if (options.modal) {
    setModal(dialog, true)
  }
  if (options.describedBy) {
    setDescription(dialog, options.describedBy)
  }

  // 创建标题
  const titleElement = document.createElement('h2')
  titleElement.textContent = title
  setRole(titleElement, 'heading')
  setState(titleElement, 'level', '2')
  dialog.appendChild(titleElement)

  // 创建内容
  const contentElement = document.createElement('p')
  contentElement.textContent = content
  dialog.appendChild(contentElement)

  return dialog
}

/**
 * 设置元素的模态状态
 */
export function setModal(element: HTMLElement, modal: boolean): void {
  element.setAttribute('aria-modal', String(modal))
}

/**
 * 创建无障碍访问的进度条
 */
export function createAccessibleProgressbar(
  label: string,
  value: number,
  max: number
): HTMLDivElement {
  const progressbar = document.createElement('div')
  setRole(progressbar, 'progressbar')
  setLabel(progressbar, label)
  setState(progressbar, 'valuenow', String(value))
  setState(progressbar, 'valuemin', '0')
  setState(progressbar, 'valuemax', String(max))

  return progressbar
}

/**
 * 创建无障碍访问的选项卡
 */
export function createAccessibleTablist(
  tabs: string[]
): HTMLDivElement {
  const tablist = document.createElement('div')
  setRole(tablist, 'tablist')

  tabs.forEach((tab, index) => {
    const tabElement = document.createElement('button')
    tabElement.textContent = tab
    setRole(tabElement, 'tab')
    setState(tabElement, 'selected', index === 0)
    setTabIndex(tabElement, index === 0 ? 0 : -1)
    tablist.appendChild(tabElement)
  })

  return tablist
}

/**
 * 创建无障碍访问的警告框
 */
export function createAccessibleAlert(
  message: string,
  type: 'info' | 'success' | 'warning' | 'error' = 'info'
): HTMLDivElement {
  const alert = document.createElement('div')
  setRole(alert, 'alert')
  setLiveRegion(alert, 'assertive')
  setAtomic(alert, true)

  const icon = document.createElement('span')
  icon.setAttribute('aria-hidden', 'true')
  switch (type) {
    case 'info':
      icon.textContent = 'ℹ️'
      break
    case 'success':
      icon.textContent = '✅'
      break
    case 'warning':
      icon.textContent = '⚠️'
      break
    case 'error':
      icon.textContent = '❌'
      break
  }
  alert.appendChild(icon)

  const messageElement = document.createElement('span')
  messageElement.textContent = message
  alert.appendChild(messageElement)

  return alert
}
