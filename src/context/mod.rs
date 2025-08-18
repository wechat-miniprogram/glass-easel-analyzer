use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
};

use lsp_server::{Message, Notification};
use lsp_types::Url;
use tokio::sync::mpsc;

pub(crate) mod backend_configuration;
pub(crate) mod project;

type TaskFn = Box<
    dyn 'static
        + Send
        + FnOnce(&mut project::Project) -> Pin<Box<dyn 'static + Send + Future<Output = ()>>>,
>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileLang {
    Unknown,
    Wxml,
    Wxss,
    Json,
    OtherSs,
}

pub(crate) struct ServerContextOptions {
    pub(crate) ignore_paths: Vec<PathBuf>,
}

#[derive(Clone)]
pub(crate) struct ServerContext {
    sender: mpsc::WeakUnboundedSender<Message>,
    backend_config: Arc<backend_configuration::BackendConfig>,
    projects: Arc<Mutex<Vec<(PathBuf, mpsc::UnboundedSender<TaskFn>)>>>,
    default_project: mpsc::UnboundedSender<TaskFn>,
    options: Arc<ServerContextOptions>,
}

impl ServerContext {
    pub(crate) fn new(
        sender: &mpsc::UnboundedSender<Message>,
        backend_config: backend_configuration::BackendConfig,
        initial_projects: Vec<project::Project>,
        options: ServerContextOptions,
    ) -> Self {
        let sender = sender.downgrade();
        let mut ret = Self {
            sender,
            backend_config: Arc::new(backend_config),
            projects: Arc::new(Mutex::new(vec![])),
            default_project: Self::spawn_project_thread(Default::default()),
            options: Arc::new(options),
        };
        for proj in initial_projects {
            ret.add_project(proj);
        }
        ret
    }

    pub(crate) fn options(&self) -> &ServerContextOptions {
        &self.options
    }

    fn spawn_project_thread(project: project::Project) -> mpsc::UnboundedSender<TaskFn> {
        let (sender, mut receiver) = mpsc::unbounded_channel::<TaskFn>();
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
    }

    pub(crate) fn add_project(&mut self, project: project::Project) {
        let p = project.root().unwrap().to_path_buf();
        let sender = Self::spawn_project_thread(project);
        self.projects.lock().unwrap().push((p, sender));
    }

    pub(crate) fn backend_config(&self) -> Arc<backend_configuration::BackendConfig> {
        self.backend_config.clone()
    }

    pub(crate) fn send_notification<T: serde::Serialize>(
        &self,
        method: &str,
        params: T,
    ) -> anyhow::Result<()> {
        let method = method.to_string();
        let params = serde_json::to_value(params)?;
        if let Some(sender) = self.sender.upgrade() {
            sender
                .send(Message::Notification(Notification { method, params }))
                .unwrap();
        }
        Ok(())
    }

    async fn get_project_thread_sender(
        &self,
        path: &Path,
    ) -> anyhow::Result<mpsc::UnboundedSender<TaskFn>> {
        let projects = self.projects.lock().unwrap();
        let item = projects
            .iter()
            .find(|(x, ..)| path.ancestors().skip(1).any(|path| path == x.as_path()));
        let sender = if let Some((_, sender)) = item {
            sender.clone()
        } else {
            self.default_project.clone()
        };
        Ok(sender)
    }

    pub(crate) async fn project_thread_async_task<
        R: 'static + Send,
        F: 'static + Send + Future<Output = R>,
    >(
        &self,
        uri: &Url,
        f: impl 'static + Send + FnOnce(&mut project::Project, PathBuf, FileLang) -> F,
    ) -> anyhow::Result<R> {
        let abs_path = uri.to_file_path();
        let sender = if let Ok(path) = abs_path.as_ref() {
            self.get_project_thread_sender(&path).await?
        } else {
            self.default_project.clone()
        };
        let abs_path = abs_path.unwrap_or_else(|_| crate::utils::generate_non_fs_fake_path(uri));
        let (ret_sender, ret_receiver) = tokio::sync::oneshot::channel();
        sender
            .send(Box::new(move |project| {
                let file_lang = match project.cached_file_content(&abs_path) {
                    Some(content) => content.file_lang,
                    None => FileLang::Unknown,
                };
                let fut = f(project, abs_path, file_lang);
                Box::pin(async {
                    let r = fut.await;
                    let _ = ret_sender.send(r);
                })
            }))
            .unwrap();
        let r = ret_receiver.await.unwrap();
        Ok(r)
    }

    pub(crate) async fn project_thread_task<R: 'static + Send>(
        &self,
        uri: &Url,
        f: impl 'static + Send + FnOnce(&mut project::Project, PathBuf, FileLang) -> R,
    ) -> anyhow::Result<R> {
        self.project_thread_async_task(uri, |project, abs_path, file_lang| {
            let ret = f(project, abs_path, file_lang);
            async { ret }
        })
        .await
    }

    pub(crate) async fn clear_all_projects(&self) {
        let mut projects = self.projects.lock().unwrap();
        projects.clear();
    }
}
