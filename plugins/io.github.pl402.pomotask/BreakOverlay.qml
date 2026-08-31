import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import qs.Commons
import qs.Ui

Item {
  id: root

  property var service: null
  property bool dismissedForCurrentBreak: false
  property string _lastLockedPhase: ""
  property int currentTipIndex: 0

  readonly property bool isBreakActive: service !== null
    && service.isBreak
    && service.isRunning

  readonly property bool overlayVisible: isBreakActive
    && !dismissedForCurrentBreak
    && (!service || !service.strictBreak)

  readonly property var breakTips: [
    {
      icon: "💧",
      title: "Hidratación",
      text: "Aprovecha para beber un vaso de agua y mantener tu cuerpo hidratado."
    },
    {
      icon: "🧘",
      title: "Respiración Consciente",
      text: "Inhala profundamente durante 4s, mantén 4s y exhala despacio durante 4s."
    },
    {
      icon: "👁️",
      title: "Regla 20-20-20",
      text: "Mira a un objeto situado a 6 metros de distancia durante 20 segundos para relajar la vista."
    },
    {
      icon: "🚶",
      title: "Estiramiento Corporal",
      text: "Ponte de pie, estira los brazos, hombros y cuello para aliviar la tensión."
    },
    {
      icon: "🍃",
      title: "Pausa Mental",
      text: "Desconecta del trabajo y deja que tu mente descanse sin pantallas."
    }
  ]

  readonly property color breakAccentColor: {
    if (!service) return Color.accent
    return service.isLongBreak ? "#06b6d4" : "#10b981"
  }

  function dismiss() {
    root.dismissedForCurrentBreak = true
  }

  function triggerStrictLock() {
    Quickshell.execDetached([
      "omarchy-notification-send",
      "-u", "normal",
      "-g", "󰒲",
      "--app-name", "PomoTask",
      "Descanso Estricto",
      "Sesión bloqueada para proteger tu tiempo de descanso."
    ])
    Quickshell.execDetached(["omarchy-system-lock"])
  }

  function handleBreakTransition() {
    if (!service) return
    var isBreak = service.isBreak && service.isRunning
    if (isBreak) {
      var phaseKey = service.mode + "_" + service.sessionPomodoros + "_" + service.totalSeconds
      if (service.strictBreak) {
        if (_lastLockedPhase !== phaseKey) {
          _lastLockedPhase = phaseKey
          triggerStrictLock()
        }
      }
    } else {
      _lastLockedPhase = ""
      dismissedForCurrentBreak = false
    }
  }

  Connections {
    target: service
    function onModeChanged() { root.handleBreakTransition() }
    function onStateChanged() { root.handleBreakTransition() }
    function onStrictBreakChanged() { root.handleBreakTransition() }
  }

  Timer {
    id: tipRotationTimer
    interval: 7000
    repeat: true
    running: root.overlayVisible
    onTriggered: {
      root.currentTipIndex = (root.currentTipIndex + 1) % root.breakTips.length
    }
  }

  // -------------------------------------------------------------------------
  // Overlay Layer-shell Window
  // -------------------------------------------------------------------------
  PanelWindow {
    id: window
    visible: root.overlayVisible
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "omarchy-pomotask-break"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand

    onVisibleChanged: {
      if (visible) {
        Qt.callLater(function() { keyCatcher.forceActiveFocus() })
      }
    }

    // Scrim backdrop
    Rectangle {
      anchors.fill: parent
      color: Qt.rgba(0, 0, 0, 0.85)

      MouseArea {
        anchors.fill: parent
        onClicked: root.dismiss()
      }
    }

    Item {
      id: keyCatcher
      anchors.fill: parent
      focus: true

      Keys.onEscapePressed: root.dismiss()
      Keys.onSpacePressed: if (root.service) root.service.timerToggle()
      Keys.onReturnPressed: if (root.service) root.service.timerSkip()
      Keys.onEnterPressed: if (root.service) root.service.timerSkip()

      // Central Break Card
      BorderSurface {
        id: card
        anchors.centerIn: parent
        width: Math.min(Style.space(520), parent.width - Style.space(48))
        height: Math.min(contentLayout.implicitHeight + Style.space(48), parent.height - Style.space(48))
        radius: Style.cornerRadius * 1.5
        color: Color.surface
        borderSpec: Border.surfaceSpec("menu", "border", Color.border, Math.max(1, Style.space(2)))
        padding: Style.space(24)

        // Swallow inner clicks
        MouseArea { anchors.fill: parent; onClicked: {} }

        ColumnLayout {
          id: contentLayout
          anchors.fill: parent
          spacing: Style.space(18)

          // Header Badge
          RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: Style.space(8)

            Rectangle {
              height: Style.space(28)
              implicitWidth: headerBadgeLabel.implicitWidth + Style.space(20)
              radius: Style.cornerRadius
              color: Qt.rgba(root.breakAccentColor.r, root.breakAccentColor.g, root.breakAccentColor.b, 0.16)
              border.color: root.breakAccentColor
              border.width: 1

              RowLayout {
                anchors.centerIn: parent
                spacing: Style.space(6)

                Text {
                  textFormat: Text.PlainText
                  text: root.service ? root.service.modeIcon : "󰅟"
                  font.pixelSize: Style.font.body
                }

                Text {
                  id: headerBadgeLabel
                  textFormat: Text.PlainText
                  text: root.service ? root.service.modeLabel.toUpperCase() : "DESCANSO"
                  color: root.breakAccentColor
                  font.family: Style.font.family
                  font.pixelSize: Style.font.caption
                  font.bold: true
                  font.letterSpacing: 1.5
                }
              }
            }
          }

          // Giant Digital Clock
          Text {
            textFormat: Text.PlainText
            text: root.service ? root.service.formattedTime : "05:00"
            color: Color.foreground
            font.family: Style.font.family
            font.pixelSize: Style.font.display * 1.6
            font.bold: true
            Layout.alignment: Qt.AlignHCenter
            horizontalAlignment: Text.AlignHCenter
          }

          // Session Pomodoros Dots & Progress Bar
          ColumnLayout {
            Layout.fillWidth: true
            spacing: Style.space(8)

            // Progress Bar Track
            Rectangle {
              Layout.fillWidth: true
              height: Style.space(6)
              radius: height / 2
              color: Qt.rgba(1, 1, 1, 0.08)

              Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: Math.max(height, parent.width * (root.service ? root.service.progress : 0.0))
                radius: height / 2
                color: root.breakAccentColor

                Behavior on width {
                  NumberAnimation { duration: 300; easing.type: Easing.OutCubic }
                }
              }
            }

            // Session Pomodoro count
            RowLayout {
              Layout.fillWidth: true
              spacing: Style.space(8)

              Text {
                textFormat: Text.PlainText
                text: "Ciclo: " + (root.service ? root.service.sessionPomodoros : 0) + " completados"
                color: Color.dim
                font.family: Style.font.family
                font.pixelSize: Style.font.caption
                Layout.fillWidth: true
              }

              Row {
                spacing: Style.space(4)
                Repeater {
                  model: 4
                  Rectangle {
                    required property int index
                    readonly property int completedInCycle: root.service ? (root.service.sessionPomodoros % 4) : 0
                    width: Style.space(8)
                    height: Style.space(8)
                    radius: width / 2
                    color: index < completedInCycle ? Color.accent : Qt.rgba(1, 1, 1, 0.15)
                  }
                }
              }
            }
          }

          // Breathing / Calming Section
          Rectangle {
            Layout.fillWidth: true
            implicitHeight: Style.space(110)
            radius: Style.cornerRadius
            color: Qt.rgba(1, 1, 1, 0.04)
            border.color: Qt.rgba(1, 1, 1, 0.08)
            border.width: 1

            RowLayout {
              anchors.fill: parent
              anchors.margins: Style.space(14)
              spacing: Style.space(16)

              // Breathing Pulse Guide
              Item {
                implicitWidth: Style.space(70)
                implicitHeight: Style.space(70)
                Layout.alignment: Qt.AlignVCenter

                Rectangle {
                  id: breathCircle
                  anchors.centerIn: parent
                  width: Style.space(54)
                  height: Style.space(54)
                  radius: width / 2
                  color: Qt.rgba(root.breakAccentColor.r, root.breakAccentColor.g, root.breakAccentColor.b, 0.22)
                  border.color: root.breakAccentColor
                  border.width: 2
                  scale: 1.0
                  opacity: 0.6
                }

                Text {
                  anchors.centerIn: parent
                  textFormat: Text.PlainText
                  text: "🧘"
                  font.pixelSize: Style.font.heading
                }

                SequentialAnimation {
                  running: root.overlayVisible
                  loops: Animation.Infinite

                  ParallelAnimation {
                    NumberAnimation { target: breathCircle; property: "scale"; to: 1.25; duration: 4000; easing.type: Easing.InOutQuad }
                    NumberAnimation { target: breathCircle; property: "opacity"; to: 0.85; duration: 4000; easing.type: Easing.InOutQuad }
                  }
                  PauseAnimation { duration: 2000 }
                  ParallelAnimation {
                    NumberAnimation { target: breathCircle; property: "scale"; to: 0.85; duration: 4000; easing.type: Easing.InOutQuad }
                    NumberAnimation { target: breathCircle; property: "opacity"; to: 0.4; duration: 4000; easing.type: Easing.InOutQuad }
                  }
                  PauseAnimation { duration: 1000 }
                }
              }

              // Health / Mindfulness rotating tip
              ColumnLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                spacing: Style.space(4)

                readonly property var currentTip: root.breakTips[root.currentTipIndex] || root.breakTips[0]

                RowLayout {
                  spacing: Style.space(6)

                  Text {
                    textFormat: Text.PlainText
                    text: parent.parent.currentTip.icon
                    font.pixelSize: Style.font.body
                  }

                  Text {
                    textFormat: Text.PlainText
                    text: parent.parent.currentTip.title
                    color: Color.foreground
                    font.family: Style.font.family
                    font.pixelSize: Style.font.bodySmall
                    font.bold: true
                  }
                }

                Text {
                  textFormat: Text.PlainText
                  text: parent.currentTip.text
                  color: Color.dim
                  font.family: Style.font.family
                  font.pixelSize: Style.font.caption
                  wrapMode: Text.Wrap
                  Layout.fillWidth: true
                }
              }
            }
          }

          // Action Buttons Row
          RowLayout {
            Layout.fillWidth: true
            spacing: Style.space(10)

            Button {
              text: root.service && root.service.isRunning ? "Pausar" : "Reanudar"
              iconText: root.service && root.service.isRunning ? "󰏤" : "󰐊"
              bordered: true
              Layout.fillWidth: true
              onClicked: if (root.service) root.service.timerToggle()
            }

            Button {
              text: "Omitir Descanso"
              iconText: "󰒭"
              bordered: true
              Layout.fillWidth: true
              onClicked: if (root.service) root.service.timerSkip()
            }

            Button {
              text: "Ocultar (Esc)"
              iconText: "󰅙"
              bordered: true
              Layout.fillWidth: true
              onClicked: root.dismiss()
            }
          }
        }
      }
    }
  }
}
