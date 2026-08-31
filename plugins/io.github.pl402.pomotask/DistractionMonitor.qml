import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland

Item {
  id: root

  property var service: null
  property int alertCooldownMs: 10000
  property double lastAlertTime: 0
  property string lastAlertAddress: ""
  property string lastAlertTitle: ""

  readonly property bool monitorActive: service !== null
    && service.isWork
    && service.isRunning
    && service.antiDistraction

  // Default blocklist fallback in case service.blocklist is null or still loading
  readonly property var fallbackBlocklist: ({
    title_keywords: [
      "facebook", "twitter", "x.com", "instagram",
      "reddit", "youtube.com", "tiktok", "netflix"
    ],
    blocked_classes: [
      "steam", "discord", "spotify"
    ],
    allowed_title_keywords: [
      "youtube music", "music.youtube.com"
    ],
    allowed_classes: [
      "youtube music", "youtube-music", "com.github.th_ch.youtube_music"
    ],
    action: "warn"
  })

  readonly property var activeBlocklist: (service && service.blocklist)
    ? service.blocklist
    : fallbackBlocklist

  function isDistraction(title, initialTitle, appClass, initialClass) {
    var bl = root.activeBlocklist
    if (!bl) return false

    var t = String(title || "").toLowerCase()
    var it = String(initialTitle || "").toLowerCase()
    var c = String(appClass || "").toLowerCase().trim()
    var ic = String(initialClass || "").toLowerCase().trim()

    // 1. Excepciones permitidas (Lista Blanca)
    var allowedKeywords = Array.isArray(bl.allowed_title_keywords) ? bl.allowed_title_keywords : []
    for (var a = 0; a < allowedKeywords.length; a++) {
      var akw = String(allowedKeywords[a] || "").toLowerCase().trim()
      if (akw !== "" && (t.indexOf(akw) !== -1 || it.indexOf(akw) !== -1)) {
        return false
      }
    }

    var allowedClasses = Array.isArray(bl.allowed_classes) ? bl.allowed_classes : []
    for (var b = 0; b < allowedClasses.length; b++) {
      var acls = String(allowedClasses[b] || "").toLowerCase().trim()
      if (acls !== "" && (c === acls || ic === acls || c.indexOf(acls) !== -1)) {
        return false
      }
    }

    // 2. Reglas de bloqueo
    var keywords = Array.isArray(bl.title_keywords) ? bl.title_keywords : []
    for (var i = 0; i < keywords.length; i++) {
      var kw = String(keywords[i] || "").toLowerCase().trim()
      if (kw !== "" && (t.indexOf(kw) !== -1 || it.indexOf(kw) !== -1)) {
        return true
      }
    }

    var classes = Array.isArray(bl.blocked_classes) ? bl.blocked_classes : []
    for (var j = 0; j < classes.length; j++) {
      var cls = String(classes[j] || "").toLowerCase().trim()
      if (cls !== "") {
        if (c === cls || ic === cls) {
          return true
        }
      }
    }

    return false
  }

  function handleDistraction(windowObj) {
    var now = Date.now()
    var address = String(windowObj.address || "")
    var title = String(windowObj.title || windowObj.initialTitle || "Distracción detectada")

    // Check cooldown
    if (address !== "" && address === root.lastAlertAddress && (now - root.lastAlertTime) < root.alertCooldownMs) {
      return
    }
    if ((now - root.lastAlertTime) < (root.alertCooldownMs / 2)) {
      return
    }

    root.lastAlertTime = now
    root.lastAlertAddress = address
    root.lastAlertTitle = title

    // Build notification
    var taskMsg = (service && service.activeTaskTitle && service.activeTaskTitle !== "")
      ? "Estás trabajando en: \"" + service.activeTaskTitle + "\"."
      : "Sesión de concentración activa."
    var displayTitle = title.length > 35 ? title.substring(0, 32) + "…" : title
    var body = taskMsg + " Evita distracciones (" + displayTitle + ")."
    var headline = "⚠️ Concentración activa"

    // Send notification via Omarchy notification sender
    Quickshell.execDetached([
      "omarchy-notification-send",
      "-u", "critical",
      "-g", "󰢌",
      "--app-name", "PomoTask",
      headline,
      body
    ])

    // Handle action
    var action = String(root.activeBlocklist.action || "warn")
    if (action === "warn_and_unfocus" || action === "unfocus") {
      Quickshell.execDetached(["hyprctl", "dispatch", "workspace", "previous"])
    } else if (action === "minimize") {
      Quickshell.execDetached(["hyprctl", "dispatch", "movetoworkspacesilent", "special:minimized"])
    }
  }

  function checkActiveWindow() {
    if (!root.monitorActive || activeWindowProc.running) return
    activeWindowProc.command = ["hyprctl", "-j", "activewindow"]
    activeWindowProc.running = true
  }

  Process {
    id: activeWindowProc
    running: false
    command: ["hyprctl", "-j", "activewindow"]
    stdout: StdioCollector {
      id: activeWindowStdout
      waitForEnd: true
    }
    onExited: function(exitCode) {
      if (exitCode !== 0) return
      var out = String(activeWindowStdout.text || "").trim()
      if (out === "" || out === "{}") return
      try {
        var win = JSON.parse(out)
        if (win && typeof win === "object") {
          var title = win.title || ""
          var initialTitle = win.initialTitle || ""
          var appClass = win["class"] || ""
          var initialClass = win.initialClass || ""

          if (root.isDistraction(title, initialTitle, appClass, initialClass)) {
            root.handleDistraction(win)
          }
        }
      } catch (e) {
        // Ignore parse error
      }
    }
  }

  // Monitor Hyprland window events
  Connections {
    target: Hyprland
    function onRawEvent(event) {
      if (!root.monitorActive) return
      var name = String(event && event.name ? event.name : "")
      if (name === "activewindow" || name === "activewindowv2" || name === "openwindow" || name === "focusedmon") {
        pollDebounce.restart()
      }
    }
  }

  Timer {
    id: pollDebounce
    interval: 200
    repeat: false
    onTriggered: root.checkActiveWindow()
  }

  // Periodic watchdog timer during work sessions
  Timer {
    id: watchdogTimer
    interval: 2000
    repeat: true
    running: root.monitorActive
    onTriggered: root.checkActiveWindow()
  }

  onMonitorActiveChanged: {
    if (monitorActive) {
      checkActiveWindow()
    } else {
      lastAlertAddress = ""
    }
  }
}
