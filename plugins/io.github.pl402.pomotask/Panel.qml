import QtQuick
import QtQuick.Controls
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
  readonly property var barIdentity: hostWidget || root

  readonly property var pomotaskService: service || (hostWidget && hostWidget.service ? hostWidget.service : fallbackService)

  PomotaskService {
    id: fallbackService
  }

  readonly property color contentForeground: bar ? bar.foreground : Color.foreground
  readonly property color dimColor: Qt.darker(contentForeground, 1.45)
  readonly property string contentFontFamily: bar ? bar.fontFamily : Style.font.family

  // Navigation view: "main" (ultra-clean timer + active task) | "settings" (modes + anti-distraction rules)
  property string currentView: "main"
  property string selectedListId: "@all"
  property bool showCompletedTasks: false

  readonly property var actionOptions: [
    { value: "warn", label: "Solo advertir (OSD)" },
    { value: "warn_and_unfocus", label: "Advertir y desenfocar" },
    { value: "minimize", label: "Minimizar ventana" }
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
  // Helpers
  // -------------------------------------------------------------------------
  function renderCycleDots(completed) {
    var count = Number(completed || 0)
    var currentInCycle = count % 4
    var dots = ""
    for (var i = 0; i < 4; i++) {
      dots += (i < currentInCycle) ? "● " : "○ "
    }
    return dots.trim()
  }

  function modeBadgeText(mode) {
    if (mode === "short_break") return " Descanso Corto"
    if (mode === "long_break") return " Descanso Largo"
    return " Enfoque"
  }

  function stateBadgeText(state) {
    if (state === "running") return " En Curso"
    if (state === "paused") return " Pausado"
    return "⏹ Detenido"
  }

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
    contentHeight: panel.fittedContentHeight(panelColumn.implicitHeight, Style.space(600))

    PanelKeyCatcher {
      id: keyCatcher
      anchors.fill: parent
      blocked: (typeof newTaskField !== "undefined" && newTaskField && newTaskField.activeFocus)
        || (typeof newBlockedTitleField !== "undefined" && newBlockedTitleField && newBlockedTitleField.activeFocus)
        || (typeof newBlockedClassField !== "undefined" && newBlockedClassField && newBlockedClassField.activeFocus)
        || (typeof newAllowedTitleField !== "undefined" && newAllowedTitleField && newAllowedTitleField.activeFocus)
        || (typeof listDropdown !== "undefined" && listDropdown && listDropdown.popupOpen)
        || (typeof actionDropdown !== "undefined" && actionDropdown && actionDropdown.popupOpen)

      onActivateRequested: pomotaskService.timerToggle()
      onCloseRequested: {
        if (root.currentView === "settings") {
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
          // VISTA 1: PRINCIPAL (Temporizador Ultra-Limpio + Tarea en Foco)
          // =================================================================
          Column {
            id: mainViewColumn
            visible: root.currentView === "main"
            width: parent.width
            spacing: Style.space(12)

            // Header Row: Badges a la izquierda | Sincronizar y Ajustes a la derecha
            Item {
              width: parent.width
              implicitHeight: Math.max(badgeRow.implicitHeight, headerActionsRow.implicitHeight)

              Row {
                id: badgeRow
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                spacing: Style.space(6)

                // Mode Badge
                BorderSurface {
                  color: Style.selectedFillFor(root.contentForeground, Color.accent)
                  radius: Style.cornerRadius
                  implicitWidth: modeBadgeTextItem.implicitWidth + Style.space(10)
                  implicitHeight: modeBadgeTextItem.implicitHeight + Style.space(4)

                  Text {
                    id: modeBadgeTextItem
                    textFormat: Text.PlainText
                    anchors.centerIn: parent
                    text: root.modeBadgeText(pomotaskService.mode)
                    color: Color.accent
                    font.family: root.contentFontFamily
                    font.pixelSize: Style.font.caption
                    font.bold: true
                  }
                }

                // State Badge
                BorderSurface {
                  color: Style.hoverFillFor(root.contentForeground, Color.accent)
                  radius: Style.cornerRadius
                  implicitWidth: stateBadgeTextItem.implicitWidth + Style.space(10)
                  implicitHeight: stateBadgeTextItem.implicitHeight + Style.space(4)

                  Text {
                    id: stateBadgeTextItem
                    textFormat: Text.PlainText
                    anchors.centerIn: parent
                    text: root.stateBadgeText(pomotaskService.state)
                    color: root.contentForeground
                    font.family: root.contentFontFamily
                    font.pixelSize: Style.font.caption
                    font.bold: true
                  }
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
                  tooltipText: "Sincronizar con Google Tasks (R)"
                  onClicked: pomotaskService.syncTasks()
                }

                // Settings button
                PanelActionButton {
                  size: Style.space(26)
                  fontSize: Style.font.body
                  iconText: ""
                  foreground: root.contentForeground
                  hoverColor: Color.accent
                  tooltipText: "Ajustes y Anti-distracciones"
                  onClicked: root.currentView = "settings"
                }
              }
            }

            // Giant Digital Clock & Cycle Row
            Column {
              width: parent.width
              spacing: Style.space(2)

              Text {
                textFormat: Text.PlainText
                text: pomotaskService.formattedTime
                color: Color.foreground
                font.family: root.contentFontFamily
                font.pixelSize: Style.font.displayLarge * 1.5
                font.bold: true
                horizontalAlignment: Text.AlignHCenter
                width: parent.width
              }

              Text {
                textFormat: Text.PlainText
                text: "Ciclo: " + (pomotaskService.sessionPomodoros % 4) + "/4 (" + pomotaskService.sessionPomodoros + " completados)"
                color: root.dimColor
                font.family: root.contentFontFamily
                font.pixelSize: Style.font.caption
                horizontalAlignment: Text.AlignHCenter
                width: parent.width
              }
            }

            // Progress Bar Track
            BorderSurface {
              width: parent.width
              height: Math.max(Style.space(8), 8)
              radius: Style.cornerRadius
              color: Style.selectedFillFor(root.contentForeground, Color.accent)
              borderSpec: Border.controlSpec("normal", root.contentForeground, Color.accent)

              Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                radius: Style.cornerRadius
                width: parent.width * Math.max(0.0, Math.min(1.0, pomotaskService.progress))
                color: pomotaskService.isWork
                  ? Color.accent
                  : (pomotaskService.isShortBreak ? Qt.rgba(0.2, 0.8, 0.5, 1.0) : Qt.rgba(0.2, 0.6, 0.9, 1.0))

                Behavior on width {
                  NumberAnimation { duration: 150; easing.type: Easing.OutCubic }
                }
              }
            }

            // -----------------------------------------------------------------
            // Título de la Tarea Actual (Grande + Marquee Horizontal)
            // -----------------------------------------------------------------
            BorderSurface {
              id: activeTaskCard
              width: parent.width
              implicitHeight: taskMarqueeContainer.implicitHeight + Style.space(12)
              radius: Style.cornerRadius
              color: Style.hoverFillFor(root.contentForeground, Color.accent)
              borderSpec: Border.controlSpec("focus", root.contentForeground, Color.accent)

              Item {
                id: taskMarqueeContainer
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.leftMargin: Style.space(12)
                anchors.rightMargin: Style.space(12)
                implicitHeight: marqueeText.implicitHeight
                clip: true

                readonly property bool hasTask: pomotaskService.activeTaskTitle && pomotaskService.activeTaskTitle !== ""
                readonly property bool isOverflowing: marqueeText.implicitWidth > width
                readonly property real scrollDistance: Math.max(0, marqueeText.implicitWidth - width + Style.space(20))

                Text {
                  id: marqueeText
                  textFormat: Text.PlainText
                  text: taskMarqueeContainer.hasTask
                    ? " " + pomotaskService.activeTaskTitle
                    : " Sin tarea seleccionada"
                  color: taskMarqueeContainer.hasTask ? root.contentForeground : root.dimColor
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.heading
                  font.bold: true
                  anchors.verticalCenter: parent.verticalCenter
                  x: 0

                  SequentialAnimation on x {
                    running: taskMarqueeContainer.hasTask && taskMarqueeContainer.isOverflowing && root.opened
                    loops: Animation.Infinite

                    PauseAnimation { duration: 1500 }
                    NumberAnimation {
                      to: -taskMarqueeContainer.scrollDistance
                      duration: Math.max(2000, taskMarqueeContainer.scrollDistance * 30)
                      easing.type: Easing.Linear
                    }
                    PauseAnimation { duration: 1500 }
                    NumberAnimation {
                      to: 0
                      duration: 800
                      easing.type: Easing.InOutQuad
                    }
                  }
                }
              }
            }

            // -----------------------------------------------------------------
            // Control Action Buttons (Iniciar/Pausar, Saltar, Reiniciar)
            // -----------------------------------------------------------------
            Row {
              width: parent.width
              spacing: Style.space(8)

              // Main Play/Pause Button
              Button {
                width: (parent.width - Style.space(16)) * 0.46
                iconText: pomotaskService.isRunning ? "" : ""
                text: pomotaskService.isRunning ? "Pausar" : "Iniciar"
                selected: pomotaskService.isRunning
                bordered: true
                foreground: root.contentForeground
                accent: Color.accent
                onClicked: pomotaskService.timerToggle()
              }

              // Skip Button
              Button {
                width: (parent.width - Style.space(16)) * 0.27
                iconText: ""
                text: "Saltar"
                bordered: true
                foreground: root.contentForeground
                accent: Color.accent
                onClicked: pomotaskService.timerSkip()
              }

              // Reset Button
              Button {
                width: (parent.width - Style.space(16)) * 0.27
                iconText: ""
                text: "Reiniciar"
                bordered: true
                foreground: root.contentForeground
                accent: Color.accent
                onClicked: pomotaskService.timerReset()
              }
            }

            // -----------------------------------------------------------------
            // Google Tasks Section (Visible solo cuando NO está en curso)
            // -----------------------------------------------------------------
            Column {
              id: tasksSection
              visible: !pomotaskService.isRunning
              width: parent.width
              spacing: Style.space(10)

              PanelSeparator {
                foreground: root.contentForeground
              }

              Item {
                width: parent.width
                implicitHeight: Math.max(tasksHeaderTitle.implicitHeight, toggleCompletedBtn.implicitHeight)

                PanelSectionHeader {
                  id: tasksHeaderTitle
                  text: "TAREAS"
                  foreground: root.contentForeground
                  fontFamily: root.contentFontFamily
                  anchors.left: parent.left
                  anchors.verticalCenter: parent.verticalCenter
                }

                Button {
                  id: toggleCompletedBtn
                  anchors.right: parent.right
                  anchors.verticalCenter: parent.verticalCenter
                  text: root.showCompletedTasks ? "Ocultar hechas" : "Ver hechas"
                  fontSize: Style.font.caption
                  bordered: false
                  foreground: root.dimColor
                  accent: Color.accent
                  onClicked: root.showCompletedTasks = !root.showCompletedTasks
                }
              }

              // List Selector Dropdown (con nombres reales)
              Dropdown {
                id: listDropdown
                label: "Lista"
                showLabel: false
                visible: root.listOptions.length > 2
                width: parent.width
                value: root.selectedListId
                options: root.listOptions
                onChanged: function(v) { root.selectedListId = v }
              }

              // Tasks List Column
              Column {
                width: parent.width
                spacing: Style.space(4)

                Repeater {
                  model: root.visibleTaskItems

                  delegate: BorderSurface {
                    id: taskRow
                    required property var modelData
                    required property int index

                    readonly property var itemData: modelData
                    readonly property bool isFocused: pomotaskService.activeTaskId === itemData.task.id

                    width: panelColumn.width
                    implicitHeight: taskContent.implicitHeight + Style.space(10)
                    radius: Style.cornerRadius
                    color: isFocused
                      ? Style.selectedFillFor(root.contentForeground, Color.accent)
                      : (taskMouse.containsMouse ? Style.hoverFillFor(root.contentForeground, Color.accent) : "transparent")
                    borderSpec: isFocused
                      ? Border.controlSpec("focus", root.contentForeground, Color.accent)
                      : (taskMouse.containsMouse ? Border.controlSpec("hover-cursor", root.contentForeground, Color.accent) : Border.none())

                    Row {
                      id: taskContent
                      anchors.left: parent.left
                      anchors.right: parent.right
                      anchors.verticalCenter: parent.verticalCenter
                      anchors.leftMargin: (itemData.isSubtask ? Style.space(24) : Style.space(8))
                      anchors.rightMargin: Style.space(8)
                      spacing: Style.space(8)

                      // Complete / Checkbox Button
                      PanelActionButton {
                        size: Style.space(22)
                        iconText: itemData.task.completed ? "󰄲" : "󰄱"
                        foreground: itemData.task.completed ? Color.accent : root.contentForeground
                        hoverColor: Color.accent
                        tooltipText: itemData.task.completed ? "Completada" : "Marcar como completada"
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked: pomotaskService.completeTask(itemData.task.id)
                      }

                      // Task Title and Badges
                      Column {
                        width: parent.width - Style.space(22) * 2 - parent.spacing * 2 - (itemData.isSubtask ? Style.space(16) : 0)
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Style.space(2)

                        Text {
                          textFormat: Text.PlainText
                          text: (itemData.isSubtask ? "↳ " : "") + itemData.task.title
                          color: itemData.task.completed ? root.dimColor : root.contentForeground
                          font.family: root.contentFontFamily
                          font.pixelSize: Style.font.body
                          font.strikeout: itemData.task.completed
                          elide: Text.ElideRight
                          width: parent.width
                        }

                        Row {
                          spacing: Style.space(6)
                          visible: (itemData.task.due && itemData.task.due !== "") || (itemData.task.notes && itemData.task.notes !== "") || itemData.task.pomodoros > 0

                          // Due date badge
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

                          // Notes badge
                          Text {
                            visible: !!itemData.task.notes && itemData.task.notes !== ""
                            textFormat: Text.PlainText
                            text: "󰈙"
                            color: root.dimColor
                            font.family: root.contentFontFamily
                            font.pixelSize: Style.font.caption
                            anchors.verticalCenter: parent.verticalCenter
                          }

                          // Pomodoros count badge
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

                      // Focus Target Button ()
                      PanelActionButton {
                        size: Style.space(22)
                        iconText: ""
                        foreground: isFocused ? Color.accent : root.dimColor
                        hoverColor: Color.accent
                        tooltipText: isFocused ? "Quitar enfoque activo" : "Enfocar esta tarea"
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked: {
                          if (isFocused) {
                            pomotaskService.clearFocusTask()
                          } else {
                            pomotaskService.focusTask(itemData.task.id)
                          }
                        }
                      }
                    }

                    MouseArea {
                      id: taskMouse
                      anchors.fill: parent
                      hoverEnabled: true
                      acceptedButtons: Qt.NoButton
                    }
                  }
                }

                Text {
                  visible: root.visibleTaskItems.length === 0
                  textFormat: Text.PlainText
                  text: "No hay tareas para mostrar"
                  color: root.dimColor
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.bodySmall
                  horizontalAlignment: Text.AlignHCenter
                  width: parent.width
                  topPadding: Style.space(8)
                  bottomPadding: Style.space(8)
                }
              }

              // Quick Task Creation Row
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
          }

          // =================================================================
          // VISTA 2: AJUSTES Y ANTI-DISTRACCIONES
          // =================================================================
          Column {
            id: settingsViewColumn
            visible: root.currentView === "settings"
            width: parent.width
            spacing: Style.space(12)

            // Header con botón Volver
            Item {
              width: parent.width
              implicitHeight: Math.max(backButton.implicitHeight, settingsHeaderTitle.implicitHeight)

              Button {
                id: backButton
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                iconText: ""
                text: "Volver"
                bordered: true
                fontSize: Style.font.caption
                foreground: root.contentForeground
                accent: Color.accent
                onClicked: root.currentView = "main"
              }

              PanelSectionHeader {
                id: settingsHeaderTitle
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                text: "AJUSTES Y REGLAS"
                foreground: root.contentForeground
                fontFamily: root.contentFontFamily
              }
            }

            PanelSeparator {
              foreground: root.contentForeground
            }

            // Sección: Selector de Modos Manuales
            Column {
              width: parent.width
              spacing: Style.space(6)

              Text {
                textFormat: Text.PlainText
                text: "Cambio Manual de Modo"
                color: root.contentForeground
                font.family: root.contentFontFamily
                font.pixelSize: Style.font.bodySmall
                font.bold: true
              }

              Button {
                width: parent.width
                iconText: ""
                text: "Sesión de Trabajo (25 min de enfoque)"
                selected: pomotaskService.mode === "work"
                bordered: true
                foreground: root.contentForeground
                accent: Color.accent
                fontSize: Style.font.caption
                onClicked: {
                  pomotaskService.setMode("work")
                  root.currentView = "main"
                }
              }

              Button {
                width: parent.width
                iconText: ""
                text: "Pausa Corta (5 min para estirar y descansar la vista)"
                selected: pomotaskService.mode === "short_break"
                bordered: true
                foreground: root.contentForeground
                accent: Color.accent
                fontSize: Style.font.caption
                onClicked: {
                  pomotaskService.setMode("short_break")
                  root.currentView = "main"
                }
              }

              Button {
                width: parent.width
                iconText: ""
                text: "Pausa Larga (15 min de recuperación profunda)"
                selected: pomotaskService.mode === "long_break"
                bordered: true
                foreground: root.contentForeground
                accent: Color.accent
                fontSize: Style.font.caption
                onClicked: {
                  pomotaskService.setMode("long_break")
                  root.currentView = "main"
                }
              }
            }

            PanelSeparator {
              foreground: root.contentForeground
            }

            // Sección: Anti-distracciones y Bloqueo
            PanelSectionHeader {
              text: "COMPORTAMIENTO Y BLOQUEO"
              foreground: root.contentForeground
              fontFamily: root.contentFontFamily
            }

            Toggle {
              width: parent.width
              label: "Modo Anti-distracciones"
              description: "Detecta sitios y apps distractoras durante el trabajo"
              checked: pomotaskService.antiDistraction
              foreground: root.contentForeground
              accent: Color.accent
              onClicked: pomotaskService.toggleAntiDistraction()
            }

            Toggle {
              width: parent.width
              label: "Bloqueo Estricto en Descanso"
              description: "Bloquea la sesión del sistema al entrar en pausa activa"
              checked: pomotaskService.strictBreak
              foreground: root.contentForeground
              accent: Color.accent
              onClicked: pomotaskService.toggleStrictBreak()
            }

            // Distraction Action Dropdown
            Dropdown {
              id: actionDropdown
              label: "Acción al detectar distracción"
              showLabel: true
              width: parent.width
              value: (pomotaskService.blocklist && pomotaskService.blocklist.action) ? pomotaskService.blocklist.action : "warn"
              options: root.actionOptions
              onChanged: function(v) { pomotaskService.blocklistSetAction(v) }
            }

            PanelSeparator {
              foreground: root.contentForeground
            }

            // Sub-section: Blocked Web Titles / Keywords
            Column {
              width: parent.width
              spacing: Style.space(6)

              Text {
                textFormat: Text.PlainText
                text: " Sitios Web y Palabras Bloqueadas"
                color: root.contentForeground
                font.family: root.contentFontFamily
                font.pixelSize: Style.font.bodySmall
                font.bold: true
              }

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

            PanelSeparator {
              foreground: root.contentForeground
            }

            // Sub-section: Blocked Application Classes
            Column {
              width: parent.width
              spacing: Style.space(6)

              Text {
                textFormat: Text.PlainText
                text: " Aplicaciones Bloqueadas"
                color: root.contentForeground
                font.family: root.contentFontFamily
                font.pixelSize: Style.font.bodySmall
                font.bold: true
              }

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

            PanelSeparator {
              foreground: root.contentForeground
            }

            // Sub-section: Allowed Exceptions (Whitelist)
            Column {
              width: parent.width
              spacing: Style.space(6)

              Text {
                textFormat: Text.PlainText
                text: " Excepciones Permitidas (Lista Blanca)"
                color: root.contentForeground
                font.family: root.contentFontFamily
                font.pixelSize: Style.font.bodySmall
                font.bold: true
              }

              Text {
                textFormat: Text.PlainText
                text: "Tienen prioridad sobre las reglas de bloqueo (ej. YouTube Music)."
                color: root.dimColor
                font.family: root.contentFontFamily
                font.pixelSize: Style.font.caption
              }

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
                  placeholderText: "Añadir excepción (ej. youtube music)..."
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

            // TUI Launcher Button
            Button {
              width: parent.width
              iconText: ""
              text: "Abrir PomoTask TUI en Terminal"
              bordered: true
              foreground: root.contentForeground
              accent: Color.accent
              onClicked: root.openTui()
            }
          }
        }
      }
    }
  }
}
