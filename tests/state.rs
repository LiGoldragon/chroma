//! Persistence boundary regression coverage.

use chroma::{ReadStoredLocation, StateStore, StoredLocation};
use kameo::actor::Spawn;
use redb::{Database, TableDefinition};
use tempfile::tempdir;

const LEGACY_LOCATION_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("location");
const LAST_KNOWN_KEY: &str = "current";

#[tokio::test]
async fn unqualified_legacy_location_record_is_not_an_authoritative_solar_location() {
    let directory = tempdir().expect("temporary state directory");
    let path = directory.path().join("state.redb");
    let legacy_location = StoredLocation { latitude: 1.0, longitude: 1.0 };
    let database = Database::create(&path).expect("create legacy state database");
    let transaction = database.begin_write().expect("begin legacy location write");
    {
        let mut table = transaction.open_table(LEGACY_LOCATION_TABLE).expect("open legacy location table");
        table
            .insert(LAST_KNOWN_KEY, legacy_location.archive().expect("archive legacy location").as_slice())
            .expect("write legacy location");
    }
    transaction.commit().expect("commit legacy location");
    drop(database);

    let store = StateStore::spawn(StateStore::open(path).expect("open current state store"));
    let stored = store.ask(ReadStoredLocation).await.expect("state read succeeds");

    assert_eq!(stored, None, "an unqualified legacy location cannot select a solar schedule");
}
