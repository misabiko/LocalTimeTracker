use chrono::{DateTime, Local, NaiveDate, NaiveDateTime};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

//TODO unwrap to ?
#[tauri::command]
fn get_entries(date: &str) -> Vec<TimeSheetEntry> {
    let mut entries: Vec<TimeSheetEntry> = Vec::new();
    if let Ok(toggl_sheet_path) = std::env::var("TOGGL_SHEET_PATH") {
        let mut toggl_rdr = csv::ReaderBuilder::new()
            .delimiter(b',')
            .from_path(toggl_sheet_path)
            .unwrap();
        entries.extend(toggl_rdr.deserialize::<TogglEntryRaw>()
            .into_iter()
            .map(|e| {
                let mut entry: TimeSheetEntry = e.unwrap().try_into().unwrap();
                if !entry.tags.iter().any(|t| t == "Toggl") {
                    entry.tags.push("Toggl".to_string());
                }
                entry
            })
        );
    }

    let timesheet_path = std::env::var("TIMESHEET_PATH").unwrap();
    if std::fs::exists(&timesheet_path).unwrap() {
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b',')
            .from_path(timesheet_path)
            .unwrap();
        entries.extend(rdr.deserialize::<TimeSheetEntryRaw>()
            .into_iter()
            .map(|e| e.unwrap().try_into().unwrap())
        );
    }

    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();

    entries
        .into_iter()
        // .map(|entry| entry.unwrap())
        .filter(|e| e.start_time.date_naive() == date)
        .collect::<Vec<TimeSheetEntry>>()
}

#[tauri::command]
fn add_entry(entry: TimeSheetEntryRaw) -> bool {
    let timesheet_path = std::env::var("TIMESHEET_PATH").unwrap();
    let existing_entries = match csv::Reader::from_path(&timesheet_path)
    {
        Ok(mut reader) => reader.deserialize::<TimeSheetEntryRaw>()
            .into_iter()
            .map(|e| e.unwrap().try_into().unwrap())
            .collect::<Vec<TimeSheetEntry>>(),
        //TODO Pattern match for NotFound specifically and panic otherwise
        Err(_) => vec![],
    };
    let mut writer = csv::Writer::from_path(&timesheet_path).unwrap();

    for entry in existing_entries {
        writer.serialize(entry).unwrap();
    }
    let entry: TimeSheetEntry = entry.try_into().unwrap();
    writer.serialize(entry).unwrap();
    writer.flush().unwrap();

    println!("[INFO] Entry added to timesheet");

    true
}

#[tauri::command]
fn update_entry(old_description: String, old_start_time: i64, entry: TimeSheetEntryRaw) -> bool {
    let timesheet_path = std::env::var("TIMESHEET_PATH").unwrap();
    let mut existing_entries = match csv::Reader::from_path(&timesheet_path)
    {
        Ok(mut reader) => reader.deserialize::<TimeSheetEntryRaw>()
            .into_iter()
            .map(|e| e.unwrap().try_into().unwrap())
            .collect::<Vec<TimeSheetEntry>>(),
        //TODO Pattern match for NotFound specifically and panic otherwise
        Err(_) => vec![],
    };

    let entry: TimeSheetEntry = entry.try_into().unwrap();
    let old_start_time = DateTime::from_timestamp_millis(old_start_time).unwrap();
    let index = existing_entries.iter().position(|e|
        old_description == e.description && old_start_time == e.start_time
    ).expect("Entry not found");
    existing_entries[index] = entry;

    let mut writer = csv::Writer::from_path(&timesheet_path).unwrap();

    for entry in existing_entries {
        // let entry: TimeSheetEntry = raw_entry.try_into().unwrap();
        writer.serialize(entry).unwrap();
    }
    writer.flush().unwrap();

    true
}

#[tauri::command]
fn delete_entry(description: String, start_time: i64) -> bool {
    let timesheet_path = std::env::var("TIMESHEET_PATH").unwrap();
    let existing_entries = match csv::Reader::from_path(&timesheet_path)
    {
        Ok(mut reader) => reader.deserialize::<TimeSheetEntryRaw>()
            .into_iter()
            .map(|e| e.unwrap().try_into().unwrap())
            .collect::<Vec<TimeSheetEntry>>(),
        //TODO Pattern match for NotFound specifically and panic otherwise
        Err(_) => vec![],
    };

    let start_time = DateTime::from_timestamp_millis(start_time).unwrap();
    let existing_entries: Vec<TimeSheetEntry> = existing_entries.into_iter()
        .filter(|e| e.description != description || e.start_time != start_time)
        .collect();

    let mut writer = csv::Writer::from_path(&timesheet_path).unwrap();

    for entry in existing_entries {
        writer.serialize(entry).unwrap();
    }
    writer.flush().unwrap();

    true
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().expect(".env file with TIMESHEET_PATH should be in src-tauri");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_entries,
            add_entry,
            update_entry,
            delete_entry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "TimeSheetEntryRaw")]
struct TimeSheetEntry {
    description: String,
    start_time: DateTime<Local>,
    end_time: Option<DateTime<Local>>,
    // duration: Duration,
    //TODO tags Set
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TogglEntryRaw {
    #[serde(alias = "Description")]
    description: String,
    #[serde(alias = "Start date")]
    start_date: String,
    #[serde(alias = "Start time")]
    start_time: String,
    #[serde(alias = "End date")]
    end_date: String,
    #[serde(alias = "End time")]
    end_time: String,
    // #[serde(alias = "Duration")]
    // duration: String,
    #[serde(alias = "Tags")]
    tags: String,
}

#[derive(Debug, Deserialize)]
struct TimeSheetEntryRaw {
    description: String,
    start_time: i64,
    end_time: Option<i64>,
    // duration: Duration,
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
        .and_local_timezone(Local)
        .unwrap();
        let end_time = NaiveDateTime::parse_from_str(
            &format!("{} {}", value.end_date, value.end_time),
            DATE_FORMAT,
        )
        .unwrap()
        .and_local_timezone(Local)
        .unwrap();

        Ok(TimeSheetEntry {
            description: value.description,
            start_time,
            end_time: Some(end_time),
            tags: value.tags.split(',').map(|s| s.to_string()).collect(),
        })
    }
}

impl TryFrom<TimeSheetEntryRaw> for TimeSheetEntry {
    type Error = &'static str;

    fn try_from(value: TimeSheetEntryRaw) -> Result<Self, Self::Error> {
        let start_time = DateTime::from_timestamp_millis(value.start_time)
            .unwrap() //TODO Use ?
            .with_timezone(&Local);
        let end_time = value
            .end_time
            .map(|et| {
                DateTime::from_timestamp_millis(et)
                    .unwrap() //TODO Use ?
                    .with_timezone(&Local)
            });

        Ok(TimeSheetEntry {
            description: value.description,
            start_time,
            end_time,
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
        state.serialize_field("start_time", &self.start_time.timestamp_millis())?;
        state.serialize_field("end_time", &self.end_time.map(|dt| dt.timestamp_millis()))?;
        state.serialize_field("tags", &self.tags.join(","))?;
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

    #[tokio::test]
    async fn test_add_entry() {
        dotenvy::dotenv().ok();
        let entry = TimeSheetEntryRaw {
            description: "Test Entry".to_string(),
            start_time: Local::now().timestamp_millis(),
            end_time: None,
            tags: "test".to_string(),
        };
        let result = add_entry(entry);
        assert!(result);
    }
}