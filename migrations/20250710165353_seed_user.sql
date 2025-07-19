-- Add migration script here
INSERT INTO users (user_id, username, password_hash)
VALUES (
    'e72e3140-0519-4d5e-b691-9aa778b1a6d9',
    'admin',
    '$argon2id$v=19$m=15000,t=2,p=1$wG0tsH/jej8UwL2OhnN0UA$/mIlTssOQ184sBTg+OcHPXHtAHQqAdHwImoRjNAk5Po'
)
