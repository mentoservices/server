use rocket::serde::json::Json;
use rocket::fs::TempFile;
use rocket::serde::Deserialize;
use rocket_okapi::openapi;
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use std::path::Path;
use tokio::fs;
use uuid::Uuid;
use crate::guards::AuthGuard;
use crate::utils::{ApiResponse, ApiError};

// ============================================================================
// BASE64 UPLOAD STRUCTS
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(crate = "rocket::serde")]
pub struct Base64UploadRequest {
    pub filename: String,
    pub mime_type: String,
    pub data: String,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn get_extension_from_filename(name: &str) -> Option<String> {
    if let Some(ext) = Path::new(name).extension() {
        return ext.to_str().map(|s| s.to_lowercase());
    }
    
    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() >= 2 {
        let last = parts.last()?;
        return Some(last.to_lowercase());
    }
    
    None
}

fn is_valid_image_extension(ext: &str) -> bool {
    matches!(ext, "jpg" | "jpeg" | "png" | "webp")
}

fn is_valid_document_extension(ext: &str) -> bool {
    matches!(ext, "pdf" | "jpg" | "jpeg" | "png")
}

fn extension_from_content_type(content_type: &str) -> Option<String> {
    match content_type {
        "image/jpeg" => Some("jpg".to_string()),
        "image/jpg" => Some("jpg".to_string()),
        "image/png" => Some("png".to_string()),
        "image/webp" => Some("webp".to_string()),
        "application/pdf" => Some("pdf".to_string()),
        _ => None
    }
}

fn get_extension_from_mime(mime_type: &str) -> Option<String> {
    match mime_type {
        "image/jpeg" | "image/jpg" => Some("jpg".to_string()),
        "image/png" => Some("png".to_string()),
        "image/webp" => Some("webp".to_string()),
        "application/pdf" => Some("pdf".to_string()),
        _ => None
    }
}

fn is_valid_document_mime(mime_type: &str) -> bool {
    matches!(
        mime_type,
        "image/jpeg" | "image/jpg" | "image/png" | "application/pdf"
    )
}

// ============================================================================
// MULTIPART FILE UPLOAD ENDPOINTS
// ============================================================================

#[openapi(tag = "File Upload")]
#[post("/upload/image", data = "<file>")]
pub async fn upload_image(
    mut file: TempFile<'_>,
    _auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    println!("\n========================================");
    println!("IMAGE UPLOAD REQUEST");
    println!("========================================");
    println!("File name: {:?}", file.name());
    println!("Content type: {:?}", file.content_type());
    println!("File length: {:?}", file.len());
    
    let extension = if let Some(name) = file.name() {
        println!("Trying to extract extension from filename: '{}'", name);
        
        if let Some(ext) = get_extension_from_filename(name) {
            println!("✓ Extension from filename: '{}'", ext);
            ext
        } else {
            println!("✗ No extension found in filename");
            
            if let Some(ct) = file.content_type() {
                let ct_str = ct.to_string();
                println!("Trying content type: '{}'", ct_str);
                
                if let Some(ext) = extension_from_content_type(&ct_str) {
                    println!("✓ Extension from content type: '{}'", ext);
                    ext
                } else if let Some(ext) = ct.extension() {
                    let ext_str = ext.as_str().to_lowercase();
                    println!("✓ Extension from CT extension(): '{}'", ext_str);
                    ext_str
                } else {
                    println!("✗ No extension from content type");
                    return Err(ApiError::bad_request(
                        format!("Cannot determine file type from filename '{}' or content type", name)
                    ));
                }
            } else {
                return Err(ApiError::bad_request(
                    format!("Cannot determine file type from filename '{}' (no content type available)", name)
                ));
            }
        }
    } else {
        println!("No filename provided in request");
        
        if let Some(ct) = file.content_type() {
            let ct_str = ct.to_string();
            println!("Trying content type: '{}'", ct_str);
            
            if let Some(ext) = extension_from_content_type(&ct_str) {
                println!("✓ Extension from content type: '{}'", ext);
                ext
            } else if let Some(ext) = ct.extension() {
                let ext_str = ext.as_str().to_lowercase();
                println!("✓ Extension from CT extension(): '{}'", ext_str);
                ext_str
            } else {
                println!("✗ No extension from content type");
                return Err(ApiError::bad_request(
                    "Cannot determine file type - no filename or recognizable content type"
                ));
            }
        } else {
            println!("✗ No content type available");
            return Err(ApiError::bad_request(
                "Cannot determine file type - no filename or content type provided"
            ));
        }
    };

    println!("Final extension: '{}'", extension);

    if !is_valid_image_extension(&extension) {
        println!("✗ Invalid extension '{}' for image", extension);
        return Err(ApiError::bad_request(
            format!("Only image files (JPEG, PNG, WebP) are allowed. Received: '{}'", extension)
        ));
    }
    
    println!("✓ Extension validated successfully");
    
    let upload_dir = "uploads/images";
    fs::create_dir_all(upload_dir)
        .await
        .map_err(|e| {
            println!("✗ Failed to create directory: {}", e);
            ApiError::internal_error(format!("Failed to create directory: {}", e))
        })?;
    
    let filename = format!(
        "{}_{}.{}",
        Uuid::new_v4(),
        chrono::Utc::now().timestamp(),
        extension
    );
    let filepath = format!("{}/{}", upload_dir, filename);
    
    println!("Saving to: {}", filepath);
    
    file.persist_to(&filepath)
        .await
        .map_err(|e| {
            println!("✗ Failed to save file: {}", e);
            ApiError::internal_error(format!("Failed to save file: {}", e))
        })?;
    
    let file_url = format!("/{}", filepath);
    
    println!("✓ File saved successfully!");
    println!("✓ File URL: {}", file_url);
    println!("========================================\n");
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "url": file_url,
        "filename": filename,
        "message": "Image uploaded successfully"
    }))))
}

