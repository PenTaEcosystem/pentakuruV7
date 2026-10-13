export const UI_SCALE = 0.8
export function createCompactUI(api) {
  const geometry = ['x','y','w','h','text_size','item_height','item_space','radius']
  const offset = { x:43, y:51 }
  function scale(value, root = false) {
    if (Array.isArray(value)) return value.map(item => scale(item))
    if (!value || typeof value !== 'object') return value
    const result = {}
    Object.keys(value).forEach(key => {
      const item = value[key]
      if (geometry.indexOf(key) >= 0 && typeof item === 'number') result[key] = Math.round(item*UI_SCALE) + (root && offset[key] || 0)
      else if ((key === 'normal_src' || key === 'press_src') && typeof item === 'string') result[key] = item.indexOf('compact/') === 0 ? item : 'compact/'+item
      else result[key] = scale(item)
    })
    return result
  }
  return {
    createWidget(type, config) {
      const native = api.createWidget(type, scale(config, true))
      return { native, setProperty(property, value) { native.setProperty(property, scale(value, property === api.prop.MORE)) } }
    },
    deleteWidget(item) { api.deleteWidget(item.native || item) }
  }
}
