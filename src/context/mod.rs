use std::{future::Future, path::{Path, PathBuf}, pin::Pin, sync::{Arc, Mutex}};

use lsp_server::{Message, Notification};
use lsp_types::Uri;
use tokio::{fs, sync::mpsc};

pub(crate) mod project;

type TaskFn = Box<dyn 'static + Send + FnOnce(&mut project::Project) -> Pin<Box<dyn 'static + Send + Future<Output = ()>>>>;

#[derive(Clone)]
pub(crate) struct ServerContext {
    sender: mpsc::WeakUnboundedSender<Message>,
    projects: Arc<Mutex<Vec<(PathBuf, mpsc::UnboundedSender<TaskFn>)>>>,
}

impl ServerContext {
    pub(crate) fn new(sender: &mpsc::UnboundedSender<Message>) -> Self {
        let sender = sender.downgrade();
        Self {
            sender,
            projects: Arc::new(Mutex::new(vec![])),
        }
    }

    pub(crate) fn send_notification<T: serde::Serialize>(&self, method: &str, params: T) -> anyhow::Result<()> {
        let method = method.to_string();
        let params = serde_json::to_value(params)?;
        if let Some(sender) = self.sender.upgrade() {
            sender.send(Message::Notification(Notification { method, params })).unwrap();
        }
        Ok(())
    }

    async fn get_project_thread_sender(&self, path: &Path) -> anyhow::Result<mpsc::UnboundedSender<TaskFn>> {
        let mut projects = self.projects.lock().unwrap();
        let item = projects
            .iter()
            .find(|(x, ..)| path.ancestors().skip(1).any(|path| path == x.as_path()));
        let sender = if let Some((_, sender)) = item {
            sender.clone()
        } else {
            // create a new project
            let mut project = None;
            for ancestor in path.ancestors().skip(1) {
                let json = ancestor.join("app.json");
                let wxss = ancestor.join("app.wxss");
                let has_json = fs::metadata(&json).await.map(|x| x.is_file()).unwrap_or(false);
                let has_wxss = fs::metadata(&wxss).await.map(|x| x.is_file()).unwrap_or(false);
                if !has_json && !has_wxss { continue; }
                let mut new_project = project::Project::new(ancestor.to_path_buf());
                new_project.file_content(&json);
                new_project.file_content(&wxss);
                project = Some(new_project);
                break;
            }
            let Some(project) = project else {
                return Err(anyhow::Error::msg("Cannot find a proper project for this file. Please make sure an `app.json` or `app.wxss` exists."));
            };
            log::debug!("Project discovered: {}", project.root().to_str().unwrap_or(""));
            let (sender, mut receiver) = mpsc::unbounded_channel();
            projects.push((project.root().to_path_buf(), sender.clone()));
            tokio::task::spawn_blocking(move || {
                let mut project = project;
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                runtime.block_on(async move {
                    while let Some(f) = receiver.recv().await {
                        f(&mut project).await;
                    }
                });
            });
            sender
        };
        Ok(sender)
    }

    pub(crate) async fn project_thread_async_task<R: 'static + Send, F: 'static + Send + Future<Output = R>>(
        &self,
        uri: &Uri,
        f: impl 'static + Send + FnOnce(&mut project::Project, PathBuf) -> F,
    ) -> anyhow::Result<R> {
        let Some(path) = crate::utils::url_to_path(uri) else {
            return Err(anyhow::Error::msg("Illegal file URI"));
        };
        let sender = self.get_project_thread_sender(&path).await?;
        let (ret_sender, ret_receiver) = tokio::sync::oneshot::channel();
        sender.send(Box::new(move |project| {
            let fut = f(project, path);
            Box::pin(async {
                let r = fut.await;
                let _ = ret_sender.send(r);
            })
        })).unwrap();
        let r = ret_receiver.await.unwrap();
        Ok(r)
    }

    pub(crate) async fn project_thread_task<R: 'static + Send>(
        &self,
        uri: &Uri,
        f: impl 'static + Send + FnOnce(&mut project::Project, PathBuf) -> R,
    ) -> anyhow::Result<R> {
        self.project_thread_async_task(uri, |project, abs_path| {
            let ret = f(project, abs_path);
            async { ret }
        }).await
    }
}
