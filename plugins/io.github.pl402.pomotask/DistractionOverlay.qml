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
  // Acción configurada (ya normalizada por el monitor): "warn" | "hud" | "minimize".
  property string mode: "warn"
  // Título de la ventana distractora detectada.
  property string title: ""

  // Aviso pequeño (modos "warn" y "minimize"): se muestra mientras la distracción sigue
  // activa y permanece unos segundos más para que dé tiempo a leerlo (en "minimize" la
  // ventana desaparece al instante y la detección se apaga en el siguiente sondeo).
  readonly property int toastLingerMs: 4000
  property bool toastLingering: false
  property string toastTitle: ""
  property string toastMode: "warn"

  readonly property bool running: service !== null && service.isWork && service.isRunning
  readonly property bool toastVisible: mode !== "hud" && running && (overlayVisible || toastLingering)

  readonly property string taskTitle: (service && service.activeTaskTitle && String(service.activeTaskTitle) !== "")
    ? String(service.activeTaskTitle)
    : "tu sesión de concentración"

  onActiveChanged: {
    if (active) {
      toastTitle = title
      toastMode = mode
      toastLingering = false
      lingerTimer.stop()
    } else if (toastTitle !== "") {
      toastLingering = true
      lingerTimer.restart()
    }
  }

  onTitleChanged: if (active && title !== "") toastTitle = title

  Timer {
    id: lingerTimer
    interval: root.toastLingerMs
    repeat: false
    onTriggered: root.toastLingering = false
  }

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

  // ---------------------------------------------------------------------------
  // Aviso discreto: tarjeta pequeña abajo al centro, estilo OSD de Omarchy.
  // Sin oscurecer la pantalla; solo informa (y en "minimize", explica qué pasó).
  // ---------------------------------------------------------------------------
  PanelWindow {
    id: toastWindow
    visible: root.toastVisible
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "omarchy-pomotask-distraction-toast"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    mask: Region {}

    BorderSurface {
      id: toastCard
      // Anchos deterministas (sin depender del layout) para evitar bucles de binding:
      // icono + hueco + la línea más larga, acotada para que no cruce la pantalla.
      readonly property real pad: Style.space(12)
      readonly property real gap: Style.space(12)
      readonly property real maxTextWidth: Math.min(Style.space(520), toastWindow.width - Style.space(48) - toastIcon.implicitWidth - gap - pad * 2 - borderLeft - borderRight)
      readonly property real textWidth: Math.max(0, Math.min(maxTextWidth, Math.max(toastLine1.implicitWidth, toastLine2.implicitWidth)))

      anchors.horizontalCenter: parent.horizontalCenter
      anchors.bottom: parent.bottom
      anchors.bottomMargin: Style.space(67)
      width: borderLeft + pad + toastIcon.implicitWidth + gap + textWidth + pad + borderRight
      height: borderTop + pad + Math.max(toastIcon.implicitHeight, toastLine1.implicitHeight + Style.space(2) + toastLine2.implicitHeight) + pad + borderBottom
      radius: Style.cornerRadius
      color: Util.alpha(Color.background, 0.97)
      borderSpec: Border.surfaceSpec("popups", "border", Color.popups.border, Math.max(1, Style.space(2)))
      opacity: root.toastVisible ? 1.0 : 0.0
      transform: Translate {
        y: root.toastVisible ? 0 : Style.space(12)
        Behavior on y { NumberAnimation { duration: 220; easing.type: Easing.OutCubic } }
      }
      Behavior on opacity { NumberAnimation { duration: 200; easing.type: Easing.OutCubic } }

      Text {
        id: toastIcon
        x: toastCard.borderLeft + toastCard.pad
        anchors.verticalCenter: parent.verticalCenter
        textFormat: Text.PlainText
        text: root.toastMode === "minimize" ? "󰖲" : "󰀦"
        color: root.toastMode === "minimize" ? Color.accent : Color.popups.text
        font.family: Style.font.family
        font.pixelSize: Style.font.display
      }

      Column {
        x: toastIcon.x + toastIcon.implicitWidth + toastCard.gap
        anchors.verticalCenter: parent.verticalCenter
        width: toastCard.textWidth
        spacing: Style.space(2)

        Text {
          id: toastLine1
          width: parent.width
          textFormat: Text.PlainText
          text: root.toastMode === "minimize"
            ? "Ventana oculta hasta el descanso: " + root.toastTitle
            : "Distracción: " + root.toastTitle
          color: Color.popups.text
          font.family: Style.font.family
          font.pixelSize: Style.font.body
          font.bold: true
          elide: Text.ElideRight
        }

        Text {
          id: toastLine2
          width: parent.width
          textFormat: Text.PlainText
          text: (root.toastMode === "minimize" ? "Sigue con " : "Vuelve a ")
            + root.taskTitle
            + " · " + (root.service ? root.service.formattedTime : "--:--") + " restantes"
          color: Util.alpha(Color.popups.text, 0.65)
          font.family: Style.font.family
          font.pixelSize: Style.font.bodySmall
          elide: Text.ElideRight
        }
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Pantalla de enfoque (modo "hud"): oscurece toda la pantalla y muestra la tarjeta
  // con la tarea, el reloj y el progreso encima de la distracción.
  // ---------------------------------------------------------------------------
  PanelWindow {
    id: window
    visible: root.mode === "hud" && root.overlayVisible
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

        }
      }
    }
  }
}
