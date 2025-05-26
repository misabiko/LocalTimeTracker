use std::collections::HashMap;
use crate::{get_entries, update_entry, TimeSheetEntry, TimeSheetEntryFrontEnd};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use chrono::{DateTime, Utc};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;

async fn create_worklog(entry: &TimeSheetEntry) -> Result<Worklog, ()> {
    if entry.properties.contains_key("jira_worklog_id") {
        return Err(());
    }

    let Some(jira_id) = entry.properties.get("jira") else {
        return Err(());
    };

    let started = format_for_jira(&entry.start_time.to_utc());
    let time_spent_seconds = ((entry.end_time.unwrap().timestamp_millis()
        - entry.start_time.timestamp_millis()) as f32)
        / (1000.0);
    let time_spent_seconds = time_spent_seconds as u32;

    let body = format!(
        r#"{{
        "started": "{}",
        "timeSpentSeconds": {},
        "comment": "{}"
    }}"#,
        started, time_spent_seconds, entry.description
    );
    //TODO Requires comment field, but it would be nice to have the description→title separate from describing what we're currently doing

    let client = reqwest::Client::new();
    let worklog_response = client
        .post(&format!(
            "{}rest/api/2/issue/{}/worklog",
            std::env::var("VITE_JIRA_URL_PREFIX").unwrap(),
            jira_id
        ))
        .header(
            AUTHORIZATION,
            format!(
                "Basic {}",
                BASE64_STANDARD.encode(format!(
                    "{}:{}",
                    std::env::var("JIRA_USERNAME").unwrap(),
                    std::env::var("JIRA_PASSWORD").unwrap()
                ))
            ),
        )
        .header(CONTENT_TYPE, "application/json")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await
        .unwrap();

    let response_str = worklog_response.text().await.unwrap();
    // println!("Response: {response_str}");

    let worklog = serde_json::from_str::<Worklog>(&response_str);

    let worklog = match worklog {
        Err(e) => {
            println!("Error creating worklog: {e}");
            return Err(());
        }
        Ok(w) => w,
    };

    let mut new_entry = entry.clone();
    new_entry
        .properties
        .insert("jira_worklog_id".to_string(), worklog.id.clone());
    let new_entry = TimeSheetEntryFrontEnd::from(new_entry);
    let description = new_entry.description.clone();
    let start_time = new_entry.start_time.clone();
    if !update_entry(description, start_time, new_entry) {
        panic!("Failed to update entry");
    }

    Ok(worklog)
}

pub async fn add_missing_worklogs() {
    let jira_prefix_url = std::env::var("VITE_JIRA_URL_PREFIX").unwrap();

    for (jira_id, entries) in get_jira_entries() {
        if jira_id.is_empty() {
            eprintln!("No jira id found on {entries:#?}");
            continue;
        }
        println!("{jira_prefix_url}browse/{jira_id}");
        for entry in entries.iter() {
            if entry.properties.contains_key("jira_worklog_id") {
                continue;
            }
            println!("{}", entry.description);
            let _r = create_worklog(entry).await.unwrap();
            // println!("{r:#?}");
        }
    }
}

