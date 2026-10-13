import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, writeFileSync } from 'node:fs'
import vm from 'node:vm'
import { createVoiceSession } from '../common/voice-session.js'
import { createMicrophone } from '../common/microphone.js'
import { legacyRecorder } from '../common/legacy-recorder.js'
import { normalizeOpus } from '../common/opus.js'
import { createChatState } from '../common/chat-state.js'
import { createInputPad } from '../common/input-pad.js'
import { createCompactUI } from '../common/compact-ui.js'
import { createPCClient, responseBody, settingUrl, uint8ToBase64, textRows, DEFAULT_URLS, MAX_AUDIO_BYTES } from '../common/transport.js'

test('Zepp fetch uses one options object, parses body and does not repeat a failed POST', async () => {
  const calls = []
  const client = createPCClient(async options => {
    calls.push(options)
    if (options.method === 'GET') return { body: JSON.stringify({ status: 'ok', app: 'PentaKuru' }) }
    throw new Error('connection lost after write')
  }, () => ['http://pc'], promise => promise)
  await assert.rejects(client.request('/api/notes', { method: 'POST', body: '{"content":"hello"}' }), /connection lost/)
  assert.equal(calls.length, 2)
  assert.equal(calls[1].url, 'http://pc/api/notes')
  assert.equal(calls[1].body, '{"content":"hello"}')
  assert.throws(() => responseBody({ status: 500, body: { error: 'API key invalid' } }), /API key invalid/)
  assert.equal(settingUrl('"http://pc:3899/"', ''), 'http://pc:3899')
})

function recorderFixture() {
  const events = {}, timers = new Map(), states = [], errors = []
  let next = 0, ready = 0, stops = 0
  const recorder = { event: { START: 1, STOP: 2 }, addEventListener: (event, cb) => events[event] = cb,
    setFormat() {}, start() {}, stop() { stops++ } }
  const session = createVoiceSession({ recorder, opusCodec: 1, onState: s => states.push(s),
    onError: e => errors.push(e), onReady: () => ready++, schedule: cb => { timers.set(++next, cb); return next }, cancel: id => timers.delete(id) })
  return { session, events, states, errors, timers, ready: () => ready, stops: () => stops }
}

test('mic becomes recording only after successful START; file is sent only after STOP', async () => {
  const f = recorderFixture()
  f.session.start(); assert.equal(f.session.phase, 'starting')
  f.events[1](true); assert.equal(f.session.phase, 'recording')
  f.session.stop(); assert.equal(f.session.phase, 'stopping'); assert.equal(f.ready(), 0)
  f.events[2](); await new Promise(resolve => setImmediate(resolve))
  assert.equal(f.ready(), 1); assert.equal(f.session.phase, 'idle')
  assert.deepEqual(f.states, ['starting', 'recording', 'stopping', 'sending', 'idle'])
})

test('failed or late microphone start never sends stale audio', async () => {
  const f = recorderFixture()
  f.session.start(); f.events[1](false); assert.equal(f.session.phase, 'idle')
  assert.equal(f.errors.length, 1); assert.equal(f.ready(), 0)
  f.session.start(); [...f.timers.values()][0]()
  f.events[1](true); f.events[2]()
  assert.equal(f.session.phase, 'idle'); assert.equal(f.ready(), 0); assert.equal(f.stops(), 2)
  f.session.dispose(); f.events[1](true); assert.equal(f.stops(), 3)
})

test('15 second auto-stop waits for the recorder STOP event', () => {
  const f = recorderFixture()
  f.session.start(); f.events[1](true); [...f.timers.values()][0]()
  assert.equal(f.session.phase, 'stopping'); assert.equal(f.ready(), 0)
})

