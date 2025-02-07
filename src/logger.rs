use std::sync::{LazyLock, RwLock};

use lsp_types::{LogMessageParams, MessageType, SetTraceParams, ShowMessageParams, TraceValue};

use crate::context::ServerContext;

static GLOBAL_LSP_LOGGER: LazyLock<&'static GlobalLspLogger> = LazyLock::new(|| {
    let g = Box::new(GlobalLspLogger {
        inner: RwLock::new(None),
    });
    Box::leak(g)
});

struct GlobalLspLogger {
    inner: RwLock<Option<LspLogger>>,
}

impl log::Log for GlobalLspLogger {
    #[inline]
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        if let Some(inner) = self.inner.read().unwrap().as_ref() {
            inner.enabled(metadata)
        } else {
            false
        }
    }

    #[inline]
    fn log(&self, record: &log::Record) {
        if let Some(inner) = self.inner.read().unwrap().as_ref() {
            inner.log(record)
        }
    }

    #[inline]
    fn flush(&self) {
        if let Some(inner) = self.inner.read().unwrap().as_ref() {
            inner.flush()
        }
    }
}

struct LspLogger {
    ctx: ServerContext,
    trace: TraceValue,
}

impl log::Log for LspLogger {
    #[inline]
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let trace = self.trace.clone();
        if trace == TraceValue::Off && metadata.level() <= log::Level::Info {
            return false;
        }
        return true;
    }

    #[inline]
    fn log(&self, record: &log::Record) {
        let trace = self.trace.clone();
        let is_current_module_message = {
            let module_path = record.module_path().unwrap_or_default();
            module_path == "glass_easel_analyzer"
                || module_path.starts_with("glass_easel_analyzer::")
        };
        if record.metadata().level() > log::Level::Info {
            if trace == TraceValue::Off || !is_current_module_message {
                return;
            }
        }
        let message = format!(
            "[{}:{}] {}",
            record.file().unwrap_or(""),
            record.line().unwrap_or(0),
            record.args()
        );
        match record.level() {
            log::Level::Error => {
                if is_current_module_message {
                    let _ = self.ctx.send_notification(
                        "window/showMessage",
                        ShowMessageParams {
                            message: format!("{}", record.args()),
                            typ: MessageType::ERROR,
                        },
                    );
                }
                let _ = self.ctx.send_notification(
                    "window/logMessage",
                    LogMessageParams {
                        message,
                        typ: MessageType::ERROR,
                    },
                );
            }
            log::Level::Warn => {
                if is_current_module_message {
                    let _ = self.ctx.send_notification(
                        "window/showMessage",
                        ShowMessageParams {
                            message: format!("{}", record.args()),
                            typ: MessageType::WARNING,
                        },
                    );
                }
                let _ = self.ctx.send_notification(
                    "window/logMessage",
                    LogMessageParams {
                        message,
                        typ: MessageType::WARNING,
                    },
                );
            }
            log::Level::Info => {
                let _ = self.ctx.send_notification(
                    "window/logMessage",
                    LogMessageParams {
                        message,
                        typ: MessageType::INFO,
                    },
                );
            }
            _ => {
                eprintln!("{}", message);
                // let _ = self.ctx.send_notification("$/logTrace", LogTraceParams {
                //     message,
                //     verbose: None,
                // });
            }
        }
    }

    #[inline]
    fn flush(&self) {
        // empty
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
    *GLOBAL_LSP_LOGGER.inner.write().unwrap() = Some(new_lsp_logger);
    Ok(())
}
