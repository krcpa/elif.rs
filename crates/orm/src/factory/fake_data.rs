//! Realistic fake data generation for factories

use std::sync::OnceLock;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use chrono::{DateTime, Utc, Duration};
use serde_json::{json, Value};

/// Thread-local random number generator
thread_local! {
    static RNG: std::cell::RefCell<StdRng> = std::cell::RefCell::new(StdRng::from_entropy());
}

/// Initialize RNG with a specific seed for deterministic generation
pub fn seed_fake_data(seed: u64) {
    RNG.with(|rng| {
        *rng.borrow_mut() = StdRng::seed_from_u64(seed);
    });
}

/// Generate a random number within a range
pub fn random_range(min: i32, max: i32) -> i32 {
    RNG.with(|rng| rng.borrow_mut().gen_range(min..=max))
}

/// Generate a random boolean with optional probability
pub fn random_bool(probability: Option<f64>) -> bool {
    let prob = probability.unwrap_or(0.5);
    RNG.with(|rng| rng.borrow_mut().gen_bool(prob))
}

/// Generate a random UUID string
pub fn random_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Generate a random email address
pub fn fake_email() -> String {
    let names = ["alice", "bob", "charlie", "diana", "eve", "frank", "grace", "henry", "ivy", "jack"];
    let domains = ["example.com", "test.org", "demo.net", "sample.io", "fake.dev"];
    
    let name = RNG.with(|rng| names.choose(&mut *rng.borrow_mut()).unwrap());
    let domain = RNG.with(|rng| domains.choose(&mut *rng.borrow_mut()).unwrap());
    let number = random_range(1, 999);
    
    format!("{}{:03}@{}", name, number, domain)
}

/// Generate a fake first name
pub fn fake_first_name() -> String {
    let names = [
        "Alice", "Bob", "Charlie", "Diana", "Eve", "Frank", "Grace", "Henry", "Ivy", "Jack",
        "Kate", "Liam", "Mia", "Noah", "Olivia", "Peter", "Quinn", "Ruby", "Sam", "Tina",
        "Uma", "Victor", "Willow", "Xander", "Yara", "Zoe", "Aaron", "Bella", "Connor", "Delia",
    ];
    
    RNG.with(|rng| names.choose(&mut *rng.borrow_mut()).unwrap().to_string())
}

/// Generate a fake last name
pub fn fake_last_name() -> String {
    let names = [
        "Anderson", "Brown", "Davis", "Evans", "Fisher", "Garcia", "Harris", "Johnson", "King", "Lopez",
        "Miller", "Nelson", "Oliveira", "Parker", "Quinn", "Roberts", "Smith", "Taylor", "Underwood", "Valdez",
        "Williams", "Xavier", "Young", "Zhang", "Adams", "Bell", "Clark", "Duncan", "Edwards", "Ford",
    ];
    
    RNG.with(|rng| names.choose(&mut *rng.borrow_mut()).unwrap().to_string())
}

/// Generate a fake full name
pub fn fake_name() -> String {
    format!("{} {}", fake_first_name(), fake_last_name())
}

/// Generate a fake company name
pub fn fake_company() -> String {
    let prefixes = ["Acme", "Global", "United", "Premium", "Elite", "Advanced", "Dynamic", "Smart"];
    let suffixes = ["Corp", "Inc", "LLC", "Solutions", "Systems", "Technologies", "Enterprises", "Group"];
    
    let prefix = RNG.with(|rng| prefixes.choose(&mut *rng.borrow_mut()).unwrap());
    let suffix = RNG.with(|rng| suffixes.choose(&mut *rng.borrow_mut()).unwrap());
    
    format!("{} {}", prefix, suffix)
}

/// Generate a fake phone number
pub fn fake_phone() -> String {
    let area_code = random_range(200, 999);
    let exchange = random_range(200, 999);
    let number = random_range(1000, 9999);
    
    format!("({}) {}-{}", area_code, exchange, number)
}

/// Generate a fake address
pub fn fake_address() -> String {
    let street_numbers = random_range(1, 9999);
    let streets = [
        "Main St", "Oak Ave", "Elm Dr", "Park Blvd", "Cedar Ln", "Maple Way", "Pine St", "River Rd",
        "Hill Ave", "Lake Dr", "Forest Ln", "Garden St", "Valley Rd", "Spring Ave", "Sunset Blvd",
    ];
    
    let street = RNG.with(|rng| streets.choose(&mut *rng.borrow_mut()).unwrap());
    
    format!("{} {}", street_numbers, street)
}

/// Generate a fake city
pub fn fake_city() -> String {
    let cities = [
        "Springfield", "Riverside", "Franklin", "Georgetown", "Fairview", "Madison", "Arlington", "Salem",
        "Richmond", "Columbia", "Austin", "Denver", "Phoenix", "Portland", "Seattle", "Boston",
    ];
    
    RNG.with(|rng| cities.choose(&mut *rng.borrow_mut()).unwrap().to_string())
}

