#[cfg(feature = "postgres")]
use sqlx::{Executor, PgPool};

#[cfg(feature = "postgres")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use hiddenshield_feedback_backend::database::{
        POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL, POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL,
    };

    let database_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| "missing disposable PostgreSQL URL")?;
    if !is_safe_http_gate_url(&database_url) {
        return Err(
            "refusing PostgreSQL HTTP schema action outside localhost/127.0.0.1 hiddenshield_http_gate database"
                .into(),
        );
    }
    let action = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "reset".to_string());
    let pool = PgPool::connect(&database_url).await?;
    match action.as_str() {
        "reset" => {
            execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL).await?;
            execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL).await?;
        }
        "up" => execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL).await?,
        "down" => execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL).await?,
        _ => return Err(format!("unsupported schema action: {action}").into()),
    }
    println!("postgres_http_schema:{action}:ok");
    Ok(())
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("postgres_http_schema requires --features postgres");
    std::process::exit(2);
}

#[cfg(feature = "postgres")]
fn is_safe_http_gate_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.contains("localhost") || lower.contains("127.0.0.1"))
        && lower.contains("hiddenshield_http_gate")
}

#[cfg(feature = "postgres")]
async fn execute_sql_batch(pool: &PgPool, sql: &str) -> Result<(), sqlx::Error> {
    for statement in sql
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        pool.execute(statement).await?;
    }
    Ok(())
}
