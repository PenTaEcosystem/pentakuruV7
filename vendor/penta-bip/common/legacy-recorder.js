// Some Zepp firmware exposes setFormat/start/stop without recorder events.
// Confirm real file growth before announcing recording, then wait for the
// stopped file to settle. Never fabricate a successful microphone start.
export function legacyRecorder(recorder, { readSize, schedule = setTimeout, cancel = clearTimeout, now = Date.now } = {}) {
  if (typeof readSize !== 'function') throw new Error('MIC_EVENTS: thiếu kiểm tra file cho bộ ghi âm không có sự kiện')
  let timer = null, active = false, generation = 0
  const callbacks = {}
  const clear = () => { generation++; if (timer !== null) cancel(timer); timer = null }
  const size = () => { try { return readSize() || 0 } catch (_) { return 0 } }
  const emit = (event, value) => { if (callbacks[event]) callbacks[event](value) }
  const poll = (callback, token) => {
    timer = schedule(() => { timer = null; if (token === generation) callback() }, 250)
  }
  return {
    event: { START: 0, STOP: 1 },
    addEventListener(event, callback) { callbacks[event] = callback },
    setFormat(format, options) { recorder.setFormat(format, options) },
    start() {
      clear(); active = true
      recorder.start()
      const token = generation, began = now()
      const check = () => {
        if (size() > 50) { emit(0, true); return }
        if (now() - began >= 4500) { emit(0, false); clear(); try { recorder.stop() } catch (_) {}; active = false; return }
        poll(check, token)
      }
      poll(check, token)
    },
    stop() {
      clear()
      if (!active) return
      active = false
      recorder.stop()
      const token = generation, began = now()
      let previous = -1, stable = 0
      const check = () => {
        const current = size()
        stable = current === previous ? stable + 1 : 0
        previous = current
        if (stable >= 3 && now() - began >= 1000) { emit(1); return }
        if (now() - began < 4500) poll(check, token)
        // If it keeps changing, the voice session's stop timeout reports it.
      }
      poll(check, token)
    },
    dispose() { clear(); if (active) { active = false; try { recorder.stop() } catch (_) {} } }
  }
}
