use lettre::{
    Message, SmtpTransport, Transport,
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
};
use log::{info, error, warn};

pub struct EmailService;

impl EmailService {
    pub async fn send_otp_email(email: &str, otp: &str, mobile: &str) -> bool {
        match Self::try_send_otp(email, otp, mobile).await {
            Ok(_) => {
                info!("OTP email sent successfully to {}", email);
                true
            }
            Err(e) => {
                error!("Failed to send OTP email to {}: {}", email, e);
                false
            }
        }
    }

    async fn try_send_otp(email: &str, otp: &str, mobile: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mail_user = crate::config::Config::mail_user();
        let mail_password = crate::config::Config::mail_password();
        
        if mail_user.is_empty() || mail_password.is_empty() {
            warn!("Email credentials not configured. Skipping email send.");
            return Err("Email not configured".into());
        }

        let from_mailbox: Mailbox = crate::config::Config::mail_from().parse()?;
        let to_mailbox: Mailbox = email.parse()?;

        let email_body = format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <style>
                    body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
                    .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
                    .header {{ background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); 
                              color: white; padding: 30px; text-align: center; border-radius: 10px 10px 0 0; }}
                    .content {{ background: #f9f9f9; padding: 30px; border-radius: 0 0 10px 10px; }}
                    .otp-box {{ background: white; border: 2px dashed #667eea; border-radius: 8px; 
                               padding: 20px; text-align: center; margin: 20px 0; }}
                    .otp-code {{ font-size: 32px; font-weight: bold; letter-spacing: 5px; color: #667eea; }}
                    .footer {{ text-align: center; margin-top: 20px; color: #666; font-size: 12px; }}
                    .warning {{ background: #fff3cd; border-left: 4px solid #ffc107; padding: 10px; margin: 20px 0; }}
                </style>
            </head>
            <body>
                <div class="container">
                    <div class="header">
                        <h1>🔐 Mento Services</h1>
                        <p>Your One-Time Password</p>
                    </div>
                    <div class="content">
                        <p>Hello,</p>
                        <p>You requested an OTP to login to Mento Services for mobile number <strong>{}</strong>.</p>
                        
                        <div class="otp-box">
                            <p style="margin: 0; color: #666;">Your OTP Code is:</p>
                            <div class="otp-code">{}</div>
                            <p style="margin: 10px 0 0 0; color: #666; font-size: 14px;">Valid for 10 minutes</p>
                        </div>
                        
                        <div class="warning">
                            <strong>⚠️ Security Note:</strong> Never share this OTP with anyone.
                        </div>
                        
                        <p>If you didn't request this OTP, please ignore this email.</p>
                        
                        <p>Best regards,<br><strong>Mento Services Team</strong></p>
                    </div>
                    <div class="footer">
                        <p>© 2025 Mento Services. All rights reserved.</p>
                    </div>
                </div>
            </body>
            </html>
            "#,
            mobile, otp
        );

        let email_message = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject("Your Mento Services OTP Code")
            .header(ContentType::TEXT_HTML)
            .body(email_body)?;

        let creds = Credentials::new(mail_user, mail_password);
        let mailer = SmtpTransport::relay(&crate::config::Config::mail_host())?
            .credentials(creds)
            .build();

        mailer.send(&email_message)?;
        Ok(())
    }

    pub async fn send_welcome_email(email: &str, name: &str) -> bool {
        match Self::try_send_welcome(email, name).await {
            Ok(_) => {
                info!("Welcome email sent to {}", email);
                true
            }
            Err(e) => {
                error!("Failed to send welcome email: {}", e);
                false
            }
        }
    }

    async fn try_send_welcome(email: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mail_user = crate::config::Config::mail_user();
        let mail_password = crate::config::Config::mail_password();
        
        if mail_user.is_empty() || mail_password.is_empty() {
            return Err("Email not configured".into());
        }

        let display_name = if name.is_empty() { "there" } else { name };
        
        let from_mailbox: Mailbox = crate::config::Config::mail_from().parse()?;
        let to_mailbox: Mailbox = email.parse()?;

        let email_body = format!(
            r#"
            <!DOCTYPE html>
            <html>
            <body>
                <h1>Welcome to Mento Services! 🎉</h1>
                <p>Hi {},</p>
                <p>Welcome aboard! Complete your profile and KYC to get started.</p>
                <p>With Mento Services, you can:</p>
                <ul>
                    <li>Find skilled workers for home services</li>
                    <li>Browse and apply for local jobs</li>
                    <li>Offer your services as a worker</li>
                    <li>Connect with customers in your area</li>
                </ul>
                <p>Best regards,<br><strong>Mento Services Team</strong></p>
            </body>
            </html>
            "#,
            display_name
        );

        let email_message = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject("Welcome to Mento Services! 🎉")
            .header(ContentType::TEXT_HTML)
            .body(email_body)?;

        let creds = Credentials::new(mail_user, mail_password);
        let mailer = SmtpTransport::relay(&crate::config::Config::mail_host())?
            .credentials(creds)
            .build();

        mailer.send(&email_message)?;
        Ok(())
    }
}