test('phone bridge reassembles BLE chunks and rejects missing or reordered chunks', async () => {
  let definition, uploaded, writes = 0
  const source = readFileSync(new URL('../app-side/index.js', import.meta.url), 'utf8').replace(/^import .*$/gm, '')
  vm.runInNewContext(source, {
    BaseSideService: x => x, AppSideService: x => definition = x,
    createPCClient, settingUrl, DEFAULT_URLS, MAX_AUDIO_BYTES,
    fetch: async options => {
      if (options.method === 'GET') return { body: { status: 'ok', app: 'PentaKuru' } }
      writes++; uploaded = JSON.parse(options.body)
      return { body: { status: 'ok', text: 'hello' } }
    }
  })
  const service = { ...definition, settings: { getItem: () => null } }
  definition.onInit.call(service)
  const request = (method, params = {}) => new Promise(resolve => definition.onRequest.call(service, { method, params }, (_, result) => resolve(result)))
  const audio = Uint8Array.from({ length: 7001 }, (_, i) => i % 256)
  await request('VOICE_BEGIN', { id: 'a', bytes: audio.length, action: 'ask', mime_type: 'audio/opus' })
  assert.equal((await request('VOICE_CHUNK', { id: 'a', index: 1, chunk: 'AAAA' })).status, 'error')
  let index = 0
  for (let offset = 0; offset < audio.length; offset += 3072) {
    assert.equal((await request('VOICE_CHUNK', { id: 'a', index: index++, chunk: uint8ToBase64(audio.slice(offset, offset + 3072)) })).status, 'ok')
  }
  assert.equal((await request('VOICE_FINISH', { id: 'a' })).status, 'ok')
  assert.deepEqual(Buffer.from(uploaded.audio_base64, 'base64'), Buffer.from(audio))
  assert.equal(writes, 1)
  assert.equal((await request('VOICE_FINISH', { id: 'a' })).status, 'error')
  await request('VOICE_BEGIN', { id: 'b', bytes: 10 })
  assert.equal((await request('VOICE_FINISH', { id: 'b' })).status, 'error')
  assert.equal(writes, 1)
})

test('scrollable reply preserves long Vietnamese text and emoji', () => {
  const text = 'Anh kéo xuống để đọc đầy đủ câu trả lời 🎤 '.repeat(40).trim()
  const rows = textRows(text)
  assert.equal(rows.map(x => x.line).join(' '), text)
  assert.ok(rows.length > 30)
  for (const row of rows) assert.ok(Array.from(row.line).length <= 27)
})

test('recorder acquisition errors retain their stage and allow a subsequent retry', () => {
  let calls = 0
  const recorder = { event: { START: 0, STOP: 1 }, addEventListener() {}, setFormat() {}, start() {}, stop() {} }
  const media = { id: { RECORDER: 0 }, codec: { OPUS: 0 }, create() { if (++calls === 1) throw new Error('resource busy'); return recorder } }
  const factory = options => options
  assert.throws(() => createMicrophone(media, factory, {}), /MIC_CREATE: resource busy/)
  assert.equal(createMicrophone(media, factory, {}).recorder, recorder)
  assert.equal(calls, 2)
  assert.throws(() => createMicrophone({ ...media, create: () => null }, factory, {}), /MIC_CREATE/)
  assert.throws(() => createMicrophone({ ...media, create: () => ({ ...recorder, event: undefined }) }, factory, {}), /MIC_EVENTS/)
  assert.throws(() => createMicrophone(media, () => { throw new Error('timer missing') }, {}), /MIC_SESSION: timer missing/)
})

test('every build manifest declares the actual Zepp mic permission', () => {
  for (const file of ['app.json', 'app-ai.json']) {
    const manifest = JSON.parse(readFileSync(new URL('../' + file, import.meta.url), 'utf8'))
    assert.ok(manifest.permissions.includes('device:os.mic'), `${file} must grant microphone access`)
  }
})

function legacyFixture() {
  const timers = new Map()
  let timerId = 0, time = 0, bytes = 0, starts = 0, stops = 0
  const device = { setFormat() {}, start() { starts++ }, stop() { stops++ } }
  const options = { readSize: () => bytes, now: () => time,
    schedule: (callback, ms) => { timers.set(++timerId, { callback, at: time + ms }); return timerId }, cancel: id => timers.delete(id) }
  const recorder = legacyRecorder(device, options)
  return { recorder, options, device, setSize: value => bytes = value, stops: () => stops, starts: () => starts,
    advance(ms) { const end = time + ms; while (true) { const entry = [...timers.entries()].sort((a, b) => a[1].at - b[1].at)[0]; if (!entry || entry[1].at > end) break; timers.delete(entry[0]); time = entry[1].at; entry[1].callback() } time = end },
    pending: () => timers.size }
}