#[openapi(tag = "File Upload")]
#[post("/upload/document", data = "<file>")]
pub async fn upload_document(
    mut file: TempFile<'_>,
    _auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    println!("\n========================================");
    println!("DOCUMENT UPLOAD REQUEST");
    println!("========================================");
    println!("File name: {:?}", file.name());
    println!("Content type: {:?}", file.content_type());
    println!("File length: {:?}", file.len());
    
    let extension = if let Some(name) = file.name() {
        println!("Trying to extract extension from filename: '{}'", name);
        
        if let Some(ext) = get_extension_from_filename(name) {
            println!("✓ Extension from filename: '{}'", ext);
            ext
        } else {
            println!("✗ No extension found in filename");
            
            if let Some(ct) = file.content_type() {
                let ct_str = ct.to_string();
                println!("Trying content type: '{}'", ct_str);
                
                if let Some(ext) = extension_from_content_type(&ct_str) {
                    println!("✓ Extension from content type: '{}'", ext);
                    ext
                } else if let Some(ext) = ct.extension() {
                    let ext_str = ext.as_str().to_lowercase();
                    println!("✓ Extension from CT extension(): '{}'", ext_str);
                    ext_str
                } else {
                    println!("✗ No extension from content type");
                    return Err(ApiError::bad_request(
                        format!("Cannot determine file type from filename '{}' or content type", name)
                    ));
                }
            } else {
                return Err(ApiError::bad_request(
                    format!("Cannot determine file type from filename '{}' (no content type available)", name)
                ));
            }
        }
    } else {
        println!("No filename provided in request");
        
        if let Some(ct) = file.content_type() {
            let ct_str = ct.to_string();
            println!("Trying content type: '{}'", ct_str);
            
            if let Some(ext) = extension_from_content_type(&ct_str) {
                println!("✓ Extension from content type: '{}'", ext);
                ext
            } else if let Some(ext) = ct.extension() {
                let ext_str = ext.as_str().to_lowercase();
                println!("✓ Extension from CT extension(): '{}'", ext_str);
                ext_str
            } else {
                println!("✗ No extension from content type");
                return Err(ApiError::bad_request(
                    "Cannot determine file type - no filename or recognizable content type"
                ));
            }
        } else {
            println!("✗ No content type available");
            return Err(ApiError::bad_request(
                "Cannot determine file type - no filename or content type provided"
            ));
        }
    };

    println!("Final extension: '{}'", extension);

    if !is_valid_document_extension(&extension) {
        println!("✗ Invalid extension '{}' for document", extension);
        return Err(ApiError::bad_request(
            format!("Only PDF, JPEG, and PNG files are allowed. Received: '{}'", extension)
        ));
    }
    
    println!("✓ Extension validated successfully");
    
    let upload_dir = "uploads/documents";
    fs::create_dir_all(upload_dir)
        .await
        .map_err(|e| {
            println!("✗ Failed to create directory: {}", e);
            ApiError::internal_error(format!("Failed to create directory: {}", e))
        })?;
    
    let filename = format!(
        "{}_{}.{}",
        Uuid::new_v4(),
        chrono::Utc::now().timestamp(),
        extension
    );
    let filepath = format!("{}/{}", upload_dir, filename);
    
    println!("Saving to: {}", filepath);
    
    file.persist_to(&filepath)
        .await
        .map_err(|e| {
            println!("✗ Failed to save file: {}", e);
            ApiError::internal_error(format!("Failed to save file: {}", e))
        })?;
    
    let file_url = format!("/{}", filepath);
    
    println!("✓ File saved successfully!");
    println!("✓ File URL: {}", file_url);
    println!("========================================\n");
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "url": file_url,
        "filename": filename,
        "message": "Document uploaded successfully"
    }))))
}

