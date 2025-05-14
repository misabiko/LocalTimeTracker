use std::collections::{HashMap, HashSet};

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

	println!("{entries:#?}");
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();

    entries
        .into_iter()
        // .map(|entry| entry.unwrap())
        .filter(|e| e.start_time.date_naive() == date)
        .collect::<Vec<TimeSheetEntry>>()
}

#[tauri::command]
fn add_entry(entry: TimeSheetEntryFrontEnd) -> bool {
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

    for existing_entry in existing_entries {
        writer.serialize(existing_entry).unwrap();
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSheetEntryTemplate {
	description: String,
	tags: Vec<String>,
	properties: HashMap<String, String>,
}

#[tauri::command]
fn suggest_entry_descriptions(partial: &str) -> Vec<TimeSheetEntryTemplate> {
	let mut entries: Vec<TimeSheetEntry> = Vec::new();
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

	let partial_lower = partial.to_lowercase();
	let mut seen = HashSet::new();
	let mut suggestions = Vec::new();
	for entry in entries.iter().rev() {
		let desc = entry.description.trim().to_lowercase();
		let tags: std::collections::BTreeSet<_> = entry.tags.iter().map(|s| s.trim().to_lowercase()).collect();
		if desc.starts_with(&partial_lower) || desc.contains(&partial_lower) {
			let key = (desc.clone(), tags.clone());
			if !seen.contains(&key) {
				seen.insert(key);
				//TODO TimeSheetEntry::into::<TimeSheetEntryTemplate>()
				suggestions.push(TimeSheetEntryTemplate {
					description: entry.description.clone(),
					tags: entry.tags.clone(),
					properties: entry.properties.clone(),
				});
				if suggestions.len() >= 5 { break; }
			}
		}
	}
	suggestions
}

fn _equivalent_entry(a: &TimeSheetEntry, b: &TimeSheetEntry) -> bool {
	if a.description.trim() != b.description.trim() {
		return false;
	}
	if HashSet::<&String>::from_iter(a.tags.iter()) != HashSet::<&String>::from_iter(b.tags.iter()) {
		return false;
	}
	if a.properties.len() != b.properties.len() {
		return false;
	}
	for (k, v) in a.properties.iter() {
		if b.properties.get(k) != Some(v) {
			return false;
		}
	}
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
			suggest_entry_descriptions,
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
    //TODO tags Set
    tags: Vec<String>,
	properties: HashMap<String, String>,
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
    description: Option<String>,
    start_time: i64,
    end_time: Option<i64>,
    tags: Option<String>,
	properties: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TimeSheetEntryFrontEnd {
    description: String,
    start_time: i64,
    end_time: Option<i64>,
	//TODO to array
    tags: String,
	properties: HashMap<String, String>,
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
            properties: HashMap::new(),
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

        // Parse properties string into HashMap
        let mut properties = HashMap::new();
		if let Some(properties_str) = value.properties {
			if !properties_str.trim().is_empty() {
				for pair in properties_str.split(',') {
					let mut kv = pair.splitn(2, '=');
					if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
						properties.insert(k.to_string(), v.to_string());
					}
				}
			}
		}

		Ok(TimeSheetEntry {
			description: value.description.unwrap_or_default(),
            start_time,
            end_time,
            tags: value.tags.unwrap_or_default().split(',').map(|s| s.to_string()).collect(),
            properties,
        })
    }
}

impl TryFrom<TimeSheetEntryFrontEnd> for TimeSheetEntry {
	type Error = &'static str;

	fn try_from(value: TimeSheetEntryFrontEnd) -> Result<Self, Self::Error> {
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

		Ok(Self {
			description: value.description,
			start_time,
			end_time,
			tags: value.tags.split(',').map(|s| s.to_string()).collect(),
			properties: value.properties,
		})
	}
}

//TODO Separate csv and json serialization
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
		//csv parser doesn't support HashMap
		let mut properties = Vec::new();
		for (k, v) in self.properties.iter() {
			properties.push(format!("{k}={v}"));
		}
		state.serialize_field("properties", &properties.join(","))?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::PathBuf;

    #[test]
    fn test_suggest_entry_descriptions() {
        let mut file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        file_path.push("tests/timesheet.csv");
        env::set_var("TIMESHEET_PATH", &file_path);

        // Should match 'work' (case-insensitive, deduped, most recent first)
        let suggestions = suggest_entry_descriptions("work");
        assert_eq!(suggestions[0].description, "Work on project");
        assert_eq!(suggestions[0].tags, vec!["dev"]);
        assert_eq!(suggestions.len(), 1);

        // Should match 'e' (multiple, most recent first, max 5)
        let suggestions = suggest_entry_descriptions("e");
        let expected = [
            ("Work on project", vec!["dev"]),
            ("Meeting", vec!["meeting"]),
            ("Email", vec!["admin"]),
            ("Another thing", vec!["misc"]),
        ];
        assert_eq!(suggestions.len(), 4);
        for (desc, tags) in expected.iter() {
            assert!(suggestions.iter().any(|s| &s.description == desc && &s.tags == tags));
        }

        // Should match nothing
        let suggestions = suggest_entry_descriptions("xyz");
        assert_eq!(suggestions.len(), 0);
    }
}