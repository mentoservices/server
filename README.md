# Mento Services - Rust Rocket Backend

A complete backend API for Mento Services platform built with Rust and Rocket framework.

## Features

✅ **OTP-based Authentication** (Email-based, SMS-ready)
✅ **JWT Token Authentication** (Access & Refresh tokens)
✅ **User Profile Management**
✅ **KYC Verification System**
✅ **Worker Profiles** with subscriptions (Silver/Gold plans)
✅ **Job Board** with posting and applications
✅ **Category & Subcategory Management**
✅ **Review & Rating System**
✅ **File Upload** (Local storage for images & documents)
✅ **Swagger/OpenAPI Documentation**
✅ **Pagination** on all GET routes
✅ **MongoDB** database

## Tech Stack

- **Rust** (Edition 2021)
- **Rocket** 0.5.0 (Web framework)
- **MongoDB** (Database)
- **JWT** (Authentication)
- **Lettre** (Email service)
- **Swagger/OpenAPI** (API documentation)

## Project Structure

```
src/
├── main.rs              # Application entry point
├── config/
│   └── mod.rs          # Configuration management
├── db/
│   └── mod.rs          # Database connection
├── models/
│   ├── mod.rs
│   ├── user.rs         # User model
│   ├── otp.rs          # OTP model
│   ├── kyc.rs          # KYC model
│   ├── worker.rs       # Worker profile model
│   ├── job.rs          # Job model
│   ├── category.rs     # Category models
│   ├── subscription.rs # Subscription model
│   └── review.rs       # Review model
├── routes/
│   ├── mod.rs
│   ├── auth.rs         # Authentication routes
│   ├── user.rs         # User management routes
│   ├── kyc.rs          # KYC routes
│   ├── worker.rs       # Worker routes
│   ├── job.rs          # Job routes
│   ├── category.rs     # Category routes
│   ├── file_upload.rs  # File upload routes
│   └── review.rs       # Review routes
├── services/
│   ├── mod.rs
│   ├── email.rs        # Email service
│   └── jwt.rs          # JWT token service
├── guards/
│   ├── mod.rs
│   ├── auth.rs         # JWT authentication guard
│   └── kyc.rs          # KYC verification guard
└── utils/
    ├── mod.rs
    ├── validation.rs   # Input validation
    └── response.rs     # API response helpers
```

## Installation

### Prerequisites

