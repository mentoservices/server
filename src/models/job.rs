use mongodb::bson::{oid::ObjectId, DateTime as BsonDateTime};
use serde::{Deserialize, Serialize};
use rocket_okapi::okapi::schemars::JsonSchema;
use chrono::{DateTime as ChronoDateTime, Utc, NaiveDateTime};

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Open,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum JobType {
    FullTime,
    PartTime,
    Contract,
    Freelance,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Job {
    pub id: Option<ObjectId>,
    pub posted_by: ObjectId,
    pub title: String,
    pub description: String,
    pub category: Option<String>,
    pub job_type: Option<String>,
    pub salary_min: Option<f64>,
    pub salary_max: Option<f64>,
    pub location: Option<String>,
    pub city: Option<String>,
    pub pincode: Option<String>,
    pub required_skills: Vec<String>,
    pub experience_required: Option<String>,
    pub status: String,
    pub applications: Vec<ObjectId>,
    pub views: i32,
    pub is_active: bool,
    pub expires_at: Option<BsonDateTime>,
    pub created_at: BsonDateTime,
    pub updated_at: BsonDateTime,
}

// API-facing DTO
#[derive(Debug, Serialize, JsonSchema)]
pub struct JobResponse {
    pub id: Option<String>,
    pub posted_by: String,
    pub title: String,
    pub description: String,
    pub category: Option<String>,
    pub job_type: Option<String>,
    pub salary_min: Option<f64>,
    pub salary_max: Option<f64>,
    pub location: Option<String>,
    pub city: Option<String>,
    pub pincode: Option<String>,
    pub required_skills: Vec<String>,
    pub experience_required: Option<String>,
    pub status: String,
    pub applications: Vec<String>,
    pub views: i32,
    pub is_active: bool,
    pub expires_at: Option<ChronoDateTime<Utc>>,
    pub created_at: ChronoDateTime<Utc>,
    pub updated_at: ChronoDateTime<Utc>,
}

impl From<Job> for JobResponse {
    fn from(j: Job) -> Self {
        // helper inline closure to convert BsonDateTime -> chrono::DateTime<Utc>
        let bson_to_chrono = |dt: BsonDateTime| -> ChronoDateTime<Utc> {
            let millis = dt.timestamp_millis();
            let secs = millis / 1000;
            // normalize remainder to positive u32 nanoseconds
            let millis_rem = (millis % 1000).abs() as u32;
            let nsecs = millis_rem * 1_000_000;
            ChronoDateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp(secs, nsecs),
                Utc,
            )
        };

        let id = j.id.map(|o| o.to_hex());
        let posted_by = j.posted_by.to_hex();
        let applications = j
            .applications
            .into_iter()
            .map(|o| o.to_hex())
            .collect::<Vec<_>>();

        let expires_at = j.expires_at.map(|b| bson_to_chrono(b));

        JobResponse {
            id,
            posted_by,
            title: j.title,
            description: j.description,
            category: j.category,
            job_type: j.job_type,
            salary_min: j.salary_min,
            salary_max: j.salary_max,
            location: j.location,
            city: j.city,
            pincode: j.pincode,
            required_skills: j.required_skills,
            experience_required: j.experience_required,
            status: j.status,
            applications,
            views: j.views,
            is_active: j.is_active,
            expires_at,
            created_at: bson_to_chrono(j.created_at),
            updated_at: bson_to_chrono(j.updated_at),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateJobDto {
    pub title: String,
    pub description: String,
    pub category: String,
    pub job_type: JobType,
    pub salary_min: Option<f64>,
    pub salary_max: Option<f64>,
    pub location: String,
    pub city: String,
    pub pincode: String,
    pub required_skills: Vec<String>,
    pub experience_required: Option<i32>,
}