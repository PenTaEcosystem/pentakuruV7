// Native in-app keyboard. No system keyboard installation or PNG required.
export function createInputPad({ createWidget, deleteWidget, widget, prop, onSubmit, onCancel }) {
  const children = []
  const group = { createWidget(type, config) { const child = createWidget(type, config); children.push(child); return child } }
  const keys = []
  let text = '', page = 0, closed = false
  const pages = [Array.from('abcdefghijklmnopqrstuvwxyz.,!?'), Array.from('0123456789+-*/=@:;_()[]{}<>"\\'), Array.from('áàảãạăắằẳẵặâấầẩẫậéèẻẽẹêếềểễệđ'), Array.from('íìỉĩịóòỏõọôốồổỗộơớờởỡợúùủũụư'), Array.from('ứừửữựýỳỷỹỵ')]
  const remove = child => { deleteWidget(child); const index = children.indexOf(child); if (index >= 0) children.splice(index,1) }
  const close = () => { if (!closed) { closed = true; while(children.length) remove(children[children.length-1]) } }
  const button = (x,y,w,h,label,click) => group.createWidget(widget.BUTTON, { x,y,w,h,text:label,text_size:22,color:0xffffff,normal_color:0x242a33,press_color:0x454f5f,radius:8,click_func:click })
  group.createWidget(widget.FILL_RECT, { x:0,y:0,w:432,h:514,color:0x0b0d10 })
  group.createWidget(widget.TEXT, { x:24,y:18,w:384,h:32,text:'Nhập cho PentaKuru',text_size:24,color:0xffffff })
  const display = group.createWidget(widget.TEXT, { x:24,y:58,w:384,h:44,text:'Chạm chữ để nhập',text_size:22,color:0xffffff })
  const update = () => display.setProperty(prop.TEXT, Array.from(text).slice(-27).join('') || 'Chạm chữ để nhập')
  const render = () => {
    keys.forEach(remove); keys.length = 0
    pages[page].forEach((letter,i) => keys.push(button(28+(i%6)*64,154+Math.floor(i/6)*52,56,44,letter,() => { if(Array.from(text).length<1000) text+=letter; update() })))
  }
  button(24,104,104,40,'ABC / 123',() => { page = page === 0 ? 1 : 0; render() })
  button(136,104,144,40,'Dấu Việt',() => { page = page < 2 || page === 4 ? 2 : page+1; render() })
  button(288,104,120,40,'Xóa',() => { text=Array.from(text).slice(0,-1).join('');update() })
  button(24,416,384,36,'Khoảng trắng',() => { if(Array.from(text).length<1000) text+=' ';update() })
  button(24,462,152,40,'Hủy',() => { close(); onCancel() })
  button(184,462,224,40,'Gửi',() => { if(!text.trim()) return; const result=text.trim();close();onSubmit(result) })
  render()
  return { close }
}
