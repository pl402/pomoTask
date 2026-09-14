import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
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
  property var service: null
  property var celebrationOverlay: null
  readonly property var barIdentity: hostWidget || root

  readonly property var pomotaskService: service || (hostWidget && hostWidget.service ? hostWidget.service : fallbackService)

  PomotaskService {
    id: fallbackService
  }

  readonly property color contentForeground: bar ? bar.foreground : Color.foreground
  readonly property color dimColor: Qt.darker(contentForeground, 1.45)
  readonly property string contentFontFamily: bar ? bar.fontFamily : Style.font.family

  readonly property bool hasActiveTask: !!pomotaskService.activeTaskId && pomotaskService.activeTaskId !== ""

  readonly property var firstPendingTask: {
    var items = root.visibleTaskItems || []
    for (var i = 0; i < items.length; i++) {
      if (items[i] && items[i].task && !items[i].task.completed) {
        return items[i].task
      }
    }
    return null
  }

  // Id de la tarea copiada recientemente; se usa para mostrar un ✓ breve en su botón
  property string copiedTaskId: ""

  Timer {
    id: copiedFeedbackTimer
    interval: 1500
    repeat: false
    onTriggered: root.copiedTaskId = ""
  }

  // Copia al portapapeles la tarea: título y, si tiene, su descripción (notas)
  function copyTaskToClipboard(task) {
    if (!task) return
    var text = String(task.title || "").trim()
    var notes = String(task.notes || "").trim()
    if (notes !== "") {
      text = text === "" ? notes : text + "\n\n" + notes
    }
    if (text === "") return
    Quickshell.execDetached(["bash", "-c", "printf %s " + Util.shellQuote(text) + " | wl-copy"])
    root.copiedTaskId = String(task.id || "")
    copiedFeedbackTimer.restart()
  }

  function completeCurrentTask() {
    var taskTitle = ""
    var taskId = ""

    if (root.hasActiveTask) {
      taskId = pomotaskService.activeTaskId
      taskTitle = pomotaskService.activeTaskTitle
    } else if (root.firstPendingTask) {
      taskId = root.firstPendingTask.id
      taskTitle = root.firstPendingTask.title
    }

    // Reiniciar el cronómetro y tiempo restante
    pomotaskService.timerReset()

    if (taskId && taskId !== "") {
      pomotaskService.completeTask(taskId)
    }

    var titleToCelebrate = taskTitle || "Tarea"
    pomotaskService.triggerCelebration(titleToCelebrate)
    if (root.celebrationOverlay && typeof root.celebrationOverlay.celebrate === "function") {
      root.celebrationOverlay.celebrate(titleToCelebrate)
    }
  }

  // Navigation view: "main" (ultra-clean timer + active task) | "settings" (modes + anti-distraction rules)
  property string currentView: "main"
  property string selectedListId: "@all"
  property bool showCompletedTasks: false

  // Las tres acciones se distinguen a la vista: aviso pequeño abajo sin tapar nada,
  // pantalla completa oscurecida con la tarea encima, o la ventana desaparece a un
  // workspace especial y vuelve sola al terminar/pausar el trabajo.
  readonly property var actionOptions: [
    { value: "warn", label: "Aviso discreto abajo" },
    { value: "hud", label: "Pantalla de enfoque (oscurece todo)" },
    { value: "minimize", label: "Ocultar ventana hasta el descanso" }
  ]

  readonly property var listOptions: {
    var opts = [{ value: "@all", label: "Todas las listas" }]
    var lists = pomotaskService.taskLists || []
    for (var i = 0; i < lists.length; i++) {
      var item = lists[i]
      if (!item) continue
      var id = (typeof item === "object" && item.id) ? String(item.id) : String(item)
      var label = (typeof item === "object" && item.title && String(item.title).trim() !== "") ? String(item.title) : id
      if (id !== "@all") {
        opts.push({ value: id, label: label })
      }
    }
    return opts
  }

  function formatDueDate(isoStr) {
    if (!isoStr) return ""
    try {
      var d = new Date(isoStr)
      if (isNaN(d.getTime())) return ""
      var now = new Date()
      var isToday = d.getFullYear() === now.getFullYear() && d.getMonth() === now.getMonth() && d.getDate() === now.getDate()
      if (isToday) return "Hoy"
      var tomorrow = new Date(now.getTime() + 86400000)
      var isTomorrow = d.getFullYear() === tomorrow.getFullYear() && d.getMonth() === tomorrow.getMonth() && d.getDate() === tomorrow.getDate()
      if (isTomorrow) return "Mañana"
      return (d.getMonth() + 1) + "/" + d.getDate()
    } catch (e) {
      return ""
    }
  }

  function buildTaskTree(taskList, listFilter, showCompleted) {
    var list = []
    var topLevel = []
    var subtaskMap = {}
    var arr = taskList || []

    for (var i = 0; i < arr.length; i++) {
      var t = arr[i]
      if (!t) continue
      if (listFilter && listFilter !== "" && listFilter !== "@all" && t.list_id !== listFilter) {
        continue
      }
      if (!showCompleted && t.completed) {
        continue
      }

      if (t.parent_id && t.parent_id !== "") {
        if (!subtaskMap[t.parent_id]) subtaskMap[t.parent_id] = []
        subtaskMap[t.parent_id].push(t)
      } else {
        topLevel.push(t)
      }
    }

    for (var j = 0; j < topLevel.length; j++) {
      var parent = topLevel[j]
      list.push({ task: parent, isSubtask: false, depth: 0 })
      if (subtaskMap[parent.id]) {
        var children = subtaskMap[parent.id]
        for (var k = 0; k < children.length; k++) {
          list.push({ task: children[k], isSubtask: true, depth: 1 })
        }
      }
    }

    for (var pId in subtaskMap) {
      var alreadyIncluded = false
      for (var m = 0; m < topLevel.length; m++) {
        if (topLevel[m].id === pId) {
          alreadyIncluded = true
          break
        }
      }
      if (!alreadyIncluded) {
        var orphans = subtaskMap[pId]
        for (var o = 0; o < orphans.length; o++) {
          list.push({ task: orphans[o], isSubtask: true, depth: 1 })
        }
      }
    }

    return list
  }

  readonly property var visibleTaskItems: buildTaskTree(pomotaskService.tasks, root.selectedListId, root.showCompletedTasks)

  function addNewTask() {
    if (!newTaskField) return
    var title = String(newTaskField.text || "").trim()
    if (title === "") return
    var targetList = (root.selectedListId && root.selectedListId !== "" && root.selectedListId !== "@all")
      ? root.selectedListId
      : (root.listOptions.length > 1 ? root.listOptions[1].value : "@default")
    pomotaskService.createTask(title, targetList, "")
    newTaskField.text = ""
  }

  readonly property var blockedTitles: (pomotaskService.blocklist && Array.isArray(pomotaskService.blocklist.title_keywords))
    ? pomotaskService.blocklist.title_keywords
    : []

  readonly property var blockedApps: (pomotaskService.blocklist && Array.isArray(pomotaskService.blocklist.blocked_classes))
    ? pomotaskService.blocklist.blocked_classes
    : []

  readonly property var allowedTitles: (pomotaskService.blocklist && Array.isArray(pomotaskService.blocklist.allowed_title_keywords))
    ? pomotaskService.blocklist.allowed_title_keywords
    : []

  readonly property var allowedApps: (pomotaskService.blocklist && Array.isArray(pomotaskService.blocklist.allowed_classes))
    ? pomotaskService.blocklist.allowed_classes
    : []

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
    if (pomotaskService) {
      pomotaskService.fetchStatus()
    }
  }

  function openTui() {
    var cmd = "omarchy-launch-or-focus-tui pomotask-cli || omarchy-launch-terminal pomotask-cli || xdg-terminal-exec pomotask-cli || alacritty -e pomotask-cli || kitty -e pomotask-cli || foot -e pomotask-cli"
    if (root.bar && typeof root.bar.run === "function") {
      root.bar.run(cmd)
    } else {
      Util.execDetached(cmd)
    }
    root.close()
  }

  function addBlockedTitle() {
    if (!newBlockedTitleField) return
    var text = String(newBlockedTitleField.text || "").trim()
    if (text === "") return
    pomotaskService.blocklistAddTitle(text)
    newBlockedTitleField.text = ""
  }

  function addBlockedClass() {
    if (!newBlockedClassField) return
    var text = String(newBlockedClassField.text || "").trim()
    if (text === "") return
    pomotaskService.blocklistAddClass(text)
    newBlockedClassField.text = ""
  }

  function addAllowedTitle() {
    if (!newAllowedTitleField) return
    var text = String(newAllowedTitleField.text || "").trim()
    if (text === "") return
    pomotaskService.blocklistAddAllowedTitle(text)
    newAllowedTitleField.text = ""
  }

  // -------------------------------------------------------------------------
  // Estado derivado para la vista principal
  // -------------------------------------------------------------------------
  // Tarea activa completa (título, notas, lista) buscada en la caché del servicio.
  readonly property var activeTask: {
    if (!root.hasActiveTask) return null
    var items = pomotaskService.tasks || []
    for (var i = 0; i < items.length; i++) {
      if (items[i] && items[i].id === pomotaskService.activeTaskId) return items[i]
    }
    return null
  }
  readonly property string activeTaskNotes: activeTask ? String(activeTask.notes || "").trim() : ""

  // Subtareas de la tarea activa (pendientes primero, luego hechas).
  readonly property var activeSubtasks: {
    if (!root.hasActiveTask) return []
    var items = pomotaskService.tasks || []
    var pending = [], done = []
    for (var i = 0; i < items.length; i++) {
      var t = items[i]
      if (t && t.parent_id === pomotaskService.activeTaskId) (t.completed ? done : pending).push(t)
    }
    return pending.concat(done)
  }

  // Pendientes en la lista seleccionada (independiente de "Ver hechas").
  readonly property int pendingCount: buildTaskTree(pomotaskService.tasks, root.selectedListId, false).length

  // Color de fase derivado del tema: acento para enfoque, acento atenuado en descansos.
  readonly property color phaseColor: pomotaskService.isWork ? Color.accent : Util.alpha(Color.accent, 0.55)

  function modeLabel(mode) {
    if (mode === "short_break") return "DESCANSO CORTO"
    if (mode === "long_break") return "DESCANSO LARGO"
    return "ENFOQUE"
  }

  function stateLabel(state) {
    if (state === "running") return "en curso"
    if (state === "paused") return "pausado"
    return "listo"
  }

  readonly property string headerCaption: {
    if (pomotaskService.isWork && !pomotaskService.isStopped) {
      return "Pomodoro " + ((pomotaskService.sessionPomodoros % 4) + 1) + " de 4"
    }
    if (pomotaskService.isStopped && !root.hasActiveTask && pomotaskService.isWork) return "listo para empezar"
    return stateLabel(pomotaskService.state)
  }

  // La lista se pliega mientras el temporizador corre; el usuario puede desplegarla.
  property bool tasksForceExpanded: false
  readonly property bool tasksCollapsed: pomotaskService.isRunning && !tasksForceExpanded

  Connections {
    target: pomotaskService
    function onIsRunningChanged() { root.tasksForceExpanded = false }
  }

  // Reiniciar pide una segunda pulsación en 3 s en lugar de un diálogo.
  property bool resetArmed: false
  Timer {
    id: resetArmTimer
    interval: 3000
    repeat: false
    onTriggered: root.resetArmed = false
  }
  function requestReset() {
    if (root.resetArmed) {
      root.resetArmed = false
      resetArmTimer.stop()
      pomotaskService.timerReset()
    } else {
      root.resetArmed = true
      resetArmTimer.restart()
    }
  }

  // Cursor de teclado sobre la lista de tareas (-1 = ninguno).
  property int cursorIndex: -1
  function moveCursor(delta) {
    var n = root.visibleTaskItems.length
    if (n === 0) { root.cursorIndex = -1; return }
    if (root.tasksCollapsed) root.tasksForceExpanded = true
    if (root.cursorIndex < 0) root.cursorIndex = delta > 0 ? 0 : n - 1
    else root.cursorIndex = Math.max(0, Math.min(n - 1, root.cursorIndex + delta))
  }
  onCursorIndexChanged: {
    if (root.cursorIndex >= 0 && typeof listScroll !== "undefined" && listScroll) listScroll.ensureVisible(root.cursorIndex)
  }
  function cursorTask() {
    var items = root.visibleTaskItems
    if (root.cursorIndex < 0 || root.cursorIndex >= items.length) return null
    return items[root.cursorIndex].task
  }
  onVisibleTaskItemsChanged: {
    if (root.cursorIndex >= root.visibleTaskItems.length) root.cursorIndex = root.visibleTaskItems.length - 1
  }

  function toggleFocus(task) {
    if (!task) return
    if (pomotaskService.activeTaskId === task.id) pomotaskService.clearFocusTask()
    else pomotaskService.focusTask(task.id)
  }

  function completeTaskWithCelebration(task) {
    if (!task) return
    var willBeCompleted = !task.completed
    var taskTitle = task.title
    pomotaskService.completeTask(task.id)
    if (willBeCompleted) {
      if (pomotaskService.activeTaskId === task.id) pomotaskService.clearFocusTask()
      pomotaskService.triggerCelebration(taskTitle)
      if (root.celebrationOverlay && typeof root.celebrationOverlay.celebrate === "function") {
        root.celebrationOverlay.celebrate(taskTitle)
      }
    }
  }

  // Fila de chips mutuamente excluyentes que reparten el ancho completo (el
  // ButtonGroup del shell dimensiona cada chip a su contenido).
  component EqualChips: Row {
    id: chips
    property var options: []
    property string value: ""
    signal changed(string value)
    spacing: Style.space(6)

    Repeater {
      model: chips.options
      delegate: Button {
        required property var modelData
        width: (chips.width - chips.spacing * (chips.options.length - 1)) / chips.options.length
        text: modelData.label
        iconText: modelData.icon || ""
        selected: modelData.value === chips.value
        bordered: true
        foreground: root.contentForeground
        accent: Color.accent
        fontFamily: root.contentFontFamily
        fontSize: Style.font.bodySmall
        onClicked: chips.changed(modelData.value)
      }
    }
  }

  // Fila "Etiqueta ........ [-] 25 min [+]" para editar una duración en minutos.
  component DurationRow: Item {
    id: durationRow
    property string label: ""
    property string key: ""          // "focus" | "short" | "long"
    property int seconds: 0
    property int minMinutes: 1
    property int maxMinutes: 180
    readonly property int minutes: Math.round(seconds / 60)
    width: parent ? parent.width : implicitWidth
    implicitHeight: Style.space(34)

    function bump(delta) {
      var next = Math.max(minMinutes, Math.min(maxMinutes, minutes + delta))
      if (next !== minutes) pomotaskService.setDuration(key, next)
    }

    Text {
      anchors.left: parent.left
      anchors.verticalCenter: parent.verticalCenter
      textFormat: Text.PlainText
      text: durationRow.label
      color: root.contentForeground
      font.family: root.contentFontFamily
      font.pixelSize: Style.font.body
    }

    Row {
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      spacing: Style.space(6)

      PanelActionButton {
        size: Style.space(24)
        fontSize: Style.font.bodySmall
        iconText: "−"
        bordered: true
        enabled: durationRow.minutes > durationRow.minMinutes
        foreground: root.dimColor
        hoverColor: Color.accent
        tooltipText: "Un minuto menos"
        anchors.verticalCenter: parent.verticalCenter
        onClicked: durationRow.bump(-1)
      }

      Item {
        width: Style.space(58)
        height: Style.space(24)

        Text {
          anchors.centerIn: parent
          textFormat: Text.PlainText
          text: durationRow.minutes + " min"
          color: root.contentForeground
          font.family: root.contentFontFamily
          font.pixelSize: Style.font.body
          font.bold: true
        }
      }

      PanelActionButton {
        size: Style.space(24)
        fontSize: Style.font.bodySmall
        iconText: "+"
        bordered: true
        enabled: durationRow.minutes < durationRow.maxMinutes
        foreground: root.dimColor
        hoverColor: Color.accent
        tooltipText: "Un minuto más"
        anchors.verticalCenter: parent.verticalCenter
        onClicked: durationRow.bump(1)
      }
    }
  }

  // Altura máxima de contenido del panel (misma regla que fittedContentHeight): la lista
  // de tareas se ajusta a lo que sobra para que cabecera, anillo, tarjeta, alta rápida y
  // pie se vean siempre a la vez; solo la lista hace scroll.
  readonly property real panelMaxContentHeight: Math.min(Style.space(760),
    panel.availableCardHeight > 0 ? panel.availableCardHeight : Style.space(760)) - panel.verticalContentInset

  // Suma la altura de los hijos visibles de una Column (más su spacing) saltando `skip`.
  function visibleHeightExcept(col, skip) {
    var h = 0
    var n = 0
    for (var i = 0; i < col.children.length; i++) {
      var c = col.children[i]
      if (!c || !c.visible || c === skip) continue
      h += c.height
      n++
    }
    return h + col.spacing * Math.max(0, n - 1)
  }

  // Pestaña activa de las listas de bloqueo en Ajustes: "titles" | "apps" | "allowed"
  property string blocklistTab: "titles"

  readonly property string shortcutsHelp: "Espacio/P: iniciar o pausar · S: saltar fase · R: sincronizar\n"
    + "J/K: mover cursor · Enter: enfocar · C: completar · Y: copiar\n"
    + "Esc: quitar cursor o cerrar · Q: cerrar"

  // -------------------------------------------------------------------------
  // KeyboardPanel
  // -------------------------------------------------------------------------
  KeyboardPanel {
    id: panel
    anchorItem: root.anchorItem
    owner: root.barIdentity
    bar: root.bar
    open: root.opened
    focusTarget: keyCatcher
    contentWidth: panel.fittedContentWidth(Style.space(380))
    contentHeight: panel.fittedContentHeight(panelColumn.implicitHeight, Style.space(760))

    PanelKeyCatcher {
      id: keyCatcher
      anchors.fill: parent
      blocked: (typeof newTaskField !== "undefined" && newTaskField && newTaskField.activeFocus)
        || (typeof newBlockedTitleField !== "undefined" && newBlockedTitleField && newBlockedTitleField.activeFocus)
        || (typeof newBlockedClassField !== "undefined" && newBlockedClassField && newBlockedClassField.activeFocus)
        || (typeof newAllowedTitleField !== "undefined" && newAllowedTitleField && newAllowedTitleField.activeFocus)
        || (typeof listDropdown !== "undefined" && listDropdown && listDropdown.popupOpen)
        || (typeof actionDropdown !== "undefined" && actionDropdown && actionDropdown.popupOpen)

      // Enter: con cursor en la lista enfoca/desenfoca esa tarea; si no, alterna el temporizador.
      onActivateRequested: {
        var t = root.cursorTask()
        if (root.currentView === "main" && t) root.toggleFocus(t)
        else pomotaskService.timerToggle()
      }
      onCloseRequested: {
        if (root.cursorIndex >= 0) {
          root.cursorIndex = -1
        } else if (root.currentView === "settings") {
          root.currentView = "main"
        } else {
          root.close()
        }
      }
      onTabRequested: function(direction) { root.switchPanel(direction) }
      onTextKey: function(t) {
        if (t === " " || t === "p" || t === "P") {
          pomotaskService.timerToggle()
        } else if (t === "s" || t === "S") {
          pomotaskService.timerSkip()
        } else if (t === "r" || t === "R") {
          pomotaskService.syncTasks()
        } else if (t === "q" || t === "Q") {
          root.close()
        } else if (root.currentView === "main" && (t === "j" || t === "J")) {
          root.moveCursor(1)
        } else if (root.currentView === "main" && (t === "k" || t === "K")) {
          root.moveCursor(-1)
        } else if (root.currentView === "main" && (t === "c" || t === "C")) {
          root.completeTaskWithCelebration(root.cursorTask())
        } else if (root.currentView === "main" && (t === "y" || t === "Y")) {
          var ct = root.cursorTask()
          if (ct) root.copyTaskToClipboard(ct)
          else if (root.activeTask) root.copyTaskToClipboard(root.activeTask)
        }
      }

      ScrollView {
        id: scrollArea
        anchors.fill: parent
        clip: true
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        ScrollBar.vertical.policy: panelColumn.implicitHeight > height ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff

        Column {
          id: panelColumn
          width: scrollArea.availableWidth
          spacing: Style.space(12)

          // =================================================================
          // VISTA 1: PRINCIPAL (anillo + tarea en foco + lista)
          // =================================================================
          Column {
            id: mainViewColumn
            visible: root.currentView === "main"
            width: parent.width
            spacing: Style.space(12)

            // Cabecera: modo y estado a la izquierda | sincronizar y ajustes a la derecha
            Item {
              width: parent.width
              implicitHeight: Math.max(modeRow.implicitHeight, headerActionsRow.implicitHeight)

              Row {
                id: modeRow
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                spacing: Style.space(8)

                Text {
                  textFormat: Text.PlainText
                  text: root.modeLabel(pomotaskService.mode)
                  color: Color.accent
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.caption
                  font.bold: true
                  font.letterSpacing: 1
                  anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                  textFormat: Text.PlainText
                  text: root.headerCaption
                  color: root.dimColor
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.caption
                  anchors.verticalCenter: parent.verticalCenter
                }
              }

              Row {
                id: headerActionsRow
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                spacing: Style.space(4)

                // Sync button
                PanelActionButton {
                  size: Style.space(26)
                  fontSize: Style.font.body
                  iconText: ""
                  foreground: root.contentForeground
                  hoverColor: Color.accent
                  tooltipText: "Sincronizar con Google Tasks (R) · " + pomotaskService.lastSyncLabel
                  onClicked: pomotaskService.syncTasks()
                }

                // Settings button
                PanelActionButton {
                  size: Style.space(26)
                  fontSize: Style.font.body
                  iconText: ""
                  foreground: root.contentForeground
                  hoverColor: Color.accent
                  tooltipText: "Ajustes, duraciones y bloqueo"
                  onClicked: root.currentView = "settings"
                }
              }
            }

            // -----------------------------------------------------------------
            // Aviso de conexión con Google Tasks (token expirado, sin red, etc.)
            // -----------------------------------------------------------------
            BorderSurface {
              id: googleStatusBanner
              // Rojo (urgent) si se perdió la conexión; acento si solo hay cambios pendientes de subir.
              readonly property bool warning: pomotaskService.googleDisconnected
              readonly property color tone: warning ? Color.urgent : Color.accent
              visible: pomotaskService.googleDisconnected || pomotaskService.pendingChanges > 0
              width: parent.width
              color: Util.alpha(tone, 0.14)
              borderSpec: Border.flat(Util.alpha(tone, 0.55), 1)
              radius: Style.cornerRadius
              implicitHeight: googleStatusRow.implicitHeight + Style.space(16)

              Row {
                id: googleStatusRow
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.leftMargin: Style.space(10)
                anchors.rightMargin: Style.space(10)
                spacing: Style.space(8)

                Text {
                  id: googleStatusIcon
                  textFormat: Text.PlainText
                  anchors.verticalCenter: parent.verticalCenter
                  text: googleStatusBanner.warning ? "󰀦" : "󰕒"
                  color: googleStatusBanner.tone
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.body
                }

                Text {
                  id: googleStatusText
                  textFormat: Text.PlainText
                  anchors.verticalCenter: parent.verticalCenter
                  width: parent.width - googleStatusIcon.implicitWidth - googleStatusActionBtn.implicitWidth - parent.spacing * 2
                  text: pomotaskService.googleStatusMessage
                  color: root.contentForeground
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.caption
                  wrapMode: Text.WordWrap
                  maximumLineCount: 3
                  elide: Text.ElideRight
                }

                Button {
                  id: googleStatusActionBtn
                  anchors.verticalCenter: parent.verticalCenter
                  text: pomotaskService.authRequired
                    ? "Iniciar sesión"
                    : (googleStatusBanner.warning ? "Reintentar" : "Sincronizar")
                  fontSize: Style.font.caption
                  bordered: true
                  foreground: root.contentForeground
                  accent: googleStatusBanner.tone
                  tooltipText: pomotaskService.authRequired
                    ? "Abre la TUI en una terminal para volver a autenticarte con Google"
                    : "Sincronizar con Google y subir los cambios pendientes"
                  onClicked: {
                    if (pomotaskService.authRequired) {
                      root.openTui()
                    } else {
                      pomotaskService.syncTasks()
                    }
                  }
                }
              }
            }

            // -----------------------------------------------------------------
            // Héroe: anillo de progreso con el tiempo dentro + puntos de ciclo
            // -----------------------------------------------------------------
            Column {
              width: parent.width
              spacing: Style.space(14)

              Item {
                id: ringWrap
                readonly property int ringSize: Style.space(168)
                width: parent.width
                implicitHeight: ringSize

                Canvas {
                  id: ringCanvas
                  anchors.horizontalCenter: parent.horizontalCenter
                  width: ringWrap.ringSize
                  height: ringWrap.ringSize

                  property real progress: pomotaskService.progress
                  property color trackColor: Style.selectedFillFor(root.contentForeground, Color.accent)
                  property color fillColor: root.phaseColor
                  property real fillOpacity: pomotaskService.isPaused ? 0.55 : 1.0

                  Behavior on progress {
                    NumberAnimation { duration: 500; easing.type: Easing.OutCubic }
                  }

                  onProgressChanged: requestPaint()
                  onTrackColorChanged: requestPaint()
                  onFillColorChanged: requestPaint()
                  onFillOpacityChanged: requestPaint()

                  onPaint: {
                    var ctx = getContext("2d")
                    ctx.reset()
                    var stroke = Style.space(8)
                    var cx = width / 2
                    var cy = height / 2
                    var r = (Math.min(width, height) - stroke) / 2
                    ctx.lineWidth = stroke
                    ctx.lineCap = "round"

                    ctx.strokeStyle = Qt.rgba(trackColor.r, trackColor.g, trackColor.b, trackColor.a)
                    ctx.beginPath()
                    ctx.arc(cx, cy, r, 0, Math.PI * 2)
                    ctx.stroke()

                    var p = Math.max(0, Math.min(1, progress))
                    if (p > 0.002) {
                      ctx.strokeStyle = Qt.rgba(fillColor.r, fillColor.g, fillColor.b, fillColor.a * fillOpacity)
                      ctx.beginPath()
                      ctx.arc(cx, cy, r, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * p)
                      ctx.stroke()
                    }
                  }
                }

                Column {
                  anchors.centerIn: ringCanvas
                  spacing: Style.space(2)

                  Text {
                    textFormat: Text.PlainText
                    text: pomotaskService.formattedTime
                    color: pomotaskService.isStopped && !root.hasActiveTask ? root.dimColor : root.contentForeground
                    font.family: root.contentFontFamily
                    font.pixelSize: Math.round(Style.font.displayLarge * 1.4)
                    font.bold: true
                    horizontalAlignment: Text.AlignHCenter
                    anchors.horizontalCenter: parent.horizontalCenter
                  }

                  Text {
                    textFormat: Text.PlainText
                    text: pomotaskService.isBreak && pomotaskService.isRunning ? "DESCANSO" : root.stateLabel(pomotaskService.state).toUpperCase()
                    color: root.dimColor
                    font.family: root.contentFontFamily
                    font.pixelSize: Style.font.caption
                    font.letterSpacing: 1
                    horizontalAlignment: Text.AlignHCenter
                    anchors.horizontalCenter: parent.horizontalCenter
                  }
                }
              }

              // Puntos del ciclo: hechos en acento, el actual con anillo, el resto atenuado
              Row {
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Style.space(10)

                Repeater {
                  model: 4
                  delegate: Rectangle {
                    required property int index
                    readonly property int doneInCycle: pomotaskService.sessionPomodoros % 4
                    readonly property bool isDone: index < doneInCycle
                    readonly property bool isCurrent: index === doneInCycle && pomotaskService.isWork
                    width: Style.space(8)
                    height: Style.space(8)
                    radius: width / 2
                    color: isDone ? Color.accent : (isCurrent ? "transparent" : Style.selectedFillFor(root.contentForeground, Color.accent))
                    border.width: isCurrent ? 2 : 0
                    border.color: Color.accent
                  }
                }
              }
            }

            // -----------------------------------------------------------------
            // Controles: Reiniciar | Iniciar/Pausar (primario) | Saltar
            // -----------------------------------------------------------------
            Row {
              anchors.horizontalCenter: parent.horizontalCenter
              spacing: Style.space(12)

              Button {
                width: root.resetArmed ? implicitWidth : Style.space(40)
                verticalPadding: Style.space(11)
                horizontalPadding: root.resetArmed ? Style.space(10) : 0
                iconText: ""
                text: root.resetArmed ? "¿Reiniciar?" : ""
                fontSize: Style.font.caption
                bordered: true
                foreground: root.resetArmed ? Color.urgent : root.dimColor
                accent: root.resetArmed ? Color.urgent : Color.accent
                tooltipText: root.resetArmed ? "Pulsa otra vez para confirmar" : "Reiniciar temporizador (pide confirmación)"
                anchors.verticalCenter: parent.verticalCenter
                onClicked: root.requestReset()
              }

              Button {
                width: Style.space(160)
                verticalPadding: Style.space(12)
                iconText: pomotaskService.isRunning ? "" : ""
                iconSize: Style.font.iconLarge
                text: pomotaskService.isRunning ? "Pausar" : (pomotaskService.isPaused ? "Continuar" : "Iniciar")
                fontSize: Style.font.subtitle
                selected: true
                bordered: true
                foreground: root.contentForeground
                accent: Color.accent
                tooltipText: pomotaskService.isRunning ? "Pausar temporizador (Espacio)" : "Iniciar temporizador (Espacio)"
                anchors.verticalCenter: parent.verticalCenter
                onClicked: pomotaskService.timerToggle()
              }

              Button {
                width: Style.space(40)
                verticalPadding: Style.space(11)
                horizontalPadding: 0
                iconText: ""
                bordered: true
                foreground: root.dimColor
                accent: Color.accent
                tooltipText: "Saltar fase (S)"
                anchors.verticalCenter: parent.verticalCenter
                onClicked: pomotaskService.timerSkip()
              }
            }

            // -----------------------------------------------------------------
            // Tarjeta de descanso (solo en descansos)
            // -----------------------------------------------------------------
            BorderSurface {
              visible: pomotaskService.isBreak
              width: parent.width
              implicitHeight: breakColumn.implicitHeight + Style.space(28)
              radius: Style.cornerRadius
              color: Style.hoverFillFor(root.contentForeground, Color.accent)
              borderSpec: Border.controlSpec("hover-cursor", root.contentForeground, Color.accent)

              Column {
                id: breakColumn
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.leftMargin: Style.space(12)
                anchors.rightMargin: Style.space(12)
                spacing: Style.space(6)

                Text {
                  width: parent.width
                  textFormat: Text.PlainText
                  text: pomotaskService.isLongBreak ? "Descanso largo: levántate y desconecta" : "Aléjate de la pantalla un momento"
                  color: root.contentForeground
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.subtitle
                  font.bold: true
                  horizontalAlignment: Text.AlignHCenter
                  wrapMode: Text.Wrap
                }

                Text {
                  width: parent.width
                  visible: root.hasActiveTask
                  textFormat: Text.PlainText
                  text: "Al terminar vuelves a: " + pomotaskService.activeTaskTitle
                  color: root.dimColor
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.bodySmall
                  horizontalAlignment: Text.AlignHCenter
                  wrapMode: Text.Wrap
                  maximumLineCount: 2
                  elide: Text.ElideRight
                }
              }
            }

            // -----------------------------------------------------------------
            // Tarjeta de la tarea en foco (título, notas, subtareas, completar)
            // -----------------------------------------------------------------
            BorderSurface {
              id: activeTaskCard
              visible: !pomotaskService.isBreak && root.hasActiveTask
              width: parent.width
              implicitHeight: activeTaskColumn.implicitHeight + Style.space(24)
              radius: Style.cornerRadius
              color: Style.hoverFillFor(root.contentForeground, Color.accent)
              borderSpec: Border.controlSpec("focus", root.contentForeground, Color.accent)

              Column {
                id: activeTaskColumn
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.leftMargin: Style.space(12)
                anchors.rightMargin: Style.space(12)
                spacing: Style.space(8)

                Item {
                  width: parent.width
                  implicitHeight: Math.max(activeLabel.implicitHeight, activeActions.implicitHeight)

                  Text {
                    id: activeLabel
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    textFormat: Text.PlainText
                    text: "TAREA EN FOCO"
                    color: Color.accent
                    font.family: root.contentFontFamily
                    font.pixelSize: Style.font.caption
                    font.bold: true
                    font.letterSpacing: 1
                  }

                  Row {
                    id: activeActions
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Style.space(2)

                    PanelActionButton {
                      readonly property bool justCopied: root.copiedTaskId !== "" && root.copiedTaskId === pomotaskService.activeTaskId
                      size: Style.space(22)
                      iconText: justCopied ? "󰄬" : "󰆏"
                      foreground: justCopied ? Color.accent : root.dimColor
                      hoverColor: Color.accent
                      tooltipText: justCopied ? "¡Copiado!" : "Copiar tarea y descripción (Y)"
                      onClicked: root.copyTaskToClipboard(root.activeTask || { id: pomotaskService.activeTaskId, title: pomotaskService.activeTaskTitle })
                    }

                    PanelActionButton {
                      size: Style.space(22)
                      iconText: ""
                      foreground: root.dimColor
                      hoverColor: Color.urgent
                      tooltipText: "Quitar el foco de esta tarea"
                      onClicked: pomotaskService.clearFocusTask()
                    }
                  }
                }

                Text {
                  width: parent.width
                  textFormat: Text.PlainText
                  text: pomotaskService.activeTaskTitle
                  color: root.contentForeground
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.title
                  font.bold: true
                  wrapMode: Text.Wrap
                }

                Text {
                  width: parent.width
                  visible: root.activeTaskNotes !== ""
                  textFormat: Text.PlainText
                  text: root.activeTaskNotes
                  color: root.dimColor
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.bodySmall
                  wrapMode: Text.Wrap
                }

                Column {
                  width: parent.width
                  visible: root.activeSubtasks.length > 0
                  spacing: Style.space(4)

                  Repeater {
                    model: root.activeSubtasks
                    delegate: Row {
                      required property var modelData
                      width: parent.width
                      spacing: Style.space(6)

                      PanelActionButton {
                        size: Style.space(20)
                        fontSize: Style.font.bodySmall
                        iconText: modelData.completed ? "󰄲" : "󰄱"
                        foreground: modelData.completed ? Color.accent : root.dimColor
                        hoverColor: Color.accent
                        tooltipText: modelData.completed ? "Subtarea completada" : "Marcar subtarea como completada"
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked: root.completeTaskWithCelebration(modelData)
                      }

                      Text {
                        width: parent.width - Style.space(20) - parent.spacing
                        textFormat: Text.PlainText
                        text: modelData.title
                        color: modelData.completed ? root.dimColor : root.contentForeground
                        font.family: root.contentFontFamily
                        font.pixelSize: Style.font.bodySmall
                        font.strikeout: !!modelData.completed
                        wrapMode: Text.Wrap
                        anchors.verticalCenter: parent.verticalCenter
                      }
                    }
                  }
                }

                Button {
                  width: parent.width
                  iconText: "󰄲"
                  text: "Completar tarea"
                  bordered: true
                  foreground: Color.accent
                  accent: Color.accent
                  fontSize: Style.font.body
                  tooltipText: "Marcar la tarea en foco como terminada (C con el cursor en la lista)"
                  onClicked: root.completeTaskWithCelebration(root.activeTask || { id: pomotaskService.activeTaskId, title: pomotaskService.activeTaskTitle, completed: false })
                }
              }
            }

            // -----------------------------------------------------------------
            // Estado vacío: sin tarea en foco
            // -----------------------------------------------------------------
            BorderSurface {
              visible: !pomotaskService.isBreak && !root.hasActiveTask
              width: parent.width
              implicitHeight: emptyColumn.implicitHeight + Style.space(32)
              radius: Style.cornerRadius
              color: "transparent"
              borderSpec: Border.flat(Util.alpha(root.contentForeground, 0.25), 1)

              Column {
                id: emptyColumn
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.leftMargin: Style.space(12)
                anchors.rightMargin: Style.space(12)
                spacing: Style.space(6)

                Text {
                  textFormat: Text.PlainText
                  text: ""
                  color: Color.muted
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.display
                  anchors.horizontalCenter: parent.horizontalCenter
                }

                Text {
                  width: parent.width
                  textFormat: Text.PlainText
                  text: "Sin tarea en foco"
                  color: root.contentForeground
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.subtitle
                  font.bold: true
                  horizontalAlignment: Text.AlignHCenter
                }

                Text {
                  width: parent.width
                  textFormat: Text.PlainText
                  text: "Enfoca una tarea de la lista con "
                  color: root.dimColor
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.bodySmall
                  horizontalAlignment: Text.AlignHCenter
                  wrapMode: Text.Wrap
                }
              }
            }

            // -----------------------------------------------------------------
            // Tareas: plegadas mientras corre el temporizador
            // -----------------------------------------------------------------
            PanelSeparator {
              foreground: root.contentForeground
            }

            BorderSurface {
              id: collapsedTasksRow
              visible: root.tasksCollapsed
              width: parent.width
              implicitHeight: Style.space(34)
              radius: Style.cornerRadius
              color: collapsedMouse.containsMouse ? Style.selectedFillFor(root.contentForeground, Color.accent) : Style.hoverFillFor(root.contentForeground, Color.accent)

              Row {
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                anchors.leftMargin: Style.space(8)
                spacing: Style.space(8)

                PanelSectionHeader {
                  text: "TAREAS"
                  foreground: root.contentForeground
                  fontFamily: root.contentFontFamily
                  anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                  textFormat: Text.PlainText
                  text: root.pendingCount + (root.pendingCount === 1 ? " pendiente" : " pendientes")
                  color: root.dimColor
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.caption
                  anchors.verticalCenter: parent.verticalCenter
                }
              }

              Text {
                anchors.right: parent.right
                anchors.rightMargin: Style.space(8)
                anchors.verticalCenter: parent.verticalCenter
                textFormat: Text.PlainText
                text: "Mostrar "
                color: root.dimColor
                font.family: root.contentFontFamily
                font.pixelSize: Style.font.caption
              }

              MouseArea {
                id: collapsedMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.tasksForceExpanded = true
              }
            }

            Column {
              id: tasksSection
              visible: !root.tasksCollapsed
              width: parent.width
              spacing: Style.space(10)

              Item {
                width: parent.width
                implicitHeight: Math.max(tasksHeaderRow.implicitHeight, tasksHeaderActions.implicitHeight)

                Row {
                  id: tasksHeaderRow
                  anchors.left: parent.left
                  anchors.verticalCenter: parent.verticalCenter
                  spacing: Style.space(8)

                  PanelSectionHeader {
                    text: "TAREAS"
                    foreground: root.contentForeground
                    fontFamily: root.contentFontFamily
                    anchors.verticalCenter: parent.verticalCenter
                  }

                  Text {
                    textFormat: Text.PlainText
                    text: root.pendingCount + (root.pendingCount === 1 ? " pendiente" : " pendientes")
                    color: root.dimColor
                    font.family: root.contentFontFamily
                    font.pixelSize: Style.font.caption
                    anchors.verticalCenter: parent.verticalCenter
                  }
                }

                Row {
                  id: tasksHeaderActions
                  anchors.right: parent.right
                  anchors.verticalCenter: parent.verticalCenter
                  spacing: Style.space(2)

                  Button {
                    text: root.showCompletedTasks ? "Ocultar hechas" : "Ver hechas"
                    fontSize: Style.font.caption
                    bordered: false
                    foreground: root.dimColor
                    accent: Color.accent
                    onClicked: root.showCompletedTasks = !root.showCompletedTasks
                  }

                  Button {
                    visible: pomotaskService.isRunning
                    text: "Plegar"
                    fontSize: Style.font.caption
                    bordered: false
                    foreground: root.dimColor
                    accent: Color.accent
                    tooltipText: "Ocultar la lista mientras corre el temporizador"
                    onClicked: { root.tasksForceExpanded = false; root.cursorIndex = -1 }
                  }
                }
              }

              // Selector de lista (solo si hay más de una)
              Dropdown {
                id: listDropdown
                label: "Lista"
                showLabel: false
                visible: root.listOptions.length > 2
                width: parent.width
                value: root.selectedListId
                options: root.listOptions
                onChanged: function(v) { root.selectedListId = v; root.cursorIndex = -1 }
              }

              // Lista con scroll propio dentro de un marco con borde: ocupa lo que sobra del panel
              BorderSurface {
                id: listFrame
                width: parent.width
                readonly property int inset: Style.space(4)
                readonly property real maxHeight: root.panelMaxContentHeight
                  - (root.visibleHeightExcept(mainViewColumn, tasksSection) + mainViewColumn.spacing)
                  - (root.visibleHeightExcept(tasksSection, listFrame) + tasksSection.spacing)
                height: Math.max(Style.space(96), Math.min(listColumn.implicitHeight + inset * 2, maxHeight))
                radius: Style.cornerRadius
                color: "transparent"
                borderSpec: Border.controlSpec("normal", root.contentForeground, Color.accent)

              Flickable {
                id: listScroll
                anchors.fill: parent
                anchors.margins: listFrame.inset
                contentWidth: width
                contentHeight: listColumn.implicitHeight
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                flickableDirection: Flickable.VerticalFlick

                ScrollBar.vertical: ScrollBar {
                  policy: listScroll.contentHeight > listScroll.height ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
                }

                // Mantiene visible la fila del cursor de teclado
                function ensureVisible(index) {
                  var it = taskRepeater.itemAt(index)
                  if (!it) return
                  if (it.y < contentY) contentY = it.y
                  else if (it.y + it.height > contentY + height) contentY = Math.max(0, it.y + it.height - height)
                }

                  Column {
                    id: listColumn
                    width: listScroll.width
                    spacing: Style.space(2)

                    Repeater {
                      id: taskRepeater
                      model: root.visibleTaskItems

                      delegate: BorderSurface {
                        id: taskRow
                        required property var modelData
                        required property int index

                        readonly property var itemData: modelData
                        readonly property bool isFocused: pomotaskService.activeTaskId === itemData.task.id
                        readonly property bool hasCursor: root.cursorIndex === index

                        width: listColumn.width
                        implicitHeight: taskContent.implicitHeight + Style.space(10)
                        radius: Style.cornerRadius
                        color: isFocused
                          ? Style.selectedFillFor(root.contentForeground, Color.accent)
                          : (hasCursor ? Style.hoverFillFor(root.contentForeground, Color.accent) : "transparent")
                        borderSpec: isFocused
                          ? Border.controlSpec("focus", root.contentForeground, Color.accent)
                          : (hasCursor ? Border.controlSpec("hover-cursor", root.contentForeground, Color.accent) : Border.none())

                        // Marca del cursor de teclado
                        Rectangle {
                          visible: taskRow.hasCursor
                          anchors.left: parent.left
                          anchors.top: parent.top
                          anchors.bottom: parent.bottom
                          anchors.topMargin: Style.space(6)
                          anchors.bottomMargin: Style.space(6)
                          width: 2
                          radius: 1
                          color: Color.accent
                        }

                        Row {
                          id: taskContent
                          anchors.left: parent.left
                          anchors.right: parent.right
                          anchors.verticalCenter: parent.verticalCenter
                          anchors.leftMargin: (itemData.isSubtask ? Style.space(24) : Style.space(8))
                          anchors.rightMargin: Style.space(8)
                          spacing: Style.space(8)

                          // Completar / checkbox
                          PanelActionButton {
                            size: Style.space(22)
                            iconText: itemData.task.completed ? "󰄲" : "󰄱"
                            foreground: itemData.task.completed ? Color.accent : root.contentForeground
                            hoverColor: Color.accent
                            tooltipText: itemData.task.completed ? "Completada" : "Marcar como completada (C)"
                            anchors.verticalCenter: parent.verticalCenter
                            onClicked: root.completeTaskWithCelebration(itemData.task)
                          }

                          // Título y badges
                          Column {
                            width: parent.width - Style.space(22) * 3 - parent.spacing * 3 - (itemData.isSubtask ? Style.space(16) : 0)
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: Style.space(3)

                            Text {
                              textFormat: Text.PlainText
                              text: (itemData.isSubtask ? "↳ " : "") + itemData.task.title
                              color: itemData.task.completed ? root.dimColor : root.contentForeground
                              font.family: root.contentFontFamily
                              font.pixelSize: Style.font.body
                              font.strikeout: itemData.task.completed
                              wrapMode: Text.Wrap
                              width: parent.width
                            }

                            Row {
                              spacing: Style.space(6)
                              visible: (itemData.task.due && itemData.task.due !== "") || itemData.task.pomodoros > 0

                              BorderSurface {
                                visible: !!itemData.task.due && itemData.task.due !== ""
                                color: Style.hoverFillFor(root.contentForeground, Color.accent)
                                radius: Style.cornerRadius
                                implicitWidth: dueLabel.implicitWidth + Style.space(8)
                                implicitHeight: dueLabel.implicitHeight + Style.space(2)

                                Text {
                                  id: dueLabel
                                  textFormat: Text.PlainText
                                  anchors.centerIn: parent
                                  text: "󰃭 " + root.formatDueDate(itemData.task.due)
                                  color: root.contentForeground
                                  font.family: root.contentFontFamily
                                  font.pixelSize: Style.font.caption
                                }
                              }

                              Text {
                                visible: itemData.task.pomodoros > 0
                                textFormat: Text.PlainText
                                text: " " + itemData.task.pomodoros
                                color: Color.accent
                                font.family: root.contentFontFamily
                                font.pixelSize: Style.font.caption
                                font.bold: true
                                anchors.verticalCenter: parent.verticalCenter
                              }
                            }
                          }

                          // Copiar (título + descripción)
                          PanelActionButton {
                            readonly property bool justCopied: root.copiedTaskId !== "" && root.copiedTaskId === itemData.task.id
                            size: Style.space(22)
                            iconText: justCopied ? "󰄬" : "󰆏"
                            foreground: justCopied ? Color.accent : root.dimColor
                            hoverColor: Color.accent
                            tooltipText: justCopied ? "¡Copiado!" : (itemData.task.notes && itemData.task.notes !== "" ? "Copiar tarea y descripción (Y)" : "Copiar tarea (Y)")
                            anchors.verticalCenter: parent.verticalCenter
                            onClicked: root.copyTaskToClipboard(itemData.task)
                          }

                          // Enfocar
                          PanelActionButton {
                            size: Style.space(22)
                            iconText: ""
                            foreground: isFocused ? Color.accent : root.dimColor
                            hoverColor: Color.accent
                            tooltipText: isFocused ? "Quitar foco (Enter)" : "Enfocar esta tarea (Enter)"
                            anchors.verticalCenter: parent.verticalCenter
                            onClicked: root.toggleFocus(itemData.task)
                          }
                        }

                        // El hover del ratón mueve el cursor: un solo resaltado en pantalla.
                        MouseArea {
                          anchors.fill: parent
                          hoverEnabled: true
                          acceptedButtons: Qt.NoButton
                          onEntered: root.cursorIndex = index
                        }
                      }
                    }

                    Text {
                      visible: root.visibleTaskItems.length === 0
                      width: parent.width
                      textFormat: Text.PlainText
                      text: root.showCompletedTasks ? "No hay tareas en esta lista" : "Todo hecho por aquí. Añade una tarea o activa \"Ver hechas\"."
                      color: root.dimColor
                      font.family: root.contentFontFamily
                      font.pixelSize: Style.font.bodySmall
                      horizontalAlignment: Text.AlignHCenter
                      wrapMode: Text.Wrap
                      topPadding: Style.space(6)
                      bottomPadding: Style.space(6)
                    }
                  }
              }
              }

              // Alta rápida
              Row {
                width: parent.width
                spacing: Style.space(8)

                TextField {
                  id: newTaskField
                  width: parent.width - addTaskBtn.width - parent.spacing
                  placeholderText: "Nueva tarea..."
                  foreground: root.contentForeground
                  accent: Color.accent
                  onAccepted: root.addNewTask()
                }

                Button {
                  id: addTaskBtn
                  text: "Añadir"
                  iconText: ""
                  bordered: true
                  foreground: root.contentForeground
                  accent: Color.accent
                  onClicked: root.addNewTask()
                }
              }
            }

            // -----------------------------------------------------------------
            // Pie: resumen de hoy + ayuda de atajos
            // -----------------------------------------------------------------
            PanelSeparator {
              foreground: root.contentForeground
            }

            Item {
              width: parent.width
              implicitHeight: Math.max(todayRow.implicitHeight, shortcutsBtn.implicitHeight)

              Row {
                id: todayRow
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                spacing: Style.space(10)

                PanelSectionHeader {
                  text: "HOY"
                  foreground: root.contentForeground
                  fontFamily: root.contentFontFamily
                  anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                  textFormat: Text.PlainText
                  text: " " + pomotaskService.todayPomodoros
                  color: Color.accent
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.caption
                  font.bold: true
                  anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                  textFormat: Text.PlainText
                  text: "󰄲 " + pomotaskService.todayTasksDone + (pomotaskService.todayTasksDone === 1 ? " tarea" : " tareas")
                  color: root.dimColor
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.caption
                  anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                  textFormat: Text.PlainText
                  text: pomotaskService.todayFocusLabel + " de foco"
                  color: root.dimColor
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.caption
                  anchors.verticalCenter: parent.verticalCenter
                }
              }

              PanelActionButton {
                id: shortcutsBtn
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                size: Style.space(22)
                fontSize: Style.font.caption
                iconText: "?"
                foreground: root.dimColor
                hoverColor: Color.accent
                tooltipText: root.shortcutsHelp
              }
            }
          }

          // =================================================================
          // VISTA 2: AJUSTES (modo, duraciones, comportamiento, bloqueo)
          // =================================================================
          Column {
            id: settingsViewColumn
            visible: root.currentView === "settings"
            width: parent.width
            spacing: Style.space(12)

            // Cabecera: volver + título | abrir TUI
            Item {
              width: parent.width
              implicitHeight: Math.max(settingsBackRow.implicitHeight, tuiButton.implicitHeight)

              Row {
                id: settingsBackRow
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                spacing: Style.space(8)

                PanelActionButton {
                  size: Style.space(26)
                  fontSize: Style.font.body
                  iconText: ""
                  bordered: true
                  foreground: root.contentForeground
                  hoverColor: Color.accent
                  tooltipText: "Volver (Esc)"
                  anchors.verticalCenter: parent.verticalCenter
                  onClicked: root.currentView = "main"
                }

                Text {
                  textFormat: Text.PlainText
                  text: "Ajustes"
                  color: root.contentForeground
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.title
                  font.bold: true
                  anchors.verticalCenter: parent.verticalCenter
                }
              }

              Button {
                id: tuiButton
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                iconText: ""
                text: "Abrir TUI"
                fontSize: Style.font.caption
                bordered: false
                foreground: root.dimColor
                accent: Color.accent
                tooltipText: "Abrir PomoTask en una terminal (calendario, estadísticas, sesión de Google)"
                onClicked: root.openTui()
              }
            }

            // Modo
            Column {
              width: parent.width
              spacing: Style.space(8)

              PanelSectionHeader {
                text: "MODO"
                foreground: root.contentForeground
                fontFamily: root.contentFontFamily
              }

              EqualChips {
                width: parent.width
                options: [
                  { value: "work", label: "Enfoque", icon: "" },
                  { value: "short_break", label: "Corto", icon: "" },
                  { value: "long_break", label: "Largo", icon: "" }
                ]
                value: pomotaskService.mode
                onChanged: function(v) { pomotaskService.setMode(v) }
              }

            }

            PanelSeparator {
              foreground: root.contentForeground
            }

            // Duraciones (minutos)
            Column {
              width: parent.width
              spacing: Style.space(6)

              PanelSectionHeader {
                text: "DURACIONES"
                foreground: root.contentForeground
                fontFamily: root.contentFontFamily
              }

              DurationRow {
                label: "Enfoque"
                key: "focus"
                seconds: pomotaskService.focusDuration
                minMinutes: 1
                maxMinutes: 180
              }

              DurationRow {
                label: "Descanso corto"
                key: "short"
                seconds: pomotaskService.shortBreakDuration
                minMinutes: 1
                maxMinutes: 60
              }

              DurationRow {
                label: "Descanso largo"
                key: "long"
                seconds: pomotaskService.longBreakDuration
                minMinutes: 1
                maxMinutes: 120
              }

            }

            PanelSeparator {
              foreground: root.contentForeground
            }

            // Comportamiento
            PanelSectionHeader {
              text: "COMPORTAMIENTO"
              foreground: root.contentForeground
              fontFamily: root.contentFontFamily
            }

            Toggle {
              width: parent.width
              label: "Anti-distracciones"
              checked: pomotaskService.antiDistraction
              foreground: root.contentForeground
              accent: Color.accent
              onClicked: pomotaskService.toggleAntiDistraction()
            }

            Toggle {
              width: parent.width
              label: "Bloquear pantalla en descansos"
              checked: pomotaskService.strictBreak
              foreground: root.contentForeground
              accent: Color.accent
              onClicked: pomotaskService.toggleStrictBreak()
            }

            Toggle {
              width: parent.width
              label: "Encadenar fases automáticamente"
              checked: pomotaskService.autoCycle
              foreground: root.contentForeground
              accent: Color.accent
              onClicked: pomotaskService.toggleAutoCycle()
            }

            PanelSeparator {
              foreground: root.contentForeground
            }

            // Bloqueo: una pestaña a la vez
            PanelSectionHeader {
              text: "BLOQUEO"
              foreground: root.contentForeground
              fontFamily: root.contentFontFamily
            }

            EqualChips {
              width: parent.width
              options: [
                { value: "titles", label: "Títulos web" },
                { value: "apps", label: "Apps" },
                { value: "allowed", label: "Excepciones" }
              ]
              value: root.blocklistTab
              onChanged: function(v) { root.blocklistTab = v }
            }

            // Sub-section: Blocked Web Titles / Keywords
            Column {
              visible: root.blocklistTab === "titles"
              width: parent.width
              spacing: Style.space(6)

              Flow {
                width: parent.width
                spacing: Style.space(6)

                Repeater {
                  model: root.blockedTitles
                  delegate: BorderSurface {
                    required property var modelData
                    color: Style.hoverFillFor(root.contentForeground, Color.accent)
                    radius: Style.cornerRadius
                    borderSpec: Border.controlSpec("input", root.contentForeground, Color.accent)
                    implicitHeight: Style.space(26)
                    implicitWidth: chipRow1.implicitWidth + Style.space(12)

                    Row {
                      id: chipRow1
                      anchors.centerIn: parent
                      spacing: Style.space(6)

                      Text {
                        textFormat: Text.PlainText
                        text: modelData
                        color: root.contentForeground
                        font.family: root.contentFontFamily
                        font.pixelSize: Style.font.caption
                        anchors.verticalCenter: parent.verticalCenter
                      }

                      Text {
                        textFormat: Text.PlainText
                        text: ""
                        color: removeMouse1.containsMouse ? Color.urgent : root.dimColor
                        font.family: root.contentFontFamily
                        font.pixelSize: Style.font.caption
                        font.bold: true
                        anchors.verticalCenter: parent.verticalCenter

                        MouseArea {
                          id: removeMouse1
                          anchors.fill: parent
                          anchors.margins: -Style.space(4)
                          hoverEnabled: true
                          cursorShape: Qt.PointingHandCursor
                          onClicked: pomotaskService.blocklistRemoveTitle(modelData)
                        }
                      }
                    }
                  }
                }
              }

              Row {
                width: parent.width
                spacing: Style.space(8)

                TextField {
                  id: newBlockedTitleField
                  width: parent.width - addTitleBtn.width - parent.spacing
                  placeholderText: "Añadir sitio (ej. facebook, reddit)..."
                  foreground: root.contentForeground
                  accent: Color.accent
                  onAccepted: root.addBlockedTitle()
                }

                Button {
                  id: addTitleBtn
                  text: "Añadir"
                  iconText: ""
                  bordered: true
                  foreground: root.contentForeground
                  accent: Color.accent
                  onClicked: root.addBlockedTitle()
                }
              }
            }

            // Sub-section: Blocked Application Classes
            Column {
              visible: root.blocklistTab === "apps"
              width: parent.width
              spacing: Style.space(6)

              Flow {
                width: parent.width
                spacing: Style.space(6)

                Repeater {
                  model: root.blockedApps
                  delegate: BorderSurface {
                    required property var modelData
                    color: Style.hoverFillFor(root.contentForeground, Color.accent)
                    radius: Style.cornerRadius
                    borderSpec: Border.controlSpec("input", root.contentForeground, Color.accent)
                    implicitHeight: Style.space(26)
                    implicitWidth: chipRow2.implicitWidth + Style.space(12)

                    Row {
                      id: chipRow2
                      anchors.centerIn: parent
                      spacing: Style.space(6)

                      Text {
                        textFormat: Text.PlainText
                        text: modelData
                        color: root.contentForeground
                        font.family: root.contentFontFamily
                        font.pixelSize: Style.font.caption
                        anchors.verticalCenter: parent.verticalCenter
                      }

                      Text {
                        textFormat: Text.PlainText
                        text: ""
                        color: removeMouse2.containsMouse ? Color.urgent : root.dimColor
                        font.family: root.contentFontFamily
                        font.pixelSize: Style.font.caption
                        font.bold: true
                        anchors.verticalCenter: parent.verticalCenter

                        MouseArea {
                          id: removeMouse2
                          anchors.fill: parent
                          anchors.margins: -Style.space(4)
                          hoverEnabled: true
                          cursorShape: Qt.PointingHandCursor
                          onClicked: pomotaskService.blocklistRemoveClass(modelData)
                        }
                      }
                    }
                  }
                }
              }

              Row {
                width: parent.width
                spacing: Style.space(8)

                TextField {
                  id: newBlockedClassField
                  width: parent.width - addClassBtn.width - parent.spacing
                  placeholderText: "Añadir app (ej. steam, discord)..."
                  foreground: root.contentForeground
                  accent: Color.accent
                  onAccepted: root.addBlockedClass()
                }

                Button {
                  id: addClassBtn
                  text: "Añadir"
                  iconText: ""
                  bordered: true
                  foreground: root.contentForeground
                  accent: Color.accent
                  onClicked: root.addBlockedClass()
                }
              }
            }

            // Sub-section: Allowed Exceptions (Whitelist)
            Column {
              visible: root.blocklistTab === "allowed"
              width: parent.width
              spacing: Style.space(6)

              Flow {
                width: parent.width
                spacing: Style.space(6)

                Repeater {
                  model: root.allowedTitles
                  delegate: BorderSurface {
                    required property var modelData
                    color: Style.hoverFillFor(root.contentForeground, Color.accent)
                    radius: Style.cornerRadius
                    borderSpec: Border.controlSpec("input", root.contentForeground, Color.accent)
                    implicitHeight: Style.space(26)
                    implicitWidth: chipRow3.implicitWidth + Style.space(12)

                    Row {
                      id: chipRow3
                      anchors.centerIn: parent
                      spacing: Style.space(6)

                      Text {
                        textFormat: Text.PlainText
                        text: modelData
                        color: root.contentForeground
                        font.family: root.contentFontFamily
                        font.pixelSize: Style.font.caption
                        anchors.verticalCenter: parent.verticalCenter
                      }

                      Text {
                        textFormat: Text.PlainText
                        text: ""
                        color: removeMouse3.containsMouse ? Color.urgent : root.dimColor
                        font.family: root.contentFontFamily
                        font.pixelSize: Style.font.caption
                        font.bold: true
                        anchors.verticalCenter: parent.verticalCenter

                        MouseArea {
                          id: removeMouse3
                          anchors.fill: parent
                          anchors.margins: -Style.space(4)
                          hoverEnabled: true
                          cursorShape: Qt.PointingHandCursor
                          onClicked: pomotaskService.blocklistRemoveAllowedTitle(modelData)
                        }
                      }
                    }
                  }
                }
              }

              Row {
                width: parent.width
                spacing: Style.space(8)

                TextField {
                  id: newAllowedTitleField
                  width: parent.width - addAllowedBtn.width - parent.spacing
                  placeholderText: "Permitir siempre (ej. youtube music)..."
                  foreground: root.contentForeground
                  accent: Color.accent
                  onAccepted: root.addAllowedTitle()
                }

                Button {
                  id: addAllowedBtn
                  text: "Añadir"
                  iconText: ""
                  bordered: true
                  foreground: root.contentForeground
                  accent: Color.accent
                  onClicked: root.addAllowedTitle()
                }
              }
            }

            PanelSeparator {
              foreground: root.contentForeground
            }

            // Distraction Action Dropdown
            Dropdown {
              id: actionDropdown
              label: "Acción al detectar distracción"
              showLabel: true
              width: parent.width
              value: pomotaskService.distractionAction
              options: root.actionOptions
              onChanged: function(v) { pomotaskService.blocklistSetAction(v) }
            }

            // Overlay Dimming / Visibility Slider (solo aplica a la pantalla de enfoque)
            Column {
              width: parent.width
              spacing: Style.space(6)
              visible: pomotaskService.distractionAction === "hud"

              Item {
                width: parent.width
                height: dimmingTitleText.implicitHeight

                Text {
                  id: dimmingTitleText
                  anchors.left: parent.left
                  anchors.verticalCenter: parent.verticalCenter
                  textFormat: Text.PlainText
                  text: "Oscurecimiento"
                  color: root.contentForeground
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.bodySmall
                }

                Text {
                  anchors.right: parent.right
                  anchors.verticalCenter: parent.verticalCenter
                  textFormat: Text.PlainText
                  text: Math.round(((pomotaskService.blocklist && typeof pomotaskService.blocklist.overlay_dimming === "number") ? pomotaskService.blocklist.overlay_dimming : 0.40) * 100) + "%"
                  color: Color.accent
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.caption
                }
              }

              PanelSlider {
                width: parent.width
                minimum: 0.0
                maximum: 0.95
                step: 0.05
                value: (pomotaskService.blocklist && typeof pomotaskService.blocklist.overlay_dimming === "number")
                  ? pomotaskService.blocklist.overlay_dimming
                  : 0.40
                onReleased: function(val) {
                  pomotaskService.blocklistSetDimming(val)
                }
              }
            }
          }
        }
      }
    }
  }
}
