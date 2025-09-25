#![cfg(feature = "obs")]

fn main() {
    let service = "obs_demo";
    let commit = option_env!("GIT_COMMIT_SHA").unwrap_or("dev");

    if let Err(e) = credit_engine_core::obs::init::init(service, commit, "127.0.0.1:9898") {
        eprintln!("[obs_demo] init failed: {e}");
        std::process::exit(1);
    }

    println!("[obs_demo] initialized (service={service}, commit={commit}).");
    println!("[obs_demo] scrape metrics em http://127.0.0.1:9898/metrics");
}
