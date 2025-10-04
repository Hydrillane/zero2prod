use validator::ValidateEmail;



#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s:String) -> Result<SubscriberEmail,String> {
        let email = Self(s.clone());
        if email.validate_email() {
            Ok(email)
        } else {
            Err(format!("{} is not a valid subscriber email!", s))
        }
    }

}

impl ValidateEmail for SubscriberEmail {
    fn as_email_string(&self) -> Option<std::borrow::Cow<str>> {
        Some(self.0.as_str().into())
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}



#[cfg(test)]
mod tests {

    use super::SubscriberEmail;
    use claim::assert_err;
    use fake::rand::SeedableRng;
    use fake::{faker::internet::en::SafeEmail, rand::rngs::StdRng};
    use fake::Fake;
    use quickcheck::Arbitrary;

    #[derive(Debug,Clone)]
    struct ValidEmailFixture(String);

    impl Arbitrary for ValidEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut seed = StdRng::seed_from_u64(u64::arbitrary(g));
            let email = SafeEmail().fake_with_rng(&mut seed);
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_email_is_parsed_successfully(valid_email:ValidEmailFixture) -> bool {
        dbg!(&valid_email.0);
        SubscriberEmail::parse(valid_email.0).is_ok()
    }

    #[test]
    fn empty_string_is_rejected() {

        let is_empty = "".to_string();
        assert_err!(SubscriberEmail::parse(is_empty));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com".to_string();
        assert_err!(SubscriberEmail::parse(email), "doesnt parsed ");
    }

    #[test]
    fn email_missing_subject_is_rejected(){
        let email = "@domain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

}



