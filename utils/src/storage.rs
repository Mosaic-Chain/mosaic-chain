use sdk::frame_support::{
	storage::types::{QueryKindTrait, StorageDoubleMap, StorageMap},
	traits::{Get, StorageInstance},
	StorageHasher,
};

pub trait ClearAllPrefix<P: codec::FullEncode> {
	fn clear_all_prefix(prefix: &P, batch: u32);
}

impl<SP, H1, K1, H2, K2, V, QueryKind, OnEmpty, MaxValues> ClearAllPrefix<K1>
	for StorageDoubleMap<SP, H1, K1, H2, K2, V, QueryKind, OnEmpty, MaxValues>
where
	SP: StorageInstance,
	H1: StorageHasher,
	K1: codec::FullCodec,
	H2: StorageHasher,
	K2: codec::FullCodec,
	V: codec::FullCodec + 'static,
	QueryKind: QueryKindTrait<V, OnEmpty>,
	OnEmpty: Get<QueryKind::Query> + 'static,
	MaxValues: Get<Option<u32>>,
{
	fn clear_all_prefix(prefix: &K1, batch: u32) {
		let mut maybe_cursor = None;
		loop {
			maybe_cursor = Self::clear_prefix(prefix, batch, maybe_cursor.as_deref()).maybe_cursor;

			if maybe_cursor.is_none() {
				break;
			}
		}
	}
}

pub trait ClearAll {
	fn clear_all(batch: u32);
}

impl<SP, H, K, V, QueryKind, OnEmpty, MaxValues> ClearAll
	for StorageMap<SP, H, K, V, QueryKind, OnEmpty, MaxValues>
where
	SP: StorageInstance,
	H: StorageHasher,
	K: codec::FullCodec,
	V: codec::FullCodec + 'static,
	QueryKind: QueryKindTrait<V, OnEmpty>,
	OnEmpty: Get<QueryKind::Query> + 'static,
	MaxValues: Get<Option<u32>>,
{
	fn clear_all(batch: u32) {
		let mut maybe_cursor = None;
		loop {
			maybe_cursor = Self::clear(batch, maybe_cursor.as_deref()).maybe_cursor;

			if maybe_cursor.is_none() {
				break;
			}
		}
	}
}
