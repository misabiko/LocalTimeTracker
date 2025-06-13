pub mod jira;

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Datelike, Days, Local, NaiveDate, NaiveDateTime};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

pub fn get_entries() -> Vec<TimeSheetEntry> {
    let mut entries: Vec<TimeSheetEntry> = Vec::new();
    if let Ok(toggl_sheet_path) = std::env::var("TOGGL_SHEET_PATH") {
        let mut toggl_rdr = csv::ReaderBuilder::new()
            .delimiter(b',')
            .from_path(toggl_sheet_path)
            .unwrap();
        entries.extend(
            toggl_rdr
                .deserialize::<TogglEntryRaw>()
                .into_iter()
                .map(|e| {
                    let mut entry: TimeSheetEntry = e.unwrap().try_into().unwrap();
                    if !entry.tags.iter().any(|t| t == "Toggl") {
                        entry.tags.push("Toggl".to_string());
                    }
                    entry
                }),
        );
    }

    let timesheet_path = std::env::var("TIMESHEET_PATH").unwrap();
    if std::fs::exists(&timesheet_path).unwrap() {
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b',')
            .from_path(timesheet_path)
            .unwrap();
        entries.extend(
            rdr.deserialize::<TimeSheetEntryRaw>()
                .into_iter()
                .map(|e| e.unwrap().try_into().unwrap()),
        );
    }

    entries
}

//TODO unwrap to ?
#[tauri::command]
fn get_date_entries(date: &str) -> Vec<TimeSheetEntryFrontEnd> {
    let entries: Vec<TimeSheetEntry> = get_entries();

    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();

    entries
        .into_iter()
        // .map(|entry| entry.unwrap())
        .filter(|e| e.start_time.date_naive() == date)
        .map(|e| e.into())
        .collect::<Vec<TimeSheetEntryFrontEnd>>()
}

#[tauri::command]
fn add_entry(entry: TimeSheetEntryFrontEnd) -> bool {
    let timesheet_path = std::env::var("TIMESHEET_PATH").unwrap();
    let existing_entries = get_entries();
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
fn update_entry(
    old_description: String,
    old_start_time: i64,
    entry: TimeSheetEntryFrontEnd,
) -> bool {
    let timesheet_path = std::env::var("TIMESHEET_PATH").unwrap();
    let mut existing_entries = get_entries();

    let entry: TimeSheetEntry = entry.try_into().unwrap();
    let old_start_time = DateTime::from_timestamp_millis(old_start_time).unwrap();
    let index = existing_entries
        .iter()
        .position(|e| old_description == e.description && old_start_time == e.start_time)
        .expect("Entry not found");
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
    let existing_entries = match csv::Reader::from_path(&timesheet_path) {
        Ok(mut reader) => reader
            .deserialize::<TimeSheetEntryRaw>()
            .into_iter()
            .map(|e| e.unwrap().try_into().unwrap())
            .collect::<Vec<TimeSheetEntry>>(),
        //TODO Pattern match for NotFound specifically and panic otherwise
        Err(_) => vec![],
    };

    let start_time = DateTime::from_timestamp_millis(start_time).unwrap();
    let existing_entries: Vec<TimeSheetEntry> = existing_entries
        .into_iter()
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
    let entries: Vec<TimeSheetEntry> = get_entries();

    let partial_lower = partial.to_lowercase();
    let mut seen = HashSet::new();
    let mut suggestions = Vec::new();
    for entry in entries.iter().rev() {
        let desc = entry.description.trim().to_lowercase();
        let tags: std::collections::BTreeSet<_> =
            entry.tags.iter().map(|s| s.trim().to_lowercase()).collect();
        if desc.starts_with(&partial_lower) || desc.contains(&partial_lower) {
            let key = (desc.clone(), tags.clone());
            if !seen.contains(&key) {
                seen.insert(key);
                let mut properties = entry.properties.clone();
                //TODO Distinguish between template properties and instance properties
                properties.remove("jira_worklog_id");
                //TODO TimeSheetEntry::into::<TimeSheetEntryTemplate>()
                suggestions.push(TimeSheetEntryTemplate {
                    description: entry.description.clone(),
                    tags: entry.tags.clone(),
                    properties,
                });
                if suggestions.len() >= 5 {
                    break;
                }
            }
        }
    }
    suggestions
}

fn _equivalent_entry(a: &TimeSheetEntry, b: &TimeSheetEntry) -> bool {
    if a.description.trim() != b.description.trim() {
        return false;
    }
    if HashSet::<&String>::from_iter(a.tags.iter()) != HashSet::<&String>::from_iter(b.tags.iter())
    {
        return false;
    }
    //TODO For now don't consider properties, later distinguish "template properties" from "instance properties" (jira id vs worklog id)
    // if a.properties.len() != b.properties.len() {
    // 	return false;
    // }
    // for (k, v) in a.properties.iter() {
    // 	if b.properties.get(k) != Some(v) {
    // 		return false;
    // 	}
    // }
    true
}

pub fn run() {
    dotenvy::dotenv().expect(".env file with TIMESHEET_PATH should be in src-tauri");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_date_entries,
            add_entry,
            update_entry,
            delete_entry,
            suggest_entry_descriptions,
            get_remaining_week_hours,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "TimeSheetEntryRaw")]
