use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use rocket_okapi::okapi::schemars::JsonSchema;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum JobSeekerSubscriptionPlan {
    None,
    Basic,
    Premium,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobSeekerProfile {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    
    // Professional Information
    pub full_name: String,
    pub headline: Option<String>, // e.g., "Senior Software Engineer"
    pub bio: Option<String>,
    
    // Skills & Experience
    pub skills: Vec<String>,
    pub experience_years: Option<i32>,
    pub education: Vec<Education>,
    pub work_experience: Vec<WorkExperience>,
    
    // Job Preferences
    pub preferred_categories: Vec<String>,
    pub preferred_job_types: Vec<String>, // "fulltime", "parttime", "contract", "freelance"
    pub preferred_locations: Vec<String>,
    pub expected_salary_min: Option<f64>,
    pub expected_salary_max: Option<f64>,
    pub willing_to_relocate: bool,
    
    // Documents
    pub resume_url: Option<String>,
    pub portfolio_url: Option<String>,
    pub linkedin_url: Option<String>,
    
    // Subscription & Status
    pub subscription_plan: JobSeekerSubscriptionPlan,
    pub subscription_expires_at: Option<DateTime>,
    pub is_verified: bool,
    pub is_available: bool, // Currently looking for jobs
    
    // Metadata
    pub profile_views: i32,
    pub applications_count: i32,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct Education {
    pub degree: String,
    pub institution: String,
    pub field_of_study: Option<String>,
    pub start_year: Option<i32>,
    pub end_year: Option<i32>,
    pub is_current: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct WorkExperience {
    pub title: String,
    pub company: String,
    pub location: Option<String>,
    pub start_date: Option<String>, // ISO date string
    pub end_date: Option<String>,
    pub is_current: bool,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateJobSeekerProfileDto {
    pub full_name: String,
    pub headline: Option<String>,
    pub bio: Option<String>,
    pub skills: Vec<String>,
    pub experience_years: Option<i32>,
    pub education: Vec<Education>,
    pub work_experience: Vec<WorkExperience>,
    pub preferred_categories: Vec<String>,
    pub preferred_job_types: Vec<String>,
    pub preferred_locations: Vec<String>,
    pub expected_salary_min: Option<f64>,
    pub expected_salary_max: Option<f64>,
    pub willing_to_relocate: bool,
    pub resume_url: Option<String>,
    pub portfolio_url: Option<String>,
    pub linkedin_url: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateJobSeekerProfileDto {
    pub full_name: Option<String>,
    pub headline: Option<String>,
    pub bio: Option<String>,
    pub skills: Option<Vec<String>>,
    pub experience_years: Option<i32>,
    pub education: Option<Vec<Education>>,
    pub work_experience: Option<Vec<WorkExperience>>,
    pub preferred_categories: Option<Vec<String>>,
    pub preferred_job_types: Option<Vec<String>>,
    pub preferred_locations: Option<Vec<String>>,
    pub expected_salary_min: Option<f64>,
    pub expected_salary_max: Option<f64>,
    pub willing_to_relocate: Option<bool>,
    pub resume_url: Option<String>,
    pub portfolio_url: Option<String>,
    pub linkedin_url: Option<String>,
    pub is_available: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct JobSeekerProfileResponse {
    pub id: String,
    pub user_id: String,
    pub full_name: String,
    pub headline: Option<String>,
    pub bio: Option<String>,
    pub skills: Vec<String>,
    pub experience_years: Option<i32>,
    pub education: Vec<Education>,
    pub work_experience: Vec<WorkExperience>,
    pub preferred_categories: Vec<String>,
    pub preferred_job_types: Vec<String>,
    pub preferred_locations: Vec<String>,
    pub expected_salary_min: Option<f64>,
    pub expected_salary_max: Option<f64>,
    pub willing_to_relocate: bool,
    pub resume_url: Option<String>,
    pub portfolio_url: Option<String>,
    pub linkedin_url: Option<String>,
    pub subscription_plan: String,
    pub is_verified: bool,
    pub is_available: bool,
    pub profile_views: i32,
    pub applications_count: i32,
}

impl From<JobSeekerProfile> for JobSeekerProfileResponse {
    fn from(profile: JobSeekerProfile) -> Self {
        JobSeekerProfileResponse {
            id: profile.id.unwrap().to_hex(),
            user_id: profile.user_id.to_hex(),
            full_name: profile.full_name,
            headline: profile.headline,
            bio: profile.bio,
            skills: profile.skills,
            experience_years: profile.experience_years,
            education: profile.education,
            work_experience: profile.work_experience,
            preferred_categories: profile.preferred_categories,
            preferred_job_types: profile.preferred_job_types,
            preferred_locations: profile.preferred_locations,
            expected_salary_min: profile.expected_salary_min,
            expected_salary_max: profile.expected_salary_max,
            willing_to_relocate: profile.willing_to_relocate,
            resume_url: profile.resume_url,
            portfolio_url: profile.portfolio_url,
            linkedin_url: profile.linkedin_url,
            subscription_plan: format!("{:?}", profile.subscription_plan).to_lowercase(),
            is_verified: profile.is_verified,
            is_available: profile.is_available,
            profile_views: profile.profile_views,
            applications_count: profile.applications_count,
        }
    }
}