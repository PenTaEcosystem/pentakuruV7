import { BasePage } from '@zeppos/zml/base-page'
import { createWidget as nativeCreateWidget, deleteWidget as nativeDeleteWidget, setStatusBarVisible, widget, align, prop } from '@zos/ui'
import { push } from '@zos/router'
import { create, id, codec } from '@zos/media'
import { getDeviceInfo } from '@zos/device'
import * as fs from '@zos/fs'
import { createVoiceSession } from '../common/voice-session'
import { createMicrophone } from '../common/microphone'
import { normalizeOpus } from '../common/opus'
import { createChatState } from '../common/chat-state'
import { createCompactUI } from '../common/compact-ui'
import { MAX_AUDIO_BYTES, uint8ToBase64, textRows } from '../common/transport'

const W = 432
const compact = createCompactUI({ createWidget: nativeCreateWidget, deleteWidget: nativeDeleteWidget, prop })
const createWidget = compact.createWidget
const LABELS = ['Ghi chú', 'Tìm kiếm', 'Trợ lý AI', 'Windows']
const ACTIONS = ['note', 'search', 'ask', 'execute']

Page(BasePage({
  state: { activeTab: 2, dockBtns: [], destroyed: false, voice: null, connected: false, requestBusy: false, result: null, taskId: '', taskQuestion: '', pollTimer: null, keyboardOpen: false },
  onInit() {
    this.state.destroyed = false
    this.state.voice = null
    this.state.requestBusy = false
    this.state.keyboardOpen = false
    this.state.chat = this.globalData.watchChat || createChatState()
    if (typeof this.globalData.watchTab === 'number') this.state.activeTab = this.globalData.watchTab
    try { this.state.deviceSource = getDeviceInfo().deviceSource } catch (_) {}
  },
  onResume() {
    this.state.keyboardOpen = false
    const text = this.globalData.inputResult
    delete this.globalData.inputResult
    if (typeof text === 'string' && text.trim()) this.sendText(text)
  },
  ensureVoice() {
    if (this.state.voice) return true
    try {
      this.state.voice = createMicrophone({ create, id, codec }, createVoiceSession, {
        legacy: { readSize: () => fs.statSync({ path: 'data://voice.opus' }).size },
        prepareFile: () => { try { fs.rmSync({ path: 'data://voice.opus' }) } catch (_) {} },
        onState: phase => this.voiceState(phase),
        onReady: () => this.sendVoiceToPC(),
        onError: message => this.showResult(message, true)
      })
      this.state.micError = ''
      return true
    } catch (error) {
      this.state.micError = `Không mở được mic: ${error.message || error}\nThiết bị: ${this.state.deviceSource || 'không xác định'}\nĐóng ứng dụng dùng mic rồi chạm mic để thử lại.`
      this.showResult(this.state.micError, true)
      return false
    }
  },
  build() {
    try { setStatusBarVisible(false) } catch (_) {}
    createWidget(widget.TEXT, { x: 28, y: 24, w: 320, h: 32, text: 'PENTAKURU · 1.2.3', text_size: 24, color: 0xffffff })
    createWidget(widget.BUTTON, { x: 364, y: 24, w: 40, h: 40, normal_src: 'chat_refresh.png', press_src: 'chat_refresh.png', click_func: () => this.checkConnection() })
    this.state.statusText = createWidget(widget.TEXT, { x: 28, y: 60, w: 376, h: 26, text: 'Đang kết nối PC...', text_size: 18, color: 0xaaaaaa })
    const rows = textRows(this.state.chat.text())
    this.state.result = createWidget(widget.SCROLL_LIST, {
      x: 20, y: 96, w: 392, h: 252, item_space: 0, item_config_count: 1,
      item_config: [{ type_id: 1, item_height: 32, item_bg_color: 0x121418, item_bg_radius: 0, text_view: [{ x: 12, y: 0, w: 368, h: 32, text_size: 22, color: 0xffffff, key: 'line' }], text_view_count: 1 }],
      data_array: rows, data_count: rows.length
    })
    this.state.dockBtns = []
    for (let i = 0; i < 4; i++) this.state.dockBtns.push(createWidget(widget.BUTTON, {
      x: 32 + i * 96, y: 360, w: 80, h: 54,
      normal_src: `chat_${i}_${i === this.state.activeTab ? 'act' : 'norm'}.png`, press_src: `chat_${i}_act.png`, click_func: () => this.switchTab(i)
    }))
    createWidget(widget.BUTTON, { x: 24, y: 426, w: 260, h: 56, normal_src: 'chat_keyboard_button.png', press_src: 'chat_keyboard_pressed.png', click_func: () => this.openInput() })
    this.state.micBtn = createWidget(widget.BUTTON, { x: 296, y: 426, w: 112, h: 56, normal_src: 'chat_mic_button.png', press_src: 'chat_mic_pressed.png', click_func: () => this.toggleListening() })
    this.checkConnection()
    this.onResume()
  },
  setStatus(text) {
    if (!this.state.destroyed && this.state.statusText) this.state.statusText.setProperty(prop.TEXT, text)
  },
  renderChat() {
    if (this.state.destroyed || !this.state.result) return
    const rows = textRows(this.state.chat.text())
    this.state.result.setProperty(prop.UPDATE_DATA, { data_array: rows, data_count: rows.length, on_page: 0 })
  },
  showResult(text, error = false, question = LABELS[this.state.activeTab]) {
    if (!this.state.chat) this.state.chat = createChatState()
    if (error) { this.state.chat.reject(text); this.setStatus('Lỗi · kéo để đọc chi tiết') }
    else { this.state.chat.accept(question, text); this.setStatus('✓ Xong · kéo để đọc') }
    this.renderChat()
  },
  busy() { return this.state.keyboardOpen || this.state.requestBusy || !!this.state.taskId || (this.state.voice && this.state.voice.phase !== 'idle') },
  async pcRequest(method, params = {}, ms = 95000) {
    const response = await this.request({ method, params }, { timeout: ms })
    if (!response || response.status !== 'ok') throw new Error(response && response.error || 'Zepp chưa kết nối Penta trên PC')
    return response
  },
  async checkConnection() {
    if (this.busy() || this.state.destroyed) return
    try {
      await this.pcRequest('GET_STATUS', {}, 16000)
      this.state.connected = true
      this.setStatus(`${LABELS[this.state.activeTab]} · PC đã kết nối`)
    } catch (error) { this.state.connected = false; this.showResult(`Mở Zepp, bật Bluetooth và kiểm tra URL PC. ${error.message}`, true) }
  },
  onBleChanged() {
    if (this.state.taskId) this.schedulePoll(0)
    else this.checkConnection()
  },
  async switchTab(index) {
    if (this.busy()) return
    this.state.activeTab = index
    this.state.dockBtns.forEach((button, i) => button.setProperty(prop.MORE, { x: 32+i*96, y: 360, w: 80, h: 54, normal_src: `chat_${i}_${i === index ? 'act' : 'norm'}.png`, press_src: `chat_${i}_act.png` }))
    this.setStatus(`${LABELS[index]} · nhập hoặc nói`)
    if (index !== 0) return
    this.state.requestBusy = true
    this.state.chat.begin()
    try {
      const data = await this.pcRequest('GET_NOTES')
      this.receiveResult(data, 'Xem ghi chú')
    } catch (error) { this.showResult(error.message, true) }
    finally { this.state.requestBusy = false }
  },
  openInput() {
    if (this.busy()) { this.setStatus('Đang xử lý · chờ xong để nhập'); return }
    this.globalData.watchChat = this.state.chat
    this.globalData.watchTab = this.state.activeTab
    this.state.keyboardOpen = true
    try {
      this.setStatus('Đang mở bàn phím...')
      push({ url: 'page/keyboard' })
    } catch (error) { this.state.keyboardOpen = false; this.showResult(`Không mở được bàn phím: ${error.message}`, true) }
  },
  async sendText(text) {
    text = String(text).trim()
    if (!text || this.busy() || this.state.destroyed) return
    this.state.requestBusy = true
    this.state.chat.begin()
    this.setStatus(this.state.activeTab === 3 ? 'Đang gửi lệnh Windows...' : 'Penta đang xử lý...')
    try {
      const method = ['SAVE_NOTE', 'SEARCH_NOTES', 'ASK_AI', 'WIN_EXECUTE'][this.state.activeTab]
      const data = await this.pcRequest(method, { text, query: text, prompt: text })
      this.receiveResult(data, text)
    } catch (error) { this.showResult(error.message, true) }
    finally { this.state.requestBusy = false }
  },
  receiveResult(data, question) {
    if (this.state.destroyed) return
    if (data.task) {
      this.state.taskId = data.task.id
      this.state.taskQuestion = data.task.goal || question
      this.setStatus('Windows · đang thực thi...')
      this.updateMic()
      this.schedulePoll(1000)
      return
    }
    const answer = data.notes ? data.notes.map(note => `${note.title}\n${note.content}`).join('\n\n') || 'Không tìm thấy ghi chú.' : data.reply || data.message || (this.state.activeTab === 0 ? 'Đã lưu ghi chú trên PC.' : '')
    this.showResult(answer || 'PC trả về câu trả lời trống; hãy thử lại.', !answer, question)
  },
  updateMic() {
    if (!this.state.destroyed && this.state.micBtn) {
      const stopping = this.state.taskId || (this.state.voice && this.state.voice.phase === 'recording')
      this.state.micBtn.setProperty(prop.MORE, { x:296, y:426, w:112, h:56, normal_src: stopping ? 'chat_stop_button.png' : 'chat_mic_button.png', press_src: stopping ? 'chat_stop_button.png' : 'chat_mic_pressed.png' })
    }
  },
  schedulePoll(ms) {
    if (this.state.pollTimer !== null) clearTimeout(this.state.pollTimer)
    if (this.state.destroyed || !this.state.taskId) return
    this.state.pollTimer = setTimeout(() => { this.state.pollTimer = null; this.pollTask() }, ms)
  },
  async pollTask() {
    if (this.state.destroyed || !this.state.taskId || this.state.pollBusy) return
    this.state.pollBusy = true
    let retry = 2000
    try {
      const data = await this.pcRequest('WIN_STATUS', { id: this.state.taskId }, 20000)
      this.handleTask(data.task)
    } catch (error) { retry = 5000; this.pollError(error) }
    finally { this.state.pollBusy = false; this.schedulePoll(retry) }
  },
  pollError(error) {
    if (String(error.message).indexOf('Không tìm thấy tác vụ') >= 0) {
      this.state.taskId = ''
      this.updateMic()
      this.showResult('PC đã khởi động lại hoặc tác vụ không còn được lưu. Kiểm tra kết quả trên PC trước khi gửi lệnh mới.', true)
    } else this.setStatus('Mất kết nối · đang thử lại...')
  },
  handleTask(task) {
    if (this.state.destroyed || !task || task.id !== this.state.taskId) return
    if (task.phase === 'pending' || task.phase === 'running') { this.setStatus(task.message || 'Windows · đang thực thi...'); return }
    this.state.taskId = ''
    this.updateMic()
    this.showResult(task.message || 'Tác vụ đã kết thúc', task.phase !== 'completed', this.state.taskQuestion)
  },
  async stopWindows() {
    if (this.state.cancelBusy) return
    this.state.cancelBusy = true
    this.setStatus('Đang gửi yêu cầu Dừng...')
    try { const data = await this.pcRequest('WIN_CANCEL', { id: this.state.taskId }, 20000); this.handleTask(data.task) }
    catch (error) { this.setStatus('Chưa gửi được Dừng · chạm lại') }
    finally { this.state.cancelBusy = false; this.schedulePoll(1000) }
  },
  voiceState(phase) {
    if (this.state.destroyed) return
    this.updateMic()
    const messages = { starting: 'Đang mở mic...', recording: '● Đang thu · chạm để gửi', stopping: 'Đang đóng file âm thanh...', sending: 'Đang gửi và xử lý...' }
    if (messages[phase]) this.setStatus(messages[phase])
  },
  async toggleListening() {
    if (this.state.taskId) { this.stopWindows(); return }
    if (this.state.requestBusy || this.state.keyboardOpen) return
    if (!this.ensureVoice()) return
    if (this.state.voice.phase === 'recording') { this.state.voice.stop(); return }
    if (this.state.voice.phase !== 'idle') return
    this.state.requestBusy = true
    try {
      this.setStatus('Đang kiểm tra PC...')
      await this.pcRequest(this.state.activeTab === 3 ? 'CHECK_WINDOWS' : 'GET_STATUS', {}, 16000)
      if (!this.state.destroyed) { this.state.chat.begin(); this.state.voice.start() }
    } catch (error) { this.showResult(error.message, true) }
    finally { this.state.requestBusy = false }
  },
  async sendVoiceToPC() {
    if (this.state.destroyed) return
    let fd = -1, bytes
    try {
      const size = fs.statSync({ path: 'data://voice.opus' }).size
      if (!size || size < 50) throw new Error('Mic không tạo âm thanh. Kiểm tra quyền ghi âm hoặc đóng ứng dụng khác đang dùng mic.')
      if (size > MAX_AUDIO_BYTES) throw new Error('Đoạn ghi âm quá lớn; hãy nói câu ngắn hơn.')
      fd = fs.openSync({ path: 'data://voice.opus', flag: fs.O_RDONLY })
      if (fd < 0) throw new Error('Không mở được file ghi âm')
      const buffer = new ArrayBuffer(size)
      const count = fs.readSync({ fd, buffer, offset: 0, length: size, position: 0 })
      if (count !== size) throw new Error('Chưa đọc đủ file ghi âm; hãy thử lại')
      bytes = new Uint8Array(buffer)
    } finally { if (fd >= 0) fs.closeSync({ fd }) }
    const transferId = String(Date.now())
    const normalized = normalizeOpus(bytes)
    bytes = normalized.bytes
    if (bytes.length > MAX_AUDIO_BYTES) throw new Error('Âm thanh sau đóng gói quá lớn; hãy nói câu ngắn hơn.')
    const mime = normalized.mimeType
    await this.pcRequest('VOICE_BEGIN', { id: transferId, bytes: bytes.length, action: ACTIONS[this.state.activeTab], mime_type: mime }, 20000)
    // Multiple-of-three blocks concatenate to the original base64 stream.
    const chunkBytes = 3072
    for (let offset = 0, index = 0; offset < bytes.length; offset += chunkBytes, index++) {
      if (this.state.destroyed) return
      this.setStatus(`Đang gửi ${Math.round(offset / bytes.length * 100)}%`)
      await this.pcRequest('VOICE_CHUNK', { id: transferId, index, chunk: uint8ToBase64(bytes.subarray(offset, Math.min(offset + chunkBytes, bytes.length))) }, 20000)
    }
    this.setStatus('PC đang nhận diện giọng nói...')
    const result = await this.pcRequest('VOICE_FINISH', { id: transferId })
    this.receiveResult(result, result.text || LABELS[this.state.activeTab])
  },
  onDestroy() {
    this.state.destroyed = true
    if (this.state.pollTimer !== null) clearTimeout(this.state.pollTimer)
    if (this.state.voice) this.state.voice.dispose()
  }
}))
