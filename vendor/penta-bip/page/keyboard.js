import { BasePage } from '@zeppos/zml/base-page'
import { createWidget, deleteWidget, setStatusBarVisible, widget, prop } from '@zos/ui'
import { pop } from '@zos/router'
import { createInputPad } from '../common/input-pad'
import { createCompactUI } from '../common/compact-ui'

Page(BasePage({
  build() {
    try { setStatusBarVisible(false) } catch (_) {}
    const ui = createCompactUI({ createWidget, deleteWidget, prop })
    this.pad = createInputPad({ createWidget: ui.createWidget, deleteWidget: ui.deleteWidget, widget, prop,
      onSubmit: text => { this.globalData.inputResult = text; pop() },
      onCancel: () => { delete this.globalData.inputResult; pop() }
    })
  },
  onDestroy() { if (this.pad) this.pad.close() }
}))
