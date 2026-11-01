use sqlx::{PgPool, Postgres, migrate::MigrateDatabase, postgres::PgPoolOptions};

const GUILDS: [(&str, &str); 3] = [
    ("guild-a", "Guild A"),
    ("guild-b", "Guild B"),
    ("guild-c", "Guild C"),
];

const ENEMIES: [&str; 3] = ["chunker", "crawler", "hopper"];

pub async fn init(url: &str) -> anyhow::Result<PgPool> {
    if !Postgres::database_exists(url).await? {
        Postgres::create_database(url).await?;
    }
    let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;
    sqlx::migrate!().run(&pool).await?;
    seed(&pool).await?;
    Ok(pool)
}

async fn seed(pool: &PgPool) -> anyhow::Result<()> {
    for (slug, name) in GUILDS {
        sqlx::query("INSERT INTO guild (slug, name) VALUES ($1, $2) ON CONFLICT (slug) DO NOTHING")
            .bind(slug)
            .bind(name)
            .execute(pool)
            .await?;
    }
    sqlx::query("INSERT INTO chat (slug, name) VALUES ('all', 'All') ON CONFLICT DO NOTHING")
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO chat (slug, name, guild_id) SELECT slug, name, id FROM guild ON CONFLICT DO NOTHING",
    )
    .execute(pool)
    .await?;
    occupy_desert(pool).await
}

async fn occupy_desert(pool: &PgPool) -> anyhow::Result<()> {
    let map: serde_json::Value = serde_json::from_slice(&std::fs::read(crate::MAP_FILE)?)?;
    for (r, row) in map["rows"].as_array().into_iter().flatten().enumerate().skip(6).take(5) {
        for (c, tile) in row.as_array().into_iter().flatten().enumerate().skip(39) {
            if tile["base"] != "sand" {
                continue;
            }
            let mut enemies = serde_json::Map::new();
            for k in 0..1 + (r * 7 + c * 5) % 3 {
                let kind = ENEMIES[(r * 5 + c * 11 + k * 7) % 3];
                let count = enemies.get(kind).and_then(|n| n.as_i64()).unwrap_or(0) + 1;
                enemies.insert(kind.into(), count.into());
            }
            sqlx::query(
                "INSERT INTO territory (hex_row, hex_col, enemies) VALUES ($1, $2, $3)
                 ON CONFLICT DO NOTHING",
            )
            .bind(r as i32)
            .bind(c as i32)
            .bind(serde_json::Value::Object(enemies))
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}
