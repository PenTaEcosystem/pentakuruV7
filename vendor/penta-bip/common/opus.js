// Package Zepp's length/final-range/Opus-packet stream as Ogg Opus (RFC 7845).
// This changes the container only; the recorded Opus packets are preserved.
const ascii = text => Uint8Array.from(Array.from(text).map(c => c.charCodeAt(0)))
const be32 = (bytes, offset) => (bytes[offset] * 0x1000000 + (bytes[offset + 1] << 16) + (bytes[offset + 2] << 8) + bytes[offset + 3]) >>> 0
const le32 = (bytes, offset, value) => { for (let i = 0; i < 4; i++) bytes[offset + i] = value >>> (8 * i) & 255 }
function samples(packet) {
  const config = packet[0] >> 3, code = packet[0] & 3
  const count = code === 0 ? 1 : code === 3 ? (packet[1] || 0) & 63 : 2
  const duration = config >= 16 ? 120 << (config & 3) : config >= 12 ? 480 << (config & 1) : [480, 960, 1920, 2880][config & 3]
  if (!count || count * duration > 5760) throw new Error('Gói Opus lỗi hoặc file ghi âm chưa hoàn tất')
  return count * duration
}
function page(packets, sequence, granule, flags) {
  const lacing = []
  let length = 0
  for (const packet of packets) {
    length += packet.length
    for (let remaining = packet.length; remaining >= 255; remaining -= 255) lacing.push(255)
    lacing.push(packet.length % 255)
  }
  if (lacing.length > 255) throw new Error('Gói Opus quá lớn')
  const bytes = new Uint8Array(27 + lacing.length + length)
  bytes.set(ascii('OggS')); bytes[5] = flags
  le32(bytes, 6, granule); le32(bytes, 10, Math.floor(granule / 0x100000000))
  le32(bytes, 14, 23129); le32(bytes, 18, sequence)
  bytes[26] = lacing.length; bytes.set(lacing, 27)
  let offset = 27 + lacing.length
  for (const packet of packets) { bytes.set(packet, offset); offset += packet.length }
  let crc = 0
  for (const byte of bytes) {
    crc = (crc ^ byte << 24) >>> 0
    for (let bit = 0; bit < 8; bit++) crc = (crc << 1 ^ (crc & 0x80000000 ? 0x04c11db7 : 0)) >>> 0
  }
  le32(bytes, 22, crc)
  return bytes
}
export function normalizeOpus(bytes) {
  if (bytes[0] === 79 && bytes[1] === 103 && bytes[2] === 103 && bytes[3] === 83) return { bytes, mimeType: 'audio/ogg' }
  const first = bytes.length >= 9 ? be32(bytes, 0) : 0
  if (!first || first > 65000 || first > bytes.length - 8) return { bytes, mimeType: 'audio/opus' }
  const packets = []
  for (let offset = 0; offset < bytes.length;) {
    if (offset + 8 > bytes.length) throw new Error('File Opus chưa ghi xong; hãy thử lại')
    const length = be32(bytes, offset)
    if (!length || length > 65000 || offset + 8 + length > bytes.length) throw new Error('File Opus thiếu dữ liệu; hãy ghi lại')
    packets.push(bytes.subarray(offset + 8, offset + 8 + length)); offset += 8 + length
  }
  const head = new Uint8Array(19)
  head.set(ascii('OpusHead')); head[8] = 1; head[9] = packets[0][0] & 4 ? 2 : 1
  le32(head, 12, 48000) // Output granule clock; source rate is not needed to decode Opus.
  const tags = new Uint8Array(16); tags.set(ascii('OpusTags')) // Empty vendor and comment list.
  const pages = [page([head], 0, 0, 2), page([tags], 1, 0, 0)]
  let granule = 0, group = [], segments = 0
  for (let i = 0; i < packets.length; i++) {
    const packet = packets[i], count = Math.floor(packet.length / 255) + 1
    if (segments + count > 255 && group.length) { pages.push(page(group, pages.length, granule, 0)); group = []; segments = 0 }
    group.push(packet); segments += count; granule += samples(packet)
    if (group.length >= 25 || i === packets.length - 1) { pages.push(page(group, pages.length, granule, i === packets.length - 1 ? 4 : 0)); group = []; segments = 0 }
  }
  const result = new Uint8Array(pages.reduce((total, p) => total + p.length, 0))
  let offset = 0
  for (const p of pages) { result.set(p, offset); offset += p.length }
  return { bytes: result, mimeType: 'audio/ogg' }
}