test('eventless firmware confirms actual file growth and waits for stopped audio to settle', async () => {
  const f = legacyFixture(), events = []
  f.recorder.addEventListener(0, ok => events.push(['start', ok]))
  f.recorder.addEventListener(1, () => events.push(['stop']))
  f.recorder.start(); f.advance(1000); assert.equal(events.length, 0)
  f.setSize(100); f.advance(250); assert.deepEqual(events, [['start', true]])
  f.recorder.stop(); f.advance(500); assert.equal(events.length, 1)
  f.setSize(140); f.advance(250); assert.equal(events.length, 1)
  f.advance(750); assert.deepEqual(events, [['start', true], ['stop']])
  assert.equal(f.stops(), 1)
  f.recorder.dispose(); assert.equal(f.pending(), 0)
})

test('eventless firmware never reports recording with an empty file and cancels on exit', () => {
  const f = legacyFixture(), events = []
  f.recorder.addEventListener(0, ok => events.push(ok))
  f.recorder.start(); f.advance(4500); assert.deepEqual(events, [false]); assert.equal(f.stops(), 1)
  f.recorder.start(); f.recorder.dispose(); f.setSize(200); f.advance(6000)
  assert.deepEqual(events, [false]); assert.equal(f.pending(), 0)
})

test('raw Opus packets retain packet boundaries in Ogg and have correct duration', () => {
  const packets = [Uint8Array.of(0xf8, 0xff, 0xfe), new Uint8Array(255).fill(0xf8)]
  const raw = new Uint8Array(packets.reduce((sum, p) => sum + 8 + p.length, 0))
  let position = 0
  for (const packet of packets) { new DataView(raw.buffer).setUint32(position, packet.length); raw.set(packet, position + 8); position += 8 + packet.length }
  const result = normalizeOpus(raw)
  assert.equal(result.mimeType, 'audio/ogg')
  const recovered = [], granules = []
  let offset = 0, flags = []
  while (offset < result.bytes.length) {
    const b = result.bytes, segments = b[offset + 26], lacing = b.slice(offset + 27, offset + 27 + segments)
    assert.equal(Buffer.from(b.slice(offset, offset + 4)).toString(), 'OggS')
    flags.push(b[offset + 5]); granules.push(new DataView(b.buffer).getUint32(offset + 6, true))
    let data = offset + 27 + segments, packet = []
    for (const length of lacing) { packet.push(...b.slice(data, data + length)); data += length; if (length < 255) { recovered.push(Uint8Array.from(packet)); packet = [] } }
    assert.equal(packet.length, 0)
    offset = data
  }
  assert.equal(Buffer.from(recovered[0].slice(0, 8)).toString(), 'OpusHead')
  assert.equal(Buffer.from(recovered[1].slice(0, 8)).toString(), 'OpusTags')
  assert.deepEqual(recovered.slice(2), packets)
  assert.deepEqual(granules, [0, 0, 1920]); assert.deepEqual(flags, [2, 0, 4])
  assert.equal(normalizeOpus(result.bytes).bytes, result.bytes)
  const truncated = raw.slice(0, raw.length - 1)
  assert.throws(() => normalizeOpus(truncated), /thiếu dữ liệu/)
})

test('watch page defers microphone acquisition and retries a failed acquisition', () => {
  let page, acquisitions = 0
  const source = readFileSync(new URL('../page/index.js', import.meta.url), 'utf8').replace(/^import .*$/gm, '')
  const recorder = { event: { START: 0, STOP: 1 }, addEventListener() {}, setFormat() {}, start() {}, stop() {} }
  vm.runInNewContext(source, { BasePage: x => x, Page: x => page = x, createVoiceSession, createMicrophone,
    create: () => { if (++acquisitions === 1) throw new Error('resource busy'); return recorder },
    id: { RECORDER: 0 }, codec: { OPUS: 0 }, getDeviceInfo: () => ({ deviceSource: 11206915 }),
    setTimeout, clearTimeout, MAX_AUDIO_BYTES, uint8ToBase64, textRows, createChatState, createCompactUI, prop: {}, nativeCreateWidget() {}, nativeDeleteWidget() {},
  })
  page.globalData = {}; page.onInit()
  assert.equal(acquisitions, 0)
  assert.equal(page.ensureVoice(), false)
  assert.match(page.state.micError, /MIC_CREATE: resource busy/)
  assert.equal(page.ensureVoice(), true)
  assert.equal(page.state.voice.phase, 'idle')
  assert.equal(page.state.micError, '')
  assert.equal(page.ensureVoice(), true)
  assert.equal(acquisitions, 2)
  page.onDestroy()
})

