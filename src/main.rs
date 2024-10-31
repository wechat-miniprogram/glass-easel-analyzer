use clap::Parser;

#[derive(clap::Parser)]
#[clap(version, about)]
struct Args {}

fn main() -> anyhow::Result<()> {
    let _args = Args::parse();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        glass_easel_analyzer::run().await
    })?;
    Ok(())
}
