import QtQuick
import Quickshell
import qs.Ui
import qs.Commons

BarWidget {
  id: root
  moduleName: "io.github.pl402.pomotask"

  PomotaskService {
    id: service
  }

  readonly property alias service: service
  readonly property string displayText: service.modeIcon + " " + service.formattedTime

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  WidgetButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: root.displayText
    labelVisible: !root.vertical
    hasVisualContent: true
    horizontalMargin: 8.75
    verticalPadding: 8.75

    onPressed: function(b) {
      if (b === Qt.RightButton) {
        service.timerToggle()
      } else if (b === Qt.MiddleButton) {
        service.timerSkip()
      }
    }
  }
}
