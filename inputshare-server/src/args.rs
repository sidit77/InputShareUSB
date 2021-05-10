use std::str::FromStr;

const HELP: &str = concat!("\
USAGE:
  inputshare-server [OPTIONS]
FLAGS:
  -h, --help            Prints help information
  --console             Prints the packages instead of using them
OPTIONS:
  --port PORT           Sets the port [default: 60067]
");

#[derive(Debug, Copy, Clone)]
pub enum BackendType {
    Hardware,
    Console
}

impl FromStr for BackendType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hardware" => Ok(BackendType::Hardware),
            "console" => Ok(BackendType::Console),
            _ => Err(anyhow::anyhow!("[{}] is a viable backend type. Supported types: [hardware, console]", s))
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ServerArgs {
    pub port: u16,
    pub backend: BackendType
}

pub fn parse_args() -> Result<ServerArgs, pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let args = ServerArgs {
        port: pargs.opt_value_from_str("--port")?.unwrap_or(60067),
        backend: match pargs.contains("--console") {
            true => BackendType::Console,
            false => BackendType::Hardware
        }
    };

    let remaining = pargs.finish();
    if !remaining.is_empty() {
        eprintln!("Warning: unused arguments left: {:?}.", remaining);
    }

    Ok(args)
}