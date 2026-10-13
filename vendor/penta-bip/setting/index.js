AppSettingsPage({
  build(props) {
    const input = (label, key, value) => TextInput({ label, labelStyle: { fontSize: '16px' }, placeholder: value, settingsKey: key, defaultValue: props.settingsStorage.getItem(key) || value })
    return Section({}, [
      Text({}, ['Đồng hồ nối BLE với Zepp trên điện thoại. Điện thoại cần truy cập Penta trên PC qua Wi-Fi hoặc Tailscale.']),
      input('URL Penta qua Tailscale', 'pc_url', 'http://100.115.158.11:3899'),
      input('URL Penta qua Wi-Fi nội bộ', 'lan_url', 'http://192.168.100.80:3899'),
      input('Mã ghép Windows (chỉ nhập một lần)', 'control_key', ''),
      Text({}, ['Mã trên chỉ dùng ghép PC với Zepp và được lưu tự động, không phải ô nhập lệnh. Sau khi ghép, nhập lệnh bằng nút BÀN PHÍM hoặc nói qua MIC trên đồng hồ; ứng dụng tự gửi đến PentaKuru. Lấy mã: PC → Alt+Q → Đồng hồ Amazfit → Sao chép mã.']),
      Text({}, ['Lấy URL trong Cài đặt Penta → Đồng hồ Amazfit. Cổng mặc định 3899. Nếu dùng Tailscale, điện thoại và PC phải ở cùng tailnet.'])
    ])
  }
})