/// Generate a fake state/province
pub fn fake_state() -> String {
    let states = [
        "California", "Texas", "Florida", "New York", "Pennsylvania", "Illinois", "Ohio", "Georgia",
        "North Carolina", "Michigan", "New Jersey", "Virginia", "Washington", "Arizona", "Massachusetts",
    ];
    
    RNG.with(|rng| states.choose(&mut *rng.borrow_mut()).unwrap().to_string())
}

/// Generate a fake postal code
pub fn fake_postal_code() -> String {
    format!("{:05}", random_range(10000, 99999))
}

/// Generate a fake country
pub fn fake_country() -> String {
    let countries = [
        "United States", "Canada", "United Kingdom", "Germany", "France", "Italy", "Spain", "Netherlands",
        "Australia", "Japan", "South Korea", "Brazil", "Mexico", "India", "China", "Russia",
    ];
    
    RNG.with(|rng| countries.choose(&mut *rng.borrow_mut()).unwrap().to_string())
}

/// Generate a fake sentence
pub fn fake_sentence() -> String {
    let subjects = ["The user", "The system", "The application", "The service", "The platform"];
    let verbs = ["creates", "updates", "processes", "manages", "handles", "provides"];
    let objects = ["data", "information", "content", "resources", "functionality", "capabilities"];
    
    let subject = RNG.with(|rng| subjects.choose(&mut *rng.borrow_mut()).unwrap());
    let verb = RNG.with(|rng| verbs.choose(&mut *rng.borrow_mut()).unwrap());
    let object = RNG.with(|rng| objects.choose(&mut *rng.borrow_mut()).unwrap());
    
    format!("{} {} {}.", subject, verb, object)
}

/// Generate fake paragraph text
pub fn fake_paragraph() -> String {
    let sentence_count = random_range(3, 7);
    let sentences: Vec<String> = (0..sentence_count).map(|_| fake_sentence()).collect();
    sentences.join(" ")
}

/// Generate a fake URL
pub fn fake_url() -> String {
    let protocols = ["https", "http"];
    let subdomains = ["www", "api", "app", "admin", "portal"];
    let domains = ["example.com", "test.org", "demo.net", "sample.io", "fake.dev"];
    let paths = ["/", "/home", "/dashboard", "/profile", "/settings", "/api/v1"];
    
    let protocol = RNG.with(|rng| protocols.choose(&mut *rng.borrow_mut()).unwrap());
    let subdomain = RNG.with(|rng| subdomains.choose(&mut *rng.borrow_mut()).unwrap());
    let domain = RNG.with(|rng| domains.choose(&mut *rng.borrow_mut()).unwrap());
    let path = RNG.with(|rng| paths.choose(&mut *rng.borrow_mut()).unwrap());
    
    format!("{}://{}.{}{}", protocol, subdomain, domain, path)
}

/// Generate a fake username
pub fn fake_username() -> String {
    let adjectives = ["cool", "super", "awesome", "great", "amazing", "fantastic"];
    let nouns = ["user", "coder", "dev", "ninja", "master", "guru"];
    
    let adjective = RNG.with(|rng| adjectives.choose(&mut *rng.borrow_mut()).unwrap());
    let noun = RNG.with(|rng| nouns.choose(&mut *rng.borrow_mut()).unwrap());
    let number = random_range(1, 999);
    
    format!("{}{}{}", adjective, noun, number)
}

/// Generate a fake password hash (NOT for actual use)
pub fn fake_password_hash() -> String {
    let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789$./";
    let mut hash = "$2b$12$".to_string();
    
    for _ in 0..53 {
        let char_index = random_range(0, chars.len() as i32 - 1) as usize;
        hash.push(chars.chars().nth(char_index).unwrap());
    }
    
    hash
}

/// Generate a fake datetime within the last year
pub fn fake_datetime() -> DateTime<Utc> {
    let now = Utc::now();
    let days_ago = random_range(0, 365);
    let hours_ago = random_range(0, 24);
    let minutes_ago = random_range(0, 60);
    
    now - Duration::days(days_ago as i64) 
        - Duration::hours(hours_ago as i64) 
        - Duration::minutes(minutes_ago as i64)
}

/// Generate a fake datetime in the future (next year)
pub fn fake_future_datetime() -> DateTime<Utc> {
    let now = Utc::now();
    let days_ahead = random_range(1, 365);
    let hours_ahead = random_range(0, 24);
    let minutes_ahead = random_range(0, 60);
    
    now + Duration::days(days_ahead as i64) 
        + Duration::hours(hours_ahead as i64) 
        + Duration::minutes(minutes_ahead as i64)
}

/// Generate a fake price (in cents to avoid floating point issues)
pub fn fake_price_cents() -> i32 {
    random_range(99, 99999) // $0.99 to $999.99
}