test('chat preserves previous response through pending requests and failures, then replaces it atomically', () => {
  const chat = createChatState()
  chat.accept('Câu trước', 'Trả lời trước')
  chat.begin()
  assert.match(chat.text(), /Trả lời trước/)
  chat.reject('Mất kết nối')
  assert.match(chat.text(), /Trả lời trước/)
  assert.match(chat.text(), /Mất kết nối/)
  chat.accept('Câu mới', 'Trả lời mới')
  assert.doesNotMatch(chat.text(), /trước|Mất kết nối/)
  assert.match(chat.text(), /Trả lời mới/)
})

test('Windows phone bridge requires pairing and keeps credentials off GET status', async () => {
  let definition, key = '', writes = []
  vm.runInNewContext(readFileSync(new URL('../app-side/index.js', import.meta.url), 'utf8').replace(/^import .*$/gm, ''), {
    BaseSideService: x => x, AppSideService: x => definition = x,
    createPCClient, settingUrl, DEFAULT_URLS, MAX_AUDIO_BYTES,
    fetch: async options => { if (options.method === 'GET') { assert.equal(options.body, undefined); return { body: { status: 'ok', app: 'PentaKuru' } } }
      writes.push({ path: options.url, body: JSON.parse(options.body) }); return { body: { status: 'ok', task: { id: 'test', phase: 'pending' } } } }
  })
  const service = { ...definition, settings: { getItem: name => name === 'control_key' ? key : null } }
  service.onInit()
  const request = (method, params = {}) => new Promise(resolve => service.onRequest({ method, params }, (_, result) => resolve(result)))
  assert.equal((await request('WIN_EXECUTE', { text: 'Mở Notepad' })).status, 'error')
  assert.equal((await request('VOICE_BEGIN', { action: 'execute', bytes: 3 })).status, 'error')
  assert.equal(writes.length, 0)
  key = '"test-pairing-key"'
  await request('WIN_EXECUTE', { text: 'Mở Notepad' })
  await request('WIN_STATUS', { id: 'test' })
  await request('WIN_CANCEL', { id: 'test' })
  assert.deepEqual(writes.map(w => w.body.control_key), ['test-pairing-key','test-pairing-key','test-pairing-key'])
  assert.equal(writes[0].body.command, 'Mở Notepad')
  await request('VOICE_BEGIN', { id: 'voice', action: 'execute', bytes: 3 })
  await request('VOICE_CHUNK', { id: 'voice', index: 0, chunk: 'AAAA' })
  await request('VOICE_FINISH', { id: 'voice' })
  assert.equal(writes[3].body.control_key, 'test-pairing-key')
})

function chatPageFixture(globalData = {}) {
  let page, keyboard, widgets = [], scheduled = []
  vm.runInNewContext(readFileSync(new URL('../page/index.js', import.meta.url), 'utf8').replace(/^import .*$/gm, ''), {
    BasePage: x => x, Page: x => page = x, createChatState, textRows, createCompactUI,
    getDeviceInfo: () => ({ deviceSource: 11206915 }),
    nativeCreateWidget: (type, config) => { const item = { type, config, updates: [], setProperty(prop, data) { this.updates.push({ prop, data }) } }; widgets.push(item); return item },
    widget: { TEXT: 'text', BUTTON: 'button', SCROLL_LIST: 'list' }, align: {}, prop: { TEXT: 'text', MORE: 'more', UPDATE_DATA: 'data' },
    push: options => keyboard = options, nativeDeleteWidget() {}, setStatusBarVisible() {},
    setTimeout: callback => { scheduled.push(callback); return scheduled.length }, clearTimeout() {},
  })
  page.globalData = globalData; page.onInit(); page.checkConnection = () => {}; page.build()
  return { page, widgets, scheduled, keyboard: () => keyboard }
}