// ============================================================================
// BASE64 UPLOAD ENDPOINT
// ============================================================================

#[openapi(tag = "File Upload")]
#[post("/upload/document-base64", data = "<request>")]
pub async fn upload_document_base64(
    request: Json<Base64UploadRequest>,
    _auth: AuthGuard,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    println!("\n========================================");
    println!("BASE64 DOCUMENT UPLOAD REQUEST");
    println!("========================================");
    println!("Filename: {}", request.filename);
    println!("MIME type: {}", request.mime_type);
    println!("Base64 data length: {}", request.data.len());
    
    // Validate MIME type
    if !is_valid_document_mime(&request.mime_type) {
        println!("✗ Invalid MIME type");
        return Err(ApiError::bad_request(
            format!("Invalid MIME type: {}. Allowed: image/jpeg, image/png, application/pdf", request.mime_type)
        ));
    }
    
    println!("✓ MIME type validated");
    
    // Get extension from MIME type
    let extension = get_extension_from_mime(&request.mime_type)
        .ok_or_else(|| {
            println!("✗ Cannot determine extension from MIME type");
            ApiError::bad_request("Cannot determine file extension from MIME type")
        })?;
    
    println!("✓ Extension: {}", extension);
    
    // Decode base64 using data_encoding (already in your Cargo.toml)
    use data_encoding::BASE64;
    
    let file_data = BASE64.decode(request.data.as_bytes())
        .map_err(|e| {
            println!("✗ Failed to decode base64: {}", e);
            ApiError::bad_request("Invalid base64 data")
        })?;
    
    let file_size = file_data.len();
    println!("✓ Decoded {} bytes", file_size);
    
    // Validate file size (max 10MB)
    if file_size > 10 * 1024 * 1024 {
        println!("✗ File too large: {} bytes", file_size);
        return Err(ApiError::bad_request("File size exceeds 10MB limit"));
    }
    
    // Create uploads directory
    let upload_dir = "uploads/documents";
    fs::create_dir_all(upload_dir)
        .await
        .map_err(|e| {
            println!("✗ Failed to create directory: {}", e);
            ApiError::internal_error(format!("Failed to create directory: {}", e))
        })?;
    
    println!("✓ Directory ready");
    
    // Generate unique filename
    let filename = format!(
        "{}_{}.{}",
        Uuid::new_v4(),
        chrono::Utc::now().timestamp(),
        extension
    );
    let filepath = format!("{}/{}", upload_dir, filename);
    
    println!("Saving to: {}", filepath);
    
    // Write file
    fs::write(&filepath, &file_data)
        .await
        .map_err(|e| {
            println!("✗ Failed to write file: {}", e);
            ApiError::internal_error(format!("Failed to save file: {}", e))
        })?;
    
    let file_url = format!("/{}", filepath);
    
    println!("✓ File saved successfully!");
    println!("✓ File URL: {}", file_url);
    println!("========================================\n");
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "url": file_url,
        "filename": filename,
        "size": file_size,
        "message": "Document uploaded successfully"
    }))))
} 