fn get_jira_entries() -> HashMap<String, Vec<TimeSheetEntry>> {
    let entries = get_entries();

    let mut jira_map = HashMap::<String, Vec<TimeSheetEntry>>::new();
    for entry in entries.into_iter() {
        if entry.properties.contains_key("jira") {
            let jira_id = entry.properties.get("jira").unwrap().to_string();
            if jira_map.contains_key(&jira_id) {
                jira_map.get_mut(&jira_id).unwrap().push(entry);
            } else {
                jira_map.insert(jira_id.clone(), vec![entry]);
            }
        }
    }

    jira_map
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct Worklog {
    // #[serde(rename = "self")]
    // _self: String,
    // author: JiraAuthor,
    // update_author: JiraAuthor,
    comment: String,
    created: String,
    updated: String,
    started: String,
    time_spent: String,
    time_spent_seconds: u32,
    id: String,
    issue_id: String,
}

fn format_for_jira(dt: &DateTime<Utc>) -> String {
    //still prints with a colon which we don't want
    // dt.to_rfc3339_opts(SecondsFormat::Millis, false)

    //Format: 2025-05-06T12:34:00.000+0000
    dt.format("%Y-%m-%dT%H:%M:%S%.3f%z").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::get_entries;
    use base64::prelude::BASE64_STANDARD;
    use base64::Engine;
    use chrono::{Local, TimeZone};
    use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
    use std::collections::HashMap;

    #[test]
    fn test_total_jira_time() {
        dotenvy::dotenv().unwrap();

        let jira_map = get_jira_entries();
        let jira_map = jira_map
            .into_iter()
            .map(|j| {
                let mut total_time = 0.0;
                for entry in j.1.iter() {
                    if let Some(end_time) = entry.end_time {
                        //TODO TimeSheetEntry::duration() -> chrono::Duration
                        total_time += ((end_time.timestamp_millis()
                            - entry.start_time.timestamp_millis())
                            as f32)
                            / (1000.0 * 60.0 * 60.0);
                    }
                }
                (j.0, total_time)
            })
            .collect::<HashMap<String, f32>>();

        println!("{jira_map:#?}");
    }

    #[test]
    fn test_individual_jira_time() {
        dotenvy::dotenv().unwrap();
        let jira_url_prefix = std::env::var("VITE_JIRA_URL_PREFIX").unwrap();

        let jira_map = get_jira_entries();
        for (jira_id, entries) in jira_map.iter() {
            println!("{jira_url_prefix}browse/{jira_id}");
            for entry in entries.iter() {
                if let Some(end_time) = entry.end_time {
                    // let duration = ((end_time.timestamp_millis() - entry.start_time.timestamp_millis()) as f32) / (1000.0 * 60.0 * 60.0);
                    let duration = ((end_time.timestamp_millis()
                        - entry.start_time.timestamp_millis())
                        as f32)
                        / (1000.0);
                    println!(
                        "\t{}, Duration: {duration:.2} sec, Entry: {}",
                        entry.start_time.with_timezone(&Local),
                        entry.description
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_getting_worklog() {
        dotenvy::dotenv().unwrap();

        let client = reqwest::Client::new();
        // let config = Configuration {
        // 	base_path: std::env::var("VITE_JIRA_URL_PREFIX").unwrap(),
        // 	user_agent: None,
        // 	client,
        // 	basic_auth: Some(
        // 		jira_v3_openapi::apis::configuration::BasicAuth {
        // 			username: std::env::var("JIRA_USERNAME").unwrap(),
        // 			password: std::env::var("JIRA_PASSWORD").unwrap(),
        // 		},
        // 	),
        // 	oauth_access_token: None,
        // 	bearer_access_token: None,
        // 	api_key: None,
        // };
        // let r = jira_v3_openapi::apis::issue_worklogs_api::get_issue_worklog(
        // 	&config,
        // 	&std::env::var("TEST_JIRA_ID").unwrap(),
        // 	None,
        // 	None,
        // 	None,
        // 	None,
        // 	None,
        // ).await.unwrap();

        //op run --env-file ../.env -- cargo test test_getting_worklog -- --nocapture

        let r = client
            .get(&format!(
                "{}rest/api/2/issue/{}/worklog",
                std::env::var("VITE_JIRA_URL_PREFIX").unwrap(),
                std::env::var("TEST_JIRA_ID").unwrap()
            ))
            .header(
                AUTHORIZATION,
                format!(
                    "Basic {}",
                    BASE64_STANDARD.encode(format!(
                        "{}:{}",
                        std::env::var("JIRA_USERNAME").unwrap(),
                        std::env::var("JIRA_PASSWORD").unwrap()
                    ))
                ),
            )
            .header(CONTENT_TYPE, "application/json")
            .header("Accept", "application/json")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        println!("{r:#?}");
    }

    #[tokio::test]
    async fn test_adding_worklog() {
        dotenvy::dotenv().unwrap();

        //op run --env-file ../.env -- cargo test test_adding_worklog -- --nocapture

        let jira_map = get_jira_entries();
        let entry = jira_map
            .get(&std::env::var("TEST_JIRA_ID").unwrap())
            .unwrap()
            .iter()
            .nth(1)
            .unwrap();
        println!("{entry:#?}");

        let r = create_worklog(entry).await.unwrap();
        println!("{r:#?}");
    }

    #[tokio::test]
    async fn test_add_missing_worklogs() {
        dotenvy::dotenv().unwrap();

        //op run --env-file ../.env -- cargo test test_add_missing_worklogs -- --nocapture

        add_missing_worklogs().await;
    }

    #[test]
    fn test_format_for_jira() {
        let dt = Utc.with_ymd_and_hms(2025, 5, 6, 12, 34, 0).unwrap();
        let dt = format_for_jira(&dt);
        println!("{dt:#?}");
        assert_eq!(dt, "2025-05-06T12:34:00.000+0000");
    }
}