pub struct TimeSheetEntry {
    pub description: String,
    pub start_time: DateTime<Local>,
    pub end_time: Option<DateTime<Local>>,
    //TODO tags Set
    pub tags: Vec<String>,
    pub properties: HashMap<String, String>,
}

impl TimeSheetEntry {
    // fn duration(&self) -> Duration {
    //     Duration::milliseconds(self.duration_millis())
    // }

    fn duration_hours(&self) -> f64 {
        self.duration_millis() as f64 / 3600000.0
    }

    fn duration_millis(&self) -> i64 {
        self.end_time.unwrap_or_else(|| Local::now()).timestamp_millis() - self.start_time.timestamp_millis()
    }
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

#[derive(Debug, Serialize, Deserialize, Clone)]
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
        let end_time = value.end_time.map(|et| {
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
            tags: value
                .tags
                .unwrap_or_default()
                .split(',')
                .map(|s| s.to_string())
                .collect(),
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
        let end_time = value.end_time.map(|et| {
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

impl From<TimeSheetEntry> for TimeSheetEntryFrontEnd {
    fn from(entry: TimeSheetEntry) -> Self {
        Self {
            description: entry.description,
            start_time: entry.start_time.timestamp_millis(),
            end_time: entry.end_time.map(|dt| dt.timestamp_millis()),
            tags: entry.tags.join(","),
            properties: entry.properties,
        }
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

fn get_total_duration_for_date(date: &NaiveDate) -> f64 {
    let entries = get_entries();

    let mut total_hours = 0.0;

    let entries = entries
        .into_iter()
        .filter(|e| &e.start_time.date_naive() == date)
        .collect::<Vec<TimeSheetEntry>>();

    for entry in entries {
        total_hours += entry.duration_hours();
    }

    total_hours
}

fn get_total_duration_for_week() -> f64 {
	let entries = get_entries();

	let mut total_hours = 0.0;
	let mut monday = Local::now().date_naive();
	let current_day = Local::now().date_naive().weekday();
	for _ in 0..current_day.num_days_from_monday() {
		monday = monday.pred_opt().expect("Date underflow");
	}
	let sunday = monday.checked_add_days(Days::new(6)).expect("Date overflow");

	let entries = entries
		.into_iter()
		.filter(|e| &e.start_time.date_naive() >= &monday && &e.start_time.date_naive() <= &sunday)
		.collect::<Vec<TimeSheetEntry>>();

	for entry in entries {
        total_hours += entry.duration_hours();
	}

	total_hours
}

#[tauri::command]
fn get_remaining_week_hours(holidays: u8) -> f64 {
    (5 - holidays) as f64 * 8.0 - get_total_duration_for_week()
}

pub fn purge_duplicates() {
	use std::fs::File;
	use std::io::{BufReader, BufWriter, BufRead, Write};
	use csv::{Reader, Writer};

	let file_path = std::env::var("TIMESHEET_PATH").unwrap();
	let header = {
		let file = File::open(&file_path).unwrap();
		BufReader::new(file).lines().next().unwrap().unwrap()
	};
	let file = File::open(&file_path).unwrap();
	let mut reader = Reader::from_reader(BufReader::new(file));

	let mut records = Vec::new();
	for result in reader.records() {
		records.push(result.unwrap());
	}

	let mut seen = HashSet::new();
	records.retain(|entry| seen.insert(entry.as_slice().to_string()));


	let file = File::create(&file_path).unwrap();

	let mut writer = BufWriter::new(file);
	//WriterBuilder::has_header only works with serialize, not with write_record
	writer.write_all(header.as_bytes()).unwrap();
	writer.write_all(b"\n").unwrap();

	let mut writer = Writer::from_writer(writer);

	for record in records {
		writer.write_record(&record).unwrap();
	}
	writer.flush().unwrap();
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
            assert!(suggestions
                .iter()
                .any(|s| &s.description == desc && &s.tags == tags));
        }

        // Should match nothing
        let suggestions = suggest_entry_descriptions("xyz");
        assert_eq!(suggestions.len(), 0);
    }

    // #[test]
    // fn test_single_day_total_time() {
    //     dotenvy::dotenv().unwrap();

    //     let date = NaiveDate::from_ymd_opt(2025, 5, 22).unwrap();
    //     let total_hours = get_total_duration_for_date(&date);

    //     println!("Total hours for {}: {:.2}", date, total_hours);
    // }

	// #[test]
	// fn test_week_remaining_time() {
	// 	dotenvy::dotenv().unwrap();

    //     let remaining_hours = get_remaining_week_hours(1);

	// 	println!("Remaining hours: {:.2}", remaining_hours);
	// }

	#[test]
	fn test_single_add_entry_no_duplication() {
		use tempfile::NamedTempFile;
		use chrono::{Local, Duration};
		use std::fs::File;
		use std::io::{BufRead, BufReader};

		// Create a temp file and set TIMESHEET_PATH
		let temp_file = NamedTempFile::new().expect("Failed to create temp file");
		let temp_path = temp_file.path().to_path_buf();
		std::env::set_var("TIMESHEET_PATH", &temp_path);

		// Add a single entry
		let now = Local::now();
		let entry = TimeSheetEntryFrontEnd {
			description: "Single entry".to_string(),
			start_time: now.timestamp_millis(),
			end_time: Some((now + Duration::minutes(1)).timestamp_millis()),
			tags: "test".to_string(),
			properties: Default::default(),
		};
		assert!(add_entry(entry));

		// Count lines in the CSV file
		let file = File::open(&temp_path).expect("Failed to open temp csv");
		let reader = BufReader::new(file);
		let line_count = reader.lines().count();

		// Should be 2 (header + entry)
		assert_eq!(line_count, 2, "CSV file should have 2 lines, but has {}. This indicates duplication bug.", line_count);
	}

	#[test]
	fn test_add_multiple_entries_no_duplication() {
		use tempfile::NamedTempFile;
		use chrono::{Local, Duration};
		use std::fs::File;
		use std::io::{BufRead, BufReader};

		// Create a temp file and set TIMESHEET_PATH
		let temp_file = NamedTempFile::new().expect("Failed to create temp file");
		let temp_path = temp_file.path().to_path_buf();
		std::env::set_var("TIMESHEET_PATH", &temp_path);

		// Add several unique entries
		let now = Local::now();
		for i in 0..3 {
			let entry = TimeSheetEntryFrontEnd {
				description: format!("Entry {i}"),
				start_time: (now + Duration::minutes(i)).timestamp_millis(),
				end_time: Some((now + Duration::minutes(i+1)).timestamp_millis()),
				tags: "test".to_string(),
				properties: Default::default(),
			};
			assert!(add_entry(entry));
		}

		// Add one more entry
		let entry = TimeSheetEntryFrontEnd {
			description: "Final entry".to_string(),
			start_time: (now + Duration::minutes(10)).timestamp_millis(),
			end_time: Some((now + Duration::minutes(11)).timestamp_millis()),
			tags: "test".to_string(),
			properties: Default::default(),
		};
		assert!(add_entry(entry));

		// Count lines in the CSV file
		let file = File::open(&temp_path).expect("Failed to open temp csv");
		let reader = BufReader::new(file);
		let line_count = reader.lines().count();

		// Should be 5 (not more)
		assert_eq!(line_count, 5, "CSV file should have 5 lines, but has {}. This indicates exponential growth/duplication bug.", line_count);
	}

	#[test]
	fn test_update_single_entry_no_duplication() {
		use tempfile::NamedTempFile;
		use chrono::{Local, Duration};
		use std::fs::File;
		use std::io::{BufRead, BufReader};

		// Create a temp file and set TIMESHEET_PATH
		let temp_file = NamedTempFile::new().expect("Failed to create temp file");
		let temp_path = temp_file.path().to_path_buf();
		std::env::set_var("TIMESHEET_PATH", &temp_path);

		// Add an entry
		let now = Local::now();
		let entry = TimeSheetEntryFrontEnd {
			description: "Original entry".to_string(),
			start_time: now.timestamp_millis(),
			end_time: Some((now + Duration::minutes(1)).timestamp_millis()),
			tags: "test".to_string(),
			properties: Default::default(),
		};
		assert!(add_entry(entry.clone()));

		// Update the entry
		let updated_entry = TimeSheetEntryFrontEnd {
			description: "Updated entry".to_string(),
			start_time: now.timestamp_millis(),
			end_time: Some((now + Duration::minutes(2)).timestamp_millis()),
			tags: "test".to_string(),
			properties: Default::default(),
		};
		assert!(update_entry(entry.description, entry.start_time, updated_entry));

		// Count lines in the CSV file
		let file = File::open(&temp_path).expect("Failed to open temp csv");
		let reader = BufReader::new(file);
		let line_count = reader.lines().count();

		// Should be 2 (header + updated entry)
		assert_eq!(line_count, 2, "CSV file should have 2 lines, but has {}. This indicates duplication bug.", line_count);
	}

	#[test]
	fn test_update_multiple_entries_no_duplication() {
		use tempfile::NamedTempFile;
		use chrono::{Local, Duration};
		use std::fs::File;
		use std::io::{BufRead, BufReader};

		// Create a temp file and set TIMESHEET_PATH
		let temp_file = NamedTempFile::new().expect("Failed to create temp file");
		let temp_path = temp_file.path().to_path_buf();
		std::env::set_var("TIMESHEET_PATH", &temp_path);

		// Add multiple entries
		let now = Local::now();
		let mut entries = Vec::new();
		for i in 0..3 {
			let entry = TimeSheetEntryFrontEnd {
				description: format!("Entry {i}"),
				start_time: (now + Duration::minutes(i)).timestamp_millis(),
				end_time: Some((now + Duration::minutes(i+1)).timestamp_millis()),
				tags: "test".to_string(),
				properties: Default::default(),
			};
			assert!(add_entry(entry.clone()));
			entries.push(entry);
		}

		// Update the entries
		for i in 0..entries.len() {
			let updated_entry = TimeSheetEntryFrontEnd {
				description: format!("Updated Entry {i}"),
				start_time: (now + Duration::minutes(i as i64)).timestamp_millis(),
				end_time: Some((now + Duration::minutes(i as i64 + 1)).timestamp_millis()),
				tags: "test".to_string(),
				properties: Default::default(),
			};
			assert!(update_entry(entries[i].description.clone(), entries[i].start_time, updated_entry.clone()));
		}

		// Count lines in the CSV file
		let file = File::open(&temp_path).expect("Failed to open temp csv");
		let reader = BufReader::new(file);
		let line_count = reader.lines().count();

		// Should be 4 (header + updated entries)
		assert_eq!(line_count, 4, "CSV file should have 4 lines, but has {}. This indicates duplication bug.", line_count);
	}

	#[test]
	fn test_get_entries_no_duplication() {
		use tempfile::NamedTempFile;
		use chrono::{Local, Duration};

		// Create a temp file and set TIMESHEET_PATH
		let temp_file = NamedTempFile::new().expect("Failed to create temp file");
		let temp_path = temp_file.path().to_path_buf();
		std::env::set_var("TIMESHEET_PATH", &temp_path);

		// Add multiple entries
		let now = Local::now();
		for i in 0..3 {
			let entry = TimeSheetEntryFrontEnd {
				description: format!("Entry {i}"),
				start_time: (now + Duration::minutes(i)).timestamp_millis(),
				end_time: Some((now + Duration::minutes(i+1)).timestamp_millis()),
				tags: "test".to_string(),
				properties: Default::default(),
			};
			assert!(add_entry(entry));
		}

		// Get entries and check count
		let entries = get_entries();
		assert_eq!(entries.len(), 3, "get_entries should return 3 entries, but returned {}. This indicates duplication bug.", entries.len());
	}

	#[test]
	fn test_purge_duplicates() {
		use tempfile::NamedTempFile;
		use chrono::{Local, Duration};
		use std::fs::File;
		use std::io::{BufRead, BufReader};

		// Create a temp file and set TIMESHEET_PATH
		let temp_file = NamedTempFile::new().expect("Failed to create temp file");
		let temp_path = temp_file.path().to_path_buf();
		std::env::set_var("TIMESHEET_PATH", &temp_path);

		// Add multiple entries with some duplicates
		let now = Local::now();
		for i in 0..3 {
			let entry = TimeSheetEntryFrontEnd {
				description: format!("Entry {i}"),
				start_time: (now + Duration::minutes(i)).timestamp_millis(),
				end_time: Some((now + Duration::minutes(i+1)).timestamp_millis()),
				tags: "test".to_string(),
				properties: Default::default(),
			};
			assert!(add_entry(entry.clone()));
			// Add duplicate
			assert!(add_entry(entry));
		}

		// Purge duplicates
		purge_duplicates();

		println!("{}", std::fs::read_to_string(&temp_path).unwrap());

		// Expect the file to have 4 lines
		let file = File::open(&temp_path).expect("Failed to open temp csv");
		let reader = BufReader::new(file);
		let line_count = reader.lines().count();
		assert_eq!(line_count, 4, "CSV file should have 4 lines, but has {}.", line_count);

		// Get entries and check count
		let entries = get_entries();
		assert_eq!(entries.len(), 3, "After purging duplicates, get_entries should return 3 entries, but returned {}. This indicates purge_duplicates is not working correctly.", entries.len());
	}
}