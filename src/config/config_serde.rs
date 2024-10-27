pub(crate) mod utc_offset {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use time::UtcOffset;

    #[derive(Serialize, Deserialize)]
    struct UtcOffsetDef {
        hours: Option<i8>,
        minutes: Option<i8>,
        seconds: Option<i8>,
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<UtcOffset, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match <Option<UtcOffsetDef>>::deserialize(deserializer)? {
            Some(UtcOffsetDef {
                hours,
                minutes,
                seconds,
            }) => UtcOffset::from_hms(
                hours.unwrap_or(0),
                minutes.unwrap_or(0),
                seconds.unwrap_or(0),
            )
            .ok(),
            None => UtcOffset::current_local_offset().ok(),
        }
        .unwrap_or(UtcOffset::UTC))
    }

    pub fn serialize<S>(utc_offset: &UtcOffset, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (hours, minutes, seconds) = utc_offset.as_hms();
        UtcOffsetDef {
            hours: Some(hours),
            minutes: Some(minutes),
            seconds: Some(seconds),
        }
        .serialize(serializer)
    }
}
