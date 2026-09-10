//! Buzón de salida: cambios hechos sin conexión con Google Tasks que aún no se han subido.
//!
//! Cuando el plugin de Omarchy crea o completa una tarea y Google no responde (sesión
//! expirada, sin red), el cambio queda en `~/.config/pomotask/outbox.json`. Cualquier
//! sincronización posterior debe (1) intentar subir lo pendiente y (2) re-aplicar lo que
//! siga pendiente sobre las tareas recién descargadas, para que no se pierda al pisar la caché.

use crate::api::ApiClient;
use crate::app::{Stats, Task};
use crate::ipc::{
    atomic_write, get_config_dir, load_runtime_state, load_tasks_cache, save_runtime_state,
    save_tasks_cache,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Prefijo de los ids temporales que asigna `ipc task create` a tareas aún no subidas.
pub const LOCAL_ID_PREFIX: &str = "task_";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PendingOp {
    /// Tarea creada localmente; `temp_id` es el id provisional que tiene en la caché.
    Create {
        temp_id: String,
        list_id: String,
        title: String,
        #[serde(default)]
        parent_id: Option<String>,
        created_at: DateTime<Utc>,
    },
    /// Tarea marcada como completada localmente.
    Complete {
        task_id: String,
        list_id: String,
        completed_at: DateTime<Utc>,
    },
}

impl PendingOp {
    pub fn list_id(&self) -> &str {
        match self {
            PendingOp::Create { list_id, .. } | PendingOp::Complete { list_id, .. } => list_id,
        }
    }
}

pub fn get_outbox_path() -> PathBuf {
    get_config_dir().join("outbox.json")
}

pub fn load_outbox() -> Vec<PendingOp> {
    load_outbox_from(&get_outbox_path())
}

pub fn load_outbox_from(path: &Path) -> Vec<PendingOp> {
    match fs::read_to_string(path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn save_outbox(ops: &[PendingOp]) -> std::io::Result<()> {
    save_outbox_to(ops, &get_outbox_path())
}

pub fn save_outbox_to(ops: &[PendingOp], path: &Path) -> std::io::Result<()> {
    let data = serde_json::to_string_pretty(ops)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    atomic_write(path, &data)
}

pub fn pending_count() -> usize {
    load_outbox().len()
}

/// Añade una operación al buzón. Un `Complete` repetido sobre la misma tarea, o un `Create`
/// con el mismo id temporal, reemplaza al anterior. Los `Create` van siempre antes que los
/// `Complete` para que una tarea local pueda crearse y completarse en el mismo envío.
pub fn enqueue(op: PendingOp) -> std::io::Result<()> {
    let mut ops = load_outbox();
    enqueue_into(&mut ops, op);
    save_outbox(&ops)
}

pub fn enqueue_into(ops: &mut Vec<PendingOp>, op: PendingOp) {
    ops.retain(|existing| match (existing, &op) {
        (PendingOp::Create { temp_id: a, .. }, PendingOp::Create { temp_id: b, .. }) => a != b,
        (PendingOp::Complete { task_id: a, .. }, PendingOp::Complete { task_id: b, .. }) => a != b,
        _ => true,
    });
    ops.push(op);
    ops.sort_by_key(|o| matches!(o, PendingOp::Complete { .. }));
}

fn task_from_create(op: &PendingOp) -> Option<Task> {
    if let PendingOp::Create {
        temp_id,
        list_id,
        title,
        parent_id,
        created_at,
    } = op
    {
        Some(Task {
            id: temp_id.clone(),
            list_id: list_id.clone(),
            title: title.clone(),
            completed: false,
            due: None,
            updated: *created_at,
            completed_at: None,
            notes: None,
            parent_id: parent_id.clone(),
            pomodoros: 0,
        })
    } else {
        None
    }
}

/// Re-aplica los cambios pendientes sobre las tareas recién descargadas de `list_id`
/// (`@all` acepta todas las listas). Idempotente.
pub fn apply_to_list(ops: &[PendingOp], list_id: &str, tasks: &mut Vec<Task>) {
    for op in ops {
        let applies = list_id == "@all" || op.list_id() == list_id;
        match op {
            PendingOp::Create { temp_id, .. } if applies => {
                if !tasks.iter().any(|t| &t.id == temp_id) {
                    if let Some(task) = task_from_create(op) {
                        tasks.push(task);
                    }
                }
            }
            PendingOp::Complete {
                task_id,
                completed_at,
                ..
            } => {
                // El `Complete` se aplica por id aunque la lista no coincida: la caché puede
                // tener la tarea bajo `@all` o bajo un list_id distinto al registrado.
                for t in tasks.iter_mut() {
                    if &t.id == task_id && !t.completed {
                        t.completed = true;
                        t.completed_at = Some(*completed_at);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Re-aplica los cambios pendientes sobre una caché completa (todas las listas).
pub fn apply_to_cache(ops: &[PendingOp], cache: &mut HashMap<String, Vec<Task>>) {
    for op in ops {
        if let PendingOp::Create { list_id, .. } = op {
            cache.entry(list_id.clone()).or_default();
        }
    }
    for (lid, tasks) in cache.iter_mut() {
        apply_to_list(ops, lid, tasks);
    }
}

/// Sustituye un id temporal por el id real que devolvió Google en caché, estadísticas y
/// estado del temporizador, para no perder los pomodoros ya contabilizados.
pub fn replace_temp_id(temp_id: &str, new_id: &str) {
    if temp_id == new_id || new_id.is_empty() {
        return;
    }

    let mut cache = load_tasks_cache();
    let mut touched = false;
    for tasks in cache.values_mut() {
        for t in tasks.iter_mut() {
            if t.id == temp_id {
                t.id = new_id.to_string();
                touched = true;
            }
            if t.parent_id.as_deref() == Some(temp_id) {
                t.parent_id = Some(new_id.to_string());
                touched = true;
            }
        }
    }
    if touched {
        let _ = save_tasks_cache(&cache);
    }

    let stats_path = get_config_dir().join("stats.json");
    if let Ok(data) = fs::read_to_string(&stats_path) {
        if let Ok(mut stats) = serde_json::from_str::<Stats>(&data) {
            let mut changed = false;
            if let Some(count) = stats.task_pomodoros.remove(temp_id) {
                *stats.task_pomodoros.entry(new_id.to_string()).or_insert(0) += count;
                changed = true;
            }
            if let Some(timer) = stats.task_timers.remove(temp_id) {
                stats.task_timers.insert(new_id.to_string(), timer);
                changed = true;
            }
            if changed {
                if let Ok(out) = serde_json::to_string_pretty(&stats) {
                    let _ = atomic_write(&stats_path, &out);
                }
            }
        }
    }

    let mut state = load_runtime_state();
    if state.active_task_id.as_deref() == Some(temp_id) {
        state.active_task_id = Some(new_id.to_string());
        let _ = save_runtime_state(&state);
    }
}

/// Intenta subir los cambios pendientes en orden. Cada operación que sube se retira del buzón
/// de inmediato; al primer fallo se detiene y devuelve el error (lo que quede sigue pendiente).
/// Devuelve cuántas operaciones subió.
pub async fn push_pending(
    client: &ApiClient,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut ops = load_outbox();
    let mut pushed = 0usize;

    while let Some(op) = ops.first().cloned() {
        match &op {
            PendingOp::Create {
                temp_id,
                list_id,
                title,
                parent_id,
                ..
            } => {
                let new_id = client
                    .create_task_returning_id(list_id, title, None, None, parent_id.clone())
                    .await?;
                if !new_id.is_empty() {
                    replace_temp_id(temp_id, &new_id);
                    // Los `Complete` pendientes sobre esta tarea deben apuntar al id real.
                    for other in ops.iter_mut() {
                        match other {
                            PendingOp::Complete { task_id, .. } if task_id == temp_id => {
                                *task_id = new_id.clone();
                            }
                            PendingOp::Create {
                                parent_id: Some(p), ..
                            } if p == temp_id => {
                                *p = new_id.clone();
                            }
                            _ => {}
                        }
                    }
                }
            }
            PendingOp::Complete {
                task_id, list_id, ..
            } => {
                if task_id.starts_with(LOCAL_ID_PREFIX) {
                    // Su `Create` no está en el buzón (se perdió): no existe en Google, nada que subir.
                    ops.remove(0);
                    save_outbox(&ops)?;
                    continue;
                }
                client
                    .toggle_task_completion(list_id, task_id, true)
                    .await?;
            }
        }
        ops.remove(0);
        save_outbox(&ops)?;
        pushed += 1;
    }

    Ok(pushed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_op(temp: &str, list: &str, title: &str) -> PendingOp {
        PendingOp::Create {
            temp_id: temp.to_string(),
            list_id: list.to_string(),
            title: title.to_string(),
            parent_id: None,
            created_at: Utc::now(),
        }
    }

    fn complete_op(id: &str, list: &str) -> PendingOp {
        PendingOp::Complete {
            task_id: id.to_string(),
            list_id: list.to_string(),
            completed_at: Utc::now(),
        }
    }

    fn remote_task(id: &str, list: &str) -> Task {
        Task {
            id: id.to_string(),
            list_id: list.to_string(),
            title: format!("remota {}", id),
            completed: false,
            due: None,
            updated: Utc::now(),
            completed_at: None,
            notes: None,
            parent_id: None,
            pomodoros: 0,
        }
    }

    #[test]
    fn enqueue_dedupes_and_orders_creates_first() {
        let mut ops = Vec::new();
        enqueue_into(&mut ops, complete_op("g1", "L"));
        enqueue_into(&mut ops, create_op("task_1", "L", "a"));
        enqueue_into(&mut ops, complete_op("g1", "L"));
        enqueue_into(&mut ops, create_op("task_1", "L", "a (renombrada)"));
        assert_eq!(ops.len(), 2);
        assert!(matches!(&ops[0], PendingOp::Create { title, .. } if title == "a (renombrada)"));
        assert!(matches!(&ops[1], PendingOp::Complete { .. }));
    }

    #[test]
    fn apply_to_list_keeps_local_tasks_and_offline_completions() {
        let ops = vec![
            create_op("task_7", "L1", "local"),
            create_op("task_8", "L2", "otra lista"),
            complete_op("g1", "L1"),
        ];
        let mut l1 = vec![remote_task("g1", "L1"), remote_task("g2", "L1")];
        apply_to_list(&ops, "L1", &mut l1);
        assert_eq!(l1.len(), 3, "la tarea local de L1 se conserva");
        assert!(l1.iter().any(|t| t.id == "task_7"));
        assert!(
            !l1.iter().any(|t| t.id == "task_8"),
            "la de L2 no se mete en L1"
        );
        assert!(l1.iter().find(|t| t.id == "g1").unwrap().completed);

        // Idempotente
        apply_to_list(&ops, "L1", &mut l1);
        assert_eq!(l1.len(), 3);

        let mut all = vec![remote_task("g1", "L1")];
        apply_to_list(&ops, "@all", &mut all);
        assert_eq!(all.len(), 3, "@all recibe las locales de todas las listas");
    }

    #[test]
    fn apply_to_cache_creates_missing_list_entry() {
        let ops = vec![create_op("task_9", "L_nueva", "x")];
        let mut cache: HashMap<String, Vec<Task>> = HashMap::new();
        cache.insert("L1".to_string(), vec![remote_task("g1", "L1")]);
        apply_to_cache(&ops, &mut cache);
        assert_eq!(cache.get("L_nueva").map(|v| v.len()), Some(1));
        assert_eq!(cache.get("L1").map(|v| v.len()), Some(1));
    }

    #[test]
    fn outbox_roundtrip_on_disk() {
        let dir = std::env::temp_dir().join(format!("pomotask_outbox_{}", rand::random::<u64>()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("outbox.json");
        let ops = vec![create_op("task_1", "L", "t"), complete_op("g", "L")];
        save_outbox_to(&ops, &path).unwrap();
        assert_eq!(load_outbox_from(&path), ops);
        let _ = fs::remove_dir_all(&dir);
    }
}
