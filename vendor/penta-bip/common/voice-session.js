export function createVoiceSession({ recorder, opusCodec, onState, onReady, onError, schedule = setTimeout, cancel = clearTimeout, prepareFile = () => {} }) {
  let phase = 'idle', timer = null, disposed = false
  const change = value => { phase = value; onState(value) }
  const clear = () => { if (timer !== null) cancel(timer); timer = null }
  const fail = message => { clear(); change('idle'); onError(message) }
  const safeStop = () => { try { recorder.stop() } catch (_) {} }
  recorder.addEventListener(recorder.event.START, success => {
    if (disposed || phase !== 'starting') {
      if (success && (disposed || phase === 'idle')) safeStop()
      return
    }
    clear()
    if (!success) { fail('Không mở được mic. Kiểm tra quyền ghi âm và ứng dụng khác đang dùng mic.'); return }
    change('recording')
    timer = schedule(() => session.stop(), 15000)
  })
  recorder.addEventListener(recorder.event.STOP, () => {
    if (disposed || (phase !== 'stopping' && phase !== 'recording')) return
    clear(); change('sending')
    Promise.resolve().then(onReady).catch(error => onError(error.message || String(error))).then(() => {
      if (!disposed) change('idle')
    })
  })
  const session = {
    get phase() { return phase },
    start() {
      if (disposed || phase !== 'idle') return
      try {
        prepareFile()
        recorder.setFormat(opusCodec, { target_file: 'data://voice.opus' })
        change('starting')
        timer = schedule(() => {
          if (phase !== 'starting') return
          fail('Mic không phản hồi. Đóng ứng dụng ghi âm khác rồi thử lại.')
          try { recorder.stop() } catch (_) {}
        }, 5000)
        recorder.start()
      } catch (error) { fail(`Lỗi mở mic: ${error.message || error}`) }
    },
    stop() {
      if (phase !== 'recording') return
      clear(); change('stopping')
      timer = schedule(() => fail('Đồng hồ chưa đóng được file ghi âm. Thử lại.'), 5000)
      try { recorder.stop() } catch (error) { fail(`Lỗi dừng mic: ${error.message || error}`) }
    },
    dispose() {
      disposed = true; clear()
      if (typeof recorder.dispose === 'function') recorder.dispose()
      else if (phase === 'starting' || phase === 'recording' || phase === 'stopping') {
        try { recorder.stop() } catch (_) {}
      }
      phase = 'idle'
    }
  }
  return session
}
