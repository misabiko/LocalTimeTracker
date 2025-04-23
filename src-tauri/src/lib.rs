use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

#[tauri::command]
fn get_entries(date: &str) -> Vec<TimeSheetEntry> {
    let timesheet_path = std::env::var("TIMESHEET_PATH").unwrap();
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .from_path(timesheet_path)
        .unwrap();
    let entries = rdr.deserialize::<TimeSheetEntry>();

    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();

    println!("[get_entries] date: {}", date);
    entries
        .into_iter()
        // .map(|entry| entry.unwrap())
        .filter_map(|e| {
            let entry = e.unwrap();
            let start_time = entry.start_time.date_naive();
            if start_time == date {
                Some(entry)
            } else {
                None
            }
        })
        .collect::<Vec<TimeSheetEntry>>()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().unwrap();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_entries,])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "TogglEntryRaw")]
struct TimeSheetEntry {
    description: String,
    start_time: DateTime<Utc>,
    end_time: Option<DateTime<Utc>>,
    // duration: Duration,
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TogglEntryRaw {
    #[serde(rename = "Description")]
    description: String,
    #[serde(rename = "Start date")]
    start_date: String,
    #[serde(rename = "Start time")]
    start_time: String,
    #[serde(rename = "End date")]
    end_date: String,
    #[serde(rename = "End time")]
    end_time: String,
    // #[serde(rename = "Duration")]
    // duration: String,
    #[serde(rename = "Tags")]
    tags: String,
}

const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
impl TryFrom<TogglEntryRaw> for TimeSheetEntry {
    type Error = &'static str;

    fn try_from(value: TogglEntryRaw) -> Result<Self, Self::Error> {
        let start_time = NaiveDateTime::parse_from_str(
            &format!("{} {}", value.start_date, value.start_time),
            DATE_FORMAT,
        )
        .unwrap() //TODO Use ?
        .and_utc();
        let end_time = NaiveDateTime::parse_from_str(
            &format!("{} {}", value.end_date, value.end_time),
            DATE_FORMAT,
        )
        .unwrap()
        .and_utc();

        Ok(TimeSheetEntry {
            description: value.description,
            start_time,
            end_time: Some(end_time),
            tags: value.tags.split(',').map(|s| s.to_string()).collect(),
        })
    }
}

impl Serialize for TimeSheetEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("TimeSheetEntry", 5)?;
        state.serialize_field("description", &self.description)?;
        state.serialize_field("start_time", &self.start_time.to_string())?;
        state.serialize_field("end_time", &self.end_time.map(|dt| dt.to_string()))?;
        state.serialize_field("tags", &self.tags)?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_entries() {
        dotenvy::dotenv().ok();
        let entries = get_entries("2025-04-02");
        println!("{:#?}", entries);
    }
}
