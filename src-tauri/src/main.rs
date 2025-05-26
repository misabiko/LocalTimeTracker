// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    //op run --env-file ../.env -- cargo run -- --add-worklogs
    if args.iter().find(|a| a.starts_with("--")).is_some() {
        if args.contains(&"--add-worklogs".to_string()) {
            dotenvy::dotenv().unwrap();
            tokio::runtime::Runtime::new().unwrap().block_on(local_timesheet_lib::jira::add_missing_worklogs());
        }else {
            panic!("Unknown argument: {:?}", args);
        }
    }else {
        local_timesheet_lib::run()
    }
}
