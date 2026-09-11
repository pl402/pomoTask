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

  DistractionOverlay {
    id: distractionOverlay
    service: root.service
    active: distractionMonitor.distractionActive
  }

  readonly property alias distractionOverlay: distractionOverlay

  BreakOverlay {
    id: breakOverlay
    service: root.service
  }

  readonly property alias breakOverlay: breakOverlay

  CelebrationOverlay {
    id: celebrationOverlay
    service: root.service
  }

  readonly property alias celebrationOverlay: celebrationOverlay

  Connections {
    target: root.service
    function onCelebrationRequested(taskTitle) {
      celebrationOverlay.celebrate(taskTitle)
    }
  }

  // -------------------------------------------------------------------------
  // Status Formatting & Helpers
  // -------------------------------------------------------------------------
  function truncateText(str, maxLen) {
    if (!str) return ""
    var s = String(str).trim()
    if (s.length <= maxLen) return s
    return s.substring(0, Math.max(1, maxLen - 1)) + "…"
  }

  function wrapText(str, maxLineLen, indent) {
    if (!str) return ""
    var s = String(str).trim()
    if (s.length <= maxLineLen) return s
    var words = s.split(/\s+/)
    var lines = []
    var currentLine = ""
    var ind = indent || ""

    for (var i = 0; i < words.length; i++) {
      var word = words[i]
      if (currentLine === "") {
        currentLine = word
      } else if ((currentLine.length + 1 + word.length) <= maxLineLen) {
        currentLine += " " + word
      } else {
        lines.push(currentLine)
        currentLine = ind + word
      }
    }
    if (currentLine !== "") {
      lines.push(currentLine)
    }
    return lines.join("\n")
  }

  readonly property string statusIcon: {
    if (service.isPaused) return ""  // nf-fa-pause
    return service.modeIcon
  }

  // Sin conexión con Google el glifo de alerta (nf-md-alert) ocupa el hueco del glifo de modo:
  // el widget nunca cambia de ancho. El tinte "urgent" y el tooltip completan el aviso.
  readonly property string leadGlyph: service.googleDisconnected ? "󰀦" : statusIcon

  // El título de la tarea ya no va en la barra (siempre salía cortado): lo sustituye un anillo
  // de progreso del pomodoro. El título completo sigue en el tooltip.
  readonly property real ringProgress: service.progress
  readonly property color contentColor: (button.active && button.useActiveColor) ? button.activeColor : button.foreground
  readonly property real ringSize: Math.max(10, Math.round(button.fontSize * 1.15))
  readonly property real glyphSlot: Math.round(button.fontSize * 1.45)

  readonly property string modeLabelEs: {
    if (service.isShortBreak) return "Descanso corto"
    if (service.isLongBreak) return "Descanso largo"
    return "Enfoque"
  }

  readonly property string tooltipStatusText: {
    var parts = []
    if (service.googleDisconnected) {
      parts.push(service.authRequired
        ? "Google Tasks: sesión expirada. Abre la TUI para iniciar sesión."
        : "Google Tasks: sin conexión. " + service.lastSyncLabel)
    }
    var state = service.isRunning ? "en curso" : (service.isPaused ? "en pausa" : "detenido")
    parts.push(root.modeLabelEs + " · " + Math.round(service.progress * 100) + "% · " + state)
    if (service.activeTaskTitle && service.activeTaskTitle !== "") {
      parts.push(wrapText(service.activeTaskTitle, 38))
    }
    return parts.join("\n")
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
    if ("celebrationOverlay" in target) target.celebrationOverlay = celebrationOverlay
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
  // La etiqueta interna del botón no se usa: el indicador se alinea con nuestra fila de contenido.
  readonly property real openPanelIndicatorWidth: root.vertical ? button.width : horizontalRow.implicitWidth
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

  // Anillo de progreso del pomodoro (se llena conforme avanza la fase actual).
  component ProgressRing: Canvas {
    id: ring
    property real progress: 0
    property color color: Color.foreground
    property bool paused: false
    property real thickness: Math.max(1.5, Math.round(width * 0.14))

    onProgressChanged: requestPaint()
    onColorChanged: requestPaint()
    onPausedChanged: requestPaint()
    onWidthChanged: requestPaint()
    onVisibleChanged: if (visible) requestPaint()
    Component.onCompleted: requestPaint()

    onPaint: {
      var ctx = getContext("2d")
      ctx.reset()
      var w = width, h = height
      var cx = w / 2, cy = h / 2
      var r = Math.min(w, h) / 2 - thickness / 2
      if (r <= 0) return
      ctx.lineWidth = thickness
      ctx.lineCap = "butt"

      // Pista completa, tenue
      ctx.beginPath()
      ctx.strokeStyle = Qt.rgba(color.r, color.g, color.b, 0.28)
      ctx.arc(cx, cy, r, 0, Math.PI * 2, false)
      ctx.stroke()

      // Progreso, desde las 12 en sentido horario (más tenue en pausa)
      var p = Math.max(0, Math.min(1, progress))
      if (p > 0) {
        ctx.beginPath()
        ctx.strokeStyle = Qt.rgba(color.r, color.g, color.b, paused ? 0.6 : 1.0)
        var start = -Math.PI / 2
        ctx.arc(cx, cy, r, start, start + Math.PI * 2 * p, false)
        ctx.stroke()
      }
    }
  }

  WidgetButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    // El contenido se dibuja abajo con ancho fijo; la etiqueta interna no se usa.
    text: ""
    labelVisible: false
    hasVisualContent: true
    fixedWidth: root.vertical ? -1 : Math.ceil(horizontalRow.implicitWidth + button.scaledHorizontalMargin * 2)
    fixedHeight: root.vertical ? Style.bar.iconSlot * 3 : -1
    horizontalMargin: 8.75
    verticalPadding: 8.75
    tooltipText: root.tooltipStatusText
    // Tinta el widget con el color "urgent" del tema mientras Google esté desconectado.
    active: root.service.googleDisconnected

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

    // Ancho constante para la hora: medimos "00:00" en vez de confiar en dígitos tabulares.
    TextMetrics {
      id: timeMetrics
      font.family: button.fontFamily
      font.pixelSize: button.fontSize
      text: "00:00"
    }

    // ---- Horizontal: [glifo de modo][mm:ss][anillo] ----
    Row {
      id: horizontalRow
      visible: !root.vertical
      anchors.centerIn: parent
      spacing: Style.space(6)

      Text {
        textFormat: Text.PlainText
        width: root.glyphSlot
        height: button.height
        text: root.leadGlyph
        color: root.contentColor
        font.family: button.fontFamily
        font.pixelSize: button.fontSize
        renderType: Text.NativeRendering
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        Behavior on color { ColorAnimation { duration: 160 } }
      }

      Text {
        textFormat: Text.PlainText
        width: Math.ceil(timeMetrics.advanceWidth)
        height: button.height
        text: service.formattedTime
        color: root.contentColor
        font.family: button.fontFamily
        font.pixelSize: button.fontSize
        renderType: Text.NativeRendering
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        Behavior on color { ColorAnimation { duration: 160 } }
      }

      ProgressRing {
        width: root.ringSize
        height: root.ringSize
        anchors.verticalCenter: parent.verticalCenter
        progress: root.ringProgress
        color: root.contentColor
        paused: service.isPaused
      }
    }

    // ---- Vertical: glifo / hora / anillo, apilados ----
    Column {
      visible: root.vertical
      anchors.fill: parent

      OpticalGlyph {
        width: button.width
        height: Style.bar.iconSlot
        text: root.leadGlyph
        fontFamily: button.fontFamily
        fontSize: button.fontSize
        color: root.contentColor
      }

      OpticalGlyph {
        width: button.width
        height: Style.bar.iconSlot
        text: service.formattedTime
        fontFamily: button.fontFamily
        fontSize: button.fontSize * 0.85
        color: root.contentColor
      }

      Item {
        width: button.width
        height: Style.bar.iconSlot
        ProgressRing {
          anchors.centerIn: parent
          width: root.ringSize
          height: root.ringSize
          progress: root.ringProgress
          color: root.contentColor
          paused: service.isPaused
        }
      }
    }
  }
}