test('watch chat routes fourth icon to Windows, keeps old reply during execution, and exposes Stop', async () => {
  const { page, widgets, scheduled } = chatPageFixture()
  assert.equal(widgets.filter(w => w.config.normal_src && /pentagon/.test(w.config.normal_src)).length, 0)
  assert.ok(widgets.find(w => w.type === 'list').config.h >= 200)
  const footer = widgets.filter(w => ['compact/chat_keyboard_button.png','compact/chat_mic_button.png'].includes(w.config.normal_src))
  assert.equal(footer.length, 2)
  for (const item of footer) {
    assert.ok(item.config.y + item.config.h <= 482)
    assert.ok(item.config.normal_src && item.config.press_src)
  }
  page.showResult('Trả lời trước', false, 'Câu trước')
  const requests = []
  page.request = async request => { requests.push(request); return { status: 'ok', task: { id: 'task1', goal: request.params.text, phase: 'pending' } } }
  await page.switchTab(3)
  assert.match(page.state.chat.text(), /Trả lời trước/)
  await page.sendText('Mở Notepad')
  assert.equal(requests[0].method, 'WIN_EXECUTE')
  assert.equal(page.state.taskId, 'task1')
  assert.match(page.state.chat.text(), /Trả lời trước/)
  assert.equal(page.state.micBtn.native.updates.at(-1).data.normal_src, 'compact/chat_stop_button.png')
  assert.equal(page.state.micBtn.native.updates.at(-1).data.w, 90)
  assert.equal(scheduled.length, 1)
  page.handleTask({ id: 'other', phase: 'completed', message: 'Sai phiên' })
  assert.equal(page.state.taskId, 'task1')
  page.handleTask({ id: 'task1', phase: 'running', message: 'Bước 2' })
  assert.match(page.state.chat.text(), /Trả lời trước/)
  page.handleTask({ id: 'task1', phase: 'completed', message: 'Đã mở Notepad' })
  assert.equal(page.state.taskId, '')
  assert.doesNotMatch(page.state.chat.text(), /Trả lời trước/)
  assert.match(page.state.chat.text(), /Đã mở Notepad/)
  page.onDestroy()
})

test('watch cancellation and disconnection retain previous answer; keyboard submits text', async () => {
  const { page, keyboard } = chatPageFixture()
  page.showResult('Giữ câu này', false, 'Câu cũ')
  let typed
  page.sendText = text => typed = text
  page.openInput(); assert.equal(keyboard().url, 'page/keyboard'); page.globalData.inputResult='Lệnh được nhập'; page.onResume()
  assert.equal(typed, 'Lệnh được nhập'); assert.equal(page.state.keyboardOpen, false)
  page.receiveResult({ task: { id: 'cancel', goal: 'Lệnh cần dừng' } }, '')
  page.request = async request => { assert.equal(request.method, 'WIN_CANCEL'); return { status: 'ok', task: { id: 'cancel', phase: 'cancelled', message: 'Đã dừng' } } }
  page.pollError(new Error('network timeout'))
  assert.equal(page.state.taskId, 'cancel')
  await page.stopWindows()
  assert.equal(page.state.taskId, '')
  assert.match(page.state.chat.text(), /Giữ câu này/)
  assert.match(page.state.chat.text(), /Đã dừng/)
  page.onDestroy()
})

if (process.env.WATCH_LAYOUT_OUTPUT) {
  const { page, widgets } = chatPageFixture()
  page.state.activeTab = 3
  page.showResult('Đã mở ứng dụng theo yêu cầu. Anh có thể gửi lệnh tiếp theo.', false, 'Mở Notepad trên PC')
  const list = widgets.find(w => w.type === 'list')
  list.config.data_array = list.updates.at(-1).data.data_array
  widgets.find(w => w.type === 'text' && w.config.y === 99).config.text = 'Windows · PC đã kết nối'
  widgets.filter(w => w.type === 'button' && w.config.y === 339).forEach((w,i) => w.config.normal_src = `compact/chat_${i}_${i===3?'act':'norm'}.png`)
  writeFileSync(process.env.WATCH_LAYOUT_OUTPUT, JSON.stringify(widgets.map(w => ({ type:w.type, config:w.config })), null, 2))
}

