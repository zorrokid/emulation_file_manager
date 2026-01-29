use core_types::{FileSize, Sha1Checksum};
use sha1::{
    Digest, Sha1,
    digest::{consts::U20, generic_array::GenericArray},
};

pub fn get_sha1_and_size(str: &str) -> (Sha1Checksum, FileSize) {
    let mut hasher = Sha1::new();
    hasher.update(str.as_bytes());

    let expected_checksum: GenericArray<u8, U20> = hasher.finalize();
    let expected_checksum: Sha1Checksum = expected_checksum.into();

    let expected_size: FileSize = str.len() as u64;
    (expected_checksum, expected_size)
}

pub fn generate_random_uuid() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}
