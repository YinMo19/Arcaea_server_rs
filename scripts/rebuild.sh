mysql -e "DROP DATABASE arcaea_core;"
sqlx database create && sqlx migrate run
cargo run --bin init_db