test('built-in keyboard enters letters, spaces, Vietnamese accents and numbers without system APIs', () => {
  const widgets = [], deleted = new Set()
  let submitted, cancelled = 0
  const options = { createWidget(type, config) { const item = { type, config, setProperty() {} };widgets.push(item);return item }, deleteWidget: item => deleted.add(item),
    widget: { GROUP: 'group', BUTTON: 'button', TEXT: 'text', FILL_RECT:'rect' }, prop: {TEXT: 'text'},
    onSubmit: text => submitted = text, onCancel: () => cancelled++ }
  createInputPad(options)
  const press = label => { const key = widgets.find(w => !deleted.has(w) && w.type === 'button' && w.config.text === label); assert.ok(key, 'Missing key: '+label); key.config.click_func() }
  press('Gửi'); assert.equal(submitted, undefined); assert.equal(deleted.size, 0)
  press('a');press('Khoảng trắng');press('b');press('Xóa');press('Dấu Việt');press('á');press('ABC / 123');press('ABC / 123');press('1');press('Gửi')
  assert.equal(submitted, 'a á1');assert.equal(widgets.filter(w => !deleted.has(w)).length, 0);assert.equal(cancelled, 0)
  for (const w of widgets) { assert.ok(w.config.x + w.config.w <= 432); assert.ok(w.config.y + w.config.h <= 514) }
  const pad = createInputPad(options);pad.close();pad.close();assert.equal(cancelled, 0)
})

test('compact keyboard button opens a separate page and submits back to the same chat', () => {
  const { page, widgets, keyboard } = chatPageFixture()
  let typed, keyboardPage, popped = 0
  page.sendText = text => typed = text
  const input = widgets.find(w => w.config.normal_src === 'compact/chat_keyboard_button.png')
  assert.equal(input.config.w, 208); assert.equal(input.config.h, 45)
  input.config.click_func()
  assert.equal(keyboard().url, 'page/keyboard')
  const keys = [], removed = new Set()
  vm.runInNewContext(readFileSync(new URL('../page/keyboard.js', import.meta.url),'utf8').replace(/^import .*$/gm,''), {
    BasePage: x => x, Page: x => keyboardPage = x, createCompactUI, createInputPad,
    createWidget(type,config) { const item = {type,config,setProperty(){}};keys.push(item);return item }, deleteWidget: key => removed.add(key),
    setStatusBarVisible(){}, pop: () => popped++,
    widget:{BUTTON:'button',TEXT:'text',FILL_RECT:'rect'}, prop:{TEXT:'text',MORE:'more'}
  })
  keyboardPage.globalData=page.globalData;keyboardPage.build()
  const press = text => keys.find(w => !removed.has(w) && w.config.text===text).config.click_func()
  press('a');press('Gửi');assert.equal(popped,1)
  page.onResume();assert.equal(typed,'a');assert.equal(page.state.keyboardOpen,false)
  assert.equal(keys.filter(w => !removed.has(w)).length,0)
  keyboardPage.onDestroy()
  for(const key of keys) {assert.ok(key.config.x>=43);assert.ok(key.config.y>=51);assert.ok(key.config.x+key.config.w<=389);assert.ok(key.config.y+key.config.h<=463)}
})

test('compact list child geometry remains relative and button updates preserve scaled hit targets', () => {
  const { page, widgets } = chatPageFixture()
  const list=widgets.find(w=>w.type==='list').config
  assert.equal(list.x,59);assert.equal(list.item_config[0].text_view[0].x,10)
  assert.equal(list.item_config[0].item_height,26)
  page.state.taskId='task';page.updateMic()
  const update=page.state.micBtn.native.updates.at(-1).data
  assert.equal(update.x,280);assert.equal(update.y,392);assert.equal(update.w,90);assert.equal(update.h,45)
})

test('keyboard navigation preserves previous reply and selected mode even if firmware recreates the chat page', () => {
  const shared = {}
  const { page } = chatPageFixture(shared)
  page.state.activeTab=3;page.showResult('Câu trả lời cần giữ',false,'Câu cũ');page.openInput();page.onDestroy()
  const returned=chatPageFixture(shared).page
  assert.equal(returned.state.activeTab,3);assert.match(returned.state.chat.text(),/Câu trả lời cần giữ/)
  assert.equal(returned.state.destroyed,false);assert.equal(returned.state.keyboardOpen,false)
})
