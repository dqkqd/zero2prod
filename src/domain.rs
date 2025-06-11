use unicode_segmentation::UnicodeSegmentation;

pub struct NewSubscriber {
    pub email: String,
    pub name: SubscriberName,
}

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(s: String) -> Result<SubscriberName, String> {
        let is_empty_or_whitespace = s.trim().is_empty();
        let is_too_long = s.graphemes(true).count() > 256;

        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars().any(|g| forbidden_characters.contains(&g));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            Err(format!("{s} is not a valid subscriber name"))
        } else {
            Ok(SubscriberName(s))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use assertor::*;
    use rstest::rstest;

    use crate::domain::SubscriberName;

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "a".repeat(256);
        assertor::assert_that!(SubscriberName::parse(name)).is_ok();
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assertor::assert_that!(SubscriberName::parse(name)).is_err();
    }

    #[test]
    fn white_space_only_names_are_rejected() {
        let name = " ".to_string();
        assertor::assert_that!(SubscriberName::parse(name)).is_err();
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assertor::assert_that!(SubscriberName::parse(name)).is_err();
    }

    #[rstest]
    #[case("/")]
    #[case("(")]
    #[case(")")]
    #[case("\"")]
    #[case("<")]
    #[case(">")]
    #[case("\\")]
    #[case("{")]
    #[case("}")]
    fn names_containing_an_invalid_character_are_rejected(#[case] name: &str) {
        assertor::assert_that!(SubscriberName::parse(name.to_string())).is_err();
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assertor::assert_that!(SubscriberName::parse(name)).is_ok();
    }
}
