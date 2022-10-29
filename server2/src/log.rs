#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 1,
    Warning = 2,
    Info = 3,
    Debug = 4,
}

pub const LOG_LEVEL: LogLevel = LogLevel::Debug;

impl LogLevel {
    pub fn init() {
        if LOG_LEVEL > LogLevel::Error {
            LogLevel::Warning.log(|| colored::control::set_override(true));
        }
    }

    pub fn log<T: FnOnce()>(&self, f: T) {
        if self <= &LOG_LEVEL {
            f()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::log::LogLevel;

    #[test]
    fn comparison() {
        assert!(LogLevel::Error < LogLevel::Info);
        let level1 = LogLevel::Warning;
        let level2 = LogLevel::Error;
        assert!(&level1 > &level2);
        assert!(&level1 == &LogLevel::Warning);
        assert!(level1 == LogLevel::Warning);
    }
}
