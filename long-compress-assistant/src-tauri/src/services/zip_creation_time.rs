//! Source modification time in ZIP's local, two-second DOS calendar.
use chrono::{Datelike,Timelike};
use std::time::SystemTime;

pub(crate) struct SourceTime {
    pub date:zip_aes::DateTime,
    pub clamped:bool,
}
pub(crate) fn from_system_time(time:SystemTime)->SourceTime {
    let local:chrono::DateTime<chrono::Local>=time.into();
    let (fields,clamped)=if local.year()<1980{((1980,1,1,0,0,0),true)}
    else if local.year()>2107{((2107,12,31,23,59,58),true)}
    else{((local.year()as u16,local.month()as u8,local.day()as u8,local.hour()as u8,local.minute()as u8,(local.second().min(59)as u8)&!1),false)};
    let (year,month,day,hour,minute,second)=fields;
    SourceTime{date:zip_aes::DateTime::from_date_and_time(year,month,day,hour,minute,second).expect("valid local calendar within DOS date range"),clamped}
}
