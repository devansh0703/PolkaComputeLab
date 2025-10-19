mod chain_spec;
mod cli;
mod command;
mod rpc;
mod service;
mod ocw;

fn main() -> sc_cli::Result<()> {
    command::run()
}
