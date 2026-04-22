use crate::app::{Task, TaskList, App};
use google_tasks1::{api, TasksHub};
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod, authenticator::Authenticator, parse_application_secret};
use yup_oauth2::authenticator_delegate::InstalledFlowDelegate;
use hyper_rustls::HttpsConnector;
use hyper::client::HttpConnector;
use std::path::PathBuf;
use std::pin::Pin;
use std::future::Future;
use tokio::sync::mpsc;
use crate::events::Event;
use chrono::{DateTime, Utc};

struct TuiDelegate { sender: mpsc::UnboundedSender<Event> }
impl InstalledFlowDelegate for TuiDelegate {
    fn present_user_url<'a>(&'a self, url: &'a str, _need_code: bool) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        let url = url.to_string(); let sender = self.sender.clone();
        Box::pin(async move { let _ = sender.send(Event::NeedsAuth(url)); Ok(String::new()) })
    }
}

pub struct ApiClient { hub: Option<TasksHub<HttpsConnector<HttpConnector>>>, auth: Option<Authenticator<HttpsConnector<HttpConnector>>> }

impl ApiClient {
    pub async fn new(sender: mpsc::UnboundedSender<Event>) -> Self {
        // Estrategia: Config Dir > Current Dir > Embedded
        let mut secret_path = App::get_config_dir();
        secret_path.push("client_secret.json");

        let secret = if secret_path.exists() {
            yup_oauth2::read_application_secret(&secret_path).await.ok()
        } else {
            let local_path = PathBuf::from("client_secret.json");
            if local_path.exists() {
                yup_oauth2::read_application_secret(&local_path).await.ok()
            } else {
                // Fallback: Credenciales incrustadas en el binario
                let embedded_json = include_str!("../client_secret.json");
                parse_application_secret(embedded_json).ok()
            }
        };

        let secret = match secret {
            Some(s) => s,
            None => return Self { hub: None, auth: None },
        };

        let mut token_path = App::get_config_dir(); token_path.push("pomotask_token.json");
        let auth = match InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect).persist_tokens_to_disk(token_path).flow_delegate(Box::new(TuiDelegate { sender })).build().await { Ok(a) => a, Err(_) => return Self { hub: None, auth: None } };
        let https = hyper_rustls::HttpsConnectorBuilder::new().with_native_roots().expect("no native roots found").https_or_http().enable_http1().build();
        let client = hyper::Client::builder().build(https);
        let hub = TasksHub::new(client, auth.clone());
        Self { hub: Some(hub), auth: Some(auth) }
    }

    async fn ensure_full_permissions(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(auth) = &self.auth {
            let scopes = &["https://www.googleapis.com/auth/tasks", "https://www.googleapis.com/auth/tasks.readonly"];
            auth.token(scopes).await?;
        }
        Ok(())
    }

    pub async fn fetch_task_lists(&self) -> Result<Vec<TaskList>, Box<dyn std::error::Error + Send + Sync>> {
        self.ensure_full_permissions().await?;
        let hub = match &self.hub { Some(h) => h, None => return Ok(vec![TaskList { id: "@default".to_string(), title: "Default".to_string() }]) };
        let (_, list) = hub.tasklists().list().doit().await?;
        Ok(list.items.unwrap_or_default().into_iter().map(|l| TaskList { id: l.id.unwrap_or_default(), title: l.title.unwrap_or_default() }).collect())
    }

    pub async fn fetch_tasks(&self, list_id: &str, show_completed: bool) -> Result<Vec<Task>, Box<dyn std::error::Error + Send + Sync>> {
        self.ensure_full_permissions().await?;
        let hub = match &self.hub { Some(h) => h, None => return Ok(self.mock_tasks()) };
        
        let (_, task_list) = hub.tasks().list(list_id)
            .show_completed(show_completed)
            .show_hidden(show_completed)
            .doit().await?;
        
        Ok(task_list.items.unwrap_or_default().into_iter().filter(|t| t.title.is_some()).map(|t| {
            let due = t.due.and_then(|d| DateTime::parse_from_rfc3339(&d).ok().map(|dt| dt.with_timezone(&Utc)));
            let updated = t.updated.and_then(|d| DateTime::parse_from_rfc3339(&d).ok().map(|dt| dt.with_timezone(&Utc))).unwrap_or_else(Utc::now);
            Task { id: t.id.unwrap_or_default(), title: t.title.unwrap_or_else(|| "Untitled".to_string()), completed: t.status == Some("completed".to_string()), due, updated, notes: t.notes, parent_id: t.parent, pomodoros: 0 }
        }).collect())
    }

    pub async fn toggle_task_completion(&self, list_id: &str, task_id: &str, completed: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.ensure_full_permissions().await?;
        let hub = match &self.hub { Some(h) => h, None => return Ok(()) };
        let mut task = api::Task::default();
        task.status = Some(if completed { "completed".to_string() } else { "needsAction".to_string() });
        hub.tasks().patch(task, list_id, task_id).doit().await?;
        Ok(())
    }

    pub async fn create_task(&self, list_id: &str, title: &str, notes: Option<String>, due: Option<DateTime<Utc>>, parent_id: Option<String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.ensure_full_permissions().await?;
        let hub = match &self.hub { Some(h) => h, None => return Ok(()) };
        let mut task = api::Task::default();
        task.title = Some(title.to_string()); 
        task.notes = notes; 
        task.due = due.map(|d| d.to_rfc3339());
        
        if let Some(ref pid) = parent_id {
            task.parent = Some(pid.clone());
        }

        let mut call = hub.tasks().insert(task, list_id);
        if let Some(ref pid) = parent_id {
            call = call.parent(pid);
        }

        let res = call.doit().await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Error creating task: {:?}", e);
                Err(Box::new(e))
            }
        }
    }

    pub async fn update_task(&self, list_id: &str, task_id: &str, title: &str, notes: Option<String>, due: Option<DateTime<Utc>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.ensure_full_permissions().await?;
        let hub = match &self.hub { Some(h) => h, None => return Ok(()) };
        let mut task = api::Task::default();
        task.title = Some(title.to_string()); task.notes = notes; task.due = due.map(|d| d.to_rfc3339());
        hub.tasks().patch(task, list_id, task_id).doit().await?;
        Ok(())
    }

    fn mock_tasks(&self) -> Vec<Task> { vec![Task { id: "1".to_string(), title: "Modo Simulado".to_string(), completed: false, due: None, updated: Utc::now(), notes: None, parent_id: None, pomodoros: 0 }] }
}
