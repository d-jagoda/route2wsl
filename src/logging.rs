use std::{env, fs, panic};
use chrono::Local;
use fern::Dispatch;
use log::{error, LevelFilter};

pub fn init_service_logger(log_level: LevelFilter) -> Result<(), fern::InitError> {

    let logs_dir = env::current_exe()?.parent().unwrap().join("logs");
    let logs_file = logs_dir.clone().join("route2wsl.log");

    fs::create_dir_all(logs_dir)?;

    Dispatch::new()
        .level(log_level)
        .chain(Dispatch::new()
        .format(move |out, message, record| {
           let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S.%3f").to_string();
           let level = record.level();
           let msg = message;

           out.finish(format_args!(
               "[{}] [{}]: {}",
               timestamp, level, msg
           ))
        })
        .chain( fern::log_file(logs_file)?))
        .apply()?;

    set_panic_hook();

    Ok(())
}

fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info | {
        let message = match panic_info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match panic_info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Unknown panic message",
            }
        };

        let location = panic_info.location().map(|loc|  format!("{}:{}", loc.file(), loc.line()))
        .unwrap_or_else(|| "Unknown location".to_string());

        error!("Panic occurred at {}: {}", location, message);
        eprintln!("Panic occurred at {}: {}", location, message);
    }));    
}