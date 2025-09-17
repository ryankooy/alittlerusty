use tokio::time::Instant;

/// Commands to be matched to LogState method calls
#[derive(Debug, Clone, Copy)]
pub enum LogCommand {
    Start,
    Pause,
    Resume,
    TogglePause,
    Quit,
}

/// Logging state
pub struct LogState {
    /// Whether logging is paused
    paused: bool,

    /// Whether logging was started and is active
    running: bool,

    /// Time logging started or was resumed after being paused
    start_time: Instant,

    /// Total hours logged
    hours: f64,

    /// Total minutes logged
    minutes: u64,
}

impl LogState {
    pub fn new() -> Self {
        Self {
            paused: false,
            running: false,
            start_time: Instant::now(),
            hours: 0.0,
            minutes: 0,
        }
    }

    pub fn toggle_pause(&mut self) {
        if self.running {
            self.paused = !self.paused;
            if self.paused {
                self.update_time();
            } else {
                self.reset_start_time();
            }
        }
    }

    pub fn pause(&mut self) {
        if self.running && !self.paused {
            self.update_time();
            self.paused = true;
        }
    }

    pub fn resume(&mut self) {
        if self.paused {
            self.reset_start_time();
            self.paused = false;
        }
    }

    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn quit(&mut self) {
        if !self.paused {
            self.update_time();
        }
        self.running = false;
    }

    pub fn is_paused(&mut self) -> bool {
        self.paused
    }

    pub fn is_running(&mut self) -> bool {
        self.running
    }

    pub fn get_total_hours(&mut self) -> f64 {
        if !self.paused {
            self.hours + self.get_hours_since_start_time()
        } else {
            self.hours
        }
    }

    pub fn get_total_minutes(&mut self) -> u64 {
        if !self.paused {
            self.minutes + self.get_minutes_since_start_time()
        } else {
            self.minutes
        }
    }

    fn update_time(&mut self) {
        self.hours += self.get_hours_since_start_time();
        self.minutes += self.get_minutes_since_start_time();
    }

    fn get_hours_since_start_time(&mut self) -> f64 {
        if self.running {
            self.start_time.elapsed().as_secs_f64() / 3600.0
        } else {
            0.0
        }
    }

    fn get_minutes_since_start_time(&mut self) -> u64 {
        if self.running {
            self.start_time.elapsed().as_secs() / 60
        } else {
            0
        }
    }

    fn reset_start_time(&mut self) {
        self.start_time = Instant::now();
    }
}
