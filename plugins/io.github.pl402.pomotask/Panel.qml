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

  property string selectedListId: "@all"
  property bool showCompletedTasks: false

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

  function addNewTask() {
    var title = String(newTaskField.text || "").trim()
    if (title === "") return
    var targetList = (root.selectedListId && root.selectedListId !== "" && root.selectedListId !== "@all")
      ? root.selectedListId
      : (root.listOptions.length > 1 ? root.listOptions[1].value : "@default")
    pomotaskService.createTask(title, targetList, "")
    newTaskField.text = ""
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
    if (mode === "short_break") return "☕ Descanso Corto"
    if (mode === "long_break") return "🌴 Descanso Largo"
    return "🍅 Enfoque"
  }

  function stateBadgeText(state) {
    if (state === "running") return "▶ En Curso"
    if (state === "paused") return "⏸ Pausado"
    return "⏹ Detenido"
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
      var subs = subtaskMap[parent.id] || []
      for (var k = 0; k < subs.length; k++) {
        list.push({ task: subs[k], isSubtask: true, depth: 1 })
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
    contentWidth: panel.fittedContentWidth(Style.space(420))
    contentHeight: panel.fittedContentHeight(panelColumn.implicitHeight, Style.space(680))

    PanelKeyCatcher {
      id: keyCatcher
      anchors.fill: parent
      blocked: newTaskField.activeFocus || (listDropdown && listDropdown.popupOpen)

      onActivateRequested: pomotaskService.timerToggle()
      onCloseRequested: root.close()
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

          // -----------------------------------------------------------------
          // 1. Hero Timer Section
          // -----------------------------------------------------------------
          Item {
            width: parent.width
            implicitHeight: Math.max(heroTitleColumn.implicitHeight, syncHeaderButton.implicitHeight)

            Column {
              id: heroTitleColumn
              anchors.left: parent.left
              anchors.right: syncHeaderButton.left
              anchors.rightMargin: Style.space(8)
              anchors.verticalCenter: parent.verticalCenter
              spacing: Style.space(2)

              Row {
                spacing: Style.space(6)

                BorderSurface {
                  color: Style.selectedFillFor(root.contentForeground, Color.accent)
                  radius: Style.cornerRadius
                  implicitWidth: modeBadgeTextItem.implicitWidth + Style.space(12)
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

                BorderSurface {
                  color: Style.hoverFillFor(root.contentForeground, Color.accent)
                  radius: Style.cornerRadius
                  implicitWidth: stateBadgeTextItem.implicitWidth + Style.space(12)
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

              Text {
                visible: pomotaskService.activeTaskTitle && pomotaskService.activeTaskTitle !== ""
                textFormat: Text.PlainText
                text: "🎯 " + pomotaskService.activeTaskTitle
                color: root.contentForeground
                font.family: root.contentFontFamily
                font.pixelSize: Style.font.bodySmall
                font.bold: true
                elide: Text.ElideRight
                width: parent.width
              }
            }

            PanelActionButton {
              id: syncHeaderButton
              anchors.right: parent.right
              anchors.verticalCenter: parent.verticalCenter
              size: Style.space(28)
              fontSize: Style.font.body
              iconText: "󰑐"
              foreground: root.contentForeground
              hoverColor: Color.accent
              tooltipText: "Sincronizar con Google Tasks (S)"
              onClicked: pomotaskService.syncTasks()
            }
          }

          // Countdown Display & Progress Bar
          Column {
            width: parent.width
            spacing: Style.space(6)

            Item {
              width: parent.width
              implicitHeight: Math.max(timeDisplay.implicitHeight, cycleIndicator.implicitHeight)

              Text {
                id: timeDisplay
                textFormat: Text.PlainText
                text: pomotaskService.formattedTime
                color: root.contentForeground
                font.family: root.contentFontFamily
                font.pixelSize: Style.font.display * 1.3
                font.bold: true
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
              }

              Column {
                id: cycleIndicator
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                spacing: Style.space(2)

                Text {
                  textFormat: Text.PlainText
                  text: root.renderCycleDots(pomotaskService.sessionPomodoros)
                  color: Color.accent
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.title
                  horizontalAlignment: Text.AlignRight
                  anchors.right: parent.right
                }

                Text {
                  textFormat: Text.PlainText
                  text: "Ciclo: " + (pomotaskService.sessionPomodoros % 4) + "/4 (" + pomotaskService.sessionPomodoros + " 🍅)"
                  color: root.dimColor
                  font.family: root.contentFontFamily
                  font.pixelSize: Style.font.caption
                  horizontalAlignment: Text.AlignRight
                  anchors.right: parent.right
                }
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
          }

          // Control Action Buttons (Play/Pause, Skip, Reset)
          Row {
            width: parent.width
            spacing: Style.space(6)

            Button {
              width: (parent.width - Style.space(12)) * 0.44
              iconText: pomotaskService.isRunning ? "⏸" : "▶"
              text: pomotaskService.isRunning ? "Pausar" : "Iniciar"
              selected: pomotaskService.isRunning
              bordered: true
              foreground: root.contentForeground
              accent: Color.accent
              onClicked: pomotaskService.timerToggle()
            }

            Button {
              width: (parent.width - Style.space(12)) * 0.28
              iconText: "⏭"
              text: "Saltar"
              bordered: true
              foreground: root.contentForeground
              accent: Color.accent
              onClicked: pomotaskService.timerSkip()
            }

            Button {
              width: (parent.width - Style.space(12)) * 0.28
              iconText: "↺"
              text: "Reiniciar"
              bordered: true
              foreground: root.contentForeground
              accent: Color.accent
              onClicked: pomotaskService.timerReset()
            }
          }

          // Mode switcher buttons (Work / Short Break / Long Break)
          Row {
            width: parent.width
            spacing: Style.space(6)

            Button {
              width: (parent.width - Style.space(12)) / 3
              iconText: "🍅"
              text: "Trabajo"
              selected: pomotaskService.mode === "work"
              bordered: true
              foreground: root.contentForeground
              accent: Color.accent
              fontSize: Style.font.bodySmall
              onClicked: pomotaskService.setMode("work")
            }

            Button {
              width: (parent.width - Style.space(12)) / 3
              iconText: "☕"
              text: "Corto"
              selected: pomotaskService.mode === "short_break"
              bordered: true
              foreground: root.contentForeground
              accent: Color.accent
              fontSize: Style.font.bodySmall
              onClicked: pomotaskService.setMode("short_break")
            }

            Button {
              width: (parent.width - Style.space(12)) / 3
              iconText: "🌴"
              text: "Largo"
              selected: pomotaskService.mode === "long_break"
              bordered: true
              foreground: root.contentForeground
              accent: Color.accent
              fontSize: Style.font.bodySmall
              onClicked: pomotaskService.setMode("long_break")
            }
          }

          // -----------------------------------------------------------------
          // 2. Google Tasks Section
          // -----------------------------------------------------------------
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

          // List Selector Dropdown (if multiple lists available)
          Dropdown {
            id: listDropdown
            label: "Lista"
            showLabel: false
            visible: pomotaskService.taskLists && pomotaskService.taskLists.length > 1
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
                          text: "📅 " + root.formatDueDate(itemData.task.due)
                          color: root.contentForeground
                          font.family: root.contentFontFamily
                          font.pixelSize: Style.font.caption
                        }
                      }

                      // Notes badge
                      Text {
                        visible: !!itemData.task.notes && itemData.task.notes !== ""
                        textFormat: Text.PlainText
                        text: "📝"
                        color: root.dimColor
                        font.family: root.contentFontFamily
                        font.pixelSize: Style.font.caption
                        anchors.verticalCenter: parent.verticalCenter
                      }

                      // Pomodoros count badge
                      Text {
                        visible: itemData.task.pomodoros > 0
                        textFormat: Text.PlainText
                        text: "🍅 " + itemData.task.pomodoros
                        color: Color.accent
                        font.family: root.contentFontFamily
                        font.pixelSize: Style.font.caption
                        font.bold: true
                        anchors.verticalCenter: parent.verticalCenter
                      }
                    }
                  }

                  // Focus Target Button (🎯)
                  PanelActionButton {
                    size: Style.space(22)
                    iconText: "🎯"
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
              iconText: "󰐕"
              bordered: true
              foreground: root.contentForeground
              accent: Color.accent
              onClicked: root.addNewTask()
            }
          }

          // -----------------------------------------------------------------
          // 3. Settings & Quick Toggles Section
          // -----------------------------------------------------------------
          PanelSeparator {
            foreground: root.contentForeground
          }

          PanelSectionHeader {
            text: "ENFOQUE Y AJUSTES"
            foreground: root.contentForeground
            fontFamily: root.contentFontFamily
          }

          Toggle {
            width: parent.width
            label: "Modo Anti-distracciones"
            description: "Avisa o minimiza apps distractoras en trabajo"
            checked: pomotaskService.antiDistraction
            foreground: root.contentForeground
            accent: Color.accent
            onClicked: pomotaskService.toggleAntiDistraction()
          }

          Toggle {
            width: parent.width
            label: "Bloqueo Estricto en Descanso"
            description: "Impide ignorar pausas y bloquea distracciones"
            checked: pomotaskService.strictBreak
            foreground: root.contentForeground
            accent: Color.accent
            onClicked: pomotaskService.toggleStrictBreak()
          }

          Button {
            width: parent.width
            iconText: ""
            text: "Abrir PomoTask TUI"
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
