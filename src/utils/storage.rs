use embedded_svc::storage::StorageImpl;
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault, EspDefaultNvs};

const POSTCARD_BUF_SIZE: usize = 500;
pub struct PostcardSerDe;

type Storage = StorageImpl::<POSTCARD_BUF_SIZE, EspDefaultNvs, PostcardSerDe>;

impl embedded_svc::storage::SerDe for PostcardSerDe {
    type Error = postcard::Error;

    fn serialize<'a, T>(&self, slice: &'a mut [u8], value: &T) -> Result<&'a [u8], Self::Error>
    where
        T: serde::Serialize,
    {
        postcard::to_slice(value, slice).map(|r| &*r)
    }

    fn deserialize<T>(&self, slice: &[u8]) -> Result<T, Self::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        postcard::from_bytes(slice)
    }
}

pub fn new(name: &str, read_write: bool) -> anyhow::Result<Storage> {
    let nvs_partition = EspNvsPartition::<NvsDefault>::take()?;
    let nvs = EspNvs::new(nvs_partition, name, read_write)?;
    let storage = Storage::new(nvs, PostcardSerDe);

    Ok(storage)
}