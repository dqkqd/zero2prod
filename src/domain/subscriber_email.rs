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
    use fake::locales::Data;
    use quickcheck::Arbitrary;
    use quickcheck_macros::quickcheck;

    use super::SubscriberEmail;

    #[derive(Debug, Clone)]
    struct ValitEmailFixture(String);

    impl Arbitrary for ValitEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            // https://github.com/BurntSushi/quickcheck/issues/320#issuecomment-2085231712
            // https://github.com/cksac/fake-rs/blob/2ab2ed66bf84f77cf0fbef7524f2c81e4066b6d5/fake/src/faker/impls/internet.rs#L46-L52
            let username = g.choose(fake::locales::EN::NAME_FIRST_NAME).unwrap();
            let domain = g.choose(&["com", "net", "org"]).unwrap();
            let email = format!("{username}@example.{domain}");
            ValitEmailFixture(email)
        }
    }

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

    #[quickcheck]
    fn valid_emails_are_parsed_successfully(valid_email: ValitEmailFixture) -> bool {
        SubscriberEmail::parse(valid_email.0).is_ok()
    }
}
