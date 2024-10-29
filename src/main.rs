use clap::Parser;

#[derive(clap::Parser)]
#[clap(version, about)]
struct Args {}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let _args = Args::parse();
    glass_easel_analyzer::run()?;
    Ok(())
}
