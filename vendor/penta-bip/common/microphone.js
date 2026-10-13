import { legacyRecorder } from './legacy-recorder.js'
// Keep SDK initialization errors distinct from recording and network errors.
export function createMicrophone(media, sessionFactory, options) {
  if (!media || typeof media.create !== 'function' || !media.id || media.id.RECORDER === undefined) {
    throw new Error('MIC_API: firmware không cung cấp API Recorder')
  }
  if (!media.codec || media.codec.OPUS === undefined) throw new Error('MIC_CODEC: firmware không cung cấp OPUS')
  let recorder
  try { recorder = media.create(media.id.RECORDER) }
  catch (error) { throw new Error(`MIC_CREATE: ${error.message || error}`) }
  if (!recorder) throw new Error('MIC_CREATE: hệ thống không cấp bộ ghi âm; đóng ứng dụng đang dùng mic rồi thử lại')
  const required = ['setFormat', 'start', 'stop']
  for (const method of required) {
    if (typeof recorder[method] !== 'function') throw new Error(`MIC_API: thiếu Recorder.${method}`)
  }
  if (typeof recorder.addEventListener !== 'function' || !recorder.event || recorder.event.START === undefined || recorder.event.STOP === undefined) {
    recorder = legacyRecorder(recorder, options.legacy)
  }
  try { return sessionFactory({ ...options, recorder, opusCodec: media.codec.OPUS }) }
  catch (error) { throw new Error(`MIC_SESSION: ${error.message || error}`) }
}
