use validator::ValidateEmail;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        if s.validate_email() {
            Ok(SubscriberEmail(s))
        } else {
            Err(format!("{s} is not a valid subscriber email"))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use assertor::*;

    use super::SubscriberEmail;

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assertor::assert_that!(SubscriberEmail::parse(email)).is_err();
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com".to_string();
        assertor::assert_that!(SubscriberEmail::parse(email)).is_err();
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assertor::assert_that!(SubscriberEmail::parse(email)).is_err();
    }
}
