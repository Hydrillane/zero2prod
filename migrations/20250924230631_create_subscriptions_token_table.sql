-- Add migration script here
CREATE TABLE subscription_tokens (
    subscription_tokens TEXT NOT NULL,
    subscriber_id uuid NOT NULL
	REFERENCES subscriptions (id),
    PRIMARY KEY (subscription_tokens)
);
