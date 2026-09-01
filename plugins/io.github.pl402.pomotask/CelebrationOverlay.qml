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
  property string completedTaskTitle: ""
  property string currentMotivationalPhrase: "¡Excelente trabajo! Has dado un paso clave hoy."

  readonly property var motivationalPhrases: [
    "¡Excelente trabajo! Has dado un paso clave hoy.",
    "¡Misión cumplida! Tómate un respiro bien merecido. ☕",
    "¡Imparable! Una tarea menos en tu lista de objetivos. 🎯",
    "¡Productividad en su máximo nivel! 🚀",
    "¡La consistencia vence a cualquier reto! Gran enfoque. ⭐",
    "¡Un pomodoro de oro! Sigue con ese gran ritmo. 🍅"
  ]

  function celebrate(title) {
    completedTaskTitle = (title && String(title).trim() !== "") ? String(title).trim() : "Tarea completada"
    var randomIndex = Math.floor(Math.random() * motivationalPhrases.length)
    currentMotivationalPhrase = motivationalPhrases[randomIndex]
    active = true
    autoDismissTimer.restart()
    celebrationProgressBar.startProgress()
  }

  function dismiss() {
    autoDismissTimer.stop()
    active = false
  }

  Timer {
    id: autoDismissTimer
    interval: 4200
    repeat: false
    running: false
    onTriggered: root.dismiss()
  }

  // -------------------------------------------------------------------------
  // Overlay Layer-shell Window
  // -------------------------------------------------------------------------
  PanelWindow {
    id: window
    visible: root.active
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "omarchy-pomotask-celebration"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand

    onVisibleChanged: {
      if (visible) {
        Qt.callLater(function() { keyCatcher.forceActiveFocus() })
      }
    }

    // Scrim backdrop con degradado sutil
    Rectangle {
      anchors.fill: parent
      color: Qt.rgba(0, 0, 0, 0.78)
      opacity: root.active ? 1.0 : 0.0

      Behavior on opacity {
        NumberAnimation { duration: 300; easing.type: Easing.OutCubic }
      }

      MouseArea {
        anchors.fill: parent
        onClicked: root.dismiss()
      }
    }

    // -----------------------------------------------------------------------
    // Lluvia de Confeti / Partículas Festivas
    // -----------------------------------------------------------------------
    Item {
      id: confettiContainer
      anchors.fill: parent
      visible: root.active
      clip: true

      readonly property var colors: [
        "#10b981", "#f59e0b", "#ef4444", "#06b6d4",
        "#ec4899", "#8b5cf6", "#3b82f6", "#eab308"
      ]

      Repeater {
        model: 50

        Item {
          id: particle
          required property int index

          readonly property real startX: (index * 73 + 17) % (window.width > 0 ? window.width : 1920)
          readonly property real particleSize: 8 + (index % 12)
          readonly property color particleColor: confettiContainer.colors[index % confettiContainer.colors.length]
          readonly property int fallDuration: 2200 + ((index * 137) % 1800)
          readonly property int initialDelay: (index * 53) % 1200
          readonly property bool isCircle: (index % 3) === 0
          readonly property bool isEmoji: (index % 11) === 0

          width: particleSize
          height: particleSize
          x: startX
          y: -50

          SequentialAnimation on y {
            running: root.active
            loops: Animation.Infinite
            PauseAnimation { duration: particle.initialDelay }
            NumberAnimation {
              from: -50
              to: window.height > 0 ? window.height + 60 : 1140
              duration: particle.fallDuration
              easing.type: Easing.Linear
            }
          }

          SequentialAnimation on x {
            running: root.active
            loops: Animation.Infinite
            PauseAnimation { duration: particle.initialDelay }
            NumberAnimation {
              to: particle.startX + (index % 2 === 0 ? 60 : -60)
              duration: particle.fallDuration / 2
              easing.type: Easing.InOutSine
            }
            NumberAnimation {
              to: particle.startX
              duration: particle.fallDuration / 2
              easing.type: Easing.InOutSine
            }
          }

          NumberAnimation on rotation {
            running: root.active
            loops: Animation.Infinite
            from: 0
            to: 360 * (index % 2 === 0 ? 1 : -1)
            duration: 1200 + ((index * 89) % 1400)
          }

          // Render de partícula: Círculo, Rectángulo o Emoji festivo
          Loader {
            anchors.fill: parent
            sourceComponent: particle.isEmoji ? emojiComp : (particle.isCircle ? circleComp : rectComp)
          }

          Component {
            id: circleComp
            Rectangle {
              anchors.fill: parent
              radius: width / 2
              color: particle.particleColor
            }
          }

          Component {
            id: rectComp
            Rectangle {
              anchors.fill: parent
              radius: 2
              color: particle.particleColor
            }
          }

          Component {
            id: emojiComp
            Text {
              anchors.centerIn: parent
              text: (particle.index % 2 === 0) ? "🍅" : "🎉"
              font.pixelSize: Style.space(16)
            }
          }
        }
      }
    }

    // -----------------------------------------------------------------------
    // Tarjeta Central Festiva de Celebración
    // -----------------------------------------------------------------------
    Item {
      id: keyCatcher
      anchors.fill: parent
      focus: true

      Keys.onEscapePressed: root.dismiss()
      Keys.onSpacePressed: root.dismiss()
      Keys.onReturnPressed: root.dismiss()
      Keys.onEnterPressed: root.dismiss()

      BorderSurface {
        id: card
        anchors.centerIn: parent
        width: Math.min(Style.space(560), parent.width - Style.space(48))
        height: Math.min(contentLayout.implicitHeight + Style.space(56), parent.height - Style.space(48))
        radius: Style.cornerRadius * 1.8
        color: Color.menu.background
        borderSpec: Border.surfaceSpec("menu", "border", "#10b981", Math.max(2, Style.space(2)))
        padding: Style.space(28)

        // Animación elástica de entrada al activarse
        scale: root.active ? 1.0 : 0.6
        opacity: root.active ? 1.0 : 0.0

        Behavior on scale {
          NumberAnimation { duration: 420; easing.type: Easing.OutBack }
        }
        Behavior on opacity {
          NumberAnimation { duration: 280; easing.type: Easing.OutCubic }
        }

        // Absorbe clicks para no cerrar accidentalmente al clicar dentro de la tarjeta
        MouseArea {
          anchors.fill: parent
          onClicked: {}
        }

        ColumnLayout {
          id: contentLayout
          anchors.centerIn: parent
          width: parent.width - Style.space(56)
          spacing: Style.space(16)

          // Ícono festivo grande y animado
          Item {
            Layout.alignment: Qt.AlignHCenter
            implicitWidth: Style.space(80)
            implicitHeight: Style.space(70)

            Text {
              id: trophyIcon
              anchors.centerIn: parent
              textFormat: Text.PlainText
              text: "🏆"
              font.pixelSize: Style.space(50)

              SequentialAnimation on scale {
                running: root.active
                loops: Animation.Infinite
                NumberAnimation { from: 1.0; to: 1.18; duration: 600; easing.type: Easing.InOutQuad }
                NumberAnimation { from: 1.18; to: 1.0; duration: 600; easing.type: Easing.InOutQuad }
              }
            }
          }

          // Header Badge de Celebración
          RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: Style.space(8)

            Rectangle {
              height: Style.space(32)
              implicitWidth: headerBadgeLabel.implicitWidth + Style.space(28)
              radius: Style.cornerRadius * 1.2
              color: Qt.rgba(0.06, 0.72, 0.51, 0.18)
              border.color: "#10b981"
              border.width: 1

              RowLayout {
                anchors.centerIn: parent
                spacing: Style.space(8)

                Text {
                  textFormat: Text.PlainText
                  text: "✨"
                  font.pixelSize: Style.font.body
                }

                Text {
                  id: headerBadgeLabel
                  textFormat: Text.PlainText
                  text: "¡TAREA COMPLETADA!"
                  color: "#10b981"
                  font.family: Style.font.family
                  font.pixelSize: Style.font.body
                  font.bold: true
                  font.letterSpacing: 2.0
                }

                Text {
                  textFormat: Text.PlainText
                  text: "🎉"
                  font.pixelSize: Style.font.body
                }
              }
            }
          }

          // Caja destacada con el Título de la Tarea
          Rectangle {
            Layout.fillWidth: true
            implicitHeight: taskBoxCol.implicitHeight + Style.space(24)
            radius: Style.cornerRadius
            color: Qt.rgba(1, 1, 1, 0.05)
            border.color: Qt.rgba(1, 1, 1, 0.12)
            border.width: 1

            Column {
              id: taskBoxCol
              anchors.centerIn: parent
              width: parent.width - Style.space(24)
              spacing: Style.space(4)

              Text {
                textFormat: Text.PlainText
                text: "Tarea finalizada"
                color: Color.dimColor ? Color.dimColor : Qt.darker(Color.foreground, 1.4)
                font.family: Style.font.family
                font.pixelSize: Style.font.caption
                font.bold: true
                horizontalAlignment: Text.AlignHCenter
                width: parent.width
              }

              Text {
                textFormat: Text.PlainText
                text: "“" + root.completedTaskTitle + "”"
                color: Color.foreground
                font.family: Style.font.family
                font.pixelSize: Style.font.title
                font.bold: true
                horizontalAlignment: Text.AlignHCenter
                width: parent.width
                wrapMode: Text.Wrap
                maximumLineCount: 2
                elide: Text.ElideRight
              }
            }
          }

          // Frase motivacional
          Text {
            textFormat: Text.PlainText
            text: root.currentMotivationalPhrase
            color: Color.foreground
            font.family: Style.font.family
            font.pixelSize: Style.font.body
            horizontalAlignment: Text.AlignHCenter
            Layout.alignment: Qt.AlignHCenter
            Layout.fillWidth: true
            wrapMode: Text.Wrap
          }

          // Barra de progreso de auto-cierre
          Item {
            id: celebrationProgressBar
            Layout.fillWidth: true
            implicitHeight: Style.space(4)

            function startProgress() {
              progressRect.width = 0
              progressAnim.restart()
            }

            Rectangle {
              anchors.fill: parent
              radius: height / 2
              color: Qt.rgba(1, 1, 1, 0.1)
            }

            Rectangle {
              id: progressRect
              height: parent.height
              width: 0
              radius: height / 2
              color: "#10b981"

              NumberAnimation on width {
                id: progressAnim
                running: false
                from: 0
                to: celebrationProgressBar.width
                duration: autoDismissTimer.interval
                easing.type: Easing.Linear
              }
            }
          }

          // Botón de Continuar
          Button {
            Layout.alignment: Qt.AlignHCenter
            implicitWidth: Style.space(220)
            iconText: "󰄲"
            text: "Continuar (Esc / Espacio)"
            bordered: true
            foreground: Color.foreground
            accent: "#10b981"
            onClicked: root.dismiss()
          }
        }
      }
    }
  }
}
