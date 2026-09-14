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
  readonly property string outboxPath: configDir + "/outbox.json"
  readonly property string statsPath: configDir + "/stats.json"
  readonly property string configPath: configDir + "/config.json"
  property string pomotaskBinary: "pomotask-cli"
  // false en las instancias secundarias (un widget por monitor): no lanzan la sincronización
  // periódica ni la comprobación de sesión; leen los resultados de los archivos compartidos.
  property bool primary: true

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
  property bool autoCycle: false     // encadenar fases sin intervención

  // -------------------------------------------------------------------------
  // Google Connection State (escrito por el CLI en runtime_state.json)
  // -------------------------------------------------------------------------
  // null = todavía no comprobado; true/false = último resultado conocido
  property var googleConnected: null
  property int lastSyncAt: 0            // segundos Unix; 0 = nunca
  property string lastSyncError: ""

  readonly property bool googleDisconnected: googleConnected === false
  // La sesión expiró / fue revocada / no existe: solo se arregla iniciando sesión en la TUI.
  readonly property bool authRequired: googleDisconnected
    && (lastSyncError.indexOf("auth_required") === 0 || lastSyncError.indexOf("no_token") === 0)

  readonly property string lastSyncLabel: {
    if (!lastSyncAt || lastSyncAt <= 0) return "Nunca sincronizado"
    var d = new Date(lastSyncAt * 1000)
    var now = new Date()
    var hh = (d.getHours() < 10 ? "0" : "") + d.getHours()
    var mm = (d.getMinutes() < 10 ? "0" : "") + d.getMinutes()
    var sameDay = d.getFullYear() === now.getFullYear() && d.getMonth() === now.getMonth() && d.getDate() === now.getDate()
    return sameDay ? ("Última sync " + hh + ":" + mm) : ("Última sync " + (d.getMonth() + 1) + "/" + d.getDate() + " " + hh + ":" + mm)
  }

  // Cambios hechos sin conexión (outbox.json) que aún no se han subido a Google.
  property int pendingChanges: 0
  readonly property string pendingLabel: pendingChanges === 1
    ? "1 cambio pendiente de subir"
    : pendingChanges + " cambios pendientes de subir"

  readonly property string googleStatusMessage: {
    var pending = pendingChanges > 0 ? " " + pendingLabel + "; se subirán al reconectar." : ""
    if (!googleDisconnected) {
      return pendingChanges > 0 ? (pendingLabel + ". Sincroniza para subirlos a Google.") : ""
    }
    if (lastSyncError.indexOf("no_token") === 0) return "No has iniciado sesión en Google Tasks." + pending
    if (authRequired) return "La sesión de Google expiró. Vuelve a iniciar sesión desde la TUI." + pending
    var detail = lastSyncError.replace(/^Error:\s*/, "")
    return "Sin conexión con Google Tasks. " + (detail !== "" ? detail : "") + pending
  }

  function parseOutbox(raw) {
    try {
      var content = String(raw || "").trim()
      if (content === "") { root.pendingChanges = 0; return }
      var arr = JSON.parse(content)
      root.pendingChanges = Array.isArray(arr) ? arr.length : 0
    } catch (e) {
      console.warn("PomotaskService", "Error parsing outbox:", e)
    }
  }

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
  // Resumen del día (stats.json: claves "YYYY-MM-DD HH:00" en hora local)
  // -------------------------------------------------------------------------
  property int todayPomodoros: 0
  property int todayTasksDone: 0
  property int todayFocusSeconds: 0

  readonly property string todayFocusLabel: {
    var total = Math.max(0, todayFocusSeconds)
    var h = Math.floor(total / 3600)
    var m = Math.floor((total % 3600) / 60)
    if (h > 0) return h + " h " + (m < 10 ? "0" : "") + m + " min"
    return m + " min"
  }

  function todayKeyPrefix() {
    var d = new Date()
    var mm = (d.getMonth() + 1 < 10 ? "0" : "") + (d.getMonth() + 1)
    var dd = (d.getDate() < 10 ? "0" : "") + d.getDate()
    return d.getFullYear() + "-" + mm + "-" + dd
  }

  function sumTodayFrom(map) {
    if (!map || typeof map !== "object") return 0
    var prefix = todayKeyPrefix()
    var total = 0
    for (var key in map) {
      if (String(key).indexOf(prefix) === 0) total += Number(map[key]) || 0
    }
    return total
  }

  function parseStats(raw) {
    try {
      var content = String(raw || "").trim()
      if (content === "") return
      var obj = JSON.parse(content)
      if (!obj || typeof obj !== "object") return
      root.todayPomodoros = sumTodayFrom(obj.hourly_pomodoros)
      root.todayTasksDone = sumTodayFrom(obj.hourly_tasks_done)
      root.todayFocusSeconds = sumTodayFrom(obj.hourly_seconds)
    } catch (e) {
      console.warn("PomotaskService", "Error parsing stats:", e)
    }
  }

  // -------------------------------------------------------------------------
  // Duraciones (config.json, en segundos). Se editan con `ipc config set`.
  // -------------------------------------------------------------------------
  property int focusDuration: 25 * 60
  property int shortBreakDuration: 5 * 60
  property int longBreakDuration: 15 * 60

  function parseConfig(raw) {
    try {
      var content = String(raw || "").trim()
      if (content === "") return
      var obj = JSON.parse(content)
      if (!obj || typeof obj !== "object") return
      if (obj.focus_duration !== undefined) root.focusDuration = Number(obj.focus_duration) || root.focusDuration
      if (obj.short_break_duration !== undefined) root.shortBreakDuration = Number(obj.short_break_duration) || root.shortBreakDuration
      if (obj.long_break_duration !== undefined) root.longBreakDuration = Number(obj.long_break_duration) || root.longBreakDuration
    } catch (e) {
      console.warn("PomotaskService", "Error parsing config:", e)
    }
  }

  // key: "focus" | "short" | "long"; minutes: 1..180
  function setDuration(key, minutes) {
    var m = Math.round(Number(minutes) || 0)
    if (m < 1 || m > 180) return
    if (key === "focus") root.focusDuration = m * 60
    else if (key === "short") root.shortBreakDuration = m * 60
    else if (key === "long") root.longBreakDuration = m * 60
    runAction(["config", "set", String(key), String(m)], "Saving duration…")
  }

  // -------------------------------------------------------------------------
  // Tasks Cache & Blocklist
  // -------------------------------------------------------------------------
  property var tasks: []
  property var taskLists: []
  property var blocklist: null

  // Acción anti-distracción normalizada: "warn" (aviso discreto abajo), "hud" (pantalla de
  // enfoque a pantalla completa) o "minimize" (ocultar la ventana hasta el descanso).
  // Los valores antiguos "warn_and_unfocus"/"unfocus" se tratan como "hud".
  readonly property string distractionAction: normalizeDistractionAction(blocklist ? blocklist.action : "")

  function normalizeDistractionAction(raw) {
    var a = String(raw || "").trim().toLowerCase()
    if (a === "hud" || a === "minimize") return a
    if (a === "warn_and_unfocus" || a === "unfocus") return "hud"
    return "warn"
  }

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
        // Solo aceptamos JSON de runtime_state: otras salidas (p. ej. la tarea creada por
        // `task create`) no deben vaciar la tarea activa.
        if (obj.state === undefined || obj.mode === undefined) return
        root.state = String(obj.state)
        root.mode = String(obj.mode)
        if (obj.google_connected === true || obj.google_connected === false) {
          root.googleConnected = obj.google_connected
        }
        root.lastSyncAt = obj.last_sync_at ? Number(obj.last_sync_at) : 0
        root.lastSyncError = obj.last_sync_error ? String(obj.last_sync_error) : ""
        if (obj.total_seconds !== undefined) root.totalSeconds = Number(obj.total_seconds)
        if (obj.session_pomodoros !== undefined) root.sessionPomodoros = Number(obj.session_pomodoros)
        root.activeTaskId = obj.active_task_id ? String(obj.active_task_id) : ""
        root.activeTaskTitle = obj.active_task_title ? String(obj.active_task_title) : ""
        if (obj.strict_break !== undefined) root.strictBreak = Boolean(obj.strict_break)
        if (obj.anti_distraction !== undefined) root.antiDistraction = Boolean(obj.anti_distraction)
        if (obj.auto_cycle !== undefined) root.autoCycle = Boolean(obj.auto_cycle)

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

  function toggleAutoCycle() {
    runAction(["timer", "toggle-auto"], "Toggling auto cycle…")
  }

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

  // Comprueba la sesión de Google sin descargar tareas (rápido, no abre el navegador).
  function checkAuth() {
    runAction(["auth-status"], "Checking Google session…")
  }

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

  FileView {
    id: statsWatcher
    path: root.statsPath
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: root.parseStats(text())
  }

  FileView {
    id: configWatcher
    path: root.configPath
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: root.parseConfig(text())
  }

  FileView {
    id: outboxWatcher
    path: root.outboxPath
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: root.parseOutbox(text())
    onLoadFailed: root.pendingChanges = 0
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

  // Al cambiar de día el resumen "Hoy" debe vaciarse aunque stats.json no cambie.
  Timer {
    interval: 60 * 1000
    repeat: true
    running: true
    onTriggered: statsWatcher.reload()
  }

  Timer {
    id: delayedStatusRefresh
    interval: 300
    repeat: false
    onTriggered: root.fetchStatus()
  }

  // Comprobación de sesión poco después de arrancar la barra: detecta un token
  // expirado sin esperar a que el usuario pulse "Sincronizar".
  Timer {
    id: initialAuthCheck
    interval: 4000
    repeat: false
    running: root.primary
    onTriggered: root.checkAuth()
  }

  // Sincronización periódica en segundo plano (antes solo se sincronizaba a mano).
  // Si Google falla, el CLI deja el motivo en runtime_state.json y el panel lo muestra.
  Timer {
    id: autoSyncTimer
    interval: 10 * 60 * 1000
    repeat: true
    running: root.primary
    onTriggered: root.syncTasks()
  }
}
