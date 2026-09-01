import QtQuick
import Quickshell
import Quickshell.Io

Item {
  id: root

  // -------------------------------------------------------------------------
  // Configuration Paths & Binary
  // -------------------------------------------------------------------------
  readonly property string configDir: {
    var custom = Quickshell.env("POMOTASK_CONFIG_DIR")
    if (custom && custom !== "") return custom
    var xdg = Quickshell.env("XDG_CONFIG_HOME")
    if (xdg && xdg !== "") return xdg + "/pomotask"
    return (Quickshell.env("HOME") || "") + "/.config/pomotask"
  }

  readonly property string runtimeStatePath: configDir + "/runtime_state.json"
  readonly property string tasksCachePath: configDir + "/tasks_cache.json"
  readonly property string listsCachePath: configDir + "/lists_cache.json"
  readonly property string blocklistPath: configDir + "/blocklist.json"
  property string pomotaskBinary: "pomotask-cli"

  // -------------------------------------------------------------------------
  // Runtime State Properties
  // -------------------------------------------------------------------------
  property string state: "stopped" // "running", "paused", "stopped"
  property string mode: "work"     // "work", "short_break", "long_break"
  property int remainingSeconds: 1500
  property int totalSeconds: 1500
  property int sessionPomodoros: 0
  property string activeTaskId: ""
  property string activeTaskTitle: ""
  property bool strictBreak: false
  property bool antiDistraction: true

  // -------------------------------------------------------------------------
  // Signals & Events
  // -------------------------------------------------------------------------
  signal celebrationRequested(string taskTitle)

  function triggerCelebration(title) {
    celebrationRequested(title || "")
  }

  // -------------------------------------------------------------------------
  // Derived State & Helpers
  // -------------------------------------------------------------------------
  readonly property bool isRunning: state === "running"
  readonly property bool isPaused: state === "paused"
  readonly property bool isStopped: state === "stopped"
  readonly property bool isWork: mode === "work"
  readonly property bool isBreak: mode === "short_break" || mode === "long_break"
  readonly property bool isShortBreak: mode === "short_break"
  readonly property bool isLongBreak: mode === "long_break"

  readonly property real progress: totalSeconds > 0
    ? Math.max(0.0, Math.min(1.0, (totalSeconds - remainingSeconds) / totalSeconds))
    : 0.0

  readonly property string formattedTime: formatSeconds(remainingSeconds)

  readonly property string modeLabel: {
    if (mode === "short_break") return "Short Break"
    if (mode === "long_break") return "Long Break"
    return "Work"
  }

  readonly property string modeIcon: {
    if (mode === "short_break") return ""
    if (mode === "long_break") return ""
    return ""
  }

  // -------------------------------------------------------------------------
  // Tasks Cache & Blocklist
  // -------------------------------------------------------------------------
  property var tasks: []
  property var taskLists: []
  property var blocklist: null

  // -------------------------------------------------------------------------
  // Process State & Diagnostics
  // -------------------------------------------------------------------------
  property string lastError: ""
  property string actionStatus: ""
  readonly property bool busy: statusProcess.running || actionProcess.running || tasksProcess.running

  property string _statusOutput: ""
  property string _statusError: ""
  property string _actionOutput: ""
  property string _actionError: ""
  property string _tasksOutput: ""
  property string _tasksError: ""

  // -------------------------------------------------------------------------
  // Helper Functions
  // -------------------------------------------------------------------------
  function formatSeconds(sec) {
    var total = Math.max(0, Math.floor(sec))
    var m = Math.floor(total / 60)
    var s = total % 60
    var mStr = (m < 10 ? "0" : "") + m
    var sStr = (s < 10 ? "0" : "") + s
    return mStr + ":" + sStr
  }

  function parseRuntimeState(raw) {
    try {
      var content = String(raw || "").trim()
      if (content === "") return
      var obj = JSON.parse(content)
      if (obj && typeof obj === "object") {
        if (obj.state !== undefined) root.state = String(obj.state)
        if (obj.mode !== undefined) root.mode = String(obj.mode)
        if (obj.total_seconds !== undefined) root.totalSeconds = Number(obj.total_seconds)
        if (obj.session_pomodoros !== undefined) root.sessionPomodoros = Number(obj.session_pomodoros)
        root.activeTaskId = obj.active_task_id ? String(obj.active_task_id) : ""
        root.activeTaskTitle = obj.active_task_title ? String(obj.active_task_title) : ""
        if (obj.strict_break !== undefined) root.strictBreak = Boolean(obj.strict_break)
        if (obj.anti_distraction !== undefined) root.antiDistraction = Boolean(obj.anti_distraction)

        if (obj.state === "running" && obj.target_end_timestamp) {
          var nowSec = Math.floor(Date.now() / 1000)
          var diff = Math.max(0, Number(obj.target_end_timestamp) - nowSec)
          if (Math.abs(root.remainingSeconds - diff) > 1 || !root.isRunning) {
            root.remainingSeconds = diff
          }
        } else if (obj.remaining_seconds !== undefined) {
          var newSec = Number(obj.remaining_seconds)
          if (root.isRunning && obj.state === "running") {
            if (Math.abs(root.remainingSeconds - newSec) > 2) {
              root.remainingSeconds = newSec
            }
          } else {
            root.remainingSeconds = newSec
          }
        }
        root.lastError = ""
      }
    } catch (e) {
      console.warn("PomotaskService", "Error parsing runtime state:", e)
    }
  }

  function parseTasksCache(raw) {
    try {
      var content = String(raw || "").trim()
      if (content === "") return
      var obj = JSON.parse(content)
      if (obj && typeof obj === "object") {
        if (Array.isArray(obj)) {
          root.tasks = obj
        } else {
          var all = []
          var lists = []
          var seenIds = {}

          for (var listId in obj) {
            if (listId !== "@all") {
              lists.push(listId)
            }
          }

          if (Array.isArray(obj["@all"]) && obj["@all"].length > 0) {
            all = obj["@all"].slice()
            for (var i = 0; i < all.length; i++) {
              if (all[i] && all[i].id) {
                seenIds[all[i].id] = true
              }
            }
            for (var j = 0; j < lists.length; j++) {
              var listTasks = obj[lists[j]]
              if (Array.isArray(listTasks)) {
                for (var k = 0; k < listTasks.length; k++) {
                  var t = listTasks[k]
                  if (t && t.id && !seenIds[t.id]) {
                    all.push(t)
                    seenIds[t.id] = true
                  }
                }
              }
            }
          } else {
            for (var m = 0; m < lists.length; m++) {
              var items = obj[lists[m]]
              if (Array.isArray(items)) {
                for (var n = 0; n < items.length; n++) {
                  var item = items[n]
                  if (item && item.id && !seenIds[item.id]) {
                    all.push(item)
                    seenIds[item.id] = true
                  }
                }
              }
            }
          }
          root.tasks = all
          if (!root.taskLists || root.taskLists.length === 0) {
            root.taskLists = lists
          }
        }
      }
    } catch (e) {
      console.warn("PomotaskService", "Error parsing tasks cache:", e)
    }
  }

  function parseListsCache(raw) {
    try {
      var content = String(raw || "").trim()
      if (content === "") return
      var arr = JSON.parse(content)
      if (Array.isArray(arr) && arr.length > 0) {
        root.taskLists = arr
      }
    } catch (e) {
      console.warn("PomotaskService", "Error parsing lists cache:", e)
    }
  }

  function parseBlocklist(raw) {
    try {
      var content = String(raw || "").trim()
      if (content === "") return
      var obj = JSON.parse(content)
      if (obj && typeof obj === "object") {
        root.blocklist = obj
      }
    } catch (e) {
      console.warn("PomotaskService", "Error parsing blocklist:", e)
    }
  }

  // -------------------------------------------------------------------------
  // IPC Command Triggers
  // -------------------------------------------------------------------------
  function fetchStatus() {
    if (statusProcess.running) return
    _statusOutput = ""
    _statusError = ""
    statusProcess.command = [root.pomotaskBinary, "ipc", "status"]
    statusProcess.running = true
  }

  function fetchTasks(listId) {
    if (tasksProcess.running) return
    _tasksOutput = ""
    _tasksError = ""
    var cmd = [root.pomotaskBinary, "ipc", "tasks", "list"]
    if (listId && listId !== "") {
      cmd.push("--list-id")
      cmd.push(String(listId))
    }
    tasksProcess.command = cmd
    tasksProcess.running = true
  }

  property var actionQueue: []

  function runAction(args, label) {
    if (!args || args.length === 0) return
    actionQueue.push({ args: args, label: label || "" })
    processNextAction()
  }

  function processNextAction() {
    if (actionProcess.running || actionQueue.length === 0) return
    var item = actionQueue.shift()
    _actionOutput = ""
    _actionError = ""
    root.actionStatus = item.label
    var cmd = [root.pomotaskBinary, "ipc"].concat(item.args)
    actionProcess.command = cmd
    actionProcess.running = true
  }

  // Timer controls
  function timerToggle() {
    if (root.isRunning) {
      root.state = "paused"
    } else {
      root.state = "running"
    }
    runAction(["timer", "toggle"], "Toggling timer…")
  }
  function timerStart() {
    root.state = "running"
    runAction(["timer", "start"], "Starting timer…")
  }
  function timerPause() {
    root.state = "paused"
    runAction(["timer", "pause"], "Pausing timer…")
  }
  function timerSkip() {
    runAction(["timer", "skip"], "Skipping phase…")
  }
  function timerReset() {
    root.state = "stopped"
    root.remainingSeconds = root.totalSeconds
    runAction(["timer", "reset"], "Resetting timer…")
  }
  function setMode(mode) { runAction(["timer", "mode", String(mode)], "Setting mode…") }

  // Task operations
  function completeTask(taskId) {
    if (!taskId || taskId === "") return
    if (root.activeTaskId === taskId) {
      root.activeTaskId = ""
      root.activeTaskTitle = ""
    }
    runAction(["task", "complete", String(taskId)], "Completing task…")
  }
  function taskComplete(taskId) { completeTask(taskId) }

  function createTask(title, listId, parentId) {
    if (!title || String(title).trim() === "") return
    var args = ["task", "create", "--title", String(title).trim()]
    if (listId && listId !== "") {
      args.push("--list-id")
      args.push(String(listId))
    }
    if (parentId && parentId !== "") {
      args.push("--parent")
      args.push(String(parentId))
    }
    runAction(args, "Creating task…")
  }
  function taskCreate(title, listId, parentId) { createTask(title, listId, parentId) }

  function focusTask(taskId) {
    if (!taskId || taskId === "") return
    root.activeTaskId = taskId
    for (var i = 0; i < root.tasks.length; i++) {
      if (root.tasks[i] && root.tasks[i].id === taskId) {
        root.activeTaskTitle = root.tasks[i].title
        break
      }
    }
    runAction(["task", "focus", String(taskId)], "Setting active task…")
  }
  function taskFocus(taskId) { focusTask(taskId) }

  function clearFocusTask() {
    root.activeTaskId = ""
    root.activeTaskTitle = ""
    runAction(["task", "focus", "clear"], "Clearing active task…")
  }

  // Settings / toggles
  function toggleStrictBreak() {
    runAction(["blocklist", "toggle-strict"], "Toggling strict break…")
  }
  function blocklistToggleStrictBreak() { toggleStrictBreak() }

  function toggleAntiDistraction() {
    runAction(["blocklist", "toggle-anti-distraction"], "Toggling anti-distraction…")
  }
  function blocklistToggleAntiDistraction() { toggleAntiDistraction() }

  function blocklistSetAction(action) {
    if (!action || String(action).trim() === "") return
    runAction(["blocklist", "set-action", String(action).trim()], "Updating distraction action…")
  }

  function blocklistSetDimming(dimming) {
    var val = Math.max(0.0, Math.min(1.0, parseFloat(dimming) || 0.40))
    runAction(["blocklist", "set-dimming", val.toFixed(2)], "Updating overlay dimming…")
  }

  function blocklistAddTitle(keyword) {
    var kw = String(keyword || "").trim()
    if (kw === "") return
    runAction(["blocklist", "add-title", kw], "Adding blocked keyword…")
  }

  function blocklistRemoveTitle(keyword) {
    var kw = String(keyword || "").trim()
    if (kw === "") return
    runAction(["blocklist", "remove-title", kw], "Removing blocked keyword…")
  }

  function blocklistAddClass(className) {
    var cls = String(className || "").trim()
    if (cls === "") return
    runAction(["blocklist", "add-class", cls], "Adding blocked app…")
  }

  function blocklistRemoveClass(className) {
    var cls = String(className || "").trim()
    if (cls === "") return
    runAction(["blocklist", "remove-class", cls], "Removing blocked app…")
  }

  function blocklistAddAllowedTitle(keyword) {
    var kw = String(keyword || "").trim()
    if (kw === "") return
    runAction(["blocklist", "add-allowed-title", kw], "Adding allowed exception…")
  }

  function blocklistRemoveAllowedTitle(keyword) {
    var kw = String(keyword || "").trim()
    if (kw === "") return
    runAction(["blocklist", "remove-allowed-title", kw], "Removing allowed exception…")
  }

  function blocklistAddAllowedClass(className) {
    var cls = String(className || "").trim()
    if (cls === "") return
    runAction(["blocklist", "add-allowed-class", cls], "Adding allowed app exception…")
  }

  function blocklistRemoveAllowedClass(className) {
    var cls = String(className || "").trim()
    if (cls === "") return
    runAction(["blocklist", "remove-allowed-class", cls], "Removing allowed app exception…")
  }

  function syncTasks() {
    runAction(["sync"], "Syncing tasks with Google…")
  }
  function forceSync() { syncTasks() }

  // -------------------------------------------------------------------------
  // File Watchers
  // -------------------------------------------------------------------------
  FileView {
    id: runtimeStateWatcher
    path: root.runtimeStatePath
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: root.parseRuntimeState(text())
    onLoadFailed: root.fetchStatus()
  }

  FileView {
    id: tasksCacheWatcher
    path: root.tasksCachePath
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: root.parseTasksCache(text())
  }

  FileView {
    id: listsCacheWatcher
    path: root.listsCachePath
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: root.parseListsCache(text())
  }

  FileView {
    id: blocklistWatcher
    path: root.blocklistPath
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: root.parseBlocklist(text())
  }

  // -------------------------------------------------------------------------
  // Processes
  // -------------------------------------------------------------------------
  Process {
    id: statusProcess
    running: false
    command: []
    stdout: StdioCollector {
      id: statusStdout
      waitForEnd: true
      onStreamFinished: root._statusOutput = text
    }
    stderr: StdioCollector {
      id: statusStderr
      waitForEnd: true
      onStreamFinished: root._statusError = text
    }
    onExited: function(exitCode) {
      var out = String(statusStdout.text || root._statusOutput || "").trim()
      if (exitCode === 0 && out !== "") {
        root.parseRuntimeState(out)
      } else {
        var err = String(statusStderr.text || root._statusError || "").trim()
        if (err !== "") root.lastError = err
      }
    }
  }

  Process {
    id: tasksProcess
    running: false
    command: []
    stdout: StdioCollector {
      id: tasksStdout
      waitForEnd: true
      onStreamFinished: root._tasksOutput = text
    }
    stderr: StdioCollector {
      id: tasksStderr
      waitForEnd: true
      onStreamFinished: root._tasksError = text
    }
    onExited: function(exitCode) {
      var out = String(tasksStdout.text || root._tasksOutput || "").trim()
      if (exitCode === 0 && out !== "") {
        root.parseTasksCache(out)
      } else {
        var err = String(tasksStderr.text || root._tasksError || "").trim()
        if (err !== "") root.lastError = err
      }
    }
  }

  Process {
    id: actionProcess
    running: false
    command: []
    stdout: StdioCollector {
      id: actionStdout
      waitForEnd: true
      onStreamFinished: root._actionOutput = text
    }
    stderr: StdioCollector {
      id: actionStderr
      waitForEnd: true
      onStreamFinished: root._actionError = text
    }
    onExited: function(exitCode) {
      root.actionStatus = ""
      var err = String(actionStderr.text || root._actionError || "").trim()
      var out = String(actionStdout.text || root._actionOutput || "").trim()
      if (exitCode === 0) {
        root.lastError = ""
        // Try parsing output if command returned runtime state
        if (out.indexOf("{") !== -1) {
          root.parseRuntimeState(out)
        }
      } else {
        root.lastError = err !== "" ? err : "Command failed"
      }
      delayedStatusRefresh.restart()
      Qt.callLater(root.processNextAction)
    }
  }

  // -------------------------------------------------------------------------
  // Timers
  // -------------------------------------------------------------------------
  // Smooth local timer countdown while running
  Timer {
    id: localSecondTick
    interval: 1000
    repeat: true
    running: root.isRunning && root.remainingSeconds > 0
    onTriggered: {
      if (root.remainingSeconds > 0) {
        root.remainingSeconds -= 1
      }
      if (root.remainingSeconds === 0) {
        root.fetchStatus()
      }
    }
  }

  // Periodic poll as watchdog in case file notifications miss an update
  Timer {
    id: periodicPoll
    interval: 5000
    repeat: true
    running: true
    triggeredOnStart: true
    onTriggered: root.fetchStatus()
  }

  Timer {
    id: delayedStatusRefresh
    interval: 300
    repeat: false
    onTriggered: root.fetchStatus()
  }
}
