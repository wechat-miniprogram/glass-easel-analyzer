use clap::Parser;

#[derive(clap::Parser)]
#[clap(version, about)]
struct Args {}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let _args = Args::parse();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async {
        glass_easel_analyzer::run().await
    })?;
    Ok(())
}
