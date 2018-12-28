pub struct ProcessedData {
    pub upcoming_trains: Vec<(i64, String)>,
    pub big_countdown: Option<String>,
    pub big_countdown_line: Option<String>,
    pub station_name: String,
}

impl ProcessedData {
    pub fn empty() -> ProcessedData {
        return ProcessedData{
            upcoming_trains: vec![],
            big_countdown: None,
            big_countdown_line: None,
            station_name: "".to_string(),
        };
    }
}
