//
// Logger
//

pub fn init(verbosity: u8) {
    use env_logger::Env;
    use log::LevelFilter;

    env_logger::Builder::from_env(Env::default())
        .format_timestamp(None)
        .format_target(verbosity > 2)
        .filter_level(match verbosity {
            1 => LevelFilter::Info,
            2 => LevelFilter::Debug,
            _ if verbosity > 2 => LevelFilter::Trace,
            _ => LevelFilter::Warn,
        })
        .init();
}
