use crate::ds::dust_core::rtc::{Backend, Date, Time};
use std::any::Any;

pub struct RtcBackend {
    _offset_seconds: i64,
}

impl RtcBackend {
    pub fn new(offset_seconds: i64) -> Self {
        RtcBackend { _offset_seconds: offset_seconds }
    }
}

impl Backend for RtcBackend {
    fn as_any(&self) -> &(dyn Any + 'static) { self }
    fn as_any_mut(&mut self) -> &mut (dyn Any + 'static) { self }
    
    fn get_time(&mut self) -> Time {
        Time {
            hour: 12,
            minute: 0,
            second: 0,
        }
    }
    
    fn get_date_time(&mut self) -> (Date, Time) {
        (
            Date {
                years_since_2000: 23,
                month: 1,
                day: 1,
                days_from_sunday: 0,
            },
            self.get_time(),
        )
    }
    
    fn set_date_time(&mut self, _dt: (Date, Time)) {
        
    }
}
