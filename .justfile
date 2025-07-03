init_db:
    ./scripts/init_db.sh

@test_debug case:
    TEST_LOG=true cargo test {{ case }} | jq -R 'fromjson?'

@format:
    cargo clippy --fix --allow-dirty --all-targets
