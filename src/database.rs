use sqlx::{PgPool, Postgres, migrate::MigrateDatabase, postgres::PgPoolOptions};

const GUILDS: [(&str, &str); 3] = [
    ("guild-a", "Guild A"),
    ("guild-b", "Guild B"),
    ("guild-c", "Guild C"),
];

pub async fn init(url: &str) -> anyhow::Result<PgPool> {
    if !Postgres::database_exists(url).await? {
        Postgres::create_database(url).await?;
    }
    let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;
    sqlx::migrate!().run(&pool).await?;
    seed(&pool).await?;
    Ok(pool)
}

async fn seed(pool: &PgPool) -> sqlx::Result<()> {
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
    Ok(())
}
