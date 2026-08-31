import QtQuick
import Quickshell
import qs.Commons
import qs.Ui

Panel {
  id: root
  moduleName: "io.github.pl402.pomotask"
  ipcTarget: "io.github.pl402.pomotask.panel"
  manageIpc: false

  property var anchorItem: null
  property var hostWidget: null
  readonly property var barIdentity: hostWidget || root

  readonly property color contentForeground: bar ? bar.foreground : Color.foreground
  readonly property string contentFontFamily: bar ? bar.fontFamily : Style.font.family

  function open() {
    refresh()
    root.controller.show()
  }

  function close() {
    root.controller.hide()
  }

  function toggle() {
    if (root.opened) root.close()
    else root.open()
  }

  function switchPanel(direction) {
    if (root.bar && typeof root.bar.switchPanelFrom === "function")
      return root.bar.switchPanelFrom(root.barIdentity, direction)
    return false
  }

  function refresh() {
    // Placeholder refresh logic for Panel
  }

  KeyboardPanel {
    id: panel
    anchorItem: root.anchorItem
    owner: root.barIdentity
    bar: root.bar
    open: root.opened
    centerOnBar: true
    contentWidth: panel.fittedContentWidth(Style.space(360))
    contentHeight: panel.fittedContentHeight(placeholderColumn.implicitHeight)

    Column {
      id: placeholderColumn
      width: parent.width
      spacing: Style.space(8)

      Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: "🍅 PomoTask"
        color: root.contentForeground
        font.family: root.contentFontFamily
        font.pixelSize: Style.font.title
        font.bold: true
      }
    }
  }
}
