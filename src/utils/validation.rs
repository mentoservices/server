use regex::Regex;

pub fn validate_mobile(mobile: &str) -> bool {
    let re = Regex::new(r"^[6-9]\d{9}$").unwrap();
    re.is_match(mobile)
}

pub fn validate_email(email: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    re.is_match(email)
}

pub fn validate_pincode(pincode: &str) -> bool {
    let re = Regex::new(r"^\d{6}$").unwrap();
    re.is_match(pincode)
}

pub fn generate_otp() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let otp: u32 = rng.gen_range(100000..999999);
    otp.to_string()
}