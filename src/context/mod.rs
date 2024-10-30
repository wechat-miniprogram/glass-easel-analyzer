use std::{collections::HashMap, future::Future, path::{Path, PathBuf}, pin::Pin, sync::{Arc, Mutex}};

use glass_easel_template_compiler::TmplGroup;
use lsp_server::{Message, Notification};
use lsp_types::Uri;
use tokio::{fs, sync::mpsc};

pub(crate) mod utils;

type TaskFn = Box<dyn 'static + Send + FnOnce(&mut Project) -> Pin<Box<dyn 'static + Send + Future<Output = ()>>>>;

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
                let app_json: JsonConfig = if let Ok(content) = fs::read_to_string(&json).await {
                    serde_json::from_str(&content).unwrap_or_default()
                } else {
                    Default::default()
                };
                let app_wxss = if let Ok(content) = fs::read_to_string(&wxss).await {
                    content
                } else {
                    Default::default()
                };
                project = Some(Project {
                    root: ancestor.to_path_buf(),
                    app_json,
                    app_wxss,
                    template_group: TmplGroup::new(),
                });
                break;
            }
            let Some(project) = project else {
                return Err(anyhow::Error::msg("Cannot find a proper project for this file. Please make sure an `app.json` or `app.wxss` exists."));
            };
            log::debug!("Project discovered: {}", project.root().to_str().unwrap_or(""));
            let (sender, mut receiver) = mpsc::unbounded_channel();
            projects.push((project.root.clone(), sender.clone()));
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

    pub(crate) async fn project_thread_task<R: 'static + Send>(
        &self,
        uri: Uri,
        f: impl 'static + Send + FnOnce(&mut Project, PathBuf) -> R,
    ) -> anyhow::Result<R> {
        let Some(path) = utils::url_to_path(&uri) else {
            return Err(anyhow::Error::msg("Illegal file URI"));
        };
        let sender = self.get_project_thread_sender(&path).await?;
        let (ret_sender, ret_receiver) = tokio::sync::oneshot::channel();
        sender.send(Box::new(move |project| {
            let r = f(project, path);
            Box::pin(async {
                let _ = ret_sender.send(r);
            })
        })).unwrap();
        let r = ret_receiver.await.unwrap();
        Ok(r)
    }

    pub(crate) async fn project_thread_async_task<R: 'static + Send, F: 'static + Send + Future<Output = R>>(
        &self,
        uri: Uri,
        f: impl 'static + Send + FnOnce(&mut Project, PathBuf) -> F,
    ) -> anyhow::Result<R> {
        let Some(path) = utils::url_to_path(&uri) else {
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
}

pub(crate) struct Project {
    root: PathBuf,
    app_json: JsonConfig,
    app_wxss: String,
    template_group: TmplGroup,
}

impl Project {
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn unix_rel_path(&self, abs_path: &Path) -> anyhow::Result<String> {
        utils::unix_rel_path(&self.root, abs_path)
    }

    pub(crate) fn template_group(&mut self) -> &mut TmplGroup {
        &mut self.template_group
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JsonConfig {
    #[serde(default)]
    component: bool,
    #[serde(default)]
    using_components: HashMap<String, String>,
}
