import { BaseSideService } from '@zeppos/zml/base-side'
import { createPCClient, settingUrl, DEFAULT_URLS, MAX_AUDIO_BYTES } from '../common/transport'

AppSideService(BaseSideService({
  onInit() {
    this.pendingVoice = null
    this.client = createPCClient(options => fetch(options), () => [
      settingUrl(this.settings.getItem('pc_url'), DEFAULT_URLS[0]),
      settingUrl(this.settings.getItem('lan_url'), DEFAULT_URLS[1])
    ].filter((url, i, all) => all.indexOf(url) === i))
  },
  onSettingsChange() { if (this.client) this.client.reset() },
  controlKey() {
    let value = this.settings.getItem('control_key') || ''
    try { value = JSON.parse(value) } catch (_) {}
    if (typeof value !== 'string' || !value.trim()) throw new Error('Dán mã ghép Windows từ Cài đặt PentaKuru vào cài đặt ứng dụng trong Zepp.')
    return value.trim()
  },
  async onRequest(req, res) {
    const params = req.params || {}
    try {
      let data
      switch (req.method || req.action) {
        case 'GET_STATUS': data = await this.client.connect(); break
        case 'GET_NOTES': data = await this.client.request('/api/notes'); break
        case 'SEARCH_NOTES': data = await this.client.request('/api/search', { method: 'POST', body: JSON.stringify({ query: params.query || '' }) }); break
        case 'ASK_AI': data = await this.client.request('/api/ai/ask', { method: 'POST', body: JSON.stringify({ prompt: params.prompt || '' }) }); break
        case 'SAVE_NOTE': data = await this.client.request('/api/notes', { method: 'POST', body: JSON.stringify({ title: 'Ghi chú từ đồng hồ', content: params.text || '' }) }); break
        case 'WIN_EXECUTE': data = await this.client.request('/api/windows/execute', { method: 'POST', body: JSON.stringify({ command: params.text || '', control_key: this.controlKey() }) }); break
        case 'WIN_STATUS': data = await this.client.request('/api/windows/status', { method: 'POST', body: JSON.stringify({ id: params.id, control_key: this.controlKey() }) }); break
        case 'WIN_CANCEL': data = await this.client.request('/api/windows/cancel', { method: 'POST', body: JSON.stringify({ id: params.id, control_key: this.controlKey() }) }); break
        case 'CHECK_WINDOWS':
          this.controlKey(); data = await this.client.connect()
          if (!data.windows_control) throw new Error('Cập nhật PentaKuru PC lên bản hỗ trợ lệnh Windows.')
          if (!data.windows_ready) throw new Error('Cấu hình AI hoặc UI-TARS trên PentaKuru PC trước khi gửi lệnh Windows.')
          break
        case 'VOICE_BEGIN': {
          if (params.action === 'execute') this.controlKey()
          if (!Number.isInteger(params.bytes) || params.bytes <= 0 || params.bytes > MAX_AUDIO_BYTES) throw new Error('Kích thước file ghi âm không hợp lệ')
          this.pendingVoice = { id: params.id, bytes: params.bytes, chunks: [], length: 0, started: Date.now(), action: params.action, mime: params.mime_type }
          data = { status: 'ok' }; break
        }
        case 'VOICE_CHUNK': {
          const pending = this.pendingVoice
          if (!pending || pending.id !== params.id || Date.now() - pending.started > 180000) throw new Error('Phiên gửi âm thanh đã hết hạn')
          const chunk = params.chunk
          if (params.index !== pending.chunks.length || typeof chunk !== 'string' || chunk.length > 8192 || !/^[A-Za-z0-9+/]*={0,2}$/.test(chunk)) throw new Error('Gói âm thanh sai thứ tự hoặc dữ liệu lỗi')
          pending.length += chunk.length
          if (pending.length > Math.ceil(MAX_AUDIO_BYTES / 3) * 4) { this.pendingVoice = null; throw new Error('Đoạn ghi âm quá lớn') }
          pending.chunks.push(chunk); data = { status: 'ok' }; break
        }
        case 'VOICE_FINISH': {
          const pending = this.pendingVoice
          this.pendingVoice = null
          if (!pending || pending.id !== params.id || pending.length !== Math.ceil(pending.bytes / 3) * 4 || Date.now() - pending.started > 180000) throw new Error('Âm thanh chưa gửi đủ; hãy ghi lại')
          data = await this.client.request('/api/voice/stt', { method: 'POST', body: JSON.stringify({ audio_base64: pending.chunks.join(''), mime_type: pending.mime || 'audio/opus', action: pending.action || 'ask', control_key: pending.action === 'execute' ? this.controlKey() : undefined }) }); break
        }
        case 'VOICE_STT': data = await this.client.request('/api/voice/stt', { method: 'POST', body: JSON.stringify({ ...params, control_key: params.action === 'execute' ? this.controlKey() : undefined }) }); break
        default: throw new Error('Lệnh đồng hồ không được hỗ trợ')
      }
      res(null, data)
    } catch (error) { res(null, { status: 'error', error: error.message || String(error) }) }
  },
  onDestroy() { this.pendingVoice = null }
}))
