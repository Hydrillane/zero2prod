
pub struct IdempotencyKey(String);


impl TryFrom<String> for IdempotencyKey {
    type Error = anyhow::Error;
    fn try_from(s: String) -> Result<Self, Self::Error> {

        if s.is_empty() {
            anyhow::bail!("The idempotency key cannot be empty");
        }
        let max_len = 50;
        if s.len() > 50 {
            anyhow::bail!("The idempotency cannot be more than 50! ");
        };
        Ok(Self(s))
    }
}

impl From<IdempotencyKey> for String {
    fn from(value: IdempotencyKey) -> Self {
        value.0
    }
}

impl AsRef<str> for IdempotencyKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
