import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui

BarWidget {
  id: root
  moduleName: "io.github.pl402.pomotask"

  PomotaskService {
    id: service
  }

  readonly property alias service: service

  DistractionMonitor {
    id: distractionMonitor
    service: root.service
  }

  readonly property alias distractionMonitor: distractionMonitor

  BreakOverlay {
    id: breakOverlay
    service: root.service
  }

  readonly property alias breakOverlay: breakOverlay

  // -------------------------------------------------------------------------
  // Status Formatting & Helpers
  // -------------------------------------------------------------------------
  function truncateText(str, maxLen) {
    if (!str) return ""
    var s = String(str).trim()
    if (s.length <= maxLen) return s
    return s.substring(0, Math.max(1, maxLen - 1)) + "…"
  }

  readonly property string statusIcon: {
    if (service.isPaused) return "⏸"
    return service.modeIcon
  }

  readonly property string activeSnippet: service.activeTaskTitle && service.activeTaskTitle !== ""
    ? " | " + truncateText(service.activeTaskTitle, 20)
    : ""

  readonly property string displayText: statusIcon + " " + service.formattedTime + activeSnippet
  readonly property var verticalLines: [statusIcon, service.formattedTime]

  readonly property string tooltipStatusText: {
    var lines = []
    lines.push("PomoTask: " + service.modeLabel + " (" + (service.isRunning ? "Running" : (service.isPaused ? "Paused" : "Stopped")) + ")")
    lines.push("Time Remaining: " + service.formattedTime)
    lines.push("Completed Cycles: " + service.sessionPomodoros)
    if (service.activeTaskTitle && service.activeTaskTitle !== "") {
      lines.push("Active Task: " + service.activeTaskTitle)
    }
    return lines.join("\n")
  }

  // -------------------------------------------------------------------------
  // Panel Loading & Injection Contract
  // -------------------------------------------------------------------------
  function injectPanel() {
    var target = panelLoader.item
    if (!target) return
    if ("bar" in target) target.bar = root.bar
    if ("settings" in target) target.settings = root.settings
    if ("anchorItem" in target) target.anchorItem = button
    if ("hostWidget" in target) target.hostWidget = root
    if ("service" in target) target.service = root.service
  }

  onBarChanged: injectPanel()
  onSettingsChanged: injectPanel()

  Loader {
    id: panelLoader
    active: true
    source: Qt.resolvedUrl("Panel.qml")
    visible: false
    onLoaded: {
      root.injectPanel()
      Qt.callLater(root.injectPanel)
    }
  }

  // -------------------------------------------------------------------------
  // Shape Contract for Shell / Bar Summon / Toggle Routing
  // -------------------------------------------------------------------------
  readonly property bool opened: panelLoader.item ? panelLoader.item.opened === true : false

  function open() {
    if (panelLoader.item) {
      if (typeof panelLoader.item.openFromHotkey === "function") {
        panelLoader.item.openFromHotkey()
      } else if (typeof panelLoader.item.open === "function") {
        panelLoader.item.open()
      }
    }
  }

  function close() {
    if (panelLoader.item && typeof panelLoader.item.close === "function") {
      panelLoader.item.close()
    }
  }

  function toggle() {
    if (panelLoader.item && typeof panelLoader.item.toggle === "function") {
      panelLoader.item.toggle()
    }
  }

  function togglePanel() {
    toggle()
  }

  readonly property bool popoutSwitchClosing: panelLoader.item ? panelLoader.item.popoutSwitchClosing === true : false

  function closeForPopoutSwitch() {
    if (panelLoader.item && typeof panelLoader.item.closeForPopoutSwitch === "function") {
      panelLoader.item.closeForPopoutSwitch()
    }
  }

  function refresh() {
    service.fetchStatus()
    if (panelLoader.item && typeof panelLoader.item.refresh === "function") {
      panelLoader.item.refresh()
    }
  }

  // Indicator hint for open popout in bar
  readonly property real openPanelIndicatorWidth: button.labelWidth
  readonly property real openPanelIndicatorHeight: Math.max(Style.space(10), Math.round(Style.bar.iconSlot * 0.55))

  // -------------------------------------------------------------------------
  // IPC Handler
  // -------------------------------------------------------------------------
  IpcHandler {
    target: "io.github.pl402.pomotask"

    function refresh(): void { root.broadcast("refresh") }
    function open(): void { root.open() }
    function close(): void { root.close() }
    function show(): void { root.open() }
    function hide(): void { root.close() }
    function toggle(): void { root.toggle() }
    function toggleTimer(): void { service.timerToggle() }
    function startTimer(): void { service.timerStart() }
    function pauseTimer(): void { service.timerPause() }
    function skipTimer(): void { service.timerSkip() }
    function resetTimer(): void { service.timerReset() }
  }

  // -------------------------------------------------------------------------
  // Geometry & Button Layout
  // -------------------------------------------------------------------------
  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  WidgetButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: root.vertical ? "" : root.displayText
    labelVisible: !root.vertical
    hasVisualContent: root.vertical ? root.verticalLines.length > 0 : text !== ""
    fixedHeight: root.vertical ? root.verticalLines.length * Style.bar.iconSlot : -1
    horizontalMargin: 8.75
    verticalPadding: 8.75
    tooltipText: root.tooltipStatusText

    onPressed: function(b) {
      if (b === Qt.RightButton) {
        service.timerToggle()
      } else if (b === Qt.MiddleButton) {
        service.timerSkip()
      } else {
        root.toggle()
      }
    }

    onWheelMoved: function(delta) {
      if (delta !== 0) {
        service.timerToggle()
      }
    }

    Column {
      visible: root.vertical
      anchors.fill: parent

      Repeater {
        model: root.verticalLines

        OpticalGlyph {
          required property string modelData
          width: button.width
          height: Style.bar.iconSlot
          text: modelData
          fontFamily: button.fontFamily
          fontSize: modelData.length > 3
            ? button.fontSize * 0.85
            : button.fontSize
          color: button.foreground
        }
      }
    }
  }
}
