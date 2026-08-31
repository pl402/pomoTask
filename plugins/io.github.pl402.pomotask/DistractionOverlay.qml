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
  property bool active: false

  readonly property real dimmingOpacity: {
    if (service && service.blocklist && typeof service.blocklist.overlay_dimming === "number") {
      return Math.max(0.0, Math.min(1.0, service.blocklist.overlay_dimming))
    }
    return 0.40 // 40% dimming = 60% window visibility
  }

  readonly property bool overlayVisible: active
    && service !== null
    && service.isWork
    && service.isRunning

  PanelWindow {
    id: window
    visible: root.overlayVisible
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "omarchy-pomotask-distraction"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    mask: Region {}

    // Configurable transparent scrim with smooth animation
    Rectangle {
      anchors.fill: parent
      color: Qt.rgba(0, 0, 0, root.dimmingOpacity)
      opacity: root.overlayVisible ? 1.0 : 0.0

      Behavior on opacity {
        NumberAnimation { duration: 250; easing.type: Easing.OutCubic }
      }
    }

    // Centered Focus HUD Card
    Item {
      id: hudContainer
      anchors.centerIn: parent
      width: Math.min(Style.space(580), parent.width - Style.space(48))
      height: card.height

      opacity: root.overlayVisible ? 1.0 : 0.0
      scale: root.overlayVisible ? 1.0 : 0.94
      transform: Translate {
        y: root.overlayVisible ? 0 : Style.space(16)
        Behavior on y {
          NumberAnimation { duration: 280; easing.type: Easing.OutCubic }
        }
      }

      Behavior on opacity {
        NumberAnimation { duration: 250; easing.type: Easing.OutCubic }
      }
      Behavior on scale {
        NumberAnimation { duration: 280; easing.type: Easing.OutCubic }
      }

      BorderSurface {
        id: card
        anchors.horizontalCenter: parent.horizontalCenter
        width: parent.width
        height: contentLayout.implicitHeight + Style.space(48)
        radius: Style.cornerRadius * 1.5
        color: Color.menu.background
        borderSpec: Border.surfaceSpec("menu", "border", Color.menu.border, Math.max(1, Style.space(2)))
        padding: Style.space(24)

        ColumnLayout {
          id: contentLayout
          anchors.fill: parent
          anchors.margins: Style.space(24)
          spacing: Style.space(16)

          // Top Header Badge
          RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: Style.space(8)

            Rectangle {
              height: Style.space(28)
              implicitWidth: headerBadgeLabel.implicitWidth + Style.space(24)
              radius: Style.cornerRadius
              color: Qt.rgba(Color.accent.r, Color.accent.g, Color.accent.b, 0.16)
              border.color: Color.accent
              border.width: 1

              RowLayout {
                anchors.centerIn: parent
                spacing: Style.space(6)

                Text {
                  textFormat: Text.PlainText
                  text: "󰢌"
                  color: Color.accent
                  font.family: Style.font.family
                  font.pixelSize: Style.font.body
                }

                Text {
                  id: headerBadgeLabel
                  textFormat: Text.PlainText
                  text: "ENFOQUE ACTIVO"
                  color: Color.accent
                  font.family: Style.font.family
                  font.pixelSize: Style.font.caption
                  font.bold: true
                  font.letterSpacing: 1.5
                }
              }
            }
          }

          // Subtitle
          Text {
            Layout.alignment: Qt.AlignHCenter
            textFormat: Text.PlainText
            text: "Deberías estar trabajando en tu tarea actual:"
            color: Color.menu.text
            opacity: 0.65
            font.family: Style.font.family
            font.pixelSize: Style.font.bodySmall
            horizontalAlignment: Text.AlignHCenter
          }

          // Active Task Box
          Rectangle {
            Layout.fillWidth: true
            implicitHeight: taskBoxRow.implicitHeight + Style.space(20)
            radius: Style.cornerRadius
            color: Qt.rgba(1, 1, 1, 0.04)
            border.color: Qt.rgba(1, 1, 1, 0.08)
            border.width: 1

            RowLayout {
              id: taskBoxRow
              anchors.fill: parent
              anchors.margins: Style.space(12)
              spacing: Style.space(12)

              // Target / Task Icon
              Rectangle {
                width: Style.space(36)
                height: Style.space(36)
                radius: Style.cornerRadius
                color: Qt.rgba(Color.accent.r, Color.accent.g, Color.accent.b, 0.18)
                Layout.alignment: Qt.AlignVCenter

                Text {
                  anchors.centerIn: parent
                  textFormat: Text.PlainText
                  text: "󰄲"
                  color: Color.accent
                  font.family: Style.font.family
                  font.pixelSize: Style.font.heading
                }
              }

              // Task Title Text
              Text {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                textFormat: Text.PlainText
                text: (root.service && root.service.activeTaskTitle && root.service.activeTaskTitle !== "")
                  ? root.service.activeTaskTitle
                  : "Sesión de concentración"
                color: Color.menu.text
                font.family: Style.font.family
                font.pixelSize: Style.font.heading * 1.05
                font.bold: true
                wrapMode: Text.Wrap
                maximumLineCount: 2
                elide: Text.ElideRight
              }
            }
          }

          // Digital Countdown Clock
          RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: Style.space(10)

            Text {
              textFormat: Text.PlainText
              text: "󰥔"
              color: Color.accent
              font.family: Style.font.family
              font.pixelSize: Style.font.display * 1.1
              Layout.alignment: Qt.AlignVCenter
            }

            Text {
              textFormat: Text.PlainText
              text: root.service ? root.service.formattedTime : "25:00"
              color: Color.menu.text
              font.family: Style.font.family
              font.pixelSize: Style.font.display * 1.5
              font.bold: true
              Layout.alignment: Qt.AlignVCenter
            }
          }

          // Progress Bar & Stats Section
          ColumnLayout {
            Layout.fillWidth: true
            spacing: Style.space(8)

            // Track & Fill Bar
            Rectangle {
              Layout.fillWidth: true
              height: Style.space(8)
              radius: height / 2
              color: Qt.rgba(1, 1, 1, 0.08)

              Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: Math.max(height, parent.width * (root.service ? root.service.progress : 0.0))
                radius: height / 2
                color: Color.accent

                Behavior on width {
                  NumberAnimation { duration: 300; easing.type: Easing.OutCubic }
                }
              }
            }

            // Stats row: Cycle pomodoros + percentage
            RowLayout {
              Layout.fillWidth: true
              spacing: Style.space(8)

              // Cycle dots and text
              RowLayout {
                spacing: Style.space(6)
                Layout.alignment: Qt.AlignLeft

                Text {
                  textFormat: Text.PlainText
                  text: "󰝥"
                  color: Color.accent
                  font.family: Style.font.family
                  font.pixelSize: Style.font.caption
                }

                Text {
                  textFormat: Text.PlainText
                  text: "Ciclo: " + (root.service ? (root.service.sessionPomodoros % 4) : 0) + "/4"
                  color: Color.menu.text
                  opacity: 0.65
                  font.family: Style.font.family
                  font.pixelSize: Style.font.caption
                }

                Row {
                  spacing: Style.space(4)
                  Repeater {
                    model: 4
                    Rectangle {
                      required property int index
                      readonly property int completedInCycle: root.service ? (root.service.sessionPomodoros % 4) : 0
                      width: Style.space(6)
                      height: Style.space(6)
                      radius: width / 2
                      color: index < completedInCycle ? Color.accent : Qt.rgba(1, 1, 1, 0.18)
                    }
                  }
                }
              }

              Item { Layout.fillWidth: true }

              // Progress percentage
              Text {
                textFormat: Text.PlainText
                text: Math.round((root.service ? root.service.progress : 0.0) * 100) + "% completado"
                color: Color.accent
                font.family: Style.font.family
                font.pixelSize: Style.font.caption
                font.bold: true
                Layout.alignment: Qt.AlignRight
              }
            }
          }

          // Footer Hint Row
          Rectangle {
            Layout.fillWidth: true
            implicitHeight: Style.space(32)
            radius: Style.cornerRadius
            color: Qt.rgba(1, 1, 1, 0.02)

            RowLayout {
              anchors.centerIn: parent
              spacing: Style.space(6)

              Text {
                textFormat: Text.PlainText
                text: "󰌌"
                color: Color.menu.text
                opacity: 0.5
                font.family: Style.font.family
                font.pixelSize: Style.font.caption
              }

              Text {
                textFormat: Text.PlainText
                text: "Regresa a tu ventana de trabajo para continuar enfocado"
                color: Color.menu.text
                opacity: 0.5
                font.family: Style.font.family
                font.pixelSize: Style.font.caption
              }
            }
          }
        }
      }
    }
  }
}
