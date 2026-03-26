use thiserror::Error;

use super::{StorePath, STORE_PATH_HASH_SIZE};

#[derive(Error, Debug)]
pub enum HarmoniaError {
    #[error(transparent)]
    NixBindings(#[from] nix_bindings_util::Error),
    #[error(transparent)]
    StorePathName(#[from] harmonia_store_core::store_path::StorePathNameError),
}

impl TryFrom<&harmonia_store_core::store_path::StorePath> for StorePath {
    type Error = HarmoniaError;

    fn try_from(
        harmonia_path: &harmonia_store_core::store_path::StorePath,
    ) -> Result<Self, HarmoniaError> {
        let hash: &[u8; STORE_PATH_HASH_SIZE] = harmonia_path.hash().as_ref();
        Ok(StorePath::from_parts(hash, harmonia_path.name().as_ref())?)
    }
}

impl TryFrom<&StorePath> for harmonia_store_core::store_path::StorePath {
    type Error = HarmoniaError;

    fn try_from(nix_path: &StorePath) -> Result<Self, HarmoniaError> {
        let hash = nix_path.hash()?;
        let harmonia_hash = harmonia_store_core::store_path::StorePathHash::new(hash);

        let name = nix_path.name()?;

        let harmonia_name: harmonia_store_core::store_path::StorePathName = name.parse()?;

        Ok(harmonia_store_core::store_path::StorePath::from((
            harmonia_hash,
            harmonia_name,
        )))
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn store_path_round_trip_harmonia() {
        let harmonia_path: harmonia_store_core::store_path::StorePath =
            "g1w7hy3qg1w7hy3qg1w7hy3qg1w7hy3q-foo.drv".parse().unwrap();

        let nix_path: crate::path::StorePath = (&harmonia_path).try_into().unwrap();

        let harmonia_round_trip: harmonia_store_core::store_path::StorePath =
            (&nix_path).try_into().unwrap();

        assert_eq!(harmonia_path, harmonia_round_trip);
    }

    #[test]
    fn store_path_harmonia_clone() {
        let harmonia_path: harmonia_store_core::store_path::StorePath =
            "g1w7hy3qg1w7hy3qg1w7hy3qg1w7hy3q-foo.drv".parse().unwrap();

        let nix_path: crate::path::StorePath = (&harmonia_path).try_into().unwrap();
        let cloned_path = nix_path.clone();

        assert_eq!(nix_path.name().unwrap(), cloned_path.name().unwrap());
        assert_eq!(nix_path.hash().unwrap(), cloned_path.hash().unwrap());
    }
}