/// Generate a fake rating (1-5)
pub fn fake_rating() -> i32 {
    random_range(1, 5)
}

/// Generate a fake status from common options
pub fn fake_status() -> String {
    let statuses = ["active", "inactive", "pending", "suspended", "verified", "draft", "published"];
    RNG.with(|rng| statuses.choose(&mut *rng.borrow_mut()).unwrap().to_string())
}

/// Generate fake JSON data with common fields
pub fn fake_json_object() -> Value {
    json!({
        "id": random_uuid(),
        "name": fake_name(),
        "email": fake_email(),
        "created_at": fake_datetime().to_rfc3339(),
        "status": fake_status(),
        "metadata": {
            "source": "factory",
            "version": "1.0"
        }
    })
}

/// Faker trait for easy data generation
pub trait Faker {
    fn fake_email() -> String { fake_email() }
    fn fake_name() -> String { fake_name() }
    fn fake_first_name() -> String { fake_first_name() }
    fn fake_last_name() -> String { fake_last_name() }
    fn fake_company() -> String { fake_company() }
    fn fake_phone() -> String { fake_phone() }
    fn fake_address() -> String { fake_address() }
    fn fake_city() -> String { fake_city() }
    fn fake_state() -> String { fake_state() }
    fn fake_postal_code() -> String { fake_postal_code() }
    fn fake_country() -> String { fake_country() }
    fn fake_sentence() -> String { fake_sentence() }
    fn fake_paragraph() -> String { fake_paragraph() }
    fn fake_url() -> String { fake_url() }
    fn fake_username() -> String { fake_username() }
    fn fake_password_hash() -> String { fake_password_hash() }
    fn fake_datetime() -> DateTime<Utc> { fake_datetime() }
    fn fake_future_datetime() -> DateTime<Utc> { fake_future_datetime() }
    fn fake_price_cents() -> i32 { fake_price_cents() }
    fn fake_rating() -> i32 { fake_rating() }
    fn fake_status() -> String { fake_status() }
    fn fake_uuid() -> String { random_uuid() }
    fn random_bool(probability: Option<f64>) -> bool { random_bool(probability) }
    fn random_range(min: i32, max: i32) -> i32 { random_range(min, max) }
}

/// Empty struct that implements Faker for convenience
pub struct Fake;
impl Faker for Fake {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_generation() {
        seed_fake_data(12345);
        let email1 = fake_email();
        let name1 = fake_name();
        
        seed_fake_data(12345);
        let email2 = fake_email();
        let name2 = fake_name();
        
        assert_eq!(email1, email2);
        assert_eq!(name1, name2);
    }
    
    #[test]
    fn test_email_generation() {
        for _ in 0..100 {
            let email = fake_email();
            assert!(email.contains('@'));
            assert!(email.contains('.'));
        }
    }
    
    #[test]
    fn test_name_generation() {
        for _ in 0..100 {
            let name = fake_name();
            assert!(name.contains(' ')); // Should have first and last name
            assert!(name.len() > 3); // Should be reasonably long
        }
    }
    
    #[test]
    fn test_phone_generation() {
        for _ in 0..100 {
            let phone = fake_phone();
            assert!(phone.starts_with('('));
            assert!(phone.contains(')'));
            assert!(phone.contains('-'));
        }
    }
    
    #[test]
    fn test_address_generation() {
        for _ in 0..100 {
            let address = fake_address();
            assert!(address.len() > 5); // Should be reasonably long
            assert!(address.chars().next().unwrap().is_ascii_digit()); // Should start with number
        }
    }
    
    #[test]
    fn test_datetime_generation() {
        let now = Utc::now();
        
        for _ in 0..100 {
            let past_dt = fake_datetime();
            let future_dt = fake_future_datetime();
            
            assert!(past_dt <= now);
            assert!(future_dt > now);
        }
    }
    
    #[test]
    fn test_price_generation() {
        for _ in 0..100 {
            let price = fake_price_cents();
            assert!(price >= 99);
            assert!(price <= 99999);
        }
    }
    
    #[test]
    fn test_rating_generation() {
        for _ in 0..100 {
            let rating = fake_rating();
            assert!(rating >= 1);
            assert!(rating <= 5);
        }
    }
    
    #[test]
    fn test_faker_trait() {
        let _email = Fake::fake_email();
        let _name = Fake::fake_name();
        let _bool = Fake::random_bool(Some(0.7));
        let _range = Fake::random_range(1, 10);
        
        // Test passes if no compilation errors
        assert!(true);
    }
    
    #[test]
    fn test_json_generation() {
        let obj = fake_json_object();
        
        assert!(obj.is_object());
        assert!(obj["id"].is_string());
        assert!(obj["name"].is_string());
        assert!(obj["email"].is_string());
        assert!(obj["created_at"].is_string());
        assert!(obj["status"].is_string());
        assert!(obj["metadata"].is_object());
    }
}