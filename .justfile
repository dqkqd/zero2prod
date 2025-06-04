init_db:
    ./scripts/init_db.sh

@test_debug case:
    TEST_LOG=true cargo test {{ case }} | jq -R 'fromjson?'
