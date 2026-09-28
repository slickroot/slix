use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reply {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub text: String,
    pub to_username: String,
    pub impressions: u64,
    pub likes: u64,
    pub profile_visits: u64,
}

#[derive(Debug)]
pub struct Me {
    pub id: String,
    pub handle: String,
}