- Rust 1.70+ (Install from https://rustup.rs/)
- MongoDB 4.4+
- SMTP server (Gmail/SendGrid/etc.) for emails

### Steps

1. **Clone the repository**
```bash
git clone <repo-url>
cd mento-services
```

2. **Configure environment variables**
```bash
cp .env.example .env
# Edit .env with your configurations
```

3. **Install dependencies**
```bash
cargo build
```

4. **Create required directories**
```bash
mkdir -p uploads/images
mkdir -p uploads/documents
mkdir -p uploads/profiles
```

5. **Run the application**
```bash
cargo run
```

The server will start at `http://localhost:3000`

## Environment Variables

```env
MONGODB_URI=mongodb://localhost:27017/mento-services
JWT_SECRET=your-super-secret-jwt-key
JWT_REFRESH_SECRET=your-super-secret-refresh-key
JWT_EXPIRY=900
JWT_REFRESH_EXPIRY=604800

# Email Configuration
MAIL_HOST=smtp.gmail.com
MAIL_PORT=587
MAIL_USER=your-email@gmail.com
MAIL_PASSWORD=your-app-password
MAIL_FROM=Mento Services <noreply@mentoservices.com>

# Payment Gateway (future use)
RAZORPAY_KEY_ID=your-razorpay-key-id
RAZORPAY_KEY_SECRET=your-razorpay-key-secret

ROCKET_ADDRESS=0.0.0.0
ROCKET_PORT=3000
```

## API Documentation

Once the server is running, access the interactive Swagger documentation at:

**http://localhost:3000/api/docs**

## API Endpoints

### Authentication
- `POST /api/v1/auth/send-otp` - Send OTP to email
- `POST /api/v1/auth/resend-otp` - Resend OTP
- `POST /api/v1/auth/verify-otp` - Verify OTP and login
- `POST /api/v1/auth/refresh` - Refresh access token

### User Management
- `GET /api/v1/user/profile` - Get user profile
- `PUT /api/v1/user/profile` - Update profile
- `POST /api/v1/user/upload-photo` - Upload profile photo
- `PUT /api/v1/user/fcm-token` - Update FCM token
- `DELETE /api/v1/user/account` - Delete account

### KYC
- `POST /api/v1/kyc/submit` - Submit KYC documents
- `GET /api/v1/kyc/status` - Get KYC status
- `GET /api/v1/kyc/admin/submissions` - Get all KYC submissions (paginated)
- `GET /api/v1/kyc/admin/:id` - Get KYC by ID
- `PUT /api/v1/kyc/admin/:id/status` - Update KYC status

### Worker
- `POST /api/v1/worker/profile` - Create worker profile
- `GET /api/v1/worker/profile` - Get worker profile
- `PUT /api/v1/worker/profile` - Update worker profile
- `DELETE /api/v1/worker/profile` - Delete worker profile
- `GET /api/v1/worker/search` - Search workers (paginated)
- `GET /api/v1/worker/:id` - Get worker by ID
- `POST /api/v1/worker/subscription` - Update subscription
- `GET /api/v1/worker/admin/stats` - Get worker statistics (paginated)

### Jobs
- `POST /api/v1/job/create` - Create job posting
- `GET /api/v1/job/search` - Search jobs (paginated)
- `GET /api/v1/job/:id` - Get job details
- `GET /api/v1/job/my/posted` - Get my posted jobs (paginated)
- `POST /api/v1/job/:id/apply` - Apply to job
- `PUT /api/v1/job/:id/status` - Update job status
- `DELETE /api/v1/job/:id` - Delete job

### Categories
- `GET /api/v1/category/all` - Get all categories with subcategories
- `GET /api/v1/category/:id/subcategories` - Get subcategories

### File Upload
- `POST /api/v1/upload/image` - Upload image
- `POST /api/v1/upload/document` - Upload document

### Reviews
- `POST /api/v1/review/create` - Create review
- `GET /api/v1/review/worker/:id` - Get worker reviews (paginated)
- `DELETE /api/v1/review/:id` - Delete review

## Pagination

All GET endpoints that return lists support pagination with query parameters:
- `page` - Page number (default: 1)
- `limit` - Items per page (default: 20, max: 100)

Example:
```
GET /api/v1/job/search?page=2&limit=10
```

Response includes pagination metadata:
```json
{
  "success": true,
  "data": {
    "items": [...],
    "pagination": {
      "page": 2,
      "limit": 10,
      "total": 45,
      "pages": 5
    }
  }
}
```

## File Storage

Files are stored locally in the `uploads/` directory:
- `uploads/images/` - User profile photos and general images
- `uploads/documents/` - KYC documents and PDFs
- `uploads/profiles/` - Profile photos

Access uploaded files via: `http://localhost:3000/uploads/...`

## Development

### Running in development mode
```bash
ROCKET_ENV=development cargo run
```

### Building for production
```bash
cargo build --release
```

### Running tests
```bash
cargo test
```

## Switching from Email to SMS (MSG91)

The codebase is ready for SMS integration. To switch:

1. Uncomment MSG91 service in `src/common/services/email.rs`
2. Update `.env` with MSG91 credentials
3. Update `src/routes/auth.rs` to use `Msg91Service` instead of `EmailService`

## Security Notes

- Always use HTTPS in production
- Store JWT secrets securely (use environment variables)
- Implement rate limiting for OTP endpoints
- Add admin authentication guards for admin endpoints
- Validate and sanitize all user inputs
- Implement proper CORS policies

## License

PRIVATELY OWNED