use idb::DatabaseEvent;
use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::JsValue;

const DB_NAME: &str = "realestate_cache";
const DB_VERSION: u32 = 1;

/// Opens (or creates) the `IndexedDB` database, ensuring the given object store exists.
#[allow(clippy::future_not_send)]
async fn open_db(store_name: &str) -> Result<idb::Database, JsValue> {
    let factory =
        idb::Factory::new().map_err(|e| JsValue::from_str(&format!("IDB factory error: {e:?}")))?;

    let store_name_owned = store_name.to_owned();
    let mut open_request = factory
        .open(DB_NAME, Some(DB_VERSION))
        .map_err(|e| JsValue::from_str(&format!("IDB open error: {e:?}")))?;

    open_request.on_upgrade_needed(move |event| {
        let db = event.database().unwrap_or_else(|e| {
            web_sys::console::error_1(&format!("IDB upgrade error: {e:?}").into());
            // This callback cannot propagate errors; the browser will abort the transaction.
            unreachable!()
        });

        if !db.store_names().contains(&store_name_owned) {
            let params = idb::ObjectStoreParams::new();
            let _store = db.create_object_store(&store_name_owned, params);
        }
    });

    open_request
        .await
        .map_err(|e| JsValue::from_str(&format!("IDB open await error: {e:?}")))
}

/// Reads a cached list from `IndexedDB`.
///
/// Returns `Some(vec)` if the key exists and deserializes successfully, `None` otherwise.
#[allow(clippy::future_not_send)]
pub async fn read_list<T: DeserializeOwned>(store: &str, key: &str) -> Option<Vec<T>> {
    let db = open_db(store).await.ok()?;

    let transaction = db
        .transaction(&[store], idb::TransactionMode::ReadOnly)
        .ok()?;

    let object_store = transaction.object_store(store).ok()?;

    let js_key = JsValue::from_str(key);
    let result = object_store.get(js_key).ok()?.await.ok()?;

    let js_value = result?;
    let items: Vec<T> = serde_wasm_bindgen::from_value(js_value).ok()?;

    Some(items)
}

/// Writes a list to `IndexedDB`, overwriting any previous value for the given key.
#[allow(clippy::future_not_send)]
pub async fn write_list<T: Serialize>(store: &str, key: &str, value: &[T]) {
    let Ok(db) = open_db(store).await else {
        return;
    };

    let Ok(transaction) = db.transaction(&[store], idb::TransactionMode::ReadWrite) else {
        return;
    };

    let Ok(object_store) = transaction.object_store(store) else {
        return;
    };

    let Ok(js_value) = serde_wasm_bindgen::to_value(value) else {
        return;
    };

    let js_key = JsValue::from_str(key);
    // Use put with an explicit key to store the serialized list under the string key.
    if let Ok(put_request) = object_store.put(&js_value, Some(&js_key)) {
        let _ = put_request.await;
    }

    if let Ok(commit_request) = transaction.commit() {
        let _ = commit_request.await;
    }
}
