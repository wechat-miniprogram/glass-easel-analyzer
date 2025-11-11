use std::sync::{LazyLock, Mutex, RwLock};

use log::Log;
use lsp_types::{LogMessageParams, MessageType, SetTraceParams, ShowMessageParams, TraceValue};

use crate::context::ServerContext;

static GLOBAL_LSP_LOGGER: LazyLock<&'static GlobalLspLogger> = LazyLock::new(|| {
    let g = Box::new(GlobalLspLogger {
        inner: RwLock::new(LspLoggerKind::Cache(Default::default())),
    });
    Box::leak(g)
});

enum LspLoggerKind {
    Cache(Mutex<Vec<LogMessage>>),
    Normal(LspLogger),
}

struct GlobalLspLogger {
    inner: RwLock<LspLoggerKind>,
}

impl GlobalLspLogger {
    fn set_normal_logger(&self, logger: LspLogger) {
        let mut inner = self.inner.write().unwrap();
        let old = std::mem::replace(&mut *inner, LspLoggerKind::Normal(logger));
        match old {
            LspLoggerKind::Cache(cache) => {
                let LspLoggerKind::Normal(inner) = &*inner else {
                    unreachable!()
                };
                for log_message in cache.lock().unwrap().drain(..) {
                    inner.log(log_message);
                }
            }
            LspLoggerKind::Normal(_) => {}
        }
    }
}

impl Log for GlobalLspLogger {
    #[inline]
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        match &*self.inner.read().unwrap() {
            LspLoggerKind::Cache(_) => true,
            LspLoggerKind::Normal(inner) => inner.enabled(metadata),
        }
    }

    #[inline]
    fn log(&self, record: &log::Record) {
        let is_current_module_message = {
            let module_path = record.module_path().unwrap_or_default();
            module_path == "glass_easel_analyzer"
                || module_path.starts_with("glass_easel_analyzer::")
        };
        let message = format!("{}", record.args());
        let full_message = format!(
            "[{}:{}] {}",
            record.file().unwrap_or(""),
            record.line().unwrap_or(0),
            message,
        );
        let log_message = LogMessage {
            message,
            full_message,
            level: record.level(),
            is_current_module_message,
        };
        match &*self.inner.read().unwrap() {
            LspLoggerKind::Cache(inner) => {
                inner.lock().unwrap().push(log_message);
            }
            LspLoggerKind::Normal(inner) => {
                inner.log(log_message);
            }
        }
    }

    #[inline]
    fn flush(&self) {
        // empty
    }
}

struct LogMessage {
    message: String,
    full_message: String,
    level: log::Level,
    is_current_module_message: bool,
}

struct LspLogger {
    ctx: ServerContext,
    trace: TraceValue,
}

impl LspLogger {
    #[inline]
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let trace = self.trace.clone();
        if trace == TraceValue::Off && metadata.level() <= log::Level::Info {
            return false;
        }
        return true;
    }

    #[inline]
    fn log(&self, log_message: LogMessage) {
        if log_message.level > log::Level::Info {
            if self.trace == TraceValue::Off && !log_message.is_current_module_message {
                return;
            }
        }
        match log_message.level {
            log::Level::Error => {
                if log_message.is_current_module_message {
                    let _ = self.ctx.send_notification(
                        "window/showMessage",
                        ShowMessageParams {
                            message: log_message.message,
                            typ: MessageType::ERROR,
                        },
                    );
                }
                let _ = self.ctx.send_notification(
                    "window/logMessage",
                    LogMessageParams {
                        message: log_message.full_message,
                        typ: MessageType::ERROR,
                    },
                );
            }
            log::Level::Warn => {
                if log_message.is_current_module_message {
                    let _ = self.ctx.send_notification(
                        "window/showMessage",
                        ShowMessageParams {
                            message: log_message.message,
                            typ: MessageType::WARNING,
                        },
                    );
                }
                let _ = self.ctx.send_notification(
                    "window/logMessage",
                    LogMessageParams {
                        message: log_message.full_message,
                        typ: MessageType::WARNING,
                    },
                );
            }
            log::Level::Info => {
                let _ = self.ctx.send_notification(
                    "window/logMessage",
                    LogMessageParams {
                        message: log_message.full_message,
                        typ: MessageType::INFO,
                    },
                );
            }
            _ => {
                eprintln!("{}", log_message.message);
                // let _ = self.ctx.send_notification("$/logTrace", LogTraceParams {
                //     message: log_message.full_message,
                //     verbose: None,
                // });
            }
        }
    }
}

pub(crate) fn init_trace() {
    log::set_logger(&*GLOBAL_LSP_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::max());
}

pub(crate) async fn set_trace(ctx: ServerContext, params: SetTraceParams) -> anyhow::Result<()> {
    let new_lsp_logger = LspLogger {
        ctx,
        trace: params.value,
    };
    GLOBAL_LSP_LOGGER.set_normal_logger(new_lsp_logger);
    Ok(())
}
