export const DEFAULT_URLS = ['http://100.115.158.11:3899', 'http://192.168.100.80:3899']
export const MAX_AUDIO_BYTES = 96 * 1024
export function settingUrl(value, fallback) {
  let text = value || fallback
  try { text = JSON.parse(text) } catch (_) {}
  if (typeof text !== 'string' || !/^https?:\/\/[^\s]+$/.test(text.trim())) throw new Error('URL PC không hợp lệ trong cài đặt Zepp')
  return text.trim().replace(/\/+$/, '')
}
export function responseBody(response) {
  const data = typeof response.body === 'string' ? JSON.parse(response.body) : response.body
  if (!data || typeof data !== 'object') throw new Error('PC trả dữ liệu không đúng định dạng')
  if ((response.status && response.status >= 400) || data.status === 'error' || data.error) {
    const error = new Error(data.error || `HTTP ${response.status}`)
    error.fromPC = true
    throw error
  }
  return data
}
export function timeout(promise, ms, schedule = setTimeout, cancel = clearTimeout) {
  return new Promise((resolve, reject) => {
    const timer = schedule(() => reject(new Error('Hết thời gian chờ PC; kiểm tra Zepp, Wi-Fi/Tailscale và Penta')), ms)
    Promise.resolve(promise).then(value => { cancel(timer); resolve(value) }, error => { cancel(timer); reject(error) })
  })
}
export function createPCClient(fetcher, urls, withTimeout = timeout) {
  let connectedURL = ''
  return {
    async connect() {
      let lastError
      for (const url of urls()) {
        try {
          const data = responseBody(await withTimeout(fetcher({ url: `${url}/api/status`, method: 'GET' }), 5000))
          if (data.app !== 'PentaKuru' || data.status !== 'ok') throw new Error('Địa chỉ không phải máy chủ PentaKuru')
          connectedURL = url
          return { ...data, connected_url: url }
        } catch (error) { lastError = error }
      }
      connectedURL = ''
      throw lastError || new Error('Chưa cấu hình URL PC')
    },
    async request(path, options = {}) {
      if (!connectedURL) await this.connect()
      try {
        return responseBody(await withTimeout(fetcher({ url: `${connectedURL}${path}`, method: options.method || 'GET', headers: { 'Content-Type': 'application/json' }, body: options.body }), 75000))
      } catch (error) {
        if (!error.fromPC) connectedURL = ''
        throw error // Never retry POST: it may already have saved a note.
      }
    },
    reset() { connectedURL = '' }
  }
}
export function uint8ToBase64(bytes) {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'
  let result = ''
  for (let i = 0; i < bytes.length; i += 3) {
    const a = bytes[i], b = bytes[i + 1] || 0, c = bytes[i + 2] || 0
    result += chars[a >> 2] + chars[((a & 3) << 4) | (b >> 4)]
    result += i + 1 < bytes.length ? chars[((b & 15) << 2) | (c >> 6)] : '='
    result += i + 2 < bytes.length ? chars[c & 63] : '='
  }
  return result
}
export function textRows(text, width = 27) {
  const rows = []
  for (const paragraph of String(text || '').split('\n')) {
    let line = ''
    for (const word of paragraph.split(/\s+/).filter(Boolean)) {
      if (line && Array.from(line + ' ' + word).length > width) { rows.push({ line }); line = '' }
      const chars = Array.from(word)
      while (chars.length > width) {
        if (line) { rows.push({ line }); line = '' }
        rows.push({ line: chars.splice(0, width).join('') })
      }
      const rest = chars.join('')
      line = line ? `${line} ${rest}` : rest
    }
    rows.push({ line: line || ' ' })
  }
  return rows
}
