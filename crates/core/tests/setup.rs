use ctor::ctor;
use tracing_subscriber::{filter, FmtSubscriber};

#[ctor]
fn setup() {
    let filter = filter::EnvFilter::new("info");
    let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting tracing default failed");
}
