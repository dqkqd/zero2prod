-- Add migration script here
ALTER TABLE subscriptions ADD COLUMN status text NULL;
