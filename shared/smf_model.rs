#[derive(Debug, Clone)]
pub struct Event {
    pub delta: u32,
    pub tick: u64,
    pub status: u8,
    pub meta_type: u8,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub events: Vec<Event>,
}

impl Track {
    pub fn name(&self) -> Option<String> {
        self.events
            .iter()
            .find(|e| e.status == 0xff && e.meta_type == 3)
            .map(|e| String::from_utf8_lossy(&e.data).into_owned())
    }
}

#[derive(Debug, Clone)]
pub struct Music {
    pub format: u8,
    pub division: u16,
    pub tracks: Vec<Track>,
}
