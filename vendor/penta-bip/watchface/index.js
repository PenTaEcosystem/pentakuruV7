import { createWidget, widget, align, prop } from '@zos/ui'

// ===== Cấu hình tối ưu hiệu năng =====
const W = 432, H = 514            // Màn hình Bip Max
const SIZE = 280                  // Canvas 280px vừa khít khối (giảm 35% diện tích clear mỗi frame)
const C = SIZE / 2                // Tâm canvas
const UNIT = 75                   // Kích thước khối
const FRAME_MS = 33               // 33ms ≈ 30 FPS (mượt mà gấp 3 lần so với 100ms cũ)
const SPIN = 0.0012               // Tốc độ xoay (rad/ms)
const DAYS = ['CN', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7']

// ===== Hình khối: Thập nhị diện (20 đỉnh, 30 cạnh) =====
const P = (1 + Math.sqrt(5)) / 2
const Q = 1 / P
const V = []
for (const x of [-1, 1]) for (const y of [-1, 1]) for (const z of [-1, 1]) V.push([x, y, z])
for (const a of [-1, 1]) for (const b of [-1, 1]) {
  V.push([0, a * Q, b * P])
  V.push([a * Q, b * P, 0])
  V.push([a * P, 0, b * Q])
}
const E = []
for (let i = 0; i < V.length; i++) {
  for (let j = i + 1; j < V.length; j++) {
    const dx = V[i][0] - V[j][0], dy = V[i][1] - V[j][1], dz = V[i][2] - V[j][2]
    if (Math.abs(Math.sqrt(dx * dx + dy * dy + dz * dz) - 2 * Q) < 1e-6) E.push([i, j])
  }
}

// Chuyển mức xám tối ưu
const gray = (g) => {
  const v = g < 0 ? 0 : g > 255 ? 255 : (g | 0)
  return (v << 16) | (v << 8) | v
}

// Tái sử dụng mảng 20 đỉnh (Tránh rác bộ nhớ / GC lag trên chip đồng hồ)
const pts = new Array(V.length)
for (let i = 0; i < V.length; i++) {
  pts[i] = { x: 0, y: 0, z: 0 }
}

const pad = (n) => (n < 10 ? '0' + n : '' + n)

let canvas = null
let timeText = null
let dateText = null
let timerId = null
let clockTimerId = null
let lastTime = ''
let lastDate = ''
const T0 = Date.now()

// Cập nhật đồng hồ 1 giây 1 lần (tách khỏi vòng lặp đồ họa)
function updateClock() {
  const d = new Date()
  const hm = pad(d.getHours()) + ':' + pad(d.getMinutes())
  if (hm !== lastTime) {
    lastTime = hm
    timeText.setProperty(prop.TEXT, hm)
  }
  const dt = DAYS[d.getDay()] + ' ' + pad(d.getDate()) + '.' + pad(d.getMonth() + 1)
  if (dt !== lastDate) {
    lastDate = dt
    dateText.setProperty(prop.TEXT, dt)
  }
}

// Vòng lặp vẽ 3D mượt mà
function frame() {
  const t = Date.now() - T0
  const ay = t * SPIN
  const ax = 0.45 + Math.sin(t * 0.0003) * 0.12

  const cy = Math.cos(ay), sy = Math.sin(ay)
  const cx = Math.cos(ax), sx = Math.sin(ax)

  // 1. Phép chiếu 3D -> 2D
  for (let i = 0; i < V.length; i++) {
    const v = V[i]
    const X = v[0] * cy + v[2] * sy
    let Z = -v[0] * sy + v[2] * cy
    const Y = v[1] * cx - Z * sx
    Z = v[1] * sx + Z * cx
    const f = 4.2 / (4.2 - Z)
    const pt = pts[i]
    pt.x = C + X * f * UNIT
    pt.y = C + Y * f * UNIT
    pt.z = Z
  }

  // 2. Xóa canvas
  canvas.clear({ x: 0, y: 0, w: SIZE, h: SIZE })

  // 3. Vẽ 30 cạnh chính (1 lượt duy nhất nét 2px, loại bỏ lớp 6px gây nghẽn CPU)
  canvas.setPaint({ color: 0xffffff, line_width: 2 })
  for (let k = 0; k < E.length; k++) {
    const a = pts[E[k][0]], b = pts[E[k][1]]
    const depth = (a.z + b.z) * 0.5
    const alpha = 0.35 + (0.65 * (depth + 1.6)) / 3.2
    canvas.drawLine({
      x1: a.x | 0,
      y1: a.y | 0,
      x2: b.x | 0,
      y2: b.y | 0,
      color: gray(255 * (alpha > 1 ? 1 : alpha < 0 ? 0 : alpha))
    })
  }

  timerId = setTimeout(frame, FRAME_MS)
}

WatchFace({
  onInit() {},

  build() {
    const left = (W - SIZE) / 2
    const top = (H - SIZE) / 2

    canvas = createWidget(widget.CANVAS, { x: left, y: top, w: SIZE, h: SIZE })

    timeText = createWidget(widget.TEXT, {
      x: 0, y: H / 2 - 50, w: W, h: 50,
      text: '00:00', text_size: 40, color: 0xf2f2f2,
      align_h: align.CENTER_H, align_v: align.CENTER_V
    })

    dateText = createWidget(widget.TEXT, {
      x: 0, y: H / 2 + 2, w: W, h: 28,
      text: '', text_size: 20, color: 0x8a8a8a,
      align_h: align.CENTER_H, align_v: align.CENTER_V
    })

    updateClock()
    clockTimerId = setInterval(updateClock, 1000)

    frame()
  },

  onDestroy() {
    if (timerId !== null) clearTimeout(timerId)
    if (clockTimerId !== null) clearInterval(clockTimerId)
  }
})
