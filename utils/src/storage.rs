use sdk::frame_support::{
	storage::types::{QueryKindTrait, StorageDoubleMap, StorageMap},
	traits::{Get, StorageInstance},
	ReversibleStorageHasher,
};

pub trait ClearAllPrefix<P: codec::FullEncode> {
	fn clear_all_prefix(prefix: &P);
}

impl<SP, H1, K1, H2, K2, V, QueryKind, OnEmpty, MaxValues> ClearAllPrefix<K1>
	for StorageDoubleMap<SP, H1, K1, H2, K2, V, QueryKind, OnEmpty, MaxValues>
where
	SP: StorageInstance,
	H1: ReversibleStorageHasher,
	K1: codec::FullCodec,
	H2: ReversibleStorageHasher,
	K2: codec::FullCodec,
	V: codec::FullCodec + 'static,
	QueryKind: QueryKindTrait<V, OnEmpty>,
	OnEmpty: Get<QueryKind::Query> + 'static,
	MaxValues: Get<Option<u32>>,
{
	fn clear_all_prefix(prefix: &K1) {
		Self::drain_prefix(prefix).for_each(|_| {});
	}
}

pub trait ClearAll {
	fn clear_all();
}

impl<SP, H, K, V, QueryKind, OnEmpty, MaxValues> ClearAll
	for StorageMap<SP, H, K, V, QueryKind, OnEmpty, MaxValues>
where
	SP: StorageInstance,
	H: ReversibleStorageHasher,
	K: codec::FullCodec,
	V: codec::FullCodec + 'static,
	QueryKind: QueryKindTrait<V, OnEmpty>,
	OnEmpty: Get<QueryKind::Query> + 'static,
	MaxValues: Get<Option<u32>>,
{
	fn clear_all() {
		Self::drain().for_each(|_| {});
	}
}
